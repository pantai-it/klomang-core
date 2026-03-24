use crate::core::crypto::Hash;
use crate::core::dag::BlockNode;
use std::collections::HashMap;

pub mod rocksdb;

pub trait Storage {
    fn get_block(&self, id: &Hash) -> Option<BlockNode>;
    fn put_block(&mut self, block: BlockNode);
    fn delete_block(&mut self, id: &Hash);
}

pub struct MemoryStorage {
    blocks: HashMap<Hash, BlockNode>,
}

impl MemoryStorage {
    pub fn new() -> Self {
        Self {
            blocks: HashMap::new(),
        }
    }
}

impl Storage for MemoryStorage {
    fn get_block(&self, id: &Hash) -> Option<BlockNode> {
        self.blocks.get(id).cloned()
    }

    fn put_block(&mut self, block: BlockNode) {
        self.blocks.insert(block.id.clone(), block);
    }

    fn delete_block(&mut self, id: &Hash) {
        self.blocks.remove(id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::dag::BlockNode;
    use crate::core::crypto::Hash;
    use std::collections::HashSet;

    #[test]
    fn memory_storage_insert_and_get_block() {
        let mut storage = MemoryStorage::new();

        let block = BlockNode {
            id: Hash::new(b"block1"),
            parents: HashSet::new(),
            children: HashSet::new(),
            selected_parent: None,
            blue_set: HashSet::new(),
            red_set: HashSet::new(),
            blue_score: 123,
            timestamp: 1000,
            difficulty: 1000,
            nonce: 0,
            transactions: Vec::new(),
        };

        storage.put_block(block.clone());

        let loaded = storage.get_block(&block.id).expect("block present");
        assert_eq!(loaded.id, block.id);
        assert_eq!(loaded.blue_score, 123);
    }

    #[test]
    fn memory_storage_missing_block_returns_none() {
        let storage = MemoryStorage::new();
        assert!(storage.get_block(&Hash::new(b"missing")).is_none());
    }
}
