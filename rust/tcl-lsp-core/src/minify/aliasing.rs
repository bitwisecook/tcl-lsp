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

//! The aggressive tier's aliases for repeated commands, arguments and string literals, and keyword abbreviation.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Write as _;

use tcl_compiler::analyser::Scope;
use tcl_lexer::{LexerConfig, Span};
use tcl_registry::Traits;
use tcl_syntax::naming::AuthoredCompactNameSequence;

use super::symbol_map::SymbolMap;
use super::{MinifyEnv, SourceEdit, apply_edits, lexical_keywords, lexical_metadata, original};

// Aggressive-tier aliasing (command / argument / string-literal)

/// Whether `word` is a clause word an `if`/`try` grammar matches by literal
/// value, so it must stay literal — aliasing it to `$var` would break the
/// clause parsing (body/expr index detection checks these by value).  The
/// set is the registry's clause-keyword vocabulary: the highlighted clause
/// keywords (`else` / `elseif` / `on` / `trap` / `finally`) plus the
/// non-highlighted clause noise word (`then`).
fn is_clause_keyword(word: &str) -> bool {
    tcl_registry::traits::clause_keywords_without_command_spec().contains(&word)
        || tcl_registry::traits::clause_noise_keywords().contains(&word)
}

/// Every compacted short name across the symbol map, so aggressive
/// aliases avoid colliding with a (possibly proc-local) compacted
/// name — a `$alias` used in command position must not resolve to a
/// local variable.
pub(super) fn collect_symbol_shorts(sm: &SymbolMap) -> HashSet<String> {
    let mut out = HashSet::new();
    out.extend(sm.procs.values().cloned());
    for m in sm.variables.values() {
        out.extend(m.values().cloned());
    }
    out
}

/// Every variable name the compiler can see in `source`, for seeding
/// the aggressive tier's alias generators: the
/// analyser's per-scope variable tables (recursively), the SSA
/// symbol tables of every function unit (which include names only
/// *read* through name-taking commands, e.g. `[set a]`), and every
/// textual `$name` / `${name}` reference, plus inert UTF-8 name candidates.
/// Conservative observed candidate set; misses remain explicit debts and
/// do not establish source or physical-name absence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum LiveNameInventoryGap {
    VariableNameCoverage,
    CompilationUnit,
    ComplexityGuardedBody,
    SourceSyntaxCoverage,
}

#[derive(Debug)]
pub(super) struct LiveNameInventory {
    pub(super) names: HashSet<String>,
    pub(super) gaps: Vec<LiveNameInventoryGap>,
}

pub(super) fn collect_live_names(
    source: &str,
    input: &tcl_compiler::analyser::ResolvedAnalysisInput,
) -> LiveNameInventory {
    let mut out: HashSet<String> = HashSet::new();

    // Analyser scope tables.
    let analysis = original::analysis_with_input(source, input);
    let mut stack: Vec<&Scope> = vec![&analysis.global_scope];
    while let Some(scope) = stack.pop() {
        out.extend(scope.variables.keys().cloned());
        stack.extend(scope.children.iter());
    }

    // SSA symbol tables (per function unit). Guarded units retain only a
    // deliberately incomplete trivial report; their names are candidates,
    // while their omitted names remain an explicit collision gap.
    let compilation_unit = crate::original_invocation::source_compilation_unit(
        source,
        &analysis,
        input.context_registry().commands(),
    );
    if let Some(cu) = &compilation_unit {
        for fu in cu.all_body_function_units() {
            out.extend(fu.ssa.var_names().iter().cloned());
        }
    }

    // Textual `$name` / `${name}` references.
    let bytes = source.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'$' {
            // Collision reservations may include inert UTF-8 source candidates.
            // This superset grants no configured reference, lookup or edit.
            let start = i + 1;
            let end = tcl_core_types::naming::scan_var_name_end_with(bytes, start, true);
            if let Some(candidate) = source.get(start..end).filter(|name| !name.is_empty()) {
                out.insert(candidate.to_owned());
            }
            if let Ok(Some(reference)) = tcl_lexer::scan_var_ref(bytes, i, input.lexer_config())
                && let Ok(name) = std::str::from_utf8(reference.name)
            {
                out.insert(name.to_owned());
                if reference.index.is_none() {
                    i = reference.next.max(i + 1);
                    continue;
                }
            }
        }
        i += 1;
    }
    let mut gaps = Vec::new();
    let syntax_complete = crate::SourceSyntaxCoverage::capture(source, &analysis)
        .and_then(|coverage| coverage.applicability(source, &analysis))
        == Some(crate::SourceSyntaxCoverageApplicability::Complete);
    if !syntax_complete {
        gaps.push(LiveNameInventoryGap::SourceSyntaxCoverage);
    }
    if !analysis.original_variable_alias_source_is_complete(
        &tcl_lexer::SourceImage::document(source),
        input.lexer_config(),
    ) {
        gaps.push(LiveNameInventoryGap::VariableNameCoverage);
    }
    match &compilation_unit {
        None => gaps.push(LiveNameInventoryGap::CompilationUnit),
        Some(cu) if cu.all_body_function_units().any(|fu| fu.complexity_guarded) => {
            gaps.push(LiveNameInventoryGap::ComplexityGuardedBody);
        }
        Some(_) => {}
    }

    LiveNameInventory { names: out, gaps }
}

