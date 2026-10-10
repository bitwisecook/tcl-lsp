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

//! Regex-source tracking: find the def-site string literal(s) that flow into a
//! `regexp` / `regsub` pattern via a variable, so the semantic-token layer can
//! highlight the *originating* literal as a regex.
//!
//! Example — `set my_re ".*abc"; regexp $my_re $s` — the `.*abc` at the `set`
//! is the pattern's real text, and this module returns its source span so it
//! reads as a regex rather than a plain string.
//!
//! # How it works
//!
//! This is a dataflow query over an already-built [`CompilationUnit`] (the SSA
//! tier), using the exact reaching definition visible at the pattern word:
//!
//! 1. Walk every function's CFG statements alongside its SSA statements.
//! 2. Record, per physical SSA value `(symbol, version)`:
//!    * the source span of the assigned **value word** for a literal
//!      assignment (`AssignConst` / `AssignValue`), and
//!    * the incoming versions of a φ (control-flow merge), so a conditionally
//!      / re-assigned pattern resolves to *all* its reaching literals.
//! 3. Select the original pattern operand through the actual handler's registry
//!    layout, then ask `SsaSourceView::read_word` for that exact substitution's
//!    physical symbol and version. Quoted and qualified reads use this same API;
//!    an unknown receiver or missing read inventory yields no source proof.
//! 4. For each such use, require every reaching definition to be a lexical
//!    source literal, then resolve its def-site spans (recursing through φs).
//!    This point-in-time proof remains valid when a callback-bearing command
//!    conservatively widens SCCP after its arguments have already substituted.
//!
//! The selected handler's registry descriptor owns pattern argument layout.
//! Original source words and the retained native dialect select that layout;
//! aliases retain their written operand origins. Variable reads require exact
//! physical SSA evidence at that operand.
//!
//! # Scope
//!
//! The query covers every function the [`CompilationUnit`] lowers: the
//! top-level script, each `proc` body, each `TclOO` method / constructor /
//! destructor body (lowered to its own `FunctionUnit` under
//! [`CompilationUnit::methods`]), and each synthetic *body unit* — `apply`
//! lambda bodies and `namespace eval` blocks (under
//! [`CompilationUnit::body_units`]).  So a `set`/`regexp` flow is tracked inside
//! a method, a lambda, or a namespace-eval body exactly as inside a proc.

use std::collections::{HashMap, HashSet};

use tcl_lexer::{SourceImage, Span};
use tcl_registry::CommandRegistry;

use crate::compilation_unit::{CompilationUnit, FunctionUnit};
use crate::ir::Statement;
use crate::ssa::{Symbol, ValueKey, Version};

/// Source spans of the def-site value literals that feed a `regexp` / `regsub`
/// pattern through a variable whose reaching definitions are all lexical
/// string literals. Sorted by start, de-duplicated. Empty when the unit has no
/// such flow (the common case), so callers pay nothing extra. The compatibility
/// dialect argument does not replace the CU's retained input or source grammar;
/// missing or stale supplied owners withhold this readonly result.
#[must_use]
pub fn regex_source_literal_spans(
    source: &str,
    cu: &CompilationUnit,
    registry: &CommandRegistry,
    _dialect: &'static tcl_dialect::DialectProfile,
) -> Vec<Span> {
    let module = &cu.ir_module;
    if module.source != SourceImage::document(source)
        || !module
            .retained_source_bindings
            .as_deref()
            .is_some_and(|owner| owner.matches_module(module, registry))
    {
        return Vec::new();
    }
    let mut spans: Vec<Span> = Vec::new();
    let mut units: Vec<&FunctionUnit> =
        Vec::with_capacity(cu.procedures.len() + cu.methods.len() + cu.body_units.len() + 1);
    units.push(&cu.top_level);
    units.extend(cu.procedures.values());
    // `TclOO` method / constructor / destructor bodies are lowered to their own
    // `FunctionUnit`s (keyed `{class}::{method}`); walk them too so a
    // `set re "…"; regexp $re` inside a method tracks like one in a proc.
    units.extend(cu.methods.values());
    // Synthetic body units — `apply` lambda bodies and `namespace eval` blocks —
    // are lowered to their own fresh-frame `FunctionUnit`s (§ Scope). Walking
    // them lets a `set re "…"; regexp $re` *inside* a lambda or namespace-eval
    // body track its def-site literal the same way a proc body does.
    units.extend(cu.body_units.values());
    for fu in units {
        collect_in_function(fu, module, registry, &mut spans);
    }
    spans.sort_by_key(|s| (s.start(), s.end()));
    spans.dedup();
    spans
}

/// Per-function scan state.
#[derive(Default)]
struct Scan {
    /// `(physical symbol, version)` → absolute source span of the assigned value word,
    /// for a literal-assignment def.
    const_def_span: HashMap<ValueKey, Span>,
    /// Definitions whose assigned value is a source literal, used for the
    /// point-in-time fallback when a later callback barrier widens SCCP.
    literal_source_defs: HashSet<ValueKey>,
    /// `(physical symbol, version)` → φ incoming versions (control-flow merge defs).
    phi_incoming: HashMap<ValueKey, Vec<Version>>,
    /// Exact physical read key for each selected sole pattern substitution.
    regex_uses: Vec<ValueKey>,
}

