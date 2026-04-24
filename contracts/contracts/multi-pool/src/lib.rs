#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, Vec, Symbol, symbol_short};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PoolParticipant {
    pub address: Address,
    pub stake: i128,
    pub joined_at: u64,
    pub payout_claimed: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PoolState {
    Open,       // Accepting participants
    Active,     // Event ongoing, no new participants
    Ended,      // Event ended, awaiting outcome
    Verified,   // Outcome verified
    Paid,       // Payouts executed
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MultiParticipantPool {
    pub pool_id: u64,
    pub creator: Address,
    pub question: Symbol,
    pub end_time: u64,
    pub state: PoolState,
    pub total_yes_stakes: i128,
    pub total_no_stakes: i128,
    pub verified_outcome: Option<bool>,
}

const PLATFORM_FEE_PERCENT: i128 = 7;

#[contract]
pub struct MultiPoolContract;

#[contractimpl]
impl MultiPoolContract {
    /// Create a new multi-participant pool
    pub fn create_pool(
        env: Env,
        creator: Address,
        question: Symbol,
        end_time: u64,
    ) -> u64 {
        creator.require_auth();
        
        let current_time = env.ledger().timestamp();
        if end_time <= current_time {
            panic!("End time must be in the future");
        }
        
        let pool_id = Self::get_next_pool_id(&env);
        
        let pool = MultiParticipantPool {
            pool_id,
            creator: creator.clone(),
            question,
            end_time,
            state: PoolState::Open,
            total_yes_stakes: 0,
            total_no_stakes: 0,
            verified_outcome: None,
        };
        
        env.storage().persistent().set(&Self::pool_key(pool_id), &pool);
        env.storage().persistent().set(&symbol_short!("next_id"), &(pool_id + 1));
        
        pool_id
    }
    
    /// Join pool with a position (true = Yes, false = No)
    pub fn join_pool(
        env: Env,
        participant: Address,
        pool_id: u64,
        position: bool,
        stake: i128,
    ) {
        participant.require_auth();
        
        if stake < 10_000_000 {
            panic!("Minimum stake is 1 XLM (10,000,000 stroops)");
        }
        
        let mut pool: MultiParticipantPool = env.storage()
            .persistent()
            .get(&Self::pool_key(pool_id))
            .expect("Pool not found");
        
        if pool.state != PoolState::Open {
            panic!("Pool is not accepting participants");
        }
        
        let current_time = env.ledger().timestamp();
        if current_time >= pool.end_time {
            panic!("Pool has ended");
        }
        
        // Update pool stakes
        if position {
            pool.total_yes_stakes += stake;
        } else {
            pool.total_no_stakes += stake;
        }
        
        // Store participant
        let participant_data = PoolParticipant {
            address: participant.clone(),
            stake,
            joined_at: current_time,
            payout_claimed: false,
        };
        
        let participants_key = Self::participants_key(pool_id, position);
        let mut participants: Vec<PoolParticipant> = env.storage()
            .persistent()
            .get(&participants_key)
            .unwrap_or(Vec::new(&env));
        
        participants.push_back(participant_data);
        
        env.storage().persistent().set(&participants_key, &participants);
        env.storage().persistent().set(&Self::pool_key(pool_id), &pool);
    }
    
    /// Get current odds (returns basis points: 10000 = 1.0x)
    pub fn get_odds(env: Env, pool_id: u64) -> (i128, i128) {
        let pool: MultiParticipantPool = env.storage()
            .persistent()
            .get(&Self::pool_key(pool_id))
            .expect("Pool not found");
        
        let total_pool = pool.total_yes_stakes + pool.total_no_stakes;
        
        if total_pool == 0 {
            return (10000, 10000); // 1.0x odds if no stakes
        }
        
        // Odds = total_pool / position_stakes (in basis points)
        let yes_odds = if pool.total_yes_stakes > 0 {
            (total_pool * 10000) / pool.total_yes_stakes
        } else {
            0
        };
        
        let no_odds = if pool.total_no_stakes > 0 {
            (total_pool * 10000) / pool.total_no_stakes
        } else {
            0
        };
        
        (yes_odds, no_odds)
    }
    
    /// Verify outcome (only creator can call)
    pub fn verify_outcome(
        env: Env,
        pool_id: u64,
        outcome: bool,
    ) {
        let mut pool: MultiParticipantPool = env.storage()
            .persistent()
            .get(&Self::pool_key(pool_id))
            .expect("Pool not found");
        
        pool.creator.require_auth();
        
        if pool.state != PoolState::Open && pool.state != PoolState::Active {
            panic!("Pool cannot be verified in current state");
        }
        
        pool.verified_outcome = Some(outcome);
        pool.state = PoolState::Verified;
        
        env.storage().persistent().set(&Self::pool_key(pool_id), &pool);
    }
    
    /// Calculate payout for a specific participant
    pub fn calculate_payout(
        env: Env,
        pool_id: u64,
        participant_address: Address,
        position: bool,
    ) -> i128 {
        let pool: MultiParticipantPool = env.storage()
            .persistent()
            .get(&Self::pool_key(pool_id))
            .expect("Pool not found");
        
        let outcome = pool.verified_outcome.expect("Outcome not verified");
        
        // Check if participant won
        if outcome != position {
            return 0; // Lost
        }
        
        // Find participant stake
        let participants_key = Self::participants_key(pool_id, position);
        let participants: Vec<PoolParticipant> = env.storage()
            .persistent()
            .get(&participants_key)
            .expect("No participants found");
        
        let mut user_stake = 0i128;
        for participant in participants.iter() {
            if participant.address == participant_address {
                user_stake = participant.stake;
                break;
            }
        }
        
        if user_stake == 0 {
            return 0;
        }
        
        // Calculate payout
        let total_pool = pool.total_yes_stakes + pool.total_no_stakes;
        let platform_fee = (total_pool * PLATFORM_FEE_PERCENT) / 100;
        let distributable = total_pool - platform_fee;
        
        let total_winning_stakes = if outcome {
            pool.total_yes_stakes
        } else {
            pool.total_no_stakes
        };
        
        // Proportional payout: (user_stake / total_winning_stakes) * distributable
        (user_stake * distributable) / total_winning_stakes
    }
    
    /// Distribute payouts to all winners
    pub fn distribute_payouts(
        env: Env,
        pool_id: u64,
    ) -> Vec<(Address, i128)> {
        let mut pool: MultiParticipantPool = env.storage()
            .persistent()
            .get(&Self::pool_key(pool_id))
            .expect("Pool not found");
        
        pool.creator.require_auth();
        
        if pool.state != PoolState::Verified {
            panic!("Pool outcome not verified");
        }
        
        let outcome = pool.verified_outcome.expect("Outcome not verified");
        
        let participants_key = Self::participants_key(pool_id, outcome);
        let participants: Vec<PoolParticipant> = env.storage()
            .persistent()
            .get(&participants_key)
            .unwrap_or(Vec::new(&env));
        
        let total_pool = pool.total_yes_stakes + pool.total_no_stakes;
        let platform_fee = (total_pool * PLATFORM_FEE_PERCENT) / 100;
        let distributable = total_pool - platform_fee;
        
        let total_winning_stakes = if outcome {
            pool.total_yes_stakes
        } else {
            pool.total_no_stakes
        };
        
        let mut payouts = Vec::new(&env);
        
        for participant in participants.iter() {
            let payout = (participant.stake * distributable) / total_winning_stakes;
            payouts.push_back((participant.address.clone(), payout));
        }
        
        pool.state = PoolState::Paid;
        env.storage().persistent().set(&Self::pool_key(pool_id), &pool);
        
        payouts
    }
    
    // Helper functions
    fn get_next_pool_id(env: &Env) -> u64 {
        env.storage()
            .persistent()
            .get(&symbol_short!("next_id"))
            .unwrap_or(1)
    }
    
    fn pool_key(pool_id: u64) -> Symbol {
        symbol_short!("pool")
    }
    
    fn participants_key(pool_id: u64, position: bool) -> Symbol {
        if position {
            symbol_short!("yes")
        } else {
            symbol_short!("no")
        }
    }
}

#[cfg(test)]
mod property_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Env};
    
    #[test]
    fn test_create_pool() {
        let env = Env::default();
        let contract_id = env.register_contract(None, MultiPoolContract);
        let client = MultiPoolContractClient::new(&env, &contract_id);
        
        let creator = Address::generate(&env);
        let question = symbol_short!("rain?");
        let end_time = env.ledger().timestamp() + 86400;
        
        let pool_id = client.create_pool(&creator, &question, &end_time);
        assert_eq!(pool_id, 1);
    }
    
    #[test]
    fn test_join_pool() {
        let env = Env::default();
        let contract_id = env.register_contract(None, MultiPoolContract);
        let client = MultiPoolContractClient::new(&env, &contract_id);
        
        let creator = Address::generate(&env);
        let participant = Address::generate(&env);
        let question = symbol_short!("rain?");
        let end_time = env.ledger().timestamp() + 86400;
        
        let pool_id = client.create_pool(&creator, &question, &end_time);
        
        client.join_pool(&participant, &pool_id, &true, &100_000_000);
        
        let (yes_odds, _) = client.get_odds(&pool_id);
        assert!(yes_odds > 0);
    }
    
    // Feature: polypulse-enhancements, Property 1: Multi-Participant Payout Fairness
    #[test]
    fn test_payout_fairness_simple() {
        let env = Env::default();
        let contract_id = env.register_contract(None, MultiPoolContract);
        let client = MultiPoolContractClient::new(&env, &contract_id);
        
        let creator = Address::generate(&env);
        let p1 = Address::generate(&env);
        let p2 = Address::generate(&env);
        let p3 = Address::generate(&env);
        
        let question = symbol_short!("test?");
        let end_time = env.ledger().timestamp() + 86400;
        
        let pool_id = client.create_pool(&creator, &question, &end_time);
        
        // Yes: 100 XLM + 200 XLM = 300 XLM
        client.join_pool(&p1, &pool_id, &true, &1_000_000_000); // 100 XLM
        client.join_pool(&p2, &pool_id, &true, &2_000_000_000); // 200 XLM
        
        // No: 300 XLM
        client.join_pool(&p3, &pool_id, &false, &3_000_000_000); // 300 XLM
        
        // Total pool: 600 XLM
        // Platform fee: 42 XLM (7%)
        // Distributable: 558 XLM
        
        // Verify outcome: Yes wins
        client.verify_outcome(&pool_id, &true);
        
        // Calculate payouts
        let payout1 = client.calculate_payout(&pool_id, &p1, &true);
        let payout2 = client.calculate_payout(&pool_id, &p2, &true);
        let payout3 = client.calculate_payout(&pool_id, &p3, &false);
        
        // P1 should get: (100/300) * 558 = 186 XLM
        // P2 should get: (200/300) * 558 = 372 XLM
        // P3 should get: 0 (lost)
        
        assert_eq!(payout1, 1_860_000_000); // 186 XLM
        assert_eq!(payout2, 3_720_000_000); // 372 XLM
        assert_eq!(payout3, 0);
        
        // Verify total payouts equal distributable
        assert_eq!(payout1 + payout2, 5_580_000_000); // 558 XLM
    }
}
