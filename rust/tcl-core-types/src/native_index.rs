//! Owned native option-table cache vocabulary, independent of object storage.

use alloc::rc::Rc;
use core::fmt;

/// The retained table cannot supply its selected native entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeIndexUnavailable;

/// An owned native table lifetime and guarded canonical-entry reader.
///
/// Implementors retain the actual table's lifetime authority. Its identity is
/// an equality key only; it never authorizes dereferencing an arbitrary address.
pub trait NativeIndexTable {
    /// Equality key for the original table while this receipt retains it.
    fn identity(&self) -> usize;
    /// Read the current terminated word at this original index and byte stride.
    ///
    /// # Errors
    /// Refuses an unavailable table, invalid stride or missing selected entry.
    fn word(&self, index: usize, stride: usize) -> Result<Rc<[u8]>, NativeIndexUnavailable>;
}

/// Requested native Index lookup flags; a selected engine validates availability.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NativeIndexLookupFlags {
    /// Accept only an exact spelling after a reached string getter.
    pub exact: bool,
    /// Neither consume nor install a table cache (native C9 `TEMP_TABLE`).
    pub temporary_table: bool,
    /// An empty or absent original returns the native no-index sentinel (C9).
    pub null_ok: bool,
}

/// A native Index cache retaining its original table, stride and entry index.
/// This is storage; native engine permission is issued independently.
#[derive(Clone)]
pub struct NativeIndexCache {
    table: Rc<dyn NativeIndexTable>,
    stride: usize,
    index: usize,
}

impl NativeIndexCache {
    /// Retain an independently owned table receipt and the reached cache fields.
    #[must_use]
    pub fn new(table: Rc<dyn NativeIndexTable>, stride: usize, index: usize) -> Self {
        Self {
            table,
            stride,
            index,
        }
    }

    /// Original native table equality key, never pointer access authority.
    #[must_use]
    pub fn table_identity(&self) -> usize {
        self.table.identity()
    }

    /// Original byte stride between table entries.
    #[must_use]
    pub const fn stride(&self) -> usize {
        self.stride
    }

    /// Original selected entry index.
    #[must_use]
    pub const fn index(&self) -> usize {
        self.index
    }

    /// Read the current canonical word through the retained table authority.
    ///
    /// # Errors
    /// Propagates an unavailable native table entry without inventing a string.
    pub fn word(&self) -> Result<Rc<[u8]>, NativeIndexUnavailable> {
        self.table.word(self.index, self.stride)
    }
}

impl fmt::Debug for NativeIndexCache {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NativeIndexCache")
            .field("table", &self.table_identity())
            .field("stride", &self.stride)
            .field("index", &self.index)
            .finish_non_exhaustive()
    }
}
