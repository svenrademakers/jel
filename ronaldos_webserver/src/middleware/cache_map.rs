use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    hash::{Hash, Hasher},
    slice::SliceIndex,
};

use log::debug;

/// This container can hold a fixed size of elements. On the occasion an
/// insertion exceeds the capacity, the oldest entry in the container will be
/// purged to make place for the new one.
/// lookup, insertion and removal all operate on a constant complexity.
pub struct CacheController<K, V, const N: usize>
where
    K: Hash + ?Sized,
{
    storage: Vec<V>,
    node_info: [NodeInfo; N],
    lookup: HashMap<u64, usize>,
    marker: std::marker::PhantomData<K>,
    first: Option<usize>,
    last: Option<usize>,
}

#[derive(Default, Clone, Copy)]
struct NodeInfo {
    prev: Option<usize>,
    next: Option<usize>,
}

impl<K, V, const N: usize> CacheController<K, V, N>
where
    V: Default + ?Sized,
    K: Hash + ?Sized,
{
    pub fn new() -> Self {
        Self {
            storage: std::iter::repeat_with(|| V::default()).take(N).collect(),
            node_info: [NodeInfo::default(); N],
            lookup: HashMap::with_capacity(N),
            marker: std::marker::PhantomData::default(),
            first: None,
            last: None,
        }
    }

    pub fn insert(&mut self, key: &K, item: V) -> &V {
        let hash = calculate_hash(key);
        let index;
        // if the key already exists, update its value and adjust node info
        if let Some(i) = self.lookup.get(&hash).cloned() {
            debug!("updating {}", i);
            self.remove_from_link_list(i);
            self.node_info[i].next = None;
            self.node_info[i].prev = self.first;
            self.first = Some(i);
            index = i;
        } else if self.lookup.len() < N {
            debug!("back inserting");
            index = self.first.map(|v| v + 1).unwrap_or(0);
            if index == 0 {
                // first item added, also init last
                self.last = Some(index);
            }
            self.lookup.insert(hash, index);
            self.first = Some(index);
        } else {
            debug!("replacing {:?}", self.last);
            // replace oldest with newest
            let last = self
                .last
                .expect("whole storage array should be filled by now");
            self.last = self.node_info[last].next;
            self.node_info[last].prev = self.first;
            self.node_info[last].next = None;
            self.first = Some(last);
            index = last;
        }

        self.storage[index] = item;
        &self.storage[index]
    }

    fn remove_from_link_list(&mut self, index: usize) {
        if let Some(next) = self.node_info[index].next {
            self.node_info[next].prev = self.node_info[index].prev;
        }
        if let Some(previous) = self.node_info[index].prev {
            self.node_info[previous].next = self.node_info[index].next;
        }
    }

    pub fn remove(&mut self) {
        todo!()
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        None
    }
}

fn calculate_hash<T: Hash + ?Sized>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}

#[cfg(test)]
mod tests {
    use crate::logger::init_log;

    use super::CacheController;

    #[test]
    fn test_insertion_below_capacity() {
        init_log(log::Level::Debug);

        // let mut cache = CacheController::<u8, &str, 3>::new();
        // cache.insert(&1, "item1");
        // cache.insert(&2, "item2");
        // cache.insert(&3, "item3");
        // cache.insert(&2, "item2");
        // assert_eq!(cache.storage[0], "item1");
        // assert_eq!(cache.storage[1], "item2");
        // assert_eq!(cache.storage[2], "item3");
        // assert_eq!(None, cache.node_info[0].prev);
        // assert_eq!(Some(2), cache.node_info[0].next);
        // assert_eq!(Some(2), cache.node_info[1].prev);
        // assert_eq!(None, cache.node_info[1].next);



        // cache.insert(&4, "item4");
        // assert_eq!(Some(&"item2"), cache.get(&2));
        // assert_eq!(Some(&"item3"), cache.get(&3));
        // assert_eq!(Some(&"item4"), cache.get(&4));
    }
}
