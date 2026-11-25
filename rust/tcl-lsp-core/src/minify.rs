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

//! Tcl code minifier.
//!
//! [`minify_with_analysis`] retains the complete current editing input. Native
//! syntax compaction retains original words and edits only separators in
//! authenticated original script regions. Expansions, escaping and variable
//! syntax retain their original bytes. Every selected region checks emitted
//! lexical correspondence independently; this makes no
//! runtime reflection or line-number equivalence claim. Native naming,
//! insertion, semantic and keyword passes require separate contracts and report
//! typed refusals while those contracts are unavailable.
//!
//! The following transformations belong to explicit lexical authoring advice.
//! Its **default tier** ([`minify_tcl`]) performs:
//!
//! 1. Stripping all comments.
//! 2. Collapsing inter-command whitespace to `;`.
//! 3. Collapsing intra-command whitespace to single spaces.
//! 4. Recursively minifying braced body arguments (and `[…]`
//!    command substitutions).
//! 5. Preserving string literals verbatim, dropping redundant
//!    double quotes when safe.
//! 6. Compressing whitespace inside `expr` bodies and applying
//!    AST-level shrinking (comparison inversion, De Morgan,
//!    double-negation) when it shortens the expression.
//! 7. Replacing `${var}` with `$var` when safe.
//! 8. Minifying the braced clause-list argument of registry
//!    `case_list` commands (`switch`, Expect's `expect`) with the
//!    Tcl **list** grammar — a braced case list is a list, not a
//!    script, so `#` is an ordinary pattern there, never a comment.
//!
//! Syntax compaction never introduces variables, writes, or any other
//! observable behaviour — its output is frame-transparent.  That rules
//! out `[subst $alias]` template deduplication, whose `set alias {…}`
//! preamble can clobber a live variable, fire traces, and change
//! `info vars`.
//!
//! Note: the expression tokeniser adds a catch-all so no character
//! is dropped — naively dropping unmatched characters (e.g. commas
//! in `atan2($a,$b)` and braces in `$x ni {a b}`) would corrupt
//! those expressions.
//!
//! The **`compact_names` tier** ([`minify_tcl_compact`]) renames
//! proc-local variables and parameters to short identifiers and
//! returns a [`SymbolMap`].  It relies on the analyser tracking
//! `$var` references inside `[…]` command substitutions and braced
//! `expr` bodies so a rename never rewrites a declaration without
//! its body references.  Renaming is fenced by the observability facts
//! of the document's command surface — the catalogue's, and the ones a
//! `# tcl-lsp: stub` declaration's flags state:
//!
//! * Scopes containing a dynamic-barrier command (`upvar`, `eval`,
//!   `trace`, a `-barrier` stub, … —
//!   [`Traits::CREATES_DYNAMIC_BARRIER`]) or a
//!   variable-name introspection ([`Traits::INTROSPECTS_BY_NAME`],
//!   e.g. `info locals` / `info vars` / `info exists`) are left
//!   untouched.
//! * A caller-frame command (`upvar` —
//!   [`Traits::ALIASES_CALLER_FRAME`], `uplevel` —
//!   [`Traits::EVALUATES_IN_SHIFTED_FRAME`]) anywhere blocks
//!   variable renaming in **every** scope: the observed frame is
//!   statically unknowable.
//! * **Procedure names are public identities** — `info procs`,
//!   `rename`, `namespace export`, external callers, and `unknown`
//!   can all observe or invoke them — so procs are renamed only
//!   under `isolated` (the caller asserts a self-contained,
//!   closed-world script), and even then only when no
//!   command-name-reflecting command
//!   ([`Traits::REFLECTS_COMMAND_NAMES`]) and no computed command
//!   name is present.
//! * Array member keys (`arr(member)`) are Tcl **data**, never
//!   compacted: `array get` / `array names` / serialisation observe
//!   them.
//!
//! `isolated` also compacts global-scope variables.
//!
//! The **`aggressive` tier** ([`minify_tcl_aggressive`]) applies the
//! compiler's optimiser rewrites, compacts names, aliases repeated
//! commands / arguments / quoted-string substrings (the last via a
//! suffix array), then minifies whitespace, returning a
//! [`MinifyResult`].  **Aggressive output is deliberately not
//! frame-transparent**: the aliasing phases inject `set alias …`
//! preambles, which create real Tcl variables (visible to `info
//! vars`, traces, and any pre-existing same-named variable).  The
//! alias generators avoid every name the compiler can see — the
//! compacted shorts, every variable name in any analysed scope or
//! SSA table, and every textual `$name` reference — but a name that
//! only exists in the *hosting interpreter* (a variable set before
//! the minified script is sourced) cannot be proven absent; use the
//! default or compact tier where frame transparency is required.
//!
//! [`unminify_error`] translates compacted names in an error
//! message back to the originals via a [`SymbolMap`] (round-tripped
//! through [`SymbolMap::format`] / [`SymbolMap::parse`]).  Both the
//! `tcl-lsp.minifyDocument` and `tcl-lsp.unminifyError`
//! `workspace/executeCommand` handlers are wired in the server.
//!
//! SCCP static-substring folding ([`fold_static_substrings`]) runs
//! between the optimiser rewrites and name compaction, and replaces
//! `$var` interpolations inside
//! quoted strings with the literal the compiler's SCCP pass proves
//! them to be (integer / string constants), taint-guarded.
//! `unminify_error` also remaps minified line references back to
//! original lines ([`remap_line_references`]) when both sources are
//! supplied.
//!
//! Static-substring folding does not do compile-time evaluation of
//! pure command substitutions (`[string …]` / `[format …]` /
//! `[expr …]`), boolean / float constants, or dead-`set`
//! elimination (leaving the now-unused `set` is harmless, just
//! larger).

