// Network configuration for Stellar testnet and mainnet

import { ContractConfig } from '../types/contracts';

export const TESTNET_CONFIG: ContractConfig = {
  network: 'testnet',
  marketContractId: '', // Will be populated after deployment
  challengeContractId: '', // Will be populated after deployment
  horizonUrl: 'https://horizon-testnet.stellar.org',
  networkPassphrase: 'Test SDF Network ; September 2015',
};

export const MAINNET_CONFIG: ContractConfig = {
  network: 'mainnet',
  marketContractId: '', // Will be populated after deployment
  challengeContractId: '', // Will be populated after deployment
  horizonUrl: 'https://horizon.stellar.org',
  networkPassphrase: 'Public Global Stellar Network ; September 2015',
};

// Default to testnet for development
export const DEFAULT_CONFIG = TESTNET_CONFIG;

// Helper to get config based on environment
export function getNetworkConfig(): ContractConfig {
  const env = process.env.REACT_APP_STELLAR_NETWORK || 'testnet';
  return env === 'mainnet' ? MAINNET_CONFIG : TESTNET_CONFIG;
}
