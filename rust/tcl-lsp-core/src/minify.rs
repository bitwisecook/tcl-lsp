//! Tcl code minifier — Rust port of `core/minifier/minifier.py`.
//!
//! Pure function: source in, minified source out.  The **default
//! tier** ([`minify_tcl`]) is complete and preserves semantic
//! equivalence by:
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
//! 8. Minifying `switch` braced case-list bodies individually.
//! 9. Deduplicating repeated dynamic templates (`[subst $alias]`).
//! 10. Abbreviating ensemble subcommands for fixed-ensemble
//!     dialects (`f5-irules` / `f5-iapps` / `f5-bigip`).
//!
//! Note: the expression tokeniser adds a catch-all so no character
//! is dropped — the Python reference's `_EXPR_TOKEN` regex silently
//! drops unmatched characters (e.g. commas in `atan2($a,$b)` and
//! braces in `$x ni {a b}`), corrupting those expressions; this
//! port preserves them.
//!
//! The **`compact_names` tier** ([`minify_tcl_compact`]) renames
//! proc-local variables, parameters, and proc names to short
//! identifiers and returns a [`SymbolMap`].  It relies on the
//! analyser tracking `$var` references inside `[…]` command
//! substitutions and braced `expr` bodies (added alongside this
//! tier) so a rename never rewrites a declaration without its body
//! references.  Scopes containing a dynamic-barrier command (e.g.
//! `upvar`) are left untouched; `isolated` also compacts the global
//! scope.  Static array-member keys (`arr(member)`) are compacted
//! too, skipping arrays whose members look user-input-derived.
//!
//! The **`aggressive` tier** ([`minify_tcl_aggressive`]) applies the
//! compiler's optimiser rewrites, then compacts names, then minifies
//! whitespace, returning a [`MinifyResult`] with the optimisation
//! count and size savings.
//!
//! **Still deferred** within the aggressive tier: static-substring
//! folding (SCCP-based, Python's phase 1.5) and the command /
//! argument / string-literal aliasing phases (2.5–2.7).  Also
//! pending: the `workspace/executeCommand` LSP wiring (mirroring
//! `lsp/commands.py::on_minify_document`).

use std::collections::{BTreeMap, HashSet};

use tcl_compiler::analyser::{Analyser, AnalysisResult, ProcDef, Scope, ScopeKind};
use tcl_compiler::expr_ast::render_expr;
use tcl_compiler::{parse_expr, BinOp, ExprNode, UnaryOp};
use tcl_lexer::{Lexer, SourceMap, Span, Token, TokenType};
use tcl_registry::{ArgRole, CommandRegistry, Traits};

/// One argument accumulated while parsing a command.
struct Arg {
    tokens: Vec<Token>,
    is_braced: bool,
    is_quoted: bool,
}

/// Map of original names to compacted names, grouped by scope.
/// Mirrors Python's `SymbolMap` (the fields the landed tiers
/// populate).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SymbolMap {
    /// Per-scope `{original_var: short}` maps, keyed by scope label.
    pub variables: BTreeMap<String, BTreeMap<String, String>>,
    /// `{original_proc: short}`.
    pub procs: BTreeMap<String, String>,
    /// Per-array `{original_member: short}` maps.
    pub array_members: BTreeMap<String, BTreeMap<String, String>>,
}

impl SymbolMap {
    /// Human-readable symbol map.  Mirrors `SymbolMap.format`.
    #[must_use]
    pub fn format(&self) -> String {
        let mut lines: Vec<String> = Vec::new();
        if !self.procs.is_empty() {
            lines.push("# Procs".to_owned());
            for (original, short) in &self.procs {
                lines.push(format!("  {short} <- {original}"));
            }
        }
        for (scope_name, var_map) in &self.variables {
            lines.push(format!("# Variables in {scope_name}"));
            let mut entries: Vec<(&String, &String)> = var_map.iter().collect();
            entries.sort_by(|a, b| a.1.cmp(b.1));
            for (original, short) in entries {
                lines.push(format!("  {short} <- {original}"));
            }
        }
        for (array_name, member_map) in &self.array_members {
            lines.push(format!("# Array members of {array_name}"));
            let mut entries: Vec<(&String, &String)> = member_map.iter().collect();
            entries.sort_by(|a, b| a.1.cmp(b.1));
            for (original, short) in entries {
                lines.push(format!("  {short} <- {original}"));
            }
        }
        lines.join("\n")
    }
}

/// Full result from aggressive minification.  Mirrors Python's
/// `MinifyResult`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MinifyResult {
    /// The minified source.
    pub source: String,
    /// Compaction symbol map.
    pub symbol_map: SymbolMap,
    /// Number of optimiser rewrites applied.
    pub optimisations_applied: usize,
    /// Length of the original source (bytes).
    pub original_length: usize,
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

/// Minify a Tcl source string for the given dialect (default tier).
#[must_use]
pub fn minify_tcl(source: &str, dialect: &str, registry: &CommandRegistry) -> String {
    minify_body(source, dialect, registry)
}

/// Aggressive minification: apply the compiler's optimiser
/// rewrites, then compact names, then minify whitespace.  Returns
/// a [`MinifyResult`].  Mirrors `minify_tcl(..., aggressive=True)`.
///
/// Deferred relative to the Python pipeline: static-substring
/// folding (SCCP-based, phase 1.5) and the command / argument /
/// string-literal aliasing phases (2.5–2.7).
#[must_use]
pub fn minify_tcl_aggressive(
    source: &str,
    dialect: &str,
    isolated: bool,
    registry: &CommandRegistry,
) -> MinifyResult {
    let original_length = source.len();

    // Phase 1: apply the optimiser's semantic-preserving rewrites.
    let optimisations =
        tcl_compiler::optimiser::optimise_with_dialect(source, registry, Some(dialect));
    let opt_count = optimisations.iter().filter(|o| !o.hint_only).count();
    let opt_edits: Vec<Edit> = optimisations
        .iter()
        .filter(|o| !o.hint_only)
        .map(|o| {
            (
                o.span.start() as usize,
                (o.span.end() - o.span.start()) as usize,
                o.replacement.clone(),
            )
        })
        .collect();
    let optimised = apply_edits(source, opt_edits);

    // Phase 2: compact names.  Phase 3: minify whitespace.
    let (renamed, symbol_map) = compact_names(&optimised, dialect, isolated, registry);
    let minified = minify_body(&renamed, dialect, registry);

    MinifyResult {
        source: minified,
        symbol_map,
        optimisations_applied: opt_count,
        original_length,
    }
}

/// Minify with local-name compaction: rename proc-local variables,
/// parameters, and proc names to short identifiers, then run the
/// default minifier.  Returns the minified source plus a
/// [`SymbolMap`].  Mirrors `minify_tcl(..., compact_names=True)`.
///
/// `isolated` also compacts global-scope variables (safe for
/// self-contained scripts like iRules event handlers).
#[must_use]
pub fn minify_tcl_compact(
    source: &str,
    dialect: &str,
    isolated: bool,
    registry: &CommandRegistry,
) -> (String, SymbolMap) {
    let (renamed, symbol_map) = compact_names(source, dialect, isolated, registry);
    let minified = minify_body(&renamed, dialect, registry);
    (minified, symbol_map)
}

