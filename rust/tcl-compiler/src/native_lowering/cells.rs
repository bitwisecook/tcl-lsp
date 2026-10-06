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
//! argument's compatibility text for itself.

use std::collections::BTreeMap;

use super::ir::NativeValueId;
use crate::executable_ir::CellReference;
use crate::ir::{Provenance, SourceSite};

/// One Tcl variable cell addressed by name.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CellPlace {
    /// A whole variable name, resolved by the selected native lookup policy.
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
    /// Original spelling, source extent or provenance does not attest the read.
    Ambiguous,
}

/// True when `text` carries a substitution a static reading cannot resolve.
pub(crate) fn has_substitution(text: &str) -> bool {
    text.bytes().any(|byte| matches!(byte, b'$' | b'[' | b'\\'))
}

/// Validate the whole original reference with the selected native word grammar.
/// A compatibility spelling or trailing word component cannot supply its syntax.
fn variable_reference(
    spelling: &str,
    config: tcl_lexer::LexerConfig,
) -> Option<tcl_lexer::word_parts::RawVarRef<'_>> {
    tcl_lexer::word_parts::whole_var_ref(spelling.as_bytes(), config).ok()?
}

/// The exact name input of one whole reference, before physical native lookup.
pub(crate) fn whole_reference(spelling: &str, config: tcl_lexer::LexerConfig) -> Option<&str> {
    let reference = variable_reference(spelling, config)?;
    if reference.index.is_some() {
        spelling.get(1..)
    } else {
        std::str::from_utf8(reference.name).ok()
    }
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
/// Bare indices retain their substitution grammar; braced names use literal
/// element keys. Both literal spellings select the same symbolic element, so
/// native shadows cannot keep two incompatible aliases for that lookup.
/// This is a syntax projection, not a physical-cell or successful-read proof.
pub(crate) fn variable_word_place(
    spelling: &str,
    source: &SourceSite,
    config: tcl_lexer::LexerConfig,
) -> Result<CellPlace, VariableWordDecline> {
    let reference = variable_reference(spelling, config).ok_or(VariableWordDecline::Dynamic)?;
    if source.provenance != Provenance::Source
        || reference.source_span(spelling.as_bytes(), 0, source.span.start()) != Some(source.span)
    {
        return Err(VariableWordDecline::Ambiguous);
    }
    variable_reference_place(spelling, config)
}

/// Symbolic lookup syntax shared by original word and expression operands.
/// This accepts no physical identity, successful-read or execution proof.
pub(crate) fn variable_reference_place(
    spelling: &str,
    config: tcl_lexer::LexerConfig,
) -> Result<CellPlace, VariableWordDecline> {
    let reference = variable_reference(spelling, config).ok_or(VariableWordDecline::Dynamic)?;
    let name = std::str::from_utf8(reference.name).map_err(|_| VariableWordDecline::Dynamic)?;
    if let Some(index) = reference.index {
        let key = std::str::from_utf8(index).map_err(|_| VariableWordDecline::Dynamic)?;
        if has_substitution(key) {
            return Err(VariableWordDecline::Dynamic);
        }
        return Ok(CellPlace::Element {
            name: name.to_owned(),
            key: key.to_owned(),
        });
    }
    Ok(tcl_syntax::naming::split_element_ref(name).map_or_else(
        || CellPlace::Named {
            name: name.to_owned(),
        },
        |(base, key)| CellPlace::Element {
            name: base.to_owned(),
            key: key.to_owned(),
        },
    ))
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

    fn config() -> tcl_lexer::LexerConfig {
        tcl_lexer::LexerConfig::for_profile(
            tcl_registry::model::ingress::static_context_for("tcl9.0")
                .commands()
                .profile(),
        )
    }

    #[test]
    fn whole_reference_uses_native_braced_and_bare_grammar() {
        for (word, expected) in [
            ("${a-b}", Some("a-b")),
            ("${a.b}", Some("a.b")),
            ("${a(b)}", Some("a(b)")),
            ("${x}", Some("x")),
            ("${}", Some("")),
            ("$x", Some("x")),
            ("$::ns::x", Some("::ns::x")),
            ("$x_1", Some("x_1")),
            ("$a(b)", Some("a(b)")),
            ("$item-suffix", None),
            ("$a(b)(c)", None),
            ("$", None),
            ("x", None),
            ("", None),
            ("[f]", None),
            ("a$b", None),
        ] {
            assert_eq!(whole_reference(word, config()), expected, "{word}");
        }
    }

    #[test]
    fn variable_words_share_literal_element_identity_and_retain_index_evaluation() {
        let site = |end| SourceSite::source(tcl_lexer::Span::new(0, end));
        let element = CellPlace::Element {
            name: "a".into(),
            key: "b".into(),
        };
        assert_eq!(
            variable_word_place("$a(b)", &site(5), config()),
            Ok(element.clone())
        );
        assert_eq!(
            variable_word_place("${a(b)}", &site(6), config()),
            Ok(element)
        );
        assert_eq!(
            variable_word_place("$x", &site(2), config()),
            Ok(CellPlace::Named { name: "x".into() })
        );
        assert_eq!(
            variable_word_place("${}", &site(3), config()),
            Ok(CellPlace::Named { name: "".into() })
        );
        assert_eq!(
            variable_word_place("$a($i)", &site(6), config()),
            Err(VariableWordDecline::Dynamic)
        );
        assert_eq!(
            variable_word_place("${a($i)}", &site(7), config()),
            Ok(CellPlace::Element {
                name: "a".into(),
                key: "$i".into()
            })
        );
        assert_eq!(
            variable_word_place("a$b", &site(3), config()),
            Err(VariableWordDecline::Dynamic)
        );
        // Compatibility text cannot borrow the extent of its bare original.
        assert_eq!(
            variable_word_place("${a(b)}", &site(5), config()),
            Err(VariableWordDecline::Ambiguous)
        );
        assert_eq!(
            variable_word_place("$x", &site(3), config()),
            Err(VariableWordDecline::Ambiguous)
        );
        assert_eq!(
            variable_word_place(
                "$a(b)",
                &SourceSite::opaque(tcl_lexer::Span::new(0, 5)),
                config()
            ),
            Err(VariableWordDecline::Ambiguous)
        );
    }

    #[test]
    fn variable_reference_close_uses_the_actual_release() {
        let old = tcl_lexer::LexerConfig::for_profile(
            tcl_registry::model::ingress::static_context_for("tcl8.4")
                .commands()
                .profile(),
        );
        assert_eq!(whole_reference("${a{b}c}", old), None);
        assert_eq!(whole_reference("${a{b}c}", config()), Some("a{b}c"));
    }

    #[test]
    fn bare_and_braced_literal_reads_share_one_native_shadow() {
        let bare = variable_word_place(
            "$a(k)",
            &SourceSite::source(tcl_lexer::Span::new(0, 5)),
            config(),
        )
        .unwrap();
        let braced = variable_word_place(
            "${a(k)}",
            &SourceSite::source(tcl_lexer::Span::new(0, 6)),
            config(),
        )
        .unwrap();
        let mut state = ShadowState::default();
        state.write(bare.clone(), NativeValueId(1));
        assert_eq!(state.read(&braced), Some(NativeValueId(1)));
        state.write(braced, NativeValueId(2));
        assert_eq!(state.read(&bare), Some(NativeValueId(2)));
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
