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

//! The inputs a hook body declares it reads, and whether a declaration is shape-only or content-cacheable.

use crate::invocation_words::InvocationWordKind;

/// One input a hook body declares it reads.
///
/// The DSL spelling is `-inputs {nwords kinds}` on the hook statement. An
/// undeclared hook depends on everything ([`HookInputs::unrestricted`]), which
/// is always sound and never cacheable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum HookInput {
    /// The words' *values* — the one input that makes a hook uncacheable.
    Words,
    /// `[llength $words]`.
    Nwords,
    /// The per-word `literal`/`dynamic`/`expanded`/`opaque` classification.
    Kinds,
    /// The resolved command name (fixed per slot).
    Command,
    /// The resolved subcommand word (fixed per slot).
    Subcommand,
    /// `tcl-version`.
    TclVersion,
    /// `dialect`.
    Dialect,
    /// `in-event-body`.
    InEventBody,
    /// The option-arity family's `option` / `option-index` /
    /// `option-value-start`.
    Option,
    /// The `constraints` family's structured view of the whole invocation —
    /// which options were supplied, their literal values, and the positional
    /// words after them. Content, so it keys the cache by content rather than
    /// by shape.
    Invocation,
}

/// The inputs a hook declared it reads — the cacheability rule's input.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HookInputs {
    declared: Option<Vec<HookInput>>,
}

impl HookInputs {
    /// The default: the hook declared nothing, so it may read anything.
    /// Fully legal, never cacheable — `spec-packs.md`'s "a hook that declares
    /// broader dependencies stays fully legal but uncacheable".
    #[must_use]
    pub fn unrestricted() -> Self {
        Self { declared: None }
    }

    /// The hook declared exactly these inputs.
    #[must_use]
    pub fn declared(inputs: impl IntoIterator<Item = HookInput>) -> Self {
        let mut inputs: Vec<HookInput> = inputs.into_iter().collect();
        inputs.sort_unstable();
        inputs.dedup();
        Self {
            declared: Some(inputs),
        }
    }

    /// The declared inputs, or `None` when the hook declared none.
    #[must_use]
    pub fn inputs(&self) -> Option<&[HookInput]> {
        self.declared.as_deref()
    }

    /// Parse the DSL's `-inputs {…}` word list. An unknown word makes the
    /// whole declaration unrestricted rather than dropping a dependency the
    /// hook actually has — degrading to "slower but correct" is the only safe
    /// direction for a tolerance rule that governs caching.
    #[must_use]
    pub fn parse(words: &[&str]) -> Self {
        let mut inputs = Vec::with_capacity(words.len());
        for word in words {
            let input = match *word {
                "words" => HookInput::Words,
                "nwords" => HookInput::Nwords,
                "kinds" => HookInput::Kinds,
                "command" => HookInput::Command,
                "subcommand" => HookInput::Subcommand,
                "tcl-version" => HookInput::TclVersion,
                "dialect" => HookInput::Dialect,
                "in-event-body" => HookInput::InEventBody,
                "option" | "option-index" | "option-value-start" => HookInput::Option,
                "invocation" => HookInput::Invocation,
                _ => return Self::unrestricted(),
            };
            inputs.push(input);
        }
        Self::declared(inputs)
    }

    /// Whether the hook's body is given the `words` parameter at all.
    ///
    /// **This is what makes `-inputs` a declaration rather than a hint.** The
    /// shape cache keys on the words' *shape* — count and kinds — and not
    /// their content, so a hook that reads content while claiming not to would
    /// be served another call's answer whenever the shapes matched. Binding
    /// `words` only when it was declared turns that silent miscompile into a
    /// loud "no such variable" from the sandbox on the hook's first call, which
    /// the host reports and quarantines like any other hook failure.
    ///
    /// An undeclared (unrestricted) hook still gets `words`: it made no claim,
    /// and it is never cached.
    #[must_use]
    pub fn binds_words(&self) -> bool {
        self.declared
            .as_ref()
            .is_none_or(|inputs| inputs.iter().any(|input| matches!(input, HookInput::Words)))
    }

    /// **The cacheability rule.** A hook is shape-cacheable when it declared
    /// its inputs and none of them is the words' content: everything else it
    /// may read is either fixed for the slot (`command`, `subcommand`) or part
    /// of the shape key (`nwords`, `kinds`, `tcl-version`, `in-event-body`).
    ///
    /// `option` is *not* in the key, so declaring it is uncacheable today.
    /// `dialect` is in the key — a release-pinned body runs on an engine
    /// pinned to it — but declaring it still makes a hook uncacheable;
    /// widening this rule is the change to make if a pack needs it.
    ///
    /// [`Self::binds_words`] is what keeps this honest: a hook that is
    /// shape-cacheable by this rule is not handed the words at all.
    #[must_use]
    pub fn shape_only(&self) -> bool {
        self.declared
            .as_ref()
            .is_some_and(|inputs| inputs.iter().all(|input| Self::is_shape_input(*input)))
    }

    const fn is_shape_input(input: HookInput) -> bool {
        matches!(
            input,
            HookInput::Nwords
                | HookInput::Kinds
                | HookInput::Command
                | HookInput::Subcommand
                | HookInput::TclVersion
                | HookInput::InEventBody
        )
    }

    /// **The content-cacheability rule** (E-R14, principle P-B). A hook that
    /// declared its inputs and reads the *content* of the call — the words'
    /// values, or the `constraints` family's structured invocation view — is
    /// still memoisable: the sandbox is deterministic (E-R2), so the same
    /// content on the same slot has the same answer. It is keyed by a hash of
    /// that content instead of by the shape alone.
    ///
    /// This is what makes "an edit elsewhere in the document must not re-run
    /// hooks for unrelated call sites" true: an unchanged call site hashes the
    /// same and is answered from the cache.
    ///
    /// An **undeclared** hook stays uncacheable — it made no claim about what
    /// it reads, so nothing can be hashed on its behalf.
    #[must_use]
    pub fn content_cacheable(&self) -> bool {
        self.declared.as_ref().is_some_and(|inputs| {
            inputs.iter().all(|input| {
                Self::is_shape_input(*input)
                    || matches!(input, HookInput::Words | HookInput::Invocation)
            })
        })
    }
}

/// The DSL's spelling of a word kind — the `kinds` entry a hook body reads.
///
/// One place, because the vocabulary is the registry's: a body compares
/// against `literal` and the loader documents `literal`, so the two cannot
/// drift apart into `Literal` versus `literal`.
#[must_use]
pub fn kind_word(kind: InvocationWordKind) -> &'static str {
    match kind {
        InvocationWordKind::Literal => "literal",
        InvocationWordKind::KnownBytes => "known-bytes",
        InvocationWordKind::Dynamic => "dynamic",
        InvocationWordKind::Expanded => "expanded",
        InvocationWordKind::Opaque => "opaque",
    }
}
