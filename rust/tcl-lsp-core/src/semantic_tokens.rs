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

//! Semantic-tokens provider.
//!
//! Produces an LSP-encoded semantic-tokens stream covering
//! the common Tcl token categories:
//!
//! * **Keyword** — command heads carrying the registry's
//!   `LANGUAGE_KEYWORD` trait (`if`, `while`, `for`, `foreach`,
//!   `switch`, `return`, `break`, `continue`, `try`, `catch`,
//!   `proc`, `namespace`, `when`, `oo::*`, …) plus the non-command
//!   clause / `TclOO` sub-keywords (`else`, `elseif`, `method`,
//!   `constructor`, …).
//! * **Function** — every other command-head token (user
//!   procs + built-in commands).
//! * **Variable** — `$name` / `${name}` substitutions.
//! * **String** — braced literals (`{...}`) and double-quoted
//!   strings.
//! * **Number** — integer / float literals.
//! * **Comment** — `# ...` comment lines.
//! * **Namespace** — namespace-qualified names containing
//!   `::`.
//! * **Regexp** — the regex-pattern argument of `regexp` / `regsub`
//!   (registry `pattern_type == Regex`, option-skipped positional),
//!   sub-tokenised into ARE components (`RegexpGroup` /
//!   `RegexpCharClass` / `RegexpQuantifier` / `RegexpAnchor` /
//!   `RegexpEscape` / `RegexpBackref` / `RegexpAlternation`).
//! * **Event** — an iRules `when EVENT` event name.
//! * **Format** — `format` / `scan` conversion strings
//!   (`FormatPercent` / `FormatFlag` / `FormatWidth` / `FormatSpec`),
//!   `clock format` / `scan` field strings (`ClockPercent` /
//!   `ClockSpec` / `ClockModifier`), `binary format` / `scan`
//!   field strings (`BinarySpec` / `BinaryCount` / `BinaryFlag`), and
//!   `regsub` replacement backrefs (`\1` → `Number`, `\&` → `Operator`).
//! * **Object** — BIG-IP object names (pools, data groups, virtuals,
//!   nodes, …) referenced from iRules code, under the `f5-irules`
//!   dialect (see [`crate::irules_object_refs`]).
//!
//! The legend is exposed via [`legend_token_types`] and
//! [`legend_token_modifiers`] so the server advertises it in
//! the LSP `initialize` capabilities response.
//!
//! Additional variants:
//!
//! * Range variant ([`range`]) — same encoding as [`full`]
//!   filtered to tokens whose start position falls inside
//!   the request range.  Server advertises `range: true`.
//! * Delta variant — when the client's `previousResultId`
//!   matches the per-URI cached stream, the server returns the
//!   minimal token-aligned edit computed by [`diff`] (an empty
//!   edit list when nothing changed); a stale / unknown previous
//!   id falls back to a fresh full stream.
//!
//! Two **document-mode** grammars are handled here too, because neither is Tcl
//! and running the Tcl tokeniser over them mis-colours the file rather than
//! merely under-colouring it (each braced block reads as one literal word, so
//! whole *lines* come out as `string`):
//!
//! * [`bigip_conf_full`] — BIG-IP config (`bigip.conf` / `.scf`): partition
//!   paths (`/Common/…`), IPv4 / route-domain / port literals, and the object
//!   taxonomy.  A `ltm rule { … }` stanza's body is iRules code, so it is
//!   re-walked as Tcl.
//! * [`apl_full`] — APL (iApp presentation).  Its `[ … ]` bracket expressions
//!   are embedded Tcl and are likewise re-walked as Tcl.
//!
//! Both lexers live in `tcl-bigip` and are non-overlapping by construction; the
//! iRules object-reference overlay (the code-relevant half of the BIG-IP
//! taxonomy) applies inside embedded rule bodies exactly as it does in a
//! standalone `.irul`.

use rustc_hash::{FxHashMap, FxHashSet};
use tcl_compiler::analyser::{AnalysisResult, ClassHierarchy};
use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_lexer::{LineIndex, Span};

use tcl_registry::CommandRegistry;

mod original;
mod receiver_source;

mod argument_override;
mod argument_tokens;
mod command_roles;
mod command_tokens;
mod config_documents;
mod delta;
mod emission;
mod format_subtokens;
mod irules_document;
mod legend;
mod object_dispatch;
mod object_handles;
mod regex_subtokens;
mod script_walk;
mod variable_name_roles;

