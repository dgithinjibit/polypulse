#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, String, Vec};

mod storage;
mod types;

pub use types::*;

#[contract]
pub struct P2PBetContract;

#[contractimpl]
impl P2PBetContract {
    /// Initialize the contract with admin address
    pub fn initialize(env: Env, admin: Address) {
        storage::set_admin(&env, &admin);
    }

    /// Create a new bet
    pub fn create_bet(
        env: Env,
        creator: Address,
        question: String,
        stake_amount: i128,
        end_time: u64,
        shareable_url_hash: String,
    ) -> u64 {
        creator.require_auth();
        
        // Validate inputs
        if question.len() == 0 {
            panic!("Question cannot be empty");
        }
        if stake_amount <= 0 {
            panic!("Stake amount must be positive");
        }
        let current_time = env.ledger().timestamp();
        if end_time <= current_time {
            panic!("End time must be in the future");
        }

        // Get next bet ID
        let bet_id = storage::get_next_bet_id(&env);
        
        // Create bet
        let bet = Bet {
            id: bet_id,
            creator: creator.clone(),
            question: question.clone(),
            stake_amount,
            end_time,
            state: BetState::Created,
            created_at: current_time,
            participants: Vec::new(&env),
            outcome_reports: Vec::new(&env),
            verified_outcome: None,
            shareable_url_hash: shareable_url_hash.clone(),
        };
        
        // Store bet
        storage::set_bet(&env, bet_id, &bet);
        
        // Lock creator stake (transfer XLM to contract)
        // Note: In production, this would transfer XLM from creator to contract
        // For now, we just record the stake
        
        // Emit event
        env.events().publish(
            (String::from_str(&env, "bet_created"),),
            (bet_id, creator, question, stake_amount, end_time),
        );
        
        bet_id
    }

    /// Join an existing bet
    pub fn join_bet(
        env: Env,
        participant: Address,
        bet_id: u64,
        position: bool,
        stake: i128,
    ) {
        participant.require_auth();
        
        // Get bet
        let mut bet = storage::get_bet(&env, bet_id)
            .unwrap_or_else(|| panic!("Bet not found"));
        
        // Validate bet state
        if bet.state != BetState::Created && bet.state != BetState::Active {
            panic!("Bet is not accepting participants");
        }
        
        // Validate end time not passed
        let current_time = env.ledger().timestamp();
        if current_time >= bet.end_time {
            panic!("Bet has ended");
        }
        
        // Validate stake
        if stake <= 0 {
            panic!("Stake must be positive");
        }
        
        // Check if participant already joined
        for p in bet.participants.iter() {
            if p.address == participant {
                panic!("Already a participant");
            }
        }
        
        // Add participant
        let new_participant = Participant {
            address: participant.clone(),
            position,
            stake,
            joined_at: current_time,
            has_reported: false,
        };
        bet.participants.push_back(new_participant);
        
        // Update bet state to Active
        bet.state = BetState::Active;
        
        // Store updated bet
        storage::set_bet(&env, bet_id, &bet);
        
        // Lock participant stake
        // Note: In production, transfer XLM from participant to contract
        
        // Emit event
        env.events().publish(
            (String::from_str(&env, "participant_joined"),),
            (bet_id, participant, position, stake),
        );
    }

    /// Cancel a bet (only creator, only if no participants)
    pub fn cancel_bet(env: Env, creator: Address, bet_id: u64) {
        creator.require_auth();
        
        let mut bet = storage::get_bet(&env, bet_id)
            .unwrap_or_else(|| panic!("Bet not found"));
        
        // Validate creator
        if bet.creator != creator {
            panic!("Only creator can cancel");
        }
        
        // Validate no participants
        if bet.participants.len() > 0 {
            panic!("Cannot cancel bet with participants");
        }
        
        // Set state to Cancelled
        bet.state = BetState::Cancelled;
        storage::set_bet(&env, bet_id, &bet);
        
        // Refund creator stake
        // Note: In production, transfer XLM back to creator
        
        // Emit event
        env.events().publish(
            (String::from_str(&env, "bet_cancelled"),),
            (bet_id, creator),
        );
    }

