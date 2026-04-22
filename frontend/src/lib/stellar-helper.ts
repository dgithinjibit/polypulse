/**
 * ============================================================
 * FILE: stellar-helper.ts
 * PURPOSE: Central Stellar blockchain utility class for PolyPulse.
 *          Wraps the @stellar/freighter-api (Freighter browser extension)
 *          and @stellar/stellar-sdk (Stellar network SDK) to provide:
 *            - Wallet connection / disconnection
 *            - Balance fetching
 *            - XLM payment sending
 *            - Transaction signing for authentication
 *            - Recent transaction history
 *            - Address formatting and explorer links
 *
 * DEPENDENCIES:
 *   - @stellar/stellar-sdk  : Official Stellar SDK for building/submitting transactions (LAZY LOADED)
 *   - @stellar/freighter-api: Direct API to the Freighter browser extension
 *
 * PERFORMANCE NOTE:
 *   The Stellar SDK is lazy loaded to improve initial page load performance.
 *   It's only loaded when wallet features are actually used.
 *
 * USAGE:
 *   Import the singleton `stellar` instance exported at the bottom.
 *   Example: import { stellar } from '@/lib/stellar-helper'
 *            const address = await stellar.connectWallet()
 *
 * NOTE FOR JUNIOR DEVS:
 *   Freighter is a browser extension wallet for Stellar (like MetaMask for Ethereum).
 *   The user must have it installed. We communicate with it via @stellar/freighter-api.
 * ============================================================
 */

// Lazy load Stellar SDK for better performance
import { loadStellarSDK } from './stellar-sdk-loader';

// Import specific functions from the Freighter browser extension API.
// These functions communicate with the Freighter extension installed in the user's browser.
import {
  requestAccess,                               // Asks the user to grant this site access to their wallet
  getAddress,                                  // Gets the user's public key without requesting access again
  signTransaction as freighterSignTransaction, // Signs a transaction XDR string using Freighter
} from '@stellar/freighter-api';

// ============================================================
// CLASS: StellarHelper
// PURPOSE: Encapsulates all Stellar blockchain operations.
//          One instance is created and exported as `stellar` singleton.
//          Junior devs: think of this as the "blockchain service layer".
// PERFORMANCE: Stellar SDK is lazy loaded on first use to improve initial page load.
// ============================================================
export class StellarHelper {
  // The Horizon server client - lazy loaded when first needed
  private server: any | null = null;

  // The network passphrase uniquely identifies which Stellar network we're on.
  private networkPassphrase: string;
  
  // Network type for initialization
  private network: 'testnet' | 'mainnet';

  // Cached public key of the connected wallet.
  private publicKey: string | null = null;
  
  // Cached Stellar SDK module
  private StellarSdk: typeof import('@stellar/stellar-sdk') | null = null;

  // ============================================================
  // CONSTRUCTOR
  // PURPOSE: Sets up network configuration (defers SDK loading for performance)
  // PARAM network: 'testnet' (default, for development) or 'mainnet' (real money, production)
  // ============================================================
  constructor(network: 'testnet' | 'mainnet' = 'testnet') {
    this.network = network;
    
    // Set the network passphrase - we can do this without loading the SDK
    this.networkPassphrase =
      network === 'testnet'
        ? 'Test SDF Network ; September 2015'  // StellarSdk.Networks.TESTNET
        : 'Public Global Stellar Network ; September 2015';  // StellarSdk.Networks.PUBLIC
  }
  
  // ============================================================
  // METHOD: ensureSDKLoaded (PRIVATE)
  // PURPOSE: Lazy loads the Stellar SDK on first use
  // RETURNS: Promise<void>
  // ============================================================
  private async ensureSDKLoaded(): Promise<void> {
    if (this.StellarSdk) return; // Already loaded
    
    // Load the SDK
    this.StellarSdk = await loadStellarSDK();
    
    // Initialize the Horizon server now that SDK is loaded
    if (!this.server) {
      this.server = new this.StellarSdk.Horizon.Server(
        this.network === 'testnet'
          ? 'https://horizon-testnet.stellar.org'
          : 'https://horizon.stellar.org'
      );
    }
  }

