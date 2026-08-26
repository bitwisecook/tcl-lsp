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

//! Reconstruct the *name patterns* an iRule could build for a dynamic object
//! attachment, so the orphan analysis can filter candidate objects instead of
//! demoting whole object types wholesale.
//!
//! When an iRule attaches a pool / node / snatpool with a computed name
//! (`pool "web_[HTTP::host]"`, `snatpool $sp`, `pool "${prefix}_pool"`), the
//! static reference graph cannot resolve *which* object is meant. But the
//! *literal fragments* of the name expression still constrain the set of objects
//! it could possibly construct: `pool "web_[HTTP::host]"` can only ever build a
//! name that starts with `web_`, so a pool called `db_backend` is still provably
//! unreachable from that rule.
//!
//! The analysis walks the real [`tcl_compiler`] IR — so `when` handlers, nested
//! `if` / `switch` / loop bodies and command boundaries are exactly what Tcl
//! sees — and performs a light in-body constant propagation over `set` (the
//! "lattice steps": a name assembled in a variable and then attached is resolved
//! back through the variable, even to a single constant). Each attach argument
//! is re-tokenised with [`tcl_lexer`] and reconstructed into an ordered
//! prefix / contained / suffix [`AttachPattern`]. A pattern with no literal
//! anchor at all (`pool $x`, `pool [class match …]`) is `unconstrained` and
//! matches every object of its type — the safe fall-back that reproduces the old
//! all-or-nothing behaviour only when nothing better can be proven.

use std::collections::HashMap;

use serde_json::{Value, json};
use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_compiler::ir::{Script, Statement};
use tcl_dialect::DialectProfile;
use tcl_lexer::{Lexer, LexerConfig, SourceMap, TokenType};

/// Cap on patterns collected per object type per rule body — a runaway-input
/// backstop far above any real iRule.
const MAX_PATTERNS: usize = 128;

/// One segment of a reconstructed name: a fixed literal, or an unknown gap
/// produced by a `$var` / `[cmd]` substitution.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Seg {
    Lit(String),
    Wild,
}

/// Merge adjacent literals, drop empty literals, collapse runs of wildcards.
fn normalise(segs: Vec<Seg>) -> Vec<Seg> {
    let mut out: Vec<Seg> = Vec::new();
    for s in segs {
        match (&s, out.last_mut()) {
            (Seg::Lit(t), _) if t.is_empty() => {}
            (Seg::Lit(t), Some(Seg::Lit(prev))) => prev.push_str(t),
            (Seg::Wild, Some(Seg::Wild)) => {}
            _ => out.push(s),
        }
    }
    out
}

/// A reconstructed object-name pattern from a single dynamic attach expression.
///
/// Matching semantics are a glob where each [`Seg::Wild`] is `*` (matching zero
/// or more characters): the name must start with `prefix`, end with `suffix`,
/// and contain each `contains` fragment in order in between. An `exact` pattern
/// (the name resolved to a constant) matches only that literal; an
/// `unconstrained` pattern (no literal anchor) matches everything.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttachPattern {
    /// The source text of the attach argument, for display.
    pub raw: String,
    /// Required leading literal (empty when the name starts with a substitution).
    pub prefix: String,
    /// Interior literals that must appear in order.
    pub contains: Vec<String>,
    /// Required trailing literal (empty when the name ends with a substitution).
    pub suffix: String,
    /// True when the expression resolved to a single constant (`prefix` is the
    /// whole name and matching requires equality).
    pub exact: bool,
    /// True when the expression carries no literal anchor at all, so any object
    /// of the type could be the target.
    pub unconstrained: bool,
}

impl AttachPattern {
    fn from_segs(segs: Vec<Seg>, raw: String) -> Self {
        let segs = normalise(segs);
        let has_lit = segs.iter().any(|s| matches!(s, Seg::Lit(_)));
        let has_wild = segs.iter().any(|s| matches!(s, Seg::Wild));
        if !has_lit {
            return Self {
                raw,
                prefix: String::new(),
                contains: Vec::new(),
                suffix: String::new(),
                exact: false,
                unconstrained: true,
            };
        }
        if !has_wild {
            // Fully resolved to a constant name (via `set`), which the static
            // reference graph missed — record it as an exact match.
            let lit: String = segs
                .into_iter()
                .map(|s| match s {
                    Seg::Lit(t) => t,
                    Seg::Wild => String::new(),
                })
                .collect();
            return Self {
                raw,
                prefix: lit,
                contains: Vec::new(),
                suffix: String::new(),
                exact: true,
                unconstrained: false,
            };
        }
        let mut prefix = String::new();
        let mut suffix = String::new();
        let mut contains: Vec<String> = Vec::new();
        let n = segs.len();
        for (i, s) in segs.into_iter().enumerate() {
            if let Seg::Lit(t) = s {
                if i == 0 {
                    prefix = t;
                } else if i == n - 1 {
                    suffix = t;
                } else {
                    contains.push(t);
                }
            }
        }
        Self {
            raw,
            prefix,
            contains,
            suffix,
            exact: false,
            unconstrained: false,
        }
    }

