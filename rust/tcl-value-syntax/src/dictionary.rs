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

//! Tcl dictionary key order, duplicate resolution and counted parse diagnostics.

/// Layout of an audited native dictionary after a member store or removal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeDictionaryLayout {
    /// C Tcl 8.5–9.1 retain the order of all surviving members.
    Tcl,
    /// Jim stores members in a packed table and fills a removed slot from its end.
    Jim,
}

impl NativeDictionaryLayout {
    /// Select only stock engines that implement the dictionary object.
    #[must_use]
    pub const fn for_string_protocol(
        protocol: crate::native_string::NativeStringProtocol,
    ) -> Option<Self> {
        use crate::native_string::NativeStringProtocol;
        match protocol {
            NativeStringProtocol::C(tcl_dialect::TclVersion::V8_4) => None,
            NativeStringProtocol::C(_) => Some(Self::Tcl),
            NativeStringProtocol::Jim084 => Some(Self::Jim),
        }
    }

    /// Remove a previously located member in the selected engine's order.
    pub fn remove_member<T>(self, members: &mut Vec<T>, index: usize) {
        match self {
            Self::Tcl => {
                members.remove(index);
            }
            Self::Jim => {
                members.swap_remove(index);
            }
        }
    }

    /// Exact dictionary string from distinct ordered pairs with known bytes.
    #[must_use]
    pub fn render<K: AsRef<[u8]>, V: AsRef<[u8]>>(self, pairs: &[(K, V)]) -> Vec<u8> {
        let words: Vec<&[u8]> = pairs
            .iter()
            .flat_map(|(key, value)| [key.as_ref(), value.as_ref()])
            .collect();
        // Tcl's UpdateStringOfDict uses counted list-element conversion.
        // Jim's UpdateStringOfDict directly calls JimMakeListStringRep.
        match self {
            Self::Tcl => {
                crate::list_result::NativeListResultSerialization::Tcl85Plus.render(&words)
            }
            Self::Jim => crate::list_result::NativeListResultSerialization::Jim084.render(&words),
        }
    }
}

/// Dictionary wording of an actual typed list-element parse failure.
#[must_use]
pub fn dictionary_parse_message(error: crate::list::ListError, source: &[u8]) -> Vec<u8> {
    let message = error.full_message_bytes(source);
    let (prefix, replacement): (&[u8], &[u8]) = match error {
        crate::list::ListError::UnmatchedBrace => (
            b"unmatched open brace in list",
            b"unmatched open brace in dict",
        ),
        crate::list::ListError::UnmatchedQuote => (
            b"unmatched open quote in list",
            b"unmatched open quote in dict",
        ),
        _ => (b"list element in ", b"dict element in "),
    };
    let mut output = replacement.to_vec();
    output.extend_from_slice(&message[prefix.len()..]);
    output
}

/// Tcl's dict canonicalisation rule, as slot indices — **the** implementation of
/// "first-occurrence key position, last value wins".
///
/// `keys` is one key string per key/value slot, in source order: slot `i` is
/// elements `2i` / `2i + 1` of the flat list rep, or arguments `2i` / `2i + 1`
/// of `dict create`. The result is one `(key_slot, value_slot)` entry per
/// *surviving* key, in canonical order — the slot whose key spelling and
/// position the dict keeps, and the slot whose value it keeps. With no
/// duplicate key that is `[(0, 0), (1, 1), …]`; for `a 1 b 2 a 3` it is
/// `[(0, 2), (1, 1)]`.
///
/// This is `SetDictFromAny` (`tmp/tcl9.0.4/generic/tclDictObj.c:589`) walking
/// the list rep and `Tcl_DictObjPut`ing each pair: the hash entry for a
/// repeated key already exists, so the *value* is overwritten while the entry
/// keeps its original chain position. `DictCreateCmd` walks its arguments the
/// same way, which is why one function answers for both.
///
/// Working in indices rather than values is what lets every layer share it:
/// the runtime seam (`ValueOps::dict_pairs`) maps them back onto its own
/// value handles, and the compile-time folders (`tcl_registry::const_fold`'s
/// `parse_dict` / `fold_dict_create`, `tcl_compiler`'s `fold_dict_create_cmd`)
/// map them onto their own strings — none of them re-derives the rule.
///
/// That mattered: the rule existed in three independent copies plus one place
/// it had been *missed*, where `dict get {a 1 a 2} a` folded to `1` while both
/// tclsh oracles say `2`. Callers must not re-implement the walk; the
/// cross-crate parity gate `dict_canonicalisation_parity` fails when a copy
/// reappears and diverges.
///
/// The odd-length check belongs to the caller: what an unpaired trailing
/// element means (an error, or a declined fold) differs per layer.
///
/// The key type is borrowed and only `Eq + Hash`, so a byte-oriented value
/// model can bind this with `&[u8]` keys as readily as a string one does with
/// `&str` — keys are compared by their exact rep either way, which is what
/// `Tcl_DictObjPut`'s hash does.
///
/// The WASM runtime's *native* dict rep (`runtime/rust/src/dict.rs`)
/// deliberately does not call this: it maintains a live key index across
/// mutation, so it canonicalises incrementally (one `Tcl_DictObjPut` per
/// insert) rather than in one walk. Its agreement with this rule is pinned by
/// its own parity test rather than by construction.
#[must_use]
pub fn canonical_dict_slots<'a, K>(keys: impl IntoIterator<Item = &'a K>) -> Vec<(usize, usize)>
where
    K: ?Sized + Eq + std::hash::Hash + 'a,
{
    // Key → its slot's position in `slots`, so a duplicate is O(1) to find: a
    // linear re-scan per element made this O(N²) on every dict operation.
    // Both containers are sized from the iterator up front — this runs on every
    // VM dict opcode, so the growth reallocations are worth avoiding.
    let keys = keys.into_iter();
    let expected = keys.size_hint().0;
    let mut first_seen: std::collections::HashMap<&'a K, usize> =
        std::collections::HashMap::with_capacity(expected);
    let mut slots: Vec<(usize, usize)> = Vec::with_capacity(expected);
    for (slot, key) in keys.enumerate() {
        if let Some(&position) = first_seen.get(key) {
            slots[position].1 = slot; // last value wins, first position kept
        } else {
            first_seen.insert(key, slots.len());
            slots.push((slot, slot));
        }
    }
    slots
}
