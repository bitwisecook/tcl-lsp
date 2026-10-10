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

//! `textDocument/declaration` — jump to a variable's `global` /
//! `variable` / `upvar` *declaration* site.
//!
//! Go-to-declaration differs from go-to-definition: for a
//! `$var` reference it looks for the scoping statement that *declares*
//! the name in a visible scope (`global x`, `variable x ?val?`,
//! `upvar ?lvl? other x`, `namespace upvar ns other x`), respecting
//! lexical visibility.  When the cursor is not on a variable, or no
//! declaration is found, it falls back to plain go-to-definition.
//!
//! Declared names and body positions come from the shared original
//! declaration-layout owner, with retained dialect, alias prefix and source
//! origins. This navigation projection supplies no executed alias facts.
//! Visibility is the set of
//! enclosing scope body spans (`definition::scope_body_spans_at`); an
//! empty set means the whole file is visible (cursor at global scope).

use tcl_compiler::analyser::AnalysisResult;
use tcl_compiler::segmenter::segment_commands_with_offset_and_config;
use tcl_lexer::{LexerConfig, LineIndex, Span};
use tcl_registry::CommandRegistry;

/// Recursion depth guard for nested body walks — a defensive stack-overflow
/// bound, kept at or above the compiler analyser's own `MAX_BODY_DEPTH` so
/// deeply (but validly) nested code keeps full go-to-declaration support.
/// Real source never nests anywhere near this.
///
/// The compiler derives its own number from a stack budget and lands well
/// below this one, so a body tree this walk receives from the analyser
/// cannot reach here at all. The bound is defence-in-depth against a tree
/// built or received some other way — the same role
/// `document_symbols`' `MAX_SCOPE_WALK_DEPTH` plays.
const MAX_BODY_DEPTH: tcl_core_types::RecursionLimit = tcl_core_types::RecursionLimit(256);

use crate::definition::{LspRange, byte_offset_at, definition, scope_body_spans_at, span_to_range};

/// Compute "go-to-declaration" locations for the symbol at the cursor.
#[must_use]
pub fn declaration(
    source: &str,
    line: u32,
    character: u32,
    dialect: &'static tcl_dialect::DialectProfile,
    analysis: &AnalysisResult,
    registry: &CommandRegistry,
) -> Vec<LspRange> {
    if let std::ops::ControlFlow::Break(selected) =
        crate::variable_symbol::select(source, analysis, line, character)
    {
        let Some(selected) = selected else {
            return vec![];
        };
        if let Some(span) = selected.original_alias_declaration_span() {
            return vec![span_to_range(source, &LineIndex::new(source), span)];
        }
        return definition(source, line, character, analysis);
    }
    // Original navigation consumes the sealed variable/caller templates in
    // definition. Missing original roots cannot select a fresh lexical scan.
    if !analysis.allows_lexical_declaration_advice() {
        return definition(source, line, character, analysis);
    }
    // Only a `$var` reference has a distinct declaration site; every
    // other symbol resolves through plain go-to-definition.
    let line_index = LineIndex::new(source);
    let cursor = byte_offset_at(&line_index, source, line, character);
    let Some(var_name) =
        crate::definition::substituting_var_at_position(source, analysis, line, character, cursor)
    else {
        return definition(source, line, character, analysis);
    };
    // The selected source root is already undecorated. Dollar bytes inside
    // a braced scalar name are literal and must never be stripped again.
    let target = var_name.strip_prefix("::").unwrap_or(&var_name);

    let visible = scope_body_spans_at(&analysis.global_scope, cursor);

    // Regions to scan: each visible scope body, or the whole file when
    // the cursor sits at global scope (no enclosing body span).
    let regions: Vec<Span> = if visible.is_empty() {
        vec![Span::new(0, u32::try_from(source.len()).unwrap_or(0))]
    } else {
        visible.clone()
    };

    // The document's proven command-identity facts, so a scoping statement is
    // recognised by the command a head *is* rather than the one it is spelled
    // as.  Empty — and lookup-free — unless the document binds
    // something.
    let Some(config) = analysis.body_lexer_config else {
        return Vec::new();
    };
    let Some(identities) = analysis.retained_command_realm() else {
        return Vec::new();
    };
    let Some(actual_registry) = analysis.resolved_registry() else {
        return Vec::new();
    };
    if registry.snapshot().semantic_key() != actual_registry.snapshot().semantic_key()
        || analysis
            .resolved_profile()
            .is_none_or(|actual| !std::ptr::eq(actual, dialect))
    {
        return Vec::new();
    }
    let scan = DeclScan {
        source,
        config,
        target,
        visible: &visible,
        registry: actual_registry,
        identities,
        cursor,
    };
    let mut found = DeclSpans::default();
    for region in &regions {
        collect_declarations_in_region(&scan, *region, 0, &mut found);
    }

    if found.spans.is_empty() {
        // No explicit declaration — fall back to the definition site.
        return definition(source, line, character, analysis);
    }
    let mut spans = found.spans;
    spans.sort_by_key(|s| s.start());
    spans
        .into_iter()
        .map(|s| span_to_range(source, &line_index, s))
        .collect()
}