    /// Does `name` fall within the set of names this expression could build?
    #[must_use]
    pub fn matches(&self, name: &str) -> bool {
        if self.unconstrained {
            return true;
        }
        if self.exact {
            return name == self.prefix;
        }
        let mut hay = name;
        if !self.prefix.is_empty() {
            match hay.strip_prefix(self.prefix.as_str()) {
                Some(rest) => hay = rest,
                None => return false,
            }
        }
        if !self.suffix.is_empty() {
            match hay.strip_suffix(self.suffix.as_str()) {
                Some(rest) => hay = rest,
                None => return false,
            }
        }
        for frag in &self.contains {
            match hay.find(frag.as_str()) {
                Some(i) => hay = &hay[i + frag.len()..],
                None => return false,
            }
        }
        true
    }

    /// A human-readable glob (`web_pool`, `web_*`, `*_pool`, `a*b*c`, `*`).
    #[must_use]
    pub fn glob(&self) -> String {
        if self.unconstrained {
            return "*".to_string();
        }
        if self.exact {
            return self.prefix.clone();
        }
        let mut g = String::new();
        g.push_str(&self.prefix);
        g.push('*');
        for c in &self.contains {
            g.push_str(c);
            g.push('*');
        }
        g.push_str(&self.suffix);
        g
    }

    /// Serialise for the `PyO3` / JSON boundary.
    #[must_use]
    pub fn to_json(&self) -> Value {
        json!({
            "raw": self.raw,
            "prefix": self.prefix,
            "contains": self.contains,
            "suffix": self.suffix,
            "glob": self.glob(),
            "exact": self.exact,
            "unconstrained": self.unconstrained,
        })
    }
}

/// The dynamic-attachment reach of one iRule body: the name patterns it could
/// build for each attachable object type.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AttachReach {
    pub pools: Vec<AttachPattern>,
    pub nodes: Vec<AttachPattern>,
    pub snatpools: Vec<AttachPattern>,
}

impl AttachReach {
    /// Patterns for a device-model type key (`"pools"` / `"nodes"` /
    /// `"snatpools"`); empty slice for any other key.
    #[must_use]
    pub fn patterns_for(&self, type_key: &str) -> &[AttachPattern] {
        match type_key {
            "pools" => &self.pools,
            "nodes" => &self.nodes,
            "snatpools" => &self.snatpools,
            _ => &[],
        }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.pools.is_empty() && self.nodes.is_empty() && self.snatpools.is_empty()
    }

    fn bucket_mut(&mut self, type_key: &str) -> &mut Vec<AttachPattern> {
        match type_key {
            "pools" => &mut self.pools,
            "nodes" => &mut self.nodes,
            _ => &mut self.snatpools,
        }
    }

    fn push(&mut self, type_key: &str, pat: AttachPattern) {
        let bucket = self.bucket_mut(type_key);
        if bucket.len() < MAX_PATTERNS && !bucket.contains(&pat) {
            bucket.push(pat);
        }
    }

    /// Serialise for the `PyO3` / JSON boundary.
    #[must_use]
    pub fn to_json(&self) -> Value {
        let ser = |v: &[AttachPattern]| -> Value {
            Value::Array(v.iter().map(AttachPattern::to_json).collect())
        };
        json!({
            "pools": ser(&self.pools),
            "nodes": ser(&self.nodes),
            "snatpools": ser(&self.snatpools),
        })
    }
}

/// What an in-body variable is known to hold.
#[derive(Clone, PartialEq, Eq)]
enum EnvVal {
    /// A concrete name shape (at least one literal anchor).
    Shape(Vec<Seg>),
    /// Conflicting or fully-dynamic assignments — resolves to a wildcard.
    Unknown,
}

/// Extract the variable name a `$…` token references, for env lookup. Strips the
/// leading `$` / `${…}` and any array `(index)` suffix.
fn var_name(tok_text: &str) -> &str {
    let s = tok_text.strip_prefix('$').unwrap_or(tok_text);
    let s = s
        .strip_prefix('{')
        .map_or(s, |inner| inner.strip_suffix('}').unwrap_or(inner));
    match s.find('(') {
        Some(i) => &s[..i],
        None => s,
    }
}