    /// Report outcome (first reporter)
    pub fn report_outcome(env: Env, reporter: Address, bet_id: u64, outcome: bool) {
        reporter.require_auth();
        
        let mut bet = storage::get_bet(&env, bet_id)
            .unwrap_or_else(|| panic!("Bet not found"));
        
        // Validate end time has passed
        let current_time = env.ledger().timestamp();
        if current_time < bet.end_time {
            panic!("Bet has not ended yet");
        }
        
        // Validate reporter is participant
        let mut is_participant = false;
        let mut already_reported = false;
        for p in bet.participants.iter() {
            if p.address == reporter {
                is_participant = true;
                if p.has_reported {
                    already_reported = true;
                }
                break;
            }
        }
        
        if !is_participant {
            panic!("Only participants can report outcome");
        }
        if already_reported {
            panic!("Already reported outcome");
        }
        
        // Add outcome report
        let report = OutcomeReport {
            reporter: reporter.clone(),
            outcome,
            reported_at: current_time,
        };
        bet.outcome_reports.push_back(report);
        
        // Mark participant as reported
        let mut updated_participants = Vec::new(&env);
        for p in bet.participants.iter() {
            let mut participant = p;
            if participant.address == reporter {
                participant.has_reported = true;
            }
            updated_participants.push_back(participant);
        }
        bet.participants = updated_participants;
        
        // Update bet state
        bet.state = BetState::Ended;
        
        storage::set_bet(&env, bet_id, &bet);
        
        // Emit event
        env.events().publish(
            (String::from_str(&env, "outcome_reported"),),
            (bet_id, reporter, outcome),
        );
    }

    /// Confirm outcome (verifiers)
    pub fn confirm_outcome(env: Env, verifier: Address, bet_id: u64, outcome: bool) {
        verifier.require_auth();
        
        let mut bet = storage::get_bet(&env, bet_id)
            .unwrap_or_else(|| panic!("Bet not found"));
        
        // Validate verifier is participant
        let mut is_participant = false;
        let mut already_reported = false;
        for p in bet.participants.iter() {
            if p.address == verifier {
                is_participant = true;
                if p.has_reported {
                    already_reported = true;
                }
                break;
            }
        }
        
        if !is_participant {
            panic!("Only participants can confirm outcome");
        }
        if already_reported {
            panic!("Already reported outcome");
        }
        
        // Add outcome report
        let report = OutcomeReport {
            reporter: verifier.clone(),
            outcome,
            reported_at: env.ledger().timestamp(),
        };
        bet.outcome_reports.push_back(report);
        
        // Mark participant as reported
        let mut updated_participants = Vec::new(&env);
        for p in bet.participants.iter() {
            let mut participant = p;
            if participant.address == verifier {
                participant.has_reported = true;
            }
            updated_participants.push_back(participant);
        }
        bet.participants = updated_participants;
        
        // Check if all participants have reported
        let all_reported = bet.participants.iter().all(|p| p.has_reported);
        
        if all_reported {
            // Check if all agree on outcome
            let first_outcome = bet.outcome_reports.get(0).unwrap().outcome;
            let all_agree = bet.outcome_reports.iter().all(|r| r.outcome == first_outcome);
            
            if all_agree {
                // Outcome verified
                bet.state = BetState::Verified;
                bet.verified_outcome = Some(first_outcome);
                
                // Execute payout
                Self::execute_payout_internal(env.clone(), &mut bet);
                
                env.events().publish(
                    (String::from_str(&env, "outcome_verified"),),
                    (bet_id, first_outcome),
                );
            } else {
                // Dispute
                bet.state = BetState::Disputed;
                
                env.events().publish(
                    (String::from_str(&env, "bet_disputed"),),
                    (bet_id,),
                );
            }
        }
        
        storage::set_bet(&env, bet_id, &bet);
        
        // Emit confirmation event
        env.events().publish(
            (String::from_str(&env, "outcome_confirmed"),),
            (bet_id, verifier, outcome),
        );
    }