/// Accumulator for declaration spans, de-duplicated by `(start, end)`.
#[derive(Default)]
struct DeclSpans {
    spans: Vec<Span>,
    seen: std::collections::HashSet<(u32, u32)>,
}

impl DeclSpans {
    /// Record `span` unless an identical one was already collected.
    fn push(&mut self, span: Span) {
        if self.seen.insert((span.start(), span.end())) {
            self.spans.push(span);
        }
    }
}

/// The constant context for a declaration scan, threaded through the
/// body-recursion so the per-call signature stays small.
struct DeclScan<'a> {
    source: &'a str,
    config: LexerConfig,
    target: &'a str,
    visible: &'a [Span],
    registry: &'a CommandRegistry,
    /// The document's statically proven command-identity facts
    /// ([`tcl_compiler::realm`]): which registry command each head
    /// spelling really names at each point in the file.  A head whose binding
    /// was provably taken over resolves to nothing, so no scope-alias grammar
    /// and no body recursion is applied to it.
    identities: &'a tcl_compiler::realm::CommandBindingRealm,
    /// The cursor's byte offset. The analyser's scope tree does not model an
    /// `apply` lambda body as its own scope (it has no `proc`/`namespace
    /// eval`-style boundary), so `visible` never reflects it; recursing into
    /// a lambda's body is instead gated directly on whether the cursor sits
    /// inside that specific lambda's span (see the `LambdaLiteral` arm of
    /// [`collect_declarations_in_region`]).
    cursor: u32,
}