#[cfg(test)]
use std::collections::{BTreeMap, HashMap, HashSet};

mod lexical_keywords;
mod lexical_metadata;
mod original;
pub use original::{MinifyOptions, MinifyRefusal, MinifyTier, minify_with_analysis};

use tcl_compiler::analyser::AnalysisResult;
#[cfg(test)]
use tcl_compiler::analyser::ScopeKind;
#[cfg(test)]
use tcl_compiler::ir::Statement;
#[cfg(test)]
use tcl_compiler::ssa::Version;
#[cfg(test)]
use tcl_compiler::taint::TaintLattice;
#[cfg(test)]
use tcl_lexer::TokenType;
use tcl_lexer::{Lexer, LexerConfig, SourceEdit, SourceMap, Span, Token, apply_edits};
use tcl_registry::CommandRegistry;
#[cfg(test)]
use tcl_registry::Traits;

mod aliasing;
mod case_lists;
mod command_rendering;
mod expressions;
mod name_compaction;
mod static_folding;
mod symbol_map;
mod unminify;

#[cfg(test)]
use aliasing::{
    AliasWordPurpose, LiveNameInventoryGap, StringLiteralFragment, alias_by_uses,
    alias_string_reference, authored_alias_spelling, collect_string_literals, source_alias_uses,
};
use aliasing::{
    abbreviate_keywords, alias_repeated_arguments, alias_repeated_commands, alias_string_literals,
    collect_live_names, collect_symbol_shorts,
};
#[cfg(test)]
use command_rendering::{
    can_strip_quotes, lexical_head_value, lexical_script_words, lexical_source_schema,
    reconstruct_variable_reference, strip_braced_var_refs,
};
use command_rendering::{parse_commands, render_command};
#[cfg(test)]
use expressions::{ExprTok, expression_token_signature, strip_expr_whitespace, tokenise_expr};
use expressions::{shrink_expr_ast, strip_expr_whitespace_with_script_compaction};
use name_compaction::compact_names;
#[cfg(test)]
use name_compaction::find_rename_barriers;
#[cfg(test)]
use name_compaction::{
    RenameBarriers, apply_rename_traits, compact_logical_formals, current_proc_call_sites,
    find_proc_call_sites, record_variable_report, rename_params_in_list, source_scope_at_offset,
    variable_reference_name_spans,
};
use static_folding::fold_original_static_substrings;
#[cfg(test)]
use static_folding::{fold_static_substrings, parse_var_ref};
#[cfg(test)]
use symbol_map::SYMBOL_MAP_RECORD;
pub use symbol_map::{ScopedSymbolMapLabel, SymbolMap, SymbolMapReportError};
pub use unminify::{remap_line_references, unminify_error};

