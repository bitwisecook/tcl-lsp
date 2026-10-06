//! Jim's original Enum and immediate-string caches, separate from C Index.
use crate::NativeIndexCache;

/// Retained static table/literal lifetime. These fields are storage, not an issuer.
#[derive(Clone, Debug)]
pub enum NativeJimOptionCache {
    /// `GetEnum` stores the exact original table, flags and selected entry.
    Enum { entry: NativeIndexCache, flags: i32 },
    /// `CompareStringImmediate` retains the exact successful static literal identity.
    ComparedString {
        entry: NativeIndexCache,
        literal_identity: usize,
    },
}
