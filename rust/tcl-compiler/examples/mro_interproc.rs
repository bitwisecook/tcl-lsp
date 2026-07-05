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

//! `mro_interproc` — measure the *ceiling* of interprocedural object→class
//! flow, the lever the class-lattice experiment identified but never
//! quantified.
//!
//! The FULL resolver abstains on 81 % of `$obj method` sites, 60 % of them
//! `unknown` — receivers that arrive as a proc/method parameter, a class
//! instance variable, or a global. This harness answers: **how many of
//! those would a real interprocedural pass recover?**
//!
//! 1. **Receiver taxonomy** of every `unknown` abstention: parameter /
//!    class-instance-variable / global-or-other.
//! 2. **Instance-variable ceiling**: a `variable v` bound by `set v [C new]`
//!    in the class's own constructor/methods is recoverable *intra-class*.
//! 3. **Parameter ceiling** via caller→callee class propagation: seed each
//!    proc parameter with the classes of the constructor-bound arguments
//!    passed at its call sites, then iterate to a fixpoint. Report how many
//!    parameter-receiver sites resolve at 1 hop, 2 hops, and fixpoint, and
//!    whether they land on a single class or a set.
//!
//! This is an *upper bound* (best-effort positional arg→param mapping, no
//! path sensitivity) — deliberately, since the question is "what is the
//! most interprocedural flow could buy?".

#![allow(
    clippy::cast_precision_loss,
    clippy::doc_markdown,
    clippy::too_many_lines,
    clippy::similar_names,
    clippy::needless_lifetimes,
    clippy::items_after_statements
)]

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use tcl_compiler::analyser::class_lattice::{
    AblationConfig, ClassValue, DispatchVerdict, NsContext, TopReason, analyse_dispatch,
    class_values,
};
use tcl_compiler::analyser::state::Analyser;
use tcl_compiler::analyser::types::{AnalysisResult, ClassDef};
use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_registry::CommandRegistry;

fn collect_tcl_files(root: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if !matches!(name, ".git" | "target" | "node_modules" | ".venv") {
                collect_tcl_files(&path, out);
            }
        } else if path
            .extension()
            .and_then(|s| s.to_str())
            .is_some_and(|e| e == "tcl" || e == "test")
        {
            out.push(path);
        }
    }
}

/// A proc or method body: its scope span, qualified name, and ordered
/// parameter names.
struct Scope {
    start: u32,
    end: u32,
    qname: String,
    params: Vec<String>,
}

/// A `foreach`/`lmap` loop: its loop-var names, the source text of the
/// collection it iterates, and its body span.
struct ForeachLoop {
    vars: Vec<String>,
    list_arg: String,
    start: u32,
    end: u32,
}

struct FileUnit {
    cu: CompilationUnit,
    ns: NsContext,
    result: AnalysisResult,
    src: String,
    sites: Vec<tcl_compiler::analyser::state::VarCommandSite>,
    values: HashMap<String, ClassValue>,
    scopes: Vec<Scope>,
    foreaches: Vec<ForeachLoop>,
}

