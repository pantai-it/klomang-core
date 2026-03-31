use crate::core::dag::BlockNode;
use crate::core::state::storage::Storage;
use crate::core::state::transaction::Transaction;
use crate::core::state::utxo::{OutPoint, UtxoSet};
use crate::core::state::v_trie::VerkleTree;
use crate::core::state::PruneMarker;
use crate::core::vm::VMExecutor;
use crate::core::consensus::{economic_constants, ghostdag::GhostDag};
use crate::core::dag::Dag;
use crate::core::errors::CoreError;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

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
    pub total_supply: u128, // Track total supply for validation
    pub gas_fees: Vec<GasFeeWitness>, // Gas fee witnesses for the block
}

/// Gas fee distribution witness for 80/20 validation
#[derive(Debug, Clone)]
pub struct GasFeeWitness {
    pub total_gas_fee: u128,
    pub miner_share: u128,
    pub fullnode_share: u128,
}

/// Error types untuk StateManager operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateManagerError {
    InvalidRollback(String),
    SnapshotNotFound(u64),
    ApplyBlockFailed(String),
    RestoreFailed(String),
    CryptographicError(String),
    SupplyCapExceeded(String),
    BurnAddressViolation(String),
}

impl std::fmt::Display for StateManagerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StateManagerError::InvalidRollback(msg) => write!(f, "Invalid rollback: {}", msg),
            StateManagerError::SnapshotNotFound(height) => write!(f, "Snapshot not found at height {}", height),
            StateManagerError::ApplyBlockFailed(msg) => write!(f, "Apply block failed: {}", msg),
            StateManagerError::RestoreFailed(msg) => write!(f, "Restore failed: {}", msg),
            StateManagerError::CryptographicError(msg) => write!(f, "Cryptographic error: {}", msg),
            StateManagerError::SupplyCapExceeded(msg) => write!(f, "Supply cap exceeded: {}", msg),
            StateManagerError::BurnAddressViolation(msg) => write!(f, "Burn address violation: {}", msg),
        }
    }
}

/// Basic state manager for applying blocks, tracking snapshots, and rolling back.
#[derive(Debug)]
pub struct StateManager<S: Storage + Clone> {
    pub tree: VerkleTree<S>,
    pub current_height: u64,
    pub snapshots: Vec<StateSnapshot>,
    pub snapshot_storages: Vec<S>,
    pub prune_markers: HashMap<OutPoint, PruneMarker>,
    pub outpoint_to_key: HashMap<OutPoint, [u8; 32]>,
    pub current_total_supply: u128, // Running total supply tracker
    pub block_gas_fees: Vec<GasFeeWitness>, // Gas fee witnesses per block
    pub pending_updates: Vec<([u8; 32], Vec<u8>)>, // Pending state updates for atomic application
    /// Atomic operation flag to prevent concurrent state modifications
    pub applying_block: std::sync::atomic::AtomicBool,
    /// Guard untuk prevent race condition saat apply_block dijalankan bersamaan
    pub apply_lock: std::sync::Mutex<()>,
}