/// Depth cap for [`minify_body`]'s recursion over nested control-flow
/// bodies, `[…]` command substitutions, and `expr` bodies.
/// Threaded through every function on the path back to `minify_body`
/// (`render_command`, `reconstruct_arg`/`reconstruct_raw`,
/// `minify_switch_case_list`, `minify_lambda_literal`,
/// `compress_expr`/`tokenise_expr`).
///
/// Same reasoning and value as
/// [`crate::formatting::engine::MAX_FORMAT_DEPTH`] (this crate is also
/// reachable from a WASM host with no stack-size guarantee, via
/// `bigip-query-wasm`) — see that constant's doc comment.
const MAX_MINIFY_DEPTH: tcl_core_types::RecursionLimit = tcl_core_types::RecursionLimit(128);

/// One argument accumulated while parsing a command.
struct Arg {
    tokens: Vec<Token>,
    is_braced: bool,
    is_quoted: bool,
}

/// Result and requested-pass refusals from minification.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MinifyResult {
    /// The minified source.
    pub source: String,
    /// Compaction symbol map.
    pub symbol_map: SymbolMap,
    /// Number of retained naming or optimiser rewrites applied.
    pub optimisations_applied: usize,
    /// Length of the original source (bytes).
    pub original_length: usize,
    /// Requested transformations whose independent contracts are unavailable.
    /// Syntax compaction can succeed while a naming or insertion pass refuses.
    pub refusals: Vec<MinifyRefusal>,
}

impl MinifyResult {
    /// Length of the minified source.
    #[must_use]
    pub fn minified_length(&self) -> usize {
        self.source.len()
    }

    /// Percentage size reduction versus the original.
    #[must_use]
    pub fn savings_pct(&self) -> f64 {
        if self.original_length == 0 {
            return 0.0;
        }
        let min = f64::from(u32::try_from(self.source.len()).unwrap_or(u32::MAX));
        let orig = f64::from(u32::try_from(self.original_length).unwrap_or(u32::MAX));
        (1.0 - min / orig) * 100.0
    }
}

/// Minify under explicit driver profile/store inputs. Native output changes
/// only lexical separators and comments, keeping whole original words.
/// Use [`minify_with_profile`] or [`minify_with_analysis`] to retain refusals.
#[must_use]
pub fn minify_tcl(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    registry: &CommandRegistry,
) -> String {
    minify_with_profile(source, dialect, registry, MinifyOptions::default()).source
}

/// Request aggressive minification under explicit driver profile/store inputs.
/// Native analyses retain whole original words and report unavailable naming,
/// semantic, keyword and alias-insertion contracts in [`MinifyResult::refusals`].
/// Only explicit lexical authoring advice uses the compatibility passes, whose
/// inserted assignments are observable and are not frame-transparent.
#[must_use]
pub fn minify_tcl_aggressive(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    isolated: bool,
    registry: &CommandRegistry,
) -> MinifyResult {
    minify_tcl_aggressive_with(source, dialect, isolated, registry, true)
}

/// [`minify_tcl_aggressive`] with the keyword-abbreviation phase
/// switchable.
///
/// `abbreviations = false` is the CLI's `--no-abbreviations`: abbreviated
/// output is correct but harder to eyeball-diff, so the emitter can be turned
/// off without giving up the rest of the tier.
#[must_use]
pub fn minify_tcl_aggressive_with(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    isolated: bool,
    registry: &CommandRegistry,
    abbreviations: bool,
) -> MinifyResult {
    minify_with_profile(
        source,
        dialect,
        registry,
        MinifyOptions {
            tier: MinifyTier::Aggressive,
            isolated,
            abbreviations,
        },
    )
}

