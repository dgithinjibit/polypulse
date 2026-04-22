#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, token, Address, Env, String, Vec};

mod storage;

// ─── Types ────────────────────────────────────────────────────────────────────

#[contracttype]
pub enum MarketState {
    Open,
    Closed,
    Resolved,
    Cancelled,
}

#[contracttype]
pub struct Market {
    pub id: u64,
    pub creator: Address,
    pub title: String,
    pub description: String,
    pub options: Vec<String>,
    pub liquidity_b: u64,
    pub shares_outstanding: Vec<u64>,
    pub state: MarketState,
    pub created_at: u64,
    pub closes_at: u64,
    pub resolved_at: Option<u64>,
    pub winning_option_id: Option<u32>,
    pub resolution_criteria: String,
}

#[contracttype]
pub struct Position {
    pub user: Address,
    pub market_id: u64,
    pub option_shares: Vec<OptionPosition>,
}

#[contracttype]
pub struct OptionPosition {
    pub option_id: u32,
    pub shares: u64,
    pub amount_spent: i128,
}

#[contracttype]
pub struct BuyResult {
    pub shares_issued: u64,
    pub new_price: u32,
}

#[contracttype]
pub struct SellResult {
    pub xlm_refund: i128,
}

// ─── Contract ─────────────────────────────────────────────────────────────────

#[contract]
pub struct MarketContract;

#[contractimpl]
impl MarketContract {
    // ── Initialisation ────────────────────────────────────────────────────────

    /// Must be called once immediately after deployment.
    /// Stores the XLM (Stellar native asset) token contract address so that
    /// buy_shares / sell_shares / claim_payout can transfer real funds.
    ///
    /// On Stellar mainnet the XLM token contract address is:
    ///   CAS3J7GYLGXMF6TDJBBYYSE3HQ6BBSMLNUQ34T6TZMYMW2EVH34XOWMA
    pub fn initialize(env: Env, admin: Address, xlm_token: Address) {
        // Can only be called once
        if storage::is_initialized(&env) {
            panic!("Contract already initialized");
        }
        admin.require_auth();
        storage::set_admin(&env, &admin);
        storage::set_xlm_token(&env, &xlm_token);
        storage::mark_initialized(&env);
    }

    // ── Market lifecycle ──────────────────────────────────────────────────────

    /// Create a new prediction market.
    pub fn create_market(
        env: Env,
        creator: Address,
        title: String,
        description: String,
        options: Vec<String>,
        close_time: u64,
        liquidity_b: u64,
        resolution_criteria: String,
    ) -> u64 {
        creator.require_auth();
        validate_market_params(&env, &title, &options, close_time, liquidity_b);

        let market_id = storage::get_next_market_id(&env);

        let mut shares_outstanding = Vec::new(&env);
        for _ in 0..options.len() {
            shares_outstanding.push_back(0u64);
        }

        let market = Market {
            id: market_id,
            creator: creator.clone(),
            title: title.clone(),
            description,
            options,
            liquidity_b,
            shares_outstanding,
            state: MarketState::Open,
            created_at: env.ledger().timestamp(),
            closes_at: close_time,
            resolved_at: None,
            winning_option_id: None,
            resolution_criteria,
        };

        storage::store_market(&env, &market);

        env.events().publish(
            (soroban_sdk::symbol_short!("mkt"), soroban_sdk::symbol_short!("created")),
            (market_id, creator, title),
        );

        market_id
    }

    // ── Trading ───────────────────────────────────────────────────────────────