use argument_override::{ArgOverride, subspec_content};
#[cfg(test)]
use argument_tokens::keyword_operand_content;
use command_tokens::classify_and_push_if;
pub use config_documents::{apl_full, apl_range, bigip_conf_full, bigip_conf_range};
pub use delta::{TokenEdit, diff};
use emission::{
    Entry, encode_entries, end_over_terminator, push_comment_tokens, push_entered_comment_tokens,
    push_object_token, push_span_entries, push_token,
};
use irules_document::{
    irules_document_object_spans, irules_top_level_declaration_heads, is_event_name,
};
#[cfg(test)]
use legend::ALL_TOKEN_KINDS;
use legend::{MOD_DECLARATION, MOD_DEFAULT_LIBRARY, TokenKind};
pub use legend::{legend_custom_token_types, legend_token_modifiers, legend_token_types};
pub use object_dispatch::{NamedInstanceMap, WorkspaceTokenFacts};
use object_dispatch::{
    ObjectClassMap, insert_user_configure_options, mark_method_word, named_instances_from_analysis,
    retag_original_variable_argument,
};
use object_handles::{augment_loop_var_handles, augment_snit_handles};
#[cfg(test)]
use object_handles::{selected_member_handle_regions, static_loop_var_names};
use regex_subtokens::mark_literal_fragments;
#[cfg(test)]
use regex_subtokens::{classify_regex_component, scan_are_class};
use script_walk::{MAX_TOKEN_RECURSION, ScriptCtx, WalkFacts, collect_script};
pub use variable_name_roles::VarNameArgRoles;

/// Encoded semantic-tokens response.  The `data` array is
/// the LSP packed integer encoding (5 ints per token: line
/// delta, column delta, length, type, modifiers).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SemanticTokens {
    /// Packed integer data.
    pub data: Vec<u32>,
}

/// Compute semantic tokens for the entire document.
#[must_use]
pub fn full(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    registry: &CommandRegistry,
) -> SemanticTokens {
    full_with_cu(source, dialect, registry, None)
}

/// First-paint semantic tokens from the written words and the registry alone.
///
/// No command is interpreted and no body is analysed, so the cost is one
/// pass over the text whatever the file's size. Command heads, subcommands,
/// options, bodies, expressions, parameters, variables and comments are
/// coloured as [`full`] colours them; what needs the analysis (user classes,
/// object dispatch, a rebound builtin, regex sources) arrives with the
/// analysed result.
#[must_use]
pub fn coarse(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    registry: &CommandRegistry,
) -> SemanticTokens {
    let shell = standalone_analyser(dialect, registry)
        .source_shell()
        .analyse(source, dialect.name);
    full_with_cu_and_analysis(source, dialect, registry, None, Some(&shell))
}

/// [`coarse`] filtered to the tokens that start inside `range`.
#[must_use]
pub fn coarse_range(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    range: crate::definition::LspRange,
    registry: &CommandRegistry,
) -> SemanticTokens {
    let shell = standalone_analyser(dialect, registry)
        .source_shell()
        .analyse(source, dialect.name);
    range_with_cu_and_analysis(source, dialect, range, registry, None, Some(&shell))
}

/// Compute semantic tokens with an optional [`CompilationUnit`] for the same
/// document.
///
/// When `cu` is `Some`, a `regexp`/`regsub` pattern supplied through a
/// provably-constant string variable (`set my_re ".*abc"; regexp $my_re $s`)
/// causes the *originating* `set` literal to be highlighted as a regex — see
/// [`tcl_compiler::regex_source`].  With `cu == None` the result is identical
/// to the pure-segmentation tokenisation (the feature is simply absent), so
/// callers without an analysis pay nothing.
#[must_use]
pub fn full_with_cu(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    registry: &CommandRegistry,
    cu: Option<&CompilationUnit>,
) -> SemanticTokens {
    full_with_cu_and_analysis(source, dialect, registry, cu, None)
}