fn collect_in_function(
    fu: &FunctionUnit,
    module: &crate::ir::Module,
    registry: &CommandRegistry,
    out: &mut Vec<Span>,
) {
    if !module.source_entry.metadata_context.is_standalone()
        && fu
            .invocation_metadata_context_for_module(registry, module)
            .is_none()
    {
        return;
    }
    let mut scan = Scan::default();

    for (block_id, block) in &fu.cfg.blocks {
        let Some(ssa_block) = fu.ssa.blocks.get(block_id) else {
            continue;
        };
        if !fu.sccp.executable_blocks.contains(block_id) {
            continue;
        }
        // φ definitions at the block head.
        for phi in &ssa_block.phis {
            let key = (phi.name, phi.version);
            let incoming: Vec<Version> = phi.incoming.values().copied().collect();
            scan.phi_incoming.entry(key).or_insert(incoming);
        }

        for (stmt_idx, stmt) in block.statements.iter().enumerate() {
            let Some(ssa_stmt) = ssa_block.statements.get(stmt_idx) else {
                continue;
            };
            // A literal assignment records its value-word span, keyed by the
            // SSA version it defines.
            if let Some(vspan) = literal_value_word_span(stmt, module, registry) {
                for (sym, ver) in &ssa_stmt.defs {
                    let key = (*sym, *ver);
                    scan.const_def_span.entry(key).or_insert(fu.abs_span(vspan));
                    // `AssignValue` also covers a quoted or bare literal
                    // (`set re "x+"`, `set re x+`) when the word has no
                    // substitution.  Admit only that statically literal
                    // subset; `$x`, `[cmd]`, and compound words remain
                    // dynamic and must never become regex sources merely
                    // because SCCP happens to infer a string value.
                    scan.literal_source_defs.insert(key);
                }
            }

            // Resolve the exact substitution rather than looking up a display
            // name in a statement-wide union of versions.
            if let Some(word) = regex_pattern_word(stmt, module, registry)
                && let Some(read) =
                    crate::ssa::SsaSourceView::at_statement(&fu.ssa, *block_id, stmt_idx)
                        .read_word(word)
                && let Some(version) = read.version
            {
                scan.regex_uses.push((read.symbol, version));
            }
        }
    }

    // Resolve each pattern-variable use to its def-site literal span(s), gated
    // on every reaching definition being a source literal.
    for (name, use_ver) in &scan.regex_uses {
        let mut literal_proof = HashSet::new();
        // Source provenance is the final gate: a substituted/dynamic
        // `AssignValue` must not count as a literal even if SCCP has inferred
        // a string for it.
        if !all_reaching_defs_are_literals(&scan, *name, *use_ver, &mut literal_proof) {
            continue;
        }
        let mut visited: HashSet<ValueKey> = HashSet::new();
        resolve_def_spans(&scan, *name, *use_ver, &mut visited, out);
    }
}

/// Whether every reaching definition of `(name, version)` is a source literal.
///
/// A same-invocation callback barrier conservatively widens SCCP after the
/// call, but Tcl substitutes the pattern word before entering the command and
/// before that callback can run. The SSA use version is therefore the exact
/// point-in-time proof needed here. Requiring every reaching definition to
/// have a literal source span preserves abstention at dynamic or mixed phis.
fn all_reaching_defs_are_literals(
    scan: &Scan,
    name: Symbol,
    version: Version,
    visited: &mut HashSet<ValueKey>,
) -> bool {
    let key = (name, version);
    if !visited.insert(key) {
        // A cyclic phi needs a fixed-point proof, which this source-span query
        // deliberately does not attempt. Abstain rather than let the cycle
        // prove itself literal.
        return false;
    }
    // `visited` is a recursion *stack*, not a memo table.  A definition can
    // legitimately be shared by two arms of a diamond; leaving it marked
    // after the first arm would mistake that ordinary DAG sharing for a phi
    // cycle and reject an otherwise literal proof.  Keep the mark only while
    // this definition is on the active recursion path, so a genuine back-edge
    // still abstains above.
    let proven = if scan.literal_source_defs.contains(&key) {
        true
    } else {
        scan.phi_incoming.get(&key).is_some_and(|incoming| {
            !incoming.is_empty()
                && incoming
                    .iter()
                    .all(|&version| all_reaching_defs_are_literals(scan, name, version, visited))
        })
    };
    visited.remove(&key);
    proven
}

/// Push the def-site value spans reaching `(name, version)`, recursing through
/// φ merges.  `visited` guards against φ cycles (loops).
fn resolve_def_spans(
    scan: &Scan,
    name: Symbol,
    version: Version,
    visited: &mut HashSet<ValueKey>,
    out: &mut Vec<Span>,
) {
    let key = (name, version);
    if !visited.insert(key) {
        return;
    }
    if let Some(span) = scan.const_def_span.get(&key) {
        out.push(*span);
        return;
    }
    if let Some(incoming) = scan.phi_incoming.get(&key) {
        for &inc in incoming {
            resolve_def_spans(scan, name, inc, visited, out);
        }
    }
}

/// True for a literal-assignment statement whose value word is a source
/// literal (`set VAR "…"` / `set VAR {…}` — `AssignConst`/`AssignValue`).
fn is_literal_assignment(stmt: &Statement) -> bool {
    matches!(
        stmt,
        Statement::AssignConst { .. } | Statement::AssignValue { .. }
    )
}