/// Configured syntax only; insertion, store, frame and dispatch obligations
/// remain the separate explicit aggressive-authoring policy, never Native grants.
pub(super) struct AuthoredAliasSpelling {
    pub(super) assignment: String,
    pub(super) reference: String,
}

pub(super) fn authored_alias_spelling(
    alias: &str,
    value: &str,
    config: LexerConfig,
) -> Option<AuthoredAliasSpelling> {
    let command = tcl_syntax::word_rules::lexical_unicode_source_word("set", config)?;
    let target = tcl_syntax::word_rules::lexical_unicode_source_word(alias, config)?;
    let literal = tcl_syntax::word_rules::lexical_unicode_source_word(value, config)?;
    let reference = tcl_syntax::naming::scalar_source_reference(alias, config, false)?;
    Some(AuthoredAliasSpelling {
        assignment: format!("{command} {target} {literal}\n"),
        reference,
    })
}

#[derive(Clone, Copy)]
pub(super) enum AliasWordPurpose {
    CommandHead,
    Argument,
}

/// Reborrow original complete words from the existing configured source-phase
/// owner. Observed partial candidates provide no absence or complete inventory.
pub(super) fn source_alias_uses(
    source: &str,
    input: &tcl_compiler::analyser::ResolvedAnalysisInput,
    purpose: AliasWordPurpose,
) -> (Vec<String>, HashMap<String, Vec<usize>>) {
    let mut order = Vec::new();
    let mut uses = HashMap::<String, Vec<usize>>::new();
    let analysis = original::analysis_with_input(source, input);
    let Some(identities) = analysis.retained_command_realm() else {
        return (order, uses);
    };
    let context = input.context_registry();
    let env = MinifyEnv {
        original_source: source,
        analysis: &analysis,
        input,
        dialect: input.unit_profile(),
        config: input.lexer_config(),
        registry: context.commands(),
        context: &context,
        identities,
    };
    let Some(census) = lexical_metadata::command_census(source, env) else {
        return (order, uses);
    };
    let mut seen = HashSet::new();
    for command in census.observed_commands() {
        // Selected descriptor compression heuristic, not nominal lookup.
        // Unavailable metadata is unknown; unknown written commands retain the
        // existing explicit behavior-changing Logical authoring policy.
        let known_byte_compiled = matches!(purpose, AliasWordPurpose::CommandHead)
            && lexical_metadata::with_command_schema(env, command, |schema| {
                schema.semantics.traits.contains(Traits::BYTE_COMPILED)
            }) == Some(true);
        if known_byte_compiled {
            continue;
        }
        for (ordinal, word) in command.words.iter().enumerate() {
            if matches!(purpose, AliasWordPurpose::CommandHead) != (ordinal == 0)
                || word.group().expand
                || word.group().kind != tcl_lexer::WordKind::Bare
            {
                continue;
            }
            let Some(value) = tcl_syntax::word_rules::original_static_word_unicode_value(word)
                .and_then(|bytes| String::from_utf8(bytes).ok())
            else {
                continue;
            };
            let span = word.word_span();
            if value.len() < 3
                || source.get(span.as_range()) != Some(value.as_str())
                || (matches!(purpose, AliasWordPurpose::Argument) && is_clause_keyword(&value))
                || !seen.insert((span.start(), span.end()))
            {
                continue;
            }
            uses.entry(value.clone())
                .or_insert_with(|| {
                    order.push(value);
                    Vec::new()
                })
                .push(span.start() as usize);
        }
    }
    (order, uses)
}

