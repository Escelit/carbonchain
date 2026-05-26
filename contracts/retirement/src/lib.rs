#![no_std]
pub mod types;

use crate::types::{DataKey, RetirementRecord};
use soroban_sdk::{
    contract, contractimpl, symbol_short,
    Address, BytesN, Env, String, Symbol, Vec,
    IntoVal,
};
use soroban_sdk::xdr::ToXdr;

#[contract]
pub struct Retirement;

#[contractimpl]
impl Retirement {
    /// Retire a carbon credit.
    ///
    /// - Stores an immutable `RetirementRecord`
    /// - Calls `mark_retired` on the credit registry to flip the credit status
    /// - Indexes the retirement under the buyer's account
    /// - Emits a `retire` event
    ///
    /// `registry_id` — the deployed credit_registry contract address.
    pub fn retire(
        env: Env,
        buyer: Address,
        credit_id: BytesN<32>,
        tonnes: i128,
        reason: String,
        registry_id: Address,
    ) -> BytesN<32> {
        buyer.require_auth();

        // Derive a deterministic retirement ID from credit_id + reason
        let mut preimage = credit_id.clone().to_xdr(&env);
        preimage.append(&reason.clone().to_xdr(&env));
        let retirement_id: BytesN<32> = env.crypto().sha256(&preimage).into();

        let record = RetirementRecord {
            credit_id: credit_id.clone(),
            buyer: buyer.clone(),
            tonnes_retired: tonnes,
            reason,
            retired_at: env.ledger().timestamp(),
        };

        env.storage()
            .persistent()
            .set(&DataKey::Retirement(retirement_id.clone()), &record);

        // Index under buyer account
        let acct_key = DataKey::AccountRetirements(buyer.clone());
        let mut list: Vec<BytesN<32>> = env
            .storage()
            .persistent()
            .get(&acct_key)
            .unwrap_or_else(|| Vec::new(&env));
        list.push_back(retirement_id.clone());
        env.storage().persistent().set(&acct_key, &list);

        // Cross-contract: mark the credit as retired in the registry
        let _: () = env.invoke_contract(
            &registry_id,
            &Symbol::new(&env, "mark_retired"),
            (credit_id.clone(),).into_val(&env),
        );

        // Emit retirement event
        env.events().publish(
            (symbol_short!("retire"), buyer),
            (credit_id, retirement_id.clone()),
        );

        retirement_id
    }

    pub fn get_retirement(env: Env, retirement_id: BytesN<32>) -> Option<RetirementRecord> {
        env.storage()
            .persistent()
            .get(&DataKey::Retirement(retirement_id))
    }

    pub fn get_retirements_by_account(env: Env, account: Address) -> Vec<BytesN<32>> {
        env.storage()
            .persistent()
            .get(&DataKey::AccountRetirements(account))
            .unwrap_or_else(|| Vec::new(&env))
    }

