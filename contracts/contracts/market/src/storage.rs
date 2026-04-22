use soroban_sdk::{Address, Env, Symbol, symbol_short};

use crate::{Market, Position};

// ─── Storage key constants ────────────────────────────────────────────────────

const MARKET_KEY: Symbol    = symbol_short!("MARKET");
const POSITION_KEY: Symbol  = symbol_short!("POSITION");
const NEXT_MKT_KEY: Symbol  = symbol_short!("NEXT_MKT");
const XLM_TOKEN_KEY: Symbol = symbol_short!("XLM_TOK");
const ADMIN_KEY: Symbol     = symbol_short!("ADMIN");
const INIT_KEY: Symbol      = symbol_short!("INIT");

// ─── Initialisation ───────────────────────────────────────────────────────────

pub fn is_initialized(env: &Env) -> bool {
    env.storage().instance().has(&INIT_KEY)
}

pub fn mark_initialized(env: &Env) {
    env.storage().instance().set(&INIT_KEY, &true);
}

pub fn set_admin(env: &Env, admin: &Address) {
    env.storage().instance().set(&ADMIN_KEY, admin);
}

pub fn get_admin(env: &Env) -> Address {
    env.storage().instance().get(&ADMIN_KEY)
        .unwrap_or_else(|| panic!("Admin not set — call initialize first"))
}

/// Store the XLM token contract address (set once in initialize).
pub fn set_xlm_token(env: &Env, token: &Address) {
    env.storage().instance().set(&XLM_TOKEN_KEY, token);
}

/// Retrieve the XLM token contract address.
/// Panics if initialize() was never called.
pub fn get_xlm_token(env: &Env) -> Address {
    env.storage().instance().get(&XLM_TOKEN_KEY)
        .unwrap_or_else(|| panic!("XLM token not set — call initialize first"))
}

// ─── Market ID counter ────────────────────────────────────────────────────────

pub fn get_next_market_id(env: &Env) -> u64 {
    let current: u64 = env.storage().persistent().get(&NEXT_MKT_KEY).unwrap_or(1);
    env.storage().persistent().set(&NEXT_MKT_KEY, &(current + 1));
    current
}

// ─── Markets ──────────────────────────────────────────────────────────────────

fn market_key(market_id: u64) -> (Symbol, u64) {
    (MARKET_KEY, market_id)
}

pub fn store_market(env: &Env, market: &Market) {
    let key = market_key(market.id);
    env.storage().persistent().set(&key, market);
    env.storage().persistent().extend_ttl(&key, 100, 6_307_200);
}

pub fn get_market(env: &Env, market_id: u64) -> Option<Market> {
    env.storage().persistent().get(&market_key(market_id))
}

pub fn market_exists(env: &Env, market_id: u64) -> bool {
    env.storage().persistent().has(&market_key(market_id))
}

// ─── Positions ────────────────────────────────────────────────────────────────

fn position_key(user: &Address, market_id: u64) -> (Symbol, Address, u64) {
    (POSITION_KEY, user.clone(), market_id)
}

pub fn store_position(env: &Env, position: &Position) {
    let key = position_key(&position.user, position.market_id);
    env.storage().persistent().set(&key, position);
    env.storage().persistent().extend_ttl(&key, 100, 6_307_200);
}

pub fn get_position(env: &Env, user: &Address, market_id: u64) -> Option<Position> {
    env.storage().persistent().get(&position_key(user, market_id))
}

pub fn position_exists(env: &Env, user: &Address, market_id: u64) -> bool {
    env.storage().persistent().has(&position_key(user, market_id))
}

pub fn remove_position(env: &Env, user: &Address, market_id: u64) {
    env.storage().persistent().remove(&position_key(user, market_id));
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Env, String, Vec};
    use crate::{Market, MarketState, OptionPosition, Position};

    #[test]
    fn test_market_id_increments() {
        let env = Env::default();
        assert_eq!(get_next_market_id(&env), 1);
        assert_eq!(get_next_market_id(&env), 2);
        assert_eq!(get_next_market_id(&env), 3);
    }

    #[test]
    fn test_xlm_token_roundtrip() {
        let env = Env::default();
        let token = Address::generate(&env);
        set_xlm_token(&env, &token);
        assert_eq!(get_xlm_token(&env), token);
    }

    #[test]
    #[should_panic(expected = "XLM token not set")]
    fn test_get_xlm_token_uninitialized() {
        let env = Env::default();
        get_xlm_token(&env);
    }

    #[test]
    fn test_store_and_get_market() {
        let env = Env::default();
        let creator = Address::generate(&env);
        let mut options = Vec::new(&env);
        options.push_back(String::from_str(&env, "Yes"));
        options.push_back(String::from_str(&env, "No"));
        let mut shares = Vec::new(&env);
        shares.push_back(0u64);
        shares.push_back(0u64);

        let market = Market {
            id: 1,
            creator,
            title: String::from_str(&env, "Test"),
            description: String::from_str(&env, "Desc"),
            options,
            liquidity_b: 100,
            shares_outstanding: shares,
            state: MarketState::Open,
            created_at: 1000,
            closes_at: 2000,
            resolved_at: None,
            winning_option_id: None,
            resolution_criteria: String::from_str(&env, "Criteria"),
        };

        store_market(&env, &market);
        let retrieved = get_market(&env, 1).unwrap();
        assert_eq!(retrieved.id, 1);
        assert_eq!(retrieved.liquidity_b, 100);
        assert!(market_exists(&env, 1));
        assert!(!market_exists(&env, 2));
    }

    #[test]
    fn test_position_lifecycle() {
        let env = Env::default();
        let user = Address::generate(&env);
        let mut opts = Vec::new(&env);
        opts.push_back(OptionPosition { option_id: 0, shares: 100, amount_spent: 1000 });

        let pos = Position { user: user.clone(), market_id: 1, option_shares: opts };
        assert!(!position_exists(&env, &user, 1));
        store_position(&env, &pos);
        assert!(position_exists(&env, &user, 1));
        let retrieved = get_position(&env, &user, 1).unwrap();
        assert_eq!(retrieved.option_shares.get(0).unwrap().shares, 100);
        remove_position(&env, &user, 1);
        assert!(!position_exists(&env, &user, 1));
    }
}