/// Shared cost-benefit + apply for command / argument aliasing:
/// each name used ≥ 2 times becomes `$alias` with a `set alias name`
/// preamble when that saves bytes.  `used` accumulates claimed alias
/// names across phases.
pub(super) fn alias_by_uses(
    source: &str,
    order: &[String],
    uses: &HashMap<String, Vec<usize>>,
    claimed: &mut HashSet<String>,
    config: LexerConfig,
) -> (String, BTreeMap<String, String>) {
    if order.is_empty() {
        return (source.to_owned(), BTreeMap::new());
    }
    let mut cands = order.to_vec();
    cands.sort_by(|a, b| {
        let (ka, kb) = (uses[a].len() * a.len(), uses[b].len() * b.len());
        kb.cmp(&ka).then_with(|| {
            order
                .iter()
                .position(|x| x == a)
                .cmp(&order.iter().position(|x| x == b))
        })
    });
    let mut names = AuthoredCompactNameSequence::new(config);
    let mut aliases: Vec<(String, String, AuthoredAliasSpelling)> = Vec::new();
    for name in &cands {
        let count = uses[name].len();
        if count < 2 {
            continue;
        }
        let Some(alias) = names.next_avoiding_observed(|name| claimed.contains(name), None) else {
            continue;
        };
        let alias = alias.into_name();
        let Some(spelling) = authored_alias_spelling(&alias, name, config) else {
            continue;
        };
        if uses[name].iter().any(|off| {
            off.checked_add(name.len())
                .and_then(|end| source.get(*off..end))
                != Some(name.as_str())
        }) {
            continue;
        }
        let original_cost = count * name.len();
        let aliased_cost = spelling.assignment.len() + count * spelling.reference.len();
        if aliased_cost >= original_cost {
            continue;
        }
        claimed.insert(alias.clone());
        aliases.push((name.clone(), alias, spelling));
    }
    if aliases.is_empty() {
        return (source.to_owned(), BTreeMap::new());
    }
    let mut edits: Vec<SourceEdit> = Vec::new();
    let mut preamble = String::new();
    let mut map = BTreeMap::new();
    for (name, alias, spelling) in &aliases {
        preamble.push_str(&spelling.assignment);
        for &off in &uses[name] {
            edits.push(SourceEdit {
                span: off..off + name.len(),
                text: spelling.reference.clone(),
            });
        }
        map.insert(name.clone(), alias.clone());
    }
    let Ok(body) = apply_edits(source, &edits) else {
        return (source.to_owned(), BTreeMap::new());
    };
    (format!("{preamble}{body}"), map)
}

/// Alias repeated long command names (`HTTP::uri` → `$a`) — the first of the
/// three aliasing passes.
pub(super) fn alias_repeated_commands(
    source: &str,
    input: &tcl_compiler::analyser::ResolvedAnalysisInput,
    claimed: &mut HashSet<String>,
) -> (String, BTreeMap<String, String>) {
    let (order, uses) = source_alias_uses(source, input, AliasWordPurpose::CommandHead);
    alias_by_uses(source, &order, &uses, claimed, input.lexer_config())
}

/// Emit unique-prefix keyword abbreviations, after the aliasing passes.
///
/// Tcl's `Tcl_GetIndexFromObj` accepts any unique prefix of an ensemble
/// subcommand or an `-option`, and `Tcl_GetBoolean` any unique prefix of a
/// boolean word, so `string equal -nocase` can be written `string eq -noc` —
/// a pure length win on top of whitespace stripping, and exactly what
/// hand-minified iRules already do, but done correctly.
///
/// Every abbreviation is computed by the registry
/// ([`tcl_registry::abbrev`]); the minifier never pattern-matches a command
/// name. Two safety rules make the rewrite observationally invisible:
///
/// * **Version-range safety.** A prefix unique today can become ambiguous
///   when a later release adds a keyword (`string cat` in 8.6.2 shortened
///   what `string c…` could mean). The abbreviation is computed against the
///   target dialect's table *and every later core-Tcl table*, so minified
///   output stays correct if it is later run on a newer interpreter.
/// * **Only dispatch-consumed words.** A subcommand or option word is
///   consumed by dispatch and never observable as a string. Boolean *values*
///   are not abbreviated here at all: `set flag true` is a value-definition
///   site whose bytes may be observed (`eq "true"`, a `switch` arm, `string
///   length`), and the minifier has no proof otherwise.
///
/// Abstains on strict tables, dynamic and `{*}`-expanded words, and anything
/// the registry does not resolve `Unique`.
pub(super) fn abbreviate_keywords(
    source: &str,
    input: &tcl_compiler::analyser::ResolvedAnalysisInput,
) -> (String, usize) {
    let analysis = original::analysis_with_input(source, input);
    let Some(identities) = analysis.retained_command_realm() else {
        return (source.to_owned(), 0);
    };
    let context = input.context_registry();
    lexical_keywords::abbreviate(
        source,
        MinifyEnv {
            original_source: source,
            analysis: &analysis,
            input,
            dialect: input.unit_profile(),
            config: input.lexer_config(),
            context: &context,
            registry: context.commands(),
            identities,
        },
    )
}