/// [`full_with_cu`] with an optional [`AnalysisResult`], enabling precise
/// `$obj method …` / `[dict get $objs $k] method …` highlighting against
/// *user-defined* classes (their methods and `oo::configurable` properties),
/// resolved through the analyser's class hierarchy.  `None` falls back to the
/// registry-only object-method path.
#[must_use]
pub fn full_with_cu_and_analysis(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    registry: &CommandRegistry,
    cu: Option<&CompilationUnit>,
    analysis: Option<&AnalysisResult>,
) -> SemanticTokens {
    let proc_roles = analysis.map(VarNameArgRoles::from_analysis);
    let named_instances = analysis.map(named_instances_from_analysis);
    full_with_cu_and_facts(
        source,
        dialect,
        registry,
        cu,
        WorkspaceTokenFacts {
            classes: analysis.map(AnalysisResult::class_hierarchy),
            proc_roles: proc_roles.as_ref(),
            named_instances: named_instances.as_ref(),
            project_classes: None,
            analysis,
        },
    )
}

/// [`full_with_cu`] with an optional [`ClassHierarchy`] — the workspace-merged
/// project class index, so a `$obj method …` dispatch resolves against a class
/// defined in *another* file.  The salsa / server path uses this; the local
/// single-file path goes through [`full_with_cu_and_analysis`].
#[must_use]
pub fn full_with_cu_and_classes(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    registry: &CommandRegistry,
    cu: Option<&CompilationUnit>,
    classes: Option<&ClassHierarchy>,
) -> SemanticTokens {
    full_with_cu_and_classes_and_roles(source, dialect, registry, cu, classes, None)
}

/// [`full_with_cu_and_classes`] with the workspace-merged inferred variable-name
/// argument roles ([`VarNameArgRoles`]), so a `myproc arr(key) …` call whose
/// `myproc` parameter aliases a caller variable highlights its array-element
/// target like `set arr(key) …`.  The project path
/// passes a cross-file index; `None` restricts the retag to the static registry
/// roles (plus source-derived stub roles).
#[must_use]
pub fn full_with_cu_and_classes_and_roles(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    registry: &CommandRegistry,
    cu: Option<&CompilationUnit>,
    classes: Option<&ClassHierarchy>,
    proc_roles: Option<&VarNameArgRoles>,
) -> SemanticTokens {
    full_with_cu_and_facts(
        source,
        dialect,
        registry,
        cu,
        WorkspaceTokenFacts {
            classes,
            proc_roles,
            named_instances: None,
            project_classes: None,
            analysis: None,
        },
    )
}

/// [`full_with_cu_and_classes_and_roles`] with the workspace-merged named
/// bareword instance-command index bundled in, so `CLASS
/// create NAME` resolves its class exactly like the single-file
/// [`full_with_cu_and_analysis`] path does — the project token-aggregation
/// path (`semantic_tokens_project`) is the one caller with a project-wide
/// [`NamedInstanceMap`] to hand; `WorkspaceTokenFacts::default()` (every
/// other caller) skips the merge.
#[must_use]
pub fn full_with_cu_and_facts(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    registry: &CommandRegistry,
    cu: Option<&CompilationUnit>,
    facts: WorkspaceTokenFacts<'_>,
) -> SemanticTokens {
    let entries = collect_entries(source, dialect, registry, cu, facts);
    encode_entries(&entries)
}

/// Compute semantic tokens for `range` within the document.
/// Tokens whose start position falls outside the range are
/// dropped.  Delta encoding starts from the first surviving
/// token rather than the document origin, matching the LSP
/// spec for `semanticTokens/range`.
#[must_use]
pub fn range(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    range: crate::definition::LspRange,
    registry: &CommandRegistry,
) -> SemanticTokens {
    range_with_cu(source, dialect, range, registry, None)
}

/// [`range`] with an optional [`CompilationUnit`] enabling regex-source
/// highlighting (see [`full_with_cu`]).
#[must_use]
pub fn range_with_cu(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    range: crate::definition::LspRange,
    registry: &CommandRegistry,
    cu: Option<&CompilationUnit>,
) -> SemanticTokens {
    range_with_cu_and_analysis(source, dialect, range, registry, cu, None)
}