/// The whole unchanged literal word selected by this original setter.
/// Source geometry and conditional Logical assignment metadata grant no
/// physical definition: the caller still uses the independently retained SSA.
fn literal_value_word_span(
    stmt: &Statement,
    module: &crate::ir::Module,
    registry: &CommandRegistry,
) -> Option<Span> {
    if !is_literal_assignment(stmt) {
        return None;
    }
    let tokens = stmt.tokens()?;
    let metadata = source_metadata_at(tokens, module, registry)?;
    let value = if let Some(metadata) =
        metadata.filter(|metadata| metadata.permits_logical_source_names())
    {
        let selected = crate::registry_invocation::original_logical_operation_invocation_with_metadata_context(
            registry, metadata, tokens,
        )?;
        if selected.facts.operation
            != tcl_registry::SemanticOperationId::StructuredLowering(
                tcl_registry::hooks::LoweringHookId::Set,
            )
            || selected.effective.words.len() != 3
        {
            return None;
        }
        selected.effective.words.get(2)?.clone()
    } else {
        crate::registry_invocation::normal_representation_invocation_with_metadata_context(
            registry, metadata, tokens,
        )?
        .value_assignment()?
        .value
    };
    let mut matches = tokens
        .words()
        .iter()
        .enumerate()
        .filter(|(_, word)| **word == value);
    let (ordinal, _) = matches.next()?;
    if matches.next().is_some() || ordinal == 0 {
        return None;
    }
    let binding = tokens.source_binding.as_ref()?;
    let site = binding.invocation_site()?;
    let image = site.source.source_image();
    let config = binding.original_lexer_config_for_tokens(tokens)?;
    let words = crate::registry_invocation::original_native_compiler_words(
        image,
        tokens.words(),
        site.offset,
        config,
    )?;
    let original = words.get(ordinal)?;
    let content = image
        .bytes()
        .get(original.content_span().ok()?.as_range())?;
    (tcl_syntax::word_rules::original_static_word_unicode_value(original).as_deref()
        == Some(content))
    .then_some(original.span())
}

/// Keep supplied missing ownership terminal. Only a positively tagged
/// standalone Module may use its explicitly selected compatibility context.
fn source_metadata_at<'a>(
    tokens: &'a crate::ir::CommandTokens,
    module: &crate::ir::Module,
    registry: &CommandRegistry,
) -> Option<Option<crate::registry_invocation::InvocationMetadataContext<'a>>> {
    let owner = module.retained_source_bindings.as_deref()?;
    if !owner.matches_module(module, registry)
        || tokens.synthetic.is_some()
        || !tokens.words_align_with_argv_text()
    {
        return None;
    }
    if module.source_entry.metadata_context.is_standalone()
        && module.source_metadata_input.is_none()
    {
        return owner.owns_original_tokens(tokens).then_some(None);
    }
    Some(Some(
        tokens
            .source_binding
            .as_ref()?
            .original_invocation_metadata_for_module(tokens, module, registry)?,
    ))
}

/// The full span of the word beginning at byte `start`: for a `"…"` / `{…}`
/// word, extend to the matching close delimiter (honouring `\`-escapes and
/// brace nesting); a bareword ends at the next unescaped whitespace or command
/// separator. Recovers the closing delimiter the segmenter's token span clamps
/// off.
#[cfg(test)]
fn delimited_word_span(source: &str, start: usize) -> Span {
    let bytes = source.as_bytes();
    let s32 = u32::try_from(start).unwrap_or(0);
    let end = match bytes.get(start) {
        Some(b'"') => scan_to_close(bytes, start + 1, b'"', b'"'),
        Some(b'{') => scan_to_close(bytes, start + 1, b'{', b'}'),
        _ => {
            let mut i = start;
            // A bare Tcl word ends at whitespace *or* an unquoted command
            // separator.  The enclosing IR span stops before `;`, so failing
            // to stop here would make `set re x+; regsub ...` highlight the
            // semicolon and possibly the next command as part of `x+`.
            while i < bytes.len() {
                if bytes[i] == b'\\' && i + 1 < bytes.len() {
                    i += 2;
                } else if bytes[i].is_ascii_whitespace() || bytes[i] == b';' {
                    break;
                } else {
                    i += 1;
                }
            }
            i
        }
    };
    Span::new(s32, u32::try_from(end).unwrap_or(s32).max(s32))
}

/// Scan from `i` for the `close` delimiter, honouring `\`-escapes and (when
/// `open != close`) nesting.  Returns the index *past* the close, or the input
/// length when unterminated.
#[cfg(test)]
fn scan_to_close(bytes: &[u8], mut i: usize, open: u8, close: u8) -> usize {
    let mut depth = 1usize;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => i += 2,
            b if b == open && open != close => {
                depth += 1;
                i += 1;
            }
            b if b == close => {
                depth -= 1;
                i += 1;
                if depth == 0 {
                    return i;
                }
            }
            _ => i += 1,
        }
    }
    bytes.len()
}

/// Project a single pattern slot through the selected registry-owned layout.
/// Unknown options and expansion keep the source position indeterminate.
#[must_use]
pub(crate) fn regexp_pattern_index(
    registry: &CommandRegistry,
    command: &str,
    args: tcl_registry::InvocationArguments<'_>,
) -> Option<usize> {
    let indices =
        registry.arg_indices_for_role_words(command, args, tcl_registry::ArgRole::Pattern)?;
    let [index] = indices.as_slice() else {
        return None;
    };
    Some(*index)
}

