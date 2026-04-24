use soroban_sdk::{Address, Env, Symbol};

use crate::types::Bet;

const BET_COUNTER: Symbol = Symbol::short("BET_CNT");
const BET_PREFIX: Symbol = Symbol::short("BET");
const ADMIN: Symbol = Symbol::short("ADMIN");

/// Get next bet ID and increment counter
pub fn get_next_bet_id(env: &Env) -> u64 {
    let current: u64 = env.storage().instance().get(&BET_COUNTER).unwrap_or(0);
    let next = current + 1;
    env.storage().instance().set(&BET_COUNTER, &next);
    next
}

/// Store bet
pub fn set_bet(env: &Env, bet_id: u64, bet: &Bet) {
    let key = (BET_PREFIX, bet_id);
    env.storage().persistent().set(&key, bet);
}

/// Get bet
pub fn get_bet(env: &Env, bet_id: u64) -> Option<Bet> {
    let key = (BET_PREFIX, bet_id);
    env.storage().persistent().get(&key)
}

/// Check if bet exists
pub fn bet_exists(env: &Env, bet_id: u64) -> bool {
    let key = (BET_PREFIX, bet_id);
    env.storage().persistent().has(&key)
}

/// Set admin address
pub fn set_admin(env: &Env, admin: &Address) {
    env.storage().instance().set(&ADMIN, admin);
}

/// Get admin address
pub fn get_admin(env: &Env) -> Address {
    env.storage().instance().get(&ADMIN).unwrap()
}