/// [`range_with_cu`] with an optional [`AnalysisResult`] for user-class
/// object-method highlighting (see [`full_with_cu_and_analysis`]).
#[must_use]
pub fn range_with_cu_and_analysis(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    range: crate::definition::LspRange,
    registry: &CommandRegistry,
    cu: Option<&CompilationUnit>,
    analysis: Option<&AnalysisResult>,
) -> SemanticTokens {
    let proc_roles = analysis.map(VarNameArgRoles::from_analysis);
    let named_instances = analysis.map(named_instances_from_analysis);
    range_with_cu_and_facts(
        source,
        dialect,
        range,
        registry,
        cu,
        WorkspaceTokenFacts {
            classes: analysis.map(AnalysisResult::class_hierarchy),
            proc_roles: proc_roles.as_ref(),
            named_instances: named_instances.as_ref(),
            project_classes: None,
            analysis,
        },
    )
}

/// [`range_with_cu`] with an optional workspace-merged [`ClassHierarchy`] (see
/// [`full_with_cu_and_classes`]).
#[must_use]
pub fn range_with_cu_and_classes(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    range: crate::definition::LspRange,
    registry: &CommandRegistry,
    cu: Option<&CompilationUnit>,
    classes: Option<&ClassHierarchy>,
) -> SemanticTokens {
    range_with_cu_and_classes_and_roles(source, dialect, range, registry, cu, classes, None)
}

/// [`range_with_cu_and_classes`] with the workspace-merged inferred
/// variable-name argument roles (see [`full_with_cu_and_classes_and_roles`]).
#[must_use]
pub fn range_with_cu_and_classes_and_roles(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    range: crate::definition::LspRange,
    registry: &CommandRegistry,
    cu: Option<&CompilationUnit>,
    classes: Option<&ClassHierarchy>,
    proc_roles: Option<&VarNameArgRoles>,
) -> SemanticTokens {
    range_with_cu_and_facts(
        source,
        dialect,
        range,
        registry,
        cu,
        WorkspaceTokenFacts {
            classes,
            proc_roles,
            named_instances: None,
            project_classes: None,
            analysis: None,
        },
    )
}

/// [`range_with_cu_and_classes_and_roles`] with the workspace-merged named
/// bareword instance-command index bundled in (see [`full_with_cu_and_facts`]).
#[must_use]
pub fn range_with_cu_and_facts(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    range: crate::definition::LspRange,
    registry: &CommandRegistry,
    cu: Option<&CompilationUnit>,
    facts: WorkspaceTokenFacts<'_>,
) -> SemanticTokens {
    let mut entries = collect_entries(source, dialect, registry, cu, facts);
    entries.retain(|(line, col, _, _, _)| {
        // Half-open interval per LSP `Range` semantics: start is
        // inclusive, end is exclusive.
        let pos = (*line, *col);
        let start = (range.start_line, range.start_character);
        let end = (range.end_line, range.end_character);
        pos >= start && pos < end
    });
    encode_entries(&entries)
}