  // ============================================================
  // METHOD: isFreighterInstalled
  // PURPOSE: Quick check to see if the Freighter extension is present in the browser.
  // RETURNS: true if window.freighter exists (extension injects this), false otherwise.
  // USE CASE: Show "Install Freighter" link if this returns false.
  // ============================================================
  isFreighterInstalled(): boolean {
    // window is undefined in SSR (server-side rendering) environments like Next.js.
    // We check for it first to avoid crashes.
    // window.freighter is injected by the Freighter extension when it's installed.
    return typeof window !== 'undefined' && !!(window as any).freighter;
  } // end isFreighterInstalled

  // ============================================================
  // METHOD: connectWallet
  // PURPOSE: Connects the user's Freighter wallet to this app.
  //          1. Checks Freighter is available
  //          2. Requests user permission (shows Freighter popup)
  //          3. Gets and caches the user's public key
  // RETURNS: Promise<string> - the user's Stellar public key (starts with 'G')
  // THROWS:
  //   - WalletConnectionError if window is undefined (SSR environment)
  //   - WalletNotInstalledError if Freighter extension is not connected
  //   - WalletRejectedError if user clicks "Deny" in the Freighter popup
  //   - WalletConnectionError for any other access error
  // CALLED BY: WalletConnection component, StellarWalletContext
  // ============================================================
  async connectWallet(): Promise<string> {
    // Ensure SDK is loaded before proceeding
    await this.ensureSDKLoaded();
    
    // Guard: this code only works in a browser, not in Node.js/SSR
    if (typeof window === 'undefined') {
      throw new WalletConnectionError('Window is not defined');
    } // end SSR guard

    // Skip isConnected() check - it returns false when the site hasn't been granted
    // access yet, even if Freighter IS installed. Just call requestAccess() directly.
    // requestAccess() will trigger the Freighter popup for the user to approve.
    // Returns { address: string, error: string | null }
    const { address, error: accessError } = await requestAccess();

    // If there was an error during access request, handle it
    if (accessError) {
      // User clicked "Deny" in the Freighter popup
      if (accessError.toString().includes('User declined')) {
        throw new WalletRejectedError('connection');
      } // end user declined check

      // Some other error occurred (e.g., extension crashed)
      throw new WalletConnectionError(accessError.toString());
    } // end access error check

    // Sanity check: address should never be empty if there's no error, but just in case
    if (!address) {
      throw new WalletConnectionError('No address returned from Freighter');
    } // end empty address check

    // Cache the public key so we don't need to ask Freighter again
    this.publicKey = address;

    // Return the public key to the caller (e.g., StellarWalletContext stores it in state)
    return address;
  } // end connectWallet

  // ============================================================
  // METHOD: getPublicKey
  // PURPOSE: Gets the currently connected wallet's public key.
  //          First checks the cache, then asks Freighter directly.
  //          Does NOT request access - silent check only.
  // RETURNS: Promise<string | null> - public key or null if not connected
  // USE CASE: Called on app startup to restore wallet state without showing a popup.
  // ============================================================
  async getPublicKey(): Promise<string | null> {
    // Can't access browser APIs in SSR
    if (typeof window === 'undefined') return null;

    // Return cached key if we already have it - avoids unnecessary Freighter calls
    if (this.publicKey) return this.publicKey;

    try {
      // Ask Freighter for the address silently (no popup shown to user)
      // getAddress() returns { address: string, error: string | null }
      const { address, error } = await getAddress();

      // If there's an error or no address, user is not connected - return null
      if (error || !address) return null;

      // Cache and return the address
      this.publicKey = address;
      return address;
    } catch (e) {
      // Freighter might throw if extension is not available - return null gracefully
      return null;
    } // end try/catch
  } // end getPublicKey

