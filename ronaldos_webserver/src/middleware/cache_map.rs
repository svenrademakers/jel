use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    hash::{Hash, Hasher},
};

use tracing::debug;
use tracing::trace;

/// This container can hold a fixed size of elements. On the occasion an
/// insertion exceeds the capacity, the oldest entry in the container will be
/// purged to make place for the new one.
/// lookup, insertion and removal all operate on a constant complexity.
/// TODO: create atomic operations so a get can be none mutable
pub struct CacheMap<K, V, const N: usize>
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

#[derive(Default, Debug, Clone, Copy)]
struct NodeInfo {
    prev: Option<usize>,
    next: Option<usize>,
}

impl<K, V, const N: usize> CacheMap<K, V, N>
where
    V: Default + ?Sized + Clone,
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
            self.update_to_most_recent(i);
            index = i;
        } else if self.lookup.len() < N {
            debug!("back inserting from {:?}", self.last);
            index = self.last.map(|v| v + 1).unwrap_or(0);
            if index == 0 {
                // first item added, also init first
                self.first = Some(index);
            } else {
                self.node_info[index - 1].next = Some(index);
                self.node_info[index].prev = self.last;
            }
            self.lookup.insert(hash, index);
            self.last = Some(index);
        } else {
            debug!("replacing {:?}", self.first);
            let recycle_id = self.first.unwrap();
            let new_oldest = self.node_info[recycle_id].next.unwrap();
            self.node_info[new_oldest].prev = None;
            self.first = Some(new_oldest);
            self.node_info[recycle_id].next = None;
            self.node_info[recycle_id].prev = self.last;
            self.node_info[self.last.unwrap()].next = Some(recycle_id);
            self.last = Some(recycle_id);
            index = recycle_id;
        }
        trace!(
            "{:?} first:{:?}, last:{:?}",
            self.node_info,
            self.first,
            self.last
        );
        self.storage[index] = item;
        &self.storage[index]
    }

    fn update_to_most_recent(&mut self, i: usize) {
        if self.first == self.last {
            return;
        }

        if self.first == Some(i) {
            self.first = self.node_info[self.first.unwrap()].next;
        }

        if Some(i) != self.last {
            self.remove_from_link_list(i);
            self.node_info[i].next = None;
            self.node_info[i].prev = self.last;
            self.node_info[self.last.unwrap()].next = Some(i);

            self.last = Some(i);
        }
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

    pub fn get(&mut self, key: &K) -> Option<&V> {
        let hash = calculate_hash(key);
        if let Some(i) = self.lookup.get(&hash).cloned() {
            self.update_to_most_recent(i);
            return Some(&self.storage[i]);
        }
        None
    }

    pub fn iter(&self) -> impl Iterator<Item = V> + '_ {
        CacheMapIterator {
            cache: self,
            node_info_index: self.first,
        }
    }
}

pub struct CacheMapIterator<'a, K, V, const N: usize>
where
    K: Hash + ?Sized,
{
    cache: &'a CacheMap<K, V, N>,
    node_info_index: Option<usize>,
}

impl<'a, K, V, const N: usize> Iterator for CacheMapIterator<'a, K, V, N>
where
    V: Default + ?Sized + Clone,
    K: Hash + ?Sized,
{
    type Item = V;

    fn next(&mut self) -> Option<Self::Item> {
        let item = self.cache.storage[self.node_info_index?].clone();
        self.node_info_index = self.cache.node_info[self.node_info_index?].next;
        Some(item)
    }
}

fn calculate_hash<T: Hash + ?Sized>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}

#[cfg(test)]
mod tests {
    use super::CacheMap;

    #[test]
    fn test_iterator() {
        let mut cache = CacheMap::<u8, &str, 3>::new();
        cache.insert(&1, "1");
        cache.insert(&2, "2");
        cache.insert(&3, "3");
        assert_eq!(cache.last, Some(2));
        assert_eq!(cache.first, Some(0));
        assert_eq!(vec!["1", "2", "3"], cache.iter().collect::<Vec<&str>>());
    }

    #[test]
    fn test_insertion_below_capacity() {
        let mut cache = CacheMap::<u8, &str, 3>::new();
        cache.insert(&1, "1");
        cache.insert(&1, "1");
        assert_eq!(vec!["1"], cache.iter().collect::<Vec<&str>>());
        assert_eq!(cache.storage[0], "1");
        assert_eq!(cache.storage[1], "");
        assert_eq!(cache.storage[2], "");
        cache.insert(&2, "2");
        assert_eq!(vec!["1", "2"], cache.iter().collect::<Vec<&str>>());
        assert_eq!(cache.storage[0], "1");
        assert_eq!(cache.storage[1], "2");
        assert_eq!(cache.storage[2], "");
        cache.insert(&2, "2");
        assert_eq!(vec!["1", "2"], cache.iter().collect::<Vec<&str>>());
        assert_eq!(cache.storage[0], "1");
        assert_eq!(cache.storage[1], "2");
        assert_eq!(cache.storage[2], "");
        cache.insert(&3, "3");
        assert_eq!(vec!["1", "2", "3"], cache.iter().collect::<Vec<&str>>());
        assert_eq!(cache.storage[0], "1");
        assert_eq!(cache.storage[1], "2");
        assert_eq!(cache.storage[2], "3");
    }

    #[test]
    fn resinserting_results_in_update() {
        let mut cache = CacheMap::<u8, &str, 3>::new();
        cache.insert(&1, "1");
        cache.insert(&2, "2");
        cache.insert(&3, "3");
        assert_eq!(vec!["1", "2", "3"], cache.iter().collect::<Vec<&str>>());
        cache.insert(&2, "test2");
        assert_eq!(vec!["1", "3", "test2"], cache.iter().collect::<Vec<&str>>());
        assert_eq!(cache.storage[1], "test2");
        assert_eq!(cache.last, Some(1));
        assert_eq!(cache.first, Some(0));
    }

    #[test]
    fn overflow_result_in_purge_of_oldest() {
        let mut cache = CacheMap::<u8, &str, 3>::new();
        cache.insert(&1, "1");
        cache.insert(&2, "2");
        cache.insert(&3, "3");
        cache.insert(&5, "5");
        assert_eq!(vec!["2", "3", "5"], cache.iter().collect::<Vec<&str>>());
        cache.insert(&3, "nr3");
        assert_eq!(vec!["2", "5", "nr3"], cache.iter().collect::<Vec<&str>>());
    }

    #[test]
    fn get_updates_history() {
        let mut cache = CacheMap::<u8, &str, 3>::new();
        cache.insert(&1, "1");
        cache.insert(&2, "2");
        cache.insert(&3, "3");
        cache.get(&1);
        assert_eq!(vec!["2", "3", "1"], cache.iter().collect::<Vec<&str>>());
        cache.get(&3);
        assert_eq!(vec!["2", "1", "3"], cache.iter().collect::<Vec<&str>>());
        cache.get(&2);
        assert_eq!(vec!["1", "3", "2"], cache.iter().collect::<Vec<&str>>());
    }
}