/// Aggressive authoring transformations, reached only after the retained
/// analysis explicitly selects lexical advice and the complete configuration
/// and availability generation match these compatibility passes. Native
/// planning never enters this path.
fn lexical_aggressive(
    source: &str,
    input: &tcl_compiler::analyser::ResolvedAnalysisInput,
    isolated: bool,
    abbreviations: bool,
) -> Result<MinifyResult, MinifyRefusal> {
    let dialect = input.unit_profile();
    let context = input.context_registry();
    let registry = context.commands().as_ref();
    let original_length = source.len();

    // Apply the optimiser's semantic-preserving rewrites.
    let profile = dialect;
    let optimisations =
        tcl_compiler::optimiser::optimise_with_dialect(source, registry, Some(profile));
    let opt_count = optimisations.iter().filter(|o| !o.hint_only).count();
    let mut opt_edits: Vec<_> = optimisations
        .iter()
        .filter(|o| !o.hint_only)
        .map(|o| SourceEdit {
            span: o.span.as_range(),
            text: o.replacement.clone(),
        })
        .collect();
    opt_edits.sort_by_key(|edit| (edit.span.start, edit.span.end));
    opt_edits.dedup();
    let optimised =
        apply_edits(source, &opt_edits).map_err(|_| MinifyRefusal::SourceCorrespondenceChanged)?;

    // Fold static substrings (SCCP-proven constants).
    let (folded, fold_count, static_folds) = fold_original_static_substrings(&optimised, input);

    // Compact names.
    let (renamed, mut symbol_map) = compact_names(&folded, input, isolated);
    symbol_map.static_folds = static_folds;

    // Keep observed collision candidates separate from the source/body gaps
    // that remain unresolved. This preserves the existing aggressive
    // authoring policy; it is not a completeness or absence certificate.
    let live_names = collect_live_names(&renamed, input);
    let _unresolved_collision_debt = &live_names.gaps;
    let mut claimed_names = collect_symbol_shorts(&symbol_map);
    claimed_names.extend(live_names.names.iter().cloned());
    let (renamed, cmd_aliases) = alias_repeated_commands(&renamed, input, &mut claimed_names);
    symbol_map.command_aliases = cmd_aliases;
    let (renamed, arg_aliases) = alias_repeated_arguments(&renamed, input, &mut claimed_names);
    symbol_map.argument_aliases = arg_aliases;
    let (renamed, str_aliases) = alias_string_literals(&renamed, input, &mut claimed_names);
    symbol_map.string_aliases = str_aliases;

    // Emit unique-prefix keyword abbreviations.
    let (renamed, abbrev_count) = if abbreviations {
        abbreviate_keywords(&renamed, input)
    } else {
        (renamed, 0)
    };

    // Rewritten source retains the same full editing input before any further
    // script/body selection; no reporting label rebuilds its context.
    let current = original::analysis_with_input(&renamed, input);
    let minified = lexical_default(&renamed, &current)?;

    Ok(MinifyResult {
        source: minified,
        symbol_map,
        optimisations_applied: opt_count + fold_count + abbrev_count,
        original_length,
        refusals: Vec::new(),
    })
}

/// Request name compaction under explicit driver profile/store inputs.
/// Native naming passes require separate complete-coverage/observer contracts;
/// until available this returns syntax compaction and an empty symbol map.
/// Use [`minify_with_profile`] or [`minify_with_analysis`] to retain the refusal.
/// Explicit lexical advice preserves the compatibility rename pass.
#[must_use]
pub fn minify_tcl_compact(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    isolated: bool,
    registry: &CommandRegistry,
) -> (String, SymbolMap) {
    let result = minify_with_profile(
        source,
        dialect,
        registry,
        MinifyOptions {
            tier: MinifyTier::Compact,
            isolated,
            ..MinifyOptions::default()
        },
    );
    (result.source, result.symbol_map)
}

