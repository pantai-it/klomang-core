use crate::core::dag::BlockNode;
use crate::core::state::storage::Storage;
use crate::core::state::transaction::Transaction;
use crate::core::state::utxo::UtxoSet;
use crate::core::state::v_trie::VerkleTree;

/// Minimal state container exposing the current Verkle root.
#[derive(Debug, Clone)]
pub struct State {
    pub root: [u8; 32],
}

/// Snapshot of the chain state at a specific block height.
#[derive(Debug, Clone)]
pub struct StateSnapshot {
    pub height: u64,
    pub root: [u8; 32],
}

/// Error types untuk StateManager operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateManagerError {
    InvalidRollback(String),
    SnapshotNotFound(u64),
    ApplyBlockFailed(String),
    RestoreFailed(String),
    CryptographicError(String),
}

/// Basic state manager for applying blocks, tracking snapshots, and rolling back.
#[derive(Debug)]
pub struct StateManager<S: Storage + Clone> {
    pub tree: VerkleTree<S>,
    pub current_height: u64,
    pub snapshots: Vec<StateSnapshot>,
    snapshot_storages: Vec<S>,
}

impl<S: Storage + Clone> StateManager<S> {
    pub fn new(tree: VerkleTree<S>) -> Result<Self, StateManagerError> {
        let root = tree.get_root()
            .map_err(|e| StateManagerError::CryptographicError(format!("Failed to get root: {}", e)))?;
        let storage_snapshot = tree.storage_clone();

        Ok(Self {
            tree,
            current_height: 0,
            snapshots: vec![StateSnapshot { height: 0, root }],
            snapshot_storages: vec![storage_snapshot],
        })
    }

    /// Apply block dengan full error handling dan DAG reorganization support
    pub fn apply_block(&mut self, block: &BlockNode, utxo: &mut UtxoSet) -> Result<(), StateManagerError> {
        // Process transactions untuk state update
        for tx in &block.transactions {
            self.apply_transaction(tx, utxo)?;
        }

        self.current_height += 1;

        // Create snapshot setelah semua transactions applied
        let new_root = self.get_root_hash()?;
        self.snapshots.push(StateSnapshot {
            height: self.current_height,
            root: new_root,
        });
        self.snapshot_storages.push(self.tree.storage_clone());

        Ok(())
    }

    /// Apply transaction dengan error handling untuk validation
    fn apply_transaction(&mut self, tx: &Transaction, utxo: &mut UtxoSet) -> Result<(), StateManagerError> {
        // Process inputs (remove from UTXO set)
        for input in &tx.inputs {
            let key = (input.prev_tx.clone(), input.index);
            utxo.utxos.remove(&key);
        }

        // Process outputs (add to UTXO set dan tree)
        for (i, output) in tx.outputs.iter().enumerate() {
            let key = tx.hash_with_index(i as u32);
            utxo.utxos.insert((tx.id.clone(), i as u32), output.clone());
            self.tree.insert(key, output.serialize());
        }

        Ok(())
    }

    /// Get root hash dari current state
    pub fn get_root_hash(&self) -> Result<[u8; 32], StateManagerError> {
        self.tree.get_root()
            .map_err(|e| StateManagerError::CryptographicError(format!("Failed to get root: {}", e)))
    }

    /// Get state snapshot pada specific height
    pub fn get_state_at(&self, height: u64) -> Option<&StateSnapshot> {
        self.snapshots.iter().find(|s| s.height == height)
    }

    /// Rollback state ke target height dengan error handling
    pub fn rollback_state(&mut self, target_height: u64) -> Result<(), StateManagerError> {
        // Validation
        if target_height > self.current_height {
            return Err(StateManagerError::InvalidRollback(
                format!("Cannot rollback to height {} when current height is {}", 
                        target_height, self.current_height)
            ));
        }

        // Check snapshot exists
        if self.get_state_at(target_height).is_none() {
            return Err(StateManagerError::SnapshotNotFound(target_height));
        }

        // Truncate snapshots dan storages
        self.snapshots.truncate(target_height as usize + 1);
        self.snapshot_storages.truncate(target_height as usize + 1);
        self.current_height = target_height;

        // Restore tree dari snapshot storage
        let snapshot_storage = self
            .snapshot_storages
            .get(target_height as usize)
            .ok_or_else(|| StateManagerError::RestoreFailed(
                "Snapshot storage missing after truncation".to_string()
            ))?;

        self.tree = VerkleTree::new(snapshot_storage.clone())
            .map_err(|e| StateManagerError::RestoreFailed(format!("Failed to restore tree: {}", e)))?;
        
        // Verify restoration
        let restored_root = self.get_root_hash()?;
        let snapshot_root = self.snapshots[target_height as usize].root;
        
        if restored_root != snapshot_root {
            return Err(StateManagerError::RestoreFailed(
                format!("Root mismatch after rollback: expected {:?}, got {:?}", 
                        snapshot_root, restored_root)
            ));
        }

        Ok(())
    }