    /// Buy shares in a market option.
    ///
    /// Pulls `xlm_amount` stroops from `buyer` into this contract, then
    /// issues the corresponding LMSR shares to the buyer's position.
    pub fn buy_shares(
        env: Env,
        buyer: Address,
        market_id: u64,
        option_id: u32,
        xlm_amount: i128,
    ) -> BuyResult {
        buyer.require_auth();

        if xlm_amount <= 0 {
            panic!("XLM amount must be positive");
        }

        let mut market = storage::get_market(&env, market_id)
            .unwrap_or_else(|| panic!("Market not found"));

        match market.state {
            MarketState::Open => {}
            MarketState::Closed => panic!("Market is closed"),
            MarketState::Resolved => panic!("Market is resolved"),
            MarketState::Cancelled => panic!("Market is cancelled"),
        }

        if option_id >= market.options.len() {
            panic!("Invalid option ID");
        }

        // ── Pull XLM from buyer into this contract ────────────────────────────
        let xlm_token_addr = storage::get_xlm_token(&env);
        let xlm = token::Client::new(&env, &xlm_token_addr);
        xlm.transfer(&buyer, &env.current_contract_address(), &xlm_amount);

        // ── LMSR accounting ───────────────────────────────────────────────────
        let shares_issued = calculate_shares_for_cost(
            &env,
            &market.shares_outstanding,
            option_id,
            xlm_amount,
            market.liquidity_b,
        );

        let current = market.shares_outstanding.get(option_id).unwrap();
        market.shares_outstanding.set(option_id, current + shares_issued);
        storage::store_market(&env, &market);

        // ── Update position ───────────────────────────────────────────────────
        let mut position = storage::get_position(&env, &buyer, market_id)
            .unwrap_or_else(|| {
                let mut opts = Vec::new(&env);
                for i in 0..market.options.len() {
                    opts.push_back(OptionPosition {
                        option_id: i,
                        shares: 0,
                        amount_spent: 0,
                    });
                }
                Position { user: buyer.clone(), market_id, option_shares: opts }
            });

        let mut updated = Vec::new(&env);
        for i in 0..position.option_shares.len() {
            let mut op = position.option_shares.get(i).unwrap();
            if op.option_id == option_id {
                op.shares += shares_issued;
                op.amount_spent += xlm_amount;
            }
            updated.push_back(op);
        }
        position.option_shares = updated;
        storage::store_position(&env, &position);

        let new_price = calculate_price(&env, &market.shares_outstanding, option_id, market.liquidity_b);

        env.events().publish(
            (soroban_sdk::symbol_short!("trade"), soroban_sdk::symbol_short!("buy")),
            (market_id, buyer, option_id, shares_issued, xlm_amount),
        );

        BuyResult { shares_issued, new_price }
    }

    /// Sell shares from a position.
    ///
    /// Burns `shares` from the seller's position and transfers the LMSR
    /// refund in XLM stroops from this contract back to `seller`.
    pub fn sell_shares(
        env: Env,
        seller: Address,
        market_id: u64,
        option_id: u32,
        shares: u64,
    ) -> SellResult {
        seller.require_auth();

        if shares == 0 {
            panic!("Shares must be positive");
        }

        let mut market = storage::get_market(&env, market_id)
            .unwrap_or_else(|| panic!("Market not found"));

        match market.state {
            MarketState::Open => {}
            MarketState::Closed => panic!("Market is closed"),
            MarketState::Resolved => panic!("Market is resolved"),
            MarketState::Cancelled => panic!("Market is cancelled"),
        }

        if option_id >= market.options.len() {
            panic!("Invalid option ID");
        }

        let mut position = storage::get_position(&env, &seller, market_id)
            .unwrap_or_else(|| panic!("Position not found"));

        // Verify seller owns enough shares
        let mut user_shares = 0u64;
        for i in 0..position.option_shares.len() {
            let op = position.option_shares.get(i).unwrap();
            if op.option_id == option_id {
                user_shares = op.shares;
                break;
            }
        }
        if user_shares < shares {
            panic!("Insufficient shares");
        }

        // ── LMSR refund calculation ───────────────────────────────────────────
        let xlm_refund = calculate_refund_for_shares(
            &env,
            &market.shares_outstanding,
            option_id,
            shares,
            market.liquidity_b,
        );

        // ── Update market shares ──────────────────────────────────────────────
        let current = market.shares_outstanding.get(option_id).unwrap();
        market.shares_outstanding.set(option_id, current - shares);
        storage::store_market(&env, &market);

        // ── Update position ───────────────────────────────────────────────────
        let mut updated = Vec::new(&env);
        for i in 0..position.option_shares.len() {
            let mut op = position.option_shares.get(i).unwrap();
            if op.option_id == option_id {
                let spent_reduction = if user_shares > 0 {
                    (op.amount_spent * shares as i128) / user_shares as i128
                } else {
                    0
                };
                op.shares -= shares;
                op.amount_spent -= spent_reduction;
            }
            updated.push_back(op);
        }
        position.option_shares = updated;
        storage::store_position(&env, &position);

        // ── Push XLM refund from contract to seller ───────────────────────────
        let xlm_token_addr = storage::get_xlm_token(&env);
        let xlm = token::Client::new(&env, &xlm_token_addr);
        xlm.transfer(&env.current_contract_address(), &seller, &xlm_refund);

        env.events().publish(
            (soroban_sdk::symbol_short!("trade"), soroban_sdk::symbol_short!("sell")),
            (market_id, seller, option_id, shares, xlm_refund),
        );

        SellResult { xlm_refund }
    }

