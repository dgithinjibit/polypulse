// TypeScript type definitions for Soroban contracts
// These types mirror the Rust contract types

export enum MarketState {
  Open = 'Open',
  Closed = 'Closed',
  Resolved = 'Resolved',
  Cancelled = 'Cancelled',
}

export interface Market {
  id: number;
  creator: string;
  title: string;
  description: string;
  options: string[];
  liquidityB: number;
  sharesOutstanding: number[];
  state: MarketState;
  createdAt: number;
  closesAt: number;
  resolvedAt?: number;
  winningOptionId?: number;
  resolutionCriteria: string;
}

export interface OptionPosition {
  optionId: number;
  shares: number;
  amountSpent: string; // i128 as string
}

export interface Position {
  user: string;
  marketId: number;
  optionShares: OptionPosition[];
}

export interface BuyResult {
  sharesIssued: number;
  newPrice: number;
}

export interface SellResult {
  xlmRefund: string; // i128 as string
}

export enum ChallengeState {
  Pending = 'Pending',
  Accepted = 'Accepted',
  Resolved = 'Resolved',
  Cancelled = 'Cancelled',
  Expired = 'Expired',
}

export interface Challenge {
  id: number;
  creator: string;
  opponent?: string;
  question: string;
  xlmStake: string; // i128 as string
  creatorChoice: string;
  state: ChallengeState;
  isOpen: boolean;
  createdAt: number;
  expiresAt: number;
  resolvedAt?: number;
  winner?: string;
  resolutionCriteria: string;
}

export interface TransactionResult {
  success: boolean;
  transactionHash?: string;
  error?: string;
}

export interface ContractConfig {
  network: 'testnet' | 'mainnet';
  marketContractId: string;
  challengeContractId: string;
  horizonUrl: string;
  networkPassphrase: string;
}