    /// Legacy rollback method - should use rollback_state() instead
    pub fn rollback(&mut self, target_height: u64) {
        assert!(target_height <= self.current_height);

        self.snapshots.truncate(target_height as usize + 1);
        self.snapshot_storages.truncate(target_height as usize + 1);
        self.current_height = target_height;

        let snapshot_storage = self
            .snapshot_storages
            .get(target_height as usize)
            .expect("rollback snapshot missing");

        // NOTE: VerkleTree is not fully versioned; reset to the storage snapshot.
        if let Ok(tree) = VerkleTree::new(snapshot_storage.clone()) {
            self.tree = tree;
        } else {
            panic!("Failed to restore VerkleTree during rollback");
        }
    }

    /// Restore entire state dari specific snapshot
    pub fn restore_from_snapshot(&mut self, snapshot_root: [u8; 32], height: u64) -> Result<(), StateManagerError> {
        let snapshot_idx = self.snapshots.iter()
            .position(|s| s.height == height && s.root == snapshot_root)
            .ok_or_else(|| StateManagerError::SnapshotNotFound(height))?;

        let storage = self.snapshot_storages
            .get(snapshot_idx)
            .ok_or_else(|| StateManagerError::RestoreFailed("Storage missing".to_string()))?;

        self.tree = VerkleTree::new(storage.clone())
            .map_err(|e| StateManagerError::RestoreFailed(format!("Failed to restore tree: {}", e)))?;
        self.current_height = height;
        
        // Truncate snapshots ke restore point
        self.snapshots.truncate(snapshot_idx + 1);
        self.snapshot_storages.truncate(snapshot_idx + 1);

        Ok(())
    }

    /// Get current state snapshot
    pub fn get_current_state(&self) -> Result<StateSnapshot, StateManagerError> {
        let root = self.tree.get_root()
            .map_err(|e| StateManagerError::CryptographicError(format!("Failed to get root: {}", e)))?;
        Ok(StateSnapshot {
            height: self.current_height,
            root,
        })
    }