/// Minify a Tcl script body (top-level or inside braces).
fn minify_body(source: &str, dialect: &str, registry: &CommandRegistry) -> String {
    let sm = SourceMap::new(source);
    let Ok(tokens) = Lexer::new(source).tokenise_all() else {
        return source.to_owned();
    };

    let commands = parse_commands(source, &tokens);
    if commands.is_empty() {
        return String::new();
    }

    // Render each command, abbreviating ensemble subcommands.
    let mut rendered: Vec<Vec<String>> = Vec::with_capacity(commands.len());
    for cmd_args in &commands {
        let mut arg_strs = render_command(&sm, cmd_args, dialect, registry);
        if arg_strs.len() >= 2 {
            arg_strs[1] = abbreviated_subcommand(&arg_strs[0], &arg_strs[1], dialect);
        }
        rendered.push(arg_strs);
    }

    // Template deduplication (subst aliasing) of repeated dynamic
    // quoted args.
    let (template_map, rendered) = dedup_templates(rendered);

    let is_irules = dialect == "f5-irules";
    let mut parts: Vec<String> = Vec::new();
    for (content, alias) in &template_map {
        parts.push(format!("set {alias} {{{content}}}"));
    }
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

/// Lazy generator of short identifier names: `a`, `b`, …, `z`,
/// `aa`, `ab`, …  Mirrors `core/common/text_edits.py::name_generator`.
struct NameGenerator {
    indices: Vec<usize>,
}

impl NameGenerator {
    fn new() -> Self {
        Self { indices: vec![0] }
    }

    fn next_name(&mut self) -> String {
        let name: String = self
            .indices
            .iter()
            .map(|&i| (b'a' + u8::try_from(i).unwrap_or(0)) as char)
            .collect();
        self.advance();
        name
    }

    fn advance(&mut self) {
        let mut pos = self.indices.len();
        loop {
            if pos == 0 {
                // All positions wrapped — grow the length.
                self.indices = vec![0; self.indices.len() + 1];
                return;
            }
            pos -= 1;
            if self.indices[pos] + 1 < 26 {
                self.indices[pos] += 1;
                return;
            }
            self.indices[pos] = 0;
        }
    }
}

// ---------------------------------------------------------------------------
// Local-name compaction (compact_names tier)
// ---------------------------------------------------------------------------

/// A text edit: replace `length` bytes at `offset` with `text`.
type Edit = (usize, usize, String);

/// Apply non-overlapping `(offset, length, new_text)` edits in
/// reverse offset order, deduplicating identical `(offset, length)`
/// pairs.  Mirrors `core/common/text_edits.py::apply_edits`.
fn apply_edits(source: &str, mut edits: Vec<Edit>) -> String {
    if edits.is_empty() {
        return source.to_owned();
    }
    edits.sort_by(|a, b| b.0.cmp(&a.0));
    let mut seen: HashSet<(usize, usize)> = HashSet::new();
    let mut result = source.to_owned();
    for (offset, length, new_text) in edits {
        if !seen.insert((offset, length)) {
            continue;
        }
        if offset + length <= result.len() {
            result.replace_range(offset..offset + length, &new_text);
        }
    }
    result
}

/// Scope label: `::` for the root, then `parent::child`.
fn child_scope_label(parent_label: &str, child_name: &str) -> String {
    if parent_label == "::" {
        format!("::{child_name}")
    } else {
        format!("{parent_label}::{child_name}")
    }
}

/// Deepest scope label whose body span contains `offset`.  Mirrors
/// `_scope_label_at_line` (byte-offset based).
fn scope_label_at_offset(
    scope: &Scope,
    offset: u32,
    prefix: &str,
    include_global: bool,
) -> Option<String> {
    for child in &scope.children {
        let label = child_scope_label(prefix, &child.name);
        if let Some(body) = child.body_span {
            if body.start() <= offset && offset <= body.end() {
                if let Some(deeper) = scope_label_at_offset(child, offset, &label, include_global) {
                    return Some(deeper);
                }
                return Some(label);
            }
        }
    }
    match scope.kind {
        ScopeKind::Proc => Some(prefix.to_owned()),
        ScopeKind::Global if include_global => Some(prefix.to_owned()),
        _ => None,
    }
}

/// Scope labels containing a dynamic-barrier command — renaming
/// inside them is unsafe.  Mirrors `_find_barrier_scopes`.
fn find_barrier_scopes(
    analysis: &AnalysisResult,
    registry: &CommandRegistry,
    include_global: bool,
) -> HashSet<String> {
    let barrier_cmds: HashSet<&str> = registry
        .commands_with_trait(Traits::CREATES_DYNAMIC_BARRIER)
        .into_iter()
        .collect();
    let mut out = HashSet::new();
    for inv in &analysis.command_invocations {
        if barrier_cmds.contains(inv.name.as_str()) {
            if let Some(label) = scope_label_at_offset(
                &analysis.global_scope,
                inv.range.start(),
                "::",
                include_global,
            ) {
                out.insert(label);
            }
        }
    }
    out
}

/// Next short name avoiding existing and claimed names.  Mirrors
/// `_next_unused_name`.
fn next_unused_name(
    gen: &mut NameGenerator,
    existing: &HashSet<String>,
    claimed: &HashSet<String>,
) -> Option<String> {
    for _ in 0..1000 {
        let short = gen.next_name();
        if !existing.contains(&short) && !claimed.contains(&short) {
            return Some(short);
        }
    }
    None
}

/// Rename parameter names within the proc's parameter-list region.
/// Mirrors `_rename_params_in_list`.
fn rename_params_in_list(
    source: &str,
    proc_def: &ProcDef,
    var_map: &BTreeMap<String, String>,
    edits: &mut Vec<Edit>,
) {
    let search_start = proc_def.name_span.end() as usize;
    let search_end = proc_def.body_span.start() as usize;
    if search_start > search_end || search_end > source.len() {
        return;
    }
    let region = &source.as_bytes()[search_start..search_end];
    for param in &proc_def.params {
        let Some(short) = var_map.get(&param.name) else {
            continue;
        };
        let pat = param.name.as_bytes();
        if pat.is_empty() {
            continue;
        }
        let mut i = 0;
        while i + pat.len() <= region.len() {
            if &region[i..i + pat.len()] == pat
                && !(i > 0 && is_word_byte(Some(region[i - 1])))
                && !is_word_byte(region.get(i + pat.len()).copied())
            {
                edits.push((search_start + i, pat.len(), short.clone()));
                i += pat.len();
            } else {
                i += 1;
            }
        }
    }
}

/// Whether `b` is `[A-Za-z0-9_]`.
fn is_word_byte(b: Option<u8>) -> bool {
    matches!(b, Some(c) if c.is_ascii_alphanumeric() || c == b'_')
}

/// Byte-span slice of `source`.
fn slice(source: &str, span: Span) -> &str {
    let (s, e) = (span.start() as usize, span.end() as usize);
    if s <= e && e <= source.len() {
        &source[s..e]
    } else {
        ""
    }
}

/// Call sites of the proc `name` / `qualified_name`.  Mirrors the
/// inlined `find_proc_call_sites`.
fn find_proc_call_sites(name: &str, qualified_name: &str, analysis: &AnalysisResult) -> Vec<Span> {
    let qn_no_prefix = qualified_name.strip_prefix("::").unwrap_or(qualified_name);
    let mut out = Vec::new();
    let mut seen: HashSet<(u32, u32)> = HashSet::new();
    for inv in &analysis.command_invocations {
        let matches = match &inv.resolved_qualified_name {
            Some(resolved) => resolved == qualified_name,
            None => inv.name == name || inv.name == qualified_name || inv.name == qn_no_prefix,
        };
        if matches && seen.insert((inv.range.start(), inv.range.end())) {
            out.push(inv.range);
        }
    }
    out
}

/// Compact proc-local (and, when `isolated`, global) variable,
/// parameter, and proc names.  Mirrors `_compact_names`; returns
/// `(renamed_source, symbol_map)`.
fn compact_names(
    source: &str,
    dialect: &str,
    isolated: bool,
    registry: &CommandRegistry,
) -> (String, SymbolMap) {
    let analysis = Analyser::new().analyse(source, dialect).clone();
    let mut symbol_map = SymbolMap::default();
    let mut edits: Vec<Edit> = Vec::new();

    let barrier_scopes = find_barrier_scopes(&analysis, registry, isolated);
    let builtin_names: HashSet<&str> = registry.command_names().collect();

    process_scope(
        source,
        &analysis,
        &analysis.global_scope,
        "::",
        isolated,
        &barrier_scopes,
        &mut symbol_map,
        &mut edits,
    );

    // Proc renaming.
    let mut proc_gen = NameGenerator::new();
    let mut used_proc_names: HashSet<String> = HashSet::new();
    let mut proc_keys: Vec<&String> = analysis.all_procs.keys().collect();
    proc_keys.sort();
    for qname in proc_keys {
        let proc_def = &analysis.all_procs[qname];
        let name = &proc_def.name;
        if name.len() <= 1 || name.contains("::") {
            continue;
        }
        let mut short = proc_gen.next_name();
        while builtin_names.contains(short.as_str()) || used_proc_names.contains(&short) {
            short = proc_gen.next_name();
        }
        if short.len() >= name.len() {
            continue;
        }
        used_proc_names.insert(short.clone());

        let r = proc_def.name_span;
        let actual = slice(source, r);
        let def_key = (r.start() as usize, actual.len());
        if actual == *name {
            edits.push((r.start() as usize, actual.len(), short.clone()));
        }
        for call in find_proc_call_sites(name, &proc_def.qualified_name, &analysis) {
            let call_text = slice(source, call);
            let key = (call.start() as usize, call_text.len());
            if key != def_key && call_text == *name {
                edits.push((call.start() as usize, call_text.len(), short.clone()));
            }
        }
        symbol_map.procs.insert(name.clone(), short);
    }

    // Static array-member compaction (global across scopes).
    let array_members = compact_array_members(source, &mut edits);
    if !array_members.is_empty() {
        symbol_map.array_members = array_members;
    }

    let result = apply_edits(source, edits);
    (result, symbol_map)
}

/// Compact static array-member names (`arr(member)` → `arr(x)`).
/// Mirrors `_compact_array_members`; renames are global across
/// scopes and skip arrays whose members look user-input-derived.
fn compact_array_members(
    source: &str,
    edits: &mut Vec<Edit>,
) -> BTreeMap<String, BTreeMap<String, String>> {
    let array_uses = collect_array_uses(source);
    let mut result = BTreeMap::new();
    for (arr, members) in &array_uses {
        if members.keys().any(|m| is_unsafe_member(m)) {
            continue;
        }
        let mut gen = NameGenerator::new();
        let mut member_map: BTreeMap<String, String> = BTreeMap::new();
        let existing: HashSet<String> = members.keys().cloned().collect();
        for member in members.keys() {
            let claimed: HashSet<String> = member_map.values().cloned().collect();
            let Some(short) = next_unused_name(&mut gen, &existing, &claimed) else {
                continue;
            };
            if short.len() >= member.len() {
                continue;
            }
            for &off in &members[member] {
                edits.push((off, member.len(), short.clone()));
            }
            member_map.insert(member.clone(), short);
        }
        if !member_map.is_empty() {
            result.insert(arr.clone(), member_map);
        }
    }
    result
}

/// Recursively scan for `arr(member)` references, descending into
/// braced and command-substitution tokens.  Mirrors
/// `_scan_array_tokens`; returns `arr -> member -> [offsets]`.
fn collect_array_uses(top_source: &str) -> BTreeMap<String, BTreeMap<String, Vec<usize>>> {
    let mut uses: BTreeMap<String, BTreeMap<String, Vec<usize>>> = BTreeMap::new();
    let mut stack: Vec<(String, u32)> = vec![(top_source.to_owned(), 0)];
    while let Some((text, base)) = stack.pop() {
        let sm = SourceMap::new(&text);
        let Ok(tokens) = Lexer::new(&text).tokenise_all() else {
            continue;
        };
        let mut prev_type = TokenType::Eol;
        let mut in_quoted = false;
        for tok in &tokens {
            match tok.kind {
                TokenType::Eof => break,
                TokenType::Sep | TokenType::Eol => {
                    prev_type = tok.kind;
                    in_quoted = false;
                    continue;
                }
                TokenType::Str => {
                    let inner = sm.token_text(*tok);
                    if inner.len() >= 4 {
                        stack.push((inner.to_owned(), base + tok.span.start() + 1));
                    }
                    prev_type = TokenType::Str;
                    in_quoted = false;
                    continue;
                }
                TokenType::Cmd => {
                    let inner = sm.token_text(*tok);
                    if inner.len() >= 4 {
                        stack.push((inner.to_owned(), base + tok.span.start() + 1));
                    }
                    prev_type = TokenType::Cmd;
                    continue;
                }
                _ => {}
            }
            if matches!(prev_type, TokenType::Sep | TokenType::Eol) {
                let abs = (base + tok.span.start()) as usize;
                in_quoted = top_source.as_bytes().get(abs) == Some(&b'"');
            }
            prev_type = tok.kind;
            if in_quoted && tok.kind == TokenType::Esc {
                continue;
            }
            if !matches!(tok.kind, TokenType::Esc | TokenType::Var) {
                continue;
            }
            let ttext = sm.token_text(*tok);
            let Some((arr, member)) = parse_array_member(ttext) else {
                continue;
            };
            if member.chars().count() <= 1 || arr.contains("::") {
                continue;
            }
            let text_start = if tok.kind == TokenType::Var {
                base + tok.span.start() + 1
            } else {
                base + tok.span.start()
            };
            let member_offset = text_start as usize + arr.len() + 1;
            uses.entry(arr.to_owned())
                .or_default()
                .entry(member.to_owned())
                .or_default()
                .push(member_offset);
        }
    }
    uses
}

/// Parse `arr(member)` token text into `(arr, member)`.  Mirrors
/// `_ARRAY_MEMBER_RE`: `arr` is `[\w:]+`, `member` excludes `)`,
/// `$`, `[`.
fn parse_array_member(text: &str) -> Option<(&str, &str)> {
    let inner = text.strip_suffix(')')?;
    let lparen = inner.find('(')?;
    let arr = &inner[..lparen];
    let member = &inner[lparen + 1..];
    if arr.is_empty() || member.is_empty() {
        return None;
    }
    if !arr
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == ':')
    {
        return None;
    }
    if member.chars().any(|c| c == ')' || c == '$' || c == '[') {
        return None;
    }
    Some((arr, member))
}