    // ── Resolution & payouts ──────────────────────────────────────────────────

    /// Close a market (Open → Closed). Only the creator can call this.
    pub fn close_market(env: Env, caller: Address, market_id: u64) {
        caller.require_auth();

        let mut market = storage::get_market(&env, market_id)
            .unwrap_or_else(|| panic!("Market not found"));

        if market.creator != caller {
            panic!("Only creator can close");
        }

        match market.state {
            MarketState::Open => {}
            _ => panic!("Market is not open"),
        }

        market.state = MarketState::Closed;
        storage::store_market(&env, &market);

        env.events().publish(
            (soroban_sdk::symbol_short!("mkt"), soroban_sdk::symbol_short!("closed")),
            (market_id, caller),
        );
    }

    /// Resolve a market (Closed → Resolved). Only the creator can call this.
    pub fn resolve_market(
        env: Env,
        resolver: Address,
        market_id: u64,
        winning_option_id: u32,
    ) {
        resolver.require_auth();

        let mut market = storage::get_market(&env, market_id)
            .unwrap_or_else(|| panic!("Market not found"));

        if market.creator != resolver {
            panic!("Only creator can resolve");
        }

        match market.state {
            MarketState::Closed => {}
            MarketState::Open => panic!("Must close before resolving"),
            _ => panic!("Cannot resolve in current state"),
        }

        if winning_option_id >= market.options.len() {
            panic!("Invalid winning option ID");
        }

        market.state = MarketState::Resolved;
        market.winning_option_id = Some(winning_option_id);
        market.resolved_at = Some(env.ledger().timestamp());
        storage::store_market(&env, &market);

        env.events().publish(
            (soroban_sdk::symbol_short!("mkt"), soroban_sdk::symbol_short!("resolved")),
            (market_id, winning_option_id, resolver),
        );
    }

    /// Claim payout for a winning position.
    ///
    /// Calculates the proportional share of the total pool and transfers
    /// XLM from this contract to the claimer.
    pub fn claim_payout(env: Env, claimer: Address, market_id: u64) -> i128 {
        claimer.require_auth();

        let market = storage::get_market(&env, market_id)
            .unwrap_or_else(|| panic!("Market not found"));

        let winning_option_id = match market.state {
            MarketState::Resolved => market.winning_option_id.unwrap(),
            _ => panic!("Market is not resolved"),
        };

        let position = storage::get_position(&env, &claimer, market_id)
            .unwrap_or_else(|| panic!("No position found"));

        let mut user_winning_shares = 0u64;
        for i in 0..position.option_shares.len() {
            let op = position.option_shares.get(i).unwrap();
            if op.option_id == winning_option_id {
                user_winning_shares = op.shares;
                break;
            }
        }

        if user_winning_shares == 0 {
            panic!("No winning shares to claim");
        }

        let total_winning_shares = market.shares_outstanding.get(winning_option_id).unwrap();
        if total_winning_shares == 0 {
            panic!("No winning shares outstanding");
        }

        // Total pool = LMSR cost of the current share distribution
        let total_pool = lmsr_cost(&env, &market.shares_outstanding, market.liquidity_b);

        // Proportional payout: user_shares / total_winning_shares * total_pool
        let payout = (total_pool * user_winning_shares as i128) / total_winning_shares as i128;

        // Remove position before transfer (re-entrancy guard)
        storage::remove_position(&env, &claimer, market_id);

        // ── Push XLM payout from contract to claimer ──────────────────────────
        let xlm_token_addr = storage::get_xlm_token(&env);
        let xlm = token::Client::new(&env, &xlm_token_addr);
        xlm.transfer(&env.current_contract_address(), &claimer, &payout);

        env.events().publish(
            (soroban_sdk::symbol_short!("payout"), soroban_sdk::symbol_short!("claimed")),
            (market_id, claimer, payout),
        );

        payout
    }