/// Alias repeated literal arguments (`-normalized` → `$a`) — the second
/// aliasing pass.
pub(super) fn alias_repeated_arguments(
    source: &str,
    input: &tcl_compiler::analyser::ResolvedAnalysisInput,
    claimed: &mut HashSet<String>,
) -> (String, BTreeMap<String, String>) {
    let (order, uses) = source_alias_uses(source, input, AliasWordPurpose::Argument);
    alias_by_uses(source, &order, &uses, claimed, input.lexer_config())
}

/// Build a suffix array for `text` (naive sort of byte suffixes).
fn build_suffix_array(text: &[u8]) -> Vec<usize> {
    let mut sa: Vec<usize> = (0..text.len()).collect();
    sa.sort_by(|&a, &b| text[a..].cmp(&text[b..]));
    sa
}

/// Kasai's LCP array from a suffix array.
fn build_lcp_array(text: &[u8], sa: &[usize]) -> Vec<usize> {
    let n = text.len();
    if n == 0 {
        return Vec::new();
    }
    let mut rank = vec![0usize; n];
    for (i, &s) in sa.iter().enumerate() {
        rank[s] = i;
    }
    let mut lcp = vec![0usize; n];
    let mut k = 0usize;
    for i in 0..n {
        if rank[i] == n - 1 {
            k = 0;
            continue;
        }
        let j = sa[rank[i] + 1];
        while i + k < n && j + k < n && text[i + k] == text[j + k] {
            k += 1;
        }
        lcp[rank[i]] = k;
        k = k.saturating_sub(1);
    }
    lcp
}

/// Find repeated substrings (≥ 3 bytes, ≥ 2 sentinel-free
/// occurrences) across the quoted-string segments of `source`,
/// returning `substring -> source byte offsets`.  Steps 1–5 of
/// `_alias_string_literals`.
fn string_alias_candidates(segments: &[StringLiteralFragment]) -> BTreeMap<Vec<u8>, Vec<usize>> {
    if segments.is_empty() {
        return BTreeMap::new();
    }
    // Concatenate segments with a `\0` sentinel; map concat byte →
    // source byte offset (`usize::MAX` for sentinels).
    let mut concat: Vec<u8> = Vec::new();
    let mut concat_to_src: Vec<usize> = Vec::new();
    for fragment in segments {
        let src_offset = fragment.span.start() as usize;
        for (j, b) in fragment.text.bytes().enumerate() {
            concat.push(b);
            concat_to_src.push(src_offset + j);
        }
        concat.push(0);
        concat_to_src.push(usize::MAX);
    }

    let sa = build_suffix_array(&concat);
    let lcp = build_lcp_array(&concat, &sa);
    let min_len = 3;

    let mut candidates: HashSet<Vec<u8>> = HashSet::new();
    for i in 0..sa.len().saturating_sub(1) {
        if lcp[i] < min_len {
            continue;
        }
        let (p1, p2) = (sa[i], sa[i + 1]);
        if concat_to_src[p1] == usize::MAX || concat_to_src[p2] == usize::MAX {
            continue;
        }
        let mut actual = 0;
        for k in 0..lcp[i] {
            if p1 + k >= concat_to_src.len()
                || p2 + k >= concat_to_src.len()
                || concat_to_src[p1 + k] == usize::MAX
                || concat_to_src[p2 + k] == usize::MAX
            {
                break;
            }
            actual = k + 1;
        }
        if actual >= min_len {
            candidates.insert(concat[p1..p1 + actual].to_vec());
        }
    }

    let mut occ: BTreeMap<Vec<u8>, Vec<usize>> = BTreeMap::new();
    for sub in &candidates {
        let mut offsets = Vec::new();
        let mut start = 0;
        while let Some(idx) = find_subslice(&concat, sub, start) {
            if !(0..sub.len()).any(|k| concat_to_src[idx + k] == usize::MAX) {
                offsets.push(concat_to_src[idx]);
            }
            start = idx + 1;
        }
        if offsets.len() >= 2 {
            occ.insert(sub.clone(), offsets);
        }
    }
    occ
}

