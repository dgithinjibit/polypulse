export enum BetState {
  Created = 'Created',
  Active = 'Active',
  Ended = 'Ended',
  Verified = 'Verified',
  Disputed = 'Disputed',
  Paid = 'Paid',
  Cancelled = 'Cancelled',
}

export interface Bet {
  id: string;
  creator: string;
  creatorUsername: string;
  question: string;
  stakeAmount: number;
  endTime: Date;
  state: BetState;
  createdAt: Date;
  shareableUrl: string;
  participants: Participant[];
  outcomeReports: OutcomeReport[];
  verifiedOutcome?: boolean;
  disputed: boolean;
}

export interface Participant {
  address: string;
  username: string;
  position: 'Yes' | 'No';
  stake: number;
  joinedAt: Date;
  hasReported: boolean;
}

export interface OutcomeReport {
  reporter: string;
  reporterUsername: string;
  outcome: boolean;
  reportedAt: Date;
}

export interface BetUpdate {
  betId: string;
  type: 'participant_joined' | 'outcome_reported' | 'outcome_verified' | 'disputed' | 'paid';
  data: any;
  timestamp: Date;
}

export interface Position {
  betId: string;
  question: string;
  position: 'Yes' | 'No';
  stake: number;
  currentValue: number;
  profitLoss: number;
  status: BetState;
}