/// Robust source-text classification of how a receiver `var` is bound
/// inside its enclosing scope `[lo, hi)`.  Sidesteps IR-lowering quirks
/// (runtime-fallback `foreach` bodies get hoisted to synthetic units, so
/// span containment fails): scans the scope source for the loop/alias
/// construct that binds `var`, and returns the construct kind plus the
/// collection expression it iterates (for element typing).
fn loopvar_binding<'a>(src: &'a str, lo: u32, hi: u32, var: &str) -> Option<(&'static str, String)> {
    let lo = (lo as usize).min(src.len());
    let hi = (hi as usize).min(src.len());
    if lo >= hi {
        return None;
    }
    let region = &src[lo..hi];
    // Match `foreach … var … LIST`, `lmap … var … LIST`,
    // `dict for {k var} DICT`, `dict map {k var} DICT`, `upvar … var`.
    for (kw, kind) in [("foreach", "foreach"), ("lmap", "lmap"), ("dict for", "dict-for"),
                       ("dict map", "dict-map"), ("upvar", "upvar")] {
        let mut from = 0usize;
        while let Some(p) = region[from..].find(kw) {
            let start = from + p;
            // token boundary before kw
            let ok_before = start == 0 || !region.as_bytes()[start - 1].is_ascii_alphanumeric();
            let after = start + kw.len();
            if ok_before {
                // Look at the words up to end-of-line: does the var list contain `var`?
                let eol = region[after..].find('\n').map_or(region.len(), |i| after + i);
                let head = &region[after..eol];
                // var must appear as a bare word (loop var) — not `$var`.
                if head.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == ':'))
                    .any(|w| w == var)
                {
                    // Collection = the expression after the (possibly braced)
                    // var-list, up to the body brace. `foreach param
                    // [dict values $Params] {…}` → `[dict values $Params]`.
                    let h = head.trim_start();
                    let rest = if let Some(after) = h.strip_prefix('{') {
                        // braced var-list: `{k v} LIST BODY`
                        after.split_once('}').map_or("", |(_, t)| t)
                    } else {
                        // bare var-list: skip the first word (the single var)
                        h.split_once(char::is_whitespace).map_or("", |(_, t)| t)
                    };
                    // Trim a trailing `{…}` body.
                    let coll = rest.trim().split_once('{').map_or(rest.trim(), |(c, _)| c.trim());
                    return Some((kind, coll.to_string()));
                }
            }
            from = after;
        }
    }
    None
}

/// Child scripts of a control-flow statement, for recursive walking.
fn child_scripts(stmt: &tcl_compiler::ir::Statement) -> Vec<&tcl_compiler::ir::Script> {
    use tcl_compiler::ir::Statement as S;
    let mut out = Vec::new();
    match stmt {
        S::If { clauses, else_body, .. } => {
            for c in clauses {
                out.push(&c.body);
            }
            if let Some(e) = else_body {
                out.push(e);
            }
        }
        S::For { init, next, body, .. } => {
            out.push(init);
            out.push(next);
            out.push(body);
        }
        S::While { body, .. } | S::Foreach { body, .. } | S::Catch { body, .. } => out.push(body),
        S::Try { body, handlers, finally_body, .. } => {
            out.push(body);
            for h in handlers {
                out.push(&h.body);
            }
            if let Some(f) = finally_body {
                out.push(f);
            }
        }
        S::Switch { arms, default_body, .. } => {
            for a in arms {
                if let Some(b) = &a.body {
                    out.push(b);
                }
            }
            if let Some(d) = default_body {
                out.push(d);
            }
        }
        _ => {}
    }
    out
}

/// Recursively collect every `foreach`/`lmap` loop in a script.
fn collect_foreach(script: &tcl_compiler::ir::Script, out: &mut Vec<ForeachLoop>) {
    use tcl_compiler::ir::Statement as S;
    for stmt in &script.statements {
        if let S::Foreach { iterators, body_span, .. } = stmt {
            for it in iterators {
                out.push(ForeachLoop {
                    vars: it.vars.clone(),
                    list_arg: it.list_arg.clone(),
                    start: body_span.start(),
                    end: body_span.end(),
                });
            }
        }
        for child in child_scripts(stmt) {
            collect_foreach(child, out);
        }
    }
}

