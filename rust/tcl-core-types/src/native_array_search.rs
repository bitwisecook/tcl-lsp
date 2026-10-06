//! Original array-search cache facts and owning cursor chains.

use crate::NativeHashWordWidth;
use alloc::vec::Vec;

/// Independently issued native integer ABI, not a Tcl release inference.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeArraySearchAbi {
    /// Actual C unsigned-long arithmetic used by strtoul.
    pub unsigned_long: NativeHashWordWidth,
    /// Actual C int width; the supported recipes require 32 bits.
    pub int_bits: u8,
}

/// Resident-only C8 array-search internal representation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeArraySearchCache {
    /// Original native signed-int search identifier.
    pub id: i32,
    /// Original byte offset of the variable-name part in resident storage.
    pub name_offset: usize,
}

struct Search<V> {
    id: i32,
    handle: Option<V>,
    keys: Vec<Vec<u8>>,
    next: usize,
}

/// The actual original array's active native search chain. Modern Tcl owns
/// each returned handle here; legacy Tcl stores no handle reference.
pub struct NativeArraySearchChain<V> {
    searches: Vec<Search<V>>,
}
impl<V> Default for NativeArraySearchChain<V> {
    fn default() -> Self {
        Self {
            searches: Vec::new(),
        }
    }
}
impl<V> NativeArraySearchChain<V> {
    /// The newest still-active ID determines the next native ID.
    #[must_use]
    pub fn next_id(&self) -> Option<i32> {
        self.searches
            .first()
            .map_or(Some(1), |search| search.id.checked_add(1))
    }
    /// Install an original cursor and, where required, its actual handle hold.
    pub fn insert(&mut self, id: i32, handle: Option<V>, keys: Vec<Vec<u8>>) {
        self.searches.insert(
            0,
            Search {
                id,
                handle,
                keys,
                next: 0,
            },
        );
    }
    /// Drop all handle holds at the actual invalidating mutation.
    pub fn clear(&mut self) {
        self.searches.clear();
    }
    /// Remove exactly one active cursor and its handle hold.
    pub fn remove(&mut self, id: i32) -> bool {
        let Some(index) = self.searches.iter().position(|search| search.id == id) else {
            return false;
        };
        self.searches.remove(index);
        true
    }
    /// Legacy ID lookup owns no variable or object reference.
    #[must_use]
    pub fn contains(&self, id: i32) -> bool {
        self.searches.iter().any(|search| search.id == id)
    }
    /// Query actual retained handles without cloning them.
    pub fn find_handle<E>(
        &self,
        mut equal: impl FnMut(&V) -> Result<bool, E>,
    ) -> Result<Option<i32>, E> {
        for search in &self.searches {
            if let Some(handle) = &search.handle
                && equal(handle)?
            {
                return Ok(Some(search.id));
            }
        }
        Ok(None)
    }
    /// Native anymore skips undefined candidates but keeps the next defined one.
    pub fn next_defined(
        &mut self,
        id: i32,
        consume: bool,
        mut defined: impl FnMut(&[u8]) -> bool,
    ) -> Option<Vec<u8>> {
        let search = self.searches.iter_mut().find(|search| search.id == id)?;
        while let Some(key) = search.keys.get(search.next) {
            if defined(key) {
                let key = key.clone();
                if consume {
                    search.next += 1;
                }
                return Some(key);
            }
            search.next += 1;
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::rc::Rc;
    #[test]
    fn original_handle_hold_retires_on_mutation_and_top_id_is_reused() {
        let handle = Rc::new(());
        let mut chain = NativeArraySearchChain::default();
        chain.insert(
            1,
            Some(handle.clone()),
            alloc::vec![b"undefined".to_vec(), b"present".to_vec()],
        );
        assert_eq!(Rc::strong_count(&handle), 2);
        assert_eq!(
            chain.next_defined(1, false, |key| key == b"present"),
            Some(b"present".to_vec())
        );
        assert_eq!(
            chain.next_defined(1, true, |_| true),
            Some(b"present".to_vec())
        );
        assert!(chain.next_defined(1, false, |_| true).is_none());
        chain.insert(2, None, Vec::new());
        assert_eq!(chain.next_id(), Some(3));
        assert!(chain.remove(2));
        assert_eq!(chain.next_id(), Some(2));
        chain.clear();
        assert_eq!(Rc::strong_count(&handle), 1);
        assert_eq!(chain.next_id(), Some(1));
    }
}
