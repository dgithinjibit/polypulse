use soroban_sdk::{contracttype, Address, String, Vec};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BetState {
    Created,
    Active,
    Ended,
    Verified,
    Disputed,
    Paid,
    Cancelled,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct Bet {
    pub id: u64,
    pub creator: Address,
    pub question: String,
    pub stake_amount: i128,
    pub end_time: u64,
    pub state: BetState,
    pub created_at: u64,
    pub participants: Vec<Participant>,
    pub outcome_reports: Vec<OutcomeReport>,
    pub verified_outcome: Option<bool>,
    pub shareable_url_hash: String,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct Participant {
    pub address: Address,
    pub position: bool, // true = Yes, false = No
    pub stake: i128,
    pub joined_at: u64,
    pub has_reported: bool,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct OutcomeReport {
    pub reporter: Address,
    pub outcome: bool, // true = Yes, false = No
    pub reported_at: u64,
}
