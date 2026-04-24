# 7% Fee Structure Implementation

## Changes Made

### Smart Contract
**File**: `contracts/contracts/p2p-bet/src/lib.rs`
- Updated `collect_fee_internal` function from 2% to 7%
- Fee calculation: `amount * 7 / 100`

### Backend
**File**: `backend/src/routes/p2p_bets.rs`
- Added constant: `PLATFORM_FEE_PERCENTAGE: f64 = 7.0`
- Ready for fee calculations in payout logic

## Fee Breakdown Example

### Scenario: 2 participants, 100 XLM each
```
Total Pool: 200 XLM
Winner's Payout: 200 XLM
Platform Fee (7%): 14 XLM
Winner Receives: 186 XLM
```

### Scenario: 5 participants, various stakes
```
Participant A: 50 XLM (Yes)
Participant B: 100 XLM (Yes)
Participant C: 75 XLM (No)
Participant D: 25 XLM (No)
Participant E: 50 XLM (Yes)

Total Pool: 300 XLM
Winning Side (Yes): 200 XLM staked
Losing Side (No): 100 XLM staked

Distribution:
- Participant A: (50/200) * 300 = 75 XLM - 7% = 69.75 XLM
- Participant B: (100/200) * 300 = 150 XLM - 7% = 139.5 XLM
- Participant E: (50/200) * 300 = 75 XLM - 7% = 69.75 XLM

Total Fees Collected: 21 XLM (7% of 300 XLM)
```

## Fee Configuration Options

### Option 1: Fixed 7% (Current)
```rust
const PLATFORM_FEE_PERCENTAGE: i128 = 7;
```

### Option 2: Tiered Fees (Recommended)
```rust
fn calculate_fee(total_pool: i128) -> i128 {
    if total_pool < 100_0000000 { // < 100 XLM
        total_pool * 10 / 100 // 10%
    } else if total_pool < 1000_0000000 { // < 1000 XLM
        total_pool * 7 / 100 // 7%
    } else {
        total_pool * 5 / 100 // 5%
    }
}
```

### Option 3: Time-Based Fees
```rust
fn calculate_fee(total_pool: i128, duration_hours: u64) -> i128 {
    let base_fee = total_pool * 7 / 100;
    
    if duration_hours < 1 {
        base_fee * 150 / 100 // 10.5% for quick bets
    } else if duration_hours > 168 { // > 1 week
        base_fee * 70 / 100 // 4.9% for long-term bets
    } else {
        base_fee // 7% standard
    }
}
```

### Option 4: Dynamic Fees Based on Participants
```rust
fn calculate_fee(total_pool: i128, participant_count: u32) -> i128 {
    let base_fee = total_pool * 7 / 100;
    
    // Discount for more participants (more liquidity)
    if participant_count >= 10 {
        base_fee * 80 / 100 // 5.6%
    } else if participant_count >= 5 {
        base_fee * 90 / 100 // 6.3%
    } else {
        base_fee // 7%
    }
}
```

## Revenue Projections

### Conservative (100 bets/day)
```
Average bet size: 50 XLM
Average participants: 2
Daily volume: 100 * 50 * 2 = 10,000 XLM
Daily revenue (7%): 700 XLM
Monthly revenue: 21,000 XLM (~$2,100 at $0.10/XLM)
```

### Moderate (500 bets/day)
```
Average bet size: 75 XLM
Average participants: 3
Daily volume: 500 * 75 * 3 = 112,500 XLM
Daily revenue (7%): 7,875 XLM
Monthly revenue: 236,250 XLM (~$23,625)
```

### Aggressive (2000 bets/day)
```
Average bet size: 100 XLM
Average participants: 4
Daily volume: 2000 * 100 * 4 = 800,000 XLM
Daily revenue (7%): 56,000 XLM
Monthly revenue: 1,680,000 XLM (~$168,000)
```

## Competitive Analysis

| Platform | Fee Structure | Notes |
|----------|--------------|-------|
| **PolyPulse** | 7% of pool | Competitive for P2P |
| Polymarket | 2% trading fee | Lower but different model |
| Traditional Sportsbooks | 5-10% vig | Similar range |
| Augur | 1% + gas fees | Lower but complex |
| PredictIt | 10% on profits + 5% withdrawal | Higher total |
| Betfair | 2-5% commission | Lower but established |

## Recommendations

1. **Start with 7% flat** - Simple, competitive
2. **Monitor metrics** - Track bet volume, user complaints
3. **A/B test** - Try 5% vs 7% with different user segments
4. **Add premium tier** - 4% fee for subscribers
5. **Implement tiered fees** - Reward high-volume users

## Next Steps

1. ✅ Update smart contract (DONE)
2. ✅ Update backend constant (DONE)
3. ⏳ Add fee display in UI
4. ⏳ Add fee calculator tool
5. ⏳ Update documentation
6. ⏳ Add admin panel to adjust fees

Want me to implement any of the advanced fee structures?