fn collect_entries(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    registry: &CommandRegistry,
    cu: Option<&CompilationUnit>,
    facts: WorkspaceTokenFacts<'_>,
) -> Vec<Entry> {
    let WorkspaceTokenFacts {
        classes,
        proc_roles,
        named_instances: _,
        project_classes,
        analysis,
    } = facts;
    // Whether the caller supplied the document's own analysis, or this is the
    // explicit standalone source API (the fresh analysis below).
    let document_analysis = analysis.is_some();
    let fresh_analysis;
    let analysis = if let Some(analysis) = analysis {
        analysis
    } else {
        fresh_analysis = standalone_token_analysis(source, dialect, registry);
        &fresh_analysis
    };
    let Some(current) = crate::original_context::CurrentSourceContext::capture(source, analysis)
    else {
        return Vec::new();
    };
    let config = current.config();
    let registry = current.registry();
    let dialect = current.profile();
    let generation = current.context();
    // A source shell interpreted nothing, so every command reads its written
    // words; any analysis query would interpret the document after all.
    let retained = analysis;
    let shell = retained.is_source_shell().then_some(retained);
    let lexical = shell.is_none() && retained.allows_lexical_declaration_advice();
    let analysis = shell.is_none().then_some(retained);
    let mut entries: Vec<Entry> = Vec::new();
    let line_index = LineIndex::new(source);
    let profile = dialect;

    // Regex-source spans: the def-site literal words (`set my_re ".*"`) whose
    // variable flows into a `regexp`/`regsub` pattern, keyed by word start so
    // the walk can retag the matching argument.  Empty (no map lookups) when
    // there is no analysis or no such flow. The compiler decides the spans
    // under the unit's own profile, so every profile reads them.
    let regex_sources: FxHashMap<u32, Span> = cu.map_or_else(FxHashMap::default, |cu| {
        tcl_compiler::regex_source::regex_source_literal_spans(source, cu, registry, dialect)
            .into_iter()
            .map(|span| (span.start(), span))
            .collect()
    });

    // The document's command-identity facts: bare-name aliases for commands
    // imported from an exported namespace (`namespace import tcltest::*` →
    // `test` = `tcltest::test`), plus every statically proven `interp alias` /
    // `rename` / built-in-shadowing `proc`.  Empty (no lookups)
    // unless the document actually binds something.
    let Some(head_identities) = retained.retained_command_realm() else {
        return Vec::new();
    };

    // The iRules declaration overlay uses the shared top-level boundary facts
    // (including offset-resolved command identity) rather than this walk's
    // recursive depth as its placement predicate.
    let irules_top_level_declaration_heads = if lexical || dialect.is_irules() {
        irules_top_level_declaration_heads(source, registry, config, head_identities)
    } else {
        FxHashSet::default()
    };

    let (object_classes, _object_collections) =
        token_object_maps(source, registry, cu, retained, lexical, facts);

    // Walk every segmented command (recursing into braced bodies, braced
    // expressions, and `[…]` command substitutions) and classify each token.
    let original_collections = cu
        .filter(|_| lexical || project_classes.is_some())
        .and_then(|unit| {
            tcl_compiler::object_types::OriginalObjectCollectionSources::capture(
                source, analysis?, unit,
            )
        });
    let original_roles = original::OriginalTokenRoles::capture(source, analysis);
    let walk = WalkFacts::new(shell, analysis);
    let ctx = ScriptCtx {
        full_source: source,
        dialect,
        config,
        context: generation.context(),
        generation: &generation,
        operand: None,
        original_words: None,
        declared_words: None,
        written_members: false,
        walk: &walk,
        lexical,
        numbers: profile.grammar.numbers,
        registry,
        line_index: &line_index,
        // The root of a document is itself a definition body when the dialect
        // says so: a `.sslictcl` document's legal top-level words are exactly
        // its document grammar's members, painted from membership like every
        // level below. An ordinary Tcl document has no such grammar.
        oo_grammar: registry.document_grammar(),
        receiver_grammar: None,
        original_definition_parent: None,
        original_definition_members: None,
        scoped_env: None,
        regex_sources: &regex_sources,
        object_classes: &object_classes,
        classes,
        project_classes,
        analysis,
        original_collections: original_collections.as_ref(),
        original_roles: &original_roles,
        proc_roles,
        enclosing_class: None,
        irules_top_level_declaration_heads: &irules_top_level_declaration_heads,
    };
    collect_context_entries(ctx, cu, document_analysis, &mut entries);
    entries
}

fn standalone_token_analysis(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    registry: &CommandRegistry,
) -> AnalysisResult {
    standalone_analyser(dialect, registry)
        .structure_only()
        .analyse(source, dialect.name)
}

fn standalone_analyser(
    dialect: &'static tcl_dialect::DialectProfile,
    registry: &CommandRegistry,
) -> tcl_compiler::analyser::Analyser {
    let context = tcl_registry::model::context_for_profile(dialect);
    let context =
        std::sync::Arc::new(context.with_command_store(registry.snapshot().shared_registry()));
    let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
        dialect,
        dialect,
        context,
        tcl_lexer::LexerConfig::for_file_grammar(dialect.grammar),
    );
    tcl_compiler::analyser::Analyser::new().with_resolved_input(input)
}