/// Recover the original segmented argument words for an authoring-only query.
/// This does not establish the handler identity or a runtime operation proof.
#[must_use]
pub(crate) fn source_pattern_index(
    source: &str,
    registry: &CommandRegistry,
    command: &str,
    arg_tokens: &[tcl_lexer::Token],
    config: tcl_lexer::LexerConfig,
    dialect: tcl_registry::InvocationDialect,
) -> Option<usize> {
    crate::registry_invocation::with_source_argument_words(
        source,
        arg_tokens,
        config,
        dialect,
        |arguments| regexp_pattern_index(registry, command, arguments),
    )?
}

/// Retain the original sole variable substitution selected as the pattern by
/// the actual handler. Physical read identity is resolved separately by SSA.
fn regex_pattern_word<'a>(
    stmt: &'a Statement,
    module: &crate::ir::Module,
    registry: &CommandRegistry,
) -> Option<&'a crate::ir::WordExpr> {
    let tokens = stmt.tokens()?;
    let metadata = source_metadata_at(tokens, module, registry)?;
    let idx = if let Some(metadata) =
        metadata.filter(|metadata| metadata.permits_logical_source_names())
    {
        let selected = crate::registry_invocation::original_logical_operation_invocation_with_metadata_context(
            registry, metadata, tokens,
        )?;
        let realm = tokens.source_binding.as_ref()?.invocation_realm()?;
        let patterns = selected.with_metadata_schema(registry, metadata, realm, |schema| {
            schema.authored_source_pattern_arguments()
        })?;
        let mut patterns = patterns
            .into_iter()
            .filter(|pattern| pattern.kind == tcl_registry::patterns::PatternType::Regex);
        let pattern = patterns.next()?;
        if patterns.next().is_some() {
            return None;
        }
        match selected
            .effective
            .origins
            .get(usize::from(pattern.index).checked_add(1)?)?
        {
            crate::registry_invocation::InvocationWordOrigin::Written(index) => {
                index.checked_sub(1)?
            }
            _ => return None,
        }
    } else {
        crate::registry_invocation::normal_representation_invocation_with_metadata_context(
            registry, metadata, tokens,
        )?
        .pattern_source_argument_index(registry)?
    };
    let word = tokens.words().get(idx.checked_add(1)?)?;
    word.sole_variable_substitution()?;
    Some(word)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::segmenter::segment_commands_with_offset_and_config;

    fn spans_text(source: &str) -> Vec<String> {
        spans_text_dialect(source, tcl_dialect::DialectProfile::find("tcl9.0").unwrap())
    }

    fn logical_regex_unit(
        source: &str,
        context: &std::sync::Arc<tcl_registry::model::ContextRegistry>,
        config: tcl_lexer::LexerConfig,
    ) -> CompilationUnit {
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            std::sync::Arc::clone(context),
            config,
        );
        CompilationUnit::build_with_analysis_input(
            source,
            crate::compilation_unit::UnitBuildOptions {
                registry: context.commands(),
                defer_top_level: false,
                config,
                dialect: Some(profile),
                external_call_sites: None,
                declared_commands: None,
            },
            None,
            &input,
        )
    }

    #[test]
    fn original_regex_roles_keep_alias_origins_and_do_not_supply_native_reads() {
        // naming.core.original-pattern-retained-context
        // docs/design/analysis/name-resolution-proofs/original-pattern-retained-context.md
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry();
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
        let source = "interp alias {} matcher {} regexp --; matcher ${ré} subject";
        let unit = logical_regex_unit(source, &context, config);
        let statement = unit.ir_module.top_level.statements.last().unwrap();
        let word = regex_pattern_word(statement, &unit.ir_module, context.commands())
            .expect("original written pattern after captured option");
        assert_eq!(&source[word.source().span.as_range()], "${ré}");
        let metadata = source_metadata_at(
            statement.tokens().unwrap(),
            &unit.ir_module,
            context.commands(),
        )
        .unwrap();
        assert!(
            crate::registry_invocation::normal_representation_invocation_with_metadata_context(
                context.commands(),
                metadata,
                statement.tokens().unwrap(),
            )
            .is_none(),
            "conditional source roles do not prove a normal Native handler"
        );
        assert!(
            regex_source_literal_spans(source, &unit, context.commands(), profile).is_empty(),
            "a source role supplies no physical SSA read"
        );
        let mut missing = statement.clone();
        missing.tokens_mut().unwrap().source_binding = None;
        assert!(regex_pattern_word(&missing, &unit.ir_module, context.commands()).is_none());
        for source in [
            "interp alias {} matcher {} regexp -- {x+}; matcher $subject",
            "interp alias {} matcher {} regexp --; matcher {*}$values",
            "interp alias {} matcher {} regexp --; proc matcher {a b} {}; matcher $re subject",
        ] {
            let unit = logical_regex_unit(source, &context, config);
            assert!(
                regex_pattern_word(
                    unit.ir_module.top_level.statements.last().unwrap(),
                    &unit.ir_module,
                    context.commands()
                )
                .is_none(),
                "{source}"
            );
        }
    }

    #[test]
    fn original_regex_inventory_keeps_full_configuration_and_withdraws_owners() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry();
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let config = tcl_lexer::LexerConfig {
            expand_syntax: false,
            strict_quoting: true,
            ..tcl_lexer::LexerConfig::for_file_grammar(profile.grammar)
        };
        let source = "set re {*}; regexp -- $re subject";
        let unit = logical_regex_unit(source, &context, config);
        let setter = unit.ir_module.top_level.statements.first().unwrap();
        let literal = literal_value_word_span(setter, &unit.ir_module, context.commands())
            .expect("actual original literal under disabled expansion");
        assert_eq!(&source[literal.as_range()], "{*}");
        let pattern = unit.ir_module.top_level.statements.last().unwrap();
        assert!(regex_pattern_word(pattern, &unit.ir_module, context.commands()).is_some());
        for mutate in [0, 1, 2] {
            let mut changed = unit.ir_module.clone();
            match mutate {
                0 => changed.source_metadata_input = None,
                1 => changed.lexer_config.expand_syntax = true,
                _ => changed.retained_source_bindings = None,
            }
            assert!(literal_value_word_span(setter, &changed, context.commands()).is_none());
            assert!(regex_pattern_word(pattern, &changed, context.commands()).is_none());
        }
        let foreign = CommandRegistry::build_default();
        assert!(regex_pattern_word(pattern, &unit.ir_module, &foreign).is_none());
        assert!(
            regex_source_literal_spans(
                "set re OTHER; regexp -- $re subject",
                &unit,
                context.commands(),
                profile
            )
            .is_empty()
        );
        let options = "regsub -command -- $re subject callback out";
        let current = logical_regex_unit(options, &context, config);
        assert!(
            regex_pattern_word(
                current.ir_module.top_level.statements.last().unwrap(),
                &current.ir_module,
                context.commands()
            )
            .is_some()
        );
        let older =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
        let older = std::sync::Arc::new(
            older.with_command_store(context.commands().snapshot().shared_registry()),
        );
        let old_unit = logical_regex_unit(options, &older, config);
        assert!(
            regex_pattern_word(
                old_unit.ir_module.top_level.statements.last().unwrap(),
                &old_unit.ir_module,
                older.commands()
            )
            .is_none()
        );
    }

    #[test]
    fn single_literal_flows_to_regexp() {
        // The `.*abc` literal at the `set` is the regex source.
        let got = spans_text("set my_re \".*abc\"\nregexp $my_re $s\n");
        assert_eq!(got, vec!["\".*abc\"".to_owned()]);
    }

    #[test]
    fn regexp_pattern_index_skips_options() {
        let idx = |a: &[&str]| {
            let registry = CommandRegistry::build_default();
            regexp_pattern_index(
                &registry,
                "regexp",
                tcl_registry::InvocationArguments::literals(a).with_dialect(
                    tcl_registry::InvocationDialect::of_profile(
                        tcl_dialect::DialectProfile::find("tcl9.0").unwrap(),
                    ),
                ),
            )
        };
        // No options — pattern is arg 0.
        assert_eq!(idx(&["pat", "s"]), Some(0));
        // Boolean flags are skipped one word each.
        assert_eq!(idx(&["-nocase", "-all", "pat", "s"]), Some(2));
        // `-start` consumes its value word.
        assert_eq!(idx(&["-start", "5", "pat", "s"]), Some(2));
        // `--` terminates the option scan; the very next word is the pattern
        // even if it starts with `-`.
        assert_eq!(idx(&["-nocase", "--", "-pat", "s"]), Some(2));
        // Only options, no pattern → None.
        assert_eq!(idx(&["-nocase"]), None);
        assert_eq!(idx(&["-start"]), None);
        // `-start` consumes the following word as its index value, so a lone
        // `-start pat` leaves no pattern behind → None.
        assert_eq!(idx(&["-start", "pat"]), None);
    }

    #[test]
    fn pattern_layout_preserves_unknown_options_and_fixed_option_values() {
        use tcl_registry::{InvocationArguments, InvocationDialect, InvocationWord};
        let registry = CommandRegistry::build_default();
        let dialect = InvocationDialect::for_version(tcl_dialect::TclVersion::V9_0);
        let locate = |words: &[InvocationWord<'_>]| {
            regexp_pattern_index(
                &registry,
                "regexp",
                InvocationArguments::structured(words).with_dialect(dialect),
            )
        };
        assert_eq!(
            locate(&[
                InvocationWord::Dynamic,
                InvocationWord::Literal("p"),
                InvocationWord::Literal("s")
            ]),
            None
        );
        assert_eq!(
            locate(&[
                InvocationWord::Literal("-start"),
                InvocationWord::Dynamic,
                InvocationWord::Literal("p"),
                InvocationWord::Dynamic
            ]),
            Some(2)
        );
        assert_eq!(
            locate(&[
                InvocationWord::Literal("--"),
                InvocationWord::Dynamic,
                InvocationWord::Dynamic
            ]),
            Some(1)
        );
        assert_eq!(
            locate(&[
                InvocationWord::Expanded,
                InvocationWord::Literal("p"),
                InvocationWord::Literal("s")
            ]),
            None
        );
    }

    #[test]
    fn source_pattern_layout_uses_original_substitution_and_quote_boundaries() {
        let registry = CommandRegistry::build_default();
        let dialect = tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V9_0);
        let config = tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar);
        for (source, expected) in [
            ("regexp -start $start \"a+$suffix\" $input", Some(2)),
            ("regexp $option {a+} $input", None),
            ("regexp -- \"-a+$suffix\" $input", Some(1)),
            ("regexp {*}$options {a+} $input", None),
        ] {
            let segment = segment_commands_with_offset_and_config(source, 0, config)
                .into_iter()
                .next()
                .unwrap();
            assert_eq!(
                source_pattern_index(
                    source,
                    &registry,
                    "regexp",
                    segment.arg_tokens(),
                    config,
                    dialect
                ),
                expected,
                "{source}"
            );
        }
    }

    #[test]
    fn proc_wrapped_literal_flows_to_regexp() {
        // Same shape as `single_literal_flows_to_regexp`, but the `set`/
        // `regexp` pair lives inside a proc body rather than at top level —
        // the per-function CFG/SSA path must resolve it the same way.
        let got = spans_text(
            "proc ::bench::regex_check {} {\n    set my_re \".*abc\"\n    regexp $my_re $s\n}\n",
        );
        assert_eq!(got, vec!["\".*abc\"".to_owned()]);
    }

    /// The retag must still fire
    /// when the enclosing proc sits alongside hundreds of unrelated ones in
    /// the same file, matching the `large_file_semantic_tokens_refresh_delivers_enriched_result`
    /// e2e fixture — proves the per-function CFG/SSA facts this depends on
    /// don't get lost or truncated at scale.
    #[test]
    fn literal_flows_to_regexp_in_a_large_multi_proc_file() {
        use std::fmt::Write as _;
        let mut s = String::new();
        s.push_str("namespace eval ::bench {\n    variable counter 0\n}\n\n");
        for i in 0..600 {
            let _ = write!(
                s,
                "# proc number {i}\n\
                 proc ::bench::step{i} {{a b}} {{\n\
                 \x20   set v{i} [expr {{$a + $b}}]\n\
                 \x20   set msg \"step {i} = $v{i}\"\n\
                 \x20   if {{$v{i} > 10}} {{\n\
                 \x20       set v{i} [expr {{$v{i} + 1}}]\n\
                 \x20   }}\n\
                 \x20   return $v{i}\n\
                 }}\n\n"
            );
        }
        s.push_str(
            "proc ::bench::regex_check {} {\n    set my_re \".*abc\"\n    regexp $my_re $s\n}\n",
        );
        let got = spans_text(&s);
        assert_eq!(got, vec!["\".*abc\"".to_owned()]);
    }

    #[test]
    fn braced_literal_flows_to_regexp() {
        let got = spans_text("set re {a+b}\nregexp $re $s\n");
        assert_eq!(got, vec!["{a+b}".to_owned()]);
    }

    #[test]
    fn regsub_pattern_variable_tracked() {
        let got = spans_text("set re {x+}\nregsub -all $re $s Y out\n");
        assert_eq!(got, vec!["{x+}".to_owned()]);
    }

    #[test]
    fn reassignment_resolves_to_reaching_def() {
        // The use reaches the SECOND assignment only.
        let got = spans_text("set re \"a\"\nset re \"b\"\nregexp $re $s\n");
        assert_eq!(got, vec!["\"b\"".to_owned()]);
    }

    #[test]
    fn conditional_resolves_to_all_reaching_literals() {
        // Both branches assign a constant → both literals are regex sources.
        let src = "proc p {c s} {if {$c} {\n  set re \"aa\"\n} else {\n  set re \"bb\"\n}\nregexp $re $s\n}";
        let mut got = spans_text(src);
        got.sort();
        assert_eq!(got, vec!["\"aa\"".to_owned(), "\"bb\"".to_owned()]);
    }

    #[test]
    fn non_constant_source_is_not_tracked() {
        // `$x` is not a compile-time constant → nothing highlighted.
        let got = spans_text("set re $x\nregexp $re $s\n");
        assert!(got.is_empty(), "{got:?}");
    }

    #[test]
    fn literal_pattern_is_not_a_variable_source() {
        // An inline literal pattern is handled by the token layer, not here.
        let got = spans_text("regexp {abc} $s\n");
        assert!(got.is_empty(), "{got:?}");
    }

    #[test]
    fn subject_variable_is_not_treated_as_pattern() {
        // `$s` is the subject string, not the pattern — must not be tracked.
        let got = spans_text("set s \"literal\"\nregexp {abc} $s\n");
        assert!(got.is_empty(), "{got:?}");
    }

    #[test]
    fn pattern_inside_proc_body_tracked() {
        let src = "proc p {s} {\n  set re \".*x\"\n  regexp $re $s\n}\n";
        let got = spans_text(src);
        assert_eq!(got, vec!["\".*x\"".to_owned()]);
    }

    #[test]
    fn pattern_inside_oo_method_body_tracked() {
        // TclOO method bodies are lowered to their own `FunctionUnit`
        // (`CompilationUnit::methods`), so the flow tracks inside a method too.
        let src = "oo::class create C {\n  method m {s} {\n    ::set re \".*x\"\n    ::regexp $re $s\n  }\n}\n";
        let got = spans_text(src);
        assert_eq!(got, vec!["\".*x\"".to_owned()]);
    }

    #[test]
    fn pattern_inside_constructor_body_tracked() {
        let src = "oo::class create C {\n  constructor {s} {\n    ::set re {a+}\n    ::regexp $re $s\n  }\n}\n";
        let got = spans_text(src);
        assert_eq!(got, vec!["{a+}".to_owned()]);
    }

    #[test]
    fn pattern_inside_namespace_eval_body_tracked() {
        // A `namespace eval` body runs in its own frame, lowered to a synthetic
        // body unit (`CompilationUnit::body_units`), so the `set`/`regexp` flow
        // tracks inside it the same as in a proc.
        let src = "namespace eval ::ns {\n  set re \".*x\"\n  regexp $re $s\n}\n";
        let got = spans_text(src);
        assert_eq!(got, vec!["\".*x\"".to_owned()]);
    }

    #[test]
    fn pattern_inside_apply_lambda_body_tracked() {
        // An `apply` lambda body is a fresh-frame body unit; a `set re`/`regexp`
        // wholly inside the lambda tracks its def-site literal.
        let src = "apply {{s} {\n  set re {a+b}\n  regexp $re $s\n}} foo\n";
        let got = spans_text(src);
        assert_eq!(got, vec!["{a+b}".to_owned()]);
    }

    #[test]
    fn apply_lambda_param_is_not_a_def_site() {
        // The pattern variable is the lambda's *parameter* `re` (bound to the
        // caller's arg), not a body literal — so there is no def-site literal to
        // highlight. Proves the body unit binds params correctly (a bare `$re`
        // read resolves to the param, not to some caller scalar).
        let src = "apply {{re s} {\n  regexp $re $s\n}} {a+} foo\n";
        let got = spans_text(src);
        assert!(got.is_empty(), "{got:?}");
    }

    #[test]
    fn pattern_inside_array_for_body_tracked() {
        // `array for {k v} arr body` (Tcl 9.0) runs its body in the caller's
        // frame, so it is inlined into the caller unit with the loop vars bound;
        // a `set re`/`regexp` inside the body tracks its def-site literal.
        let src =
            "array set a {one foo}\narray for {k v} a {\n  set re \".*x\"\n  regexp $re $v\n}\n";
        let got = spans_text(src);
        assert_eq!(got, vec!["\".*x\"".to_owned()]);
    }

    #[test]
    fn array_for_body_reads_caller_literal() {
        // The body runs in the caller frame (`vm.eval_source` in place), so a
        // `regexp $re` reading a *caller* literal resolves to it — the inline
        // lowering shares the caller unit's reaching defs (regression for the
        // fresh-frame-body-unit false-negative).
        let src = "proc p {} {\n  array set a {one foo}\n  set re {a+}\n  array for {k v} a {regexp $re $v}\n}\n";
        let got = spans_text(src);
        assert_eq!(got, vec!["{a+}".to_owned()]);
    }

    #[test]
    fn array_for_loop_var_is_not_a_def_site() {
        // The pattern variable is the loop var `v` (bound per entry), not a body
        // literal — so there is no def-site literal to highlight. Proves the
        // inline lowering binds the loop vars so they shadow any caller scalar.
        let src = "array set a {one foo}\narray for {k v} a {\n  regexp $v $s\n}\n";
        let got = spans_text(src);
        assert!(got.is_empty(), "{got:?}");
    }

    #[test]
    fn missing_array_rejects_iteration_before_pattern_body_entry() {
        // C Tcl 9.0/9.1 rejects a missing array before entering the body;
        // earlier releases do not provide this subcommand. A lexical body
        // must not invent reached pattern reads in either case.
        let src = "array for {k v} a {set re {a+}; regexp $re $v}";
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let profile = tcl_dialect::DialectProfile::find(dialect).unwrap();
            let got = spans_text_dialect(src, profile);
            assert!(got.is_empty(), "{dialect}: {got:?}");
        }
    }

    #[test]
    fn namespace_eval_body_unit_does_not_change_bytecode() {
        // Recording a body unit is analysis-only: the module still emits the
        // `namespace` runtime barrier, and codegen never reads `body_units`, so
        // the presence of a body unit must not perturb compiled output. Assert
        // the body unit is recorded (coverage) but lives outside procedures.
        let registry = CommandRegistry::build_default();
        let src = "namespace eval ::ns { set re \".*x\"\n regexp $re $s }\n";
        let cu = CompilationUnit::build_for(src, &registry, false);
        assert_eq!(cu.body_units.len(), 1, "one namespace-eval body unit");
        assert!(
            cu.body_units
                .keys()
                .all(|k| k.starts_with("::ns::namespace-eval#")),
            "the qname's enclosing namespace must be `::ns` (the block's own \
             target namespace), not global: {:?}",
            cu.body_units.keys().collect::<Vec<_>>()
        );
        // The body unit is not a real procedure (codegen would emit it).
        assert!(cu.procedures.is_empty(), "no procedures materialised");
    }

    fn spans_text_dialect(
        source: &str,
        dialect: &'static tcl_dialect::DialectProfile,
    ) -> Vec<String> {
        let registry = CommandRegistry::build_default().project_for_profile(dialect);
        let cu = CompilationUnit::build_for_profile(source, &registry, false, dialect);
        regex_source_literal_spans(source, &cu, &registry, dialect)
            .into_iter()
            .map(|s| source[s.start() as usize..s.end() as usize].to_owned())
            .collect()
    }

    #[test]
    fn result_is_dialect_invariant() {
        // ARE pattern syntax, `set`, and variable resolution are identical
        // across Tcl 8.4–9.0, so the tracked source literal is the same under
        // every dialect.
        let src = "set re {a+b[0-9]*}\nregexp $re $s\n";
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0"] {
            assert_eq!(
                spans_text_dialect(
                    src,
                    tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile()
                ),
                vec!["{a+b[0-9]*}".to_owned()],
                "dialect {dialect}"
            );
        }
    }

    #[test]
    fn wrong_dialect_option_does_not_misplace_pattern() {
        // The physical-read query must use the selected release's actual
        // option grammar; C8.6 rejects -command before selecting a pattern.
        let src = "set re {x+}\nregsub -command $re $s Y out\n";
        assert_eq!(
            spans_text_dialect(
                src,
                tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile()
            ),
            Vec::<String>::new()
        );
        assert_eq!(
            spans_text_dialect(
                src,
                tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile()
            ),
            vec!["{x+}".to_owned()]
        );
    }

    #[test]
    fn callback_barrier_does_not_promote_a_mixed_pattern_phi() {
        let src =
            "if {$flag} { set re {x+} } else { set re $dynamic }\nregsub -command $re $s Y out\n";
        assert!(
            spans_text(src).is_empty(),
            "a same-invocation callback barrier must not make one literal arm stand for a dynamic phi"
        );
    }

    #[test]
    fn substituted_assign_value_is_not_a_regex_source() {
        for value in ["$dynamic", "x+$dynamic", "[make_pattern]"] {
            let src = format!("set re {value}\nregsub -command $re $s callback out\n");
            assert!(spans_text(&src).is_empty(), "source {src:?}");
        }
    }

    #[test]
    fn regsub_command_tracks_all_literal_word_forms() {
        for (value, expected) in [("{x+}", "{x+}"), ("\"x+\"", "\"x+\""), ("x+", "x+")] {
            let src = format!("set re {value}\nregsub -command $re $s callback out\n");
            assert_eq!(
                spans_text(&src),
                vec![expected.to_owned()],
                "source {src:?}"
            );
        }
    }

    #[test]
    fn bare_literal_span_stops_at_command_separator() {
        let got = spans_text("set re x+; regsub -command $re $s callback out\n");
        assert_eq!(got, vec!["x+".to_owned()]);
        assert_eq!(delimited_word_span(r"x\;y; puts", 0), Span::new(0, 4));
    }

    #[test]
    fn regsub_command_tracks_global_qualified_spelling() {
        let got = spans_text("set re {x+}\n::regsub -command $re $s callback out\n");
        assert_eq!(got, vec!["{x+}".to_owned()]);
    }

    #[test]
    fn regsub_command_tracks_static_command_alias() {
        let src = "interp alias {} myregsub {} regsub\n\
                   set re {x+}\n\
                   myregsub -command $re $s callback out\n";
        assert_eq!(spans_text(src), vec!["{x+}".to_owned()]);
    }

    #[test]
    fn pattern_read_uses_physical_identity_for_qualified_and_quoted_words() {
        for read in ["$::re", "\"$::re\"", "${::re}"] {
            let source = format!("set ::re {{x+}}; regexp {read} x");
            assert_eq!(spans_text(&source), vec!["{x+}"], "{source}");
        }
        let source = "set ::re {x+}; upvar #0 ::re alias; regexp $alias x";
        assert_eq!(spans_text(source), vec!["{x+}"]);
    }

    #[test]
    fn pattern_read_preserves_the_receiver_selected_after_alias_retargeting() {
        let source = "set ::first {x+}; set ::second {y+}; upvar #0 ::first alias; \
                      regexp $alias x; upvar #0 ::second alias; regexp $alias y";
        assert_eq!(spans_text(source), vec!["{x+}", "{y+}"]);
        let opaque = "set ::first {x+}; upvar #0 $receiver alias; regexp $alias x";
        assert_eq!(spans_text(opaque), [] as [String; 0]);
    }

    #[test]
    fn shared_diamond_literal_phi_is_not_rejected_as_cycle() {
        // The seed definition is shared by two nested-diamond arms.  The
        // proof must revisit that same incoming version after the first arm;
        // a global visited set would falsely treat the second visit as a
        // cycle and abstain.
        for invocation in [
            "regexp $re $s",
            "regsub -command $re $s callback out",
            "regsub $re $s replacement out",
        ] {
            let source = format!(
                "proc p {{outer inner s}} {{set re {{seed}}\n\
                 if {{$outer}} {{if {{$inner}} {{set re {{left}}}}}}\n\
                 {invocation}\n}}"
            );
            let mut got = spans_text(&source);
            got.sort();
            assert_eq!(
                got,
                vec!["{left}".to_owned(), "{seed}".to_owned()],
                "nested diamond at {invocation}"
            );
        }
    }

    #[test]
    fn joined_values_that_change_generic_pattern_layout_are_not_tracked() {
        let source = "proc p {choice subject} {\n\
                      set re {seed}\n\
                      if {$choice} {set re {-all}}\n\
                      regsub -command $re $subject callback out\n}";
        assert_eq!(spans_text(source), [] as [String; 0]);
    }

    #[test]
    fn literal_proof_uses_stack_for_cycles_but_revisits_shared_defs() {
        let mut scan = Scan::default();
        scan.literal_source_defs.insert((Symbol(0), 1));
        scan.phi_incoming.insert((Symbol(0), 2), vec![1, 1]);
        scan.phi_incoming.insert((Symbol(0), 3), vec![3, 1]);

        let mut stack = HashSet::new();
        assert!(all_reaching_defs_are_literals(
            &scan,
            Symbol(0),
            2,
            &mut stack
        ));
        stack.clear();
        assert!(!all_reaching_defs_are_literals(
            &scan,
            Symbol(0),
            3,
            &mut stack
        ));
    }
}
