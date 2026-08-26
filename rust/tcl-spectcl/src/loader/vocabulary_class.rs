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

//! The §6.1 fail-closed vocabulary classes (review B13, invariant I9).
//!
//! Revision 1 of the spec-pack contract said "warn and continue" for every
//! unknown word. That is wrong in one direction: an unknown word saying
//! *this argument is code*, *this method is a sink*, or *this environment
//! is closed-world* must not be discarded while the rest of the spec
//! loads, because the older server would then publish **stronger**,
//! safer-looking answers precisely by ignoring the field it did not
//! understand.
//!
//! So an unknown word is classified by its compatibility effect:
//!
//! | Class | Effect on an unknown word | Why |
//! |---|---|---|
//! | [`Presentation`](VocabularyClass::Presentation) | warn and drop — today's behaviour | hover prose, display names, help terms: absence loses polish, never soundness |
//! | [`Assistance`](VocabularyClass::Assistance) | the spec loads, but is marked [`degraded`](super::PackCommand::degraded) | arity shapes, roles, value sets: the command stays known, and the affected capability must answer `Unknown` rather than confidently |
//! | [`Semantic`](VocabularyClass::Semantic) | the command (or the whole block) is excluded | security, control flow, binding, lowering, codegen: no taint verdict and no specialised lowering is better than a wrong one |
//!
//! ## How an *unknown* word gets a class
//!
//! A word this build has never seen cannot declare its own class, so the
//! class comes from two things it *does* have: the scope it appears in and
//! its name.
//!
//! - **Scope.** Every unknown word inside a `dialect` or `environment`
//!   block is [`Semantic`](VocabularyClass::Semantic) by construction —
//!   those blocks say what a language *is* and which world is closed, so
//!   there is no such thing as a decorative word in them.
//! - **Name.** Elsewhere, [`MARKERS`] is a closed, ordered table of
//!   substrings that mark the semantic and assistance families. It is
//!   deliberately a table rather than a heuristic: adding a word to a new
//!   family is a visible edit here, and a new word that matches nothing
//!   falls to `Presentation`, which is the only class whose absence cannot
//!   make a claim stronger.

/// What dropping an unknown word would do to the answers this build gives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum VocabularyClass {
    /// Prose and labels. Warn and drop.
    #[default]
    Presentation,
    /// Arity, roles, value sets. The spec loads degraded.
    Assistance,
    /// Security, control flow, binding, lowering, codegen, and everything
    /// inside a `dialect` or `environment` block. The spec is excluded.
    Semantic,
}

impl VocabularyClass {
    /// The class's name as a notice spells it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Presentation => "presentation",
            Self::Assistance => "assistance",
            Self::Semantic => "semantic",
        }
    }
}

impl std::fmt::Display for VocabularyClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// Name substrings that put an unknown word in a class, most specific
/// first. The first match wins, so a `taint_*` word is semantic even
/// though it also contains `arg`.
///
/// The semantic markers are the five families §6.1 names — security,
/// control flow, binding, lowering, codegen — plus the world-shape words
/// (`ambient`, `closed`, `policy`) whose absence would silently open a
/// closed world. The assistance markers are the shape-and-value families.
const MARKERS: &[(&str, VocabularyClass)] = &[
    ("taint", VocabularyClass::Semantic),
    // The 2.0 batch (P2-H): dropping a `provides`/`co_provides` opens
    // availability a provider gate was closing; dropping a
    // `dynamic_surface`/`unknown_members` closes a surface the author
    // declared open (false unknown-member diagnostics); dropping an
    // `include` silently loses declared surface. None may load past an
    // older build as decoration.
    ("provides", VocabularyClass::Semantic),
    ("surface", VocabularyClass::Semantic),
    ("members", VocabularyClass::Semantic),
    ("include", VocabularyClass::Semantic),
    ("sink", VocabularyClass::Semantic),
    ("unsafe", VocabularyClass::Semantic),
    ("safe", VocabularyClass::Semantic),
    ("credential", VocabularyClass::Semantic),
    ("sensitive", VocabularyClass::Semantic),
    ("secret", VocabularyClass::Semantic),
    ("effect", VocabularyClass::Semantic),
    ("hook", VocabularyClass::Semantic),
    ("codegen", VocabularyClass::Semantic),
    ("lowering", VocabularyClass::Semantic),
    ("intrinsic", VocabularyClass::Semantic),
    ("binding", VocabularyClass::Semantic),
    ("scope", VocabularyClass::Semantic),
    ("body", VocabularyClass::Semantic),
    ("frame", VocabularyClass::Semantic),
    ("control", VocabularyClass::Semantic),
    ("switch", VocabularyClass::Semantic),
    ("target", VocabularyClass::Semantic),
    ("ambient", VocabularyClass::Semantic),
    ("closed", VocabularyClass::Semantic),
    ("policy", VocabularyClass::Semantic),
    ("world", VocabularyClass::Semantic),
    ("arity", VocabularyClass::Assistance),
    ("role", VocabularyClass::Assistance),
    ("values", VocabularyClass::Assistance),
    ("value", VocabularyClass::Assistance),
    ("form", VocabularyClass::Assistance),
    ("constraint", VocabularyClass::Assistance),
    ("arg", VocabularyClass::Assistance),
    ("option", VocabularyClass::Assistance),
    ("type", VocabularyClass::Assistance),
    ("abbrev", VocabularyClass::Assistance),
];

/// The class of an unknown `word`.
///
/// `word` is matched case-insensitively and with a leading flag dash
/// stripped, so `-taints-var-write` and `taint_transform` classify alike.
#[must_use]
pub(super) fn classify(word: &str) -> VocabularyClass {
    let normalised = word.trim_start_matches('-').to_ascii_lowercase();
    MARKERS
        .iter()
        .find(|(marker, _)| normalised.contains(marker))
        .map_or(VocabularyClass::Presentation, |(_, class)| *class)
}

#[cfg(test)]
mod tests {
    use super::{VocabularyClass, classify};

    #[test]
    fn security_and_control_flow_words_classify_semantic() {
        for word in [
            "taint_double_encode_colour",
            "-taints-var-write",
            "world_effect",
            "codegen_hook",
            "body_scope",
            "side_switch_target",
            "unsafe_command",
            "policy",
        ] {
            assert_eq!(classify(word), VocabularyClass::Semantic, "{word}");
        }
    }

    /// The §6.1 downgrade fixture for the 2.0 batch: an older build that
    /// does not speak these words must abstain (exclude the affected
    /// spec), never publish a stronger claim by ignoring them.
    #[test]
    fn the_two_point_oh_words_classify_semantic() {
        for word in [
            "provides",
            "co_provides",
            "dynamic_surface",
            "unknown_members",
            "-dynamic-surface",
            "-unknown-members",
            "include",
        ] {
            assert_eq!(classify(word), VocabularyClass::Semantic, "{word}");
        }
    }

    #[test]
    fn shape_and_value_words_classify_assistance() {
        for word in ["arity_window", "-min-abbrev", "arg_role", "option_values"] {
            assert_eq!(classify(word), VocabularyClass::Assistance, "{word}");
        }
    }

    #[test]
    fn prose_words_classify_presentation() {
        for word in ["hover", "display_name", "detail", "synopsis", "help_terms"] {
            assert_eq!(classify(word), VocabularyClass::Presentation, "{word}");
        }
    }
}