/// Whether an array-member name looks user-input-derived (and so
/// must not be renamed).  Mirrors `_UNSAFE_MEMBER_PATTERN`.
fn is_unsafe_member(member: &str) -> bool {
    const PREFIXES: &[&str] = &[
        "uri", "url", "path", "header", "cookie", "query", "param", "filename", "request", "input",
        "form", "method", "remote", "client", "addr", "password", "auth", "token", "session",
    ];
    let lower = member.to_ascii_lowercase();
    PREFIXES.iter().any(|p| {
        lower
            .strip_prefix(p)
            .is_some_and(|rest| rest.is_empty() || rest.starts_with('_'))
    })
}

/// Recursively rename variables (and params) in a scope, mirroring
/// `_process_scope`.
#[allow(clippy::too_many_arguments)]
fn process_scope(
    source: &str,
    analysis: &AnalysisResult,
    scope: &Scope,
    scope_label: &str,
    isolated: bool,
    barrier_scopes: &HashSet<String>,
    symbol_map: &mut SymbolMap,
    edits: &mut Vec<Edit>,
) {
    let rename_scope = (scope.kind == ScopeKind::Proc
        || (isolated && scope.kind == ScopeKind::Global))
        && !barrier_scopes.contains(scope_label);

    if rename_scope {
        let proc_def = if scope.kind == ScopeKind::Proc {
            analysis.all_procs.values().find(|pd| pd.name == scope.name)
        } else {
            None
        };
        let param_names: HashSet<&str> = proc_def
            .map(|pd| pd.params.iter().map(|p| p.name.as_str()).collect())
            .unwrap_or_default();

        let mut var_gen = NameGenerator::new();
        let existing: HashSet<String> = scope.variables.keys().cloned().collect();
        let mut var_map: BTreeMap<String, String> = BTreeMap::new();

        let mut var_names: Vec<&String> = scope.variables.keys().collect();
        var_names.sort();
        for var_name in var_names {
            let var_def = &scope.variables[var_name];
            if var_name.len() <= 1 || var_name.contains("::") {
                continue;
            }
            let claimed: HashSet<String> = var_map.values().cloned().collect();
            let Some(short) = next_unused_name(&mut var_gen, &existing, &claimed) else {
                continue;
            };
            if short.len() >= var_name.len() {
                continue;
            }
            let is_param = param_names.contains(var_name.as_str());

            // Definition site (non-params only — param defs point at
            // the proc-name token).
            if !is_param {
                let r = var_def.definition_span;
                if slice(source, r) == *var_name {
                    edits.push((r.start() as usize, var_name.len(), short.clone()));
                }
            }
            // Reference sites (`$var`): skip the `$`.
            for &reference in &var_def.references {
                let ref_text = slice(source, reference);
                if let Some(rest) = ref_text.strip_prefix('$') {
                    if rest == var_name {
                        edits.push((
                            reference.start() as usize + 1,
                            var_name.len(),
                            short.clone(),
                        ));
                    }
                }
            }
            var_map.insert(var_name.clone(), short);
        }

        if let Some(pd) = proc_def {
            if !var_map.is_empty() {
                rename_params_in_list(source, pd, &var_map, edits);
            }
        }
        if !var_map.is_empty() {
            symbol_map.variables.insert(scope_label.to_owned(), var_map);
        }
    }

    for child in &scope.children {
        let label = child_scope_label(scope_label, &child.name);
        process_scope(
            source,
            analysis,
            child,
            &label,
            isolated,
            barrier_scopes,
            symbol_map,
            edits,
        );
    }
}

