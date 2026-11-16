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

//! Resolving a command name, or the head of a literal callback prefix, to a procedure of the module.

#[cfg(test)]
use super::*;

// Call-target resolution

/// Predicate-backed String candidate reporting through the shared lookup order.
/// This path-free projection supplies no original body or implementation identity.
#[cfg(test)]
fn resolve_internal_call_with<F>(command: &str, caller_qname: &str, exists: F) -> Option<String>
where
    F: FnMut(&str) -> bool,
{
    if command.is_empty() {
        return None;
    }
    let ns_parts = namespace_parts_from_proc(caller_qname);
    let ns = if ns_parts.is_empty() {
        "::".to_owned()
    } else {
        format!("::{}", ns_parts.join("::"))
    };
    crate::naming::resolve_command_with::<&str, _>(&ns, &[], command, exists)
}

/// Resolve a command name to a qualified procedure name if it refers to one
/// defined in `known`.
///
/// Uses the shared existence-checked rule: an absolute name is looked up
/// directly; a relative name tries the caller's namespace first, then global,
/// and never walks intermediate ancestor namespaces.
#[must_use]
#[cfg(test)]
pub(super) fn resolve_internal_call<S: std::hash::BuildHasher>(
    command: &str,
    caller_qname: &str,
    known: &HashSet<String, S>,
) -> Option<String> {
    resolve_internal_call_with(command, caller_qname, |qname| known.contains(qname))
}

/// Top-level call-target resolver. Convenience wrapper that
/// handles the common case where the caller has no special
/// aliasing information.
#[must_use]
#[cfg(test)]
pub(super) fn resolve_call_target<S: std::hash::BuildHasher>(
    command: &str,
    _args: &[String],
    caller_qname: &str,
    known: &HashSet<String, S>,
) -> Option<String> {
    resolve_internal_call(command, caller_qname, known)
}

/// Head of a literal Tcl callback-list value under the selected grammar.
/// Braces, quotes and escapes belong to list syntax; bracket text in a literal
/// value remains data. Executed builders require original source receipts.
#[must_use]
pub fn command_prefix_head(
    registry: &tcl_registry::CommandRegistry,
    prefix: &str,
) -> Option<String> {
    command_prefix_literal_head(
        tcl_syntax::word_rules::WordValueRules::from_config(&tcl_lexer::LexerConfig::for_profile(
            registry.profile(),
        )),
        prefix,
    )
}

pub(crate) fn command_prefix_literal_head(
    rules: tcl_syntax::word_rules::WordValueRules,
    prefix: &str,
) -> Option<String> {
    rules
        .split_list(prefix)
        .ok()?
        .first()
        .map(ToString::to_string)
}

/// Return the namespace segments of a qualified proc name —
/// everything except the trailing simple name. The split is the one canonical
/// [`crate::naming::qualifier_segments_owned`] (colon runs are a single
/// separator, so `::a:::b::p` yields `["a", "b"]`).
#[must_use]
#[cfg(test)]
pub(super) fn namespace_parts_from_proc(qname: &str) -> Vec<String> {
    let mut parts = crate::naming::qualifier_segments_owned(qname);
    parts.pop();
    parts
}
