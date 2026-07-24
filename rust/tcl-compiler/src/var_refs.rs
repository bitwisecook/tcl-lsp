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
//! Results are cached in a bounded LRU keyed by source text — the same
//! word/script strings are scanned repeatedly across SSA, GVN, and
//! interprocedural passes.

use std::collections::{BTreeSet, HashMap, VecDeque};

use tcl_lexer::{Lexer, SourceMap, Token, TokenType};
use tcl_registry::{ArgRole, CommandRegistry};

use crate::naming::{element_var_name, normalise_var_name};

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
/// Results are cached in a bounded LRU keyed by source text.
/// The same word/script strings are scanned repeatedly across SSA,
/// GVN, and interprocedural passes, so caching avoids redundant
/// lexer creation and tokenisation.
pub struct VarReferenceScanner {
    options: VarScanOptions,
    /// Bounded LRU: `order` tracks access recency, `cache` stores results.
    cache: HashMap<String, BTreeSet<String>>,
    order: VecDeque<String>,
    cache_size: usize,
}

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
        if self.options.element_qualified {
            element_var_name(raw)
        } else {
            normalise_var_name(raw)
        }
    }

    /// Scan one Tcl word for variable references.
    pub fn scan_word(&mut self, text: &str, registry: &CommandRegistry) -> BTreeSet<String> {
        self.scan_script(text, registry)
    }

    /// Scan a Tcl script for variable references (LRU-cached).
    pub fn scan_script(&mut self, source: &str, registry: &CommandRegistry) -> BTreeSet<String> {
        // Check cache.
        if let Some(cached) = self.cache.get(source) {
            let result = cached.clone();
            // Move to end of LRU order.
            self.order.retain(|k| k != source);
            self.order.push_back(source.to_owned());
            return result;
        }

        let result = self.scan_script_uncached(source, registry);

        // Insert into cache.
        let key = source.to_owned();
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

    /// Scan without cache — called on cache miss. The outermost scan is
    /// always over *value body* text (see [`scan_tokens`]): whatever
    /// produced `source` (a `set` value, a proc-arg default, …) already
    /// stripped its own enclosing quotes/braces, so any `{`/`}` surviving
    /// in `source` is literal content, never a fresh word boundary.
    fn scan_script_uncached(
        &mut self,
        source: &str,
        registry: &CommandRegistry,
    ) -> BTreeSet<String> {
        scan_tokens(source, registry, self.options, true)
    }
}

/// The variable name a `TokenType::Var` token contributes, per
/// `options.element_qualified`, or `None` for a degenerate/empty name.
fn var_token_name(source_map: &SourceMap, tok: &Token, options: VarScanOptions) -> Option<String> {
    let text = source_map.token_text(*tok);
    let name = if options.element_qualified {
        // The `${…}` brace form substitutes nothing inside, so its key is
        // literal even when it spells `$x` — the sigil-stripped token text
        // can't show that, but the raw span keeps the `${` prefix.
        let braced = source_map.text(tok.span).starts_with("${");
        crate::naming::element_var_name_braced(text, braced)
    } else {
        normalise_var_name(text)
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
    let mut lexer = Lexer::new(source);
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
    let lexer = Lexer::new(source);

    let Ok(tokens) = lexer.tokenise_all() else {
        return result;
    };

    // Segment into commands by splitting on EOL/EOF.
    let mut words: Vec<String> = Vec::new();
    let mut prev_is_sep = true;

    let flush = |words: &mut Vec<String>, result: &mut BTreeSet<String>| {
        if words.is_empty() {
            return;
        }
        let cmd_name = &words[0];
        let args: Vec<&str> = words[1..].iter().map(String::as_str).collect();
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
                let name = if element_qualified {
                    element_var_name(args[idx])
                } else {
                    normalise_var_name(args[idx])
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
                    words.push(text.to_owned());
                } else if let Some(last) = words.last_mut() {
                    last.push_str(text);
                } else {
                    words.push(text.to_owned());
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

fn collect_ref_forms(text: &str, out: &mut Vec<String>) {
    if text.is_empty() {
        return;
    }
    let source_map = SourceMap::new(text);
    let Ok(tokens) = Lexer::new(text).tokenise_all() else {
        return;
    };
    for tok in &tokens {
        match tok.kind {
            TokenType::Var => {
                let form = deref_form(source_map.token_text(*tok));
                if !form.is_empty() {
                    out.push(form.to_owned());
                }
            }
            TokenType::Cmd => {
                let inner = source_map.token_text(*tok);
                if !inner.is_empty() {
                    collect_ref_forms(inner, out);
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
    let mut out = Vec::new();
    collect_ref_forms(text, &mut out);
    out
}

/// Return the inner text of every top-level `[...]` command substitution in
/// *text* (without the surrounding brackets).  Used by the place bridge to
/// recover the reads of commands nested in an argument word.
#[must_use]
pub fn command_subst_texts(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    if !text.contains('[') {
        return out;
    }
    let source_map = SourceMap::new(text);
    let Ok(tokens) = Lexer::new(text).tokenise_all() else {
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
            !scanner.cache.contains_key("$a"),
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