  // ============================================================
  // METHOD: getBalance
  // PURPOSE: Fetches the XLM and token balances for a given Stellar address.
  //          Calls the Horizon API to load the account's balance data.
  // PARAM publicKey: The Stellar public key (starts with 'G') to check balance for
  // RETURNS: Promise with { xlm: string, assets: Array<{code, issuer, balance}> }
  //   - xlm: The native XLM balance as a string (e.g., "100.0000000")
  //   - assets: Array of non-native tokens (e.g., USDC, custom tokens)
  // THROWS: If the account doesn't exist on the network or network is unreachable
  // ============================================================
  async getBalance(publicKey: string): Promise<{
    xlm: string;
    assets: Array<{ code: string; issuer: string; balance: string }>;
  }> {
    // Ensure SDK is loaded before proceeding
    await this.ensureSDKLoaded();
    
    // Load the full account data from Horizon - includes all balances
    const account = await this.server.loadAccount(publicKey);

    // Find the native XLM balance - asset_type === 'native' means XLM
    const xlmBalance = account.balances.find((b: any) => b.asset_type === 'native');

    // Extract all non-native token balances (custom assets like USDC)
    const assets = account.balances
      .filter((b: any) => b.asset_type !== 'native')  // Exclude XLM
      .map((b: any) => ({
        code: b.asset_code,      // Token symbol e.g. 'USDC'
        issuer: b.asset_issuer,  // The account that issued this token
        balance: b.balance,      // Balance as string e.g. '50.0000000'
      }));

    return {
      // Use 'balance' in xlmBalance check because TypeScript needs type narrowing here
      xlm: xlmBalance && 'balance' in xlmBalance ? xlmBalance.balance : '0',
      assets,
    };
  } // end getBalance

  // ============================================================
  // METHOD: sendPayment
  // PURPOSE: Sends XLM from one account to another.
  //          Builds a Stellar transaction, signs it with Freighter, submits to network.
  // PARAMS:
  //   - from: Sender's public key
  //   - to: Recipient's public key
  //   - amount: Amount of XLM to send as string (e.g., "10.5")
  //   - memo: Optional text memo attached to the transaction (max 28 bytes)
  // RETURNS: Promise<{ hash: string, success: boolean }>
  //   - hash: The transaction hash (can be looked up on Stellar Explorer)
  //   - success: Whether the transaction was accepted by the network
  // THROWS: SignatureError if user rejects signing in Freighter
  // ============================================================
  async sendPayment(params: {
    from: string;
    to: string;
    amount: string;
    memo?: string;
  }): Promise<{ hash: string; success: boolean }> {
    // Ensure SDK is loaded before proceeding
    await this.ensureSDKLoaded();
    
    // Load the sender's account to get the current sequence number.
    // Stellar requires a sequence number to prevent replay attacks.
    const account = await this.server.loadAccount(params.from);

    // Start building the transaction with the sender's account info
    const transactionBuilder = new this.StellarSdk!.TransactionBuilder(account, {
      fee: this.StellarSdk!.BASE_FEE,              // Minimum fee in stroops (1 XLM = 10,000,000 stroops)
      networkPassphrase: this.networkPassphrase, // Must match the network we're on
    }).addOperation(
      // Add a payment operation - this is what actually moves the XLM
      this.StellarSdk!.Operation.payment({
        destination: params.to,              // Recipient's public key
        asset: this.StellarSdk!.Asset.native(),    // native() means XLM (not a custom token)
        amount: params.amount,               // Amount as string e.g. "10.5"
      })
    );

    // Optionally add a text memo (useful for exchanges that need a memo to credit your account)
    if (params.memo) {
      transactionBuilder.addMemo(this.StellarSdk!.Memo.text(params.memo));
    } // end memo check

    // Finalize the transaction with a 180-second timeout window
    // After 180 seconds, the transaction will be rejected if not submitted
    const transaction = transactionBuilder.setTimeout(180).build();

    // Ask Freighter to sign the transaction - shows popup to user
    // toXDR() converts the transaction to a base64 string that Freighter can sign
    const { signedTxXdr, error } = await freighterSignTransaction(transaction.toXDR(), {
      networkPassphrase: this.networkPassphrase,
    });

    // If user rejected signing or an error occurred, throw SignatureError
    if (error) throw new SignatureError(error.toString());

    // Reconstruct the transaction from the signed XDR string
    const transactionToSubmit = this.StellarSdk!.TransactionBuilder.fromXDR(
      signedTxXdr,
      this.networkPassphrase
    );

    // Submit the signed transaction to the Stellar network via Horizon
    const result = await this.server.submitTransaction(
      transactionToSubmit as any
    );

    // Return the transaction hash and success status
    return { hash: result.hash, success: result.successful };
  } // end sendPayment

