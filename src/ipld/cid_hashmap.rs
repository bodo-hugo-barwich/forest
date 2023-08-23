// Copyright 2019-2023 ChainSafe Systems
// SPDX-License-Identifier: Apache-2.0, MIT

use crate::utils::cid::SmallCid;
use ahash::{HashMap, HashMapExt};
use cid::Cid;

// The size of a CID is 96 bytes. A CID contains:
//   - a version
//   - a codec
//   - a hash code
//   - a length
//   - 64 bytes pre-allocated buffer
// Each non-buffer field takes 8 bytes with padding. So, 4*8 = 32 bytes, 32 + 64 = 96 bytes.
//
// However, we know that nearly all Filecoin CIDs have version=V1, codec=DAG_CBOR, code=Blake2b and
// length=32. Taking advantage of this knowledge, we can store the vast majority of CIDs (+99.99%)
// in one third of the usual space (32 bytes vs 96 bytes).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CidHashMap<V>(HashMap<SmallCid, V>);

impl<V> Extend<(Cid, V)> for CidHashMap<V> {
    fn extend<T: IntoIterator<Item = (Cid, V)>>(&mut self, iter: T) {
        for (k, v) in iter {
            self.insert(k, v);
        }
    }
}

impl<V> FromIterator<(Cid, V)> for CidHashMap<V> {
    fn from_iter<T: IntoIterator<Item = (Cid, V)>>(iter: T) -> Self {
        let mut map = Self::new();
        map.extend(iter);
        map
    }
}

pub struct IntoIter<V>(std::collections::hash_map::IntoIter<SmallCid, V>);

impl<V> IntoIterator for CidHashMap<V> {
    type Item = (Cid, V);
    type IntoIter = IntoIter<V>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter(self.0.into_iter())
    }
}

impl<V> Iterator for IntoIter<V> {
    type Item = (Cid, V);
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|(small_cid, v)| (small_cid.cid(), v))
    }
}

impl<V> CidHashMap<V> {
    /// Creates an empty `HashMap` with CID type keys.
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    /// Returns `true` if the map contains a value for the specified key.
    pub fn contains_key(&self, k: Cid) -> bool {
        self.0.contains_key(&SmallCid::from(k))
    }

    /// Inserts a key-value pair into the map; if the map did not have this key present, [`None`] is returned.
    pub fn insert(&mut self, k: Cid, v: V) -> Option<V> {
        self.0.insert(SmallCid::from(k), v)
    }

    /// Removes a key from the map, returning the value at the key if the key
    /// was previously in the map.
    pub fn remove(&mut self, k: Cid) -> Option<V> {
        self.0.remove(&SmallCid::from(k))
    }

    /// Returns the number of elements the map can hold without reallocating.
    pub fn capacity(&self) -> usize {
        self.0.capacity()
    }

    /// Returns a reference to the value corresponding to the key.
    pub fn get(&self, k: Cid) -> Option<&V> {
        self.0.get(&SmallCid::from(k))
    }

    /// Returns the number of elements in the map.
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cid::{
        multihash::{self, MultihashDigest},
        Cid,
    };
    use fvm_ipld_encoding::DAG_CBOR;
    use quickcheck_macros::quickcheck;

    fn generate_hash_maps(cid_vector: Vec<(Cid, u64)>) -> (CidHashMap<u64>, HashMap<Cid, u64>) {
        let mut cid_hash_map = CidHashMap::new();
        let mut hash_map = HashMap::new();
        for item in cid_vector.iter() {
            cid_hash_map.insert(item.0, item.1);
            hash_map.insert(item.0, item.1);

            // Quickcheck does not reliably generate the DAG_CBOR/Blake2b variant of V1 CIDs; need to ensure we have enough samples of this variant in the map for testing, so generate this variant from the values in the key-value pairs.
            let cid_v1 = Cid::new_v1(
                DAG_CBOR,
                multihash::Code::Blake2b256.digest(&item.1.to_be_bytes()),
            );
            cid_hash_map.insert(cid_v1, item.1);
            hash_map.insert(cid_v1, item.1);
        }
        (cid_hash_map, hash_map)
    }

    #[quickcheck]
    fn insert_new_key_is_none(cid_vector: Vec<(Cid, u64)>, cid: Cid, payload: u64) {
        let (mut cid_hash_map, _) = generate_hash_maps(cid_vector);
        // Quickcheck occasionally generates a key that is already present in the map, so remove it if it is present.
        if cid_hash_map.contains_key(cid) {
            cid_hash_map.remove(cid);
        }
        assert!(cid_hash_map.insert(cid, payload).is_none());
    }

    #[quickcheck]
    fn insert_existing_key_is_some(cid_vector: Vec<(Cid, u64)>, cid: Cid, payload: u64) {
        let (mut cid_hash_map, _) = generate_hash_maps(cid_vector);
        cid_hash_map.insert(cid, payload);
        assert!(cid_hash_map.insert(cid, payload).is_some());
    }

    #[quickcheck]
    fn contains_key(cid_vector: Vec<(Cid, u64)>, cid: Cid, insert: bool) {
        let (mut cid_hash_map, mut hash_map) = generate_hash_maps(cid_vector);
        // Quickcheck rarely generates a key that is already present in the maps, so insert it with 50% probability to test `contains_key` with an equal distribution of results.
        if insert {
            cid_hash_map.insert(cid, 0);
            hash_map.insert(cid, 0);
        }
        assert_eq!(cid_hash_map.contains_key(cid), hash_map.contains_key(&cid));
    }

    #[quickcheck]
    fn remove_key(cid_vector: Vec<(Cid, u64)>, cid: Cid, insert: bool) {
        let (mut cid_hash_map, mut hash_map) = generate_hash_maps(cid_vector);
        // Quickcheck rarely generates a key that is already present in the maps, so insert it with 50% probability to test `remove` with an equal distribution of results.
        if insert {
            cid_hash_map.insert(cid, 0);
            hash_map.insert(cid, 0);
        }
        assert_eq!(cid_hash_map.remove(cid), hash_map.remove(&cid));
    }

    #[quickcheck]
    fn get_value_at_key(cid_vector: Vec<(Cid, u64)>, cid: Cid, insert: bool) {
        let (mut cid_hash_map, mut hash_map) = generate_hash_maps(cid_vector);
        // Quickcheck rarely generates a key that is already present in the maps, so insert it with 50% probability to test `get` with an equal distribution of results.
        if insert {
            cid_hash_map.insert(cid, 0);
            hash_map.insert(cid, 0);
        }
        assert_eq!(cid_hash_map.get(cid), hash_map.get(&cid));
    }

    #[quickcheck]
    fn len(cid_vector: Vec<(Cid, u64)>) {
        let (cid_hash_map, hash_map) = generate_hash_maps(cid_vector);
        assert_eq!(cid_hash_map.len(), hash_map.len());
    }

    #[quickcheck]
    fn cidhashmap_to_hashmap_to_cidhashmap(cid_vector: Vec<(Cid, u64)>) {
        let (cid_hash_map, _) = generate_hash_maps(cid_vector);
        let hash_map: HashMap<Cid, u64> = cid_hash_map.clone().into_iter().collect();
        let cid_hash_map_2: CidHashMap<u64> = hash_map.into_iter().collect();
        assert_eq!(cid_hash_map, cid_hash_map_2);
    }
}