/// Alias repeated substrings inside double-quoted strings — the last
/// aliasing pass. (+ `_collect_string_literals`).
pub(super) fn alias_string_literals(
    source: &str,
    input: &tcl_compiler::analyser::ResolvedAnalysisInput,
    claimed: &mut HashSet<String>,
) -> (String, BTreeMap<String, String>) {
    let fragments = collect_string_literals(source, input);
    let occ = string_alias_candidates(&fragments);
    if occ.is_empty() {
        return (source.to_owned(), BTreeMap::new());
    }

    // Step 6: score, then greedily select non-overlapping aliases.
    let mut scored: Vec<(usize, Vec<u8>, Vec<usize>)> = Vec::new();
    for (sub, offsets) in &occ {
        let count = offsets.len();
        let original_cost = count * sub.len();
        let preamble_cost = 4 + 1 + 1 + 1 + sub.len() + 1 + 1;
        let aliased_cost = preamble_cost + count * 2;
        let savings = original_cost.saturating_sub(aliased_cost);
        if savings > 0 {
            scored.push((savings, sub.clone(), offsets.clone()));
        }
    }
    scored.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.len().cmp(&a.1.len())));

    let mut names = AuthoredCompactNameSequence::new(input.lexer_config());
    let mut claimed_bytes: HashSet<usize> = HashSet::new();
    let mut aliases: Vec<(String, String, Vec<SourceEdit>)> = Vec::new();
    for (_, sub, mut offsets) in scored {
        let Ok(sub_string) = std::str::from_utf8(&sub) else {
            continue;
        };
        let preamble_word = format!("{{{sub_string}}}");
        if standalone_alias_word(&preamble_word, input.lexer_config())
            .and_then(|word| tcl_syntax::word_rules::original_static_word_unicode_value(&word))
            .as_deref()
            != Some(sub.as_slice())
        {
            continue;
        }
        offsets.sort_unstable();
        let mut free = Vec::new();
        let mut end = 0;
        for off in offsets {
            if off >= end && !(off..off + sub.len()).any(|p| claimed_bytes.contains(&p)) {
                free.push(off);
                end = off + sub.len();
            }
        }
        if free.len() < 2 {
            continue;
        }
        let Some(alias) = names.next_avoiding_observed(|name| claimed.contains(name), None) else {
            continue;
        };
        let alias = alias.into_name();
        let count = free.len();
        let original_cost = count * sub.len();
        let preamble_cost = 4 + alias.len() + 1 + 1 + sub.len() + 1 + 1;
        let proposals = free
            .iter()
            .map(|&off| {
                let end = off.checked_add(sub.len())?;
                let fragment = fragments.iter().find(|fragment| {
                    fragment.span.start() as usize <= off && end <= fragment.span.end() as usize
                })?;
                let replacement =
                    alias_string_reference(source, fragment, off, end, &alias, sub_string)?;
                Some(SourceEdit {
                    span: off..end,
                    text: replacement,
                })
            })
            .collect::<Option<Vec<_>>>();
        let Some(proposals) = proposals else {
            continue;
        };
        let aliased_cost =
            preamble_cost + proposals.iter().map(|edit| edit.text.len()).sum::<usize>();
        if aliased_cost >= original_cost {
            continue;
        }
        claimed.insert(alias.clone());
        for &off in &free {
            for p in off..off + sub.len() {
                claimed_bytes.insert(p);
            }
        }
        aliases.push((sub_string.to_owned(), alias, proposals));
    }
    if aliases.is_empty() {
        return (source.to_owned(), BTreeMap::new());
    }

    // Step 7: build preamble + edits.
    let mut preamble = String::new();
    let mut edits: Vec<SourceEdit> = Vec::new();
    let mut map = BTreeMap::new();
    for (sub, alias, proposals) in aliases {
        let _ = writeln!(preamble, "set {alias} {{{sub}}}");
        edits.extend(proposals);
        map.insert(sub, alias);
    }
    let Ok(body) = apply_edits(source, &edits) else {
        return (source.to_owned(), BTreeMap::new());
    };
    (format!("{preamble}{body}"), map)
}

pub(super) struct StringLiteralFragment {
    pub(super) span: Span,
    pub(super) text: String,
    pub(super) config: LexerConfig,
    pub(super) following_end: usize,
}

