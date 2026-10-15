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

//! Variable-reference scanning utilities for compiler passes.
//!
//! [`VarReferenceScanner`] scans Tcl words and scripts for referenced
//! variable names.  It tokenises the input with the Rust lexer, collects
//! `VAR` tokens, and optionally recurses into command substitutions.
//!
//! Results are cached in a bounded LRU keyed by source text *and* scan
//! mode — the same word/script strings are scanned repeatedly across SSA,
//! GVN, and interprocedural passes, and the two modes
//! ([`VarReferenceScanner::scan_word`] vs
//! [`VarReferenceScanner::scan_script`]) can legitimately disagree about
//! the same text.

use std::collections::{BTreeSet, HashMap, VecDeque};

use tcl_lexer::{Lexer, LexerConfig, SourceMap, Span, Token, TokenType};
use tcl_registry::{ArgRole, CommandRegistry};

use crate::segmenter::SegmentedCommand;

/// One argument word whose registry role **is** a variable name
/// ([`ArgRole::names_variable`]) — the cell it denotes, where it is written,
/// and how it is spelled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NameRoleWord {
    /// Literal name value, separate from its original source geometry.
    pub word: String,
    /// The word's own source span, i.e. what a reference / highlight /
    /// rename provider points at.
    pub span: Span,
    /// The name text is already literal; preserve substitution sigils in it.
    pub braced_literal: bool,
    /// Which naming role the registry gave the word.
    pub role: ArgRole,
    /// Index into the command's own `texts` / `argv` (0 is the command name,
    /// so this is always ≥ 1).
    pub word_index: usize,
}

impl NameRoleWord {
    /// The canonical cell name — [`Self::word`] with any array-element
    /// suffix dropped, honouring `braced_literal`.
    #[must_use]
    pub fn base_name(&self) -> &str {
        crate::naming::normalise_var_name_braced(&self.word, self.braced_literal)
    }
}

/// Test-only catalogue convenience for detached command layouts.
/// Production roles require [`original_variable_name_role_words`] ownership.
#[must_use]
#[cfg(test)]
pub fn variable_name_role_words(
    cmd: &SegmentedCommand,
    registry: &CommandRegistry,
) -> Vec<NameRoleWord> {
    if cmd.texts.is_empty() {
        return Vec::new();
    }
    let head = cmd.name();
    let args: Vec<&str> = cmd.args().iter().map(String::as_str).collect();
    let mut out: Vec<NameRoleWord> = Vec::new();
    for &role in ArgRole::ALL {
        if !role.names_variable() {
            continue;
        }
        for idx in registry.arg_indices_for_role(head, &args, role) {
            let word = idx + 1;
            let (Some(tok), Some(text)) = (cmd.argv.get(word), cmd.texts.get(word)) else {
                continue;
            };
            let braced_literal =
                tok.kind == TokenType::Str && cmd.single_token_word.get(word) == Some(&true);
            if !braced_literal && crate::naming::is_dynamic_word(text) {
                continue;
            }
            out.push(NameRoleWord {
                word: text.clone(),
                span: tok.span,
                braced_literal,
                role,
                word_index: word,
            });
        }
    }
    out.sort_by_key(|w| w.span.start());
    out.dedup_by(|a, b| a.span == b.span);
    out
}

/// Readonly name words selected by the retained complete original invocation.
#[must_use]
pub fn original_variable_name_role_words(
    source: &str,
    analysis: &crate::analyser::AnalysisResult,
    command: &SegmentedCommand,
) -> Vec<NameRoleWord> {
    use crate::registry_invocation::InvocationWordOrigin;
    let Some(selected) = crate::registry_invocation::source_structure::source_registry_words(
        source, analysis, command,
    ) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for &(argument, role) in selected.roles().unwrap_or(&[]) {
        if !role.names_variable() {
            continue;
        }
        let Some(InvocationWordOrigin::Written(written)) =
            selected.origins().get(argument + 1).copied()
        else {
            continue;
        };
        let Some(original) = selected.original_argument_word(argument) else {
            continue;
        };
        let Some(bytes) = selected
            .arguments()
            .get(argument)
            .and_then(crate::registry_invocation::EffectiveInvocationWord::literal_bytes)
        else {
            continue;
        };
        let Ok(value) = std::str::from_utf8(bytes) else {
            continue;
        };
        let Ok(span) = original.content_span() else {
            continue;
        };
        out.push(NameRoleWord {
            word: value.to_owned(),
            span,
            braced_literal: true,
            role,
            word_index: written,
        });
    }
    out.sort_by_key(|word| word.span.start());
    out.dedup_by(|left, right| left.span == right.span);
    out
}

/// Options controlling what a [`VarReferenceScanner`] looks for.
#[allow(clippy::struct_excessive_bools)] // option flags, not a state machine
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VarScanOptions {
    /// Include explicitly standalone catalogue read-role assumptions.
    /// Source-bound roles come from the retained original invocation.
    pub include_var_read_roles: bool,
    /// When `true`, recurse into `[…]` command substitutions.
    pub recurse_cmd_substitutions: bool,
    /// When `true`, a read-modify-write command (`incr` / `append` /
    /// `lappend` — the `READS_BEFORE_WRITE` trait) also reports its
    /// `VarWrite` target as a *read* (it reads the prior value).  Name-level
    /// only — intended for dead-store / unused-variable liveness recovery,
    /// kept out of SSA `uses` so read-before-set versioning is unperturbed.
    pub include_reads_before_write: bool,
    /// When `true`, report constant-keyed array elements as their own
    /// variables (`arr(k)`) instead of the conflated base — the SSA build's
    /// per-element naming ([`crate::naming::element_var_name`]). Off for
    /// name-level consumers (unused-variable liveness, scope bookkeeping),
    /// which reason about the base.
    pub element_qualified: bool,
}

impl Default for VarScanOptions {
    fn default() -> Self {
        Self {
            include_var_read_roles: false,
            recurse_cmd_substitutions: true,
            include_reads_before_write: false,
            element_qualified: false,
        }
    }
}

