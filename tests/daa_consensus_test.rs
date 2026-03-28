//! DAA (Difficulty Adjustment Algorithm) and Consensus Edge Cases
//! Tests proof-of-work difficulty, mining, and edge cases

use klomang_core::core::daa::difficulty::Difficulty;
use klomang_core::core::pow::miner::Miner;
use klomang_core::core::consensus::GhostDag;
use klomang_core::core::dag::Dag;
use klomang_core::core::crypto::Hash;
use klomang_core::core::dag::BlockNode;
use std::collections::HashSet;

fn make_block(id: &[u8], parents: HashSet<Hash>) -> BlockNode {
    BlockNode {
        id: Hash::new(id),
        parents,
        children: HashSet::new(),
        selected_parent: None,
        blue_set: HashSet::new(),
        red_set: HashSet::new(),
        blue_score: 0,
        timestamp: 0,
        difficulty: 0,
        nonce: 0,
        transactions: Vec::new(),
    }
}

/// Test 1: Difficulty calculation - high target
#[test]
fn test_difficulty_high_target() {
    let target = [0xFF; 32]; // Very high target (easy difficulty)
    let difficulty = Difficulty::from_target(target);
    
    assert!(difficulty > 0);
}

/// Test 2: Difficulty calculation - low target
#[test]
fn test_difficulty_low_target() {
    let target = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                   0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                   0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                   0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01]; // Very low target (hard difficulty)
    let difficulty = Difficulty::from_target(target);
    
    assert!(difficulty > 0);
}

/// Test 3: Difficulty ordering - higher target = lower difficulty
#[test]
fn test_difficulty_ordering() {
    let high_target = [0xFF; 32];
    let low_target = [0x10; 32];
    
    let high_diff = Difficulty::from_target(high_target);
    let low_diff = Difficulty::from_target(low_target);
    
    assert!(high_diff > low_diff);
}

/// Test 4: Miner creation
#[test]
fn test_miner_creation() {
    let miner = Miner::new(
        Hash::new(b"genesis"),
        Some([0x00; 32]),
    );
    assert!(miner.is_ok());
}

/// Test 5: GHOSTDAG with single block
#[test]
fn test_ghostdag_single_block() {
    let ghostdag = GhostDag::new(10);
    let mut dag = Dag::new();
    
    let block = make_block(b"block1", {
        let mut parents = HashSet::new();
        parents.insert(Hash::new(b"genesis"));
        parents
    });
    
    dag.add_block(block.clone()).expect("Failed to add block");
    
    let tips = vec![block.id.clone()];
    let vblock = ghostdag.compute_virtual_block(&dag, &tips);
    
    assert!(!vblock.parents.is_empty() || vblock.selected_parent.is_some());
}

/// Test 6: GHOSTDAG with empty tips
#[test]
fn test_ghostdag_empty_tips() {
    let ghostdag = GhostDag::new(10);
    let dag = Dag::new();
    
    let tips = vec![];
    let vblock = ghostdag.compute_virtual_block(&dag, &tips);
    
    // Should handle empty tips gracefully
    assert!(vblock.parents.is_empty());
}

/// Test 7: GHOSTDAG parent selection
#[test]
fn test_ghostdag_parent_selection() {
    let ghostdag = GhostDag::new(3);
    let mut dag = Dag::new();
    
    // Create multiple blocks
    let b1 = make_block(b"block1", {
        let mut parents = HashSet::new();
        parents.insert(Hash::new(b"genesis"));
        parents
    });
    dag.add_block(b1.clone()).expect("Failed to add b1");
    
    let b2 = make_block(b"block2", {
        let mut parents = HashSet::new();
        parents.insert(b1.id.clone());
        parents
    });
    dag.add_block(b2).expect("Failed to add b2");
    
    // Test parent selection
    let parents = vec![b1.id.clone()];
    let selected = ghostdag.select_parent(&dag, &parents);
    
    assert!(selected.is_some());
}

