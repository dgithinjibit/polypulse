#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracttype,
    token, Address, Env, String,
};

// ─── Types ────────────────────────────────────────────────────────────────────

#[contracttype]
pub enum ChallengeState {
    Pending,
    Accepted,
    Resolved,
    Cancelled,
    Expired,
}

#[contracttype]
pub struct Challenge {
    pub id: u64,
    pub creator: Address,
    pub opponent: Option<Address>,
    pub question: String,
    /// XLM stake in stroops (each side puts this in)
    pub xlm_stake: i128,
    pub creator_choice: String,
    pub state: ChallengeState,
    pub is_open: bool,
    pub created_at: u64,
    pub expires_at: u64,
    pub resolved_at: Option<u64>,
    pub winner: Option<Address>,
    pub resolution_criteria: String,
}

// ─── Storage keys ─────────────────────────────────────────────────────────────

const CHALLENGE_KEY: soroban_sdk::Symbol = soroban_sdk::symbol_short!("CHAL");
const NEXT_ID_KEY: soroban_sdk::Symbol   = soroban_sdk::symbol_short!("NEXT_ID");
const XLM_TOK_KEY: soroban_sdk::Symbol   = soroban_sdk::symbol_short!("XLM_TOK");
const ADMIN_KEY: soroban_sdk::Symbol     = soroban_sdk::symbol_short!("ADMIN");
const INIT_KEY: soroban_sdk::Symbol      = soroban_sdk::symbol_short!("INIT");

fn challenge_key(id: u64) -> (soroban_sdk::Symbol, u64) {
    (CHALLENGE_KEY, id)
}

fn next_id(env: &Env) -> u64 {
    let id: u64 = env.storage().persistent().get(&NEXT_ID_KEY).unwrap_or(1);
    env.storage().persistent().set(&NEXT_ID_KEY, &(id + 1));
    id
}

fn store_challenge(env: &Env, c: &Challenge) {
    let key = challenge_key(c.id);
    env.storage().persistent().set(&key, c);
    env.storage().persistent().extend_ttl(&key, 100, 6_307_200);
}

fn load_challenge(env: &Env, id: u64) -> Challenge {
    env.storage().persistent().get(&challenge_key(id))
        .unwrap_or_else(|| panic!("Challenge not found"))
}

fn get_xlm_token(env: &Env) -> Address {
    env.storage().instance().get(&XLM_TOK_KEY)
        .unwrap_or_else(|| panic!("Not initialized"))
}

// ─── Contract ─────────────────────────────────────────────────────────────────

#[contract]
pub struct ChallengeContract;

#[contractimpl]
impl ChallengeContract {
    // ── Init ──────────────────────────────────────────────────────────────────

    /// Call once after deployment to set the XLM token address and admin.
    pub fn initialize(env: Env, admin: Address, xlm_token: Address) {
        if env.storage().instance().has(&INIT_KEY) {
            panic!("Already initialized");
        }
        admin.require_auth();
        env.storage().instance().set(&ADMIN_KEY, &admin);
        env.storage().instance().set(&XLM_TOK_KEY, &xlm_token);
        env.storage().instance().set(&INIT_KEY, &true);
    }

    // ── Challenge lifecycle ───────────────────────────────────────────────────

    /// Create a challenge and lock the creator's XLM stake.
    ///
    /// If `is_open` is true, any user can accept it.
    /// If `is_open` is false, only `opponent` (if provided) can accept.
    pub fn create_challenge(
        env: Env,
        creator: Address,
        opponent: Option<Address>,
        question: String,
        xlm_stake: i128,
        creator_choice: String,
        expires_at: u64,
        is_open: bool,
        resolution_criteria: String,
    ) -> u64 {
        creator.require_auth();

        if xlm_stake <= 0 {
            panic!("Stake must be positive");
        }
        if question.len() == 0 {
            panic!("Question cannot be empty");
        }
        if expires_at <= env.ledger().timestamp() {
            panic!("expires_at must be in the future");
        }

        // Pull creator's stake into contract
        let xlm = token::Client::new(&env, &get_xlm_token(&env));
        xlm.transfer(&creator, &env.current_contract_address(), &xlm_stake);

        let id = next_id(&env);
        let challenge = Challenge {
            id,
            creator: creator.clone(),
            opponent,
            question: question.clone(),
            xlm_stake,
            creator_choice,
            state: ChallengeState::Pending,
            is_open,
            created_at: env.ledger().timestamp(),
            expires_at,
            resolved_at: None,
            winner: None,
            resolution_criteria,
        };

        store_challenge(&env, &challenge);

        env.events().publish(
            (soroban_sdk::symbol_short!("chal"), soroban_sdk::symbol_short!("created")),
            (id, creator, question),
        );

        id
    }

