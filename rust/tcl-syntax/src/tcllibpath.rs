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

use tcl_dialect::{EscapeSyntax, ListParse, TclVersion, model::DialectPoint};

/// Accepted list grammars for external Tcl builds with no selected dialect.
pub fn external_list_grammars() -> impl Iterator<Item = (ListParse, EscapeSyntax)> {
    EscapeSyntax::ALL.iter().flat_map(|&escapes| {
        [ListParse::Strict, ListParse::Lenient]
            .into_iter()
            .map(move |syntax| (syntax, escapes))
    })
}

/// Decode the discovery list only when the C Tcl releases agree on all words.
#[must_use]
pub fn discovery_directories(raw: &str) -> Vec<String> {
    let mut agreed = None;
    for version in TclVersion::ALL {
        let grammar = DialectPoint::for_tcl_version(version).grammar();
        let Ok(elements) = crate::list::split_list_in(raw, grammar.escapes) else {
            return Vec::new();
        };
        if let Some(previous) = &agreed {
            if previous != &elements {
                return Vec::new();
            }
        } else {
            agreed = Some(elements);
        }
    }
    agreed
        .unwrap_or_default()
        .into_iter()
        .map(std::borrow::Cow::into_owned)
        .collect()
}

/// Decode directories common to the possible external interpreter grammars.
#[must_use]
pub fn external_directories(raw: &str) -> Vec<String> {
    let mut common = None::<Vec<String>>;
    for (syntax, escapes) in external_list_grammars() {
        let Ok(elements) = crate::list::split_list_bytes_in(raw.as_bytes(), syntax, escapes) else {
            return Vec::new();
        };
        let paths: Vec<_> = elements
            .iter()
            .filter_map(|value| {
                let value = std::str::from_utf8(value).ok()?;
                (!value.is_empty() && !value.contains('\0')).then(|| value.to_owned())
            })
            .collect();
        match &mut common {
            Some(common) => common.retain(|directory| {
                paths
                    .iter()
                    .any(|path| std::path::Path::new(path) == std::path::Path::new(directory))
            }),
            None => common = Some(paths),
        }
    }
    common.unwrap_or_default()
}