/// Dialects where ensemble commands are fixed (no user-added
/// subcommands), so prefix abbreviation is safe.
const FIXED_ENSEMBLE_DIALECTS: &[&str] = &["f5-irules", "f5-iapps", "f5-bigip"];

/// Return the abbreviated subcommand text when safe for `dialect`.
/// Mirrors `_abbreviated_subcommand`.
fn abbreviated_subcommand(command_name: &str, subcommand_name: &str, dialect: &str) -> String {
    if !FIXED_ENSEMBLE_DIALECTS.contains(&dialect) {
        return subcommand_name.to_owned();
    }
    subcommand_abbreviation(command_name, subcommand_name)
        .unwrap_or(subcommand_name)
        .to_owned()
}

/// Shortest unambiguous abbreviation for `sub` of ensemble
/// `command`, or `None`.  Mirrors `_SUBCMD_ABBREVIATIONS` (only the
/// entries strictly shorter than the full subcommand are kept).
fn subcommand_abbreviation(command: &str, sub: &str) -> Option<&'static str> {
    let table: &[(&str, &str)] = match command {
        "string" => &[
            ("bytelength", "b"),
            ("cat", "ca"),
            ("compare", "co"),
            ("equal", "e"),
            ("first", "f"),
            ("index", "in"),
            ("last", "la"),
            ("length", "le"),
            ("match", "mat"),
            ("range", "ra"),
            ("repeat", "repe"),
            ("replace", "repl"),
            ("reverse", "rev"),
            ("tolower", "tol"),
            ("totitle", "tot"),
            ("toupper", "tou"),
            ("trimleft", "triml"),
            ("trimright", "trimr"),
            ("wordend", "worde"),
            ("wordstart", "words"),
        ],
        "info" => &[
            ("args", "a"),
            ("body", "b"),
            ("cmdcount", "cm"),
            ("commands", "comm"),
            ("complete", "comp"),
            ("default", "d"),
            ("exists", "e"),
            ("frame", "fr"),
            ("functions", "fu"),
            ("globals", "g"),
            ("hostname", "h"),
            ("level", "le"),
            ("library", "li"),
            ("loaded", "loa"),
            ("locals", "loc"),
            ("nameofexecutable", "n"),
            ("patchlevel", "pa"),
            ("procs", "pr"),
            ("script", "sc"),
            ("sharedlibextension", "sh"),
            ("tclversion", "t"),
        ],
        "clock" => &[
            ("add", "a"),
            ("clicks", "c"),
            ("format", "f"),
            ("microseconds", "mic"),
            ("milliseconds", "mil"),
            ("scan", "sc"),
            ("seconds", "se"),
        ],
        _ => return None,
    };
    table
        .iter()
        .find(|(full, _)| *full == sub)
        .map(|(_, abbr)| *abbr)
}

/// Replace repeated dynamic quoted args with `[subst $alias]` and a
/// shared `set alias {content}` preamble.  Mirrors
/// `_dedup_templates`; returns the ordered `(content, alias)`
/// preamble pairs plus the rewritten commands.
fn dedup_templates(rendered: Vec<Vec<String>>) -> (Vec<(String, String)>, Vec<Vec<String>>) {
    // content -> use sites, preserving first-seen order.
    let mut order: Vec<String> = Vec::new();
    let mut uses: std::collections::HashMap<String, Vec<(usize, usize)>> =
        std::collections::HashMap::new();
    for (ci, args) in rendered.iter().enumerate() {
        for (ai, s) in args.iter().enumerate() {
            if !(s.starts_with('"') && s.ends_with('"') && s.len() >= 2) {
                continue;
            }
            let content = &s[1..s.len() - 1];
            if !content.contains('$') && !content.contains('[') {
                continue;
            }
            if content.len() < 10 {
                continue;
            }
            if content.matches('{').count() != content.matches('}').count() {
                continue;
            }
            let key = content.to_owned();
            uses.entry(key.clone()).or_insert_with(|| {
                order.push(key.clone());
                Vec::new()
            });
            uses.get_mut(&key).expect("inserted").push((ci, ai));
        }
    }
    if order.is_empty() {
        return (Vec::new(), rendered);
    }

    // Names already referenced as $var, so aliases don't shadow them.
    let mut used_names = collect_used_var_names(&rendered);

    // Process candidates by descending count * content-length.
    let mut candidates = order.clone();
    candidates.sort_by(|a, b| {
        let ka = uses[a].len() * a.len();
        let kb = uses[b].len() * b.len();
        kb.cmp(&ka).then_with(|| {
            // Stable on first-seen order for ties.
            order
                .iter()
                .position(|x| x == a)
                .cmp(&order.iter().position(|x| x == b))
        })
    });

    let mut gen = NameGenerator::new();
    let mut template_map: Vec<(String, String)> = Vec::new();
    for content in &candidates {
        let count = uses[content].len();
        if count < 2 {
            continue;
        }
        let mut alias = gen.next_name();
        while used_names.contains(&alias) {
            alias = gen.next_name();
        }
        let original_cost = count * (content.len() + 2);
        let preamble_cost = 4 + alias.len() + 1 + 1 + content.len() + 1 + 1;
        let subst_ref = format!("[subst ${alias}]");
        let aliased_cost = preamble_cost + count * subst_ref.len();
        if aliased_cost >= original_cost {
            continue;
        }
        if content.contains(&format!("${alias}")) || content.contains(&format!("${{{alias}}}")) {
            continue;
        }
        template_map.push((content.clone(), alias.clone()));
        used_names.insert(alias);
    }
    if template_map.is_empty() {
        return (Vec::new(), rendered);
    }

    // Apply replacements.
    let mut result = rendered;
    for (content, alias) in &template_map {
        let subst_ref = format!("[subst ${alias}]");
        for &(ci, ai) in &uses[content] {
            result[ci][ai].clone_from(&subst_ref);
        }
    }
    (template_map, result)
}

