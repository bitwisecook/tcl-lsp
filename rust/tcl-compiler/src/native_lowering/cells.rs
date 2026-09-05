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

//! Variable cells: the cell-storage lattice (plan §3.4) and the native
//! *shadow* of a cell's value that lets values stay native between the
//! statements that write and read them.
//!
//! A top-level script keeps every variable as a named runtime cell that is
//! written at the statement defining it (a hosted module must leave its
//! globals observable). What the native tier elides is the *read back*: when
//! no trace, no invocation, and no other observer can reach the cell between
//! its write and a later read, the later read reuses the NLIR value that was
//! written. That value is the cell's shadow.
//!
//! Shadows are tracked per block. They flow along an edge only when the
//! successor has exactly that one predecessor and is not a loop header, so a
//! join or a back edge never sees a shadow from just one of its paths.
//!
//! This module is also the **single owner** of "which cell does this statically
//! spelled name or variable word denote" for every backend — [`cell_place`] for
//! a name word, [`variable_word_place`] for a `$…` / `${…}` word. Both are
//! built on the `tcl_syntax::naming` split rules, so no backend re-parses an
//! argument's compatibility text for itself (issue #1772).

use std::collections::BTreeMap;

use tcl_lexer::Span;

use super::ir::NativeValueId;
use crate::codegen::values::is_bare_var_name;
use crate::executable_ir::CellReference;
use crate::ir::{Provenance, SourceSite};

/// One Tcl variable cell addressed by name.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CellPlace {
    /// A scalar variable (or a whole array, when written as one).
    Named {
        /// The exact variable name.
        name: String,
    },
    /// One element of an array with a literal key.
    Element {
        /// The array name.
        name: String,
        /// The literal element key.
        key: String,
    },
}

impl CellPlace {
    /// The base variable name the place belongs to.
    #[must_use]
    pub fn base(&self) -> &str {
        match self {
            Self::Named { name } | Self::Element { name, .. } => name,
        }
    }

    /// The name as Tcl spells it (`a` or `a(k)`).
    #[must_use]
    pub fn spelling(&self) -> String {
        match self {
            Self::Named { name } => name.clone(),
            Self::Element { name, key } => format!("{name}({key})"),
        }
    }
}

/// Why a variable word could not be resolved to a cell statically.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum VariableWordDecline {
    /// The word is not one whole static reference, or its element key
    /// substitutes.
    Dynamic,
    /// `$a(b)` and `${a(b)}` share a compatibility spelling and the recorded
    /// lexical extent cannot tell the two apart.
    Ambiguous,
}

/// True when `text` carries a substitution a static reading cannot resolve.
pub(crate) fn has_substitution(text: &str) -> bool {
    text.bytes().any(|byte| matches!(byte, b'$' | b'[' | b'\\'))
}

/// The exact Tcl variable name `spelling` refers to, when the **whole** word is
/// one simple variable reference — otherwise `None`.
///
/// The two spellings are **not** validated alike, and that asymmetry is the
/// point: braces are Tcl's own escape for a name the bare charset cannot
/// express, so `${…}` accepts any non-empty name verbatim, while a bare
/// `$name` must be a whole [`is_bare_var_name`] run — otherwise the word is
/// `$name` *followed by literal text* (`$item-suffix`) and loading
/// `item-suffix` would be wrong. Routing the braced form through the charset
/// check as well would change behaviour, not just deduplicate.
///
/// Note the braced form here is decided by the *last* `}` in the word, so it
/// only ever sees a word the caller already knows is one variable reference.
/// The release-aware `${…}` close rule lives with the decoders in
/// `codegen::values::parse_simple_var_ref` and is issue #1568's territory.
pub(crate) fn whole_reference(spelling: &str) -> Option<&str> {
    if let Some(name) = spelling
        .strip_prefix("${")
        .and_then(|rest| rest.strip_suffix('}'))
    {
        return (!name.is_empty()).then_some(name);
    }
    let name = spelling.strip_prefix('$')?;
    is_bare_var_name(name).then_some(name)
}

/// The cell a statically spelled variable **name** denotes, or `None` when the
/// name is computed at run time.
///
/// `braced` is the source token's brace-literal flag: a braced name word
/// suppresses substitution, so `{a($k)}` names a literal element of `a`.
pub(crate) fn cell_place(name: &str, braced: bool) -> Option<CellPlace> {
    match CellReference::from_name(name, braced) {
        CellReference::Named {
            name: base,
            element,
        } => {
            if !element {
                return Some(CellPlace::Named { name: base });
            }
            let (_, key) = tcl_syntax::naming::split_array_name_braced(name, braced);
            let key = key?;
            if !braced && has_substitution(key) {
                return None;
            }
            Some(CellPlace::Element {
                name: base,
                key: key.to_owned(),
            })
        }
        CellReference::Computed => None,
    }
}

