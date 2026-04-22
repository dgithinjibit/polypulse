/**
 * ============================================================
 * FILE: WalletConnection.tsx
 * PURPOSE: A reusable wallet connection UI component.
 *          Shows a "Connect Wallet" button when disconnected.
 *          Shows the connected wallet address with copy and disconnect options when connected.
 *          Uses the stellar helper directly (not the context) for simplicity.
 *
 * USED IN: Login.tsx (and potentially other pages that need inline wallet connection)
 *
 * PROPS:
 *   - onConnect: callback called with the public key after successful connection
 *   - onDisconnect: callback called after disconnection
 *
 * JUNIOR DEV NOTE:
 *   'use client' at the top is a Next.js directive. In Vite/React it's ignored,
 *   but it's kept for compatibility if the project ever migrates to Next.js.
 *   It means "this component runs in the browser, not on the server".
 * ============================================================
 */

'use client'; // Next.js directive - ignored in Vite, kept for compatibility

// useState: manages local component state (publicKey, isConnected, loading, copied)
// useEffect: runs side effects (checking existing connection on mount)
import { useState, useEffect } from 'react';

// stellar: the StellarHelper singleton for blockchain operations
import { stellar } from '@/lib/stellar-helper';

// React Icons - wallet, copy, and checkmark icons for the UI
import { FaWallet, FaCopy, FaCheck } from 'react-icons/fa';

// Logout icon for the disconnect button
import { MdLogout } from 'react-icons/md';

// ============================================================
// TYPE: WalletConnectionProps
// PURPOSE: Defines the callback props this component requires.
// ============================================================
interface WalletConnectionProps {
  onConnect: (publicKey: string) => void;    // Called with public key after successful connection
  onDisconnect: () => void;                  // Called after wallet is disconnected
} // end WalletConnectionProps

