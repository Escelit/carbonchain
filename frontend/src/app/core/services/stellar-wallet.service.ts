import { Injectable, signal, computed } from '@angular/core';

// Freighter injects `window.freighter` — we declare a minimal interface here
// rather than pulling in the full SDK to keep the bundle lean.
interface FreighterApi {
  isConnected(): Promise<boolean>;
  getPublicKey(): Promise<string>;
  signTransaction(xdr: string, opts?: { networkPassphrase?: string }): Promise<string>;
  getNetworkDetails(): Promise<{ network: string; networkPassphrase: string }>;
}

declare global {
  interface Window {
    freighter?: FreighterApi;
  }
}

export type WalletState = 'disconnected' | 'connecting' | 'connected' | 'error';

@Injectable({ providedIn: 'root' })
export class StellarWalletService {
  private readonly _publicKey = signal<string | null>(null);
  private readonly _state = signal<WalletState>('disconnected');
  private readonly _error = signal<string | null>(null);

  readonly publicKey = this._publicKey.asReadonly();
  readonly state = this._state.asReadonly();
  readonly error = this._error.asReadonly();
  readonly isConnected = computed(() => this._state() === 'connected');

  /** Returns true if the Freighter extension is installed in the browser. */
  get isFreighterInstalled(): boolean {
    return typeof window !== 'undefined' && !!window.freighter;
  }

  /** Connects to Freighter and retrieves the user's public key. */
  async connect(): Promise<string> {
    if (!this.isFreighterInstalled) {
      const msg = 'Freighter wallet extension is not installed.';
      this._error.set(msg);
      this._state.set('error');
      throw new Error(msg);
    }

    this._state.set('connecting');
    this._error.set(null);

    try {
      const connected = await window.freighter!.isConnected();
      if (!connected) {
        throw new Error('Freighter is not connected. Please unlock your wallet.');
      }

      const publicKey = await window.freighter!.getPublicKey();
      this._publicKey.set(publicKey);
      this._state.set('connected');
      return publicKey;
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'Failed to connect to Freighter.';
      this._error.set(msg);
      this._state.set('error');
      throw err;
    }
  }

  /** Signs a Stellar transaction XDR string using Freighter. */
  async signTransaction(xdr: string, networkPassphrase?: string): Promise<string> {
    if (!this.isConnected()) {
      throw new Error('Wallet is not connected. Call connect() first.');
    }

    try {
      return await window.freighter!.signTransaction(xdr, { networkPassphrase });
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'Transaction signing failed.';
      this._error.set(msg);
      throw err;
    }
  }

  /** Returns the current network details from Freighter. */
  async getNetworkDetails(): Promise<{ network: string; networkPassphrase: string }> {
    if (!this.isFreighterInstalled) {
      throw new Error('Freighter wallet extension is not installed.');
    }
    return window.freighter!.getNetworkDetails();
  }

  /** Disconnects the wallet (clears local state — Freighter has no explicit disconnect API). */
  disconnect(): void {
    this._publicKey.set(null);
    this._state.set('disconnected');
    this._error.set(null);
  }
}
