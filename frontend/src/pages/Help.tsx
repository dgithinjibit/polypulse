import React from 'react'
import { Link } from 'react-router-dom'
import LegalPageNav from '../components/LegalPageNav'

export default function Help() {
  return (
    <div className="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
      <LegalPageNav currentPage="faq" />
      <h1 className="text-3xl font-bold text-white mb-8">Help & Support</h1>

      {/* Wallet Connection Issues */}
      <section className="bg-white/10 backdrop-blur-sm rounded-xl p-6 mb-6">
        <h2 className="text-xl font-semibold text-white mb-4">Wallet Connection Issues</h2>
        
        <div className="space-y-4">
          <div>
            <h3 className="text-lg font-medium text-purple-200 mb-2">Freighter Wallet Not Installed</h3>
            <p className="text-gray-300 mb-2">
              If you see a "Wallet Not Installed" error, you need to install the Freighter wallet extension.
            </p>
            <ol className="list-decimal list-inside text-gray-300 space-y-1 ml-4">
              <li>Visit <a href="https://www.freighter.app/" target="_blank" rel="noopener noreferrer" className="text-purple-400 hover:underline">freighter.app</a></li>
              <li>Click "Install" for your browser (Chrome, Firefox, Edge, or Brave)</li>
              <li>Follow the installation instructions</li>
              <li>Create or import a Stellar wallet</li>
              <li>Return to PolyPulse and click "Connect Wallet"</li>
            </ol>
          </div>

          <div>
            <h3 className="text-lg font-medium text-purple-200 mb-2">Connection Failed</h3>
            <p className="text-gray-300 mb-2">
              If the wallet connection fails, try these steps:
            </p>
            <ul className="list-disc list-inside text-gray-300 space-y-1 ml-4">
              <li>Make sure Freighter is unlocked (click the extension icon and enter your password)</li>
              <li>Refresh the page and try connecting again</li>
              <li>Check if Freighter is up to date (update if needed)</li>
              <li>Try disabling other wallet extensions temporarily</li>
              <li>Clear your browser cache and cookies</li>
            </ul>
          </div>

          <div>
            <h3 className="text-lg font-medium text-purple-200 mb-2">Connection Cancelled</h3>
            <p className="text-gray-300">
              If you accidentally cancelled the connection, simply click "Connect Wallet" again and approve the connection in the Freighter popup.
            </p>
          </div>
        </div>
      </section>

      {/* Authentication Issues */}
      <section className="bg-white/10 backdrop-blur-sm rounded-xl p-6 mb-6">
        <h2 className="text-xl font-semibold text-white mb-4">Authentication Issues</h2>
        
        <div className="space-y-4">
          <div>
            <h3 className="text-lg font-medium text-purple-200 mb-2">Why Do I Need to Sign?</h3>
            <p className="text-gray-300 mb-2">
              When you connect your wallet, we ask you to sign a message to prove you own the wallet address. This is a security measure that:
            </p>
            <ul className="list-disc list-inside text-gray-300 space-y-1 ml-4">
              <li>Doesn't cost any XLM (it's not a transaction)</li>
              <li>Doesn't give us access to your funds</li>
              <li>Only proves you control the wallet address</li>
              <li>Prevents others from impersonating you</li>
            </ul>
          </div>

          <div>
            <h3 className="text-lg font-medium text-purple-200 mb-2">Signature Failed</h3>
            <p className="text-gray-300 mb-2">
              If signature fails, try:
            </p>
            <ul className="list-disc list-inside text-gray-300 space-y-1 ml-4">
              <li>Make sure Freighter is unlocked</li>
              <li>Check your internet connection</li>
              <li>Try connecting again from the beginning</li>
              <li>Update Freighter to the latest version</li>
            </ul>
          </div>
        </div>
      </section>

      {/* Network Issues */}
      <section className="bg-white/10 backdrop-blur-sm rounded-xl p-6 mb-6">
        <h2 className="text-xl font-semibold text-white mb-4">Network Issues</h2>
        
        <div className="space-y-4">
          <div>
            <h3 className="text-lg font-medium text-purple-200 mb-2">Network Error</h3>
            <p className="text-gray-300 mb-2">
              If you see network errors:
            </p>
            <ul className="list-disc list-inside text-gray-300 space-y-1 ml-4">
              <li>Check your internet connection</li>
              <li>Try refreshing the page</li>
              <li>Check if you're behind a firewall or VPN that might block connections</li>
              <li>Wait a few minutes and try again</li>
            </ul>
          </div>

          <div>
            <h3 className="text-lg font-medium text-purple-200 mb-2">Stellar Network Unavailable</h3>
            <p className="text-gray-300 mb-2">
              If the Stellar network is down:
            </p>
            <ul className="list-disc list-inside text-gray-300 space-y-1 ml-4">
              <li>Check <a href="https://status.stellar.org/" target="_blank" rel="noopener noreferrer" className="text-purple-400 hover:underline">status.stellar.org</a> for network status</li>
              <li>Wait for the network to come back online</li>
              <li>Your funds are safe - the network will resume normal operations</li>
            </ul>
          </div>
        </div>
      </section>

      {/* Transaction Issues */}
      <section className="bg-white/10 backdrop-blur-sm rounded-xl p-6 mb-6">
        <h2 className="text-xl font-semibold text-white mb-4">Transaction Issues</h2>
        
        <div className="space-y-4">
          <div>
            <h3 className="text-lg font-medium text-purple-200 mb-2">Insufficient Balance</h3>
            <p className="text-gray-300 mb-2">
              If you don't have enough XLM:
            </p>
            <ul className="list-disc list-inside text-gray-300 space-y-1 ml-4">
              <li>Check your balance in the <Link to="/wallet" className="text-purple-400 hover:underline">Wallet</Link> page</li>
              <li>You need at least 1 XLM minimum balance (Stellar requirement)</li>
              <li>Add funds to your wallet using an exchange or faucet (testnet)</li>
              <li>Remember to account for transaction fees (usually 0.00001 XLM)</li>
            </ul>
          </div>

          <div>
            <h3 className="text-lg font-medium text-purple-200 mb-2">Transaction Failed</h3>
            <p className="text-gray-300 mb-2">
              If a transaction fails:
            </p>
            <ul className="list-disc list-inside text-gray-300 space-y-1 ml-4">
              <li>Check if you have sufficient balance</li>
              <li>Make sure you approved the transaction in Freighter</li>
              <li>Wait a moment and try again</li>
              <li>Check the transaction details for errors</li>
            </ul>
          </div>
        </div>
      </section>

      {/* Funding Your Wallet */}
      <section className="bg-white/10 backdrop-blur-sm rounded-xl p-6 mb-6">
        <h2 className="text-xl font-semibold text-white mb-4">Funding Your Wallet</h2>
        
        <div className="space-y-4">
          <div>
            <h3 className="text-lg font-medium text-purple-200 mb-2">Testnet (For Testing)</h3>
            <p className="text-gray-300 mb-2">
              If you're using the testnet version:
            </p>
            <ol className="list-decimal list-inside text-gray-300 space-y-1 ml-4">
              <li>Copy your Stellar address from the <Link to="/wallet" className="text-purple-400 hover:underline">Wallet</Link> page</li>
              <li>Visit <a href="https://laboratory.stellar.org/#account-creator" target="_blank" rel="noopener noreferrer" className="text-purple-400 hover:underline">Stellar Laboratory</a></li>
              <li>Paste your address and click "Get test network lumens"</li>
              <li>Wait a few seconds for the XLM to arrive</li>
            </ol>
          </div>

          <div>
            <h3 className="text-lg font-medium text-purple-200 mb-2">Mainnet (Real Money)</h3>
            <p className="text-gray-300 mb-2">
              To fund your mainnet wallet:
            </p>
            <ul className="list-disc list-inside text-gray-300 space-y-1 ml-4">
              <li>Buy XLM from a cryptocurrency exchange (Coinbase, Kraken, Binance, etc.)</li>
              <li>Withdraw XLM to your Stellar address</li>
              <li>Make sure to include the memo if required by the exchange</li>
              <li>Wait for the transaction to confirm (usually 5-10 seconds)</li>
            </ul>
          </div>
        </div>
      </section>

      {/* Still Need Help? */}
      <section className="bg-white/10 backdrop-blur-sm rounded-xl p-6">
        <h2 className="text-xl font-semibold text-white mb-4">Still Need Help?</h2>
        <p className="text-gray-300 mb-4">
          If you're still experiencing issues, please contact our support team:
        </p>
        <ul className="list-disc list-inside text-gray-300 space-y-2 ml-4">
          <li>Email: <a href="mailto:supportpolypulse@gmail.com" className="text-purple-400 hover:underline">supportpolypulse@gmail.com</a></li>
          <li>Twitter: <a href="https://twitter.com/polypulse" target="_blank" rel="noopener noreferrer" className="text-purple-400 hover:underline">@polypulse</a></li>
          <li>Discord: <a href="https://discord.gg/polypulse" target="_blank" rel="noopener noreferrer" className="text-purple-400 hover:underline">Join our community</a></li>
        </ul>
        <p className="text-gray-300 mt-4">
          When reporting an issue, please include:
        </p>
        <ul className="list-disc list-inside text-gray-300 space-y-1 ml-4">
          <li>What you were trying to do</li>
          <li>The exact error message you saw</li>
          <li>Your browser and version</li>
          <li>Screenshots if possible</li>
        </ul>
      </section>
    </div>
  )
}
