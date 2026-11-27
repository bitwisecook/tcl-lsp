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

use super::*;

#[test]
fn source_memos_do_not_change_identity_or_retain_dead_allocations() {
    let text: Arc<str> = Arc::from("set value 1");
    let first = SourceOriginId::authored(&text);
    let equal = SourceOriginId::authored(&text);
    assert_eq!(first, equal);
    assert!(!Arc::ptr_eq(first.shared_kind(), equal.shared_kind()));
    let memo = first.original_lexical_projection();
    let clone = first.clone();
    assert!(Arc::ptr_eq(&memo, &clone.original_lexical_projection()));
    assert!(!Arc::ptr_eq(&memo, &equal.original_lexical_projection()));
    let hash = |source: &SourceOriginId| {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        source.hash(&mut hasher);
        std::hash::Hasher::finish(&hasher)
    };
    let before = hash(&first);
    let config =
        tcl_lexer::LexerConfig::for_file_grammar(tcl_dialect::DialectProfile::plain_tcl().grammar);
    assert!(first.original_lexical_command(&[], 0, config).is_none());
    assert_eq!(hash(&first), before);
    assert_eq!(first, equal);
    let owner = Arc::downgrade(first.shared_kind());
    let memo_lifetime = Arc::downgrade(&memo);
    drop(memo);
    drop(first);
    drop(clone);
    assert!(owner.upgrade().is_none());
    let _ = equal.original_lexical_projection();
    assert!(memo_lifetime.upgrade().is_none());
}
