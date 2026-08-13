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

//! The eight `SpecTcl` **hook property** statements.
//!
//! Each is written one of three ways:
//!
//! ```tcl
//! <field> {words ctx} { … }     ;# a pure Tcl body, run in the sandbox
//! <field> -native <id>          ;# a shipped implementation, by name
//! <field> <derivation-keyword>  ;# a rule the loader runs over declared data
//! ```
//!
//! The body form's two words are a parameter list and a script, so they take
//! the `ParamList` / `Body` roles — and because a walker only paints a script
//! for a **braced** word, the `-native` and derivation spellings are left
//! alone by construction, with no per-form branch anywhere.
//!
//! A hook body carries no `definition_body` grammar, which is exactly right:
//! its contents are ordinary Tcl (the sandbox whitelist), so the shared
//! walker drops back out of definition context for it, the same way it does
//! for a `method` body inside a class definition.
use crate::prelude::*;

use super::SOURCE;

/// A hook property statement.  `silence` is what the field's abstention
/// means — the one thing an author most needs and most often assumes wrongly,
/// since it is conservative *per field* rather than uniform.
fn hook_statement(
    name: &'static str,
    verbs: &'static str,
    silence: &'static str,
    summary: &'static str,
) -> CommandSpec {
    CommandSpec {
        name,
        traits: Traits::CREATES_BARRIER | Traits::NEVER_INLINE_BODY | Traits::LANGUAGE_KEYWORD,
        dialects: Some(DialectSet::SPECTCL),
        arity: Arity::new(1, 2),
        hover: Some(HoverSnippet {
            summary,
            synopsis: &[],
            snippet: verbs,
            source: SOURCE,
            examples: silence,
            return_value: "",
        }),
        arg_roles: &[(0, ArgRole::ParamList), (1, ArgRole::Body)],
        ..CommandSpec::DEFAULT
    }
}

/// The shared half of every hook's documentation: what `words` and `ctx`
/// hold, and the rule that the return value is ignored.
const CALLING_CONVENTION: &str = "`words` is the call's argument words after the command name (or after the subcommand word, for a hook on a `subcommand`) — a word whose kind is not `literal` arrives as the empty string. `ctx` is a dict always carrying `command`, `subcommand`, `nwords`, `kinds`, `tcl-version`, `dialect` and `in-event-body`. The hook's own return value is ignored: it speaks by calling its emitter verbs, and returning early is the ordinary way to abstain.";

pub(super) fn specs() -> Vec<CommandSpec> {
    vec![
        hook_statement(
            "arg_role_resolver",
            CALLING_CONVENTION,
            "emitter verb: `role IDX ROLE`. Silence means no roles — fall back to the `arg` rows. Also derivable: `from-manufacturers` (look words[0] up in this spec's own `manufacturer` rows and emit `role N Body` for its `-definition-body-at N`, bounds-checked, Body only), or implied by `clause_grammar`.",
            "Resolve argument roles for a variable-layout command.",
        ),
        hook_statement(
            "command_prefix_resolver",
            CALLING_CONVENTION,
            "emitter verb: `prefix IDX {Exactly N}`. Silence means no prefix positions.",
            "Resolve which argument positions carry a callback command prefix.",
        ),
        hook_statement(
            "const_fold",
            CALLING_CONVENTION,
            "emitter verb: `fold VALUE`. Silence means no fold. Invoked only when every word's `kinds` entry is `literal` — the precondition is the loader's, stated once, so no fold body re-checks `kinds` itself. An unversioned fold body must be version-invariant over the inputs it accepts.",
            "Fold a fully-literal call to a constant.",
        ),
        hook_statement(
            "const_fold_versioned",
            CALLING_CONVENTION,
            "emitter verb: `fold VALUE`. Silence means no fold. Same all-literal precondition as `const_fold`, with `tcl-version` in `ctx` — which is always present, so this family needs nothing else.",
            "Fold a fully-literal call to a constant, per Tcl version.",
        ),
        hook_statement(
            "taint_sink_gate",
            CALLING_CONVENTION,
            "emitter verbs: `sink-applies` / `sink-suppressed`. Silence means **the sink applies** — the one family whose abstention is not \"no opinion\", because silence must keep the security finding alive.",
            "Decide whether the command's taint sink applies to this call.",
        ),
        hook_statement(
            "context_gate",
            CALLING_CONVENTION,
            "emitter verb: `reject MESSAGE`. Silence means the call is allowed. `ctx`'s `in-event-body` is the one lexical fact this family reads.",
            "Reject the call in a context where it is not legal.",
        ),
        hook_statement(
            "literal_argument_validator",
            CALLING_CONVENTION,
            "emitter verbs: `invalid -index N -subject S -reason … -allowed {…} ?-replacement V?` and `abstain REASON`. Silence means valid. `kinds` is load-bearing here: a substituted or `{*}`-expanded word must never be mistaken for a value.",
            "Validate literal argument words and offer a fix.",
        ),
        hook_statement(
            "clause_shape_check",
            CALLING_CONVENTION,
            "emitter verbs: `missing-expr ?after?`, `missing-body after`, `extra-words first`. Silence means the shape is accepted. Ordinarily **derived** from `clause_grammar` — no hook body is needed for any chain the grammar can spell; this is the escape hatch.",
            "Validate a clause-chain shape an Arity range cannot express.",
        ),
    ]
}
