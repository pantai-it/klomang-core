use crate::core::crypto::Hash;
use crate::core::dag::BlockNode;
use rocksdb::DB;
use std::path::Path;

pub struct RocksDBStorage {
    db: DB,
}

impl RocksDBStorage {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, rocksdb::Error> {
        let db = DB::open_default(path)?;
        Ok(Self { db })
    }
}

impl crate::core::storage::Storage for RocksDBStorage {
    fn get_block(&self, id: &Hash) -> Option<BlockNode> {
        let key = id.as_bytes();
        if let Ok(data) = self.db.get(key) {
            if let Some(bytes) = data {
                bincode::deserialize(&bytes).ok()
            } else {
                None
            }
        } else {
            None
        }
    }

    fn put_block(&mut self, block: BlockNode) {
        let key = block.id.as_bytes();
        let value = bincode::serialize(&block).unwrap();
        self.db.put(key, value).unwrap();
    }

    fn delete_block(&mut self, id: &Hash) {
        let key = id.as_bytes();
        self.db.delete(key).unwrap();
    }
}
