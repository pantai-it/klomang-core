use crate::core::crypto::Hash;
use crate::core::dag::dag::Dag;
use std::collections::HashSet;

pub fn get_anticone(dag: &Dag, id: &Hash) -> HashSet<Hash> {
    let ancestors = dag.get_ancestors(id);
    let descendants: HashSet<Hash> = dag.get_descendants(id).into_iter().collect();
    let mut anticone = HashSet::new();
    for hash in dag.blocks.keys() {
        if hash != id && !ancestors.contains(hash) && !descendants.contains(hash) {
            anticone.insert(hash.clone());
        }
    }
    anticone
}