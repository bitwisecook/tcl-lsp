// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Bounded Logical source geometry, independently of Native naming authority.

use tcl_core_types::{ByteCommandSlot, ByteNamespacePath, NameBytes};

fn source_units(current: &ByteNamespacePath, written: &[u8]) -> Option<()> {
    (written.is_ascii()
        && !written.contains(&0)
        && current
            .as_segments()
            .iter()
            .all(|segment| segment.as_bytes().is_ascii() && !segment.as_bytes().contains(&0)))
    .then_some(())
}

/// Construct a bounded ASCII authored command slot under a retained Logical
/// source namespace. Written qualifiers use the shared parser; constructed
/// namespace components are appended without reparsing. No Native name recipe,
/// namespace existence, table binding or publication follows from this slot.
#[must_use]
pub fn authored_source_command_slot(
    current: &ByteNamespacePath,
    written: &[u8],
) -> Option<ByteCommandSlot> {
    // naming.source.original-declared-command-word-contract
    // docs/design/analysis/name-resolution-proofs/source-original-declared-command-word-contract.md
    source_units(current, written)?;
    let parts = super::qualifier_segments(written);
    let simple = super::written_command_tail(written);
    let parent_count = parts.len().checked_sub(usize::from(!simple.is_empty()))?;
    let parents = &parts[..parent_count];
    let mut namespace = if written.starts_with(b"::") {
        ByteNamespacePath::root()
    } else {
        current.clone()
    };
    for parent in parents {
        namespace.push(NameBytes::from(*parent));
    }
    Some(ByteCommandSlot::new(namespace, NameBytes::from(simple)))
}

/// Ordered Logical source candidates from independently retained namespace
/// components. This is conditional metadata geometry, not native lookup.
#[must_use]
pub fn authored_source_command_candidates(
    current: &ByteNamespacePath,
    path: &[ByteNamespacePath],
    written: &[u8],
) -> Option<Vec<ByteCommandSlot>> {
    // naming.source.original-declared-command-word-contract
    // docs/design/analysis/name-resolution-proofs/source-original-declared-command-word-contract.md
    if written.starts_with(b"::") {
        return Some(vec![authored_source_command_slot(current, written)?]);
    }
    let mut result = Vec::new();
    let root = ByteNamespacePath::root();
    for namespace in std::iter::once(current)
        .chain(path)
        .chain(std::iter::once(&root))
    {
        let key = authored_source_command_slot(namespace, written)?;
        if !result.contains(&key) {
            result.push(key);
        }
    }
    Some(result)
}

/// Retain a bounded authored namespace recipe. It provides source scope only;
/// a selected native input, allocated namespace or frame remains independent.
#[must_use]
pub fn authored_source_namespace_path(
    current: &ByteNamespacePath,
    written: &[u8],
) -> Option<ByteNamespacePath> {
    // naming.source.original-declared-command-word-contract
    // docs/design/analysis/name-resolution-proofs/source-original-declared-command-word-contract.md
    source_units(current, written)?;
    let mut namespace = if written.starts_with(b"::") {
        ByteNamespacePath::root()
    } else {
        current.clone()
    };
    for part in super::qualifier_segments(written) {
        if !part.is_empty() {
            namespace.push(NameBytes::from(part));
        }
    }
    Some(namespace)
}

#[cfg(test)]
mod tests {
    use super::{
        authored_source_command_candidates, authored_source_command_slot,
        authored_source_namespace_path,
    };
    use tcl_core_types::{ByteNamespacePath, NameBytes};

    #[test]
    fn original_authored_scope_preserves_constructed_components_and_empty_tail() {
        // naming.source.original-declared-command-word-contract
        // docs/design/analysis/name-resolution-proofs/source-original-declared-command-word-contract.md
        let mut current = ByteNamespacePath::root();
        current.push(NameBytes::from(&b"literal::component"[..]));
        let slot = authored_source_command_slot(&current, b"child:::command").unwrap();
        assert_eq!(
            slot.namespace.as_segments()[0].as_bytes(),
            b"literal::component"
        );
        assert_eq!(slot.namespace.as_segments()[1].as_bytes(), b"child");
        assert_eq!(slot.simple.as_bytes(), b"command");
        let empty = authored_source_command_slot(&current, b"child::").unwrap();
        assert!(empty.simple.as_bytes().is_empty());
        assert_eq!(empty.namespace, slot.namespace);
        let root = authored_source_command_slot(&current, b"::command").unwrap();
        assert!(root.namespace.is_root());
        let namespace = authored_source_namespace_path(&current, b"child").unwrap();
        assert_eq!(namespace, slot.namespace);
    }

    #[test]
    fn original_authored_candidates_keep_order_and_decline_unmodelled_units() {
        // naming.source.original-declared-command-word-contract
        // docs/design/analysis/name-resolution-proofs/source-original-declared-command-word-contract.md
        let mut current = ByteNamespacePath::root();
        current.push(NameBytes::from(&b"current"[..]));
        let mut search = ByteNamespacePath::root();
        search.push(NameBytes::from(&b"search"[..]));
        let candidates =
            authored_source_command_candidates(&current, &[search.clone()], b"command").unwrap();
        assert_eq!(candidates.len(), 3);
        assert_eq!(candidates[0].namespace, current);
        assert_eq!(candidates[1].namespace, search);
        assert!(candidates[2].namespace.is_root());
        assert_eq!(
            authored_source_command_candidates(&current, &[search], b"::command")
                .unwrap()
                .len(),
            1
        );
        for bytes in [&b"name\0suffix"[..], &b"name\xff"[..]] {
            assert!(authored_source_command_slot(&current, bytes).is_none());
            assert!(authored_source_namespace_path(&current, bytes).is_none());
        }
    }
}
