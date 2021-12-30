#![cfg_attr(not(feature = "std"), no_std)]

use codec::Codec;
use parami_primitives::names;
use sp_std::prelude::*;

pub fn derive_storage_key<C: Codec>(group: &[u8], key: &C) -> Vec<u8> {
    let delimiter = vec![b'/', b'/'];

    let mut storage_key = Vec::from(*names::DID);
    storage_key.append(&mut delimiter.clone());
    storage_key.append(&mut Vec::from(group));
    storage_key.append(&mut delimiter.clone());
    storage_key.append(&mut key.encode());

    storage_key
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_storage_key() {
        let key = derive_storage_key(b"group", b"key");
        assert_eq!(key, b"prm/did //group//key");
    }
}
