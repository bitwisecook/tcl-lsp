// SPDX-License-Identifier: AGPL-3.0-or-later
//! Symbolic source geometry of the stock C9 `TclOO` link worker.

use super::{NativeNameProtocol, canonical_written_command};
use tcl_dialect::TclVersion;

/// A source location relative to the actual receiver or the interpreter root.
/// It supplies neither an object namespace nor an installed alias allocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeOoLinkAliasGeometry {
    /// The link worker prefixes the actual receiver namespace.
    ReceiverRelative(String),
    /// The original alias names a root-relative slot.
    Absolute(String),
}

/// Stock C9 `link` concatenates its receiver namespace and `::` before parsing
/// a relative alias. That separator absorbs a single leading colon.
#[must_use]
pub fn native_oo_link_alias_geometry(
    protocol: NativeNameProtocol,
    alias: &[u8],
) -> Option<NativeOoLinkAliasGeometry> {
    if !matches!(protocol, NativeNameProtocol::C(version) if version >= TclVersion::V9_0)
        || alias.contains(&0)
    {
        return None;
    }
    let alias = std::str::from_utf8(alias).ok()?;
    if alias.starts_with("::") {
        Some(NativeOoLinkAliasGeometry::Absolute(
            canonical_written_command(alias),
        ))
    } else {
        let relative = alias.strip_prefix(':').unwrap_or(alias);
        Some(NativeOoLinkAliasGeometry::ReceiverRelative(
            canonical_written_command(relative),
        ))
    }
}

/// Original call spelling has its own separator geometry. A lone leading
/// colon remains data here, independently of how `link` published its alias.
#[must_use]
pub fn native_oo_link_call_geometry(
    protocol: NativeNameProtocol,
    command: &[u8],
) -> Option<NativeOoLinkAliasGeometry> {
    if !matches!(protocol, NativeNameProtocol::C(version) if version >= TclVersion::V9_0)
        || command.contains(&0)
    {
        return None;
    }
    let command = std::str::from_utf8(command).ok()?;
    let geometry = canonical_written_command(command);
    Some(if command.starts_with("::") {
        NativeOoLinkAliasGeometry::Absolute(geometry)
    } else {
        NativeOoLinkAliasGeometry::ReceiverRelative(geometry)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stock_link_publication_and_call_geometry_match_original_c9_results() {
        for (version, output) in [
            (
                TclVersion::V9_0,
                include_str!(
                    "../../../tcl-registry/tests/data/source_contract_vectors/tcloo-linked-source/publication-followon/9.0.4/stdout.bin"
                ),
            ),
            (
                TclVersion::V9_1,
                include_str!(
                    "../../../tcl-registry/tests/data/source_contract_vectors/tcloo-linked-source/publication-followon/9.1.0/stdout.bin"
                ),
            ),
        ] {
            assert!(output.contains("PUBLICATION {ordinary 0 TARGET loneColon 1 {invalid command name \":shortcut\"} zeroArguments 0 {}}"));
            let recipe = NativeNameProtocol::C(version);
            let alias = native_oo_link_alias_geometry(recipe, b":shortcut").unwrap();
            assert_eq!(
                Some(alias.clone()),
                native_oo_link_call_geometry(recipe, b"shortcut")
            );
            assert_ne!(
                Some(alias),
                native_oo_link_call_geometry(recipe, b":shortcut")
            );
            assert_eq!(
                native_oo_link_alias_geometry(recipe, b"nested:::shortcut"),
                native_oo_link_call_geometry(recipe, b"nested::shortcut")
            );
            assert_eq!(
                native_oo_link_alias_geometry(recipe, b":::rootAlias"),
                native_oo_link_call_geometry(recipe, b"::rootAlias")
            );
            assert_eq!(
                native_oo_link_alias_geometry(recipe, b""),
                native_oo_link_call_geometry(recipe, b"")
            );
            assert_eq!(
                native_oo_link_alias_geometry(recipe, b"$literal"),
                native_oo_link_call_geometry(recipe, b"$literal")
            );
        }
        assert!(
            native_oo_link_alias_geometry(NativeNameProtocol::C(TclVersion::V8_6), b"shortcut")
                .is_none()
        );
        assert!(native_oo_link_alias_geometry(NativeNameProtocol::Jim084, b"shortcut").is_none());
    }
}
