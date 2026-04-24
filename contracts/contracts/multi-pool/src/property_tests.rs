// Feature: polypulse-enhancements, Property 1: Multi-Participant Payout Fairness
// For any multi-participant bet with verified outcome, each winner SHALL receive 
// payout proportional to their stake: (user_stake / total_winning_stakes) * (total_pool * 0.93)

#[cfg(test)]
mod property_tests {
    use super::super::*;
    use soroban_sdk::{testutils::Address as _, Env};
    
    // Helper to generate test stakes
    fn generate_stakes(count: usize, base: i128) -> Vec<i128> {
        (0..count)
            .map(|i| base * (i as i128 + 1))
            .collect()
    }
    
    #[test]
    fn property_payout_fairness_two_winners() {
        let env = Env::default();
        let contract_id = env.register_contract(None, MultiPoolContract);
        let client = MultiPoolContractClient::new(&env, &contract_id);
        
        let creator = Address::generate(&env);
        let question = symbol_short!("test?");
        let end_time = env.ledger().timestamp() + 86400;
        
        // Test with various stake combinations
        let test_cases = vec![
            (10_000_000, 20_000_000, 30_000_000),  // 1:2:3 ratio
            (50_000_000, 50_000_000, 100_000_000), // 1:1:2 ratio
            (100_000_000, 200_000_000, 150_000_000), // 2:4:3 ratio
        ];
        
        for (stake1, stake2, stake3) in test_cases {
            let pool_id = client.create_pool(&creator, &question, &end_time);
            
            let p1 = Address::generate(&env);
            let p2 = Address::generate(&env);
            let p3 = Address::generate(&env);
            
            // Yes side
            client.join_pool(&p1, &pool_id, &true, &stake1);
            client.join_pool(&p2, &pool_id, &true, &stake2);
            
            // No side
            client.join_pool(&p3, &pool_id, &false, &stake3);
            
            // Yes wins
            client.verify_outcome(&pool_id, &true);
            
            let payout1 = client.calculate_payout(&pool_id, &p1, &true);
            let payout2 = client.calculate_payout(&pool_id, &p2, &true);
            let payout3 = client.calculate_payout(&pool_id, &p3, &false);
            
            // Verify loser gets 0
            assert_eq!(payout3, 0, "Loser should get 0 payout");
            
            // Calculate expected values
            let total_pool = stake1 + stake2 + stake3;
            let platform_fee = (total_pool * 7) / 100;
            let distributable = total_pool - platform_fee;
            let total_winning = stake1 + stake2;
            
            let expected1 = (stake1 * distributable) / total_winning;
            let expected2 = (stake2 * distributable) / total_winning;
            
            // Verify payouts match expected (within rounding error)
            assert!(
                (payout1 - expected1).abs() <= 1,
                "P1 payout mismatch: got {}, expected {}",
                payout1,
                expected1
            );
            assert!(
                (payout2 - expected2).abs() <= 1,
                "P2 payout mismatch: got {}, expected {}",
                payout2,
                expected2
            );
            
            // Verify total payouts equal distributable (within rounding)
            let total_paid = payout1 + payout2;
            assert!(
                (total_paid - distributable).abs() <= 2,
                "Total payout mismatch: got {}, expected {}",
                total_paid,
                distributable
            );
        }
    }
    
