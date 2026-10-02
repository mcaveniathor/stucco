use crate::StoreError;
use serde::{Serialize, de::DeserializeOwned};
use std::marker::PhantomData;
/// Application value encoding. Persisted schemas require migration when changed.
pub trait Codec<T>: Send + Sync + 'static {
    /// Encodes an owned value.
    fn encode(&self, value: &T) -> Result<Vec<u8>, StoreError>;
    /// Decodes a value without borrowing its transaction.
    fn decode(&self, bytes: &[u8]) -> Result<T, StoreError>;
}
/// A codec whose byte ordering is the intended key ordering.
pub trait KeyCodec<K>: Codec<K> {}
/// Eight-byte big-endian unsigned keys.
#[derive(Clone, Copy, Debug, Default)]
pub struct U64Key;
impl Codec<u64> for U64Key {
    fn encode(&self, value: &u64) -> Result<Vec<u8>, StoreError> {
        Ok(value.to_be_bytes().to_vec())
    }
    fn decode(&self, bytes: &[u8]) -> Result<u64, StoreError> {
        Ok(u64::from_be_bytes(
            bytes
                .try_into()
                .map_err(|_| StoreError::invalid("invalid u64 key"))?,
        ))
    }
}
impl KeyCodec<u64> for U64Key {}
/// UTF-8 keys, ordered lexicographically by bytes.
#[derive(Clone, Copy, Debug, Default)]
pub struct Utf8Key;
impl Codec<String> for Utf8Key {
    fn encode(&self, value: &String) -> Result<Vec<u8>, StoreError> {
        Ok(value.as_bytes().to_vec())
    }
    fn decode(&self, bytes: &[u8]) -> Result<String, StoreError> {
        String::from_utf8(bytes.to_vec()).map_err(StoreError::new)
    }
}
impl KeyCodec<String> for Utf8Key {}
/// Postcard value serialization. Not an order-preserving key codec.
#[derive(Debug)]
pub struct PostcardCodec<T>(PhantomData<fn() -> T>);
impl<T> Default for PostcardCodec<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}
impl<T: Serialize + DeserializeOwned + 'static> Codec<T> for PostcardCodec<T> {
    fn encode(&self, value: &T) -> Result<Vec<u8>, StoreError> {
        postcard::to_allocvec(value).map_err(StoreError::new)
    }
    fn decode(&self, bytes: &[u8]) -> Result<T, StoreError> {
        postcard::from_bytes(bytes).map_err(StoreError::new)
    }
}
