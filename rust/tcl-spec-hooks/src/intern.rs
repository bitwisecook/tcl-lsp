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

//! `&'static str` for strings a hook body produced.
//!
//! Several registry types hold `&'static str` because a shipped spec's strings
//! live in `.rodata`. A pack's are minted at run time, so they are leaked —
//! the same trade the pack loader makes for the spec itself — and interned so
//! a hook emitting the same message on every call site leaks once rather than
//! once per call.

use std::collections::HashSet;
use std::sync::{LazyLock, Mutex};

static INTERNED: LazyLock<Mutex<HashSet<&'static str>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));

/// The `&'static str` for `text`, leaking it on first sight.
pub fn intern(text: &str) -> &'static str {
    let mut table = match INTERNED.lock() {
        Ok(table) => table,
        Err(poisoned) => poisoned.into_inner(),
    };
    if let Some(existing) = table.get(text) {
        return existing;
    }
    let leaked: &'static str = Box::leak(text.to_owned().into_boxed_str());
    table.insert(leaked);
    leaked
}

/// The `&'static [&'static str]` for a word list — an `allowed_values` table
/// on a literal-argument issue.
pub fn intern_words(words: &[String]) -> &'static [&'static str] {
    if words.is_empty() {
        return &[];
    }
    let leaked: Vec<&'static str> = words.iter().map(|word| intern(word)).collect();
    Box::leak(leaked.into_boxed_slice())
}

#[cfg(test)]
mod tests {
    use super::{intern, intern_words};

    #[test]
    fn the_same_text_leaks_once() {
        let first = intern("a message");
        let second = intern("a message");
        assert!(std::ptr::eq(first, second));
    }

    #[test]
    fn an_empty_word_list_leaks_nothing() {
        assert!(intern_words(&[]).is_empty());
        assert_eq!(intern_words(&["a".to_owned(), "b".to_owned()]), ["a", "b"]);
    }
}
