use soroban_sdk::{contracttype, Address, String, Vec};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PoolState {
    Open,       // Accepting participants
    Active,     // Event ongoing
    Ended,      // Event ended, awaiting outcome
    Verified,   // Outcome verified
    Paid,       // Payouts executed
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct MultiParticipantPool {
    pub id: u64,
    pub creator: Address,
    pub question: String,
    pub end_time: u64,
    pub state: PoolState,
    pub created_at: u64,
    pub yes_participants: Vec<PoolParticipant>,
    pub no_participants: Vec<PoolParticipant>,
    pub total_yes_stakes: i128,
    pub total_no_stakes: i128,
    pub verified_outcome: Option<bool>,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct PoolParticipant {
    pub address: Address,
    pub stake: i128,
    pub joined_at: u64,
    pub payout_claimed: bool,
}
