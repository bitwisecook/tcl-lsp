// SPDX-License-Identifier: AGPL-3.0-or-later
//! Pinned Jim local-command lineage rules, separate from node lifetime authority.

/// Selected original Jim core command.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeJimLocalCommand {
    /// Evaluate the original argv tail while native local publication is active.
    Local,
    /// Invoke a previous node by changing the selected procedure's upcall count.
    Upcall,
}

/// Pure pinned Jim recipe. Actual table nodes and frame cleanup names are owned
/// by the interpreter; this value cannot authenticate a node or namespace.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeJimLocalProtocol(());
impl NativeJimLocalProtocol {
    /// Exact pure recipe for the pinned Jim implementation.
    #[must_use]
    pub const fn jim084() -> Self {
        Self(())
    }
    /// Occupied local publication changes the epoch; a fresh binding does not.
    #[must_use]
    pub const fn changes_epoch(self, occupied: bool) -> bool {
        occupied
    }
    /// Cached top-node selection precedes this live previous-node traversal.
    #[must_use]
    pub const fn follows_previous(self, procedure: bool, upcalls: usize) -> bool {
        procedure && upcalls != 0
    }
    /// Upcall requires an actual procedure and an owning previous-command edge.
    #[must_use]
    pub const fn accepts_upcall(self, procedure: bool, previous: bool) -> bool {
        procedure && previous
    }
    /// Native Jim `%#s` formats the object's resident bytes through snprintf.
    #[must_use]
    pub fn missing_previous_command(self, original: &[u8]) -> Vec<u8> {
        [
            b"no previous command: \"".as_slice(),
            tcl_core_types::c_string_extent(original),
            b"\"",
        ]
        .concat()
    }
    /// Failed successful-local result lookup reports the original name.
    #[must_use]
    pub fn missing_cleanup_command(self, original: &[u8]) -> Vec<u8> {
        [
            b"invalid command name \"".as_slice(),
            tcl_core_types::c_string_extent(original),
            b"\"",
        ]
        .concat()
    }
    /// Nonempty rename of a node with a previous edge is a native error.
    #[must_use]
    pub fn local_rename_error(self, original: &[u8]) -> Vec<u8> {
        [
            b"can't rename local command \"".as_slice(),
            tcl_core_types::c_string_extent(original),
            b"\"",
        ]
        .concat()
    }
    /// Deletion is permitted for local nodes; nonempty rename is refused.
    #[must_use]
    pub const fn accepts_rename(self, previous: bool, delete: bool) -> bool {
        delete || !previous
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn local_lineage_rules_match_original_native_nodes() {
        let recipe = NativeJimLocalProtocol::jim084();
        let rows = include_str!("../tests/data/native_jim_local/observations.tsv");
        for label in [
            "ordinary-local",
            "nested-one",
            "nested-two",
            "native-wrapper",
            "fresh-local",
            "rename-local",
            "base-after-delete",
            "delete-local-active-fresh",
        ] {
            assert!(
                rows.lines()
                    .any(|row| row.starts_with(&format!("{label}\t"))),
                "{label}"
            );
        }
        assert!(recipe.changes_epoch(true));
        assert!(!recipe.changes_epoch(false));
        assert!(recipe.follows_previous(true, 1));
        assert!(!recipe.follows_previous(false, 1));
        assert!(!recipe.follows_previous(true, 0));
        assert!(recipe.accepts_upcall(true, true));
        assert!(!recipe.accepts_upcall(false, true));
        assert!(!recipe.accepts_upcall(true, false));
        assert!(!recipe.accepts_rename(true, false));
        assert!(recipe.accepts_rename(true, true));
    }
}
