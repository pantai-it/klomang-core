use std::collections::{VecDeque};
use crate::core::state::transaction::Transaction;
use crate::core::state::access_set::AccessSet;
use crate::core::crypto::Hash;
use crate::core::state_manager::{StateManager, StateManagerError};
use crate::core::state::utxo::UtxoSet;
use crate::core::state::storage::Storage;

/// Represents a transaction with its access set for scheduling
#[derive(Clone)]
pub struct ScheduledTransaction {
    pub tx: Transaction,
    pub access_set: AccessSet,
    pub index: usize, // For deterministic ordering
}

/// Parallel scheduler for transaction execution
pub struct ParallelScheduler;

impl ParallelScheduler {
    /// Schedule transactions into parallelizable groups
    /// Returns a vector of groups, where each group can be executed in parallel
    pub fn schedule_transactions(txs: Vec<Transaction>) -> Vec<Vec<ScheduledTransaction>> {
        let mut scheduled: Vec<ScheduledTransaction> = txs
            .into_iter()
            .enumerate()
            .map(|(i, tx)| ScheduledTransaction {
                access_set: tx.generate_access_set(),
                tx,
                index: i,
            })
            .collect();

        // Sort by index for deterministic ordering
        scheduled.sort_by_key(|s| s.index);

        let mut groups = Vec::new();
        let mut remaining: VecDeque<ScheduledTransaction> = scheduled.into_iter().collect();

        while !remaining.is_empty() {
            let mut current_group = Vec::new();
            let mut to_remove = Vec::new();

            // Find non-conflicting transactions
            for i in 0..remaining.len() {
                let candidate = &remaining[i];
                let conflicts = current_group.iter().any(|existing: &ScheduledTransaction| {
                    existing.access_set.has_conflict(&candidate.access_set)
                });

                if !conflicts {
                    current_group.push(candidate.clone());
                    to_remove.push(i);
                }
            }

            // Remove selected transactions from remaining
            for &idx in to_remove.iter().rev() {
                remaining.remove(idx);
            }

            if current_group.is_empty() {
                // If no non-conflicting found, take the first one
                current_group.push(remaining.pop_front().unwrap());
            }

            groups.push(current_group);
        }

        groups
    }

    /// Execute scheduled groups in parallel, integrating with StateManager
    /// Applies transactions to the state and records changes in the Verkle Tree
    /// Ensures atomic state transitions and explicit conflict detection between groups
    pub fn execute_groups<S: Storage + Clone + Send + Sync + 'static>(
        groups: Vec<Vec<ScheduledTransaction>>,
        state_manager: &mut StateManager<S>,
        utxo: &mut UtxoSet,
    ) -> Result<(), StateManagerError> {
        // Explicit conflict detection between groups
        if let Some(conflict) = Self::detect_group_conflicts(&groups) {
            return Err(StateManagerError::ApplyBlockFailed(
                format!("Conflict detected between groups: {:?}", conflict)
            ));
        }

        for (group_idx, group) in groups.into_iter().enumerate() {
            // Backup state before executing group for atomic rollback
            let backup_tree = state_manager.tree.clone();
            let backup_height = state_manager.current_height;
            let backup_snapshots = state_manager.snapshots.clone();
            let backup_snapshot_storages = state_manager.snapshot_storages.clone();
            let backup_total_supply = state_manager.current_total_supply;
            let backup_gas_fees = state_manager.block_gas_fees.clone();
            let backup_prune_markers = state_manager.prune_markers.clone();
            let backup_outpoint_to_key = state_manager.outpoint_to_key.clone();
            let backup_pending_updates = state_manager.pending_updates.clone();
            let backup_utxo = utxo.clone();

            // Execute transactions in the group sequentially to maintain state consistency
            // StateManager is not thread-safe, so sequential execution is required
            let mut execution_result = Ok(());
            for (tx_idx, scheduled) in group.into_iter().enumerate() {
                if let Err(e) = state_manager.apply_transaction(&scheduled.tx, utxo) {
                    execution_result = Err(StateManagerError::ApplyBlockFailed(
                        format!("Transaction {} in group {} failed: {:?}", tx_idx, group_idx, e)
                    ));
                    break;
                }
            }

            // Handle execution result
            match execution_result {
                Ok(()) => {
                    // Group executed successfully, verify state changed
                    let post_group_root = state_manager.get_root_hash()
                        .map_err(|e| StateManagerError::CryptographicError(
                            format!("Failed to get root after group {}: {}", group_idx, e)
                        ))?;
                    
                    let pre_group_root = backup_tree.get_root()
                        .map_err(|e| StateManagerError::CryptographicError(
                            format!("Failed to get backup root for group {}: {}", group_idx, e)
                        ))?;
                    
                    if pre_group_root == post_group_root {
                        // Restore backup if no state change (possible invalid transactions)
                        state_manager.tree = backup_tree;
                        state_manager.current_height = backup_height;
                        state_manager.snapshots = backup_snapshots;
                        state_manager.snapshot_storages = backup_snapshot_storages;
                        state_manager.current_total_supply = backup_total_supply;
                        state_manager.block_gas_fees = backup_gas_fees;
                        state_manager.prune_markers = backup_prune_markers;
                        state_manager.outpoint_to_key = backup_outpoint_to_key;
                        state_manager.pending_updates = backup_pending_updates;
                        *utxo = backup_utxo;
                        
                        return Err(StateManagerError::ApplyBlockFailed(
                            format!("Group {} execution resulted in no state change", group_idx)
                        ));
                    }
                    // Success, continue to next group
                }
                Err(e) => {
                    // Restore backup state on failure
                    state_manager.tree = backup_tree;
                    state_manager.current_height = backup_height;
                    state_manager.snapshots = backup_snapshots;
                    state_manager.snapshot_storages = backup_snapshot_storages;
                    state_manager.current_total_supply = backup_total_supply;
                    state_manager.block_gas_fees = backup_gas_fees;
                    state_manager.prune_markers = backup_prune_markers;
                    state_manager.outpoint_to_key = backup_outpoint_to_key;
                    state_manager.pending_updates = backup_pending_updates;
                    *utxo = backup_utxo;
                    
                    return Err(e);
                }
            }
        }
        Ok(())
    }

    /// Detect conflicts between execution groups based on their combined access sets
    fn detect_group_conflicts(groups: &[Vec<ScheduledTransaction>]) -> Option<(usize, usize)> {
        let mut group_access_sets = Vec::new();

        // Compute combined access set for each group
        for group in groups {
            let mut combined = AccessSet::new();
            for scheduled in group {
                combined.merge(&scheduled.access_set);
            }
            group_access_sets.push(combined);
        }

        // Check for conflicts between any two groups
        for i in 0..group_access_sets.len() {
            for j in (i + 1)..group_access_sets.len() {
                if group_access_sets[i].has_conflict(&group_access_sets[j]) {
                    return Some((i, j));
                }
            }
        }

        None
    }
}

/// Canonical ordering based on DAG timestamp and hash
pub fn canonical_order_key(tx: &Transaction, timestamp: u64) -> (u64, Hash) {
    (timestamp, tx.id.clone())
}