fn lexical_compact(
    source: &str,
    input: &tcl_compiler::analyser::ResolvedAnalysisInput,
    isolated: bool,
) -> Result<(String, SymbolMap), MinifyRefusal> {
    // Implementation contract: naming.minifier.lexical-authoring-passes
    // docs/design/analysis/name-resolution-proofs/minifier-lexical-authoring-passes.md
    let (renamed, symbol_map) = compact_names(source, input, isolated);
    let current = original::analysis_with_input(&renamed, input);
    let minified = lexical_default(&renamed, &current)?;
    Ok((minified, symbol_map))
}

/// Analyse an explicitly supplied profile and command store once, then use the
/// retained-analysis planner. Custom editor inputs should call
/// [`minify_with_analysis`] directly. The display label is not re-resolved.
#[must_use]
pub fn minify_with_profile(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    registry: &CommandRegistry,
    options: MinifyOptions,
) -> MinifyResult {
    let analysis = original::analysis_for_profile(source, dialect, registry);
    minify_with_analysis(source, &analysis, options)
}

fn lexical_default(source: &str, analysis: &AnalysisResult) -> Result<String, MinifyRefusal> {
    // naming.minifier.retained-lexical-context
    // docs/design/analysis/name-resolution-proofs/minifier-retained-lexical-context.md
    let input = analysis
        .resolved_input
        .as_ref()
        .ok_or(MinifyRefusal::MissingAnalysisContext)?;
    let context = input.context_registry();
    let identities = analysis
        .retained_command_realm()
        .ok_or(MinifyRefusal::MissingAnalysisContext)?;
    Ok(minify_body(
        source,
        MinifyEnv {
            original_source: source,
            analysis,
            input,
            dialect: input.unit_profile(),
            config: input.lexer_config(),
            registry: context.commands(),
            context: &context,
            identities,
        },
        0,
        tcl_lexer::SourceSegmentationPurpose::WholeInput,
    ))
}

/// The document-wide context every step of the minify recursion carries
/// unchanged.
///
/// Bundled rather than passed as separate parameters because the recursion is
/// deep (`minify_body` → `render_command` → `reconstruct_raw` → `minify_body`)
/// and one of its steps was already at the argument limit.
#[derive(Clone, Copy)]
struct MinifyEnv<'a> {
    /// Complete unchanged source backing every borrowed rendering region.
    original_source: &'a str,
    /// Actual analysis owning this complete source, input and Realm.
    analysis: &'a AnalysisResult,
    /// Same immutable complete driver input sealed by the retained Realm.
    input: &'a tcl_compiler::analyser::ResolvedAnalysisInput,
    /// The document's dialect, for the lexer and the abbreviation tables.
    dialect: &'static tcl_dialect::DialectProfile,
    /// Complete actual body grammar, already checked for compatibility support.
    config: LexerConfig,
    /// Actual full availability generation; no profile-default re-selection.
    context: &'a tcl_registry::model::ContextRegistry,
    /// The registry the argument roles and clause-list shapes come from.
    registry: &'a CommandRegistry,
    /// The document's statically proven command-identity facts
    /// ([`tcl_compiler::realm`]), so a body / lambda / expression /
    /// clause-list argument is recognised by the command a head *is* rather
    /// than the one it is spelled as.
    ///
    /// Read *unpositioned*: this recursion re-minifies each nested body from
    /// its own slice, segmented at offset 0, so no document-absolute offset
    /// exists at the point of the query.
    identities: &'a tcl_compiler::realm::CommandBindingRealm,
}

impl MinifyEnv<'_> {
    /// Only an actual subslice of the complete held input retains coordinates.
    /// Decoded or generated text cannot recapture a source-body occurrence.
    fn original_region(self, source: &str) -> Option<Span> {
        let start =
            (source.as_ptr() as usize).checked_sub(self.original_source.as_ptr() as usize)?;
        let end = start.checked_add(source.len())?;
        (self.original_source.get(start..end)? == source).then_some(())?;
        Some(Span::new(
            u32::try_from(start).ok()?,
            u32::try_from(end).ok()?,
        ))
    }
}