/// Re-tokenise an argument / value string into name segments, resolving `$var`
/// references against `env`. The whole string is treated as one word.
fn text_to_segs(text: &str, env: &HashMap<String, EnvVal>) -> Vec<Seg> {
    // iRule attach reconstruction always operates on iRule bodies, so lex with
    // the f5-irules preset (`}{` valid) to match TMM.
    let Ok(tokens) =
        Lexer::with_source_map(SourceMap::new(text), LexerConfig::for_dialect("f5-irules"))
            .tokenise_all()
    else {
        return vec![Seg::Wild];
    };
    let mut segs: Vec<Seg> = Vec::new();
    for tok in &tokens {
        let s = (tok.span.start() as usize).min(text.len());
        let e = (tok.span.end() as usize).min(text.len());
        let t = &text[s..e];
        match tok.kind {
            // Braced literal or plain text fragment — fixed text.
            TokenType::Str | TokenType::Esc => segs.push(Seg::Lit(t.to_string())),
            TokenType::Cmd => segs.push(Seg::Wild),
            TokenType::Var => match env.get(var_name(t)) {
                Some(EnvVal::Shape(inner)) => segs.extend(inner.iter().cloned()),
                _ => segs.push(Seg::Wild),
            },
            _ => {}
        }
    }
    normalise(segs)
}

/// Record `var = segs`, demoting to [`EnvVal::Unknown`] on a conflicting
/// reassign (conservative: a variable with two different shapes constrains
/// nothing).
fn update_env(env: &mut HashMap<String, EnvVal>, var: &str, segs: Vec<Seg>) {
    let val = if segs.iter().any(|s| matches!(s, Seg::Lit(_))) {
        EnvVal::Shape(segs)
    } else {
        EnvVal::Unknown
    };
    match env.get(var) {
        Some(existing) if *existing == val => {}
        Some(_) => {
            env.insert(var.to_string(), EnvVal::Unknown);
        }
        None => {
            env.insert(var.to_string(), val);
        }
    }
}

/// The three attachable object types, keyed by the source command name and the
/// device-model container the pattern filters.
fn attach_type(command: &str) -> Option<&'static str> {
    match command.trim_start_matches("::") {
        "pool" => Some("pools"),
        "node" => Some("nodes"),
        "snatpool" => Some("snatpools"),
        _ => None,
    }
}

/// Walk a lowered IR script, threading the constant-propagation env and
/// collecting attach patterns. Variables are function-scoped, so nested blocks
/// share the enclosing proc's env.
fn walk_script(script: &Script, env: &mut HashMap<String, EnvVal>, reach: &mut AttachReach) {
    for st in &script.statements {
        match st {
            Statement::AssignConst { name, value, .. }
            | Statement::AssignValue { name, value, .. } => {
                let segs = text_to_segs(value, env);
                update_env(env, name, segs);
            }
            Statement::AssignExpr { name, .. } | Statement::Incr { name, .. } => {
                update_env(env, name, vec![Seg::Wild]);
            }
            Statement::Call { command, args, .. } => {
                // `set var value` that survived as a generic call.
                if command.trim_start_matches("::") == "set" && args.len() >= 2 {
                    let segs = text_to_segs(&args[1], env);
                    update_env(env, &args[0], segs);
                } else if let (Some(ty), Some(arg)) = (attach_type(command), args.first()) {
                    // Gate on a genuinely dynamic source argument; a literal
                    // `pool /Common/x` is already a static reference.
                    if arg.contains('$') || arg.contains('[') {
                        let pat = AttachPattern::from_segs(text_to_segs(arg, env), arg.clone());
                        reach.push(ty, pat);
                    }
                }
            }
            Statement::If {
                clauses, else_body, ..
            } => {
                for c in clauses {
                    walk_script(&c.body, env, reach);
                }
                if let Some(body) = else_body {
                    walk_script(body, env, reach);
                }
            }
            Statement::For {
                init, next, body, ..
            } => {
                walk_script(init, env, reach);
                walk_script(body, env, reach);
                walk_script(next, env, reach);
            }
            Statement::While { body, .. }
            | Statement::Foreach { body, .. }
            | Statement::Catch { body, .. }
            | Statement::Block { body, .. }
            | Statement::UpFrame { body, .. } => {
                walk_script(body, env, reach);
            }
            Statement::Try {
                body,
                handlers,
                finally_body,
                ..
            } => {
                walk_script(body, env, reach);
                for h in handlers {
                    walk_script(&h.body, env, reach);
                }
                if let Some(f) = finally_body {
                    walk_script(f, env, reach);
                }
            }
            Statement::Switch {
                arms, default_body, ..
            } => {
                for a in arms {
                    if let Some(b) = &a.body {
                        walk_script(b, env, reach);
                    }
                }
                if let Some(b) = default_body {
                    walk_script(b, env, reach);
                }
            }
            _ => {}
        }
    }
}

