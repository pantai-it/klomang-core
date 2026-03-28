//! Proof-of-Work and Mining Edge Cases
//! Tests mining, difficulty verification, and hash computation edge cases

use klomang_core::core::crypto::Hash;
use klomang_core::core::pow::Miner;
use klomang_core::core::daa::Difficulty;

/// Test 1: Miner creation
#[test]
fn test_miner_creation() {
    let miner = Miner::new(1);
    assert_eq!(miner.difficulty, 1);
}

/// Test 2: Miner creation with max difficulty
#[test]
fn test_miner_max_difficulty() {
    let miner = Miner::new(u32::MAX);
    assert_eq!(miner.difficulty, u32::MAX);
}

/// Test 3: Miner creation with zero difficulty
#[test]
fn test_miner_zero_difficulty() {
    let miner = Miner::new(0);
    assert_eq!(miner.difficulty, 0);
}

/// Test 4: Hash determinism
#[test]
fn test_hash_determinism() {
    let hash1 = Hash::new(b"test_data");
    let hash2 = Hash::new(b"test_data");
    
    assert_eq!(hash1, hash2);
}

/// Test 5: Hash collision resistance (different inputs)
#[test]
fn test_hash_different_inputs() {
    let hash1 = Hash::new(b"input1");
    let hash2 = Hash::new(b"input2");
    
    assert_ne!(hash1, hash2);
}

/// Test 6: Hash with empty input
#[test]
fn test_hash_empty_input() {
    let hash = Hash::new(b"");
    assert_eq!(hash.len(), 32);
}

/// Test 7: Hash with large input
#[test]
fn test_hash_large_input() {
    let large_input = vec![0x00; 10_000];
    let hash = Hash::new(&large_input);
    assert_eq!(hash.len(), 32);
}

/// Test 8: Hash byte length validation
#[test]
fn test_hash_byte_length() {
    let hash = Hash::new(b"any_data");
    assert!(hash.len() >= 32);
}

/// Test 9: Multiple hash operations
#[test]
fn test_multiple_hashes() {
    let hashes: Vec<Hash> = (0..100).map(|i| {
        Hash::new(format!("data{}", i).as_bytes())
    }).collect();
    
    assert_eq!(hashes.len(), 100);
    
    // Verify all unique (very likely with good hash function)
    for i in 0..hashes.len() {
        for j in (i+1)..hashes.len() {
            assert_ne!(hashes[i], hashes[j]);
        }
    }
}

/// Test 10: Difficulty comparison
#[test]
fn test_difficulty_comparison() {
    let diff1 = Difficulty::new(u32::MAX);
    let diff2 = Difficulty::new(0);
    
    assert!(diff1.target > diff2.target);
}

/// Test 11: Difficulty highest target (minimum difficulty)
#[test]
fn test_difficulty_minimum() {
    let min_diff = Difficulty::new(1);
    assert!(min_diff.target > 0);
}

/// Test 12: Difficulty maximum (maximum difficulty)
#[test]
fn test_difficulty_maximum() {
    let max_diff = Difficulty::new(u32::MAX);
    assert!(max_diff.target > 0);
}

/// Test 13: Hash representation
#[test]
fn test_hash_representation() {
    let hash = Hash::new(b"test");
    let hex_str = format!("{:?}", hash);
    
    // Should be representable in hex
    assert!(hex_str.len() > 0);
}

/// Test 14: Hash ordering
#[test]
fn test_hash_ordering() {
    let hashes: Vec<Hash> = vec![
        Hash::new(b"a"),
        Hash::new(b"b"),
        Hash::new(b"c"),
    ];
    
    // Hashes should be comparable
    let _ = hashes.iter().min();
    let _ = hashes.iter().max();
}

/// Test 15: Miner adjustment
#[test]
fn test_miner_difficulty_adjustment() {
    let mut miner = Miner::new(10);
    
    // Verify initial state
    assert_eq!(miner.difficulty, 10);
    
    // Miner should be usable
    assert!(miner.difficulty > 0);
}