fn analyse_file(path: &Path, reg: &CommandRegistry, merged: &HashMap<String, ClassDef>) -> Option<FileUnit> {
    let src = std::fs::read_to_string(path).ok()?;
    if src.len() > 2_000_000 {
        return None;
    }
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut a = Analyser::new();
        let result = a.analyse(&src, "tcl8.6");
        let cu =
            CompilationUnit::build_for(&src, reg, false).with_interprocedural(reg, Some("tcl8.6"));
        let ns = NsContext::from_result(&result);
        // Resolve against the merged (cross-file) index — matches the FULL
        // config that produced the 81 % ⊤-rate.
        let values = class_values(&cu, merged, &ns, &AblationConfig::full());
        // Build proc + method scopes.
        let mut scopes = Vec::new();
        for (qname, pd) in &result.all_procs {
            scopes.push(Scope {
                start: pd.body_span.start(),
                end: pd.body_span.end(),
                qname: qname.clone(),
                params: pd.params.iter().map(|p| p.name.clone()).collect(),
            });
        }
        for cd in result.all_classes.values() {
            for (mname, md) in cd.methods.iter().chain(cd.class_methods.iter()) {
                scopes.push(Scope {
                    start: md.body_span.start(),
                    end: md.body_span.end(),
                    qname: format!("{}::{mname}", cd.qualified_name),
                    params: md.params.iter().map(|p| p.name.clone()).collect(),
                });
            }
        }
        let mut foreaches = Vec::new();
        collect_foreach(&cu.ir_module.top_level, &mut foreaches);
        for p in cu.ir_module.procedures.values() {
            collect_foreach(&p.body, &mut foreaches);
        }
        for p in cu.ir_module.body_units.values() {
            collect_foreach(&p.body, &mut foreaches);
        }
        for m in cu.ir_module.methods.values() {
            collect_foreach(&m.body, &mut foreaches);
        }
        // Runtime-fallback `foreach`/`lmap` (dynamic list arg like
        // `[dict values $Params]`) never becomes a structured `Foreach`
        // node — it lands in the CFG as a `foreach`/`lmap` Call/Barrier.
        // Recover the loop vars + list + a body range (the command span
        // contains the body) so object-container iterations are counted.
        use tcl_compiler::ir::Statement as St;
        let cfg_units = std::iter::once(&cu.top_level)
            .chain(cu.procedures.values())
            .chain(cu.methods.values());
        for fu in cfg_units {
            for block in fu.cfg.blocks.values() {
                for stmt in &block.statements {
                    let (St::Call { command, args, span, .. }
                    | St::Barrier { command, args, span, .. }) = stmt
                    else {
                        continue;
                    };
                    if (command == "foreach" || command == "lmap") && args.len() >= 3 {
                        // `foreach {v…} LIST BODY` — args = [varlist, list, body].
                        let vars: Vec<String> = args[0]
                            .split_whitespace()
                            .map(std::string::ToString::to_string)
                            .collect();
                        foreaches.push(ForeachLoop {
                            vars,
                            list_arg: args[1].clone(),
                            start: span.start(),
                            end: span.end(),
                        });
                    }
                }
            }
        }
        FileUnit { cu, ns, result, src, sites: a.var_command_sites.clone(), values, scopes, foreaches }
    }))
    .ok()
}

/// Innermost scope containing `offset`.
fn enclosing<'a>(scopes: &'a [Scope], offset: u32) -> Option<&'a Scope> {
    scopes
        .iter()
        .filter(|s| s.start <= offset && offset <= s.end)
        .min_by_key(|s| s.end.saturating_sub(s.start))
}

fn strip_dollar(text: &str) -> Option<String> {
    let rest = text.trim().strip_prefix('$')?;
    let inner = rest
        .strip_prefix('{')
        .and_then(|r| r.strip_suffix('}'))
        .unwrap_or(rest);
    (!inner.is_empty()
        && inner.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b':'))
    .then(|| inner.to_string())
}

/// Extract the *object* container variable from a `foreach` list argument.
/// Handles `$X`, `${X}`, `[dict values $X]`, `[dict get …$X…]`.  Returns
/// `None` for a `[dict keys …]` (yields names, not objects) or anything
/// whose element type is not a container of objects.
fn container_var(list_arg: &str) -> Option<String> {
    let a = list_arg.trim();
    if let Some(v) = strip_dollar(a) {
        return Some(v);
    }
    let inner = a.strip_prefix('[').and_then(|s| s.strip_suffix(']'))?.trim();
    let rest = inner.strip_prefix("dict ")?;
    // `dict values $X` → objects; `dict keys` → names (reject).
    let rest = rest.trim_start();
    if let Some(after) = rest.strip_prefix("values ") {
        return strip_dollar(after.trim());
    }
    if let Some(after) = rest.strip_prefix("get ") {
        // `dict get $X k…` — first `$word` is the container.
        return after.split_whitespace().find_map(strip_dollar);
    }
    None
}