impl<S: Storage + Clone + Send + Sync + 'static> StateManager<S> {
    pub fn new(tree: VerkleTree<S>) -> Result<Self, StateManagerError> {
        let root = tree.get_root()
            .map_err(|e| StateManagerError::CryptographicError(format!("Failed to get root: {}", e)))?;
        let storage_snapshot = tree.storage_clone();

        Ok(Self {
            tree,
            current_height: 0,
            snapshots: vec![StateSnapshot { height: 0, root, total_supply: 0, gas_fees: Vec::new() }],
            snapshot_storages: vec![storage_snapshot],
            prune_markers: HashMap::new(),
            outpoint_to_key: HashMap::new(),
            current_total_supply: 0,
            block_gas_fees: Vec::new(),
            pending_updates: Vec::new(),
            applying_block: std::sync::atomic::AtomicBool::new(false),
            apply_lock: std::sync::Mutex::new(()),
        })
    }

    /// Atomic operation - prevents concurrent modifications
    pub fn apply_block(&mut self, block: &BlockNode, utxo: &mut UtxoSet) -> Result<(), StateManagerError> {
        // Acquire exclusive lock when starting block application
        let lock_ptr = &self.apply_lock as *const std::sync::Mutex<()>;
        let _apply_guard = unsafe { (&*lock_ptr).lock().map_err(|e| StateManagerError::ApplyBlockFailed(
            format!("Failed to acquire apply lock: {}", e)
        ))? };

        // Check if another block application is already in progress
        if self.applying_block.load(std::sync::atomic::Ordering::SeqCst) {
            return Err(StateManagerError::ApplyBlockFailed(
                "Concurrent block application not allowed".to_string()
            ));
        }

        // Set atomic flag and ensure it resets in Drop guard
        self.applying_block.store(true, std::sync::atomic::Ordering::SeqCst);

        let applying_block_ptr = &self.applying_block as *const std::sync::atomic::AtomicBool;
        struct AtomicFlagGuard(*const std::sync::atomic::AtomicBool);
        impl Drop for AtomicFlagGuard {
            fn drop(&mut self) {
                unsafe {
                    (*self.0).store(false, std::sync::atomic::Ordering::SeqCst);
                }
            }
        }
        let _guard = AtomicFlagGuard(applying_block_ptr);

        // Clear pending updates for new block
        self.pending_updates.clear();

        // Reset gas fees untuk block baru
        self.block_gas_fees.clear();

        // Process transactions untuk state update
        for tx in &block.transactions {
            self.apply_transaction(tx, utxo)?;
        }

        // Apply all pending updates atomically with anti-burn and supply cap checks
        self.tree.apply_state_transition(self.pending_updates.clone(), self.current_total_supply)
            .map_err(|e| StateManagerError::ApplyBlockFailed(format!("State transition failed: {}", e)))?;

        self.current_height += 1;

        // Create snapshot setelah semua transactions applied
        let new_root = self.get_root_hash()?;
        self.snapshots.push(StateSnapshot {
            height: self.current_height,
            root: new_root,
            total_supply: self.current_total_supply,
            gas_fees: self.block_gas_fees.clone(),
        });
        self.snapshot_storages.push(self.tree.storage_clone());

        Ok(())
    }

    /// Validate block with consensus rules and apply atomically
    /// This ensures consensus validation happens before any state changes
    pub fn validate_and_apply_block(
        &mut self,
        block: &BlockNode,
        utxo: &mut UtxoSet,
        dag: &Dag,
        consensus: &GhostDag
    ) -> Result<(), StateManagerError> {
        // Get current time for validation
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| StateManagerError::CryptographicError(format!("Time error: {}", e)))?
            .as_secs();

        // First, validate block with consensus rules
        consensus.validate_block(block, dag, &self.tree, current_time)?;

        // Check finality constraints for reorganization
        if !consensus.can_reorganize(dag, &block.id)? {
            return Err(StateManagerError::ApplyBlockFailed(
                "Block reorganization would violate finality constraints".to_string()
            ));
        }

        // Only after consensus validation passes, apply the block
        self.apply_block(block, utxo)
    }

    /// Apply transaction dengan error handling untuk validation
    pub fn apply_transaction(&mut self, tx: &Transaction, utxo: &mut UtxoSet) -> Result<(), StateManagerError> {
        if tx.execution_payload.is_empty() && tx.contract_address.is_none() {
            self.apply_utxo_transaction(tx, utxo)
        } else {
            self.apply_contract_transition(tx, utxo)
        }
    }

    /// Internal helper: apply standar UTXO-only transaction
    fn apply_utxo_transaction(&mut self, tx: &Transaction, utxo: &mut UtxoSet) -> Result<(), StateManagerError> {
        // Process inputs (remove from UTXO set and subtract from total supply)
        for input in &tx.inputs {
            let outpoint = (input.prev_tx.clone(), input.index);
            if let Some(output) = utxo.utxos.get(&outpoint) {
                // Subtract spent amount from total supply
                self.current_total_supply = self.current_total_supply.saturating_sub(output.value as u128);
            }
            utxo.utxos.remove(&outpoint);
            self.outpoint_to_key.remove(&outpoint);
            self.prune_markers.remove(&outpoint);
        }

        // Process outputs (add to UTXO set dan tree)
        for (i, output) in tx.outputs.iter().enumerate() {
            let key = tx.hash_with_index(i as u32);
            let outpoint = (tx.id.clone(), i as u32);
            utxo.utxos.insert(outpoint.clone(), output.clone());
            self.pending_updates.push((key, output.serialize()));
            self.outpoint_to_key.insert(outpoint.clone(), key);

            // Add new output amount to total supply
            self.current_total_supply = self.current_total_supply.saturating_add(output.value as u128);
        }

        Ok(())
    }

    /// Internal helper: apply contract execution transaction
    fn apply_contract_transition(&mut self, tx: &Transaction, utxo: &mut UtxoSet) -> Result<(), StateManagerError> {
        // Gas validation based on intrinsic + calldata scoring.
        let payload_data_cost: u64 = tx.execution_payload.iter().fold(0, |acc, byte| {
            acc + if *byte == 0 { 4 } else { 16 }
        });
        let intrinsic_cost: u64 = 21_000;
        let required_gas = intrinsic_cost.saturating_add(payload_data_cost);

        if tx.gas_limit < required_gas {
            return Err(StateManagerError::ApplyBlockFailed(format!(
                "Insufficient gas limit ({}), required {} for intrinsic+payload cost {} bytes",
                tx.gas_limit,
                required_gas,
                tx.execution_payload.len()
            )));
        }

        // Snapshot state tree for rollback
        let tree_snapshot = self.tree.clone();

        // Execute contract payload using VMExecutor
        let exec_res = VMExecutor::execute(&tx.execution_payload, self, [0u8; 32], tx.gas_limit);

        match exec_res {
            Ok(gas_used) => {
                let total_gas_fee = (gas_used as u128).saturating_mul(tx.max_fee_per_gas);

                // Create gas fee distribution witness untuk 80/20 validation
                let gas_witness = self.create_gas_fee_witness(total_gas_fee);
                self.block_gas_fees.push(gas_witness);

                // Gas fee is accounted for and pooled, no burn.
                // This will be included in reward calculations in consensus/reward.rs.
                let _ = (gas_used, tx.max_fee_per_gas, total_gas_fee); // keep info for debugging, avoid warning

                // Keep the state updates done by host functions from VM
                // For compatibility, still apply UTXO outputs on top of contract run
                self.apply_utxo_transaction(tx, utxo)
            }
            Err(err) => {
                // Roll back Verkle tree state
                self.tree = tree_snapshot;
                Err(StateManagerError::ApplyBlockFailed(format!("VM execution failed: {}", err)))
            }
        }
    }

    /// Verify gas fee distribution witnesses untuk 80/20 compliance
    pub fn verify_gas_fee_distribution(&self, witness: &GasFeeWitness) -> bool {
        // Verify 80/20 split calculation
        let expected_miner = (witness.total_gas_fee * economic_constants::MINER_REWARD_PERCENT) / 100;
        let expected_fullnode = witness.total_gas_fee.saturating_sub(expected_miner);
        
        witness.miner_share == expected_miner && 
        witness.fullnode_share == expected_fullnode &&
        witness.miner_share + witness.fullnode_share == witness.total_gas_fee
    }

    /// Verify global supply cap is not exceeded
    pub fn verify_global_supply(&self) -> Result<(), StateManagerError> {
        if self.current_total_supply > economic_constants::MAX_GLOBAL_SUPPLY_NANO_SLUG {
            return Err(StateManagerError::SupplyCapExceeded(
                format!("Total supply {} exceeds maximum allowed {}", 
                        self.current_total_supply, economic_constants::MAX_GLOBAL_SUPPLY_NANO_SLUG)
            ));
        }
        Ok(())
    }

    /// Get root hash dari current state
    pub fn get_root_hash(&self) -> Result<[u8; 32], StateManagerError> {
        self.tree.get_root()
            .map_err(|e| StateManagerError::CryptographicError(format!("Failed to get root: {}", e)))
    }

    /// Create gas fee distribution witness untuk 80/20 validation
    pub fn create_gas_fee_witness(&self, total_gas_fee: u128) -> GasFeeWitness {
        let miner_share = (total_gas_fee * economic_constants::MINER_REWARD_PERCENT) / 100;
        let fullnode_share = total_gas_fee.saturating_sub(miner_share);
        
        GasFeeWitness {
            total_gas_fee,
            miner_share,
            fullnode_share,
        }
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
        
        // Restore total supply from snapshot
        self.current_total_supply = self.snapshots[target_height as usize].total_supply;
        
        // Restore gas fees from snapshot
        self.block_gas_fees = self.snapshots[target_height as usize].gas_fees.clone();
        
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
    pub fn rollback(&mut self, target_height: u64) -> Result<(), StateManagerError> {
        if target_height > self.current_height {
            return Err(StateManagerError::InvalidRollback(format!(
                "Requested rollback to {} is beyond current height {}",
                target_height, self.current_height
            )));
        }

        // Keep safe backup in case restore fails.
        let backup_tree = self.tree.clone();
        let backup_height = self.current_height;
        let backup_snapshots = self.snapshots.clone();
        let backup_snapshot_storages = self.snapshot_storages.clone();
        let backup_total_supply = self.current_total_supply;
        let backup_gas_fees = self.block_gas_fees.clone();

        match self.rollback_state(target_height) {
            Ok(()) => {
                // Additional verification: ensure root hash matches snapshot after successful rollback
                // This catches any silent corruption that might have occurred during restoration
                if let Ok(current_root) = self.get_root_hash() {
                    let expected_root = self.snapshots[target_height as usize].root;
                    if current_root != expected_root {
                        eprintln!(
                            "[CRITICAL] Silent corruption detected after rollback: root mismatch at height {}. Expected {:?}, got {:?}. Attempting emergency restore.",
                            target_height, expected_root, current_root
                        );
                        
                        // Emergency restore from backup
                        self.tree = backup_tree;
                        self.current_height = backup_height;
                        self.snapshots = backup_snapshots;
                        self.snapshot_storages = backup_snapshot_storages;
                        self.current_total_supply = backup_total_supply;
                        self.block_gas_fees = backup_gas_fees;
                        
                        return Err(StateManagerError::RestoreFailed(
                            format!("Silent corruption detected: root hash mismatch after rollback to height {}", target_height)
                        ));
                    }
                } else {
                    eprintln!(
                        "[WARNING] Could not verify root hash after rollback to height {}: get_root_hash failed. Proceeding with caution.",
                        target_height
                    );
                }
                Ok(())
            }
            Err(err) => {
                eprintln!(
                    "[ERROR] rollback to height {} failed: {:?}. Restoring last known safe state at height {}.",
                    target_height, err, backup_height
                );

                // Restore safe state from backup
                self.tree = backup_tree;
                self.current_height = backup_height;
                self.snapshots = backup_snapshots;
                self.snapshot_storages = backup_snapshot_storages;
                self.current_total_supply = backup_total_supply;
                self.block_gas_fees = backup_gas_fees;

                Err(err)
            }
        }
    }

    /// Restore entire state dari specific snapshot
    pub fn restore_from_snapshot(&mut self, snapshot_root: [u8; 32], height: u64) -> Result<(), StateManagerError> {
        let snapshot_idx = self.snapshots.iter()
            .position(|s| s.height == height && s.root == snapshot_root)
            .ok_or(StateManagerError::SnapshotNotFound(height))?;

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
            total_supply: self.current_total_supply,
            gas_fees: self.block_gas_fees.clone(),
        })
    }

    /// VM host read state from Verkle tree
    pub fn state_read(&self, key: [u8; 32]) -> Result<Option<Vec<u8>>, String> {
        match self.tree.get(key) {
            Ok(val) => Ok(val),
            Err(e) => Err(e.to_string()),
        }
    }

    /// VM host write state to Verkle tree
    pub fn state_write(&mut self, key: [u8; 32], value: Vec<u8>) -> Result<(), String> {
        // For simplicity, state writes use insert (overwrite leaf)
        self.tree.insert(key, value);
        Ok(())
    }

    /// Tandai UTXO/outpoint untuk pruning pada epoch/timestamp tertentu.
    pub fn mark_outpoint_for_pruning(&mut self, outpoint: OutPoint, epoch: u64, timestamp: u64) {
        self.prune_markers.insert(outpoint, PruneMarker { epoch, timestamp });
    }

    /// Jalankan pruning cycle: prune semua outpoint yang melewati epoch threshold.
    pub fn execute_pruning_cycle(&mut self, epoch_threshold: u64, utxo: &mut UtxoSet) -> Result<Vec<OutPoint>, StateManagerError> {
        let keys_to_prune: Vec<OutPoint> = self
            .prune_markers
            .iter()
            .filter(|(_, marker)| marker.epoch <= epoch_threshold)
            .map(|(outpoint, _)| outpoint.clone())
            .collect();

        let mut pruned = Vec::new();
        for outpoint in keys_to_prune {
            if let Some(key) = self.outpoint_to_key.get(&outpoint).cloned() {
                self.tree
                    .prune_key(key)
                    .map_err(|e| StateManagerError::CryptographicError(format!("Prune failed: {}", e)))?;
                utxo.utxos.remove(&outpoint);
                self.outpoint_to_key.remove(&outpoint);
                self.prune_markers.remove(&outpoint);
                pruned.push(outpoint);
            }
        }

        Ok(pruned)
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

impl From<CoreError> for StateManagerError {
    fn from(err: CoreError) -> Self {
        StateManagerError::CryptographicError(format!("CoreError: {}", err))
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

        manager.rollback(1).expect("rollback failed");

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