  // ============================================================
  // METHOD: signAuthMessage
  // PURPOSE: Signs a message for backend authentication using the wallet.
  //          Instead of username/password, PolyPulse uses "sign this message" to prove
  //          you own the wallet. The backend verifies the signature.
  //          We encode the message as a Stellar manageData transaction.
  // PARAM message: The authentication message string from the backend (contains nonce)
  // RETURNS: Promise<{ signature: string, publicKey: string }>
  //   - signature: The signed transaction XDR (proof of wallet ownership)
  //   - publicKey: The wallet's public key
  // THROWS:
  //   - Error if no wallet is connected (call connectWallet first)
  //   - SignatureError if signing fails or user rejects
  // CALLED BY: StellarWalletContext.authenticateWithBackend
  // ============================================================
  async signAuthMessage(message: string): Promise<{ signature: string; publicKey: string }> {
    // Ensure SDK is loaded before proceeding
    await this.ensureSDKLoaded();
    
    // Can't sign without a connected wallet
    if (!this.publicKey) {
      throw new Error('No wallet connected. Call connectWallet() first.');
    } // end no wallet check

    // Load the account to get the sequence number (required for building transactions)
    const account = await this.server.loadAccount(this.publicKey);

    // Build a transaction with a manageData operation.
    // manageData stores a key-value pair on the account - we use it as a "sign this message" mechanism.
    // The backend can verify this transaction was signed by the wallet owner.
    const tx = new this.StellarSdk!.TransactionBuilder(account, {
      fee: this.StellarSdk!.BASE_FEE,
      networkPassphrase: this.networkPassphrase,
    })
      .addOperation(
        this.StellarSdk!.Operation.manageData({
          name: 'polypulse_auth',       // Key name - identifies this as a PolyPulse auth operation
          value: message.slice(0, 64), // Value - the auth message (max 64 bytes for manageData)
        })
      )
      .setTimeout(30) // 30 second window - auth should be fast
      .build();

    // Ask Freighter to sign this transaction - user sees a popup
    const { signedTxXdr, error } = await freighterSignTransaction(tx.toXDR(), {
      networkPassphrase: this.networkPassphrase,
    });

    // Signing failed (e.g., user rejected or extension error)
    if (error) throw new SignatureError(error.toString());

    // Extra safety check - signedTxXdr should never be empty if no error
    if (!signedTxXdr) throw new SignatureError('Signing returned empty XDR');

    // Return the signed XDR as the "signature" proof and the public key
    return { signature: signedTxXdr, publicKey: this.publicKey };
  } // end signAuthMessage

  // ============================================================
  // METHOD: getRecentTransactions
  // PURPOSE: Fetches recent payment transactions for a given account from Horizon.
  //          Used to display transaction history in the Wallet page.
  // PARAM publicKey: The Stellar address to fetch transactions for
  // PARAM limit: Max number of transactions to return (default 50)
  // RETURNS: Array of transaction objects with id, type, amount, asset, from, to, date, hash
  // ============================================================
  async getRecentTransactions(publicKey: string, limit: number = 50) {
    // Ensure SDK is loaded before proceeding
    await this.ensureSDKLoaded();
    
    // Query Horizon for payment operations on this account, newest first
    const payments = await this.server
      .payments()
      .forAccount(publicKey)  // Filter to this specific account
      .order('desc')          // Most recent first
      .limit(limit)           // Cap the results
      .call();                // Execute the HTTP request

    // Map the raw Horizon response to a cleaner format for the UI
    return payments.records.map((payment: any) => ({
      id: payment.id,
      type: payment.type,
      amount: payment.amount,
      // If asset_type is 'native' it's XLM, otherwise use the asset code (e.g., 'USDC')
      asset: payment.asset_type === 'native' ? 'XLM' : payment.asset_code,
      from: payment.from,
      to: payment.to,
      createdAt: payment.created_at,
      hash: payment.transaction_hash,  // Use this to look up on Stellar Explorer
    }));
  } // end getRecentTransactions

  // ============================================================
  // METHOD: getExplorerLink
  // PURPOSE: Generates a Stellar Explorer URL for a transaction or account.
  //          Useful for "View on Explorer" links in the UI.
  // PARAM hash: Transaction hash or account public key
  // PARAM type: 'tx' for transaction, 'account' for account page (default 'tx')
  // RETURNS: Full URL string to stellar.expert explorer
  // ============================================================
  getExplorerLink(hash: string, type: 'tx' | 'account' = 'tx'): string {
    // Determine which network explorer to use based on our network passphrase
    const network =
      this.networkPassphrase === 'Test SDF Network ; September 2015' ? 'testnet' : 'public';

    // stellar.expert is the most popular Stellar blockchain explorer
    return `https://stellar.expert/explorer/${network}/${type}/${hash}`;
  } // end getExplorerLink

