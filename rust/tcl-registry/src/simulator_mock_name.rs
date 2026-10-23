// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Authored simulator mock publication, independent of native name lookup.

/// A simulator's mock suffix and decision-log labels.
/// These labels describe authored mocks, without F5 binding or frame authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimulatorMockName {
    suffix: String,
    category: String,
    action: String,
}

impl SimulatorMockName {
    /// Project an ASCII simulator label with one optional constructed root.
    /// Nested labels share the first namespace and final action convention.
    /// Empty components, noncanonical separators and Unicode have no recipe.
    #[must_use]
    pub fn for_label(label: &str) -> Option<Self> {
        let label = tcl_syntax::naming::unroot_rooted_key(label).unwrap_or(label);
        if label.is_empty() || !label.is_ascii() || label.as_bytes().contains(&0) {
            return None;
        }
        if tcl_syntax::naming::unroot_rooted_key(label).is_some()
            || tcl_syntax::naming::canonical_written_command(label) != label
        {
            return None;
        }
        let parts = tcl_syntax::naming::key_segments(label);
        if parts.is_empty() || parts.iter().any(String::is_empty) || parts.join("::") != label {
            return None;
        }
        let action = parts.last()?;
        let (category, suffix) = if parts.len() > 1 {
            let category = parts.first()?.to_ascii_lowercase();
            let suffix = format!("{}_{}", safe_piece(&category), safe_piece(action));
            (category, suffix)
        } else {
            ("toplevel".to_owned(), format!("cmd_{}", safe_piece(action)))
        };
        if suffix.starts_with(':') {
            return None;
        }
        let action = action.clone();
        Some(Self {
            suffix,
            category,
            action,
        })
    }

    /// Exact mock suffix beneath the simulator's own command namespace.
    #[must_use]
    pub fn suffix(&self) -> &str {
        &self.suffix
    }

    /// Construct the mock command address beneath its authored publication namespace.
    /// This supplies no registered worker, original name lookup or F5 identity.
    #[must_use]
    pub fn command_address(&self) -> String {
        let namespace = tcl_core_types::ByteNamespacePath::from_segments(["itest", "cmd"]);
        let slot = tcl_core_types::ByteCommandSlot::new(
            namespace,
            tcl_core_types::NameBytes::from(self.suffix.as_str()),
        );
        String::from_utf8(tcl_syntax::naming::constructed_command_address_bytes(&slot))
            .expect("authored ASCII mock command address")
    }

    /// Authored decision-log category.
    #[must_use]
    pub fn category(&self) -> &str {
        &self.category
    }

    /// Authored decision-log action.
    #[must_use]
    pub fn action(&self) -> &str {
        &self.action
    }
}

fn safe_piece(piece: &str) -> String {
    piece.replace(['-', '.'], "_")
}

#[cfg(test)]
mod tests {
    use super::SimulatorMockName;

    #[test]
    fn authored_mock_addresses_keep_the_publication_namespace_and_literal_suffix() {
        for (label, expected) in [
            ("HTTP::header", "::itest::cmd::http_header"),
            ("::NS::inner::cmd", "::itest::cmd::ns_cmd"),
            ("traffic-group", "::itest::cmd::cmd_traffic_group"),
            ("a:b", "::itest::cmd::cmd_a:b"),
            ("$literal", "::itest::cmd::cmd_$literal"),
        ] {
            let publication = SimulatorMockName::for_label(label).unwrap();
            assert_eq!(publication.command_address(), expected);
        }
    }

    #[test]
    fn authored_mock_labels_preserve_scope_and_explicit_collisions() {
        for (label, suffix, category, action) in [
            ("HTTP::header", "http_header", "http", "header"),
            ("::HTTP::header", "http_header", "http", "header"),
            ("NS::inner::cmd", "ns_cmd", "ns", "cmd"),
            (
                "traffic-group",
                "cmd_traffic_group",
                "toplevel",
                "traffic-group",
            ),
            ("a:b", "cmd_a:b", "toplevel", "a:b"),
        ] {
            let projected = SimulatorMockName::for_label(label).unwrap();
            assert_eq!(
                (projected.suffix(), projected.category(), projected.action()),
                (suffix, category, action)
            );
        }
        assert_eq!(
            SimulatorMockName::for_label("NS::cmd"),
            SimulatorMockName::for_label("NS::inner::cmd")
        );
        for label in [
            "",
            "::::HTTP::header",
            "HTTP:::header",
            ":HTTP::cmd",
            "HTTP::",
            "é",
            "a\0b",
        ] {
            assert!(SimulatorMockName::for_label(label).is_none(), "{label:?}");
        }
    }
}
