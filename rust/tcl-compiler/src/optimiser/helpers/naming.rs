// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Namespace and qualified-name helpers for the optimiser.

use crate::interprocedural::InterproceduralAnalysis;
use crate::naming::normalise_qualified_name;

/// Split a namespace string into its `::`-delimited parts,
/// skipping empty segments. `"::"` yields `[]`, `"::foo::bar"`
/// yields `["foo", "bar"]`.
#[must_use]
pub fn namespace_parts(namespace: &str) -> Vec<String> {
    normalise_qualified_name(namespace)
        .split("::")
        .filter(|p| !p.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Extract the enclosing-namespace portion of a fully qualified
/// proc name.
///
/// - `"::"` → `"::"`.
/// - `"::foo"` → `"::"`.
/// - `"::a::b::c"` → `"::a::b"`.
#[must_use]
pub fn namespace_from_qualified(qname: &str) -> String {
    let parts = namespace_parts(qname);
    if parts.len() <= 1 {
        return "::".into();
    }
    let prefix: Vec<&str> = parts[..parts.len() - 1]
        .iter()
        .map(String::as_str)
        .collect();
    format!("::{}", prefix.join("::"))
}

/// Resolve a Tcl command name to a fully qualified proc summary
/// key, walking up the caller's namespace for bare names.
/// Resolution order:
///
/// 1. Absolute names starting with `::` → looked up directly.
/// 2. Names containing `::` (but not starting with `::`) → treated
///    as global-relative (prefixed with `::`).
/// 3. Bare names → walked from innermost namespace up to global.
///
/// Returns `None` when no candidate is known to `interproc`.
#[must_use]
pub fn resolve_summary_proc_name(
    command: &str,
    namespace: &str,
    interproc: &InterproceduralAnalysis,
) -> Option<String> {
    if command.is_empty() {
        return None;
    }
    if command.starts_with("::") {
        let qname = normalise_qualified_name(command);
        return interproc.procedures.contains_key(&qname).then_some(qname);
    }
    if command.contains("::") {
        let qname = normalise_qualified_name(&format!("::{command}"));
        return interproc.procedures.contains_key(&qname).then_some(qname);
    }

    let ns_parts = namespace_parts(namespace);
    for depth in (0..=ns_parts.len()).rev() {
        let prefix = &ns_parts[..depth];
        let candidate = if prefix.is_empty() {
            format!("::{command}")
        } else {
            let joined = prefix.join("::");
            format!("::{joined}::{command}")
        };
        if interproc.procedures.contains_key(&candidate) {
            return Some(candidate);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interprocedural::ProcSummary;

    fn interproc_with(names: &[&str]) -> InterproceduralAnalysis {
        let mut ip = InterproceduralAnalysis::default();
        for name in names {
            ip.procedures
                .insert((*name).into(), ProcSummary::unknown(*name));
        }
        ip
    }

    #[test]
    fn namespace_parts_splits_and_filters_empty() {
        assert_eq!(namespace_parts("::"), Vec::<String>::new());
        assert_eq!(namespace_parts("::foo"), vec!["foo".to_string()]);
        assert_eq!(
            namespace_parts("::a::b::c"),
            vec!["a".to_string(), "b".into(), "c".into()],
        );
    }

    #[test]
    fn namespace_from_qualified_peels_last_segment() {
        assert_eq!(namespace_from_qualified("::"), "::");
        assert_eq!(namespace_from_qualified("::foo"), "::");
        assert_eq!(namespace_from_qualified("::a::b"), "::a");
        assert_eq!(namespace_from_qualified("::a::b::c"), "::a::b");
    }

    #[test]
    fn resolve_empty_command_returns_none() {
        let ip = interproc_with(&["::foo"]);
        assert!(resolve_summary_proc_name("", "::", &ip).is_none());
    }

    #[test]
    fn resolve_absolute_name_directly() {
        let ip = interproc_with(&["::foo"]);
        assert_eq!(
            resolve_summary_proc_name("::foo", "::other", &ip),
            Some("::foo".into()),
        );
        assert!(resolve_summary_proc_name("::missing", "::", &ip).is_none());
    }

    #[test]
    fn resolve_global_relative_containing_colons() {
        let ip = interproc_with(&["::ns::bar"]);
        assert_eq!(
            resolve_summary_proc_name("ns::bar", "::other", &ip),
            Some("::ns::bar".into()),
        );
    }

    #[test]
    fn resolve_bare_name_walks_up_namespace() {
        let ip = interproc_with(&["::a::b::foo", "::a::foo", "::foo"]);
        // From ::a::b — innermost wins.
        assert_eq!(
            resolve_summary_proc_name("foo", "::a::b", &ip),
            Some("::a::b::foo".into()),
        );
        // From ::a — skips ::a::b, hits ::a.
        assert_eq!(
            resolve_summary_proc_name("foo", "::a", &ip),
            Some("::a::foo".into()),
        );
        // From :: — only global matches.
        assert_eq!(
            resolve_summary_proc_name("foo", "::", &ip),
            Some("::foo".into()),
        );
    }

    #[test]
    fn resolve_bare_name_none_when_missing_everywhere() {
        let ip = interproc_with(&["::nope"]);
        assert!(resolve_summary_proc_name("foo", "::a::b", &ip).is_none());
    }
}