/// Default maximum LRU cache size.
const DEFAULT_CACHE_SIZE: usize = 512;

/// Scan Tcl words/scripts for referenced variable names.
///
/// Results are cached in a bounded LRU keyed by source text, scan mode, and
/// this scanner's normalised exact lexer config. The same word/script strings are scanned repeatedly across SSA,
/// GVN, and interprocedural passes, so caching avoids redundant
/// lexer creation and tokenisation. The mode is part of the key because
/// value-body and script-body scans of identical text give different
/// answers (`set literal {$x}` reads `x` as a value word, nothing as a
/// script).
pub struct VarReferenceScanner {
    options: VarScanOptions,
    /// Exact lexical policy for the local words / bodies this scanner reads.
    /// Source-coordinate offsets and the whole-file BOM rule are normalised at
    /// construction because every scan owns a fresh offset-zero buffer.
    config: LexerConfig,
    /// Bounded LRU: `order` tracks access recency, `cache` stores results.
    cache: HashMap<CacheKey, BTreeSet<String>>,
    order: VecDeque<CacheKey>,
    cache_size: usize,
}

/// LRU key: the scanned text, the mode it was scanned in (`quoted_body` — see
/// [`scan_tokens`]), and the exact local lexical policy.
type CacheKey = (
    String,
    bool,
    LexerConfig,
    bool,
    Option<tcl_registry::registry::RegistrySemanticKey>,
);

impl VarReferenceScanner {
    /// Create a new scanner with the given options and default cache size.
    #[must_use]
    pub fn new(options: VarScanOptions) -> Self {
        Self::with_config(options, LexerConfig::default())
    }

    /// Create a scanner under an exact lexer configuration.
    ///
    /// Scanner inputs are extracted words or nested scripts, never the whole
    /// file. Normalising here keeps every token span local to its [`SourceMap`]
    /// while preserving all dialect grammar axes.
    #[must_use]
    pub fn with_config(options: VarScanOptions, config: LexerConfig) -> Self {
        Self::with_config_and_cache_size(options, config, DEFAULT_CACHE_SIZE)
    }

    /// Create a scanner for a registry-backed compatibility caller that does
    /// not already carry the document's exact [`LexerConfig`].
    #[must_use]
    pub fn for_registry(options: VarScanOptions, registry: &CommandRegistry) -> Self {
        Self::with_config(options, LexerConfig::for_profile(registry.profile()))
    }

    /// Create a new scanner with a custom cache size.
    #[must_use]
    pub fn with_cache_size(options: VarScanOptions, cache_size: usize) -> Self {
        Self::with_config_and_cache_size(options, LexerConfig::default(), cache_size)
    }

    /// [`Self::with_config`] with a custom cache size.
    #[must_use]
    pub fn with_config_and_cache_size(
        options: VarScanOptions,
        config: LexerConfig,
        cache_size: usize,
    ) -> Self {
        Self {
            options,
            config: config.nested().normalized(),
            cache: HashMap::new(),
            order: VecDeque::new(),
            cache_size,
        }
    }

    /// Whether this caller requested detached catalogue read roles.
    /// The flag supplies no original source or receiver authority.
    pub(crate) const fn standalone_roles_requested(&self) -> bool {
        self.options.include_var_read_roles
    }

    /// Whether this scanner reports element-qualified names
    /// ([`VarScanOptions::element_qualified`]).
    #[must_use]
    pub fn element_qualified(&self) -> bool {
        self.options.element_qualified
    }

    /// The normalised local-source lexer configuration owned by this scanner.
    #[must_use]
    pub fn lexer_config(&self) -> LexerConfig {
        self.config
    }