    /// Cancel a market (Open → Cancelled). Only the creator can call this.
    pub fn cancel_market(env: Env, caller: Address, market_id: u64) {
        caller.require_auth();

        let mut market = storage::get_market(&env, market_id)
            .unwrap_or_else(|| panic!("Market not found"));

        if market.creator != caller {
            panic!("Only creator can cancel");
        }

        match market.state {
            MarketState::Open => {}
            _ => panic!("Can only cancel an open market"),
        }

        market.state = MarketState::Cancelled;
        storage::store_market(&env, &market);

        env.events().publish(
            (soroban_sdk::symbol_short!("mkt"), soroban_sdk::symbol_short!("cancelled")),
            (market_id, caller),
        );
    }

    /// Refund a user's position from a cancelled market.
    ///
    /// Returns the exact XLM `amount_spent` stored in the position back to
    /// the user. This is safe because amount_spent is only ever set from
    /// actual XLM transferred in during buy_shares.
    pub fn refund_cancelled(env: Env, user: Address, market_id: u64) -> i128 {
        user.require_auth();

        let market = storage::get_market(&env, market_id)
            .unwrap_or_else(|| panic!("Market not found"));

        match market.state {
            MarketState::Cancelled => {}
            _ => panic!("Market is not cancelled"),
        }

        let position = storage::get_position(&env, &user, market_id)
            .unwrap_or_else(|| panic!("No position to refund"));

        // Sum all XLM spent across all options
        let mut total_spent: i128 = 0;
        for i in 0..position.option_shares.len() {
            let op = position.option_shares.get(i).unwrap();
            total_spent += op.amount_spent;
        }

        if total_spent <= 0 {
            panic!("Nothing to refund");
        }

        // Remove position before transfer (re-entrancy guard)
        storage::remove_position(&env, &user, market_id);

        // ── Push XLM refund from contract to user ─────────────────────────────
        let xlm_token_addr = storage::get_xlm_token(&env);
        let xlm = token::Client::new(&env, &xlm_token_addr);
        xlm.transfer(&env.current_contract_address(), &user, &total_spent);

        env.events().publish(
            (soroban_sdk::symbol_short!("refund"), soroban_sdk::symbol_short!("cancel")),
            (market_id, user, total_spent),
        );

        total_spent
    }

    // ── Read-only ─────────────────────────────────────────────────────────────

    pub fn get_price(env: Env, market_id: u64, option_id: u32) -> u32 {
        let market = storage::get_market(&env, market_id)
            .unwrap_or_else(|| panic!("Market not found"));
        if option_id >= market.options.len() {
            panic!("Invalid option ID");
        }
        calculate_price(&env, &market.shares_outstanding, option_id, market.liquidity_b)
    }

    pub fn get_position(env: Env, user: Address, market_id: u64) -> Position {
        storage::get_position(&env, &user, market_id).unwrap_or_else(|| Position {
            user,
            market_id,
            option_shares: Vec::new(&env),
        })
    }

    pub fn get_market(env: Env, market_id: u64) -> Market {
        storage::get_market(&env, market_id)
            .unwrap_or_else(|| panic!("Market not found"))
    }