/// Names referenced as `$var` / `${var}` anywhere in the rendered
/// commands.  Mirrors the `_used_var_re` scan in `_dedup_templates`.
fn collect_used_var_names(rendered: &[Vec<String>]) -> std::collections::HashSet<String> {
    let mut out = std::collections::HashSet::new();
    for args in rendered {
        for s in args {
            let bytes = s.as_bytes();
            let mut i = 0;
            while i < bytes.len() {
                if bytes[i] == b'$' {
                    let mut j = i + 1;
                    if j < bytes.len() && bytes[j] == b'{' {
                        j += 1;
                    }
                    let start = j;
                    while j < bytes.len() && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'_')
                    {
                        j += 1;
                    }
                    if j > start {
                        out.insert(s[start..j].to_owned());
                    }
                    i = j;
                } else {
                    i += 1;
                }
            }
        }
    }
    out
}

/// Group a token stream into commands (lists of arguments),
/// dropping comments and whitespace.
fn parse_commands(source: &str, tokens: &[Token]) -> Vec<Vec<Arg>> {
    let mut commands: Vec<Vec<Arg>> = Vec::new();
    let mut current: Vec<Arg> = Vec::new();
    let mut prev_type = TokenType::Eol;

    for &tok in tokens {
        match tok.kind {
            TokenType::Eof => break,
            TokenType::Comment => continue,
            TokenType::Sep => {
                prev_type = TokenType::Sep;
                continue;
            }
            TokenType::Eol => {
                if !current.is_empty() {
                    commands.push(std::mem::take(&mut current));
                }
                prev_type = TokenType::Eol;
                continue;
            }
            _ => {}
        }

        let is_start = matches!(prev_type, TokenType::Sep | TokenType::Eol);
        let detected_quoted =
            is_start && source.as_bytes().get(tok.span.start() as usize) == Some(&b'"');

        if is_start || current.is_empty() {
            current.push(Arg {
                tokens: vec![tok],
                is_braced: tok.kind == TokenType::Str,
                is_quoted: detected_quoted,
            });
        } else {
            current.last_mut().expect("non-empty").tokens.push(tok);
        }
        prev_type = tok.kind;
    }
    if !current.is_empty() {
        commands.push(current);
    }
    commands
}

/// Render one command's arguments to their minified string forms.
fn render_command(
    sm: &SourceMap,
    cmd_args: &[Arg],
    dialect: &str,
    registry: &CommandRegistry,
) -> Vec<String> {
    let cmd_name = cmd_args
        .first()
        .map(|a| token_text(sm, a))
        .unwrap_or_default();
    let post: Vec<String> = cmd_args.iter().skip(1).map(|a| token_text(sm, a)).collect();
    let post_refs: Vec<&str> = post.iter().map(String::as_str).collect();

    let body_indices = role_indices(registry, &cmd_name, &post_refs, ArgRole::Body);
    let expr_indices = role_indices(registry, &cmd_name, &post_refs, ArgRole::Expr);
    let is_case_list = cmd_name == "switch" && is_switch_case_list_form(&post_refs);

    let mut out: Vec<String> = Vec::with_capacity(cmd_args.len());
    for (i, arg) in cmd_args.iter().enumerate() {
        let single_braced = arg.is_braced && arg.tokens.len() == 1;
        if body_indices.contains(&i) && single_braced {
            let inner = sm.token_text(arg.tokens[0]);
            let minified = if is_case_list {
                minify_switch_case_list(inner, dialect, registry)
            } else {
                minify_body(inner, dialect, registry)
            };
            out.push(format!("{{{minified}}}"));
        } else if expr_indices.contains(&i) && single_braced {
            let inner = sm.token_text(arg.tokens[0]);
            out.push(format!("{{{}}}", compress_expr(inner, dialect, registry)));
        } else {
            out.push(reconstruct_arg(sm, arg, dialect, registry));
        }
    }
    out
}

/// Registry role indices, offset by 1 for the command-name slot.
fn role_indices(
    registry: &CommandRegistry,
    name: &str,
    post_args: &[&str],
    role: ArgRole,
) -> Vec<usize> {
    if name.is_empty() {
        return Vec::new();
    }
    registry
        .arg_indices_for_role(name, post_args, role)
        .into_iter()
        .map(|i| i + 1)
        .collect()
}

/// Text of an argument's first token (Python's `_token_text`).
fn token_text(sm: &SourceMap, arg: &Arg) -> String {
    arg.tokens
        .first()
        .map(|&t| sm.token_text(t).to_owned())
        .unwrap_or_default()
}

/// First character a token will render as.  Mirrors
/// `_first_rendered_char`.
fn first_rendered_char(sm: &SourceMap, tok: Token) -> Option<char> {
    match tok.kind {
        TokenType::Str | TokenType::Expand => Some('{'),
        TokenType::Cmd => Some('['),
        TokenType::Var => Some('$'),
        _ => sm.token_text(tok).chars().next(),
    }
}

/// Rebuild source text from a single token, re-adding delimiters
/// and recursively minifying `[…]` substitutions.  Mirrors
/// `_reconstruct_raw`.
fn reconstruct_raw(
    sm: &SourceMap,
    tok: Token,
    next_tok: Option<Token>,
    dialect: &str,
    registry: &CommandRegistry,
) -> String {
    match tok.kind {
        TokenType::Str => format!("{{{}}}", sm.token_text(tok)),
        TokenType::Cmd => format!("[{}]", minify_body(sm.token_text(tok), dialect, registry)),
        TokenType::Var => {
            // Inside a quoted string, keep `${var}` when the next
            // token would otherwise extend the variable name.
            if let Some(next) = next_tok {
                if let Some(c) = first_rendered_char(sm, next) {
                    if c.is_alphanumeric() || c == '_' {
                        return format!("${{{}}}", sm.token_text(tok));
                    }
                }
            }
            format!("${}", sm.token_text(tok))
        }
        TokenType::Expand => "{*}".to_owned(),
        _ => sm.token_text(tok).to_owned(),
    }
}

/// Characters that would change semantics if they appear unquoted.
const NEEDS_QUOTING: &[char] = &[' ', '\t', '\n', '\r', '\u{0b}', '\u{0c}', ';', '"', '\0'];

/// Whether a quoted argument can safely drop its double quotes.
/// Mirrors `_can_strip_quotes`.
fn can_strip_quotes(raw: &str) -> bool {
    if raw.is_empty() {
        return false;
    }
    let first = raw.chars().next().unwrap();
    if matches!(first, '"' | '{' | '#') {
        return false;
    }
    if raw == "{*}" {
        return false;
    }
    if raw.chars().any(|c| NEEDS_QUOTING.contains(&c)) {
        return false;
    }
    // Any `{` / `}` outside `${var}` references blocks stripping.
    let stripped = strip_braced_var_refs(raw);
    !(stripped.contains('{') || stripped.contains('}'))
}

/// Remove `${…}` references from `raw` so the residual brace check
/// in [`can_strip_quotes`] only sees bare braces.
fn strip_braced_var_refs(raw: &str) -> String {
    let bytes = raw.as_bytes();
    let n = bytes.len();
    let mut out = String::with_capacity(n);
    let mut i = 0;
    while i < n {
        if bytes[i] == b'$' && i + 1 < n && bytes[i + 1] == b'{' {
            if let Some(close) = raw[i + 2..].find('}') {
                i = i + 2 + close + 1;
                continue;
            }
        }
        let ch_len = utf8_len(bytes[i]);
        out.push_str(&raw[i..i + ch_len]);
        i += ch_len;
    }
    out
}

