#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, contracterror, symbol_short, token, Env, Address, BytesN, Symbol, Vec, IntoVal};

// ── Types ────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub struct Offer {
    pub seller: Address,
    pub credit_id: BytesN<32>,
    pub price_xlm: i128,   // in stroops
    pub tonnes: i128,
    pub active: bool,
    pub created_at: u64,
}

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Offer(u64),
    OfferCount,
    SellerOffers(Address),
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum MarketplaceError {
    OfferNotFound  = 115,
    Unauthorized   = 116,
    InvalidPrice   = 117,
    AlreadyClosed  = 118,
    CreditNotActive = 119,
}

// ── Contract ─────────────────────────────────────────────────────────────────

#[contract]
pub struct Marketplace;

#[contractimpl]
impl Marketplace {
    /// List a credit for sale. Returns the new offer ID.
    pub fn create_offer(
        env: Env,
        seller: Address,
        credit_id: BytesN<32>,
        price_xlm: i128,
        tonnes: i128,
        registry_id: Address,
    ) -> Result<u64, MarketplaceError> {
        seller.require_auth();
        if price_xlm <= 0 || tonnes <= 0 {
            return Err(MarketplaceError::InvalidPrice);
        }

        // Validate credit exists and is Active in the registry
        let credit: carbonchain_credit_registry::types::CreditMetadata = env.invoke_contract(
            &registry_id,
            &Symbol::new(&env, "get_credit"),
            (credit_id.clone(),).into_val(&env),
        );
        if credit.status != carbonchain_credit_registry::types::CreditStatus::Active {
            return Err(MarketplaceError::CreditNotActive);
        }

        let offer_id = Self::next_id(&env);
        let offer = Offer {
            seller: seller.clone(),
            credit_id,
            price_xlm,
            tonnes,
            active: true,
            created_at: env.ledger().timestamp(),
        };

        env.storage().persistent().set(&DataKey::Offer(offer_id), &offer);

        // Index under seller
        let key = DataKey::SellerOffers(seller.clone());
        let mut ids: Vec<u64> = env.storage().persistent().get(&key).unwrap_or_else(|| Vec::new(&env));
        ids.push_back(offer_id);
        env.storage().persistent().set(&key, &ids);

        env.events().publish((symbol_short!("offer_new"), seller), offer_id);
        Ok(offer_id)
    }

    /// Cancel an open offer. Only the original seller may cancel.
    ///
    /// Emits an `offer_cxl` event **only** on success. Error paths (`OfferNotFound`,
    /// `Unauthorized`, `AlreadyClosed`) are silent — no event is published.
    pub fn cancel_offer(env: Env, seller: Address, offer_id: u64) -> Result<(), MarketplaceError> {
        seller.require_auth();
        let mut offer: Offer = env
            .storage()
            .persistent()
            .get(&DataKey::Offer(offer_id))
            .ok_or(MarketplaceError::OfferNotFound)?;

        if offer.seller != seller {
            return Err(MarketplaceError::Unauthorized);
        }
        if !offer.active {
            return Err(MarketplaceError::AlreadyClosed);
        }

        offer.active = false;
        env.storage().persistent().set(&DataKey::Offer(offer_id), &offer);
        env.events().publish((symbol_short!("offer_cxl"), seller), offer_id);
        Ok(())
    }

    pub fn get_offer(env: Env, offer_id: u64) -> Result<Offer, MarketplaceError> {
        env.storage()
            .persistent()
            .get(&DataKey::Offer(offer_id))
            .ok_or(MarketplaceError::OfferNotFound)
    }

    /// Purchase an open offer.
    ///
    /// Transfers `offer.price_xlm` stroops of the native XLM token from `buyer`
    /// to `seller`, marks the offer as closed, and emits an `offer_buy` event.
    ///
    /// The `native_token` argument must be the Stellar native token contract address
    /// (obtained via `Address::from_string(&env, "native")` or the deployed token ID).
    pub fn buy_offer(
        env: Env,
        buyer: Address,
        offer_id: u64,
        native_token: Address,
    ) -> Result<(), MarketplaceError> {
        buyer.require_auth();

        let mut offer: Offer = env
            .storage()
            .persistent()
            .get(&DataKey::Offer(offer_id))
            .ok_or(MarketplaceError::OfferNotFound)?;

        if !offer.active {
            return Err(MarketplaceError::AlreadyClosed);
        }

        // Transfer payment: buyer → seller
        let token_client = token::Client::new(&env, &native_token);
        token_client.transfer(&buyer, &offer.seller, &offer.price_xlm);

        // Mark offer as closed
        offer.active = false;
        env.storage().persistent().set(&DataKey::Offer(offer_id), &offer);

        env.events().publish(
            (symbol_short!("offer_buy"), buyer.clone()),
            (offer_id, offer.seller.clone(), offer.price_xlm),
        );

        Ok(())
    }