    /// Validate snapshot consistency
    pub fn validate_snapshots(&self) -> Result<(), StateManagerError> {
        for (i, snapshot) in self.snapshots.iter().enumerate() {
            if snapshot.height != i as u64 {
                return Err(StateManagerError::ApplyBlockFailed(
                    format!("Snapshot height mismatch at index {}", i)
                ));
            }
        }

        if self.snapshots.len() != self.snapshot_storages.len() {
            return Err(StateManagerError::ApplyBlockFailed(
                "Snapshot and storage length mismatch".to_string()
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::crypto::Hash;
    use crate::core::dag::BlockNode;
    use crate::core::state::storage::MemoryStorage;
    use crate::core::state::transaction::{TxOutput, Transaction};
    use std::collections::HashSet;

    fn make_coinbase_transaction(value: u64, pubkey_hash: Hash) -> Transaction {
        Transaction::new(
            Vec::new(),
            vec![TxOutput {
                value,
                pubkey_hash,
            }],
        )
    }

    fn make_block(id_bytes: &[u8], transactions: Vec<Transaction>) -> BlockNode {
        BlockNode {
            id: Hash::new(id_bytes),
            parents: HashSet::new(),
            children: HashSet::new(),
            selected_parent: None,
            blue_set: HashSet::new(),
            red_set: HashSet::new(),
            blue_score: 0,
            timestamp: 0,
            difficulty: 0,
            nonce: 0,
            transactions,
        }
    }

    #[test]
    fn test_state_manager_apply_block() {
        let storage = MemoryStorage::new();
        let tree = VerkleTree::new(storage).expect("failed to create VerkleTree");
        let mut manager = StateManager::new(tree).expect("failed to create StateManager");
        let mut utxo = UtxoSet::new();

        let tx = make_coinbase_transaction(42, Hash::new(b"alice"));
        let block = make_block(b"block-1", vec![tx.clone()]);

        let root_before = manager.tree.get_root().expect("failed to get root");
        manager.apply_block(&block, &mut utxo).expect("apply block failed");
        let root_after = manager.tree.get_root().expect("failed to get root");

        assert_ne!(root_before, root_after);
        assert_eq!(manager.current_height, 1);
        assert_eq!(manager.snapshots.len(), 2);
        assert_eq!(utxo.utxos.len(), 1);
        assert_eq!(manager.get_state_at(1).unwrap().root, root_after);
    }

    #[test]
    fn test_state_manager_snapshot() {
        let storage = MemoryStorage::new();
        let tree = VerkleTree::new(storage).expect("failed to create VerkleTree");
        let mut manager = StateManager::new(tree).expect("failed to create StateManager");
        let mut utxo = UtxoSet::new();

        let block1 = make_block(b"block-1", vec![make_coinbase_transaction(10, Hash::new(b"alice"))]);
        manager.apply_block(&block1, &mut utxo).expect("apply block failed");
        let snapshot1 = manager.get_state_at(1).expect("snapshot at height 1");
        let snapshot1_root = snapshot1.root;
        let snapshot1_height = snapshot1.height;

        let block2 = make_block(b"block-2", vec![make_coinbase_transaction(20, Hash::new(b"bob"))]);
        manager.apply_block(&block2, &mut utxo).expect("apply block failed");
        let snapshot2 = manager.get_state_at(2).expect("snapshot at height 2");

        assert_ne!(snapshot1_root, snapshot2.root);
        assert_eq!(snapshot1_height, 1);
        assert_eq!(snapshot2.height, 2);
    }

    #[test]
    fn test_state_manager_rollback() {
        let storage = MemoryStorage::new();
        let tree = VerkleTree::new(storage).expect("failed to create VerkleTree");
        let mut manager = StateManager::new(tree).expect("failed to create StateManager");
        let mut utxo = UtxoSet::new();

        let block1 = make_block(b"block-1", vec![make_coinbase_transaction(10, Hash::new(b"alice"))]);
        manager.apply_block(&block1, &mut utxo).expect("apply block failed");
        let root1 = manager.tree.get_root().expect("failed to get root");

        let block2 = make_block(b"block-2", vec![make_coinbase_transaction(20, Hash::new(b"bob"))]);
        manager.apply_block(&block2, &mut utxo).expect("apply block failed");
        let root2 = manager.tree.get_root().expect("failed to get root");

        assert_ne!(root1, root2);
        assert_eq!(manager.current_height, 2);

        manager.rollback(1);

        assert_eq!(manager.current_height, 1);
        assert_eq!(manager.snapshots.len(), 2);
        assert_eq!(manager.get_state_at(1).unwrap().root, root1);
        assert_eq!(manager.tree.get_root().expect("failed to get root"), root1);
    }

    #[test]
    fn test_state_manager_rollback_state_result() {
        let storage = MemoryStorage::new();
        let tree = VerkleTree::new(storage).expect("failed to create VerkleTree");
        let mut manager = StateManager::new(tree).expect("failed to create StateManager");
        let mut utxo = UtxoSet::new();

        let block1 = make_block(b"block-1", vec![make_coinbase_transaction(10, Hash::new(b"alice"))]);
        manager.apply_block(&block1, &mut utxo).expect("apply block failed");

        let block2 = make_block(b"block-2", vec![make_coinbase_transaction(20, Hash::new(b"bob"))]);
        manager.apply_block(&block2, &mut utxo).expect("apply block failed");

        // Successful rollback
        let result = manager.rollback_state(1);
        assert!(result.is_ok());
        assert_eq!(manager.current_height, 1);

        // Valid rollback to same height
        let result = manager.rollback_state(1);
        assert!(result.is_ok());

        // Invalid rollback to future height
        let result = manager.rollback_state(5);
        assert!(result.is_err());
        match result {
            Err(StateManagerError::InvalidRollback(_)) => {},
            _ => panic!("Expected InvalidRollback error"),
        }
    }

    #[test]
    fn test_state_manager_get_root_hash() {
        let storage = MemoryStorage::new();
        let tree = VerkleTree::new(storage).expect("failed to create VerkleTree");
        let manager = StateManager::new(tree).expect("failed to create StateManager");

        let root = manager.get_root_hash().expect("failed to get root hash");
        assert_eq!(root.len(), 32);
        assert_eq!(root, manager.snapshots[0].root);
    }

    #[test]
    fn test_state_manager_restore_from_snapshot() {
        let storage = MemoryStorage::new();
        let tree = VerkleTree::new(storage).expect("failed to create VerkleTree");
        let mut manager = StateManager::new(tree).expect("failed to create StateManager");
        let mut utxo = UtxoSet::new();

        let block1 = make_block(b"block-1", vec![make_coinbase_transaction(10, Hash::new(b"alice"))]);
        manager.apply_block(&block1, &mut utxo).expect("apply block failed");
        let snapshot1_root = manager.get_state_at(1).unwrap().root;

        let block2 = make_block(b"block-2", vec![make_coinbase_transaction(20, Hash::new(b"bob"))]);
        manager.apply_block(&block2, &mut utxo).expect("apply block failed");

        // Restore to height 1
        let result = manager.restore_from_snapshot(snapshot1_root, 1);
        assert!(result.is_ok());
        assert_eq!(manager.current_height, 1);
        assert_eq!(manager.get_root_hash().expect("failed to get root hash"), snapshot1_root);
    }

    #[test]
    fn test_state_manager_validate_snapshots() {
        let storage = MemoryStorage::new();
        let tree = VerkleTree::new(storage).expect("failed to create VerkleTree");
        let mut manager = StateManager::new(tree).expect("failed to create StateManager");
        let mut utxo = UtxoSet::new();

        // Valid snapshots
        assert!(manager.validate_snapshots().is_ok());

        let block1 = make_block(b"block-1", vec![make_coinbase_transaction(10, Hash::new(b"alice"))]);
        manager.apply_block(&block1, &mut utxo).expect("apply block failed");
        assert!(manager.validate_snapshots().is_ok());

        let block2 = make_block(b"block-2", vec![make_coinbase_transaction(20, Hash::new(b"bob"))]);
        manager.apply_block(&block2, &mut utxo).expect("apply block failed");
        assert!(manager.validate_snapshots().is_ok());
    }

    #[test]
    fn test_state_manager_dag_reorganization() {
        let storage = MemoryStorage::new();
        let tree = VerkleTree::new(storage).expect("failed to create VerkleTree");
        let mut manager = StateManager::new(tree).expect("failed to create StateManager");
        let mut utxo = UtxoSet::new();

        // Apply block on main chain
        let block1_main = make_block(b"block-1-main", vec![make_coinbase_transaction(10, Hash::new(b"alice"))]);
        manager.apply_block(&block1_main, &mut utxo).expect("apply block failed");
        let root_1_main = manager.get_root_hash().expect("failed to get root hash");

        let block2_main = make_block(b"block-2-main", vec![make_coinbase_transaction(20, Hash::new(b"bob"))]);
        manager.apply_block(&block2_main, &mut utxo).expect("apply block failed");

        // DAG reorganization - rollback to height 1 and apply different chain
        manager.rollback_state(1).expect("rollback failed");

        let block2_alt = make_block(b"block-2-alt", vec![make_coinbase_transaction(15, Hash::new(b"charlie"))]);
        manager.apply_block(&block2_alt, &mut utxo).expect("apply block failed");
        let root_2_alt = manager.get_root_hash().expect("failed to get root hash");

        // Verify different root after reorg
        assert_ne!(root_1_main, root_2_alt);
        assert_eq!(manager.current_height, 2);
    }

    #[test]
    fn test_state_manager_multiple_snapshots() {
        let storage = MemoryStorage::new();
        let tree = VerkleTree::new(storage).expect("failed to create VerkleTree");
        let mut manager = StateManager::new(tree).expect("failed to create StateManager");
        let mut utxo = UtxoSet::new();

        // Create multiple blocks and snapshots
        for i in 1..=5 {
            let block = make_block(
                format!("block-{}", i).as_bytes(),
                vec![make_coinbase_transaction(10 * i as u64, Hash::new(format!("user-{}", i).as_bytes()))],
            );
            manager.apply_block(&block, &mut utxo).expect("apply block failed");
        }

        assert_eq!(manager.snapshots.len(), 6); // genesis + 5 blocks
        assert_eq!(manager.current_height, 5);

        // Verify snapshot progression
        for i in 0..=5 {
            let snapshot = manager.get_state_at(i as u64).expect("snapshot missing");
            assert_eq!(snapshot.height, i as u64);
        }

        // Rollback to middle
        manager.rollback_state(3).expect("rollback failed");
        assert_eq!(manager.snapshots.len(), 4); // genesis + 3 blocks
        assert_eq!(manager.current_height, 3);
    }
}
