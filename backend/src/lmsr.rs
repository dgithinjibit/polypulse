/// LMSR (Logarithmic Market Scoring Rule) pricing functions.
/// Matches the Python implementation in backend/base/models.py exactly.

/// LMSR cost function: b * ln(sum(exp(q_i / b)))
/// Uses the log-sum-exp trick for numerical stability.
pub fn lmsr_cost(shares: &[f64], b: f64) -> f64 {
    if shares.is_empty() {
        return 0.0;
    }
    // log-sum-exp trick: subtract max before exponentiating
    let max_q = shares.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let sum_exp: f64 = shares.iter().map(|&q| ((q - max_q) / b).exp()).sum();
    b * (sum_exp.ln() + max_q / b)
}

/// Current price (probability) for option at `option_idx`.
/// price_i = exp(q_i / b) / sum_j(exp(q_j / b))
pub fn calculate_price(shares: &[f64], option_idx: usize, b: f64) -> f64 {
    if shares.is_empty() || option_idx >= shares.len() {
        return 0.0;
    }
    let max_q = shares.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let exps: Vec<f64> = shares.iter().map(|&q| ((q - max_q) / b).exp()).collect();
    let total: f64 = exps.iter().sum();
    if total == 0.0 {
        1.0 / shares.len() as f64
    } else {
        exps[option_idx] / total
    }
}

/// Binary-search for how many shares `xlm_amount` buys for `option_idx`.
/// Returns the number of shares issued.
pub fn calculate_shares_for_cost(
    current: &[f64],
    option_idx: usize,
    xlm_amount: f64,
    b: f64,
) -> f64 {
    let cost_before = lmsr_cost(current, b);
    let target_cost = cost_before + xlm_amount;

    let mut lo = 0.0_f64;
    let mut hi = xlm_amount * 10.0 + 1.0;

    for _ in 0..64 {
        let mid = (lo + hi) / 2.0;
        let mut after = current.to_vec();
        after[option_idx] += mid;
        if lmsr_cost(&after, b) < target_cost {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    (lo + hi) / 2.0
}

/// Calculate the refund for selling `shares` of `option_idx`.
/// refund = cost(before) - cost(after)
pub fn calculate_refund(current: &[f64], option_idx: usize, shares: f64, b: f64) -> f64 {
    let cost_before = lmsr_cost(current, b);
    let mut after = current.to_vec();
    after[option_idx] = (after[option_idx] - shares).max(0.0);
    let cost_after = lmsr_cost(&after, b);
    (cost_before - cost_after).max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lmsr_cost_symmetric() {
        // Equal shares → cost = b * ln(n)
        let shares = vec![0.0, 0.0];
        let b = 100.0;
        let cost = lmsr_cost(&shares, b);
        let expected = b * (2.0_f64.ln());
        assert!((cost - expected).abs() < 1e-9, "cost={cost} expected={expected}");
    }

    #[test]
    fn test_price_sums_to_one() {
        let shares = vec![50.0, 30.0, 20.0];
        let b = 100.0;
        let total: f64 = (0..3).map(|i| calculate_price(&shares, i, b)).sum();
        assert!((total - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_shares_for_cost_roundtrip() {
        let current = vec![0.0, 0.0];
        let b = 100.0;
        let amount = 50.0;
        let shares = calculate_shares_for_cost(&current, 0, amount, b);
        assert!(shares > 0.0);
        // Cost of buying those shares should equal amount
        let mut after = current.clone();
        after[0] += shares;
        let cost = lmsr_cost(&after, b) - lmsr_cost(&current, b);
        assert!((cost - amount).abs() < 0.01, "cost={cost} amount={amount}");
    }

    #[test]
    fn test_refund_positive() {
        let current = vec![100.0, 0.0];
        let b = 100.0;
        let refund = calculate_refund(&current, 0, 50.0, b);
        assert!(refund > 0.0);
    }
}
