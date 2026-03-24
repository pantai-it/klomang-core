use crate::core::crypto::Hash;
use crate::core::state::transaction::Transaction;
use std::collections::HashSet;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct BlockNode {
    pub id: Hash,
    pub parents: HashSet<Hash>,
    pub children: HashSet<Hash>,
    pub selected_parent: Option<Hash>,
    pub blue_set: HashSet<Hash>,
    pub red_set: HashSet<Hash>,
    pub blue_score: u64,
    pub timestamp: u64,
    pub difficulty: u64,
    pub nonce: u64,
    pub transactions: Vec<Transaction>,
}

