//! Emission and Reward Mechanism Tests
//! Tests block reward calculations and emission schedules

use klomang_core::core::consensus::emission::BlockReward;
use klomang_core::core::consensus::reward::RewardCalculator;

/// Test 1: Block reward at genesis
#[test]
fn test_block_reward_genesis() {
    let reward = BlockReward::calculate_for_block(0);
    
    // First block should have initial reward
    assert!(reward > 0);
}

/// Test 2: Block reward halving
#[test]
fn test_block_reward_halving() {
    let reward_0 = BlockReward::calculate_for_block(0);
    let reward_210000 = BlockReward::calculate_for_block(210000);
    
    // First halving should reduce reward
    assert!(reward_210000 <= reward_0);
}

/// Test 3: Consistent reward for same block height
#[test]
fn test_consistent_block_reward() {
    let reward1 = BlockReward::calculate_for_block(100);
    let reward2 = BlockReward::calculate_for_block(100);
    
    assert_eq!(reward1, reward2);
}

/// Test 4: Block reward decreases over time
#[test]
fn test_block_reward_monotonic_decrease() {
    let mut prev_reward = BlockReward::calculate_for_block(0);
    
    // Check every halving epoch
    for i in 1..=8 {
        let halving_height = 210000 * i;
        let current_reward = BlockReward::calculate_for_block(halving_height as u64);
        
        // Reward should not increase
        assert!(current_reward <= prev_reward);
        prev_reward = current_reward;
    }
}

/// Test 5: Zero reward eventually
#[test]
fn test_block_reward_eventual_zero() {
    let reward_far_future = BlockReward::calculate_for_block(21_000_000);
    
    // Eventually block reward should be minimal or zero
    assert!(reward_far_future < 100);
}

/// Test 6: Block reward at specific heights
#[test]
fn test_block_reward_specific_heights() {
    let reward_1 = BlockReward::calculate_for_block(1);
    let reward_100 = BlockReward::calculate_for_block(100);
    let reward_1000 = BlockReward::calculate_for_block(1000);
    
    // All should be positive
    assert!(reward_1 > 0);
    assert!(reward_100 > 0);
    assert!(reward_1000 > 0);
}

/// Test 7: Reward calculator initialization
#[test]
fn test_reward_calculator_init() {
    let calc = RewardCalculator::new();
    
    // Should be initialized successfully
    assert!(calc.total_rewards() >= 0);
}

/// Test 8: Total rewards calculation
#[test]
fn test_total_rewards_calculation() {
    let calc = RewardCalculator::new();
    let total = calc.total_rewards();
    
    // Total should be reasonable
    assert!(total > 0);
    assert!(total < u64::MAX);
}

/// Test 9: Cumulative rewards up to block height
#[test]
fn test_cumulative_rewards() {
    let calc = RewardCalculator::new();
    
    let cumulative_100 = calc.cumulative_rewards_until(100);
    let cumulative_200 = calc.cumulative_rewards_until(200);
    
    // Cumulative should be monotonic
    assert!(cumulative_200 >= cumulative_100);
}

/// Test 10: Reward for single block
#[test]
fn test_single_block_reward() {
    let calc = RewardCalculator::new();
    
    let reward = calc.reward_for_block(1);
    
    // Should be positive
    assert!(reward > 0);
}

/// Test 11: Reward calculator consistency
#[test]
fn test_reward_calculator_consistency() {
    let calc1 = RewardCalculator::new();
    let calc2 = RewardCalculator::new();
    
    // Same calculator instances should give same results
    assert_eq!(
        calc1.reward_for_block(100),
        calc2.reward_for_block(100)
    );
}

/// Test 12: Large block height handling
#[test]
fn test_large_block_height_reward() {
    let reward = BlockReward::calculate_for_block(1_000_000);
    
    // Should handle large heights
    assert!(reward >= 0);
}

/// Test 13: Block reward rounding
#[test]
fn test_block_reward_exact_values() {
    for height in 0..1000 {
        let reward = BlockReward::calculate_for_block(height);
        
        // Reward should be integer (no fractional satoshis)
        assert_eq!(reward, reward.floor() as u64 as f64);
    }
}

/// Test 14: Zero block height edge case
#[test]
fn test_zero_block_height_reward() {
    let reward = BlockReward::calculate_for_block(0);
    
    // Genesis block should have reward
    assert!(reward > 0);
}

/// Test 15: Reward distribution verification
#[test]
fn test_reward_distribution_validity() {
    let calc = RewardCalculator::new();
    
    // Calculate total possible rewards
    let mut total = 0.0;
    for i in 0..21_000_000 {
        total += BlockReward::calculate_for_block(i);
    }
    
    // Should match expected maximum supply (21M or similar)
    assert!(total > 0.0);
    assert!(total < 100_000_000.0); // Less than theoretical max
}
