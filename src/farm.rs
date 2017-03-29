//! Faster (but not DoS-resistant) hashmap
use farmhash;
use hash_hasher::HashBuildHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher, BuildHasherDefault};

/// Act like a farmhash
///
/// But since farmhash isn't a streaming hash we only compute the last bytes
/// so it's not really fulfilling the Hasher trait. But it's enough for us.
pub struct FarmHashLie (u64);

impl Default for FarmHashLie {
    #[inline]
    fn default() -> FarmHashLie { FarmHashLie(0) }
}

impl Hasher for FarmHashLie {
    #[inline]
    fn finish(&self) -> u64 {
        self.0
    }
    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        self.0 = farmhash::hash64(bytes);
    }
}

pub type Farm = BuildHasherDefault<FarmHashLie>;
pub type FarmMap<X, Y> = HashMap<X, Y, Farm>;

pub fn new_farm<X: Hash+Eq, Y>() -> FarmMap<X, Y> {
    Default::default()
}


pub type PlainMap<X, Y> = HashMap<X, Y, HashBuildHasher>;