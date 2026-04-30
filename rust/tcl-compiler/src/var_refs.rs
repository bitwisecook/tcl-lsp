//! Variable-reference scanning utilities for compiler passes.
//!
//! [`VarReferenceScanner`] scans Tcl words and scripts for referenced
//! variable names.  It tokenises the input with the Rust lexer, collects
//! `VAR` tokens, and optionally recurses into command substitutions.
//!
//! Results are cached in a bounded LRU keyed by source text — the same
//! word/script strings are scanned repeatedly across SSA, GVN, and
//! interprocedural passes.  This is the Rust port of Python's
//! `core/compiler/var_refs.py`.

use std::collections::{BTreeSet, HashMap, VecDeque};

use tcl_lexer::{Lexer, SourceMap, TokenType};
use tcl_registry::{ArgRole, CommandRegistry};

use crate::naming::normalise_var_name;

/// Options controlling what a [`VarReferenceScanner`] looks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VarScanOptions {
    /// When `true`, also collect variable names passed as
    /// `ArgRole::VarRead` arguments to known commands.
    pub include_var_read_roles: bool,
    /// When `true`, recurse into `[…]` command substitutions.
    pub recurse_cmd_substitutions: bool,
}

impl Default for VarScanOptions {
    fn default() -> Self {
        Self {
            include_var_read_roles: false,
            recurse_cmd_substitutions: true,
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

    /// Scan without cache — called on cache miss.
    fn scan_script_uncached(
        &mut self,
        source: &str,
        registry: &CommandRegistry,
    ) -> BTreeSet<String> {
        let mut vars_found = BTreeSet::new();
        let source_map = SourceMap::new(source);
        let lexer = Lexer::new(source);

        let Ok(tokens) = lexer.tokenise_all() else {
            return vars_found;
        };

        for tok in &tokens {
            match tok.kind {
                TokenType::Var => {
                    let text = source_map.token_text(*tok);
                    let name = normalise_var_name(text);
                    if !name.is_empty() {
                        vars_found.insert(name.to_owned());
                    }
                }
                TokenType::Cmd if self.options.recurse_cmd_substitutions => {
                    let text = source_map.token_text(*tok);
                    if !text.is_empty() {
                        let nested = self.scan_script(text, registry);
                        vars_found.extend(nested);
                    }
                }
                _ => {}
            }
        }

        if self.options.include_var_read_roles {
            let role_vars = scan_var_read_role_names(source, registry);
            vars_found.extend(role_vars);
        }

        vars_found
    }
}

/// Extract variable names from `ArgRole::VarRead` positions in commands.
///
/// This is a standalone function (not cached) that tokenises a script,
/// segments it into commands, and queries the registry for which argument
/// positions hold variable-read references.
fn scan_var_read_role_names(source: &str, registry: &CommandRegistry) -> BTreeSet<String> {
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
        for idx in registry.arg_indices_for_role(cmd_name, &args, ArgRole::VarRead) {
            if idx < args.len() {
                let name = normalise_var_name(args[idx]);
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

    #[test]
    fn scan_no_recurse() {
        let reg = default_registry();
        let mut scanner = VarReferenceScanner::new(VarScanOptions {
            include_var_read_roles: false,
            recurse_cmd_substitutions: false,
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
