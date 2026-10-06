// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Written namespace ranges selected from original bytes and retained geometry.

use std::ops::Range;

use super::{NativeNameContext, NativeNameProtocol};

fn selects(
    protocol: NativeNameProtocol,
    current: NativeNameContext<'_>,
    original: &[u8],
    wanted: NativeNameContext<'_>,
) -> bool {
    match protocol {
        NativeNameProtocol::C(_) => protocol
            .namespace_address_path(current, original)
            .is_ok_and(|path| path == *wanted.namespace),
        NativeNameProtocol::Jim084 => wanted.jim_namespace_object.is_some_and(|wanted| {
            protocol
                .jim_namespace_canonical_input(current, original)
                .is_ok_and(|selected| selected.selected() == wanted)
        }),
    }
}

/// The shortest nonempty original prefix selecting the exact wanted namespace.
/// Only the original operand is interpreted. Retained contexts supply no namespace
/// existence, entered frame, command token or native compiler authority.
#[must_use]
pub fn native_written_namespace_prefix_extent(
    protocol: NativeNameProtocol,
    current: NativeNameContext<'_>,
    original: &[u8],
    wanted: NativeNameContext<'_>,
) -> Option<Range<usize>> {
    let root = match protocol {
        NativeNameProtocol::C(_) => wanted.namespace.is_root(),
        NativeNameProtocol::Jim084 => wanted.jim_namespace_object?.is_empty(),
    };
    if root {
        return None;
    }
    match protocol {
        NativeNameProtocol::C(_) => {
            let mut selected = protocol.namespace_address_path(current, original).ok()?;
            loop {
                if selected == *wanted.namespace {
                    break;
                }
                selected = selected.parent()?;
            }
        }
        NativeNameProtocol::Jim084 => {
            let selected = protocol
                .jim_namespace_canonical_input(current, original)
                .ok()?;
            let mut object = selected.selected();
            let wanted = wanted.jim_namespace_object?;
            loop {
                if object == wanted {
                    break;
                }
                if object.is_empty() {
                    return None;
                }
                let parent = protocol.namespace_qualifier_bytes(object);
                if parent == object {
                    return None;
                }
                object = parent;
            }
        }
    }
    (1..=original.len())
        .find(|&end| selects(protocol, current, &original[..end], wanted))
        .map(|end| 0..end)
}

/// The last namespace component written in the original prefix, with its exact
/// retained parent selected independently. A relative word omitting that component
/// has no range. Opaque Jim namespace objects without a supported parent decline.
#[must_use]
pub fn native_written_namespace_member_extent(
    protocol: NativeNameProtocol,
    current: NativeNameContext<'_>,
    original: &[u8],
    wanted: NativeNameContext<'_>,
) -> Option<Range<usize>> {
    let end = native_written_namespace_prefix_extent(protocol, current, original, wanted)?.end;
    let start = match protocol {
        NativeNameProtocol::C(_) => {
            let parent = wanted.namespace.parent()?;
            let parent = NativeNameContext::new(&parent);
            (0..end).rfind(|&start| {
                if start == 0 {
                    current.namespace == parent.namespace
                } else {
                    selects(protocol, current, &original[..start], parent)
                }
            })?
        }
        NativeNameProtocol::Jim084 => {
            let object = wanted.jim_namespace_object?;
            if object.contains(&0) {
                return None;
            }
            let parent = protocol.namespace_qualifier_bytes(object);
            let root = NativeNameContext::root();
            let parent = NativeNameContext::with_jim_namespace(root.namespace, parent);
            (0..end).rfind(|&start| {
                if start == 0 {
                    current.jim_namespace_object.unwrap_or(b"")
                        == parent.jim_namespace_object.unwrap_or(b"")
                } else {
                    selects(protocol, current, &original[..start], parent)
                }
            })?
        }
    };
    (start < end).then_some(start..end)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_core_types::ByteNamespacePath;
    use tcl_dialect::TclVersion;

    #[test]
    fn namespace_spans_preserve_original_colon_runs_and_retained_components() {
        for version in [
            TclVersion::V8_4,
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ] {
            let protocol = NativeNameProtocol::C(version);
            let wanted = ByteNamespacePath::from_segments(["app", "old"]);
            let wanted = NativeNameContext::new(&wanted);
            assert_eq!(
                native_written_namespace_member_extent(
                    protocol,
                    NativeNameContext::root(),
                    b"::::app:::old::leaf",
                    wanted
                ),
                Some(10..13)
            );
            let current = ByteNamespacePath::from_segments(["app"]);
            assert_eq!(
                native_written_namespace_member_extent(
                    protocol,
                    NativeNameContext::new(&current),
                    b"old::leaf",
                    wanted
                ),
                Some(0..3)
            );
            let parent = NativeNameContext::new(&current);
            assert_eq!(
                native_written_namespace_member_extent(protocol, parent, b"old::leaf", parent),
                None
            );
        }
    }

    #[test]
    fn namespace_spans_do_not_recover_display_colliding_scope_components() {
        let first = ByteNamespacePath::from_segments(["a:", "b"]);
        let second = ByteNamespacePath::from_segments(["a", ":b"]);
        let protocol = NativeNameProtocol::C(TclVersion::V9_0);
        for path in [&first, &second] {
            assert_eq!(
                native_written_namespace_prefix_extent(
                    protocol,
                    NativeNameContext::root(),
                    b"::a:::b",
                    NativeNameContext::new(path)
                ),
                None
            );
        }
        let jim = NativeNameProtocol::Jim084;
        let root = NativeNameContext::root();
        let wanted = NativeNameContext::with_jim_namespace(root.namespace, b"app::old");
        assert_eq!(
            native_written_namespace_member_extent(jim, root, b"::app::old::leaf", wanted),
            Some(7..10)
        );
    }
}