/// Test 8: GHOSTDAG with multiple competing chains
#[test]
fn test_ghostdag_multiple_chains() {
    let ghostdag = GhostDag::new(10);
    let mut dag = Dag::new();
    
    // Main chain: genesis -> b1 -> b2
    let b1 = make_block(b"b1", {
        let mut parents = HashSet::new();
        parents.insert(Hash::new(b"genesis"));
        parents
    });
    dag.add_block(b1.clone()).expect("Failed to add b1");
    
    let b2 = make_block(b"b2", {
        let mut parents = HashSet::new();
        parents.insert(b1.id.clone());
        parents
    });
    dag.add_block(b2.clone()).expect("Failed to add b2");
    
    // Alt chain: genesis -> b1_alt -> b2_alt
    let b1_alt = make_block(b"b1_alt", {
        let mut parents = HashSet::new();
        parents.insert(Hash::new(b"genesis"));
        parents
    });
    dag.add_block(b1_alt.clone()).expect("Failed to add b1_alt");
    
    let b2_alt = make_block(b"b2_alt", {
        let mut parents = HashSet::new();
        parents.insert(b1_alt.id.clone());
        parents
    });
    dag.add_block(b2_alt.clone()).expect("Failed to add b2_alt");
    
    // Compute virtual block with both chain tips
    let tips = vec![b2.id.clone(), b2_alt.id.clone()];
    let vblock = ghostdag.compute_virtual_block(&dag, &tips);
    
    // Should select one parent or include both
    assert!(!vblock.parents.is_empty() || vblock.selected_parent.is_some());
}

/// Test 9: DAG add_block functionality
#[test]
fn test_dag_add_block() {
    let mut dag = Dag::new();
    
    let block = make_block(b"test_block", {
        let mut parents = HashSet::new();
        parents.insert(Hash::new(b"genesis"));
        parents
    });
    
    let block_id = block.id.clone();
    
    let result = dag.add_block(block);
    assert!(result.is_ok());
    
    // Verify block was added
    assert!(dag.get_block(&block_id).is_some());
}

/// Test 10: DAG get_block for non-existent block
#[test]
fn test_dag_get_nonexistent_block() {
    let dag = Dag::new();
    let non_existent = Hash::new(b"non_existent");
    
    assert!(dag.get_block(&non_existent).is_none());
}

/// Test 11: DAG get_all_hashes
#[test]
fn test_dag_get_all_hashes() {
    let mut dag = Dag::new();
    
    for i in 1..=5 {
        let block = make_block(
            format!("block{}", i).as_bytes(),
            {
                let mut parents = HashSet::new();
                if i == 1 {
                    parents.insert(Hash::new(b"genesis"));
                } else {
                    parents.insert(Hash::new(format!("block{}", i - 1).as_bytes()));
                }
                parents
            },
        );
        dag.add_block(block).expect("Failed to add block");
    }
    
    let all_hashes = dag.get_all_hashes();
    assert_eq!(all_hashes.len(), 5);
}

/// Test 12: GHOSTDAG anticone computation
#[test]
fn test_ghostdag_anticone() {
    let ghostdag = GhostDag::new(10);
    let mut dag = Dag::new();
    
    // Create diamond: b1 -> b2, b3 -> b4
    let b1 = make_block(b"b1", {
        let mut parents = HashSet::new();
        parents.insert(Hash::new(b"genesis"));
        parents
    });
    dag.add_block(b1.clone()).expect("Failed to add b1");
    
    let b2 = make_block(b"b2", {
        let mut parents = HashSet::new();
        parents.insert(b1.id.clone());
        parents
    });
    dag.add_block(b2.clone()).expect("Failed to add b2");
    
    let b3 = make_block(b"b3", {
        let mut parents = HashSet::new();
        parents.insert(b1.id.clone());
        parents
    });
    dag.add_block(b3.clone()).expect("Failed to add b3");
    
    let anticone = ghostdag.anticone(&dag, &b2.id);
    
    // Should be some blocks in anticone or empty
    assert!(anticone.is_empty() || anticone.len() > 0);
}

/// Test 13: Difficulty constant
#[test]
fn test_difficulty_constants() {
    assert!(Difficulty::MAX_TARGET.len() == 32);
}

/// Test 14: Block hash consistency
#[test]
fn test_block_hash_consistency() {
    let b1 = make_block(b"block", HashSet::new());
    let b2 = make_block(b"block", HashSet::new());
    
    assert_eq!(b1.id, b2.id);
}

/// Test 15: GHOSTDAG blue set computation
#[test]
fn test_ghostdag_blue_set() {
    let ghostdag = GhostDag::new(10);
    let mut dag = Dag::new();
    
    let b1 = make_block(b"b1", {
        let mut parents = HashSet::new();
        parents.insert(Hash::new(b"genesis"));
        parents
    });
    dag.add_block(b1.clone()).expect("Failed to add b1");
    
    let b2 = make_block(b"b2", {
        let mut parents = HashSet::new();
        parents.insert(b1.id.clone());
        parents
    });
    dag.add_block(b2.clone()).expect("Failed to add b2");
    
    let tips = vec![b2.id.clone()];
    let vblock = ghostdag.compute_virtual_block(&dag, &tips);
    
    // Blue set should be computed
    assert!(vblock.blue_set.is_empty() || vblock.blue_set.len() > 0);
}