/// Analyse one iRule body and return the name patterns it could dynamically
/// build for each attachable object type.
#[must_use]
pub fn attach_reach(source: &str) -> AttachReach {
    let mut reach = AttachReach::default();
    if source.is_empty() {
        return reach;
    }
    let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    let cu = CompilationUnit::build_for_profile(source, registry, false, DialectProfile::irules());

    // Each `when` handler (and any user proc) is its own frame; walk each with a
    // fresh env in source order for stable output.
    let mut procs: Vec<_> = cu.ir_module.procedures.values().collect();
    procs.sort_by_key(|p| p.span.start());
    for proc in procs {
        let mut env: HashMap<String, EnvVal> = HashMap::new();
        walk_script(&proc.body, &mut env, &mut reach);
    }
    reach
}

/// Analyse one iRule body and serialise its attach reach as JSON — the entry
/// point the `PyO3` facade exposes to the Python report layer.
#[must_use]
pub fn irule_attach_patterns(source: &str) -> Value {
    attach_reach(source).to_json()
}

/// iRule names referenced via `call <rule>::<proc>` — the F5 cross-iRule proc
/// call (a `proc` defined in one iRule invoked from another). Returns the
/// `<rule>` parts (the iRule that defines the proc), deduped in source order;
/// the caller resolves each name to an actual iRule object. A same-rule /
/// namespaced call (`call ::helper`, `call ns::helper` where `ns` is not an
/// iRule) simply won't resolve and is ignored downstream.
#[must_use]
pub fn proc_call_refs(source: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    if source.is_empty() {
        return out;
    }
    let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    let cu = CompilationUnit::build_for_profile(source, registry, false, DialectProfile::irules());
    let mut procs: Vec<_> = cu.ir_module.procedures.values().collect();
    procs.sort_by_key(|p| p.span.start());
    for proc in procs {
        collect_proc_calls(&proc.body, &mut out);
    }
    out
}

