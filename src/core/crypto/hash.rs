use blake3;
use hex;

#[derive(Clone, Eq, PartialEq, Hash, PartialOrd, Ord, Debug, serde::Serialize, serde::Deserialize)]
pub struct Hash([u8; 32]);

impl Hash {
    pub fn new(data: &[u8]) -> Self {
        let mut hasher = blake3::Hasher::new();
        hasher.update(data);
        let hash = hasher.finalize();
        let bytes = hash.as_bytes();
        let mut array = [0u8; 32];
        array.copy_from_slice(bytes);
        Self(array)
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}