/// Original quoted text components from the selected source-role inventory.
pub(super) fn collect_string_literals(
    source: &str,
    input: &tcl_compiler::analyser::ResolvedAnalysisInput,
) -> Vec<StringLiteralFragment> {
    let analysis = original::analysis_with_input(source, input);
    let Some(identities) = analysis.retained_command_realm() else {
        return Vec::new();
    };
    let context = input.context_registry();
    let env = MinifyEnv {
        original_source: source,
        analysis: &analysis,
        input,
        dialect: input.unit_profile(),
        config: input.lexer_config(),
        registry: context.commands(),
        context: &context,
        identities,
    };
    let Some(census) = lexical_metadata::command_census(source, env) else {
        return Vec::new();
    };
    let mut fragments = Vec::new();
    for command in census.observed_commands() {
        for word in command.words.iter().skip(1) {
            if word.group().kind != tcl_lexer::WordKind::Quoted {
                continue;
            }
            let Ok(word_span) = word.content_span() else {
                continue;
            };
            let arena = word.executable_parts();
            for component in arena.list(arena.root()) {
                let span = component.span;
                if matches!(component.part, tcl_lexer::ExecutablePart::Text(_))
                    && let Some(text) = source.get(span.as_range())
                    && text.len() >= 2
                {
                    fragments.push(StringLiteralFragment {
                        span,
                        text: text.to_owned(),
                        config: word.config(),
                        following_end: word_span.end() as usize,
                    });
                }
            }
        }
    }
    fragments.sort_by_key(|fragment| fragment.span.start());
    fragments.dedup_by_key(|fragment| fragment.span);
    fragments
}

pub(super) fn standalone_alias_word(
    source: &str,
    config: LexerConfig,
) -> Option<tcl_lexer::NativeWord> {
    let image = tcl_lexer::SourceImage::document(source);
    let plan = tcl_lexer::native_script_words_in(
        image,
        Span::new(0, u32::try_from(source.len()).ok()?),
        config,
    )
    .ok()?;
    if plan.fatal_tail.is_some() || plan.commands.len() != 1 {
        return None;
    }
    let mut words = plan.commands.into_iter().next()?.words;
    if words.len() != 1 || words[0].word_span().as_range() != (0..source.len()) {
        return None;
    }
    words.pop()
}

/// Hypothetical value comparison supplies syntax only; alias movement keeps its
/// separate aggressive-authoring admission.
pub(super) fn alias_string_reference(
    source: &str,
    fragment: &StringLiteralFragment,
    start: usize,
    end: usize,
    alias: &str,
    value: &str,
) -> Option<String> {
    let prefix = source.get(fragment.span.start() as usize..start)?;
    let suffix = source.get(end..fragment.span.end() as usize)?;
    if source.get(start..end)? != value {
        return None;
    }
    let config = fragment.config;
    let replacement = [false, true].into_iter().find_map(|braced| {
        let candidate = tcl_syntax::naming::scalar_source_reference(alias, config, braced)?;
        let following = source.get(end..fragment.following_end)?;
        let joined = format!("{candidate}{following}");
        let reference = tcl_lexer::scan_var_ref(joined.as_bytes(), 0, config).ok()??;
        (reference.next == candidate.len()
            && reference.name == alias.as_bytes()
            && reference.index.is_none())
        .then_some(candidate)
    })?;
    let original = standalone_alias_word(&format!("\"{}\"", fragment.text), config)?;
    let original_value = tcl_syntax::word_rules::original_static_word_unicode_value(&original)?;
    let proposed = standalone_alias_word(&format!("\"{prefix}{replacement}{suffix}\""), config)?;
    let mut selected = false;
    let proposed_value = tcl_syntax::word_rules::original_word_unicode_value_with_variables(
        &proposed,
        |arena, part| {
            let tcl_lexer::ExecutablePart::Variable { name, index: None } = part.part else {
                return None;
            };
            if selected
                || arena.bytes(name)? != alias.as_bytes()
                || part.span.as_range() != (1 + prefix.len()..1 + prefix.len() + replacement.len())
            {
                return None;
            }
            selected = true;
            Some(value.as_bytes().to_vec())
        },
    )?;
    (selected && original_value == proposed_value).then_some(replacement)
}

/// First index ≥ `from` where `needle` occurs in `haystack`.
fn find_subslice(haystack: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    if needle.is_empty() || from + needle.len() > haystack.len() {
        return None;
    }
    (from..=haystack.len() - needle.len()).find(|&i| &haystack[i..i + needle.len()] == needle)
}