/// Scan the commands of `region` for `global` / `variable` / `upvar` /
/// `namespace upvar` statements declaring `scan.target`, recording each
/// matching declaration token span (when visible) into `found`, and
/// recursing into body-role arguments (`proc` / `if` / `foreach` /
/// `while` / `catch` / `namespace eval` … bodies) so declarations nested
/// in control-flow blocks are reached.
fn collect_declarations_in_region(
    scan: &DeclScan<'_>,
    region: Span,
    depth: u32,
    found: &mut DeclSpans,
) {
    if MAX_BODY_DEPTH.exceeded(depth) {
        return;
    }
    let (source, config, target, visible, registry) = (
        scan.source,
        scan.config,
        scan.target,
        scan.visible,
        scan.registry,
    );
    let identities = scan.identities;
    let (mut start, mut end) = (region.start() as usize, region.end() as usize);
    if start >= end || end > source.len() {
        return;
    }
    // A scope `body_span` keeps its delimiters (`proc p {} { … }`'s
    // span starts on the `{`), so segmenting from `start` would parse
    // the whole body as one braced word.  Peel a leading `{` / `"` /
    // `[` and the matching trailing delimiter so the body's own
    // commands are segmented — mirrors `references.rs`'s `[`/`]` strip.
    let bytes = source.as_bytes();
    if matches!(bytes.get(start), Some(b'{' | b'"' | b'[')) {
        start += 1;
    }
    if end > start && matches!(bytes.get(end - 1), Some(b'}' | b'"' | b']')) {
        end -= 1;
    }
    if start >= end {
        return;
    }
    let commands = segment_commands_with_offset_and_config(
        &source[start..end],
        u32::try_from(start).unwrap_or(0),
        config,
    );
    for cmd in &commands {
        if cmd.argv.is_empty() {
            continue;
        }
        // The retained original declaration frame selects unanimous grammar
        // and maps effective alias arguments to their written source tokens.
        let mut tokens = tcl_compiler::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::new(source),
            config,
            cmd,
        );
        identities.stamp_original_tokens(&mut tokens);
        let Some(assistance) = identities.original_declaration_assistance(&tokens, registry) else {
            continue;
        };
        // The owner returns positions among the original post-head arguments.
        let arg_tokens = &cmd.argv[1..];
        // A scoping statement records its declared-name token spans …
        record_declaration_tokens(&assistance.declarations, arg_tokens, target, visible, found);

        // … and *every* command may carry body-role arguments to recurse
        // into (control-flow blocks, `namespace eval`, `catch`, …).
        for body_idx in assistance.body_arguments {
            if let Some(body_tok) = arg_tokens.get(body_idx) {
                collect_declarations_in_region(scan, body_tok.span, depth + 1, found);
            }
        }

        // `apply {argList body ?ns?} …` (and any future command sharing the
        // shape) — recurse into the real body *element*, not the whole
        // lambda literal: re-segmenting the whole `{argList}
        // {body}` blob as a script misread the parameter word as a command
        // name, so a `global` / `variable` / `upvar` genuinely inside the
        // body was never reached (and, worse, a parameter that happened to
        // be *named* `global`/`variable`/`upvar` could misfire as a bogus
        // declaration). `split_lambda_literal`'s span is already
        // delimiter-stripped, so it can be handed to
        // `collect_declarations_in_region` directly — its own peel step is a
        // no-op on a region that has nothing left to peel.
        //
        // Unlike an `if`/`foreach`/`catch` body, an `apply` body runs in a
        // *fresh* call frame — a `global`/`variable`/`upvar` declared inside
        // it is scoped to that frame alone, never to the frame containing
        // this `apply` call. Descending unconditionally would make such a
        // declaration "visible" to a cursor sitting *outside* the lambda too
        // (`apply {{} {global x}}` inside a proc, followed by an unrelated
        // `puts $x` in the same proc, must not resolve `$x`'s declaration
        // into the lambda). So recurse only
        // when the cursor itself is positioned inside this specific lambda's
        // body — the analyser's scope tree has no `apply`-body scope kind for
        // `scan.visible` to reflect, so the direct span/offset containment
        // check stands in for "the cursor's scope chain includes this frame".
        for lambda_idx in assistance.lambda_arguments {
            if let Some(body_span) = original_lambda_body_at(source, cmd, lambda_idx, config)
                && body_span.start() <= scan.cursor
                && scan.cursor <= body_span.end()
            {
                collect_declarations_in_region(scan, body_span, depth + 1, found);
            }
        }
    }
}

fn original_lambda_body_at(
    source: &str,
    command: &tcl_compiler::segmenter::SegmentedCommand,
    argument: usize,
    config: LexerConfig,
) -> Option<Span> {
    let plan = tcl_lexer::native_script_words_in(
        tcl_lexer::SourceImage::document(source),
        command.execution_span(source),
        config,
    )
    .ok()?;
    if plan.fatal_tail.is_some() || plan.commands.len() != 1 {
        return None;
    }
    let word = plan.commands.first()?.words.get(argument.checked_add(1)?)?;
    tcl_compiler::lambda_literal::split_original_lambda_literal(word)?.braced_body()
}