/// Byte length of the UTF-8 char whose lead byte is `b`.
fn utf8_len(b: u8) -> usize {
    if b < 0x80 {
        1
    } else if b >> 5 == 0b110 {
        2
    } else if b >> 4 == 0b1110 {
        3
    } else {
        4
    }
}

/// Rebuild the source text of an argument from its tokens.
/// Mirrors `_reconstruct_arg`.
fn reconstruct_arg(sm: &SourceMap, arg: &Arg, dialect: &str, registry: &CommandRegistry) -> String {
    let mut raw = String::new();
    for (idx, &tok) in arg.tokens.iter().enumerate() {
        let next = if arg.is_quoted {
            arg.tokens.get(idx + 1).copied()
        } else {
            None
        };
        raw.push_str(&reconstruct_raw(sm, tok, next, dialect, registry));
    }
    if arg.is_quoted && !can_strip_quotes(&raw) {
        format!("\"{raw}\"")
    } else {
        raw
    }
}

// ---------------------------------------------------------------------------
// switch case-list handling
// ---------------------------------------------------------------------------

/// Whether the post-name args use the braced case-list form (a
/// single trailing word after any leading options).  Mirrors
/// `is_switch_case_list_form`.
fn is_switch_case_list_form(args: &[&str]) -> bool {
    let i = skip_switch_options(args);
    i < args.len() && i == args.len() - 1
}

/// Skip leading `switch` option words and the match-value arg,
/// returning the index of the first case-list element.  Mirrors
/// `_skip_switch_options` (options, then one value arg).
fn skip_switch_options(args: &[&str]) -> usize {
    let mut i = 0;
    while i < args.len() {
        let a = args[i];
        if a == "--" {
            i += 1;
            break;
        }
        if a.starts_with('-') {
            // `-matchvar` / `-indexvar` consume a following value.
            if matches!(a, "-matchvar" | "-indexvar") {
                i += 1;
            }
            i += 1;
        } else {
            break;
        }
    }
    // Skip the match-value argument itself.
    if i < args.len() {
        i += 1;
    }
    i
}

/// Minify the content of a `switch` braced case list, recursively
/// minifying each braced body.  Mirrors `_minify_switch_case_list`.
fn minify_switch_case_list(source: &str, dialect: &str, registry: &CommandRegistry) -> String {
    let sm = SourceMap::new(source);
    let Ok(tokens) = Lexer::new(source).tokenise_all() else {
        return source.to_owned();
    };
    // Segment into words (pattern / body), grouping multi-token words.
    let mut words: Vec<(String, bool, Token)> = Vec::new(); // (raw, is_braced, first_tok)
    let mut prev_type = TokenType::Eol;
    for tok in tokens {
        match tok.kind {
            TokenType::Eof => break,
            TokenType::Sep | TokenType::Eol | TokenType::Comment => {
                prev_type = tok.kind;
                continue;
            }
            _ => {}
        }
        let raw = reconstruct_raw(&sm, tok, None, dialect, registry);
        if matches!(
            prev_type,
            TokenType::Sep | TokenType::Eol | TokenType::Comment
        ) || words.is_empty()
        {
            words.push((raw, tok.kind == TokenType::Str, tok));
        } else {
            words.last_mut().expect("non-empty").0.push_str(&raw);
        }
        prev_type = tok.kind;
    }

    let mut parts: Vec<String> = Vec::new();
    let mut idx = 0;
    while idx + 1 < words.len() {
        let pattern = &words[idx].0;
        let (body_raw, body_braced, body_tok) = &words[idx + 1];
        let body_inner = sm.token_text(*body_tok);
        if body_inner == "-" && *body_raw == "-" {
            parts.push(format!("{pattern} -"));
        } else if *body_braced {
            let minified = minify_body(body_inner, dialect, registry);
            parts.push(format!("{pattern} {{{minified}}}"));
        } else {
            parts.push(format!("{pattern} {body_raw}"));
        }
        idx += 2;
    }
    parts.join(" ")
}

// ---------------------------------------------------------------------------
// expr whitespace compression
// ---------------------------------------------------------------------------

/// One token of an `expr` body for whitespace compression.
enum ExprTok {
    /// A `[…]` command substitution (already minified).
    Cmd(String),
    /// Any other token (string, var, word, operator, punctuation).
    Other(String),
    /// A run of whitespace.
    Space,
}

/// Remove unnecessary whitespace inside an `expr` body, keeping
/// spaces only around word-operators and between adjacent word
/// tokens.  Mirrors `_strip_expr_whitespace` (no AST shrinking).
fn strip_expr_whitespace(text: &str, dialect: &str, registry: &CommandRegistry) -> String {
    let toks = tokenise_expr(text, dialect, registry);
    let rendered: Vec<String> = toks
        .iter()
        .filter_map(|t| match t {
            ExprTok::Space => None,
            ExprTok::Cmd(s) | ExprTok::Other(s) => Some(s.clone()),
        })
        .collect();
    if rendered.is_empty() {
        return text.to_owned();
    }
    let mut out = String::new();
    out.push_str(&rendered[0]);
    for w in rendered.windows(2) {
        let (prev, cur) = (&w[0], &w[1]);
        if is_word_op(prev) || is_word_op(cur) || (is_word_token(prev) && is_word_token(cur)) {
            out.push(' ');
        }
        out.push_str(cur);
    }
    out
}

/// Compress and shrink an `expr` body: strip whitespace, then try
/// AST transforms (De Morgan / comparison inversion / double
/// negation) and keep whichever is shorter.  Mirrors
/// `_compress_expr`.
fn compress_expr(text: &str, dialect: &str, registry: &CommandRegistry) -> String {
    let compressed = strip_expr_whitespace(text, dialect, registry);
    let shrunk = shrink_expr_ast(&compressed, dialect, registry);
    if shrunk.len() < compressed.len() {
        shrunk
    } else {
        compressed
    }
}

/// AST-based expression shrinking.  Mirrors `_shrink_expr_ast`.
fn shrink_expr_ast(text: &str, dialect: &str, registry: &CommandRegistry) -> String {
    let node = parse_expr(text, Some(dialect));
    if matches!(node, ExprNode::Raw { .. }) {
        return text.to_owned();
    }
    let shrunk = shrink_node(&node);
    if shrunk == node {
        return text.to_owned();
    }
    let rendered = render_expr(&shrunk);
    strip_expr_whitespace(&rendered, dialect, registry)
}

/// The logical complement of a comparison / membership operator,
/// or `None` when it has none.  Mirrors `_COMPARISON_INVERSION`.
fn comparison_inversion(op: BinOp) -> Option<BinOp> {
    Some(match op {
        BinOp::Eq => BinOp::Ne,
        BinOp::Ne => BinOp::Eq,
        BinOp::Lt => BinOp::Ge,
        BinOp::Ge => BinOp::Lt,
        BinOp::Gt => BinOp::Le,
        BinOp::Le => BinOp::Gt,
        BinOp::StrEq => BinOp::StrNe,
        BinOp::StrNe => BinOp::StrEq,
        BinOp::In => BinOp::Ni,
        BinOp::Ni => BinOp::In,
        BinOp::StrLt => BinOp::StrGe,
        BinOp::StrGe => BinOp::StrLt,
        BinOp::StrGt => BinOp::StrLe,
        BinOp::StrLe => BinOp::StrGt,
        _ => return None,
    })
}

/// Build a `!operand` node.
fn negate(operand: ExprNode) -> ExprNode {
    ExprNode::Unary {
        op: UnaryOp::Not,
        operand: Box::new(operand),
    }
}

/// Pick `candidate` over `original` when its rendering is shorter.
fn pick_shorter(candidate: ExprNode, original: &ExprNode) -> ExprNode {
    if render_expr(&candidate).len() < render_expr(original).len() {
        candidate
    } else {
        original.clone()
    }
}