    /// Execute payout (internal, called after verification)
    fn execute_payout_internal(env: Env, bet: &mut Bet) {
        if bet.verified_outcome.is_none() {
            panic!("Outcome not verified");
        }
        
        let winning_outcome = bet.verified_outcome.unwrap();
        
        // Calculate total pool
        let mut total_pool = bet.stake_amount;
        for p in bet.participants.iter() {
            total_pool += p.stake;
        }
        
        // Find winners
        let mut winners = Vec::new(&env);
        for p in bet.participants.iter() {
            if p.position == winning_outcome {
                winners.push_back(p);
            }
        }
        
        if winners.len() == 0 {
            // No winners, refund all
            // Note: In production, refund all participants
            return;
        }
        
        // Calculate payout per winner
        let payout_per_winner = total_pool / (winners.len() as i128);
        
        // Collect fee and payout winners
        for winner in winners.iter() {
            let payout_after_fee = Self::collect_fee_internal(env.clone(), payout_per_winner);
            
            // Transfer payout to winner
            // Note: In production, transfer XLM to winner
            
            env.events().publish(
                (String::from_str(&env, "payout_executed"),),
                (bet.id, winner.address.clone(), payout_after_fee),
            );
        }
        
        bet.state = BetState::Paid;
    }

    /// Collect platform fee (2%)
    fn collect_fee_internal(env: Env, amount: i128) -> i128 {
        let fee = amount * 2 / 100; // 2% fee
        let amount_after_fee = amount - fee;
        
        // Transfer fee to treasury
        // Note: In production, transfer fee to treasury address
        
        env.events().publish(
            (String::from_str(&env, "fee_collected"),),
            (fee,),
        );
        
        amount_after_fee
    }

    /// Admin resolve dispute
    pub fn admin_resolve_dispute(
        env: Env,
        admin: Address,
        bet_id: u64,
        winning_outcome: bool,
    ) {
        admin.require_auth();
        
        // Validate admin
        let stored_admin = storage::get_admin(&env);
        if admin != stored_admin {
            panic!("Only admin can resolve disputes");
        }
        
        let mut bet = storage::get_bet(&env, bet_id)
            .unwrap_or_else(|| panic!("Bet not found"));
        
        // Validate bet is disputed
        if bet.state != BetState::Disputed {
            panic!("Bet is not disputed");
        }
        
        // Set verified outcome
        bet.verified_outcome = Some(winning_outcome);
        bet.state = BetState::Verified;
        
        // Execute payout
        Self::execute_payout_internal(env.clone(), &mut bet);
        
        storage::set_bet(&env, bet_id, &bet);
        
        // Emit event
        env.events().publish(
            (String::from_str(&env, "dispute_resolved"),),
            (bet_id, winning_outcome),
        );
    }

    /// Get bet details
    pub fn get_bet(env: Env, bet_id: u64) -> Option<Bet> {
        storage::get_bet(&env, bet_id)
    }