    /// The canonical name this scanner's consumer records for a raw
    /// variable word: element-qualified or base, per the options.
    #[must_use]
    pub fn canonical_name<'a>(&self, raw: &'a str) -> &'a str {
        self.canonical_name_braced(raw, false)
    }

    /// [`Self::canonical_name`] for a word whose delimiters make its content a
    /// literal name — a brace-quoted write target (`set {$n} 1`) or the
    /// `${…}` reference form. See
    /// [`crate::naming::element_var_name_braced`]: the content is the name
    /// verbatim, `$` and all.
    #[must_use]
    pub fn canonical_name_braced<'a>(&self, raw: &'a str, braced_literal: bool) -> &'a str {
        if self.options.element_qualified {
            crate::naming::element_var_name_braced(raw, braced_literal)
        } else {
            crate::naming::normalise_var_name_braced(raw, braced_literal)
        }
    }

    /// Scan one Tcl word for variable references (LRU-cached).
    ///
    /// `text` is already-extracted word/value text whose own enclosing
    /// quotes or braces were stripped by whatever produced it, so it is
    /// scanned in *value body* mode — see [`scan_tokens`].
    pub fn scan_word(&mut self, text: &str, registry: &CommandRegistry) -> BTreeSet<String> {
        self.scan_cached(text, registry, true)
    }

    /// Source syntax alone; command heads grant no receiver read roles.
    pub fn scan_lexical_word(
        &mut self,
        text: &str,
        registry: &CommandRegistry,
    ) -> BTreeSet<String> {
        self.scan_cached_roles(text, registry, true, false)
    }

    /// Detached catalogue assumptions may protect stores but prove no binding.
    pub fn scan_standalone_word(
        &mut self,
        text: &str,
        registry: &CommandRegistry,
    ) -> BTreeSet<String> {
        self.scan_cached_roles(text, registry, true, true)
    }

    /// Scan a Tcl script for variable references (LRU-cached).
    ///
    /// `source` is a genuine script body (an `eval`/`catch`/`uplevel` body,
    /// a proc body), so ordinary top-level Tcl word-splitting and
    /// brace-quoting apply: a `{…}` word inside it really does suppress
    /// substitution. Use [`Self::scan_word`] for a value word.
    pub fn scan_script(&mut self, source: &str, registry: &CommandRegistry) -> BTreeSet<String> {
        self.scan_cached(source, registry, false)
    }

    /// Script syntax alone; command heads grant no receiver read roles.
    pub fn scan_lexical_script(
        &mut self,
        source: &str,
        registry: &CommandRegistry,
    ) -> BTreeSet<String> {
        self.scan_cached_roles(source, registry, false, false)
    }

    /// Detached script catalogue advice may protect stores but proves no binding.
    pub fn scan_standalone_script(
        &mut self,
        source: &str,
        registry: &CommandRegistry,
    ) -> BTreeSet<String> {
        self.scan_cached_roles(source, registry, false, true)
    }

    /// Shared cache lookup/insert for both scan modes.
    fn scan_cached(
        &mut self,
        source: &str,
        registry: &CommandRegistry,
        quoted_body: bool,
    ) -> BTreeSet<String> {
        self.scan_cached_roles(
            source,
            registry,
            quoted_body,
            self.options.include_var_read_roles,
        )
    }

    fn scan_cached_roles(
        &mut self,
        source: &str,
        registry: &CommandRegistry,
        quoted_body: bool,
        standalone_roles: bool,
    ) -> BTreeSet<String> {
        let key: CacheKey = (
            source.to_owned(),
            quoted_body,
            self.config,
            standalone_roles,
            standalone_roles.then(|| registry.snapshot().semantic_key()),
        );

        // Check cache.
        if let Some(cached) = self.cache.get(&key) {
            let result = cached.clone();
            // Move to end of LRU order.
            self.order.retain(|k| k != &key);
            self.order.push_back(key);
            return result;
        }

        let options = VarScanOptions {
            include_var_read_roles: standalone_roles,
            ..self.options
        };
        let result = scan_tokens(source, registry, options, quoted_body, self.config);

        // Insert into cache.
        self.cache.insert(key.clone(), result.clone());
        self.order.push_back(key);

        // Evict oldest if over capacity.
        while self.cache.len() > self.cache_size {
            if let Some(oldest) = self.order.pop_front() {
                self.cache.remove(&oldest);
            }
        }

        result
    }

    /// Drop all cached results.
    pub fn clear_cache(&mut self) {
        self.cache.clear();
        self.order.clear();
    }
}

/// The variable name a `TokenType::Var` token contributes, per
/// `options.element_qualified`. An empty literal name remains a name.
fn var_token_name(source_map: &SourceMap, tok: &Token, options: VarScanOptions) -> String {
    let text = source_map.token_text(*tok);
    // The `${…}` brace form substitutes nothing inside, so its content is the
    // variable's literal name — `${$n}` reads the variable *called* `$n`, and
    // `${arr($i)}` the element whose key is the two characters `$i` (tclsh
    // 9.0.4 / 8.6.14: `set {$n} v; set ${$n}` → `can't read "v"`, i.e. it read
    // `$n` and got `v`).  The sigil-stripped token text can't show that; the
    // raw span keeps the `${` prefix.  Applies to both naming modes — dropping
    // the `$` in the base-name mode would key the read on `n`.
    let braced = source_map.text(tok.span).starts_with("${");
    let name = if options.element_qualified {
        crate::naming::element_var_name_braced(text, braced)
    } else {
        crate::naming::normalise_var_name_braced(text, braced)
    };
    name.to_owned()
}

/// Index components borrow the original variable token's source allocation.
fn scan_original_index_reads(
    source_map: &SourceMap<'_>,
    token: &Token,
    registry: &CommandRegistry,
    options: VarScanOptions,
    config: LexerConfig,
    names: &mut BTreeSet<String>,
) {
    use tcl_lexer::word_parts::{SubstFlags, WordBody, decompose, scan_var_ref};
    let source = source_map.source_bytes();
    let Ok(Some(reference)) = scan_var_ref(source, token.span.start() as usize, config) else {
        return;
    };
    let Some(index) = reference.index else {
        return;
    };
    if let WordBody::Parts(parts) = decompose(index, SubstFlags::default(), config) {
        collect_original_index_parts(&parts, index, registry, options, config, names);
    }
}

