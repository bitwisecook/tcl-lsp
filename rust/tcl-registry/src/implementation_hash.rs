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

//! The one hasher an implementation's content is named by. A declared body's
//! identity (`ImplementationIdentity::content_hash`, over its parameters and its
//! text, as a reference body and the loader's `evaluate -implementation` derive
//! it) and an extension artefact's name (`extension_host::artefact_hash`, over
//! its bytes) both come from [`content_hash`], so an edited body or artefact is
//! a different implementation under the same rule.
//!
//! One hasher does not make two identities one: a body and an artefact hash
//! different content, and nothing yet binds an extension's artefact hash to an
//! implementation identity — the extension evaluation route and the shipped
//! implementation's attestation are where that binding will be stated.

use std::hash::{DefaultHasher, Hash, Hasher};

/// The content hash of `content`: the standard library's default hasher over its
/// `Hash`, the in-process key every implementation identity is compared by.
#[must_use]
pub fn content_hash<T: Hash + ?Sized>(content: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::content_hash;

    /// Hashing a body's parts in one call is hashing them one after another, so
    /// every identity derived before the helper keeps its number.
    #[test]
    fn a_body_s_parts_hash_as_they_did_one_by_one() {
        use std::hash::{DefaultHasher, Hash, Hasher};
        let (params, text) = (vec!["a".to_owned(), "b".to_owned()], "expr {$a + $b}");
        let mut hasher = DefaultHasher::new();
        params.hash(&mut hasher);
        text.hash(&mut hasher);
        assert_eq!(content_hash(&(&params, text)), hasher.finish());
        assert_ne!(content_hash(&(&params, "expr {$a - $b}")), hasher.finish());
    }
}