// ============================================================
// COMPONENT: WalletConnection
// PURPOSE: Renders wallet connection UI with connect/disconnect functionality.
// ============================================================
export default function WalletConnection({ onConnect, onDisconnect }: WalletConnectionProps) {
  // The connected wallet's full public key (empty string = not connected)
  const [publicKey, setPublicKey] = useState<string>('');

  // Whether the wallet is currently connected
  const [isConnected, setIsConnected] = useState(false);

  // True while the connection request is in progress (shows "Connecting..." text)
  const [loading, setLoading] = useState(false);

  // True for 2 seconds after copying address (shows checkmark instead of copy icon)
  const [copied, setCopied] = useState(false);

  // ============================================================
  // EFFECT: Check for existing wallet connection on component mount
  // PURPOSE: When this component first renders, silently check if the user
  //          already has a connected wallet (e.g., from a previous session).
  //          If so, restore the connected state without showing a popup.
  // RUNS: Once when component mounts (empty dependency array)
  // ============================================================
  useEffect(() => {
    checkExistingConnection();
  }, []); // end useEffect

  // ============================================================
  // FUNCTION: checkExistingConnection
  // PURPOSE: Silently checks if Freighter already has an address available.
  //          Uses stellar.getPublicKey() which does NOT show a popup.
  //          If an address is found, updates state to show connected UI.
  // ============================================================
  const checkExistingConnection = async () => {
    // getPublicKey() returns null if not connected, or the address if connected
    // We use the helper's method instead of accessing private 'kit'
    const address = await stellar.getPublicKey();

    if (address) {
      // Wallet is already connected - restore connected state
      setPublicKey(address);
      setIsConnected(true);
      // Notify parent component of the existing connection
      onConnect(address);
    } // end if address
  }; // end checkExistingConnection

  // ============================================================
  // FUNCTION: handleConnect
  // PURPOSE: Initiates the wallet connection flow when user clicks "Connect Wallet".
  //          Calls stellar.connectWallet() which shows the Freighter popup.
  //          Updates state on success, shows alert on unexpected errors.
  // ============================================================
  const handleConnect = async () => {
    try {
      // Show loading state on the button
      setLoading(true);

      // Connect wallet - shows Freighter popup for user to approve
      const key = await stellar.connectWallet();

      // Connection successful - update state
      setPublicKey(key);
      setIsConnected(true);

      // Notify parent component with the new public key
      onConnect(key);
    } catch (error: any) {
      console.error('Connection error:', error);

      // Don't show an alert if user simply cancelled/rejected the connection.
      // "cancelled" and "rejected" are expected user actions, not errors.
      if (!error.message?.includes('cancelled') && !error.message?.includes('rejected')) {
        // Show a simple alert for unexpected errors
        // TODO: Replace with toast notification for better UX
        alert(`Failed to connect wallet:\n${error.message}`);
      } // end if not cancelled
    } finally {
      // Always clear loading state, whether connection succeeded or failed
      setLoading(false);
    } // end try/catch/finally
  }; // end handleConnect

  // ============================================================
  // FUNCTION: handleDisconnect
  // PURPOSE: Disconnects the wallet and resets component state.
  //          Calls stellar.disconnect() to clear the cached public key.
  //          Notifies parent component via onDisconnect callback.
  // ============================================================
  const handleDisconnect = () => {
    // Clear the stellar helper's cached public key
    stellar.disconnect();

    // Reset local state to disconnected
    setPublicKey('');
    setIsConnected(false);

    // Notify parent component that wallet was disconnected
    onDisconnect();
  }; // end handleDisconnect

  // ============================================================
  // FUNCTION: handleCopyAddress
  // PURPOSE: Copies the full wallet address to the clipboard.
  //          Shows a checkmark icon for 2 seconds to confirm the copy.
  // ============================================================
  const handleCopyAddress = async () => {
    // Guard: nothing to copy if no address
    if (!publicKey) return;

    // Copy to clipboard using the browser's Clipboard API
    await navigator.clipboard.writeText(publicKey);

    // Show checkmark icon to confirm copy
    setCopied(true);

    // Reset back to copy icon after 2 seconds
    setTimeout(() => setCopied(false), 2000);
  }; // end handleCopyAddress

  // ============================================================
  // RENDER: Disconnected State
  // Shows the "Connect Wallet" button when no wallet is connected.
  // ============================================================
  if (!isConnected) {
    return (
      <button
        onClick={handleConnect}
        disabled={loading}  // Disable button while connection is in progress
        className="w-full flex items-center justify-center gap-3 bg-gradient-polypulse-hero hover:opacity-90 text-white font-semibold py-3.5 px-4 rounded-xl transition-all disabled:opacity-50 disabled:cursor-not-allowed shadow-lg focus:outline-none focus:ring-2 focus:ring-purple-500 focus:ring-offset-2 min-h-[48px] touch-manipulation"
        aria-label="Connect Stellar wallet"
      >
        {loading ? (
          // Loading state: spinning circle + "Connecting..." text
          <>
            <span className="animate-spin rounded-full h-4 w-4 border-b-2 border-white" />
            <span>Connecting...</span>
          </>
        ) : (
          // Default state: wallet icon + "Connect Wallet" text
          <>
            <FaWallet />
            <span>Connect Wallet</span>
          </>
        )}
      </button>
    );
  } // end disconnected render

  // ============================================================
  // RENDER: Connected State
  // Shows the wallet address with copy and disconnect options.
  // ============================================================
  return (
    <div className="flex items-center justify-between p-3 bg-gray-50 rounded-xl border border-gray-100">
      {/* Left side: green dot + truncated address + copy button */}
      <div className="flex items-center gap-2">
        {/* Green dot indicates connected status */}
        <span className="text-green-500 text-sm">●</span>

        {/* Truncated wallet address - max-w-[200px] prevents overflow */}
        <p className="font-mono text-sm text-gray-700 truncate max-w-[200px]">{publicKey}</p>

        {/* Copy button: shows checkmark for 2s after copying */}
        <button onClick={handleCopyAddress} className="text-gray-400 hover:text-purple-600 transition-colors">
          {copied ? <FaCheck className="text-green-500" /> : <FaCopy />}
        </button>
      </div>

      {/* Right side: disconnect button */}
      <button onClick={handleDisconnect} className="text-gray-400 hover:text-red-500 transition-colors flex items-center gap-1 text-sm">
        <MdLogout /> Disconnect
      </button>
    </div>
  );
} // end WalletConnection