    #[test]
    fn property_payout_fairness_many_winners() {
        let env = Env::default();
        let contract_id = env.register_contract(None, MultiPoolContract);
        let client = MultiPoolContractClient::new(&env, &contract_id);
        
        let creator = Address::generate(&env);
        let question = symbol_short!("test?");
        let end_time = env.ledger().timestamp() + 86400;
        
        let pool_id = client.create_pool(&creator, &question, &end_time);
        
        // Create 5 yes participants with varying stakes
        let yes_participants: Vec<_> = (0..5)
            .map(|_| Address::generate(&env))
            .collect();
        
        let yes_stakes = vec![
            10_000_000,  // 1 XLM
            20_000_000,  // 2 XLM
            30_000_000,  // 3 XLM
            40_000_000,  // 4 XLM
            50_000_000,  // 5 XLM
        ];
        
        // Create 3 no participants
        let no_participants: Vec<_> = (0..3)
            .map(|_| Address::generate(&env))
            .collect();
        
        let no_stakes = vec![
            50_000_000,  // 5 XLM
            60_000_000,  // 6 XLM
            40_000_000,  // 4 XLM
        ];
        
        // Join pool
        for (addr, stake) in yes_participants.iter().zip(yes_stakes.iter()) {
            client.join_pool(addr, &pool_id, &true, stake);
        }
        
        for (addr, stake) in no_participants.iter().zip(no_stakes.iter()) {
            client.join_pool(addr, &pool_id, &false, stake);
        }
        
        // Yes wins
        client.verify_outcome(&pool_id, &true);
        
        // Calculate payouts
        let mut total_paid = 0i128;
        let total_yes: i128 = yes_stakes.iter().sum();
        let total_no: i128 = no_stakes.iter().sum();
        let total_pool = total_yes + total_no;
        let platform_fee = (total_pool * 7) / 100;
        let distributable = total_pool - platform_fee;
        
        for (addr, stake) in yes_participants.iter().zip(yes_stakes.iter()) {
            let payout = client.calculate_payout(&pool_id, addr, &true);
            let expected = (stake * distributable) / total_yes;
            
            assert!(
                (payout - expected).abs() <= 1,
                "Payout mismatch for stake {}: got {}, expected {}",
                stake,
                payout,
                expected
            );
            
            total_paid += payout;
        }
        
        // Verify no participants get 0
        for addr in no_participants.iter() {
            let payout = client.calculate_payout(&pool_id, addr, &false);
            assert_eq!(payout, 0, "Loser should get 0");
        }
        
        // Verify total paid equals distributable (within rounding)
        assert!(
            (total_paid - distributable).abs() <= yes_participants.len() as i128,
            "Total paid {} != distributable {}",
            total_paid,
            distributable
        );
    }
    
    #[test]
    fn property_platform_fee_consistency() {
        let env = Env::default();
        let contract_id = env.register_contract(None, MultiPoolContract);
        let client = MultiPoolContractClient::new(&env, &contract_id);
        
        let creator = Address::generate(&env);
        let question = symbol_short!("test?");
        let end_time = env.ledger().timestamp() + 86400;
        
        // Test various pool sizes
        let test_pools = vec![
            (100_000_000, 100_000_000),   // 10 XLM each side
            (500_000_000, 300_000_000),   // 50 vs 30 XLM
            (1_000_000_000, 2_000_000_000), // 100 vs 200 XLM
        ];
        
        for (yes_total, no_total) in test_pools {
            let pool_id = client.create_pool(&creator, &question, &end_time);
            
            let p1 = Address::generate(&env);
            let p2 = Address::generate(&env);
            
            client.join_pool(&p1, &pool_id, &true, &yes_total);
            client.join_pool(&p2, &pool_id, &false, &no_total);
            
            client.verify_outcome(&pool_id, &true);
            
            let payout = client.calculate_payout(&pool_id, &p1, &true);
            
            let total_pool = yes_total + no_total;
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
    fn property_odds_calculation() {
        let env = Env::default();
        let contract_id = env.register_contract(None, MultiPoolContract);
        let client = MultiPoolContractClient::new(&env, &contract_id);
        
        let creator = Address::generate(&env);
        let question = symbol_short!("test?");
        let end_time = env.ledger().timestamp() + 86400;
        
        let pool_id = client.create_pool(&creator, &question, &end_time);
        
        let p1 = Address::generate(&env);
        let p2 = Address::generate(&env);
        
        // 100 XLM on Yes, 200 XLM on No
        client.join_pool(&p1, &pool_id, &true, &1_000_000_000);
        client.join_pool(&p2, &pool_id, &false, &2_000_000_000);
        
        let (yes_odds, no_odds) = client.get_odds(&pool_id);
        
        // Total pool: 300 XLM
        // Yes odds: 300/100 = 3.0x = 30000 basis points
        // No odds: 300/200 = 1.5x = 15000 basis points
        
        assert_eq!(yes_odds, 30000, "Yes odds should be 3.0x");
        assert_eq!(no_odds, 15000, "No odds should be 1.5x");
    }
}
