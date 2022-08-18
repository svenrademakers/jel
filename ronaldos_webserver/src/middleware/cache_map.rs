use std::{
    char::MAX,
    collections::{hash_map::DefaultHasher, HashMap},
    hash::{Hash, Hasher},
    io::{self, Write},
    ops::{Deref, DerefMut},
    path::Path,
    sync::Arc,
};
use tokio::sync::RwLock;

pub struct CacheController<K, V, const N: usize>
where
    K: Hash + ?Sized,
{
    storage: Vec<V>,
    lookup: HashMap<u64, usize>,
    history: ([u64; N], usize, usize),
    hasher: DefaultHasher,
    marker: std::marker::PhantomData<K>,
}

impl<K, V, const N: usize> CacheController<K, V, N>
where
    V: Default,
    K: Hash + ?Sized,
{
    pub fn new() -> Self {
        Self {
            storage: std::iter::repeat_with(|| V::default()).take(N).collect(),
            lookup: HashMap::with_capacity(N),
            history: ([0; N], 0, 0),
            hasher: DefaultHasher::new(),
            marker: std::marker::PhantomData::default(),
        }
    }

    pub fn insert(&mut self, key: &K, item: V) -> &V {
        let hash = calculate_hash(key);

        let (stack, insert, remove) = &mut self.history;

        // fill up the cache first
        if *insert == 0 || *insert % N != 0 {
            self.storage[*insert] = item;
            self.lookup.insert(hash, *insert);
            stack[*insert] = hash;
            *insert += 1;
        }
        todo!()
    }

    pub fn remove(&mut self) {
        todo!()
    }

    pub fn get(&self) -> Option<&V> {
       None
    }
}

fn calculate_hash<T: Hash + ?Sized>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}

//     if take == insert {
//         return None;
//     }

//     let item = queue[*take];
//     *take += 1;
//     Some(item)
// }

// pub async fn get_source<P>(&self, file: P) -> Arc<Vec<u8>>
// where
//     P: Hash + AsRef<Path>,
// {
//     todo!()
//     //let hash = self.hasher.
//     // match self.cache.get(&hash) {
//     //     Some(buffer) => {}
//     //     None => {
//     //         if let Some(key) = self.take_oldest_cache_line().await {
//     //             self.cache
//     //         }
//     //     }
//     // }
// }
//}