/// Minify a Tcl script body (top-level or inside braces). `depth` is this
/// body's nesting level — see [`MAX_MINIFY_DEPTH`].
fn minify_body(
    source: &str,
    env: MinifyEnv<'_>,
    depth: u32,
    purpose: tcl_lexer::SourceSegmentationPurpose,
) -> String {
    let local_config = purpose.config(env.config);
    let dialect = env.dialect;
    // Native-stack safety net — see `MAX_MINIFY_DEPTH`'s doc comment.
    // Past the cap, leave this (deeply nested) body unminified rather than
    // recursing further, matching the give-up-gracefully fallback just below
    // for an unparseable body.
    if MAX_MINIFY_DEPTH.exceeded(depth) {
        return source.to_owned();
    }
    let sm = SourceMap::new(source);
    let Ok(tokens) = Lexer::with_config(source, local_config).tokenise_all() else {
        return source.to_owned();
    };

    let commands = parse_commands(source, &tokens, local_config);
    let original_words = if let Some(region) = env.original_region(source) {
        let capture = || -> Option<tcl_lexer::NativeScriptWordsPlan> {
            let image = env.identities.original_source_image()?.clone();
            if image.bytes() != env.original_source.as_bytes()
                || env.config != env.input.lexer_config()
                || !env.identities.matches_resolved_analysis_input(env.input)
                || !env
                    .identities
                    .matches_original_source_image(&image, env.config)
            {
                return None;
            }
            let plan =
                tcl_lexer::native_script_words_in_with_purpose(image, region, env.config, purpose)
                    .ok()?;
            (plan.fatal_tail.is_none() && plan.commands.len() == commands.len()).then_some(plan)
        };
        let Some(plan) = capture() else {
            return source.to_owned();
        };
        Some(plan)
    } else {
        // Deliberately decoded/generated fields have no original coordinates.
        None
    };
    if commands.is_empty() {
        return String::new();
    }

    let mut rendered: Vec<Vec<String>> = Vec::with_capacity(commands.len());
    for (index, cmd_args) in commands.iter().enumerate() {
        let original = original_words
            .as_ref()
            .and_then(|plan| plan.commands.get(index));
        let Some(command) = render_command(&sm, cmd_args, original, env, depth) else {
            return source.to_owned();
        };
        rendered.push(command);
    }

    // NB: no template deduplication here.  The former `[subst $alias]`
    // rewrite injected a `set alias {…}` preamble — a real Tcl variable
    // write that could clobber a live variable read through a
    // name-taking command (`puts [set a]`), fire traces, and change
    // `info vars` — so it is banned from this semantics-preserving tier.
    // Aggressive aliasing (an explicitly
    // behaviour-changing tier) covers the same compression ground.
    let is_irules = dialect.is_irules();
    let mut parts: Vec<String> = Vec::new();
    for arg_strs in &rendered {
        if is_irules && arg_strs.len() > 1 {
            // In iRules, `}{` is a valid word boundary — omit the
            // space between adjacent braced args to save bytes.
            let mut piece = arg_strs[0].clone();
            for w in arg_strs.windows(2) {
                let (prev, cur) = (&w[0], &w[1]);
                if prev.ends_with('}') && cur.starts_with('{') {
                    piece.push_str(cur);
                } else {
                    piece.push(' ');
                    piece.push_str(cur);
                }
            }
            parts.push(piece);
        } else {
            parts.push(arg_strs.join(" "));
        }
    }
    parts.join(";")
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod constructed_compatibility_tests;

#[cfg(test)]
mod selected_scalar_reference_emission_tests;

#[cfg(test)]
mod symbol_report_tests;

#[cfg(test)]
mod original_formal_name_geometry_tests;

#[cfg(test)]
mod authored_compact_candidate_tests;

#[cfg(test)]
mod original_alias_phase_tests;