/// The cell a `$…` / `${…}` variable **word** reads.
///
/// The compatibility spelling normalises both `$a(b)` and `${a(b)}` to
/// `${a(b)}`, so the recorded lexical extent is what tells them apart: a
/// `${…}` reference is exactly two bytes longer than its name, a `$…` exactly
/// one. Only the latter is an array-element access; `${a(b)}` names a scalar
/// whose name happens to contain parentheses.
pub(crate) fn variable_word_place(
    spelling: &str,
    source: &SourceSite,
) -> Result<CellPlace, VariableWordDecline> {
    let name = whole_reference(spelling).ok_or(VariableWordDecline::Dynamic)?;
    let Some((base, key)) = tcl_syntax::naming::split_element_ref(name) else {
        return Ok(CellPlace::Named {
            name: name.to_owned(),
        });
    };
    if source.provenance != Provenance::Source {
        return Err(VariableWordDecline::Ambiguous);
    }
    match extent(source.span).checked_sub(name.len()) {
        // `${name}` — the whole thing is one scalar name.
        Some(2) => Ok(CellPlace::Named {
            name: name.to_owned(),
        }),
        // `$name(key)` — a genuine array element access.
        Some(1) if has_substitution(key) => Err(VariableWordDecline::Dynamic),
        Some(1) => Ok(CellPlace::Element {
            name: base.to_owned(),
            key: key.to_owned(),
        }),
        _ => Err(VariableWordDecline::Ambiguous),
    }
}

fn extent(span: Span) -> usize {
    span.end().saturating_sub(span.start()) as usize
}

/// Where a cell lives — the cell-storage lattice element decided for a
/// function's variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CellStorage {
    /// No runtime cell: the value lives only in NLIR values.
    Register,
    /// An indexed runtime slot whose name is bound lazily.
    Slot(u32),
    /// A named runtime cell; traces and introspection see it.
    Cell,
    /// A cell linked to another frame's cell (`upvar`/`global` target).
    Linked,
}

impl CellStorage {
    /// Stable Explorer spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Register => "register",
            Self::Slot(_) => "slot",
            Self::Cell => "cell",
            Self::Linked => "linked",
        }
    }
}

/// The native shadows live in one block.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ShadowState {
    shadows: BTreeMap<CellPlace, NativeValueId>,
}

impl ShadowState {
    /// The value a read of `place` may reuse instead of reading the cell.
    #[must_use]
    pub fn read(&self, place: &CellPlace) -> Option<NativeValueId> {
        self.shadows.get(place).copied()
    }

    /// Record that `place` now holds `value`.
    ///
    /// A whole-variable write invalidates every element shadow of the same
    /// base name, and an element write invalidates the whole-variable shadow.
    pub fn write(&mut self, place: CellPlace, value: NativeValueId) {
        let base = place.base().to_owned();
        self.shadows
            .retain(|shadowed, _| shadowed.base() != base || shadowed == &place);
        self.shadows.insert(place, value);
    }

    /// Forget every shadow of `base` and its elements.
    pub fn forget_base(&mut self, base: &str) {
        self.shadows.retain(|shadowed, _| shadowed.base() != base);
    }

    /// Forget every shadow: an observer may have reached any cell.
    pub fn clobber(&mut self) {
        self.shadows.clear();
    }

    /// Keep only the shadows this state and `other` agree on.
    ///
    /// The merge for a conditionally executed arm: an entry survives only when
    /// both paths reach it with the same value, so a shadow an arm established
    /// (whose defining operation the other path never runs) and one an arm
    /// invalidated are both dropped.
    pub fn intersect(&mut self, other: &Self) {
        self.shadows
            .retain(|place, value| other.shadows.get(place) == Some(value));
    }

