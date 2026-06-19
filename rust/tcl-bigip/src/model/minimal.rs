//! Shared minimal / generic objects for the long-tail kinds. Mirrors
//! `dialects/f5/bigip/model/_minimal.py`.

use crate::range::Range;

/// Shared shape for every "minimal" projection — the long-tail kinds
/// that carry only the identity tuple plus a description and TMSH kind
/// label. Mirrors Python `BigipMinimalObject` (and all its per-module
/// aliases, which resolve to the same class).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BigipMinimalObject {
    /// Leaf name.
    pub name: String,
    /// Full TMSH path (empty for singletons).
    pub full_path: String,
    /// Full TMSH kind label (e.g. `"net routing as-path"`).
    pub kind: String,
    /// Unquoted `description`, when present.
    pub description: String,
    /// Source span, when captured.
    pub range: Option<Range>,
}

/// A generic BIG-IP stanza retained when no specialised model exists.
/// Mirrors Python `BigipGenericObject`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BigipGenericObject {
    /// tmsh module word (e.g. `"net"`, `"auth"`, `"sys"`).
    pub module: String,
    /// tmsh object-type (e.g. `"route-domain"`, `"user"`).
    pub object_type: String,
    /// Identifier (e.g. `"/Common/0"`, `"admin"`, or `""` for singletons).
    pub identifier: String,
    /// Raw header text.
    pub header: String,
    /// Source span, when captured.
    pub range: Option<Range>,
}
