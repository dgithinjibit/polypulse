// TypeScript types for wagers, participants, and chat messages

export type WagerStatus = 'pending' | 'accepted' | 'active' | 'resolved' | 'cancelled' | 'expired'

export type ResolutionMethod = 'ai_oracle' | 'trusted_judge' | 'social_consensus'

export type DefiProtocol = 'aave' | 'movement'

export interface Participant {
  id: string
  address: string
  displayName?: string
  joinedAt: string
  choice?: string
}

export interface Wager {
  id: string
  title: string
  description: string
  resolutionCriteria: string
  amount: number
  currency: string
  status: WagerStatus
  creatorAddress: string
  participants: Participant[]
  maxParticipants: number
  expiresAt: string
  createdAt: string
  resolvedAt?: string
  resolutionMethod: ResolutionMethod
  defiProtocol: DefiProtocol
  trustedJudgeAddress?: string
  isPublic: boolean
  shareLink: string
  currentYield?: number
  winnersAddresses?: string[]
}

export interface CreateWagerPayload {
  title: string
  description: string
  resolutionCriteria: string
  amount: number
  currency?: string
  maxParticipants?: number
  expiresAt: string
  resolutionMethod: ResolutionMethod
  defiProtocol: DefiProtocol
  trustedJudgeAddress?: string
  isPublic?: boolean
}

export interface WagerListItem {
  id: string
  title: string
  description: string
  amount: number
  currency: string
  status: WagerStatus
  participantCount: number
  maxParticipants: number
  expiresAt: string
  createdAt: string
  isPublic: boolean
  currentYield?: number
}

export interface ChatMessage {
  id: string
  wagerId: string
  senderAddress: string
  senderDisplayName?: string
  content: string
  sentAt: string
  editedAt?: string
  isDeleted?: boolean
}

export interface SendMessagePayload {
  wagerId: string
  content: string
}

export interface WagerPortfolio {
  activeWagers: WagerListItem[]
  resolvedWagers: WagerListItem[]
  totalAtRisk: number
  totalYieldEarned: number
  winRate: number
}
