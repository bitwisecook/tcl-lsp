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
    /// The word's text as written, delimiters already stripped by the
    /// segmenter — `m`, `arr(k)`, or, for a brace-quoted word, its content
    /// verbatim (`[set {$n}]` gives `$n`).  Raw so a consumer that needs the
    /// array element (`crate::naming::split_array_name_braced`) still has it;
    /// [`Self::base_name`] is the canonical cell name.
    pub word: String,
    /// The word's own source span, i.e. what a reference / highlight /
    /// rename provider points at.
    pub span: Span,
    /// The word is a single brace-quoted token, so Tcl substitutes nothing
    /// inside it and its content is the name verbatim (issue #1078).
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

/// Every argument of `cmd` whose registry role names a variable, in argument
/// order.
///
/// **The** answer to "which words of this command are variable names?", so
/// the analyser's reference recorder, the dead-store suppressor's
/// substitution scan, and the LSP's cursor resolver cannot disagree about a
/// command they were never taught by name. Roles come from
/// [`CommandRegistry::arg_indices_for_role`], so any spec that declares a
/// `VarRead` / `VarWrite` position contributes — `set` (one-argument read
/// form), `info exists`, `array get`, `unset`, `incr`, a dialect command, or
/// one added tomorrow.
///
/// A **computed** name (`set $n`, `incr [pick]`) is skipped: its cell is not
/// statically known, and the `$n` inside it is already an ordinary read the
/// token scan sees. A brace-quoted word is *not* computed — braces suppress
/// substitution, so `{$n}` is the literal name `$n`.
#[must_use]
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
            if crate::naming::normalise_var_name_braced(text, braced_literal).is_empty() {
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

/// Options controlling what a [`VarReferenceScanner`] looks for.
#[allow(clippy::struct_excessive_bools)] // option flags, not a state machine
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VarScanOptions {
    /// When `true`, also collect variable names passed as
    /// `ArgRole::VarRead` arguments to known commands.
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
/// Results are cached in a bounded LRU keyed by source text **and** scan
/// mode. The same word/script strings are scanned repeatedly across SSA,
/// GVN, and interprocedural passes, so caching avoids redundant
/// lexer creation and tokenisation. The mode is part of the key because
/// value-body and script-body scans of identical text give different
/// answers (`set literal {$x}` reads `x` as a value word, nothing as a
/// script) — issue #1024.
pub struct VarReferenceScanner {
    options: VarScanOptions,
    /// Bounded LRU: `order` tracks access recency, `cache` stores results.
    cache: HashMap<CacheKey, BTreeSet<String>>,
    order: VecDeque<CacheKey>,
    cache_size: usize,
}

/// LRU key: the scanned text plus the mode it was scanned in
/// (`quoted_body` — see [`scan_tokens`]).
type CacheKey = (String, bool);

impl VarReferenceScanner {
    /// Create a new scanner with the given options and default cache size.
    #[must_use]
    pub fn new(options: VarScanOptions) -> Self {
        Self {
            options,
            cache: HashMap::new(),
            order: VecDeque::new(),
            cache_size: DEFAULT_CACHE_SIZE,
        }
    }

    /// Create a new scanner with a custom cache size.
    #[must_use]
    pub fn with_cache_size(options: VarScanOptions, cache_size: usize) -> Self {
        Self {
            options,
            cache: HashMap::new(),
            order: VecDeque::new(),
            cache_size,
        }
    }

    /// Whether this scanner reports element-qualified names
    /// ([`VarScanOptions::element_qualified`]).
    #[must_use]
    pub fn element_qualified(&self) -> bool {
        self.options.element_qualified
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
    /// [`crate::naming::element_var_name_braced`] for the oracle: the content
    /// is the name verbatim, `$` and all (issue #1078).
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

    /// Scan a Tcl script for variable references (LRU-cached).
    ///
    /// `source` is a genuine script body (an `eval`/`catch`/`uplevel` body,
    /// a proc body), so ordinary top-level Tcl word-splitting and
    /// brace-quoting apply: a `{…}` word inside it really does suppress
    /// substitution (issue #1024). Use [`Self::scan_word`] for a value word.
    pub fn scan_script(&mut self, source: &str, registry: &CommandRegistry) -> BTreeSet<String> {
        self.scan_cached(source, registry, false)
    }

    /// Shared cache lookup/insert for both scan modes.
    fn scan_cached(
        &mut self,
        source: &str,
        registry: &CommandRegistry,
        quoted_body: bool,
    ) -> BTreeSet<String> {
        let key: CacheKey = (source.to_owned(), quoted_body);

        // Check cache.
        if let Some(cached) = self.cache.get(&key) {
            let result = cached.clone();
            // Move to end of LRU order.
            self.order.retain(|k| k != &key);
            self.order.push_back(key);
            return result;
        }

        let result = scan_tokens(source, registry, self.options, quoted_body);

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
/// `options.element_qualified`, or `None` for a degenerate/empty name.
fn var_token_name(source_map: &SourceMap, tok: &Token, options: VarScanOptions) -> Option<String> {
    let text = source_map.token_text(*tok);
    // The `${…}` brace form substitutes nothing inside, so its content is the
    // variable's literal name — `${$n}` reads the variable *called* `$n`, and
    // `${arr($i)}` the element whose key is the two characters `$i` (tclsh
    // 9.0.4 / 8.6.14: `set {$n} v; set ${$n}` → `can't read "v"`, i.e. it read
    // `$n` and got `v`).  The sigil-stripped token text can't show that; the
    // raw span keeps the `${` prefix.  Applies to both naming modes — dropping
    // the `$` in the base-name mode keyed the read on `n` (issue #1078).
    let braced = source_map.text(tok.span).starts_with("${");
    let name = if options.element_qualified {
        crate::naming::element_var_name_braced(text, braced)
    } else {
        crate::naming::normalise_var_name_braced(text, braced)
    };
    (!name.is_empty()).then(|| name.to_owned())
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
///   ordinary literal content, not a fresh word boundary (issue #923 idx
///   125: `set s "prefix {$vroot} suffix"` — the value word's `{$vroot}`
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
) -> BTreeSet<String> {
    let mut vars_found = BTreeSet::new();
    let source_map = SourceMap::new(source);
    // The document's grammar comes from the dialect-selected registry's own
    // profile (the route `dynamic_names::lexer_config_for` takes): a name
    // recovered under the wrong word grammar tracks the wrong cell.
    let mut lexer = Lexer::with_config(source, LexerConfig::for_profile(registry.profile()));
    if quoted_body {
        lexer = lexer.as_quoted_body();
    }

    let Ok(tokens) = lexer.tokenise_all() else {
        return vars_found;
    };

    for tok in &tokens {
        match tok.kind {
            TokenType::Var => {
                if let Some(name) = var_token_name(&source_map, tok, options) {
                    vars_found.insert(name);
                }
            }
            TokenType::Cmd if options.recurse_cmd_substitutions => {
                let text = source_map.token_text(*tok);
                if !text.is_empty() {
                    vars_found.extend(scan_tokens(text, registry, options, false));
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
                    vars_found.extend(scan_tokens(text, registry, options, false));
                }
            }
            _ => {}
        }
    }

    if options.include_var_read_roles {
        let role_vars = scan_var_read_role_names(
            source,
            registry,
            options.include_reads_before_write,
            options.element_qualified,
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
fn scan_var_read_role_names(
    source: &str,
    registry: &CommandRegistry,
    include_rmw: bool,
    element_qualified: bool,
) -> BTreeSet<String> {
    let mut result = BTreeSet::new();
    let source_map = SourceMap::new(source);
    let lexer = Lexer::with_config(source, LexerConfig::for_profile(registry.profile()));

    let Ok(tokens) = lexer.tokenise_all() else {
        return result;
    };

    // Segment into commands by splitting on EOL/EOF.  Each word carries
    // whether it is a *brace-quoted literal* — a single `Str` token, the one
    // word form Tcl leaves entirely unsubstituted, so `unset {$n}` names the
    // variable literally called `$n` rather than reading `n` (issue #1078).
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
                .get(cmd_name)
                .is_some_and(|s| s.traits.contains(tcl_registry::Traits::READS_BEFORE_WRITE))
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
                if !name.is_empty() {
                    result.insert(name.to_owned());
                }
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
    let mut scanner = VarReferenceScanner::new(VarScanOptions {
        include_var_read_roles: true,
        recurse_cmd_substitutions: true,
        include_reads_before_write: false,
        element_qualified: false,
    });
    scanner.scan_word(text, registry)
}

/// Extract variable names from an expression AST node.
///
/// Delegates to [`ExprNode::vars()`](crate::expr_ast::ExprNode::vars).
#[must_use]
pub fn vars_in_expr(expr: &crate::expr_ast::ExprNode) -> BTreeSet<String> {
    expr.vars().into_iter().collect()
}

/// De-sigil a `VAR`-token's source text, keeping any array-index suffix.
///
/// `$a(k)` → `a(k)`, `${a(k)}` → `a(k)`, `${a}` → `a`, `$a` → `a`.  Unlike
/// [`normalise_var_name`], the `(idx)` suffix is preserved so the form can be
/// classified as a scalar vs an array element by `var_resolve::resolve_place`.
fn deref_form(text: &str) -> &str {
    if let Some(inner) = text.strip_prefix("${") {
        inner.strip_suffix('}').unwrap_or(inner)
    } else {
        text.strip_prefix('$').unwrap_or(text)
    }
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
                // `token_text` already drops the `$` / `${` decoration, so the
                // remainder *is* the form.  Re-running `deref_form` over it
                // would strip a second sigil that belongs to the name itself:
                // `${$n}` reads the variable literally called `$n` (issue
                // #1078), whose de-decorated text is `$n`, not `n`.  The
                // `${…}` form's content is literal, which the flag carries so
                // consumers do not re-normalise it either.
                let braced = source_map.text(tok.span).starts_with("${");
                let form = source_map.token_text(*tok);
                let form = if braced { form } else { deref_form(form) };
                if !form.is_empty() {
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
#[must_use]
pub fn scan_var_ref_forms(text: &str) -> Vec<String> {
    // dialect-drift-ok: compatibility shim for the call sites outside this
    // lane (analyser, place_bridge); `scan_var_ref_forms_with_config` is what
    // a caller holding the document's config uses.
    scan_var_ref_forms_with_config(text, LexerConfig::default())
}

/// [`scan_var_ref_forms`] under the document's own [`LexerConfig`].
#[must_use]
pub fn scan_var_ref_forms_with_config(text: &str, config: LexerConfig) -> Vec<String> {
    scan_var_ref_forms_braced_with_config(text, config)
        .into_iter()
        .map(|(f, _)| f)
        .collect()
}

/// [`scan_var_ref_forms`], also reporting whether each reference used the
/// `${…}` **brace form**, whose content is a literal name — `${$n}` reads the
/// variable called `$n`, `${arr($i)}` the element whose key is the two
/// characters `$i` (issue #1078).
///
/// A consumer that canonicalises the form must pass the flag on to
/// [`crate::naming::element_var_name_braced`] rather than re-stripping a `$`
/// that is part of the name.
#[must_use]
pub fn scan_var_ref_forms_braced(text: &str) -> Vec<(String, bool)> {
    // dialect-drift-ok: compatibility shim for the call sites outside this
    // lane (analyser, def_use); the `_with_config` form below is what a
    // caller holding the document's config uses.
    scan_var_ref_forms_braced_with_config(text, LexerConfig::default())
}

/// [`scan_var_ref_forms_braced`] under the document's own [`LexerConfig`].
#[must_use]
pub fn scan_var_ref_forms_braced_with_config(
    text: &str,
    config: LexerConfig,
) -> Vec<(String, bool)> {
    let mut out = Vec::new();
    collect_ref_forms(text, &mut out, config);
    out
}

/// Return the inner text of every top-level `[...]` command substitution in
/// *text* (without the surrounding brackets).  Used by the place bridge to
/// recover the reads of commands nested in an argument word.
#[must_use]
pub fn command_subst_texts(text: &str) -> Vec<String> {
    // dialect-drift-ok: compatibility shim for the call sites outside this
    // lane (place_bridge); `command_subst_texts_with_config` is what a caller
    // holding the document's config uses.
    command_subst_texts_with_config(text, LexerConfig::default())
}

/// [`command_subst_texts`] under the document's own [`LexerConfig`], so the
/// `[…]` boundaries it reports are the ones the document's grammar draws.
#[must_use]
pub fn command_subst_texts_with_config(text: &str, config: LexerConfig) -> Vec<String> {
    let mut out = Vec::new();
    if !text.contains('[') {
        return out;
    }
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
    fn scan_simple_var() {
        let reg = default_registry();
        let mut scanner = VarReferenceScanner::new(VarScanOptions::default());
        let vars = scanner.scan_word("$x", &reg);
        assert!(vars.contains("x"), "should find $x; got {vars:?}");
    }

    #[test]
    fn scan_var_ref_forms_keeps_array_index() {
        // Unlike the name scanner, forms preserve the `(idx)` suffix.
        assert_eq!(scan_var_ref_forms("$a(k)"), vec!["a(k)".to_owned()]);
        assert_eq!(scan_var_ref_forms("${a(k)}"), vec!["a(k)".to_owned()]);
        assert_eq!(scan_var_ref_forms("$x"), vec!["x".to_owned()]);
        assert_eq!(
            scan_var_ref_forms("$x + $a($i)"),
            vec!["x".to_owned(), "a($i)".to_owned()]
        );
        // recurses into command substitutions
        assert_eq!(scan_var_ref_forms("[foo $b]"), vec!["b".to_owned()]);
    }

    #[test]
    fn command_subst_texts_extracts_inner() {
        assert_eq!(
            command_subst_texts("foo [bar $x] baz"),
            vec!["bar $x".to_owned()]
        );
        assert!(command_subst_texts("no subst here").is_empty());
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

    // Issue #923 idx 125: a value-body word (a `set` value, a `foreach` list
    // arg, …) may carry an embedded `{…}` run that survived, as ordinary
    // literal text, from an originally double-quoted or bareword-
    // concatenated source word — real tcllib repro
    // (`modules/htmlparse/htmlparse.tcl`): `eval "$cmd {$vroot} {} {}
    // \{$html\}"`. Braces have no word-grouping meaning *inside* an
    // already-open quoted string (only `$`, `[`, `\`, and the closing quote
    // are special there), so `{$vroot}` is an ordinary substitution, exactly
    // like the bare `$cmd` beside it — tclsh9.0/8.6-verified.

    #[test]
    fn scan_word_finds_a_var_inside_braces_within_a_value_body() {
        // TP — the core idx 125 fix: `{$a}` survived from a double-quoted
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
            !scanner.cache.contains_key(&("$a".to_owned(), true)),
            "oldest entry should be evicted"
        );
    }

    #[test]
    fn empty_input() {
        let reg = default_registry();
        let mut scanner = VarReferenceScanner::new(VarScanOptions::default());
        let vars = scanner.scan_word("", &reg);
        assert!(vars.is_empty());
    }

    #[test]
    fn no_vars() {
        let reg = default_registry();
        let mut scanner = VarReferenceScanner::new(VarScanOptions::default());
        let vars = scanner.scan_word("hello world", &reg);
        assert!(vars.is_empty());
    }

    #[test]
    fn clear_cache() {
        let reg = default_registry();
        let mut scanner = VarReferenceScanner::new(VarScanOptions::default());
        scanner.scan_word("$x", &reg);
        assert_eq!(scanner.cache.len(), 1);
        scanner.clear_cache();
        assert!(scanner.cache.is_empty());
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
        let vars = vars_in_expr(&expr);
        assert!(vars.contains("x"));
    }
}
