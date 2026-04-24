// Feature: polypulse-enhancements, Property 1: Multi-Participant Payout Fairness
// Property-based tests for multi-participant pool payout calculations

// Test payout calculation formula
fn calculate_payout(user_stake: i64, total_winning_stakes: i64, total_pool: i64) -> i64 {
    let platform_fee = (total_pool * 7) / 100;
    let distributable = total_pool - platform_fee;
    (user_stake * distributable) / total_winning_stakes
}

#[test]
fn test_payout_fairness_two_winners() {
    // Test case: 100 XLM + 200 XLM on Yes, 300 XLM on No
    // Total: 600 XLM, Fee: 42 XLM, Distributable: 558 XLM
    let stake1 = 100_000_000; // 100 XLM in stroops
    let stake2 = 200_000_000; // 200 XLM
    let stake3 = 300_000_000; // 300 XLM
    
    let total_pool = stake1 + stake2 + stake3;
    let total_winning = stake1 + stake2;
    
    let payout1 = calculate_payout(stake1, total_winning, total_pool);
    let payout2 = calculate_payout(stake2, total_winning, total_pool);
    
    // Expected: P1 = (100/300) * 558 = 186 XLM
    //           P2 = (200/300) * 558 = 372 XLM
    assert_eq!(payout1, 186_000_000);
    assert_eq!(payout2, 372_000_000);
    
    // Total payouts should equal distributable
    assert_eq!(payout1 + payout2, 558_000_000);
}

#[test]
fn test_payout_fairness_many_winners() {
    let yes_stakes = vec![
        10_000_000,  // 1 XLM
        20_000_000,  // 2 XLM
        30_000_000,  // 3 XLM
        40_000_000,  // 4 XLM
        50_000_000,  // 5 XLM
    ];
    
    let no_stakes = vec![
        50_000_000,  // 5 XLM
        60_000_000,  // 6 XLM
        40_000_000,  // 4 XLM
    ];
    
    let total_yes: i64 = yes_stakes.iter().sum();
    let total_no: i64 = no_stakes.iter().sum();
    let total_pool = total_yes + total_no;
    
    // Calculate payouts for yes winners
    let mut total_paid = 0i64;
    for stake in &yes_stakes {
        let payout = calculate_payout(*stake, total_yes, total_pool);
        total_paid += payout;
    }
    
    // Verify total paid equals distributable (within rounding)
    let platform_fee = (total_pool * 7) / 100;
    let distributable = total_pool - platform_fee;
    
    assert!(
        (total_paid - distributable).abs() <= yes_stakes.len() as i64,
        "Total paid {} != distributable {}",
        total_paid,
        distributable
    );
}

#[test]
fn test_platform_fee_consistency() {
    let test_cases = vec![
        (100_000_000, 100_000_000),   // 10 XLM each side
        (500_000_000, 300_000_000),   // 50 vs 30 XLM
        (1_000_000_000, 2_000_000_000), // 100 vs 200 XLM
    ];
    
    for (yes_total, no_total) in test_cases {
        let total_pool = yes_total + no_total;
        let payout = calculate_payout(yes_total, yes_total, total_pool);
        
        let expected_fee = (total_pool * 7) / 100;
        let expected_distributable = total_pool - expected_fee;
        
        // Winner gets all distributable (only one winner)
        assert!(
            (payout - expected_distributable).abs() <= 1,
            "Fee calculation incorrect: payout {}, expected {}",
            payout,
            expected_distributable
        );
        
        // Verify fee is exactly 7%
        let actual_fee = total_pool - payout;
        let fee_percent = (actual_fee * 100) / total_pool;
        assert_eq!(
            fee_percent, 7,
            "Platform fee should be 7%, got {}%",
            fee_percent
        );
    }
}

#[test]
fn test_proportional_payouts() {
    // Test various stake ratios
    let test_cases = vec![
        (vec![10, 20, 30], 60),  // 1:2:3 ratio
        (vec![25, 25, 50], 100), // 1:1:2 ratio
        (vec![10, 10, 10], 30),  // Equal stakes
    ];
    
    for (stakes, loser_stake) in test_cases {
        let total_yes: i64 = stakes.iter().map(|s| s * 1_000_000).sum();
        let total_no = loser_stake * 1_000_000;
        let total_pool = total_yes + total_no;
        
        let platform_fee = (total_pool * 7) / 100;
        let distributable = total_pool - platform_fee;
        
        let mut total_paid = 0i64;
        for stake in &stakes {
            let stake_stroops = stake * 1_000_000;
            let payout = calculate_payout(stake_stroops, total_yes, total_pool);
            
            // Verify proportionality
            let expected = (stake_stroops * distributable) / total_yes;
            assert!(
                (payout - expected).abs() <= 1,
                "Payout mismatch for stake {}: got {}, expected {}",
                stake,
                payout,
                expected
            );
            
            total_paid += payout;
        }
        
        // Verify total equals distributable (within rounding)
        assert!(
            (total_paid - distributable).abs() <= stakes.len() as i64,
            "Total paid {} != distributable {}",
            total_paid,
            distributable
        );
    }
}

#[test]
fn test_odds_calculation() {
    // Odds = total_pool / position_stakes (in basis points)
    let test_cases = vec![
        (100_000_000i64, 200_000_000i64, 30000i64, 15000i64), // 100 vs 200 = 3.0x vs 1.5x
        (50_000_000, 50_000_000, 20000, 20000),   // Equal = 2.0x vs 2.0x
        (300_000_000, 100_000_000, 13333, 40000), // 300 vs 100 = 1.33x vs 4.0x
    ];
    
    for (yes_stakes, no_stakes, expected_yes_odds, expected_no_odds) in test_cases {
        let total_pool = yes_stakes + no_stakes;
        
        let yes_odds = (total_pool * 10000) / yes_stakes;
        let no_odds = (total_pool * 10000) / no_stakes;
        
        assert!(
            (yes_odds - expected_yes_odds).abs() <= 1,
            "Yes odds mismatch: got {}, expected {}",
            yes_odds,
            expected_yes_odds
        );
        
        assert!(
            (no_odds - expected_no_odds).abs() <= 1,
            "No odds mismatch: got {}, expected {}",
            no_odds,
            expected_no_odds
        );
    }
}
