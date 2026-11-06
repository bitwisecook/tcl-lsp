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

//! Authored namespace-import hints from a static wrapper-shaped source call.

/// Alias spelling retained by the explicitly authored hint consumer.
#[derive(Clone, Copy)]
pub enum AuthoredImportAliasSpelling {
    /// Preserve absolute source spelling and join relative aliases verbatim.
    Written,
    /// Canonicalise a written alias under the constructed caller namespace.
    Canonical,
}

/// Conditional hint geometry, independent of wrapper dispatch or import effects.
pub struct AuthoredImportWrapperHint {
    /// Authored alias namespace candidate.
    pub alias_namespace: String,
    /// Root-decorated source pattern supplied as presentation advice.
    pub pattern: String,
    /// Shared canonical source namespace candidate.
    pub source_namespace: String,
}

/// A static `<namespace>::import <alias>` shape supplies only conjectural advice.
/// This does not prove the wrapper's implementation, import flags or a Native source.
#[must_use]
pub fn authored_import_wrapper_hint(
    head: &str,
    alias: &str,
    caller_namespace: &str,
    alias_spelling: AuthoredImportAliasSpelling,
) -> Option<AuthoredImportWrapperHint> {
    let source = head.strip_suffix("::import")?;
    if alias.is_empty() || alias.contains(['$', '[']) {
        return None;
    }
    let pattern_namespace = if super::unroot_rooted_key(source).is_some() {
        source.to_owned()
    } else {
        super::root_unrooted_key(source)
    };
    let alias_namespace = match alias_spelling {
        AuthoredImportAliasSpelling::Canonical => super::qualify(caller_namespace, alias),
        AuthoredImportAliasSpelling::Written => {
            if super::unroot_rooted_key(alias).is_some() {
                alias.to_owned()
            } else if caller_namespace == "::" {
                super::root_unrooted_key(alias)
            } else {
                format!("{caller_namespace}::{alias}")
            }
        }
    };
    Some(AuthoredImportWrapperHint {
        alias_namespace,
        pattern: format!("{pattern_namespace}::*"),
        source_namespace: super::qualify_namespace(caller_namespace, source),
    })
}