    pub fn get_xlm_token(env: Env) -> Address {
        storage::get_xlm_token(&env)
    }
}

// ─── Validation ───────────────────────────────────────────────────────────────

fn validate_market_params(
    env: &Env,
    title: &String,
    options: &Vec<String>,
    close_time: u64,
    liquidity_b: u64,
) {
    if title.len() == 0 {
        panic!("Title cannot be empty");
    }
    if options.len() < 2 {
        panic!("Need at least 2 options");
    }
    for i in 0..options.len() {
        if options.get(i).unwrap().len() == 0 {
            panic!("Option cannot be empty");
        }
    }
    if close_time <= env.ledger().timestamp() {
        panic!("Close time must be in the future");
    }
    if liquidity_b == 0 || liquidity_b > 1_000_000 {
        panic!("liquidity_b must be 1..1_000_000");
    }
}

// ─── LMSR pricing ─────────────────────────────────────────────────────────────

/// LMSR cost function: b * ln(sum(exp(q_i / b)))
/// Uses log-sum-exp trick for numerical stability.
fn lmsr_cost(env: &Env, shares: &Vec<u64>, liquidity_b: u64) -> i128 {
    if shares.is_empty() {
        return 0;
    }
    if liquidity_b == 0 {
        panic!("liquidity_b cannot be zero");
    }

    let b = liquidity_b as f64;

    // Find max for numerical stability
    let mut max_val: f64 = 0.0;
    for i in 0..shares.len() {
        let v = shares.get(i).unwrap() as f64 / b;
        if i == 0 || v > max_val {
            max_val = v;
        }
    }

    let mut sum_exp: f64 = 0.0;
    for i in 0..shares.len() {
        sum_exp += ((shares.get(i).unwrap() as f64 / b) - max_val).exp();
    }

    let cost = b * (sum_exp.ln() + max_val);

    if cost.is_nan() || cost.is_infinite() || cost < 0.0 {
        panic!("LMSR cost invalid");
    }

    cost as i128
}

/// Binary search to find shares issued for a given XLM spend.
fn calculate_shares_for_cost(
    env: &Env,
    current_shares: &Vec<u64>,
    option_index: u32,
    xlm_amount: i128,
    liquidity_b: u64,
) -> u64 {
    if xlm_amount <= 0 {
        return 0;
    }
    if option_index >= current_shares.len() {
        panic!("Option index out of bounds");
    }

    let cost_before = lmsr_cost(env, current_shares, liquidity_b);
    let target = cost_before + xlm_amount;

    let mut lo: u64 = 0;
    let mut hi: u64 = (xlm_amount as u64) * 10 + 10_000;

    for _ in 0..64 {
        if hi <= lo + 1 {
            break;
        }
        let mid = lo + (hi - lo) / 2;
        let mut test = current_shares.clone();
        let cur = test.get(option_index).unwrap();
        test.set(option_index, cur + mid);
        if lmsr_cost(env, &test, liquidity_b) < target {
            lo = mid;
        } else {
            hi = mid;
        }
    }

    lo
}

/// LMSR refund for selling shares: cost_before - cost_after.
fn calculate_refund_for_shares(
    env: &Env,
    current_shares: &Vec<u64>,
    option_index: u32,
    shares_to_sell: u64,
    liquidity_b: u64,
) -> i128 {
    if shares_to_sell == 0 {
        return 0;
    }
    if option_index >= current_shares.len() {
        panic!("Option index out of bounds");
    }

    let current_val = current_shares.get(option_index).unwrap();
    if shares_to_sell > current_val {
        panic!("Cannot sell more than outstanding");
    }

    let cost_before = lmsr_cost(env, current_shares, liquidity_b);
    let mut after = current_shares.clone();
    after.set(option_index, current_val - shares_to_sell);
    let cost_after = lmsr_cost(env, &after, liquidity_b);

    let refund = cost_before - cost_after;
    if refund < 0 {
        panic!("Refund negative");
    }
    refund
}