  // ============================================================
  // METHOD: formatAddress
  // PURPOSE: Shortens a long Stellar public key for display in the UI.
  //          e.g., 'GBTEST...WXYZ' instead of the full 56-character key.
  // PARAM address: Full Stellar public key
  // PARAM startChars: How many characters to show at the start (default 4)
  // PARAM endChars: How many characters to show at the end (default 4)
  // RETURNS: Shortened string like 'GBTE...WXYZ'
  // ============================================================
  formatAddress(address: string, startChars = 4, endChars = 4): string {
    // If the address is already short enough, return it as-is
    if (address.length <= startChars + endChars) return address;

    // Slice the start and end, join with '...' in the middle
    return `${address.slice(0, startChars)}...${address.slice(-endChars)}`;
  } // end formatAddress

  // ============================================================
  // METHOD: disconnect
  // PURPOSE: Clears the cached wallet state.
  //          Note: This does NOT revoke Freighter's access to the site -
  //          that can only be done from within the Freighter extension itself.
  //          It just clears our local state so the app treats the user as logged out.
  // RETURNS: Promise<boolean> - always true (disconnect can't really fail)
  // CALLED BY: StellarWalletContext.disconnect, WalletConnection component
  // ============================================================
  async disconnect(): Promise<boolean> {
    // Clear the cached public key - app will treat user as disconnected
    this.publicKey = null;

    // Always return true - disconnect is a local operation that can't fail
    return true;
  } // end disconnect

} // end class StellarHelper

// ============================================================
// SINGLETON EXPORT
// PURPOSE: Creates one shared instance of StellarHelper for the whole app.
//          The network is read from the VITE_STELLAR_NETWORK environment variable.
//          Set VITE_STELLAR_NETWORK=testnet in .env for development.
//          Set VITE_STELLAR_NETWORK=mainnet in .env.production for production.
//          Falls back to 'testnet' if the env var is not set.
// USAGE: import { stellar } from '@/lib/stellar-helper'
// ============================================================
export const stellar = new StellarHelper(
  (import.meta.env.VITE_STELLAR_NETWORK as 'testnet' | 'mainnet') || 'testnet'
);

// ============================================================
// ERROR CLASSES
// PURPOSE: Custom error types for wallet operations.
//          Using specific error classes (instead of generic Error) lets the
//          error-handler.tsx show the right user-friendly message for each case.
//          Junior devs: think of these as "named error categories".
// ============================================================

// Thrown when Freighter extension is not installed or not connected.
// The error handler will show an "Install Freighter" button linking to freighter.app
export class WalletNotInstalledError extends Error {
  constructor(
    public walletName: string,  // e.g., 'Freighter'
    public installUrl: string   // e.g., 'https://www.freighter.app/'
  ) {
    super(`${walletName} wallet is not installed`);
    this.name = 'WalletNotInstalledError';
  }
} // end WalletNotInstalledError

// Thrown when wallet connection fails for a reason other than "not installed" or "rejected".
// e.g., extension crashed, unexpected API response
export class WalletConnectionError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'WalletConnectionError';
  }
} // end WalletConnectionError

// Thrown when the user explicitly rejects/cancels a wallet action.
// e.g., clicks "Deny" on the Freighter connection popup.
// The error handler shows a "Connection Cancelled" message (not an error, just info).
export class WalletRejectedError extends Error {
  constructor(
    public action: string  // What was rejected e.g., 'connection', 'signing'
  ) {
    super(`User rejected ${action}`);
    this.name = 'WalletRejectedError';
  }
} // end WalletRejectedError

// Thrown when a network/HTTP request fails.
// e.g., Horizon API is down, no internet connection
export class NetworkError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'NetworkError';
  }
} // end NetworkError

// Thrown when transaction signing fails.
// e.g., user rejects signing, Freighter returns empty XDR
export class SignatureError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'SignatureError';
  }
} // end SignatureError