/// Record the visible declaration-name token spans for a single
/// scope-alias command (`global` / `variable` / `upvar` /
/// `namespace upvar` / `my variable`) into `found`.  Recognition and the
/// per-form argument grammar come from the shared original declaration
/// assistance, including alias prefixes and selected member positions.
fn record_declaration_tokens(
    declarations: &[tcl_compiler::registry_invocation::DeclarationArgument],
    arg_tokens: &[tcl_lexer::Token],
    target: &str,
    visible: &[Span],
    found: &mut DeclSpans,
) {
    for declaration in declarations {
        let Some(tok) = arg_tokens.get(declaration.argument) else {
            continue;
        };
        if declaration.name != target {
            continue;
        }
        if !is_visible(tok.span, visible) {
            continue;
        }
        found.push(tok.span);
    }
}

/// A declaration token is visible when it lies within any enclosing
/// scope body span; an empty `visible` set (global scope) admits all.
fn is_visible(span: Span, visible: &[Span]) -> bool {
    visible.is_empty()
        || visible
            .iter()
            .any(|v| v.start() <= span.start() && span.end() <= v.end())
}

/// Explicit standalone decoration compatibility controls. Actual cursor roots
/// are already selected labels and do not pass through this utility.
#[cfg(test)]
fn bare_name(raw: &str) -> &str {
    let s = raw.trim();
    let s = if s.starts_with('$') {
        // An incomplete `${x` remains decorated and cannot alias `x`.
        tcl_syntax::naming::var_reference(s)
    } else {
        s
    };
    // Declaration arguments may carry an absolute root marker; preserve the
    // remainder verbatim, including interior colon-runs such as `a:::b`.
    s.strip_prefix("::").unwrap_or(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    fn analyse(source: &str) -> AnalysisResult {
        let mut a = Analyser::new();
        a.analyse(source, "tcl8.6").clone()
    }

    fn reg() -> CommandRegistry {
        CommandRegistry::build_default()
    }

    fn pos_of(src: &str, needle: &str, occurrence: usize) -> (u32, u32) {
        let mut start = 0;
        for _ in 0..occurrence {
            let idx = src[start..].find(needle).expect("needle not found") + start;
            start = idx + 1;
        }
        let idx = start - 1;
        let prefix = &src[..idx];
        let line = u32::try_from(prefix.matches('\n').count()).unwrap();
        let col = u32::try_from(idx - prefix.rfind('\n').map_or(0, |n| n + 1)).unwrap();
        (line, col)
    }

    #[test]
    fn original_declaration_navigation_declines_unowned_analysis_without_recapture() {
        // Implementation contract: naming.core.original-declaration-navigation
        // docs/design/analysis/name-resolution-proofs/core-original-declaration-navigation.md
        let source = "set x 1; proc p {} {global x; puts $x}";
        let analysis = analyse(source);
        let (line, character) = pos_of(source, "$x", 1);
        let profile = analysis.resolved_profile().unwrap();
        let registry = analysis.resolved_registry().unwrap();
        let selected = declaration(source, line, character + 1, profile, &analysis, registry);
        assert_eq!(selected.len(), 1);
        let alias = source.find("global x").unwrap() + "global ".len();
        assert_eq!(selected[0].start_character, u32::try_from(alias).unwrap());
        let mut unowned = AnalysisResult::default();
        unowned.body_lexer_config = analysis.body_lexer_config;
        unowned.resolved_input = analysis.resolved_input.clone();
        unowned.global_scope = analysis.global_scope.clone();
        assert!(!unowned.allows_lexical_declaration_advice());
        assert!(unowned.retained_command_realm().is_none());
        assert!(declaration(source, line, character + 1, profile, &unowned, registry).is_empty());
        assert!(
            declaration(
                &format!("#{source}"),
                line,
                character + 1,
                profile,
                &analysis,
                registry
            )
            .is_empty()
        );
    }

    #[test]
    fn logical_declaration_navigation_keeps_its_explicit_compatibility_scan() {
        // Implementation contract: naming.core.original-declaration-navigation
        // docs/design/analysis/name-resolution-proofs/core-original-declaration-navigation.md
        let source = "proc p {} {global x; puts $x}";
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let config = LexerConfig::for_file_grammar(profile.grammar);
        let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            tcl_registry::model::ingress::context_for_profile(profile),
            config,
        );
        let analysis = Analyser::new()
            .with_resolved_input(input)
            .analyse(source, profile.name);
        assert!(analysis.allows_lexical_declaration_advice());
        let (line, character) = pos_of(source, "$x", 1);
        let selected = declaration(
            source,
            line,
            character + 1,
            profile,
            &analysis,
            analysis.resolved_registry().unwrap(),
        );
        assert_eq!(selected.len(), 1);
        let alias = source.find("global x").unwrap() + "global ".len();
        assert_eq!(selected[0].start_character, u32::try_from(alias).unwrap());
    }

    #[test]
    fn global_declaration_in_proc_body() {
        let src = "proc bump {} {\n\
                   global counter\n\
                   incr counter\n\
                   puts $counter\n\
                   }\n";
        let analysis = analyse(src);
        // Cursor on the `$counter` reference in the proc body.
        let (l, c) = pos_of(src, "$counter", 1);
        let locs = declaration(
            src,
            l,
            c + 1,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            &analysis,
            &reg(),
        );
        assert_eq!(locs.len(), 1, "{locs:?}");
        // The `global counter` declaration is on line 1.
        assert_eq!(locs[0].start_line, 1);
    }

    #[test]
    fn variable_declaration_in_namespace() {
        let src = "namespace eval ns {\n\
                   variable config 1\n\
                   proc get {} {\n\
                   variable config\n\
                   return $config\n\
                   }\n\
                   }\n";
        let analysis = analyse(src);
        // Cursor on `$config` inside `get`.
        let (l, c) = pos_of(src, "$config", 1);
        let locs = declaration(
            src,
            l,
            c + 1,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            &analysis,
            &reg(),
        );
        // The `variable config` inside the proc body is the visible
        // declaration (the namespace-level one may also be visible).
        assert!(!locs.is_empty(), "{locs:?}");
    }

    #[test]
    fn upvar_local_alias_is_a_declaration() {
        // The caller side must be a static name (not a `$`-substituted
        // reference) for the upvar grammar helper to treat the pair as a
        // resolvable declaration — same gate the memory-SSA pass uses.
        let src = "proc wrap {} {\n\
                   upvar 1 other local\n\
                   return $local\n\
                   }\n";
        let analysis = analyse(src);
        let (l, c) = pos_of(src, "$local", 1);
        let locs = declaration(
            src,
            l,
            c + 1,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            &analysis,
            &reg(),
        );
        assert_eq!(locs.len(), 1, "{locs:?}");
        // `upvar 1 other local` declares `local` on line 1.
        assert_eq!(locs[0].start_line, 1);
    }

    #[test]
    fn global_declared_inside_control_flow_body_is_found() {
        // A `global` nested in an `if` body (no scope of its own) must be
        // reached via body recursion, not just the top-level proc scan.
        let src = "proc p {} {\n\
                   if {[info exists x]} { global x }\n\
                   puts $x\n\
                   }\n";
        let analysis = analyse(src);
        let (l, c) = pos_of(src, "$x", 1);
        let locs = declaration(
            src,
            l,
            c + 1,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            &analysis,
            &reg(),
        );
        assert_eq!(locs.len(), 1, "{locs:?}");
        // `global x` is on line 1 (the `if` line).
        assert_eq!(locs[0].start_line, 1);
    }

    #[test]
    fn global_declared_inside_apply_lambda_body_is_found() {
        // `apply`'s lambda-literal argument is
        // `ArgRole::LambdaLiteral`, not `Body` — recursing the whole
        // `{argList} {body}` blob as a script would misread the parameter
        // word as a command name and never reach a `global` declared
        // *inside* the real body.
        let src = "apply {dir {\n\
                   global x\n\
                   puts $x\n\
                   }} /tmp\n";
        let analysis = analyse(src);
        let (l, c) = pos_of(src, "$x", 1);
        let locs = declaration(
            src,
            l,
            c + 1,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            &analysis,
            &reg(),
        );
        assert_eq!(locs.len(), 1, "{locs:?}");
        // `global x` is on line 1 (inside the lambda body).
        assert_eq!(locs[0].start_line, 1);
    }

    /// An `apply` body runs in a *fresh*
    /// call frame, so a `global x` declared inside it must not leak out as a
    /// visible declaration for an unrelated `$x` reference sitting outside
    /// the lambda in the enclosing proc.
    #[test]
    fn global_declared_inside_apply_lambda_body_is_not_visible_outside_it() {
        let src = "proc p {} {\n\
                   apply {{} {global x}}\n\
                   puts $x\n\
                   }\n";
        let analysis = analyse(src);
        let (l, c) = pos_of(src, "$x", 1);
        let locs = declaration(
            src,
            l,
            c + 1,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            &analysis,
            &reg(),
        );
        assert!(
            !locs.iter().any(|loc| loc.start_line == 1),
            "the lambda's `global x` (line 1) must not resolve as `p`'s own \
             declaration site for the unrelated `$x` on line 2; got {locs:?}"
        );
    }

    #[test]
    fn namespace_variable_declaration_does_not_supply_a_global_bare_read() {
        let src = "namespace eval ns {\n\
                   variable cfg 1\n\
                   }\n\
                   puts $cfg\n";
        let analysis = analyse(src);
        let (l, c) = pos_of(src, "$cfg", 1);
        let locs = declaration(
            src,
            l,
            c + 1,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            &analysis,
            &reg(),
        );
        assert!(
            locs.is_empty(),
            "the namespace cfg declaration cannot supply global cfg: {locs:?}"
        );
    }

    #[test]
    fn original_alias_declaration_navigation_keeps_the_independent_local_operand() {
        let src = r"set ::v\uD800 VALUE
proc p {} {upvar #0 ::v\uD800 local; puts $local}";
        for profile in ["tcl8.6", "tcl9.0", "jimtcl"] {
            let mut analysis = tcl_compiler::analyser::Analyser::new().analyse(src, profile);
            analysis.global_scope.variables.clear();
            for scope in &mut analysis.global_scope.children {
                scope.variables.clear();
            }
            let dialect = tcl_registry::model::ingress::resolve_known_environment(profile)
                .unwrap()
                .unit_profile();
            let (line, character) = pos_of(src, "$local", 1);
            let actual = declaration(src, line, character + 1, dialect, &analysis, &reg());
            let start = u32::try_from(src.find("local;").unwrap()).unwrap();
            assert_eq!(
                actual,
                vec![span_to_range(
                    src,
                    &LineIndex::new(src),
                    Span::new(start, start + 5)
                )],
                "{profile}"
            );
            assert!(
                declaration(
                    &src.replace("VALUE", "OTHER"),
                    line,
                    character + 1,
                    dialect,
                    &analysis,
                    &reg(),
                )
                .is_empty(),
                "stale consumer source cannot borrow the alias declaration"
            );
        }
    }

    #[test]
    fn non_variable_falls_back_to_definition() {
        let src = "proc greet {} {}\ngreet\n";
        let analysis = analyse(src);
        // Cursor on the `greet` call — not a variable, so this defers
        // to go-to-definition (the proc name span on line 0).
        let locs = declaration(
            src,
            1,
            2,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            &analysis,
            &reg(),
        );
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(locs[0].start_line, 0);
    }

    #[test]
    fn declaration_var_decoration_vectors_preserve_incomplete_braces() {
        assert_eq!(bare_name("${x}"), "x");
        assert_eq!(bare_name("$x"), "x");
        assert_eq!(bare_name("$arr(idx)"), "arr(idx)");
        assert_eq!(bare_name("$::a:::b"), "a:::b");
        assert_eq!(bare_name("$foo:::"), "foo:::");
        assert_eq!(bare_name("${x"), "{x");
    }

    #[test]
    fn declaration_does_not_resolve_an_unclosed_braced_reference() {
        let src = "proc p {} {\n    global x\n    puts ${x\n}\n";
        let analysis = analyse(src);
        let locs = declaration(
            src,
            2,
            9,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            &analysis,
            &reg(),
        );
        assert!(
            locs.is_empty(),
            "malformed `${{x` must not resolve to the real `x`: {locs:?}"
        );
    }

    /// The declaration scan must resolve a command head's *effective
    /// identity*, not its written spelling.
    ///
    /// tclsh oracle (8.6.16 and 9.0.4, byte-identical): `interp alias {} decl
    /// {} upvar` makes `decl 1 other local` alias the caller's variable;
    /// `rename upvar decl` does the same and leaves `upvar` gone; `proc upvar
    /// …` takes the name over so the built-in's grammar no longer applies.
    ///
    /// The probe drives [`collect_declarations_in_region`] rather than the
    /// public [`declaration`] entry point.
    /// When the scan finds nothing, `declaration` falls back to plain
    /// go-to-definition, which reads the **analyser's** own scope model: a
    /// separate consumer that still resolves scope aliases by spelling, and
    /// which therefore answers with the identical span and would mask every
    /// abstention this test exists to pin.
    fn scanned_declaration_lines(src: &str, target: &str) -> Vec<u32> {
        let registry = reg();
        let identities = tcl_compiler::realm::document_realm_bindings(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            &registry,
        );
        let end = u32::try_from(src.len()).unwrap_or(0);
        let scan = DeclScan {
            source: src,
            config: LexerConfig::for_profile(
                tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            ),
            target,
            visible: &[],
            registry: &registry,
            identities: &identities,
            cursor: end,
        };
        let mut found = DeclSpans::default();
        collect_declarations_in_region(&scan, Span::new(0, end), 0, &mut found);
        let line_index = LineIndex::new(src);
        let mut lines: Vec<u32> = found
            .spans
            .iter()
            .map(|s| line_index.line_at(s.start()))
            .collect();
        lines.sort_unstable();
        lines
    }

    /// `proc wrap` whose body declares `local` through `declarator`, preceded
    /// by `prelude`.  The declarator always lands on line `prelude_lines + 1`.
    fn upvar_body(prelude: &str, declarator: &str) -> String {
        format!(
            "{prelude}proc wrap {{}} {{\n\
             {declarator} 1 other local\n\
             return $local\n\
             }}\n"
        )
    }

    #[test]
    fn declaration_scan_follows_an_aliased_scoping_command() {
        let src = upvar_body("interp alias {} decl {} upvar\n", "decl");
        assert_eq!(scanned_declaration_lines(&src, "local"), vec![2]);
        // The `::`-qualified spelling of the alias classifies alike.
        let src = upvar_body("interp alias {} decl {} upvar\n", "::decl");
        assert_eq!(scanned_declaration_lines(&src, "local"), vec![2]);
        // Guard: an unbound `decl` declares nothing.
        let src = upvar_body("set y 1\n", "decl");
        assert!(scanned_declaration_lines(&src, "local").is_empty());
    }

    #[test]
    fn declaration_scan_maps_alias_prefixes_to_original_local_operands() {
        let src = upvar_body("interp alias {} decl {} upvar 1 other\n", "decl");
        // The captured prefix supplies the level and target. Only the written
        // local argument can be a source declaration position.
        let src = src.replace("decl 1 other local", "decl local");
        assert_eq!(scanned_declaration_lines(&src, "local"), vec![2]);
        assert!(scanned_declaration_lines(&src, "other").is_empty());
    }

    #[test]
    fn declaration_scan_maps_a_prefixed_body_without_reparsing_alias_argv() {
        let src = "interp alias {} body {} if 1\nproc f {} {\nbody {global x}\nputs $x\n}\n";
        assert_eq!(scanned_declaration_lines(src, "x"), vec![2]);
    }

    #[test]
    fn declaration_scan_uses_the_declared_local_tail_of_a_global_name() {
        let src = "namespace eval ::n {variable x 1}\nproc f {} {\nglobal ::n::x\nputs $x\n}\n";
        assert_eq!(scanned_declaration_lines(src, "x"), vec![2]);
        assert!(scanned_declaration_lines(src, "n::x").is_empty());
    }

    #[test]
    fn declaration_scan_follows_a_renamed_scoping_command() {
        let src = upvar_body("rename upvar decl\n", "decl");
        assert_eq!(scanned_declaration_lines(&src, "local"), vec![2]);
        // The old spelling is gone from the rename onwards.
        let src = upvar_body("rename upvar decl\n", "upvar");
        assert!(
            scanned_declaration_lines(&src, "local").is_empty(),
            "a renamed-away `upvar` must not keep the built-in's grammar"
        );
    }

    #[test]
    fn declaration_scan_abstains_for_a_builtin_shadowed_by_a_user_proc() {
        let src = upvar_body("proc upvar {args} { return 1 }\n", "upvar");
        assert!(
            scanned_declaration_lines(&src, "local").is_empty(),
            "a user `proc upvar` takes the name over; no scope-alias grammar applies"
        );
        // Guard: the unshadowed built-in still declares.
        let src = upvar_body("set y 1\n", "upvar");
        assert_eq!(scanned_declaration_lines(&src, "local"), vec![2]);
    }

    #[test]
    fn declaration_scan_keeps_conditional_lexical_advice_after_a_dynamic_binding() {
        let src = upvar_body("rename $old decl\n", "decl");
        assert!(
            scanned_declaration_lines(&src, "local").is_empty(),
            "a dynamic rename must not make `decl` a scoping statement"
        );
        let src = upvar_body("rename $old decl\n", "upvar");
        assert_eq!(
            scanned_declaration_lines(&src, "local"),
            vec![2],
            "retained original upvar grammar supplies conditional lexical navigation only"
        );
    }

    #[test]
    fn conditional_declaration_preview_declines_explicit_body_replacements() {
        for replacement in ["rename upvar decl", "proc upvar args {}"] {
            let src = format!(
                "rename $old decl\nproc wrap {{}} {{\n{replacement}\nupvar 1 other local\nreturn $local\n}}\n"
            );
            assert!(scanned_declaration_lines(&src, "local").is_empty());
        }
        let src = upvar_body("rename $old decl\nproc proc args {}\n", "upvar");
        assert!(scanned_declaration_lines(&src, "local").is_empty());
    }

    /// The body recursion is registry-driven too: a `global` buried in an
    /// aliased control-flow body is only reached if the alias resolves to the
    /// command whose argument carries `ArgRole::Body`.
    #[test]
    fn declaration_scan_recurses_through_an_aliased_body_command() {
        let src = "interp alias {} maybe {} if\n\
                   proc p {} {\n\
                   maybe {[info exists x]} { global x }\n\
                   puts $x\n\
                   }\n";
        assert_eq!(scanned_declaration_lines(src, "x"), vec![2]);
        // Guard: an unbound `maybe` carries no body argument to recurse into.
        let src = "set y 1\n\
                   proc p {} {\n\
                   maybe {[info exists x]} { global x }\n\
                   puts $x\n\
                   }\n";
        assert!(scanned_declaration_lines(src, "x").is_empty());
    }
}