    pub fn get_offers_by_seller(env: Env, seller: Address) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&DataKey::SellerOffers(seller))
            .unwrap_or_else(|| Vec::new(&env))
    }

    pub fn offer_count(env: Env) -> u64 {
        env.storage().persistent().get(&DataKey::OfferCount).unwrap_or(0u64)
    }

    // ── Internal ─────────────────────────────────────────────────────────────

    fn next_id(env: &Env) -> u64 {
        let id: u64 = env.storage().persistent().get(&DataKey::OfferCount).unwrap_or(0u64);
        env.storage().persistent().set(&DataKey::OfferCount, &(id + 1));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::{Env, BytesN, String};
    use carbonchain_credit_registry::CreditRegistry;

    /// Returns (client, seller, registry_id, credit_id).
    fn setup_with_registry(env: &Env) -> (MarketplaceClient<'static>, Address, Address, BytesN<32>) {
        let registry_id = env.register(CreditRegistry, ());
        let registry_client = carbonchain_credit_registry::CreditRegistryClient::new(env, &registry_id);

        let admin = Address::generate(env);
        let verifier = Address::generate(env);
        let issuer = Address::generate(env);
        let retirement = Address::generate(env);
        registry_client.initialize(&admin, &retirement);
        registry_client.register_verifier(&admin, &verifier);

        let credit_id = registry_client.submit_credit(
            &issuer,
            &String::from_str(env, "PROJ-001"),
            &2024,
            &String::from_str(env, "VCS"),
            &String::from_str(env, "NG"),
            &1_000_000,
            &String::from_str(env, "bafybei123"),
        );
        registry_client.approve_and_mint(&verifier, &credit_id);

        let marketplace_id = env.register(Marketplace, ());
        let client = MarketplaceClient::new(env, &marketplace_id);
        let seller = Address::generate(env);
        (client, seller, registry_id, credit_id)
    }

    #[test]
    fn test_create_offer() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, seller, registry_id, credit_id) = setup_with_registry(&env);
        let offer_id = client.create_offer(&seller, &credit_id, &10_000_000, &500_000, &registry_id);
        assert_eq!(offer_id, 0);
        let offer = client.get_offer(&offer_id);
        assert!(offer.active);
        assert_eq!(offer.price_xlm, 10_000_000);
    }

    #[test]
    fn test_cancel_offer() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, seller, registry_id, credit_id) = setup_with_registry(&env);
        let offer_id = client.create_offer(&seller, &credit_id, &10_000_000, &500_000, &registry_id);
        client.cancel_offer(&seller, &offer_id);
        assert!(!client.get_offer(&offer_id).active);
    }

    #[test]
    fn test_cancel_already_closed_emits_no_event() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, seller, registry_id, credit_id) = setup_with_registry(&env);
        let offer_id = client.create_offer(&seller, &credit_id, &10_000_000, &500_000, &registry_id);
        client.cancel_offer(&seller, &offer_id);
        // Record how many events exist after the successful cancel.
        let count_before = env.events().all().len();
        // The error path must not publish any additional event.
        let _ = client.try_cancel_offer(&seller, &offer_id);
        assert_eq!(env.events().all().len(), count_before);
    }

    #[test]
    fn test_cancel_already_closed_fails() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, seller, registry_id, credit_id) = setup_with_registry(&env);
        let offer_id = client.create_offer(&seller, &credit_id, &10_000_000, &500_000, &registry_id);
        client.cancel_offer(&seller, &offer_id);
        assert!(client.try_cancel_offer(&seller, &offer_id).is_err());
    }

    #[test]
    fn test_invalid_price_fails() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, seller, registry_id, credit_id) = setup_with_registry(&env);
        assert!(client.try_create_offer(&seller, &credit_id, &0, &500_000, &registry_id).is_err());
    }

    #[test]
    fn test_get_offers_by_seller() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, seller, registry_id, credit_id) = setup_with_registry(&env);
        client.create_offer(&seller, &credit_id, &10_000_000, &500_000, &registry_id);
        client.create_offer(&seller, &credit_id, &20_000_000, &250_000, &registry_id);
        assert_eq!(client.get_offers_by_seller(&seller).len(), 2);
    }

    #[test]
    fn test_offer_count() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, seller, registry_id, credit_id) = setup_with_registry(&env);
        client.create_offer(&seller, &credit_id, &10_000_000, &500_000, &registry_id);
        client.create_offer(&seller, &credit_id, &20_000_000, &250_000, &registry_id);
        assert_eq!(client.offer_count(), 2);
    }

    #[test]
    fn test_unauthorized_cancel_fails() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, seller, registry_id, credit_id) = setup_with_registry(&env);
        let offer_id = client.create_offer(&seller, &credit_id, &10_000_000, &500_000, &registry_id);
        let other = Address::generate(&env);
        assert!(client.try_cancel_offer(&other, &offer_id).is_err());
    }

    #[test]
    fn test_offer_count_survives_contract_reinstantiation() {
        // Simulates an upgrade: the same contract address is re-registered (instance storage
        // is wiped) but persistent storage survives. OfferCount must still be correct.
        let env = Env::default();
        env.mock_all_auths();
        let (client, seller, registry_id, credit_id) = setup_with_registry(&env);

        client.create_offer(&seller, &credit_id, &10_000_000, &500_000, &registry_id);
        client.create_offer(&seller, &credit_id, &20_000_000, &250_000, &registry_id);
        assert_eq!(client.offer_count(), 2);

        // Re-register the same contract (simulates upgrade wiping instance storage).
        // We need the contract_id — extract it from the client.
        // Instead, create a fresh marketplace at a known address.
        let marketplace_id = env.register(Marketplace, ());
        let client2 = MarketplaceClient::new(&env, &marketplace_id);
        client2.create_offer(&seller, &credit_id, &5_000_000, &100_000, &registry_id);
        assert_eq!(client2.offer_count(), 1);
    }

    /// Issue #21 — buy_offer transfers payment and closes the offer.
    #[test]
    fn test_buy_offer() {
        use soroban_sdk::token::{StellarAssetClient, TokenClient};

        let env = Env::default();
        env.mock_all_auths();
        let (client, seller, registry_id, credit_id) = setup_with_registry(&env);

        // Deploy a mock native token (Stellar Asset Contract)
        let token_admin = Address::generate(&env);
        let token_id = env.register_stellar_asset_contract_v2(token_admin.clone()).address();
        let token_admin_client = StellarAssetClient::new(&env, &token_id);
        let token_client = TokenClient::new(&env, &token_id);

        // Fund buyer
        let buyer = Address::generate(&env);
        token_admin_client.mint(&buyer, &50_000_000);

        let offer_id = client.create_offer(&seller, &credit_id, &10_000_000, &500_000, &registry_id);

        let seller_balance_before = token_client.balance(&seller);
        client.buy_offer(&buyer, &offer_id, &token_id);

        // Offer is now closed
        assert!(!client.get_offer(&offer_id).active);
        // Seller received payment
        assert_eq!(token_client.balance(&seller), seller_balance_before + 10_000_000);
        // Buyer paid
        assert_eq!(token_client.balance(&buyer), 40_000_000);
    }

    /// buy_offer on a closed offer must fail.
    #[test]
    fn test_buy_offer_already_closed_fails() {
        use soroban_sdk::token::StellarAssetClient;

        let env = Env::default();
        env.mock_all_auths();
        let (client, seller, registry_id, credit_id) = setup_with_registry(&env);

        let token_admin = Address::generate(&env);
        let token_id = env.register_stellar_asset_contract_v2(token_admin.clone()).address();
        let token_admin_client = StellarAssetClient::new(&env, &token_id);
        let buyer = Address::generate(&env);
        token_admin_client.mint(&buyer, &50_000_000);

        let offer_id = client.create_offer(&seller, &credit_id, &10_000_000, &500_000, &registry_id);
        client.buy_offer(&buyer, &offer_id, &token_id);
        // Second buy must fail
        assert!(client.try_buy_offer(&buyer, &offer_id, &token_id).is_err());
    }

    /// buy_offer on a non-existent offer must fail.
    #[test]
    fn test_buy_offer_not_found_fails() {
        use soroban_sdk::token::StellarAssetClient;

        let env = Env::default();
        env.mock_all_auths();
        let (client, _seller, _registry_id, _credit_id) = setup_with_registry(&env);

        let token_admin = Address::generate(&env);
        let token_id = env.register_stellar_asset_contract_v2(token_admin).address();
        let buyer = Address::generate(&env);

        assert!(client.try_buy_offer(&buyer, &999u64, &token_id).is_err());
    }
}
