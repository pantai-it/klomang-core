use crate::core::crypto::Hash;
use crate::core::dag::block::BlockNode;
use crate::core::dag::anticone;
use std::collections::{HashMap, HashSet};

pub struct Dag {
    pub(crate) blocks: HashMap<Hash, BlockNode>,
    tips: HashSet<Hash>,
}

impl Dag {
    pub fn new() -> Self {
        Self {
            blocks: HashMap::new(),
            tips: HashSet::new(),
        }
    }

    pub fn get_all_hashes(&self) -> HashSet<Hash> {
        self.blocks.keys().cloned().collect()
    }

    pub(crate) fn get_block_mut(&mut self, id: &Hash) -> Option<&mut BlockNode> {
        self.blocks.get_mut(id)
    }

    pub fn add_block(&mut self, block: BlockNode) -> Result<(), crate::core::errors::CoreError> {
        let id = block.id.clone();

        if self.blocks.contains_key(&id) {
            return Err(crate::core::errors::CoreError::DuplicateBlock);
        }

        if block.parents.contains(&id) {
            return Err(crate::core::errors::CoreError::ConsensusError);
        }

        if block.parents.is_empty() && !self.blocks.is_empty() {
            return Err(crate::core::errors::CoreError::ConsensusError);
        }

        for parent in &block.parents {
            if !self.blocks.contains_key(parent) {
                return Err(crate::core::errors::CoreError::InvalidParent);
            }

            let ancestor_set = self.get_ancestors(parent);
            if ancestor_set.contains(&id) {
                return Err(crate::core::errors::CoreError::ConsensusError);
            }
        }

        let parents = block.parents.clone();
        self.blocks.insert(id.clone(), block);

        for parent_hash in &parents {
            if let Some(parent_block) = self.blocks.get_mut(parent_hash) {
                parent_block.children.insert(id.clone());
            }
        }

        for parent in &parents {
            self.tips.remove(parent);
        }
        self.tips.insert(id);

        Ok(())
    }

    pub fn get_block(&self, id: &Hash) -> Option<&BlockNode> {
        self.blocks.get(id)
    }

    pub fn get_tips(&self) -> HashSet<Hash> {
        self.tips.iter().cloned().collect()
    }

    pub fn is_ancestor(&self, a: &Hash, b: &Hash) -> bool {
        if a == b {
            return false;
        }
        let mut visited = HashSet::new();
        let mut stack = vec![b.clone()];
        while let Some(current) = stack.pop() {
            if visited.contains(&current) {
                continue;
            }
            visited.insert(current.clone());
            if let Some(block) = self.blocks.get(&current) {
                for parent in &block.parents {
                    if parent == a {
                        return true;
                    }
                    stack.push(parent.clone());
                }
            }
        }
        false
    }

    pub fn get_ancestors(&self, id: &Hash) -> HashSet<Hash> {
        let mut ancestors = HashSet::new();
        let mut stack = vec![id.clone()];
        let mut visited = HashSet::new();
        while let Some(current) = stack.pop() {
            if visited.contains(&current) {
                continue;
            }
            visited.insert(current.clone());
            if let Some(block) = self.blocks.get(&current) {
                for parent in &block.parents {
                    ancestors.insert(parent.clone());
                    stack.push(parent.clone());
                }
            }
        }
        ancestors
    }

    pub fn get_descendants(&self, id: &Hash) -> Vec<Hash> {
        let mut descendants = Vec::new();
        let mut stack = vec![id.clone()];
        let mut visited = HashSet::new();
        visited.insert(id.clone());

        while let Some(current) = stack.pop() {
            if let Some(block) = self.blocks.get(&current) {
                for child in &block.children {
                    if visited.insert(child.clone()) {
                        descendants.push(child.clone());
                        stack.push(child.clone());
                    }
                }
            }
        }

        descendants
    }

    pub fn get_anticone(&self, id: &Hash) -> HashSet<Hash> {
        anticone::get_anticone(self, id)
    }

    pub fn block_exists(&self, id: &Hash) -> bool {
        self.blocks.contains_key(id)
    }

    pub fn get_block_count(&self) -> usize {
        self.blocks.len()
    }
}
