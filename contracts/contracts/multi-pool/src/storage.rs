use soroban_sdk::{Address, Env, Symbol};

use crate::types::MultiParticipantPool;

const POOL_COUNTER: Symbol = Symbol::short("POOL_CNT");
const POOL_PREFIX: Symbol = Symbol::short("POOL");
const ADMIN: Symbol = Symbol::short("ADMIN");
const TREASURY: Symbol = Symbol::short("TREASURY");

pub fn get_next_pool_id(env: &Env) -> u64 {
    let current: u64 = env.storage().instance().get(&POOL_COUNTER).unwrap_or(0);
    let next = current + 1;
    env.storage().instance().set(&POOL_COUNTER, &next);
    next
}

pub fn set_pool(env: &Env, pool_id: u64, pool: &MultiParticipantPool) {
    let key = (POOL_PREFIX, pool_id);
    env.storage().persistent().set(&key, pool);
}

pub fn get_pool(env: &Env, pool_id: u64) -> Option<MultiParticipantPool> {
    let key = (POOL_PREFIX, pool_id);
    env.storage().persistent().get(&key)
}

pub fn set_admin(env: &Env, admin: &Address) {
    env.storage().instance().set(&ADMIN, admin);
}

pub fn get_admin(env: &Env) -> Address {
    env.storage().instance().get(&ADMIN).unwrap()
}

pub fn set_treasury(env: &Env, treasury: &Address) {
    env.storage().instance().set(&TREASURY, treasury);
}

pub fn get_treasury(env: &Env) -> Address {
    env.storage().instance().get(&TREASURY).unwrap()
}
