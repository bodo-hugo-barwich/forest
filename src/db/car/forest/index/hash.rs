// Copyright 2019-2025 ChainSafe Systems
// SPDX-License-Identifier: Apache-2.0, MIT
use std::num::NonZeroUsize;

use super::NonMaximalU64;
use cid::Cid;

/// Summarize a [`Cid`]'s internal hash as a `u64`-sized hash.
pub fn summary(cid: &Cid) -> NonMaximalU64 {
    NonMaximalU64::fit(
        cid.hash()
            .digest()
            .chunks_exact(8)
            .map(<[u8; 8]>::try_from)
            .filter_map(Result::ok)
            .fold(cid.codec() ^ cid.hash().code(), |hash, chunk| {
                hash ^ u64::from_le_bytes(chunk)
            }),
    )
}

/// Desired slot for a hash with a given table length
///
/// See: <https://lemire.me/blog/2016/06/27/a-fast-alternative-to-the-modulo-reduction/>
pub fn ideal_slot_ix(hash: NonMaximalU64, num_buckets: NonZeroUsize) -> usize {
    // One could simply write `self.0 as usize % buckets` but that involves
    // a relatively slow division.
    // Splitting the hash into chunks and mapping them linearly to buckets is much faster.
    // On modern computers, this mapping can be done with a single multiplication
    // (the right shift is optimized away).

    // break 0..=u64::MAX into 'buckets' chunks and map each chunk to 0..len.
    // if buckets=2, 0..(u64::MAX/2) maps to 0, and (u64::MAX/2)..=u64::MAX maps to 1.
    usize::try_from((hash.get() as u128 * num_buckets.get() as u128) >> 64).unwrap()
}

/// Reverse engineer hashes which will be mapped to `ideal`.
///
/// Guaranteed to return at least one value.
///
/// # Panics
/// - If `ideal` >= `num_buckets` - that index is impossible to achieve!
#[cfg(test)]
pub fn from_ideal_slot_ix(
    ideal: usize,
    num_buckets: NonZeroUsize,
) -> impl Iterator<Item = NonMaximalU64> + Clone {
    assert!(ideal < num_buckets.get());

    fn div_ceil(a: u128, b: u128) -> u64 {
        (a / b + (if a % b == 0 { 0 } else { 1 })) as u64
    }
    let min_in_bucket = div_ceil(
        (1_u128 << u64::BITS) * ideal as u128,
        num_buckets.get() as u128,
    );
    let bucket_height = u64::MAX / u64::try_from(num_buckets.get()).unwrap();
    (0..bucket_height)
        .map(move |offset_in_bucket| min_in_bucket + offset_in_bucket)
        .map(|it| NonMaximalU64::new(it).unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::{cid::CidCborExt as _, multihash::prelude::*};

    quickcheck::quickcheck! {
        fn always_in_range(hash: NonMaximalU64, num_buckets: NonZeroUsize) -> bool {
            ideal_slot_ix(hash, num_buckets) < num_buckets.get()
        }
        fn backwards(ideal: usize, num_buckets: NonZeroUsize) -> () {
            do_backwards(ideal, num_buckets)
        }
    }

    fn do_backwards(ideal: usize, num_buckets: NonZeroUsize) {
        let ideal = ideal % num_buckets;
        let candidates = from_ideal_slot_ix(ideal, num_buckets);
        assert!(
            candidates.clone().next().is_some(),
            "must have at least one candidate"
        );
        // .take(_): don't want to check e.g u64::MAX candidates
        for candidate in candidates.take(1024) {
            assert_eq!(ideal, ideal_slot_ix(candidate, num_buckets))
        }
    }

    /// Small offsets and lengths can be checked exhaustively
    #[test]
    fn small_backwards() {
        for num_buckets in 1..u8::MAX {
            let num_buckets = NonZeroUsize::new(usize::from(num_buckets)).unwrap();
            for ideal in 0..num_buckets.get() {
                do_backwards(ideal, num_buckets)
            }
        }
    }

    /// hash stability tests
    #[test]
    fn snapshots() {
        for (cid, expected) in [
            (Cid::default(), 0),
            (
                Cid::from_cbor_blake2b256(&"forest").unwrap(),
                7060553106844083342,
            ),
            (
                Cid::from_cbor_blake2b256(&"lotus").unwrap(),
                10998694778601859716,
            ),
            (
                Cid::from_cbor_blake2b256(&"libp2p").unwrap(),
                15878333306608412239,
            ),
            (
                Cid::from_cbor_blake2b256(&"ChainSafe").unwrap(),
                17464860692676963753,
            ),
            (
                Cid::from_cbor_blake2b256(&"haskell").unwrap(),
                10392497608425502268,
            ),
            (Cid::new_v1(0xAB, MultihashCode::Identity.digest(&[])), 170),
            (
                Cid::new_v1(0xAC, MultihashCode::Identity.digest(&[1, 2, 3, 4])),
                171,
            ),
            (
                Cid::new_v1(
                    0xAD,
                    MultihashCode::Identity.digest(&[1, 2, 3, 4, 5, 6, 7, 8]),
                ),
                578437695752307371,
            ),
        ] {
            assert_eq!(summary(&cid), NonMaximalU64::new(expected).unwrap())
        }
    }
}