fn token_object_maps(
    source: &str,
    registry: &CommandRegistry,
    cu: Option<&CompilationUnit>,
    retained: &AnalysisResult,
    lexical: bool,
    facts: WorkspaceTokenFacts<'_>,
) -> (ObjectClassMap, ObjectClassMap) {
    let WorkspaceTokenFacts {
        classes,
        named_instances,
        project_classes,
        ..
    } = facts;
    let analysis = Some(retained);
    // Source candidates retain the same complete analysis/Module inputs as
    // their shared producer. A release profile colours handles from the unit's
    // own map; the map is display data and proves no receiver.
    let mut object_classes: ObjectClassMap = cu
        .and_then(|cu| {
            if lexical {
                tcl_compiler::object_types::logical_source_object_handle_classes(
                    cu, registry, analysis?,
                )
            } else {
                Some(tcl_compiler::object_types::object_handle_classes(
                    cu, registry,
                ))
            }
        })
        .unwrap_or_default();

    // Logical object classification shares this compatibility handle map.
    // Native source roles never recover receiver identity from its labels.
    if project_classes.is_none()
        && let Some(named) = named_instances
    {
        for (name, class) in named.compatibility_rows() {
            object_classes
                .entry(name.clone())
                .or_default()
                .insert(class.clone());
        }
    }

    // Object-*collection* → element-class map (`dict set Pins $k [Pin new]` →
    // `Pins` is a `Dict` of `Pin`), so a `[dict get $Pins $k] method …`
    // retrieval dispatch resolves the element's method.  Empty
    // without a `CompilationUnit`.
    let object_collections: ObjectClassMap = cu.map_or_else(ObjectClassMap::default, |cu| {
        tcl_compiler::object_types::object_collection_classes(cu)
    });

    // A loop that iterates an object collection binds its value variable to an
    // element, so `$v method …` in the body resolves like `[dict get $coll $k]
    // method …`.  A *syntactic* scan of the source catches every loop shape —
    // including `return [dict map {k v} $coll {…}]` / `set x [dict map …]`,
    // where the loop is nested in a command substitution and the IR never
    // surfaces it as a loop — and feeds the value variable(s) into the handle
    // map (the SpiceGenTcl `allNodes` / `actOnParam` shape).
    augment_loop_var_handles(source, retained, &object_collections, &mut object_classes);

    // snit object-handle bindings the compiler CFG doesn't surface — `install
    // NAME using TYPE` components and `set NAME [Type inst]` bare constructors —
    // via a source scan, since snit method bodies (where these live) are not
    // lowered into the CFG `object_handle_classes` reads.
    augment_snit_handles(source, retained, classes, &mut object_classes);

    (object_classes, object_collections)
}

fn collect_context_entries(
    ctx: ScriptCtx<'_>,
    cu: Option<&CompilationUnit>,
    document_analysis: bool,
    entries: &mut Vec<Entry>,
) {
    let source = ctx.full_source;
    let analysis = ctx.analysis;
    let registry = ctx.registry;
    let profile = ctx.dialect;
    let lexical = ctx.lexical;
    let line_index = ctx.line_index;
    collect_script(ctx, source, 0, entries, 0, false);

    // The shared original source walker retains comment phase independently
    // of the semantic colours already emitted for other source roles.
    push_comment_tokens(source, line_index, analysis, entries);
    // A body the analysis did not enter is still coloured as script by the
    // walk above; its comments are read from the same regions.
    push_entered_comment_tokens(ctx, &ctx.walk.entered_scripts.borrow(), entries);

    // BIG-IP object references (iRules dialect): overlay `object` tokens
    // at recognised pool / data-group / virtual / … name positions.
    // Skipped when an entry already covers the position (e.g. a
    // single-line body's enclosing `string` token) so the token stream
    // never carries overlaps.  Multi-line bodies aren't tokenised by the
    // main walk, so refs inside them surface cleanly.
    if lexical && profile.is_irules() {
        let spans = match analysis {
            Some(analysis) if document_analysis => {
                irules_document_object_spans(source, analysis, cu, registry)
            }
            _ => crate::irules_object_refs::object_ref_spans(source, registry),
        };
        for span in spans {
            push_object_token(source, line_index, span, entries);
        }
    }

    // Sort by (line, column) so the delta encoding works.
    entries.sort_by_key(|(line, col, _, _, _)| (*line, *col));
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod original_proc_role_tests;

#[cfg(test)]
mod original_procedure_topology_colour_tests;

#[cfg(test)]
mod original_variable_argument_geometry_colour_tests;

#[cfg(test)]
mod original_realm_consumer_tests;

#[cfg(test)]
mod original_definition_vocabulary_tests;

#[cfg(test)]
mod selected_source_token_tests;

#[cfg(test)]
mod configured_semantic_receiver_tests;

#[cfg(test)]
mod original_comment_token_tests;

/// The iRules overlays (BIG-IP object references, `when`/procedure
/// declarations) over representative fixtures, read the way the server
/// supplies an iRules document: its own analysis, input and unit.
#[cfg(test)]
mod irules_overlay_tests;