fn collect_original_index_parts(
    parts: &[tcl_lexer::word_parts::WordPart<'_>],
    source: &[u8],
    registry: &CommandRegistry,
    options: VarScanOptions,
    config: LexerConfig,
    names: &mut BTreeSet<String>,
) {
    use tcl_lexer::word_parts::WordPart;
    for part in parts {
        match part {
            WordPart::Variable(reference) => {
                let Some(range) = reference.source_range_in(source, config) else {
                    continue;
                };
                let Ok(spelling) = std::str::from_utf8(&source[range]) else {
                    continue;
                };
                let raw = tcl_syntax::naming::var_reference_for_style(spelling, config.braced_var);
                let braced = spelling.starts_with("${");
                let name = if options.element_qualified {
                    crate::naming::element_var_name_braced(raw, braced)
                } else {
                    crate::naming::normalise_var_name_braced(raw, braced)
                };
                names.insert(name.to_owned());
                if let Some(index) = &reference.index {
                    collect_original_index_parts(index, source, registry, options, config, names);
                }
            }
            WordPart::Command(script) if options.recurse_cmd_substitutions => {
                if let Ok(script) = std::str::from_utf8(script) {
                    let lexical = VarScanOptions {
                        include_var_read_roles: false,
                        ..options
                    };
                    names.extend(scan_tokens(script, registry, lexical, false, config));
                }
            }
            WordPart::Expression(expression) => {
                if let Ok(expression) = std::str::from_utf8(expression) {
                    let lexical = VarScanOptions {
                        include_var_read_roles: false,
                        ..options
                    };
                    names.extend(scan_tokens(expression, registry, lexical, false, config));
                }
            }
            WordPart::Text(_) | WordPart::Command(_) | WordPart::ParseError(_) => {}
        }
    }
}

/// Scan `source` for `Var`/`Cmd` tokens, recursing into every `[…]`
/// substitution found.
///
/// `quoted_body` selects how `source` itself is tokenised:
///
/// - `true` — *value body* mode: `source` is already-extracted word/value
///   text whose own enclosing quotes (if any) were stripped by whatever
///   produced it, so it's tokenised via [`Lexer::as_quoted_body`] — no
///   top-level word-splitting/brace-quoting, since a `{`/`}` here is
///   ordinary literal content, not a fresh word boundary
///   (`set s "prefix {$vroot} suffix"` — the value word's `{$vroot}`
///   is an ordinary substitution, exactly like a bare `$x` beside it,
///   because braces have no grouping meaning *inside* an already-open
///   quoted string; re-tokenising the extracted text with ordinary
///   top-level rules instead mis-read it as a fresh, non-substituting
///   brace-quoted word).
/// - `false` — *command words* mode: `source` is a nested `[…]`
///   substitution's own inner content, itself a fresh Tcl command, so
///   ordinary top-level word-splitting/brace-quoting rules apply (Tcl
///   really does treat e.g. `[foo {$bar}]`'s `{$bar}` as a literal,
///   non-substituting argument — tclsh-verified).
///
/// Every `[…]` substitution found, in *either* mode, recurses in command-
/// words mode (`quoted_body: false`): its content is always a fresh
/// command, regardless of what mode enclosed it.
fn scan_tokens(
    source: &str,
    registry: &CommandRegistry,
    options: VarScanOptions,
    quoted_body: bool,
    config: LexerConfig,
) -> BTreeSet<String> {
    let mut vars_found = BTreeSet::new();
    let source_map = SourceMap::new(source);
    let mut lexer = Lexer::with_config(source, config);
    if quoted_body {
        lexer = lexer.as_quoted_body();
    }

    let Ok(tokens) = lexer.tokenise_all() else {
        return vars_found;
    };

    for tok in &tokens {
        match tok.kind {
            TokenType::Var => {
                vars_found.insert(var_token_name(&source_map, tok, options));
                scan_original_index_reads(
                    &source_map,
                    tok,
                    registry,
                    options,
                    config,
                    &mut vars_found,
                );
            }
            TokenType::Cmd if options.recurse_cmd_substitutions => {
                let text = source_map.token_text(*tok);
                if !text.is_empty() {
                    vars_found.extend(scan_tokens(text, registry, options, false, config));
                }
            }
            // `JimTcl`'s `$(…)`. Its body is an *expression*, not a nested
            // script, so its `$x` reads belong to the enclosing word exactly
            // as `$x` in a quoted word would — hence no
            // `recurse_cmd_substitutions` gate, which is about `[…]`.
            // Without this arm a variable read only inside `$(…)` is invisible
            // to every unused/rename/reference consumer.
            TokenType::ExprSugar => {
                let text = source_map.token_text(*tok);
                if !text.is_empty() {
                    vars_found.extend(scan_tokens(text, registry, options, false, config));
                }
            }
            _ => {}
        }
    }

    if options.include_var_read_roles {
        let role_vars = scan_standalone_var_read_role_names(
            source,
            registry,
            options.include_reads_before_write,
            options.element_qualified,
            config,
        );
        vars_found.extend(role_vars);
    }

    vars_found
}

/// Extract variable names from `ArgRole::VarRead` positions in commands.
///
/// This is a standalone function (not cached) that tokenises a script,
/// segments it into commands, and queries the registry for which argument
/// positions hold variable-read references.
fn scan_standalone_var_read_role_names(
    source: &str,
    registry: &CommandRegistry,
    include_rmw: bool,
    element_qualified: bool,
    config: LexerConfig,
) -> BTreeSet<String> {
    let mut result = BTreeSet::new();
    let source_map = SourceMap::new(source);
    let lexer = Lexer::with_config(source, config);

    let Ok(tokens) = lexer.tokenise_all() else {
        return result;
    };

    // Segment into commands by splitting on EOL/EOF.  Each word carries
    // whether it is a *brace-quoted literal* — a single `Str` token, the one
    // word form Tcl leaves entirely unsubstituted, so `unset {$n}` names the
    // variable literally called `$n` rather than reading `n`.
    let mut words: Vec<(String, bool)> = Vec::new();
    let mut prev_is_sep = true;

    let flush = |words: &mut Vec<(String, bool)>, result: &mut BTreeSet<String>| {
        if words.is_empty() {
            return;
        }
        let cmd_name = &words[0].0;
        let args: Vec<&str> = words[1..].iter().map(|(w, _)| w.as_str()).collect();
        let mut read_idx: Vec<usize> =
            registry.arg_indices_for_role(cmd_name, &args, ArgRole::VarRead);
        // A read-modify-write command (`incr` / `append` / `lappend`) reads
        // its `VarWrite` target's prior value, so report it as a read too.
        if include_rmw
            && registry
                .invocation_traits(cmd_name, &args, None)
                .contains(tcl_registry::Traits::READS_BEFORE_WRITE)
        {
            read_idx.extend(registry.arg_indices_for_role(cmd_name, &args, ArgRole::VarWrite));
        }
        for idx in read_idx {
            if idx < args.len() {
                let braced = words[idx + 1].1;
                let name = if element_qualified {
                    crate::naming::element_var_name_braced(args[idx], braced)
                } else {
                    crate::naming::normalise_var_name_braced(args[idx], braced)
                };
                result.insert(name.to_owned());
            }
        }
    };

    for tok in &tokens {
        match tok.kind {
            TokenType::Eol | TokenType::Eof => {
                flush(&mut words, &mut result);
                words.clear();
                prev_is_sep = true;
            }
            TokenType::Sep | TokenType::Comment => {
                prev_is_sep = true;
            }
            _ => {
                let text = source_map.token_text(*tok);
                if prev_is_sep {
                    words.push((text.to_owned(), tok.kind == TokenType::Str));
                } else if let Some(last) = words.last_mut() {
                    last.0.push_str(text);
                    // A second token in the word means it is not a single
                    // brace-quoted literal.
                    last.1 = false;
                } else {
                    words.push((text.to_owned(), tok.kind == TokenType::Str));
                }
                prev_is_sep = false;
            }
        }
    }
    flush(&mut words, &mut result);
    result
}

/// Extract variable names from Tcl words without caching.
///
/// Convenience function that creates a temporary scanner, scans the
/// text, and returns the result. For repeated scanning, prefer
/// creating a [`VarReferenceScanner`] and reusing it.
#[must_use]
pub fn vars_in_word(text: &str, registry: &CommandRegistry) -> BTreeSet<String> {
    let mut scanner = VarReferenceScanner::for_registry(
        VarScanOptions {
            include_var_read_roles: true,
            recurse_cmd_substitutions: true,
            include_reads_before_write: false,
            element_qualified: false,
        },
        registry,
    );
    scanner.scan_word(text, registry)
}

/// Extract variable names from an expression AST node, reading any `Raw`
/// operand text under the document's own lexer `grammar`.
///
/// Delegates to
/// [`ExprNode::vars_with_grammar()`](crate::expr_ast::ExprNode::vars_with_grammar).
#[must_use]
pub fn vars_in_expr(
    expr: &crate::expr_ast::ExprNode,
    grammar: tcl_dialect::LexerGrammar,
) -> BTreeSet<String> {
    expr.vars_with_grammar(grammar).into_iter().collect()
}

/// Exact original variable token and complete configured reference spelling.
/// The shared scanner owns the closer and original name range. This geometry
/// supplies no variable cell, lookup, read permission or standalone source owner.
pub(crate) fn original_variable_reference_form(
    source: &str,
    token: Token,
    config: LexerConfig,
) -> Option<(&str, bool)> {
    if token.kind != TokenType::Var {
        return None;
    }
    let start = usize::try_from(token.span.start()).ok()?;
    let reference =
        tcl_lexer::word_parts::scan_var_ref(source.as_bytes(), start, config).ok()??;
    if reference.source_span(source.as_bytes(), start, 0)? != token.span
        || reference.name_range_in(source.as_bytes())?.start
            != start.checked_add(usize::from(token.content_offset))?
    {
        return None;
    }
    let spelling = source.get(start..reference.next)?;
    tcl_syntax::naming::is_whole_variable_reference(spelling, config).then_some(())?;
    Some((
        tcl_syntax::naming::var_reference_for_style(spelling, config.braced_var),
        token.content_offset == 2,
    ))
}

fn collect_ref_forms(text: &str, out: &mut Vec<(String, bool)>, config: LexerConfig) {
    if text.is_empty() {
        return;
    }
    let source_map = SourceMap::new(text);
    let Ok(tokens) = Lexer::with_config(text, config).tokenise_all() else {
        return;
    };
    for tok in &tokens {
        match tok.kind {
            TokenType::Var => {
                if let Some((form, braced)) = original_variable_reference_form(text, *tok, config) {
                    out.push((form.to_owned(), braced));
                }
            }
            // `ExprSugar` is `JimTcl`'s `$(…)`. Its body is an *expression*
            // rather than a nested script, but either way the reference forms
            // inside it are read by the enclosing word, so both recurse.
            TokenType::Cmd | TokenType::ExprSugar => {
                let inner = source_map.token_text(*tok);
                if !inner.is_empty() {
                    collect_ref_forms(inner, out, config);
                }
            }
            _ => {}
        }
    }
}

/// Return the *full* variable-reference forms read in *text* — keeping the
/// array-index suffix that [`VarReferenceScanner`] normalises away.
///
/// `$a` → `"a"`, `$a(k)` → `"a(k)"`, `$state($whom)` → `"state($whom)"`.
/// Recurses into `[...]` command substitutions.  Unlike the name-set scanners,
/// this preserves enough structure for `var_resolve::resolve_place` to
/// distinguish a scalar from an array element from a dynamic ref.
/// `config` is the document's own [`LexerConfig`], so the `$`/`[…]`
/// boundaries this reports are the ones the document's grammar draws.
#[must_use]
pub fn scan_var_ref_forms_with_config(text: &str, config: LexerConfig) -> Vec<String> {
    scan_var_ref_forms_braced_with_config(text, config)
        .into_iter()
        .map(|(f, _)| f)
        .collect()
}

/// [`scan_var_ref_forms_with_config`], also reporting whether each reference used the
/// `${…}` **brace form**, whose content is a literal name — `${$n}` reads the
/// variable called `$n`, `${arr($i)}` the element whose key is the two
/// characters `$i`.
///
/// A consumer that canonicalises the form must pass the flag on to
/// [`crate::naming::element_var_name_braced`] rather than re-stripping a `$`
/// that is part of the name.
/// `config` is the document's own [`LexerConfig`].
#[must_use]
pub fn scan_var_ref_forms_braced_with_config(
    text: &str,
    config: LexerConfig,
) -> Vec<(String, bool)> {
    let mut out = Vec::new();
    collect_ref_forms(text, &mut out, config.nested().normalized());
    out
}

/// Return the inner text of every top-level `[...]` command substitution in
/// *text* (without the surrounding brackets).  Used by the place bridge to
/// recover the reads of commands nested in an argument word.
/// `config` is the document's own [`LexerConfig`], so the `[…]` boundaries it
/// reports are the ones the document's grammar draws.
#[must_use]
pub fn command_subst_texts_with_config(text: &str, config: LexerConfig) -> Vec<String> {
    let mut out = Vec::new();
    if !text.contains('[') {
        return out;
    }
    let config = config.nested().normalized();
    let source_map = SourceMap::new(text);
    let Ok(tokens) = Lexer::with_config(text, config).tokenise_all() else {
        return out;
    };
    for tok in &tokens {
        if tok.kind == TokenType::Cmd {
            let inner = source_map.token_text(*tok);
            if !inner.is_empty() {
                out.push(inner.to_owned());
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_registry() -> CommandRegistry {
        CommandRegistry::build_default()
    }

    #[test]
    fn source_name_roles_preserve_literal_sigils_and_empty_names() {
        let source = "set {$n} VALUE\nset {$n}\nset {} EMPTY\nset {}\n";
        let analysis = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
        let commands = crate::segmenter::segment_commands_with_offset_and_config(
            source,
            0,
            analysis.body_lexer_config.unwrap(),
        );
        let named = original_variable_name_role_words(source, &analysis, &commands[1]);
        assert!(named.iter().any(|word| word.role == ArgRole::VarRead
            && word.word == "$n"
            && word.base_name() == "$n"));
        let empty = original_variable_name_role_words(source, &analysis, &commands[3]);
        assert!(
            empty
                .iter()
                .any(|word| word.role == ArgRole::VarRead && word.word.is_empty())
        );
        assert!(
            original_variable_name_role_words("set changed VALUE\n", &analysis, &commands[1])
                .is_empty()
        );
    }

    #[test]
    fn a_source_replacement_does_not_inherit_builtin_variable_read_roles() {
        let source = "proc set {args} {return LITERAL}\nset k\n";
        let analysis = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
        let command = crate::segmenter::segment_commands_with_offset_and_config(
            source,
            0,
            analysis.body_lexer_config.unwrap(),
        )
        .pop()
        .unwrap();
        assert!(original_variable_name_role_words(source, &analysis, &command).is_empty());
    }

    #[test]
    fn lexical_and_standalone_role_results_have_distinct_cache_entries() {
        let registry = default_registry();
        let mut scanner = VarReferenceScanner::new(VarScanOptions::default());
        assert_eq!(
            scanner.scan_standalone_word("[set {}]", &registry),
            BTreeSet::from([String::new()])
        );
        assert!(scanner.scan_lexical_word("[set {}]", &registry).is_empty());
        assert_eq!(
            scanner.scan_standalone_word("[set {}]", &registry),
            BTreeSet::from([String::new()])
        );
    }

    #[test]
    fn original_array_index_components_include_nested_variable_reads() {
        let registry = default_registry();
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let mut scanner = VarReferenceScanner::with_config(
                VarScanOptions::default(),
                LexerConfig::for_dialect(dialect),
            );
            assert_eq!(
                scanner.scan_word("$a($b($k))", &registry),
                BTreeSet::from(["a".to_owned(), "b".to_owned(), "k".to_owned()])
            );
            assert_eq!(
                scanner.scan_word("$a([pick $k])", &registry),
                BTreeSet::from(["a".to_owned(), "k".to_owned()])
            );
        }
    }

    #[test]
    fn a_braced_variable_name_does_not_substitute_its_index_text() {
        let registry = default_registry();
        let mut scanner = VarReferenceScanner::new(VarScanOptions::default());
        assert_eq!(
            scanner.scan_word("${a($k)}", &registry),
            BTreeSet::from(["a".to_owned()])
        );
    }

    #[test]
    fn original_empty_variable_names_remain_present_reads() {
        let registry = default_registry();
        let mut scanner = VarReferenceScanner::new(VarScanOptions::default());
        assert_eq!(
            scanner.scan_word("${}", &registry),
            BTreeSet::from([String::new()])
        );
        assert!(scanner.scan_word("literal", &registry).is_empty());
    }

    #[test]
    fn index_words_do_not_infer_receiver_roles_from_a_written_head() {
        let registry = default_registry();
        let mut scanner = VarReferenceScanner::new(VarScanOptions {
            include_var_read_roles: true,
            ..VarScanOptions::default()
        });
        assert_eq!(
            scanner.scan_word("$a([set k])", &registry),
            BTreeSet::from(["a".to_owned()])
        );
    }

    #[test]
    fn scan_simple_var() {
        let reg = default_registry();
        let mut scanner = VarReferenceScanner::new(VarScanOptions::default());
        let vars = scanner.scan_word("$x", &reg);
        assert!(vars.contains("x"), "should find $x; got {vars:?}");
    }

    fn scan_var_ref_forms_cfg(text: &str) -> Vec<String> {
        scan_var_ref_forms_with_config(text, LexerConfig::default())
    }

    fn command_subst_texts_cfg(text: &str) -> Vec<String> {
        command_subst_texts_with_config(text, LexerConfig::default())
    }

    #[test]
    fn scan_var_ref_forms_keeps_array_index() {
        // Unlike the name scanner, forms preserve the `(idx)` suffix.
        assert_eq!(scan_var_ref_forms_cfg("$a(k)"), vec!["a(k)".to_owned()]);
        assert_eq!(scan_var_ref_forms_cfg("${a(k)}"), vec!["a(k)".to_owned()]);
        assert_eq!(scan_var_ref_forms_cfg("$x"), vec!["x".to_owned()]);
        assert_eq!(
            scan_var_ref_forms_cfg("$x + $a($i)"),
            vec!["x".to_owned(), "a($i)".to_owned()]
        );
        // recurses into command substitutions
        assert_eq!(scan_var_ref_forms_cfg("[foo $b]"), vec!["b".to_owned()]);
    }

    #[test]
    fn local_scans_normalize_offsets_bom_and_keep_jim_grammar() {
        let config = LexerConfig {
            base_offset: 91,
            base_line: 7,
            base_col: 13,
            leading_bom: tcl_lexer::LeadingBom::Skip,
            ..LexerConfig::for_dialect("jim")
        };
        assert_eq!(
            scan_var_ref_forms_with_config("$($a)", config),
            vec!["a".to_owned()]
        );

        let scanner = VarReferenceScanner::with_config(VarScanOptions::default(), config);
        assert_eq!(scanner.lexer_config().base_offset, 0);
        assert_eq!(scanner.lexer_config().base_line, 0);
        assert_eq!(scanner.lexer_config().base_col, 0);
        assert_eq!(
            scanner.lexer_config().leading_bom,
            tcl_lexer::LeadingBom::Content
        );
        assert_eq!(
            scanner.lexer_config().var_syntax,
            tcl_dialect::VarSyntax::Jim
        );
    }

    #[test]
    fn command_subst_texts_extracts_inner() {
        assert_eq!(
            command_subst_texts_cfg("foo [bar $x] baz"),
            vec!["bar $x".to_owned()]
        );
        assert_eq!(
            command_subst_texts_cfg("no subst here"),
            [] as [std::string::String; 0]
        );
    }

    #[test]
    fn scan_multiple_vars() {
        let reg = default_registry();
        let mut scanner = VarReferenceScanner::new(VarScanOptions::default());
        // Use unbraced expression so $a and $b are tokenised as VAR.
        let vars = scanner.scan_script("set result [expr $a + $b]", &reg);
        assert!(vars.contains("a"), "should find $a; got {vars:?}");
        assert!(vars.contains("b"), "should find $b; got {vars:?}");
    }

    #[test]
    fn scan_braced_var() {
        let reg = default_registry();
        let mut scanner = VarReferenceScanner::new(VarScanOptions::default());
        let vars = scanner.scan_word("${name}", &reg);
        assert!(vars.contains("name"), "should find ${{name}}");
    }

    #[test]
    fn scan_array_var() {
        let reg = default_registry();
        let mut scanner = VarReferenceScanner::new(VarScanOptions::default());
        let vars = scanner.scan_word("$arr(idx)", &reg);
        assert!(vars.contains("arr"), "should find array base name");
    }

    // A value-body word (a `set` value, a `foreach` list arg, …) may carry an
    // embedded `{…}` run that survived, as ordinary literal text, from an
    // originally double-quoted or bareword-concatenated source word — as in
    // tcllib's `modules/htmlparse/htmlparse.tcl`: `eval "$cmd {$vroot} {} {}
    // \{$html\}"`. Braces have no word-grouping meaning *inside* an
    // already-open quoted string (only `$`, `[`, `\`, and the closing quote
    // are special there), so `{$vroot}` is an ordinary substitution, exactly
    // like the bare `$cmd` beside it — tclsh9.0/8.6-verified.

    #[test]
    fn scan_word_finds_a_var_inside_braces_within_a_value_body() {
        // TP — the core case: `{$a}` survived from a double-quoted
        // source word, so `a` must be found, not swallowed as a
        // non-substituting brace-quoted word.
        let reg = default_registry();
        let mut scanner = VarReferenceScanner::new(VarScanOptions::default());
        let vars = scanner.scan_word("prefix {$a} suffix $b $c", &reg);
        assert!(
            vars.contains("a"),
            "should find $a inside {{$a}}; got {vars:?}"
        );
        assert!(
            vars.contains("b"),
            "should still find bare $b; got {vars:?}"
        );
        assert!(
            vars.contains("c"),
            "should still find bare $c; got {vars:?}"
        );
    }

    #[test]
    fn scan_word_still_recurses_a_nested_command_substitution_inside_a_value_body() {
        // A genuine `[…]` substitution embedded in a value body still
        // recurses (tclsh9.0/8.6-verified: `set x 1; set s "pre [bar $x]
        // post"` reads `x`).
        let reg = default_registry();
        let mut scanner = VarReferenceScanner::new(VarScanOptions::default());
        let vars = scanner.scan_word("pre [bar $x] post", &reg);
        assert!(
            vars.contains("x"),
            "should recurse into the nested [bar $x]; got {vars:?}"
        );
    }

    #[test]
    fn scan_word_keeps_brace_quoting_inside_a_nested_command_substitutions_own_words() {
        // TN — a nested `[…]` substitution's own content is a fresh Tcl
        // command: full word-splitting/brace-quoting rules apply there,
        // same as any top-level command (tclsh9.0/8.6-verified: `proc foo {a
        // b} {return "a=$a b=$b"}; puts [foo {$bar} world]` prints `a=$bar
        // b=world` — `{$bar}` is a literal, non-substituting argument, so
        // `bar` itself is never read).
        let reg = default_registry();
        let mut scanner = VarReferenceScanner::new(VarScanOptions::default());
        let vars = scanner.scan_word("[foo {$bar} baz]", &reg);
        assert!(
            !vars.contains("bar"),
            "a nested command's own {{$bar}} argument must stay literal; got {vars:?}"
        );
    }

    #[test]
    fn scan_script_keeps_brace_quoting_in_a_genuine_script_body_issue_1024() {
        // TN twin of `scan_word_finds_a_var_inside_braces_within_a_value_body`
        // — same `{$x}` text, opposite answer, because a *script* body is
        // ordinary Tcl: its `{…}` is real brace-quoting, not literal text
        // that survived from an enclosing quoted word. tclsh8.6/9.0: `set x
        // GLOBALX; eval {set literal {$x}}; puts [set literal]` prints `$x`
        // — unsubstituted.
        let reg = default_registry();
        let mut scanner = VarReferenceScanner::new(VarScanOptions::default());
        let vars = scanner.scan_script("set literal {$x}", &reg);
        assert!(
            !vars.contains("x"),
            "a script body's brace-quoted {{$x}} must stay literal; got {vars:?}"
        );
    }

    #[test]
    fn scan_script_still_finds_a_bare_substitution_in_a_script_body_issue_1024() {
        // TP guard for the same fix: dropping quoted-body mode must not
        // stop `scan_script` seeing ordinary substitutions.
        let reg = default_registry();
        let mut scanner = VarReferenceScanner::new(VarScanOptions::default());
        let vars = scanner.scan_script("set copy $x\nputs [foo $y]", &reg);
        assert!(vars.contains("x"), "should find bare $x; got {vars:?}");
        assert!(
            vars.contains("y"),
            "should recurse into [foo $y]; got {vars:?}"
        );
    }

    #[test]
    fn scan_word_and_scan_script_cache_the_same_text_separately_issue_1024() {
        // The LRU key must carry the mode: the same source text has two
        // legitimate answers, and whichever ran first must not poison the
        // other.
        let reg = default_registry();
        let mut scanner = VarReferenceScanner::new(VarScanOptions::default());
        let as_word = scanner.scan_word("set literal {$x}", &reg);
        let as_script = scanner.scan_script("set literal {$x}", &reg);
        assert!(
            as_word.contains("x"),
            "value-body mode substitutes {{$x}}; got {as_word:?}"
        );
        assert!(
            !as_script.contains("x"),
            "script mode must not inherit the cached value-body answer; got {as_script:?}"
        );
        // And the reverse order, on a fresh scanner.
        let mut scanner = VarReferenceScanner::new(VarScanOptions::default());
        let as_script = scanner.scan_script("set literal {$x}", &reg);
        let as_word = scanner.scan_word("set literal {$x}", &reg);
        assert!(!as_script.contains("x"), "got {as_script:?}");
        assert!(as_word.contains("x"), "got {as_word:?}");
    }

    #[test]
    fn scan_no_recurse() {
        let reg = default_registry();
        let mut scanner = VarReferenceScanner::new(VarScanOptions {
            include_var_read_roles: false,
            recurse_cmd_substitutions: false,
            include_reads_before_write: false,
            element_qualified: false,
        });
        // With recursion off, $inner inside [cmd $inner] should NOT be found.
        let vars = scanner.scan_word("[set x $inner]", &reg);
        assert!(
            !vars.contains("inner"),
            "should not recurse into cmd substitution"
        );
    }

    #[test]
    fn scan_with_recurse() {
        let reg = default_registry();
        let mut scanner = VarReferenceScanner::new(VarScanOptions {
            include_var_read_roles: false,
            recurse_cmd_substitutions: true,
            include_reads_before_write: false,
            element_qualified: false,
        });
        let vars = scanner.scan_word("[set x $inner]", &reg);
        assert!(
            vars.contains("inner"),
            "should recurse into cmd substitution"
        );
    }

    #[test]
    fn cache_works() {
        let reg = default_registry();
        let mut scanner = VarReferenceScanner::new(VarScanOptions::default());
        let vars1 = scanner.scan_word("$cached_var", &reg);
        let vars2 = scanner.scan_word("$cached_var", &reg);
        assert_eq!(vars1, vars2);
        assert_eq!(scanner.cache.len(), 1);
    }

    #[test]
    fn cache_eviction() {
        let reg = default_registry();
        let mut scanner = VarReferenceScanner::with_cache_size(VarScanOptions::default(), 2);
        scanner.scan_word("$a", &reg);
        scanner.scan_word("$b", &reg);
        scanner.scan_word("$c", &reg); // should evict $a
        assert_eq!(scanner.cache.len(), 2);
        assert!(
            !scanner.cache.contains_key(&(
                "$a".to_owned(),
                true,
                LexerConfig::default().nested().normalized(),
                false,
                None,
            )),
            "oldest entry should be evicted"
        );
    }

    #[test]
    fn empty_input() {
        let reg = default_registry();
        let mut scanner = VarReferenceScanner::new(VarScanOptions::default());
        let vars = scanner.scan_word("", &reg);
        assert_eq!(vars.len(), 0);
    }

    #[test]
    fn no_vars() {
        let reg = default_registry();
        let mut scanner = VarReferenceScanner::new(VarScanOptions::default());
        let vars = scanner.scan_word("hello world", &reg);
        assert_eq!(vars.len(), 0);
    }

    #[test]
    fn clear_cache() {
        let reg = default_registry();
        let mut scanner = VarReferenceScanner::new(VarScanOptions::default());
        scanner.scan_word("$x", &reg);
        assert_eq!(scanner.cache.len(), 1);
        scanner.clear_cache();
        assert_eq!(scanner.cache.len(), 0);
    }

    #[test]
    fn vars_in_expr_test() {
        use crate::expr_ast::ExprNode;
        let expr = ExprNode::Var {
            text: "$x".into(),
            name: "x".into(),
            start: 0,
            end: 2,
        };
        let vars = vars_in_expr(&expr, tcl_dialect::LexerGrammar::default());
        assert!(vars.contains("x"));
    }
}

#[cfg(test)]
mod original_reference_form_tests {
    use super::*;

    #[test]
    fn original_reference_forms_preserve_selected_closers_arrays_empty_and_literal_names() {
        for (dialect, spelling, expected) in [
            ("tcl8.4", "${a{b}c}", "a{b"),
            ("tcl9.0", "${a{b}c}", "a{b}c"),
            ("tcl9.0", "${a{b}}", "a{b}"),
            ("tcl8.6", "${$n}", "$n"),
            ("tcl8.6", "${}", ""),
            ("tcl8.6", "$array($index)", "array($index)"),
        ] {
            let config = LexerConfig::for_dialect(dialect);
            let token = Lexer::with_config(spelling, config)
                .tokenise_all()
                .unwrap()
                .into_iter()
                .find(|token| token.kind == TokenType::Var)
                .unwrap();
            let (form, _) = original_variable_reference_form(spelling, token, config).unwrap();
            assert_eq!(form, expected, "{dialect}: {spelling}");
            assert_eq!(
                scan_var_ref_forms_braced_with_config(spelling, config)[0].0,
                expected
            );
            let mut changed = token;
            changed.content_offset = 0;
            assert!(original_variable_reference_form(spelling, changed, config).is_none());
            changed = token;
            changed.span = Span::new(token.span.start(), token.span.end() - 1);
            assert!(original_variable_reference_form(spelling, changed, config).is_none());
        }
        let source = "${a{b}c}";
        let old = LexerConfig::for_dialect("tcl8.4");
        let token = Lexer::with_config(source, old).tokenise_all().unwrap()[0];
        assert!(
            original_variable_reference_form(source, token, LexerConfig::for_dialect("tcl9.0"))
                .is_none()
        );
    }
}