/// LMSR price for an option: exp(q_i/b) / sum(exp(q_j/b)), scaled 0-100.
fn calculate_price(
    env: &Env,
    shares: &Vec<u64>,
    option_index: u32,
    liquidity_b: u64,
) -> u32 {
    if shares.is_empty() || liquidity_b == 0 {
        return 0;
    }
    if option_index >= shares.len() {
        panic!("Option index out of bounds");
    }

    let b = liquidity_b as f64;

    let mut max_val: f64 = 0.0;
    for i in 0..shares.len() {
        let v = shares.get(i).unwrap() as f64 / b;
        if i == 0 || v > max_val {
            max_val = v;
        }
    }

    let mut total: f64 = 0.0;
    let mut option_exp: f64 = 0.0;
    for i in 0..shares.len() {
        let v = ((shares.get(i).unwrap() as f64 / b) - max_val).exp();
        if i == option_index {
            option_exp = v;
        }
        total += v;
    }

    let price = (option_exp / total) * 100.0;
    if price.is_nan() || price.is_infinite() || price < 0.0 || price > 100.0 {
        panic!("Price out of bounds");
    }

    price.round() as u32
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{
        testutils::{Address as _, Ledger},
        Env,
    };

    fn setup_env() -> (Env, Address, Address) {
        let env = Env::default();
        env.mock_all_auths();
        // Advance ledger so close_time can be in the future
        env.ledger().with_mut(|l| l.timestamp = 1_000_000);
        let admin = Address::generate(&env);
        let xlm_token = Address::generate(&env);
        (env, admin, xlm_token)
    }

    #[test]
    fn test_lmsr_cost_equal_shares() {
        let env = Env::default();
        let mut shares = Vec::new(&env);
        shares.push_back(100u64);
        shares.push_back(100u64);
        let cost = lmsr_cost(&env, &shares, 100);
        assert!(cost > 0);
    }

    #[test]
    fn test_lmsr_cost_zero_shares() {
        let env = Env::default();
        let mut shares = Vec::new(&env);
        shares.push_back(0u64);
        shares.push_back(0u64);
        let cost = lmsr_cost(&env, &shares, 100);
        // b * ln(2) ≈ 69 for b=100
        assert!(cost >= 60 && cost <= 80);
    }

    #[test]
    #[should_panic(expected = "liquidity_b cannot be zero")]
    fn test_lmsr_cost_zero_b() {
        let env = Env::default();
        let mut shares = Vec::new(&env);
        shares.push_back(100u64);
        lmsr_cost(&env, &shares, 0);
    }

    #[test]
    fn test_price_sums_to_100() {
        let env = Env::default();
        let mut shares = Vec::new(&env);
        shares.push_back(100u64);
        shares.push_back(200u64);
        shares.push_back(150u64);
        let p0 = calculate_price(&env, &shares, 0, 100);
        let p1 = calculate_price(&env, &shares, 1, 100);
        let p2 = calculate_price(&env, &shares, 2, 100);
        // Rounding means sum is within 3 of 100
        let sum = p0 + p1 + p2;
        assert!(sum >= 97 && sum <= 103, "prices sum to {}", sum);
    }

    #[test]
    fn test_shares_for_cost_roundtrip() {
        let env = Env::default();
        let mut shares = Vec::new(&env);
        shares.push_back(100u64);
        shares.push_back(100u64);
        let xlm = 1_000i128;
        let issued = calculate_shares_for_cost(&env, &shares, 0, xlm, 100);
        assert!(issued > 0);
        // Verify cost of issued shares <= xlm (conservative)
        let mut after = shares.clone();
        let cur = after.get(0).unwrap();
        after.set(0, cur + issued);
        let cost_diff = lmsr_cost(&env, &after, 100) - lmsr_cost(&env, &shares, 100);
        assert!(cost_diff <= xlm);
    }

    #[test]
    fn test_refund_positive() {
        let env = Env::default();
        let mut shares = Vec::new(&env);
        shares.push_back(500u64);
        shares.push_back(100u64);
        let refund = calculate_refund_for_shares(&env, &shares, 0, 100, 100);
        assert!(refund > 0);
    }
}