    /// Get participant details
    pub fn get_participant(env: Env, bet_id: u64, address: Address) -> Option<Participant> {
        let bet = storage::get_bet(&env, bet_id)?;
        
        for p in bet.participants.iter() {
            if p.address == address {
                return Some(p);
            }
        }
        
        None
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{
        testutils::{Address as _, Ledger},
        Env,
    };

    fn setup_env() -> (Env, Address, Address, Address) {
        let env = Env::default();
        env.mock_all_auths();
        // Set initial timestamp
        env.ledger().with_mut(|l| l.timestamp = 1_000_000);
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let participant = Address::generate(&env);
        (env, admin, creator, participant)
    }

    // ─── Bet State Tests ──────────────────────────────────────────────────────

    #[test]
    fn test_bet_state_transitions() {
        let (env, _, creator, participant) = setup_env();
        
        // Created state
        let mut bet = Bet {
            id: 1,
            creator: creator.clone(),
            question: String::from_str(&env, "Will it rain?"),
            stake_amount: 1_000_000i128,
            end_time: 2_000_000u64,
            state: BetState::Created,
            created_at: 1_000_000u64,
            participants: Vec::new(&env),
            outcome_reports: Vec::new(&env),
            verified_outcome: None,
            shareable_url_hash: String::from_str(&env, "abc123"),
        };
        
        assert_eq!(bet.state, BetState::Created);
        
        // Add participant -> Active
        let p = Participant {
            address: participant.clone(),
            position: true,
            stake: 500_000i128,
            joined_at: 1_000_000u64,
            has_reported: false,
        };
        bet.participants.push_back(p);
        bet.state = BetState::Active;
        assert_eq!(bet.state, BetState::Active);
        
        // Report outcome -> Ended
        bet.state = BetState::Ended;
        assert_eq!(bet.state, BetState::Ended);
        
        // Verify outcome -> Verified
        bet.state = BetState::Verified;
        bet.verified_outcome = Some(true);
        assert_eq!(bet.state, BetState::Verified);
        assert_eq!(bet.verified_outcome, Some(true));
        
        // Payout -> Paid
        bet.state = BetState::Paid;
        assert_eq!(bet.state, BetState::Paid);
    }

    #[test]
    fn test_bet_state_disputed() {
        let (env, _, creator, _) = setup_env();
        
        let mut bet = Bet {
            id: 1,
            creator,
            question: String::from_str(&env, "Will it rain?"),
            stake_amount: 1_000_000i128,
            end_time: 2_000_000u64,
            state: BetState::Ended,
            created_at: 1_000_000u64,
            participants: Vec::new(&env),
            outcome_reports: Vec::new(&env),
            verified_outcome: None,
            shareable_url_hash: String::from_str(&env, "abc123"),
        };
        
        bet.state = BetState::Disputed;
        assert_eq!(bet.state, BetState::Disputed);
    }

    #[test]
    fn test_bet_state_cancelled() {
        let (env, _, creator, _) = setup_env();
        
        let mut bet = Bet {
            id: 1,
            creator,
            question: String::from_str(&env, "Will it rain?"),
            stake_amount: 1_000_000i128,
            end_time: 2_000_000u64,
            state: BetState::Created,
            created_at: 1_000_000u64,
            participants: Vec::new(&env),
            outcome_reports: Vec::new(&env),
            verified_outcome: None,
            shareable_url_hash: String::from_str(&env, "abc123"),
        };
        
        bet.state = BetState::Cancelled;
        assert_eq!(bet.state, BetState::Cancelled);
    }

    // ─── Participant Tests ────────────────────────────────────────────────────

    #[test]
    fn test_add_single_participant() {
        let (env, _, creator, participant) = setup_env();
        
        let mut bet = Bet {
            id: 1,
            creator,
            question: String::from_str(&env, "Will it rain?"),
            stake_amount: 1_000_000i128,
            end_time: 2_000_000u64,
            state: BetState::Created,
            created_at: 1_000_000u64,
            participants: Vec::new(&env),
            outcome_reports: Vec::new(&env),
            verified_outcome: None,
            shareable_url_hash: String::from_str(&env, "abc123"),
        };
        
        let p = Participant {
            address: participant.clone(),
            position: true,
            stake: 500_000i128,
            joined_at: 1_000_000u64,
            has_reported: false,
        };
        
        bet.participants.push_back(p);
        
        assert_eq!(bet.participants.len(), 1);
        assert_eq!(bet.participants.get(0).unwrap().address, participant);
        assert_eq!(bet.participants.get(0).unwrap().stake, 500_000i128);
    }

    #[test]
    fn test_add_multiple_participants() {
        let (env, _, creator, _) = setup_env();
        
        let mut bet = Bet {
            id: 1,
            creator,
            question: String::from_str(&env, "Will it rain?"),
            stake_amount: 1_000_000i128,
            end_time: 2_000_000u64,
            state: BetState::Created,
            created_at: 1_000_000u64,
            participants: Vec::new(&env),
            outcome_reports: Vec::new(&env),
            verified_outcome: None,
            shareable_url_hash: String::from_str(&env, "abc123"),
        };
        
        let p1 = Address::generate(&env);
        let p2 = Address::generate(&env);
        let p3 = Address::generate(&env);
        
        bet.participants.push_back(Participant {
            address: p1.clone(),
            position: true,
            stake: 500_000i128,
            joined_at: 1_000_000u64,
            has_reported: false,
        });
        
        bet.participants.push_back(Participant {
            address: p2.clone(),
            position: false,
            stake: 300_000i128,
            joined_at: 1_000_000u64,
            has_reported: false,
        });
        
        bet.participants.push_back(Participant {
            address: p3.clone(),
            position: true,
            stake: 200_000i128,
            joined_at: 1_000_000u64,
            has_reported: false,
        });
        
        assert_eq!(bet.participants.len(), 3);
        assert_eq!(bet.participants.get(0).unwrap().position, true);
        assert_eq!(bet.participants.get(1).unwrap().position, false);
        assert_eq!(bet.participants.get(2).unwrap().position, true);
    }

    // ─── Outcome Reporting Tests ──────────────────────────────────────────────

    #[test]
    fn test_add_outcome_report() {
        let (env, _, creator, participant) = setup_env();
        
        let mut bet = Bet {
            id: 1,
            creator,
            question: String::from_str(&env, "Will it rain?"),
            stake_amount: 1_000_000i128,
            end_time: 2_000_000u64,
            state: BetState::Ended,
            created_at: 1_000_000u64,
            participants: Vec::new(&env),
            outcome_reports: Vec::new(&env),
            verified_outcome: None,
            shareable_url_hash: String::from_str(&env, "abc123"),
        };
        
        let report = OutcomeReport {
            reporter: participant.clone(),
            outcome: true,
            reported_at: 2_000_000u64,
        };
        
        bet.outcome_reports.push_back(report);
        
        assert_eq!(bet.outcome_reports.len(), 1);
        assert_eq!(bet.outcome_reports.get(0).unwrap().reporter, participant);
        assert_eq!(bet.outcome_reports.get(0).unwrap().outcome, true);
    }

    #[test]
    fn test_multiple_outcome_reports_agreement() {
        let (env, _, creator, _) = setup_env();
        
        let mut bet = Bet {
            id: 1,
            creator,
            question: String::from_str(&env, "Will it rain?"),
            stake_amount: 1_000_000i128,
            end_time: 2_000_000u64,
            state: BetState::Ended,
            created_at: 1_000_000u64,
            participants: Vec::new(&env),
            outcome_reports: Vec::new(&env),
            verified_outcome: None,
            shareable_url_hash: String::from_str(&env, "abc123"),
        };
        
        let p1 = Address::generate(&env);
        let p2 = Address::generate(&env);
        
        bet.outcome_reports.push_back(OutcomeReport {
            reporter: p1,
            outcome: true,
            reported_at: 2_000_000u64,
        });
        
        bet.outcome_reports.push_back(OutcomeReport {
            reporter: p2,
            outcome: true,
            reported_at: 2_000_100u64,
        });
        
        // Check all reports agree
        let first_outcome = bet.outcome_reports.get(0).unwrap().outcome;
        let all_agree = bet.outcome_reports.iter().all(|r| r.outcome == first_outcome);
        
        assert!(all_agree);
    }

    #[test]
    fn test_multiple_outcome_reports_disagreement() {
        let (env, _, creator, _) = setup_env();
        
        let mut bet = Bet {
            id: 1,
            creator,
            question: String::from_str(&env, "Will it rain?"),
            stake_amount: 1_000_000i128,
            end_time: 2_000_000u64,
            state: BetState::Ended,
            created_at: 1_000_000u64,
            participants: Vec::new(&env),
            outcome_reports: Vec::new(&env),
            verified_outcome: None,
            shareable_url_hash: String::from_str(&env, "abc123"),
        };
        
        let p1 = Address::generate(&env);
        let p2 = Address::generate(&env);
        
        bet.outcome_reports.push_back(OutcomeReport {
            reporter: p1,
            outcome: true,
            reported_at: 2_000_000u64,
        });
        
        bet.outcome_reports.push_back(OutcomeReport {
            reporter: p2,
            outcome: false,
            reported_at: 2_000_100u64,
        });
        
        // Check reports disagree
        let first_outcome = bet.outcome_reports.get(0).unwrap().outcome;
        let all_agree = bet.outcome_reports.iter().all(|r| r.outcome == first_outcome);
        
        assert!(!all_agree);
    }

    // ─── Payout Calculation Tests ─────────────────────────────────────────────

    #[test]
    fn test_calculate_total_pool() {
        let (env, _, creator, _) = setup_env();
        
        let mut bet = Bet {
            id: 1,
            creator,
            question: String::from_str(&env, "Will it rain?"),
            stake_amount: 1_000_000i128,
            end_time: 2_000_000u64,
            state: BetState::Verified,
            created_at: 1_000_000u64,
            participants: Vec::new(&env),
            outcome_reports: Vec::new(&env),
            verified_outcome: Some(true),
            shareable_url_hash: String::from_str(&env, "abc123"),
        };
        
        let p1 = Address::generate(&env);
        let p2 = Address::generate(&env);
        
        bet.participants.push_back(Participant {
            address: p1,
            position: true,
            stake: 500_000i128,
            joined_at: 1_000_000u64,
            has_reported: true,
        });
        
        bet.participants.push_back(Participant {
            address: p2,
            position: false,
            stake: 300_000i128,
            joined_at: 1_000_000u64,
            has_reported: true,
        });
        
        // Calculate total pool
        let mut total_pool = bet.stake_amount;
        for p in bet.participants.iter() {
            total_pool += p.stake;
        }
        
        assert_eq!(total_pool, 1_800_000i128);
    }

    #[test]
    fn test_identify_winners() {
        let (env, _, creator, _) = setup_env();
        
        let mut bet = Bet {
            id: 1,
            creator,
            question: String::from_str(&env, "Will it rain?"),
            stake_amount: 1_000_000i128,
            end_time: 2_000_000u64,
            state: BetState::Verified,
            created_at: 1_000_000u64,
            participants: Vec::new(&env),
            outcome_reports: Vec::new(&env),
            verified_outcome: Some(true),
            shareable_url_hash: String::from_str(&env, "abc123"),
        };
        
        let p1 = Address::generate(&env);
        let p2 = Address::generate(&env);
        let p3 = Address::generate(&env);
        
        bet.participants.push_back(Participant {
            address: p1.clone(),
            position: true,
            stake: 500_000i128,
            joined_at: 1_000_000u64,
            has_reported: true,
        });
        
        bet.participants.push_back(Participant {
            address: p2,
            position: false,
            stake: 300_000i128,
            joined_at: 1_000_000u64,
            has_reported: true,
        });
        
        bet.participants.push_back(Participant {
            address: p3.clone(),
            position: true,
            stake: 200_000i128,
            joined_at: 1_000_000u64,
            has_reported: true,
        });
        
        // Find winners
        let winning_outcome = bet.verified_outcome.unwrap();
        let mut winners = Vec::new(&env);
        for p in bet.participants.iter() {
            if p.position == winning_outcome {
                winners.push_back(p);
            }
        }
        
        assert_eq!(winners.len(), 2);
        assert_eq!(winners.get(0).unwrap().address, p1);
        assert_eq!(winners.get(1).unwrap().address, p3);
    }

    #[test]
    fn test_platform_fee_calculation() {
        let amount = 1_000_000i128;
        let fee = amount * 2 / 100; // 2%
        let amount_after_fee = amount - fee;
        
        assert_eq!(fee, 20_000i128);
        assert_eq!(amount_after_fee, 980_000i128);
    }

    #[test]
    fn test_payout_distribution_equal_stakes() {
        let total_pool = 1_800_000i128;
        let num_winners = 2;
        let payout_per_winner = total_pool / num_winners;
        
        assert_eq!(payout_per_winner, 900_000i128);
    }

    // ─── Edge Case Tests ──────────────────────────────────────────────────────

    #[test]
    fn test_no_participants() {
        let (env, _, creator, _) = setup_env();
        
        let bet = Bet {
            id: 1,
            creator,
            question: String::from_str(&env, "Will it rain?"),
            stake_amount: 1_000_000i128,
            end_time: 2_000_000u64,
            state: BetState::Created,
            created_at: 1_000_000u64,
            participants: Vec::new(&env),
            outcome_reports: Vec::new(&env),
            verified_outcome: None,
            shareable_url_hash: String::from_str(&env, "abc123"),
        };
        
        assert_eq!(bet.participants.len(), 0);
    }

    #[test]
    fn test_no_winners_scenario() {
        let (env, _, creator, _) = setup_env();
        
        let mut bet = Bet {
            id: 1,
            creator,
            question: String::from_str(&env, "Will it rain?"),
            stake_amount: 1_000_000i128,
            end_time: 2_000_000u64,
            state: BetState::Verified,
            created_at: 1_000_000u64,
            participants: Vec::new(&env),
            outcome_reports: Vec::new(&env),
            verified_outcome: Some(false), // Outcome is false
            shareable_url_hash: String::from_str(&env, "abc123"),
        };
        
        let p1 = Address::generate(&env);
        let p2 = Address::generate(&env);
        
        // Both participants bet on true, but outcome is false
        bet.participants.push_back(Participant {
            address: p1,
            position: true,
            stake: 500_000i128,
            joined_at: 1_000_000u64,
            has_reported: true,
        });
        
        bet.participants.push_back(Participant {
            address: p2,
            position: true,
            stake: 300_000i128,
            joined_at: 1_000_000u64,
            has_reported: true,
        });
        
        // Find winners
        let winning_outcome = bet.verified_outcome.unwrap();
        let mut winners = Vec::new(&env);
        for p in bet.participants.iter() {
            if p.position == winning_outcome {
                winners.push_back(p);
            }
        }
        
        assert_eq!(winners.len(), 0);
    }

    #[test]
    fn test_all_participants_reported() {
        let (env, _, creator, _) = setup_env();
        
        let mut bet = Bet {
            id: 1,
            creator,
            question: String::from_str(&env, "Will it rain?"),
            stake_amount: 1_000_000i128,
            end_time: 2_000_000u64,
            state: BetState::Ended,
            created_at: 1_000_000u64,
            participants: Vec::new(&env),
            outcome_reports: Vec::new(&env),
            verified_outcome: None,
            shareable_url_hash: String::from_str(&env, "abc123"),
        };
        
        let p1 = Address::generate(&env);
        let p2 = Address::generate(&env);
        
        bet.participants.push_back(Participant {
            address: p1,
            position: true,
            stake: 500_000i128,
            joined_at: 1_000_000u64,
            has_reported: true,
        });
        
        bet.participants.push_back(Participant {
            address: p2,
            position: false,
            stake: 300_000i128,
            joined_at: 1_000_000u64,
            has_reported: true,
        });
        
        let all_reported = bet.participants.iter().all(|p| p.has_reported);
        assert!(all_reported);
    }

    #[test]
    fn test_some_participants_not_reported() {
        let (env, _, creator, _) = setup_env();
        
        let mut bet = Bet {
            id: 1,
            creator,
            question: String::from_str(&env, "Will it rain?"),
            stake_amount: 1_000_000i128,
            end_time: 2_000_000u64,
            state: BetState::Ended,
            created_at: 1_000_000u64,
            participants: Vec::new(&env),
            outcome_reports: Vec::new(&env),
            verified_outcome: None,
            shareable_url_hash: String::from_str(&env, "abc123"),
        };
        
        let p1 = Address::generate(&env);
        let p2 = Address::generate(&env);
        
        bet.participants.push_back(Participant {
            address: p1,
            position: true,
            stake: 500_000i128,
            joined_at: 1_000_000u64,
            has_reported: true,
        });
        
        bet.participants.push_back(Participant {
            address: p2,
            position: false,
            stake: 300_000i128,
            joined_at: 1_000_000u64,
            has_reported: false,
        });
        
        let all_reported = bet.participants.iter().all(|p| p.has_reported);
        assert!(!all_reported);
    }
}