/// Recursively try size-reducing transforms on an expression node.
/// Mirrors `_shrink_node`.
fn shrink_node(node: &ExprNode) -> ExprNode {
    match node {
        ExprNode::Unary {
            op: UnaryOp::Not,
            operand,
        } => shrink_not(node, operand),
        ExprNode::Binary { op, left, right }
            if matches!(op, BinOp::Or | BinOp::WordOr) && both_negations(left, right) =>
        {
            // De Morgan reverse: !a || !b → !(a && b) (if shorter).
            let (a, b) = (unwrap_not(left), unwrap_not(right));
            let dual = if *op == BinOp::Or {
                BinOp::And
            } else {
                BinOp::WordAnd
            };
            let combined = negate(ExprNode::Binary {
                op: dual,
                left: Box::new(shrink_node(a)),
                right: Box::new(shrink_node(b)),
            });
            pick_shorter(combined, node)
        }
        ExprNode::Binary { op, left, right }
            if matches!(op, BinOp::And | BinOp::WordAnd) && both_negations(left, right) =>
        {
            // De Morgan reverse: !a && !b → !(a || b) (if shorter).
            let (a, b) = (unwrap_not(left), unwrap_not(right));
            let dual = if *op == BinOp::And {
                BinOp::Or
            } else {
                BinOp::WordOr
            };
            let combined = negate(ExprNode::Binary {
                op: dual,
                left: Box::new(shrink_node(a)),
                right: Box::new(shrink_node(b)),
            });
            pick_shorter(combined, node)
        }
        ExprNode::Binary { op, left, right } => {
            let new_left = shrink_node(left);
            let new_right = shrink_node(right);
            ExprNode::Binary {
                op: *op,
                left: Box::new(new_left),
                right: Box::new(new_right),
            }
        }
        ExprNode::Unary { op, operand } => ExprNode::Unary {
            op: *op,
            operand: Box::new(shrink_node(operand)),
        },
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => ExprNode::Ternary {
            condition: Box::new(shrink_node(condition)),
            true_branch: Box::new(shrink_node(true_branch)),
            false_branch: Box::new(shrink_node(false_branch)),
        },
        other => other.clone(),
    }
}

/// Whether both operands are `!`-negations.
fn both_negations(left: &ExprNode, right: &ExprNode) -> bool {
    matches!(
        left,
        ExprNode::Unary {
            op: UnaryOp::Not,
            ..
        }
    ) && matches!(
        right,
        ExprNode::Unary {
            op: UnaryOp::Not,
            ..
        }
    )
}

/// The operand of a `!`-negation (caller guarantees the shape).
fn unwrap_not(node: &ExprNode) -> &ExprNode {
    match node {
        ExprNode::Unary {
            op: UnaryOp::Not,
            operand,
        } => operand,
        _ => node,
    }
}

/// Handle the `!`-prefixed shrink cases (double negation,
/// comparison inversion, De Morgan forward), falling back to a
/// generic operand recurse.
fn shrink_not(node: &ExprNode, operand: &ExprNode) -> ExprNode {
    // Double negation: !!x → x.
    if let ExprNode::Unary {
        op: UnaryOp::Not,
        operand: inner,
    } = operand
    {
        return shrink_node(inner);
    }
    if let ExprNode::Binary { op, left, right } = operand {
        // Comparison inversion: !($a == $b) → $a != $b.
        if let Some(inv) = comparison_inversion(*op) {
            let inverted = ExprNode::Binary {
                op: inv,
                left: Box::new(shrink_node(left)),
                right: Box::new(shrink_node(right)),
            };
            return pick_shorter(inverted, node);
        }
        // De Morgan forward.
        if matches!(op, BinOp::And | BinOp::WordAnd | BinOp::Or | BinOp::WordOr) {
            let neg_l = negate(shrink_node(left));
            let neg_r = negate(shrink_node(right));
            let dual = match op {
                BinOp::And => BinOp::Or,
                BinOp::WordAnd => BinOp::WordOr,
                BinOp::Or => BinOp::And,
                _ => BinOp::WordAnd,
            };
            let demorgan = ExprNode::Binary {
                op: dual,
                left: Box::new(shrink_node(&neg_l)),
                right: Box::new(shrink_node(&neg_r)),
            };
            return pick_shorter(demorgan, node);
        }
    }
    // Generic recurse into the operand.
    negate(shrink_node(operand))
}

/// Tokenise an `expr` body, mirroring the `_EXPR_TOKEN` alternation
/// (with a catch-all so no character is dropped — safer than the
/// Python reference, which silently drops unmatched characters).
fn tokenise_expr(text: &str, dialect: &str, registry: &CommandRegistry) -> Vec<ExprTok> {
    let bytes = text.as_bytes();
    let n = bytes.len();
    let mut out = Vec::new();
    let mut i = 0;
    while i < n {
        let c = bytes[i];
        if c.is_ascii_whitespace() {
            let start = i;
            while i < n && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            let _ = start;
            out.push(ExprTok::Space);
        } else if c == b'"' {
            let start = i;
            i += 1;
            while i < n {
                if bytes[i] == b'\\' && i + 1 < n {
                    i += 2;
                    continue;
                }
                if bytes[i] == b'"' {
                    i += 1;
                    break;
                }
                i += 1;
            }
            out.push(ExprTok::Other(text[start..i].to_owned()));
        } else if c == b'[' {
            let start = i;
            i += 1;
            let mut depth = 1;
            while i < n && depth > 0 {
                match bytes[i] {
                    b'[' => depth += 1,
                    b']' => depth -= 1,
                    _ => {}
                }
                i += 1;
            }
            let inner = &text[start + 1..i.saturating_sub(1).max(start + 1)];
            out.push(ExprTok::Cmd(format!(
                "[{}]",
                minify_body(inner, dialect, registry)
            )));
        } else if c == b'$' {
            let start = i;
            i += 1;
            while i < n
                && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_' || bytes[i] == b':')
            {
                i += 1;
            }
            out.push(ExprTok::Other(text[start..i].to_owned()));
        } else if c.is_ascii_alphanumeric() || c == b'.' || c == b'_' {
            let start = i;
            while i < n
                && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'.' || bytes[i] == b'_')
            {
                i += 1;
            }
            out.push(ExprTok::Other(text[start..i].to_owned()));
        } else if is_expr_op_byte(c) {
            let start = i;
            while i < n && is_expr_op_byte(bytes[i]) {
                i += 1;
            }
            out.push(ExprTok::Other(text[start..i].to_owned()));
        } else {
            // Catch-all single char (`(`, `)`, `,`, etc.).
            let ch_len = utf8_len(c);
            out.push(ExprTok::Other(text[i..i + ch_len].to_owned()));
            i += ch_len;
        }
    }
    out
}

/// Whether `b` is a byte that forms a symbolic `expr` operator.
fn is_expr_op_byte(b: u8) -> bool {
    matches!(
        b,
        b'+' | b'-'
            | b'*'
            | b'/'
            | b'%'
            | b'<'
            | b'>'
            | b'='
            | b'!'
            | b'&'
            | b'|'
            | b'^'
            | b'?'
            | b':'
            | b'~'
    )
}

/// Whether `tok` is a Tcl expr word-operator needing surrounding
/// whitespace (`eq`, `ne`, `in`, `ni`).
fn is_word_op(tok: &str) -> bool {
    matches!(tok, "eq" | "ne" | "in" | "ni")
}

/// Whether `tok` is a "word" (identifier / number / variable /
/// string / command-substitution).  Mirrors `_is_word_token`.
fn is_word_token(tok: &str) -> bool {
    let Some(c) = tok.chars().next() else {
        return false;
    };
    c == '$' || c == '"' || c == '[' || c.is_alphanumeric() || c == '_'
}

#[cfg(test)]
mod tests {
    use super::*;