    /// Returns one page of retirement IDs for `account`. `page` is 0-indexed; `page_size` capped at 50.
    pub fn get_retirements_by_account_paginated(
        env: Env,
        account: Address,
        page: u32,
        page_size: u32,
    ) -> Vec<BytesN<32>> {
        let page_size = if page_size == 0 || page_size > 50 { 50 } else { page_size };
        let all: Vec<BytesN<32>> = env
            .storage()
            .persistent()
            .get(&DataKey::AccountRetirements(account))
            .unwrap_or_else(|| Vec::new(&env));
        let start = page * page_size;
        let mut out: Vec<BytesN<32>> = Vec::new(&env);
        let mut i = start;
        while i < start + page_size && i < all.len() {
            out.push_back(all.get(i).unwrap());
            i += 1;
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::{Env, String};
    use carbonchain_credit_registry::CreditRegistry;

    fn setup_registry(env: &Env) -> (Address, Address, Address, BytesN<32>) {
        let registry_id = env.register(CreditRegistry, ());
        let registry_client =
            carbonchain_credit_registry::CreditRegistryClient::new(env, &registry_id);

        let admin = Address::generate(env);
        let verifier = Address::generate(env);
        let issuer = Address::generate(env);

        registry_client.initialize(&admin);
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

        (registry_id, admin, verifier, credit_id)
    }

    #[test]
    fn test_retire_stores_record() {
        let env = Env::default();
        env.mock_all_auths();

        let (registry_id, _admin, _verifier, credit_id) = setup_registry(&env);

        let contract_id = env.register(Retirement, ());
        let client = RetirementClient::new(&env, &contract_id);
        let buyer = Address::generate(&env);

        let ret_id = client.retire(
            &buyer,
            &credit_id,
            &1_000_000,
            &String::from_str(&env, "2024 Scope 3 offset"),
            &registry_id,
        );

        let record = client.get_retirement(&ret_id).unwrap();
        assert_eq!(record.buyer, buyer);
        assert_eq!(record.tonnes_retired, 1_000_000);
        assert_eq!(record.credit_id, credit_id);
    }

    #[test]
    fn test_retire_indexes_by_account() {
        let env = Env::default();
        env.mock_all_auths();

        let (registry_id, _admin, _verifier, credit_id) = setup_registry(&env);

        let contract_id = env.register(Retirement, ());
        let client = RetirementClient::new(&env, &contract_id);
        let buyer = Address::generate(&env);

        let ret_id = client.retire(
            &buyer,
            &credit_id,
            &1_000_000,
            &String::from_str(&env, "offset"),
            &registry_id,
        );

        let ids = client.get_retirements_by_account(&buyer);
        assert_eq!(ids.len(), 1);
        assert_eq!(ids.get(0).unwrap(), ret_id);
    }

    #[test]
    fn test_retire_marks_credit_retired_in_registry() {
        let env = Env::default();
        env.mock_all_auths();

        let (registry_id, _admin, _verifier, credit_id) = setup_registry(&env);
        let registry_client =
            carbonchain_credit_registry::CreditRegistryClient::new(&env, &registry_id);

        let contract_id = env.register(Retirement, ());
        let client = RetirementClient::new(&env, &contract_id);
        let buyer = Address::generate(&env);

        client.retire(
            &buyer,
            &credit_id,
            &1_000_000,
            &String::from_str(&env, "offset"),
            &registry_id,
        );

        let credit = registry_client.get_credit(&credit_id);
        assert_eq!(
            credit.status,
            carbonchain_credit_registry::types::CreditStatus::Retired
        );
    }

    #[test]
    fn test_get_retirements_by_account_paginated() {
        let env = Env::default();
        env.mock_all_auths();

        // Register 3 separate credits so we can retire each once.
        let registry_id = env.register(CreditRegistry, ());
        let registry_client =
            carbonchain_credit_registry::CreditRegistryClient::new(&env, &registry_id);
        let admin = Address::generate(&env);
        let verifier = Address::generate(&env);
        let issuer = Address::generate(&env);
        registry_client.initialize(&admin);
        registry_client.register_verifier(&admin, &verifier);

        let contract_id = env.register(Retirement, ());
        let client = RetirementClient::new(&env, &contract_id);
        let buyer = Address::generate(&env);

        let project_ids = [
            String::from_str(&env, "PROJ-A"),
            String::from_str(&env, "PROJ-B"),
            String::from_str(&env, "PROJ-C"),
        ];
        let mut ret_ids = soroban_sdk::Vec::new(&env);
        for proj in project_ids.iter() {
            let cid = registry_client.submit_credit(
                &issuer,
                proj,
                &2024,
                &String::from_str(&env, "VCS"),
                &String::from_str(&env, "NG"),
                &1_000_000,
                &String::from_str(&env, "bafybei"),
            );
            registry_client.approve_and_mint(&verifier, &cid);
            let rid = client.retire(
                &buyer,
                &cid,
                &1_000_000,
                &String::from_str(&env, "offset"),
                &registry_id,
            );
            ret_ids.push_back(rid);
        }

        // page 0, size 2 → first 2
        let p0 = client.get_retirements_by_account_paginated(&buyer, &0, &2);
        assert_eq!(p0.len(), 2);
        assert_eq!(p0.get(0).unwrap(), ret_ids.get(0).unwrap());
        // page 1, size 2 → last 1
        let p1 = client.get_retirements_by_account_paginated(&buyer, &1, &2);
        assert_eq!(p1.len(), 1);
        assert_eq!(p1.get(0).unwrap(), ret_ids.get(2).unwrap());
    }
}