fn classes_of(v: &ClassValue) -> Option<HashSet<String>> {
    match v {
        ClassValue::Concrete(c) => Some([c.clone()].into_iter().collect()),
        ClassValue::Set(s) => Some(s.iter().cloned().collect()),
        _ => None,
    }
}

fn main() {
    let roots: Vec<PathBuf> = std::env::args().skip(1).map(PathBuf::from).collect();
    if roots.is_empty() {
        eprintln!("usage: mro_interproc <corpus dirs...>");
        std::process::exit(1);
    }
    let mut files = Vec::new();
    for r in &roots {
        collect_tcl_files(r, &mut files);
    }
    files.sort();
    let reg = CommandRegistry::build_default();

    // Pass A: analyse once (per-file index) to build the merged class index.
    let mut merged: HashMap<String, ClassDef> = HashMap::new();
    let mut srcs: Vec<PathBuf> = Vec::new();
    for p in &files {
        if let Some(src) = std::fs::read_to_string(p).ok().filter(|s| s.len() <= 2_000_000) {
            let mut a = Analyser::new();
            let r = a.analyse(&src, "tcl8.6");
            for (k, v) in r.all_classes {
                merged.entry(k).or_insert(v);
            }
            srcs.push(p.clone());
        }
    }
    // Pass B: full analysis against the merged index.
    let mut units: Vec<FileUnit> = Vec::new();
    for p in &srcs {
        if let Some(u) = analyse_file(p, &reg, &merged) {
            units.push(u);
        }
    }

    // Build the corpus proc registry: bare name → qualified names, for
    // resolving call heads.
    let mut proc_params: HashMap<String, Vec<String>> = HashMap::new();
    for u in &units {
        for (qn, pd) in &u.result.all_procs {
            proc_params.insert(qn.clone(), pd.params.iter().map(|p| p.name.clone()).collect());
        }
    }
    let mut bare_to_q: HashMap<String, Vec<String>> = HashMap::new();
    for qn in proc_params.keys() {
        let bare = qn.rsplit("::").next().unwrap_or(qn).to_string();
        bare_to_q.entry(bare).or_default().push(qn.clone());
    }
    let resolve_call = |head: &str| -> Option<String> {
        if proc_params.contains_key(head) {
            return Some(head.to_string());
        }
        let q = format!("::{head}");
        if proc_params.contains_key(&q) {
            return Some(q);
        }
        let bare = head.rsplit("::").next().unwrap_or(head);
        match bare_to_q.get(bare) {
            Some(v) if v.len() == 1 => Some(v[0].clone()),
            _ => None,
        }
    };

    // --- Collection element-typing map (corpus-wide) ---
    // coll_name → set of element classes, from `lappend coll <arg>` and
    // `set coll [list <args>]` where each arg is `[X new]` or a var that
    // resolves to a class. This types `foreach x $coll { $x method }`.
    use tcl_compiler::ir::Statement;
    let mut coll_elem: HashMap<String, HashSet<String>> = HashMap::new();
    let elem_class_of = |arg: &str, values: &HashMap<String, ClassValue>| -> HashSet<String> {
        let a = arg.trim();
        // `[Class new …]` literal element
        if let Some(inner) = a.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            let mut it = inner.split_whitespace();
            if let (Some(head), Some(sub)) = (it.next(), it.next())
                && (sub == "new" || sub == "create")
            {
                let (q, ok) = {
                    // resolve via any file's ns is overkill; try global/bare.
                    let bare = head.trim_start_matches("::");
                    let cand = format!("::{bare}");
                    if merged.contains_key(&cand) {
                        (cand, true)
                    } else if merged.contains_key(head) {
                        (head.to_string(), true)
                    } else {
                        (cand, false)
                    }
                };
                if ok {
                    return [q].into_iter().collect();
                }
            }
        }
        // `$v` element that already resolves to a class
        if let Some(v) = strip_dollar(a)
            && let Some(c) = values.get(&v).and_then(classes_of)
        {
            return c;
        }
        HashSet::new()
    };
    for u in &units {
        let cu_units = std::iter::once(&u.cu.top_level)
            .chain(u.cu.procedures.values())
            .chain(u.cu.methods.values());
        for fu in cu_units {
            for block in fu.cfg.blocks.values() {
                for stmt in &block.statements {
                    let (Statement::Call { command, args, .. }
                    | Statement::Barrier { command, args, .. }) = stmt
                    else {
                        continue;
                    };
                    let canon = stmt.canonical_command_or_source();
                    // `lappend COLL <elem>…`  → COLL holds those element classes.
                    if command == "lappend" && args.len() >= 2 {
                        for a in &args[1..] {
                            let cls = elem_class_of(a, &u.values);
                            if !cls.is_empty() {
                                coll_elem.entry(args[0].clone()).or_default().extend(cls);
                            }
                        }
                    }
                    // `dict set CONTAINER key <obj>` / `dict append` → the
                    // container's *values* are those object classes. This is
                    // the dominant TclOO pattern (child objects kept in an
                    // instance-variable dict keyed by name).
                    if (command == "dict" || canon == "::dict")
                        && matches!(args.first().map(String::as_str), Some("set" | "append" | "lappend"))
                        && args.len() >= 4
                    {
                        let container = args[1].clone();
                        let cls = elem_class_of(args.last().unwrap(), &u.values);
                        if !cls.is_empty() {
                            coll_elem.entry(container).or_default().extend(cls);
                        }
                    }
                }
            }
        }
    }

    // --- Receiver taxonomy over `unknown` abstentions ---
    let mut n_unknown = 0usize;
    let mut n_param = 0usize;
    let mut n_instvar = 0usize;
    let mut n_iter = 0usize;
    let mut iter_typeable = 0usize;
    let mut n_upvar = 0usize;
    let mut n_global = 0usize;
    // Instance-variable ceiling: (class, var) → classes bound in the class body.
    // Parameter-receiver sites, for the ceiling: (proc_qname, param) list.
    let mut param_sites: Vec<(String, String)> = Vec::new();
    let mut instvar_recoverable = 0usize;

    // Pre-compute instance-var bindings: for each class, scan its methods'
    // bodies (in the CU) for `set v [C new]` where v is a declared variable.
    // Approximate via the file's `values` map keyed by the var name, scoped
    // to the class body.
    for u in &units {
        // Resolve every site in the file once (not per-site).
        let (reports, _stats) =
            analyse_dispatch(&u.cu, &merged, &u.sites, &u.ns, &AblationConfig::full());
        for rep in &reports {
            if !matches!(rep.verdict, DispatchVerdict::Abstain(TopReason::Unknown)) {
                continue;
            }
            n_unknown += 1;
            let var = &rep.var;
            let encl = enclosing(&u.scopes, rep.offset);
            let is_param = encl.is_some_and(|s| s.params.iter().any(|p| p == var));
            // Instance variable: declared `variable v` in the enclosing class.
            let in_class_var = u.result.all_classes.values().any(|cd| {
                cd.body_span.start() <= rep.offset
                    && rep.offset <= cd.body_span.end()
                    && cd.variables.iter().any(|v| v == var)
            });
            // Source-scan: is `var` bound as a loop / dict-iteration / upvar
            // variable in its enclosing scope? (Robust to IR lowering.)
            let (lo, hi) = encl.map_or((0, rep.offset), |s| (s.start, s.end));
            let binding = loopvar_binding(&u.src, lo, hi, var);

            if is_param {
                n_param += 1;
                if let Some(s) = encl {
                    param_sites.push((s.qname.clone(), var.clone()));
                }
            } else if let Some((kind, coll)) = binding {
                if kind == "upvar" {
                    n_upvar += 1;
                } else {
                    n_iter += 1;
                    // Typeable if the iterated *object* container has a known
                    // element class. `$X` / `${X}` / `[dict values $X]` /
                    // `[dict get …$X…]` → extract `X`. (`[dict keys …]` /
                    // name-lists yield strings and stay untypeable.)
                    if let Some(cv) = container_var(&coll)
                        && coll_elem.get(&cv).is_some_and(|s| !s.is_empty())
                    {
                        iter_typeable += 1;
                    }
                }
            } else if in_class_var {
                n_instvar += 1;
                let bound = units.iter().any(|u2| {
                    u2.values.get(var).and_then(classes_of).is_some()
                });
                if bound {
                    instvar_recoverable += 1;
                }
            } else {
                n_global += 1;
                if std::env::var("SAMPLE").is_ok() && n_global <= 40 {
                    let scope = encl.map_or("TOP", |s| s.qname.as_str());
                    eprintln!("  other: ${var} {} in scope [{scope}]",
                        rep.method.as_deref().unwrap_or(""));
                }
            }
        }
    }

    // --- Parameter ceiling: caller→param class propagation to a fixpoint ---
    // param_class[(proc_qname, param)] = set of classes.
    // Seed: over every Call `p $a $b …`, resolve each `$arg` in the caller
    // file's `values`; propagate to the callee's positional param.
    // Iterate: after params gain classes, a call passing `$param` propagates
    // them onward.
    let mut param_class: HashMap<(String, String), HashSet<String>> = HashMap::new();

    // Per-file caller resolution: a var's classes = constructor `values` map
    // OR (after seeding) its own param_class if the var is a param of the
    // enclosing scope at the call site.
    let hop_counts = |param_class: &HashMap<(String, String), HashSet<String>>| -> usize {
        param_sites
            .iter()
            .filter(|(q, p)| param_class.get(&(q.clone(), p.clone())).is_some_and(|s| !s.is_empty()))
            .count()
    };
    let mut ceiling_by_hop: Vec<usize> = Vec::new();

    for hop in 0..12 {
        let mut next = param_class.clone();
        let mut changed = false;
        for u in &units {
            let cu_units = std::iter::once(("::top", &u.cu.top_level))
                .chain(u.cu.procedures.iter().map(|(q, fu)| (q.as_str(), fu)))
                .chain(u.cu.methods.iter().map(|(q, fu)| (q.as_str(), fu)));
            for (_qn, fu) in cu_units {
                for block in fu.cfg.blocks.values() {
                    for stmt in &block.statements {
                        let (Statement::Call { command, args, span, .. }
                        | Statement::Barrier { command, args, span, .. }) = stmt
                        else {
                            continue;
                        };
                        let Some(callee) = resolve_call(command) else { continue };
                        let Some(params) = proc_params.get(&callee) else { continue };
                        let caller = enclosing(&u.scopes, span.start());
                        for (i, arg) in args.iter().enumerate() {
                            let Some(pname) = params.get(i) else { break };
                            let Some(v) = strip_dollar(arg) else { continue };
                            // classes of the arg in the caller
                            let mut cls: HashSet<String> = HashSet::new();
                            if let Some(c) = u.values.get(&v).and_then(classes_of) {
                                cls.extend(c);
                            }
                            if let Some(s) = caller
                                && s.params.iter().any(|p| p == &v)
                                && let Some(c) = param_class.get(&(s.qname.clone(), v.clone()))
                            {
                                cls.extend(c.iter().cloned());
                            }
                            if cls.is_empty() {
                                continue;
                            }
                            let entry = next.entry((callee.clone(), pname.clone())).or_default();
                            let before = entry.len();
                            entry.extend(cls);
                            if entry.len() != before {
                                changed = true;
                            }
                        }
                    }
                }
            }
        }
        param_class = next;
        ceiling_by_hop.push(hop_counts(&param_class));
        if !changed {
            break;
        }
        let _ = hop;
    }

    // Single vs multi class among recovered param sites.
    let mut recovered_single = 0usize;
    let mut recovered_multi = 0usize;
    for (q, p) in &param_sites {
        if let Some(s) = param_class.get(&(q.clone(), p.clone())) {
            match s.len() {
                0 => {}
                1 => recovered_single += 1,
                _ => recovered_multi += 1,
            }
        }
    }
    let param_recovered = recovered_single + recovered_multi;

    // ---- report ----
    println!("# mro_interproc — interprocedural object→class ceiling");
    println!("files={} merged_classes={}", units.len(), merged.len());
    if std::env::var("CEDBG").is_ok() {
        eprintln!("[CEDBG] coll_elem entries: {}", coll_elem.len());
        for (k, v) in coll_elem.iter().take(15) {
            eprintln!("[CEDBG]   {k} -> {v:?}");
        }
    }
    if std::env::var("FEDBG").is_ok() {
        let total_fe: usize = units.iter().map(|u| u.foreaches.len()).sum();
        let with_obj_disp: usize = units.iter().map(|u| u.foreaches.iter().filter(|f| f.list_arg.contains("dict values") || f.list_arg.contains("dict get")).count()).sum();
        eprintln!("[FEDBG] total foreach loops collected: {total_fe} ({with_obj_disp} iterate dict values/get)");
        for u in units.iter().take(3) {
            for f in u.foreaches.iter().take(6) {
                eprintln!("[FEDBG]   vars={:?} list_arg={:?} span=[{},{}]", f.vars, f.list_arg, f.start, f.end);
            }
        }
    }
    println!();
    println!("## Receiver taxonomy of `unknown` ⊤ abstentions");
    let pct = |n: usize| if n_unknown == 0 { 0.0 } else { 100.0 * n as f64 / n_unknown as f64 };
    println!("  total unknown-⊤ sites:        {n_unknown}");
    println!("    proc/method parameter:      {n_param}  ({:.1}%)", pct(n_param));
    println!("    container iteration:        {n_iter}  ({:.1}%)   [foreach/lmap/dict-for over a collection]", pct(n_iter));
    println!("    class instance variable:    {n_instvar}  ({:.1}%)", pct(n_instvar));
    println!("    upvar alias:                {n_upvar}  ({:.1}%)", pct(n_upvar));
    println!("    global / other:             {n_global}  ({:.1}%)", pct(n_global));
    println!();
    println!("## Container element-typing ceiling (iteration over a typed object collection)");
    println!("  typeable container receivers: {iter_typeable} / {n_iter}");
    println!();
    println!("## Instance-variable ceiling (intra-class constructor binding)");
    println!("  recoverable instance-var receivers: {instvar_recoverable} / {n_instvar}");
    println!();
    println!("## Parameter ceiling (caller→param class propagation)");
    for (h, c) in ceiling_by_hop.iter().enumerate() {
        let tag = if h == 0 { "1-hop" } else if h + 1 == ceiling_by_hop.len() { "fixpoint" } else { "iter" };
        println!("  after hop {}: {c} / {} param sites resolvable  [{tag}]", h + 1, param_sites.len());
    }
    let ppct = if param_sites.is_empty() { 0.0 } else { 100.0 * param_recovered as f64 / param_sites.len() as f64 };
    println!("  fixpoint recovered: {param_recovered} / {} ({:.1}%)  [single-class {recovered_single}, multi-class {recovered_multi}]",
        param_sites.len(), ppct);
    println!();
    // Combined ceiling: how much of the unknown-⊤ bucket becomes resolvable.
    let combined = param_recovered + instvar_recoverable + iter_typeable;
    println!("## Combined ceiling over `unknown` ⊤ (all three levers)");
    println!("  {combined} / {n_unknown} unknown-⊤ sites recoverable ({:.1}%)",
        if n_unknown == 0 { 0.0 } else { 100.0 * combined as f64 / n_unknown as f64 });
    println!("    container element-typing: {iter_typeable}");
    println!("    interproc param flow:     {param_recovered}  (single-class {recovered_single})");
    println!("    instance-var binding:     {instvar_recoverable}");
}