    fn min(src: &str) -> String {
        let registry = CommandRegistry::build_default();
        minify_tcl(src, "tcl8.6", &registry)
    }

    fn check(input: &str, expected: &str) {
        let got = min(input);
        assert_eq!(
            got, expected,
            "\ninput:    {input:?}\ngot:      {got:?}\nexpected: {expected:?}"
        );
    }

    #[test]
    fn strips_comments() {
        check("# a comment\nputs hi\n", "puts hi");
    }

    fn min_dialect(src: &str, dialect: &str) -> String {
        let registry = CommandRegistry::build_default();
        minify_tcl(src, dialect, &registry)
    }

    fn min_compact(src: &str) -> String {
        let registry = CommandRegistry::build_default();
        minify_tcl_compact(src, "tcl8.6", false, &registry).0
    }

    #[test]
    fn compact_renames_proc_local_vars_and_params() {
        // `greet`→`a`, param `name`→`b`, local `message`→`a`.
        assert_eq!(
            min_compact(
                "proc greet {name} {\n    set message \"hi $name\"\n    return $message\n}\n"
            ),
            "proc a {b} {set a \"hi $b\";return $a}",
        );
    }

    #[test]
    fn compact_renames_refs_inside_expr_and_command_subst() {
        // The `$value` ref lives inside `[expr {...}]`; it must be
        // renamed in lock-step with the param declaration (relies on
        // the analyser tracking expr/command-subst references).
        assert_eq!(
            min_compact(
                "proc helper {value} {\n    return [expr {$value * 2}]\n}\nproc main {} {\n    set result [helper 21]\n    puts $result\n}\n"
            ),
            "proc a {a} {return [expr {$a*2}]};proc b {} {set a [a 21];puts $a}",
        );
    }

    #[test]
    fn compact_returns_symbol_map() {
        let registry = CommandRegistry::build_default();
        let (_, sym) = minify_tcl_compact(
            "proc greet {name} {\n    return $name\n}\n",
            "tcl8.6",
            false,
            &registry,
        );
        assert_eq!(sym.procs.get("greet").map(String::as_str), Some("a"));
        assert!(sym
            .variables
            .values()
            .any(|m| m.get("name").map(String::as_str) == Some("a")));
    }

    #[test]
    fn compact_isolated_renames_global_vars() {
        let registry = CommandRegistry::build_default();
        let (out, _) = minify_tcl_compact(
            "set globalvar 1\nputs $globalvar\n",
            "tcl8.6",
            true,
            &registry,
        );
        assert_eq!(out, "set a 1;puts $a");
    }

    #[test]
    fn aggressive_runs_optimise_compact_minify() {
        let registry = CommandRegistry::build_default();
        let src = "proc greet {name} {\n    set message \"hi $name\"\n    return $message\n}\n";
        let res = minify_tcl_aggressive(src, "tcl8.6", false, &registry);
        // With no applicable optimisations this equals the compact tier.
        assert_eq!(res.source, "proc a {b} {set a \"hi $b\";return $a}");
        assert_eq!(res.original_length, src.len());
        assert_eq!(res.minified_length(), res.source.len());
        assert!(res.savings_pct() > 0.0);
    }

    #[test]
    fn compact_renames_static_array_members() {
        assert_eq!(
            min_compact("proc f {} {\n    set config(database) 1\n    set config(timeout) 2\n    puts $config(database)$config(timeout)\n}\n"),
            "proc f {} {set config(a) 1;set config(b) 2;puts $config(a)$config(b)}",
        );
    }

    #[test]
    fn compact_skips_user_input_array_members() {
        // `uri` looks user-input-derived — leave the array alone.
        assert_eq!(
            min_compact("proc f {} {\n    set config(uri) 1\n    puts $config(uri)\n}\n"),
            "proc f {} {set config(uri) 1;puts $config(uri)}",
        );
    }

    #[test]
    fn compact_non_isolated_keeps_global_vars() {
        assert_eq!(
            min_compact("set globalvar 1\nputs $globalvar\n"),
            "set globalvar 1;puts $globalvar"
        );
    }

    #[test]
    fn dedup_repeated_dynamic_templates() {
        check(
            "puts \"value is $longvariablename here\"\nputs \"value is $longvariablename here\"\n",
            "set a {value is $longvariablename here};puts [subst $a];puts [subst $a]",
        );
    }

    #[test]
    fn abbreviates_ensemble_subcommand_in_irules() {
        assert_eq!(
            min_dialect("string length $x\n", "f5-irules"),
            "string le $x"
        );
        assert_eq!(min_dialect("info exists $x\n", "f5-irules"), "info e $x");
    }

    #[test]
    fn no_subcommand_abbreviation_in_plain_tcl() {
        assert_eq!(
            min_dialect("string length $x\n", "tcl8.6"),
            "string length $x"
        );
    }

    #[test]
    fn expr_comparison_inversion() {
        check("if {!($a == $b)} {puts x}\n", "if {$a!=$b} {puts x}");
    }

    #[test]
    fn expr_de_morgan_forward() {
        check("if {!($a && $b)} {puts x}\n", "if {!$a||!$b} {puts x}");
    }

    #[test]
    fn expr_de_morgan_reverse() {
        check("if {!$a || !$b} {puts x}\n", "if {!$a||!$b} {puts x}");
    }

    #[test]
    fn expr_double_negation() {
        check("if {!!$x} {puts x}\n", "if {$x} {puts x}");
    }

    #[test]
    fn expr_no_change_when_already_minimal() {
        check("if {$a < $b} {puts x}\n", "if {$a<$b} {puts x}");
    }

    #[test]
    fn expr_shrink_nested_in_command_subst() {
        check(
            "set y [expr {!($a==1 && $b==2)}]\n",
            "set y [expr {$a!=1||$b!=2}]",
        );
    }

    #[test]
    fn collapses_commands_to_semicolons() {
        check("set x 1\nset y 2\n", "set x 1;set y 2");
    }

    #[test]
    fn collapses_intra_command_whitespace() {
        check("set    x     1\n", "set x 1");
    }

    #[test]
    fn recurses_into_proc_body() {
        check(
            "proc f {} {\n    # c\n    set x 1\n}\n",
            "proc f {} {set x 1}",
        );
    }

    #[test]
    fn recurses_into_command_substitution() {
        check("set y [ expr {1 + 2} ]\n", "set y [expr {1+2}]");
    }

    #[test]
    fn strips_redundant_quotes() {
        check("puts \"hello\"\n", "puts hello");
    }

    #[test]
    fn keeps_quotes_when_needed() {
        check("puts \"hello world\"\n", "puts \"hello world\"");
    }

    #[test]
    fn compresses_expr_whitespace() {
        check("if {$a == 1} {\n    puts hi\n}\n", "if {$a==1} {puts hi}");
    }

    #[test]
    fn keeps_word_operator_spacing() {
        check(
            "if {$a eq $b} {\n    puts hi\n}\n",
            "if {$a eq $b} {puts hi}",
        );
    }

    #[test]
    fn minifies_switch_case_bodies() {
        check(
            "switch $x {\n    a {\n        puts 1\n    }\n    b {\n        puts 2\n    }\n}\n",
            "switch $x {a {puts 1} b {puts 2}}",
        );
    }

    #[test]
    fn switch_fallthrough_preserved() {
        check(
            "switch $x {\n    a -\n    b {\n        puts 2\n    }\n}\n",
            "switch $x {a - b {puts 2}}",
        );
    }

    #[test]
    fn nested_body_recursion() {
        check(
            "proc f {} {\n    if {$x} {\n        set y 1\n    }\n}\n",
            "proc f {} {if {$x} {set y 1}}",
        );
    }

    #[test]
    fn empty_source_minifies_to_empty() {
        check("\n\n# only a comment\n", "");
    }
}