    /// Accept a challenge and lock the opponent's XLM stake.
    pub fn accept_challenge(env: Env, opponent: Address, challenge_id: u64) {
        opponent.require_auth();

        let mut c = load_challenge(&env, challenge_id);

        match c.state {
            ChallengeState::Pending => {}
            _ => panic!("Challenge is not pending"),
        }

        if env.ledger().timestamp() > c.expires_at {
            panic!("Challenge has expired");
        }

        if c.creator == opponent {
            panic!("Creator cannot accept their own challenge");
        }

        // If not open, only the designated opponent can accept
        if !c.is_open {
            match &c.opponent {
                Some(designated) => {
                    if *designated != opponent {
                        panic!("Only the designated opponent can accept");
                    }
                }
                None => panic!("No opponent designated for closed challenge"),
            }
        }

        // Pull opponent's stake into contract
        let xlm = token::Client::new(&env, &get_xlm_token(&env));
        xlm.transfer(&opponent, &env.current_contract_address(), &c.xlm_stake);

        c.opponent = Some(opponent.clone());
        c.state = ChallengeState::Accepted;
        store_challenge(&env, &c);

        env.events().publish(
            (soroban_sdk::symbol_short!("chal"), soroban_sdk::symbol_short!("accepted")),
            (challenge_id, opponent),
        );
    }

    /// Resolve a challenge and pay out 2x stake to the winner.
    ///
    /// Only the admin or the creator can resolve.
    pub fn resolve_challenge(
        env: Env,
        resolver: Address,
        challenge_id: u64,
        winner: Address,
    ) {
        resolver.require_auth();

        let mut c = load_challenge(&env, challenge_id);

        match c.state {
            ChallengeState::Accepted => {}
            _ => panic!("Challenge must be accepted before resolving"),
        }

        // Only admin or creator can resolve
        let admin: Address = env.storage().instance().get(&ADMIN_KEY)
            .unwrap_or_else(|| panic!("Not initialized"));
        if resolver != admin && resolver != c.creator {
            panic!("Only admin or creator can resolve");
        }

        // Winner must be creator or opponent
        let opponent = c.opponent.clone().unwrap();
        if winner != c.creator && winner != opponent {
            panic!("Winner must be creator or opponent");
        }

        // Pay out 2x stake to winner (both stakes)
        let payout = c.xlm_stake * 2;
        let xlm = token::Client::new(&env, &get_xlm_token(&env));
        xlm.transfer(&env.current_contract_address(), &winner, &payout);

        c.state = ChallengeState::Resolved;
        c.winner = Some(winner.clone());
        c.resolved_at = Some(env.ledger().timestamp());
        store_challenge(&env, &c);

        env.events().publish(
            (soroban_sdk::symbol_short!("chal"), soroban_sdk::symbol_short!("resolved")),
            (challenge_id, winner, payout),
        );
    }

    /// Cancel an unaccepted challenge and refund the creator's stake.
    pub fn cancel_challenge(env: Env, creator: Address, challenge_id: u64) {
        creator.require_auth();

        let mut c = load_challenge(&env, challenge_id);

        if c.creator != creator {
            panic!("Only creator can cancel");
        }

        match c.state {
            ChallengeState::Pending => {}
            ChallengeState::Accepted => panic!("Cannot cancel an accepted challenge"),
            _ => panic!("Challenge already finalised"),
        }

        // Refund creator's stake
        let xlm = token::Client::new(&env, &get_xlm_token(&env));
        xlm.transfer(&env.current_contract_address(), &creator, &c.xlm_stake);

        c.state = ChallengeState::Cancelled;
        store_challenge(&env, &c);

        env.events().publish(
            (soroban_sdk::symbol_short!("chal"), soroban_sdk::symbol_short!("cancelled")),
            (challenge_id, creator),
        );
    }

    /// Expire a challenge that has passed its deadline without being accepted.
    /// Refunds the creator's stake.
    pub fn expire_challenge(env: Env, challenge_id: u64) {
        let mut c = load_challenge(&env, challenge_id);

        match c.state {
            ChallengeState::Pending => {}
            _ => panic!("Only pending challenges can expire"),
        }

        if env.ledger().timestamp() <= c.expires_at {
            panic!("Challenge has not expired yet");
        }

        // Refund creator's stake
        let xlm = token::Client::new(&env, &get_xlm_token(&env));
        xlm.transfer(&env.current_contract_address(), &c.creator, &c.xlm_stake);

        c.state = ChallengeState::Expired;
        store_challenge(&env, &c);

        env.events().publish(
            (soroban_sdk::symbol_short!("chal"), soroban_sdk::symbol_short!("expired")),
            (challenge_id, c.creator),
        );
    }

    // ── Read-only ─────────────────────────────────────────────────────────────

    pub fn get_challenge(env: Env, challenge_id: u64) -> Challenge {
        load_challenge(&env, challenge_id)
    }

    pub fn get_xlm_token(env: Env) -> Address {
        get_xlm_token(&env)
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{
        testutils::{Address as _, Ledger},
        Env,
    };

    #[test]
    fn test_next_id_increments() {
        let env = Env::default();
        assert_eq!(next_id(&env), 1);
        assert_eq!(next_id(&env), 2);
    }

    #[test]
    #[should_panic(expected = "Not initialized")]
    fn test_get_xlm_token_uninitialized() {
        let env = Env::default();
        get_xlm_token(&env);
    }
}