    /// Whether no shadow is held.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.shadows.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intersect_keeps_only_shadows_both_paths_agree_on() {
        let place = |name: &str| CellPlace::Named {
            name: name.to_owned(),
        };
        let mut taken = ShadowState::default();
        taken.write(place("a"), NativeValueId(1));
        taken.write(place("b"), NativeValueId(2));
        taken.write(place("c"), NativeValueId(3));
        let mut other = ShadowState::default();
        other.write(place("a"), NativeValueId(1));
        other.write(place("b"), NativeValueId(9));
        taken.intersect(&other);
        assert_eq!(taken.read(&place("a")), Some(NativeValueId(1)));
        assert_eq!(taken.read(&place("b")), None, "values disagree");
        assert_eq!(taken.read(&place("c")), None, "only one path has it");
    }

    #[test]
    fn element_and_whole_shadows_invalidate_each_other() {
        let mut state = ShadowState::default();
        let whole = CellPlace::Named {
            name: "a".to_owned(),
        };
        let element = CellPlace::Element {
            name: "a".to_owned(),
            key: "k".to_owned(),
        };
        state.write(whole.clone(), NativeValueId(1));
        assert_eq!(state.read(&whole), Some(NativeValueId(1)));
        state.write(element.clone(), NativeValueId(2));
        assert_eq!(state.read(&whole), None);
        assert_eq!(state.read(&element), Some(NativeValueId(2)));
        state.write(whole.clone(), NativeValueId(3));
        assert_eq!(state.read(&element), None);
        state.forget_base("a");
        assert!(state.is_empty());
    }

    /// The braced and bare spellings are validated **differently**, and the
    /// asymmetry is load-bearing: braces are Tcl's own escape for a name the
    /// bare charset cannot express. Hoisting the braced arm through
    /// `is_bare_var_name` — the obvious "deduplication" — would change
    /// behaviour, so it is pinned here.
    #[test]
    fn whole_reference_accepts_any_non_empty_braced_name() {
        assert_eq!(whole_reference("${a-b}"), Some("a-b"));
        assert_eq!(whole_reference("${a.b}"), Some("a.b"));
        assert_eq!(whole_reference("${a(b)}"), Some("a(b)"));
        assert_eq!(whole_reference("${x}"), Some("x"));
        // …but not an empty one.
        assert_eq!(whole_reference("${}"), None);
    }

    #[test]
    fn whole_reference_charset_checks_only_the_bare_form() {
        assert_eq!(whole_reference("$x"), Some("x"));
        assert_eq!(whole_reference("$::ns::x"), Some("::ns::x"));
        assert_eq!(whole_reference("$x_1"), Some("x_1"));
        // `$item-suffix` is `$item` followed by literal text, not a whole
        // reference — so the *word* is not a simple variable load.
        assert_eq!(whole_reference("$item-suffix"), None);
        assert_eq!(whole_reference("$a(b)"), None);
        assert_eq!(whole_reference("$"), None);
    }

    #[test]
    fn whole_reference_rejects_a_word_that_is_not_a_reference() {
        assert_eq!(whole_reference("x"), None);
        assert_eq!(whole_reference(""), None);
        assert_eq!(whole_reference("[f]"), None);
        assert_eq!(whole_reference("a$b"), None);
    }

    /// One owner decides the element split, and the lexical extent — not the
    /// compatibility text — is what separates `$a(b)` from `${a(b)}`.
    #[test]
    fn variable_words_tell_element_and_odd_scalar_spellings_apart() {
        let site = |start: u32, end: u32| SourceSite::source(Span::new(start, end));
        // `$a(b)`: five source bytes for a four-byte name.
        assert_eq!(
            variable_word_place("${a(b)}", &site(0, 5)),
            Ok(CellPlace::Element {
                name: "a".to_owned(),
                key: "b".to_owned(),
            })
        );
        // `${a(b)}`: six, because the braces are part of the word.
        assert_eq!(
            variable_word_place("${a(b)}", &site(0, 6)),
            Ok(CellPlace::Named {
                name: "a(b)".to_owned(),
            })
        );
        // A name with no element suffix never consults the extent.
        assert_eq!(
            variable_word_place("$x", &site(0, 2)),
            Ok(CellPlace::Named {
                name: "x".to_owned(),
            })
        );
        // A substituted key is not a static cell: `$a($i)` is six source
        // bytes for a five-byte name, so the extent reads it as an element.
        assert_eq!(
            variable_word_place("${a($i)}", &site(0, 6)),
            Err(VariableWordDecline::Dynamic)
        );
        // Neither is a word that is not one whole reference.
        assert_eq!(
            variable_word_place("a$b", &site(0, 3)),
            Err(VariableWordDecline::Dynamic)
        );
        // A rewritten word has no lexical extent to read.
        assert_eq!(
            variable_word_place(
                "${a(b)}",
                &SourceSite {
                    span: Span::new(0, 5),
                    provenance: Provenance::Opaque,
                }
            ),
            Err(VariableWordDecline::Ambiguous)
        );
    }

    /// A braced name word is a literal, so `{a(b)}` is the element `a(b)` and
    /// `{a($k)}` its literal key — while the same text unbraced substitutes.
    #[test]
    fn name_words_split_elements_through_the_shared_rule() {
        assert_eq!(
            cell_place("a", false),
            Some(CellPlace::Named {
                name: "a".to_owned()
            })
        );
        assert_eq!(
            cell_place("a(b)", false),
            Some(CellPlace::Element {
                name: "a".to_owned(),
                key: "b".to_owned(),
            })
        );
        assert_eq!(
            cell_place("a($k)", true),
            Some(CellPlace::Element {
                name: "a".to_owned(),
                key: "$k".to_owned(),
            })
        );
        assert_eq!(cell_place("a($k)", false), None);
        assert_eq!(cell_place("", true), None);
    }

    #[test]
    fn places_spell_themselves_as_tcl_does() {
        assert_eq!(
            CellPlace::Element {
                name: "a".to_owned(),
                key: "k".to_owned()
            }
            .spelling(),
            "a(k)"
        );
        assert_eq!(CellStorage::Slot(3).as_str(), "slot");
    }
}