fn collect_proc_calls(script: &Script, out: &mut Vec<String>) {
    for st in &script.statements {
        match st {
            Statement::Call { command, args, .. } => {
                // `call <rule>::<proc>` — the namespace (before `::`) is the
                // iRule that defines the proc; link the caller to it.
                if command.trim_start_matches("::") == "call"
                    && let Some((rule, _proc)) = args.first().and_then(|a| a.rsplit_once("::"))
                    && !rule.is_empty()
                    && !out.iter().any(|r| r == rule)
                {
                    out.push(rule.to_string());
                }
            }
            Statement::If {
                clauses, else_body, ..
            } => {
                for c in clauses {
                    collect_proc_calls(&c.body, out);
                }
                if let Some(b) = else_body {
                    collect_proc_calls(b, out);
                }
            }
            Statement::For {
                init, next, body, ..
            } => {
                collect_proc_calls(init, out);
                collect_proc_calls(body, out);
                collect_proc_calls(next, out);
            }
            Statement::While { body, .. }
            | Statement::Foreach { body, .. }
            | Statement::Catch { body, .. }
            | Statement::Block { body, .. }
            | Statement::UpFrame { body, .. } => collect_proc_calls(body, out),
            Statement::Try {
                body,
                handlers,
                finally_body,
                ..
            } => {
                collect_proc_calls(body, out);
                for h in handlers {
                    collect_proc_calls(&h.body, out);
                }
                if let Some(f) = finally_body {
                    collect_proc_calls(f, out);
                }
            }
            Statement::Switch {
                arms, default_body, ..
            } => {
                for a in arms {
                    if let Some(b) = &a.body {
                        collect_proc_calls(b, out);
                    }
                }
                if let Some(b) = default_body {
                    collect_proc_calls(b, out);
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn globs(v: &[AttachPattern]) -> Vec<String> {
        v.iter().map(AttachPattern::glob).collect()
    }

    #[test]
    fn prefix_from_bracket_substitution() {
        let r = attach_reach(r#"when HTTP_REQUEST { pool "web_[HTTP::host]" }"#);
        assert_eq!(globs(&r.pools), vec!["web_*"]);
        let p = &r.pools[0];
        assert!(p.matches("web_pool"));
        assert!(p.matches("web_"));
        assert!(!p.matches("db_pool"));
        assert!(!p.unconstrained);
    }

    #[test]
    fn suffix_and_contains() {
        let r = attach_reach(r#"when HTTP_REQUEST { pool "[getfield [HTTP::uri] / 1]_pool" }"#);
        assert_eq!(globs(&r.pools), vec!["*_pool"]);
        assert!(r.pools[0].matches("anything_pool"));
        assert!(!r.pools[0].matches("pool_anything"));
    }

    #[test]
    fn prefix_and_suffix() {
        let r = attach_reach(r#"when HTTP_REQUEST { pool "app_[HTTP::path]_v2" }"#);
        assert_eq!(globs(&r.pools), vec!["app_*_v2"]);
        let p = &r.pools[0];
        assert!(p.matches("app_x_v2"));
        assert!(p.matches("app__v2"));
        assert!(!p.matches("app_x_v3"));
        assert!(!p.matches("other_x_v2"));
    }

    #[test]
    fn pure_variable_is_unconstrained() {
        let r = attach_reach("when HTTP_REQUEST { pool $backend }");
        assert_eq!(r.pools.len(), 1);
        assert!(r.pools[0].unconstrained);
        assert!(r.pools[0].matches("literally_anything"));
    }

    #[test]
    fn constant_propagation_through_set() {
        let r = attach_reach(r#"when HTTP_REQUEST { set p "web_[HTTP::host]"; pool $p }"#);
        assert_eq!(globs(&r.pools), vec!["web_*"]);
        assert!(r.pools[0].matches("web_eu"));
        assert!(!r.pools[0].matches("api_eu"));
    }

    #[test]
    fn propagation_of_literal_prefix_variable() {
        let r = attach_reach(
            r#"when HTTP_REQUEST { set prefix "svc_"; pool "${prefix}[HTTP::host]" }"#,
        );
        assert_eq!(globs(&r.pools), vec!["svc_*"]);
    }

    #[test]
    fn full_constant_resolves_to_exact() {
        let r = attach_reach(r#"when HTTP_REQUEST { set p "web_pool"; pool $p }"#);
        assert_eq!(r.pools.len(), 1);
        assert!(r.pools[0].exact);
        assert_eq!(r.pools[0].glob(), "web_pool");
        assert!(r.pools[0].matches("web_pool"));
        assert!(!r.pools[0].matches("web_pool2"));
    }

    #[test]
    fn conflicting_assignment_is_unconstrained() {
        let r = attach_reach(r#"when HTTP_REQUEST { set p "web_[X]"; set p "api_[Y]"; pool $p }"#);
        assert_eq!(r.pools.len(), 1);
        assert!(r.pools[0].unconstrained);
    }

    #[test]
    fn literal_pool_is_not_a_dynamic_risk() {
        let r = attach_reach("when HTTP_REQUEST { pool /Common/static_pool }");
        assert!(r.pools.is_empty());
    }

    #[test]
    fn attach_inside_nested_if() {
        let r = attach_reach(
            r#"when HTTP_REQUEST { if { [HTTP::host] eq "x" } { pool "web_[HTTP::uri]" } }"#,
        );
        assert_eq!(globs(&r.pools), vec!["web_*"]);
    }

    #[test]
    fn node_and_snatpool_buckets() {
        let r = attach_reach(
            r#"when CLIENT_ACCEPTED { snatpool "snat_[whichPart]"; node 10.0.[expr {$x}].5 }"#,
        );
        assert_eq!(globs(&r.snatpools), vec!["snat_*"]);
        assert_eq!(globs(&r.nodes), vec!["10.0.*.5"]);
        assert!(r.nodes[0].matches("10.0.7.5"));
        assert!(!r.nodes[0].matches("10.1.7.5"));
    }

    #[test]
    fn proc_call_refs_cross_rule() {
        let refs = proc_call_refs(
            r"when HTTP_REQUEST { call MyLib::helper $x; if {1} { call /Common/Other::doit 1 } }",
        );
        assert_eq!(refs, vec!["MyLib", "/Common/Other"]);
    }

    #[test]
    fn proc_call_refs_ignores_local_and_dedups() {
        let refs = proc_call_refs(r"when HTTP_REQUEST { call ::plain; call Lib::a; call Lib::b }");
        assert_eq!(refs, vec!["Lib"]);
    }

    #[test]
    fn json_roundtrip_shape() {
        let v = irule_attach_patterns(r#"when HTTP_REQUEST { pool "web_[HTTP::host]" }"#);
        let p = &v["pools"][0];
        assert_eq!(p["prefix"], "web_");
        assert_eq!(p["glob"], "web_*");
        assert_eq!(p["unconstrained"], false);
        assert_eq!(p["exact"], false);
    }
}
