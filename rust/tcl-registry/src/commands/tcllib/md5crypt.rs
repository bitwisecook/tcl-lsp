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

//! tcllib `md5crypt` package — MD5-based password encryption (crypt(3) style).
//!
//! Public command surface from tcllib `modules/md5crypt/md5crypt.man`.
//! Requires Tcl 8.5+ and the `md5` package.

use crate::prelude::*;

/// A pure password-hashing command (`md5crypt`, `aprcrypt`).
fn crypt(
    name: &'static str,
    synopsis: &'static [&'static str],
    summary: &'static str,
) -> CommandSpec {
    CommandSpec {
        name,
        traits: Traits::PURE,
        arity: Arity::exact(2),
        hover: Some(HoverSnippet {
            summary,
            synopsis,
            snippet: "",
            source: "tcllib md5crypt package",
            examples: "",
            return_value: "The encrypted password string in crypt(3) format.",
        }),
        tcllib_package: Some("md5crypt"),
        required_package: Some("md5crypt"),
        ..CommandSpec::DEFAULT
    }
}

/// All `md5crypt` command specs.
pub fn specs() -> Vec<CommandSpec> {
    vec![
        crypt(
            "md5crypt::md5crypt",
            &["md5crypt::md5crypt password salt"],
            "Encrypt a password using the MD5-based crypt algorithm.",
        ),
        crypt(
            "md5crypt::aprcrypt",
            &["md5crypt::aprcrypt password salt"],
            "Encrypt a password using the Apache-variant MD5 crypt algorithm.",
        ),
        CommandSpec {
            name: "md5crypt::salt",
            traits: Traits::empty(),
            arity: Arity::new(0, 1),
            hover: Some(HoverSnippet {
                summary: "Generate a random salt string for md5crypt.",
                synopsis: &["md5crypt::salt ?length?"],
                snippet: "",
                source: "tcllib md5crypt package",
                examples: "",
                return_value: "A random salt string of the requested length (default 8).",
            }),
            tcllib_package: Some("md5crypt"),
            required_package: Some("md5crypt"),
            ..CommandSpec::DEFAULT
        },
    ]
}
