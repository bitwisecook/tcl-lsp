//! Variable-as-command and `TclOO` method-dispatch checks (the
//! cross-function post-pass).
//!
//! Resolves `$var`-as-command call sites collected during the walk: a
//! non-literal command word that cannot be proved safe (W307), and an
//! unknown method invoked on an object whose class is known (W308), using
//! MRO-aware method resolution over the class hierarchy with the usual
//! suppression paths (inherited `unknown` handler, external superclass,
//! `oo::objdefine` per-instance methods). Tracks the object types produced
//! by constructors and factory procedures so a later `$obj badMethod`
//! resolves against the right class, and resolves partially-interpolated
//! command heads that fold to a finite known-command set (W123 suppression).

use std::collections::{HashMap, HashSet};
use tcl_core_types::DiagCode;

use rustc_hash::{FxHashMap, FxHashSet};

use crate::analyser::state::Analyser;
use crate::analyser::types::Severity;

impl Analyser {
    /// True when `my <method>` / `self <method>` dispatched at `site_offset`
    /// resolves to a method in the enclosing class whose body is a simple
    /// `return <literal>` — i.e. it returns a plain string, not an object
    /// handle.  The enclosing class is the one whose `body_span` contains the
    /// dispatch offset; the method is looked up in its `methods` /
    /// `class_methods`.  A literal return is `return <word>` on a single line
    /// with no command substitution (`[`) or variable interpolation (`$`) in
    /// the returned word.
    fn oo_self_method_returns_literal(&self, site_offset: u32, method_name: &str) -> bool {
        for class_def in self.result.all_classes.values() {
            let body = class_def.body_span;
            if !(body.start() <= site_offset && site_offset <= body.end()) {
                continue;
            }
            let Some(md) = class_def
                .methods
                .get(method_name)
                .or_else(|| class_def.class_methods.get(method_name))
            else {
                // Enclosing class found but no such method — stay conservative
                // (treat as object-returning).
                return false;
            };
            let start = md.body_span.start() as usize;
            let end = (md.body_span.end() as usize).min(self.source.len());
            if start >= end {
                return false;
            }
            let mut bt = self.source[start..end].trim();
            // Strip one layer of surrounding braces.
            if let Some(inner) = bt.strip_prefix('{') {
                bt = inner.trim_end();
                bt = bt.strip_suffix('}').unwrap_or(bt).trim();
            }
            // Simple `return <literal>` — single statement, no substitutions.
            if bt.contains('\n') || bt.contains(';') {
                return false;
            }
            let Some(ret_arg) = bt.strip_prefix("return ") else {
                return false;
            };
            let ret_arg = ret_arg.trim();
            return !ret_arg.is_empty() && !ret_arg.contains('[') && !ret_arg.contains('$');
        }
        false
    }

    /// Harvest `set x [Cls new]` / `set x [Cls create name]` where `Cls` is a
    /// known `TclOO` class: `x` then holds an Object of class `Cls`, so a later
    /// `$x method` dispatch resolves through the W308 method check instead of
    /// firing W307.  The type lattice doesn't model the constructor return
    /// type for a var assignment yet (the cmd-site path recognises the
    /// bare-class `new`/`create` pattern directly), so mirror that recognition
    /// here for the var-assignment shape.
    fn harvest_constructor_object_types(
        &self,
        cu: &crate::compilation_unit::CompilationUnit,
        out: &mut HashMap<String, HashSet<String>>,
    ) {
        use crate::ir::Statement;
        let units = std::iter::once(&cu.top_level).chain(cu.procedures.values());
        for fu in units {
            for block in fu.cfg.blocks.values() {
                for stmt in &block.statements {
                    let Statement::AssignValue { name, value, .. } = stmt else {
                        continue;
                    };
                    let Some((head, args)) =
                        crate::value_shapes::parse_command_substitution(value.trim())
                    else {
                        continue;
                    };
                    if !args.first().is_some_and(|s| s == "new" || s == "create") {
                        continue;
                    }
                    let class_qn = self.canonicalise_class_name(&head);
                    if self.result.all_classes.contains_key(&class_qn)
                        || self.result.all_classes.contains_key(&head)
                    {
                        out.entry(name.clone()).or_default().insert(class_qn);
                    }
                }
            }
        }
    }

    /// Per-proc `(body_start, body_end, factory_local_vars)` ranges — the
    /// variables that hold an *object factory* result, so a `$var method`
    /// dispatch on them suppresses W307 (an object handle is a designed,
    /// non-literal command target, not a static error).
    ///
    /// A factory local is a `set X [head …]`
    /// where `head` is object-returning: a known `TclOO` class command, a
    /// namespaced factory (the documented tcllib `::ns::cmd` convention, minus
    /// known user procs and registry commands with a non-OBJECT return type),
    /// or another proc proven object-returning by the fixpoint.  This tracks
    /// *no* class identity — it only suppresses W307 (it never enables W308).
    #[allow(clippy::too_many_lines)]
    // A single fixpoint algorithm (classify heads → collect factory locals →
    // seed → propagate → extend → materialise ranges) whose phases share local
    // state; splitting it would only scatter that state behind extra args.
    fn compute_factory_object_ranges(
        &self,
        cu: &crate::compilation_unit::CompilationUnit,
        registry: &tcl_registry::CommandRegistry,
    ) -> Vec<(u32, u32, HashSet<String>)> {
        use crate::ir::Statement;

        let class_qnames: HashSet<&String> = self.result.all_classes.keys().collect();
        let class_tails: HashSet<&str> = class_qnames
            .iter()
            .filter_map(|qn| qn.rsplit_once("::").map(|(_, t)| t))
            .filter(|t| !t.is_empty())
            .collect();
        let is_user_proc = |head: &str| {
            self.result.all_procs.contains_key(head)
                || self.result.all_procs.contains_key(&format!("::{head}"))
        };
        // A command head whose value-returning invocation yields an object
        // handle (excluding user procs, which the fixpoint classifies).
        let is_object_returning_head = |head: &str| -> bool {
            if class_tails.contains(head) || class_qnames.contains(&format!("::{head}")) {
                return true;
            }
            if head.contains("::") {
                let qualified = if head.starts_with("::") {
                    head.to_string()
                } else {
                    format!("::{head}")
                };
                // A known user proc defers to the fixpoint (returns false here).
                if self.result.all_procs.contains_key(&qualified) {
                    return false;
                }
                // A registered command with an explicit non-OBJECT return type
                // (http::*, clock::*, …) is not a factory.
                if let Some(spec) = registry.get(head).or_else(|| registry.get(&qualified))
                    && let Some(rt) = spec.return_type
                    && rt != tcl_registry::TclType::Object
                {
                    return false;
                }
                // Unregistered `::pkg::cmd` — treat as a factory (the tcllib
                // convention; documented heuristic).
                return true;
            }
            false
        };

        // All analysable units: top level + procedures (not methods, which are
        // `in_method`-suppressed for W307 anyway).
        let units: Vec<(&str, &crate::compilation_unit::FunctionUnit)> =
            std::iter::once(("::top", &cu.top_level))
                .chain(cu.procedures.iter().map(|(q, fu)| (q.as_str(), fu)))
                .collect();

        // Per-proc: factory-local vars (non-user-proc factory heads), the last
        // returned var, and the `{var -> rhs command head}` assignment map.
        let mut factory_locals: FxHashMap<String, HashSet<String>> = FxHashMap::default();
        let mut return_var: FxHashMap<String, Option<String>> = FxHashMap::default();
        let mut assigns: FxHashMap<String, FxHashMap<String, String>> = FxHashMap::default();
        let mut object_returning: FxHashSet<String> = FxHashSet::default();
        for (qname, fu) in &units {
            let mut names = HashSet::new();
            let mut amap = FxHashMap::default();
            for block in fu.cfg.blocks.values() {
                for stmt in &block.statements {
                    let Statement::AssignValue { name, value, .. } = stmt else {
                        continue;
                    };
                    let Some((head, _)) =
                        crate::value_shapes::parse_command_substitution(value.trim())
                    else {
                        continue;
                    };
                    amap.insert(name.clone(), head.clone());
                    if is_object_returning_head(&head) && !is_user_proc(&head) {
                        names.insert(name.clone());
                    }
                }
            }
            factory_locals.insert((*qname).to_string(), names);
            assigns.insert((*qname).to_string(), amap);
            return_var.insert((*qname).to_string(), last_return_var_of(&fu.cfg));
            // Seed: a proc whose every return value is a namespaced
            // object-returning cmd-sub is itself object-returning (G4: ALL
            // returns must qualify, so a string-returning branch disqualifies).
            let rvs = return_values_of(&fu.cfg);
            if !rvs.is_empty()
                && rvs.iter().all(|rv| {
                    crate::value_shapes::parse_command_substitution(rv.trim())
                        .is_some_and(|(head, _)| is_object_returning_head(&head))
                })
            {
                object_returning.insert((*qname).to_string());
            }
        }
        // A proc returning one of its own factory locals is object-returning.
        for (qname, rv) in &return_var {
            if let Some(rv) = rv
                && factory_locals.get(qname).is_some_and(|s| s.contains(rv))
            {
                object_returning.insert(qname.clone());
            }
        }

        // Bare-name → qualified-name index for resolving relative call heads.
        let mut bare_to_qnames: FxHashMap<&str, Vec<&str>> = FxHashMap::default();
        for qname in cu.ir_module.procedures.keys() {
            let bare = qname.rsplit_once("::").map_or(qname.as_str(), |(_, t)| t);
            bare_to_qnames.entry(bare).or_default().push(qname.as_str());
        }
        let resolve_candidates = |head: &str| -> Vec<String> {
            let mut c = vec![head.to_string(), format!("::{head}")];
            if let Some(qs) = bare_to_qnames.get(head) {
                c.extend(qs.iter().map(|s| (*s).to_string()));
            }
            c
        };

        // Fixpoint: a proc whose returned var is assigned `[other]` where
        // `other` is a proven object-returning user proc is itself one.
        let mut changed = true;
        while changed {
            changed = false;
            for (qname, rv) in &return_var {
                let Some(rv) = rv else { continue };
                if object_returning.contains(qname) {
                    continue;
                }
                let Some(rhs) = assigns.get(qname).and_then(|m| m.get(rv)) else {
                    continue;
                };
                if resolve_candidates(rhs)
                    .iter()
                    .any(|c| object_returning.contains(c))
                {
                    object_returning.insert(qname.clone());
                    changed = true;
                }
            }
        }
        // Extend factory locals: `set X [user_proc]` where the proc is now
        // proven object-returning makes `X` a factory local too.
        for (qname, amap) in &assigns {
            let mut add = FxHashSet::default();
            for (var, head) in amap {
                if factory_locals.get(qname).is_some_and(|s| s.contains(var)) {
                    continue;
                }
                if resolve_candidates(head)
                    .iter()
                    .any(|c| object_returning.contains(c))
                {
                    add.insert(var.clone());
                }
            }
            factory_locals.entry(qname.clone()).or_default().extend(add);
        }

        // Materialise ranges (top level spans the whole source).
        let mut ranges = Vec::new();
        for (qname, names) in factory_locals {
            if names.is_empty() {
                continue;
            }
            if qname == "::top" {
                ranges.push((0, u32::MAX, names));
            } else if let Some(p) = cu.ir_module.procedures.get(&qname) {
                ranges.push((p.span.start(), p.span.end(), names));
            }
        }
        ranges
    }

    /// W307 — non-literal command name (variable / command-sub
    /// used as command head) and W308 (unknown method on object).
    ///
    /// Walks every recorded
    /// site in [`Self::var_command_sites`] / [`Self::cmd_command_sites`] and
    /// emits W307 unless the command head is statically resolvable to a finite
    /// set of known command names, an OBJECT of a known class (→ W308 method
    /// check), or a positive OO-dispatch signal (`$self`, `my`/`self`
    /// self-dispatch, namespaced ensemble, callback-array, dict-with unpack).
    #[allow(clippy::too_many_lines)]
    // Long-running analyser pass with many sequential phases over the CompilationUnit; splitting requires threading shared local state.
    pub(super) fn emit_var_command_diagnostics(
        &mut self,
        cu: &crate::compilation_unit::CompilationUnit,
        registry: &tcl_registry::CommandRegistry,
    ) {
        use crate::types::TypeKind;
        use std::collections::HashMap;

        if self.var_command_sites.is_empty() && self.cmd_command_sites.is_empty() {
            return;
        }
        // Aggregate type-lattice knowledge per variable name
        // across every FunctionUnit.  For each var with a
        // ``TclType::Object`` lattice entry that has a
        // ``class_name``, record the class qualified name so
        // W308 can validate the method against the class
        // hierarchy.
        let mut all_object_types: HashMap<String, HashSet<String>> = HashMap::new();
        let collect_object_types =
            |fu: &crate::compilation_unit::FunctionUnit,
             out: &mut HashMap<String, HashSet<String>>| {
                for ((sym, _ver), tl) in &fu.types {
                    if tl.kind != TypeKind::Known {
                        continue;
                    }
                    if !matches!(tl.tcl_type, Some(tcl_registry::TclType::Object)) {
                        continue;
                    }
                    let Some(class_name) = &tl.class_name else {
                        continue;
                    };
                    out.entry(fu.ssa.var_name(*sym).to_owned())
                        .or_default()
                        .insert(class_name.clone());
                }
            };
        collect_object_types(&cu.top_level, &mut all_object_types);
        for fu in cu.procedures.values() {
            collect_object_types(fu, &mut all_object_types);
        }
        // Method bodies are real analysable units (`cu.methods` carries a full
        // FunctionUnit per method). Including them lets `$var method` dispatch
        // inside a method body see object/const evidence from the same body.
        for fu in cu.methods.values() {
            collect_object_types(fu, &mut all_object_types);
        }
        self.harvest_constructor_object_types(cu, &mut all_object_types);

        // Build the class hierarchy once for W308 method
        // resolution (uses the ``ClassHierarchy``).
        let hierarchy = if self.result.all_classes.is_empty() {
            None
        } else {
            Some(super::class_hierarchy::build_class_hierarchy(
                self.result.all_classes.clone(),
            ))
        };

        // Aggregate constant-string knowledge per variable name
        // across every function in the CompilationUnit.  CONST and
        // CONSTSET are expanded into a flat set of values.
        let mut all_constsets: HashMap<String, HashSet<String>> = HashMap::new();
        let collect_from = |fu: &crate::compilation_unit::FunctionUnit,
                            out: &mut HashMap<String, HashSet<String>>| {
            for ((sym, _ver), lv) in &fu.sccp.values {
                let Some(values) = lattice_command_values(lv) else {
                    continue;
                };
                let entry = out.entry(fu.ssa.var_name(*sym).to_owned()).or_default();
                for v in values {
                    entry.insert(v);
                }
            }
        };
        collect_from(&cu.top_level, &mut all_constsets);
        for fu in cu.procedures.values() {
            collect_from(fu, &mut all_constsets);
        }
        // A literal `set cmd nope` inside an `oo::class` method body must be
        // captured so SCCP can prove `$cmd` is a non-command — defeating the
        // blanket `in_method` W307 suppression (FP-OBJ-D4-F5).
        for fu in cu.methods.values() {
            collect_from(fu, &mut all_constsets);
        }

        harvest_array_set_constants(cu, &mut all_constsets);
        harvest_array_element_set_constants(cu, &mut all_constsets);
        harvest_dict_with_constants(cu, &mut all_constsets);

        // Build the "known commands" universe — registry +
        // user-defined procs + class tail names.
        let known_cmds: HashSet<String> = registry.command_names().map(str::to_string).collect();
        let known_procs: HashSet<String> = self.result.all_procs.keys().cloned().collect();
        let known_proc_bare: HashSet<String> = known_procs
            .iter()
            .filter_map(|qn| qn.rsplit_once("::").map(|(_, tail)| tail.to_string()))
            .filter(|s| !s.is_empty())
            .collect();
        let known_class_tails: HashSet<String> = self
            .result
            .all_classes
            .keys()
            .filter_map(|qn| qn.rsplit_once("::").map(|(_, tail)| tail.to_string()))
            .filter(|s| !s.is_empty())
            .collect();

        let is_known_command = |v: &str| {
            known_cmds.contains(v)
                || known_procs.contains(v)
                || known_proc_bare.contains(v)
                || known_procs.contains(&format!("::{v}"))
                || known_class_tails.contains(v)
                || self.result.all_classes.contains_key(&format!("::{v}"))
        };

        // Per-SSA-version refinement: map each
        // function to its source range + FunctionUnit so the W307
        // suppression can read the value at the dispatch's *exact* SSA
        // use-version instead of the merged set.  ``::top`` covers the
        // whole source; a proc's narrower range wins where it contains
        // the offset.  Methods are ``in_method``-suppressed, so
        // they are left out.
        let mut func_ranges: Vec<(String, u32, u32)> = vec![("::top".to_string(), 0, u32::MAX)];
        let mut fu_by_qname: HashMap<String, &crate::compilation_unit::FunctionUnit> =
            HashMap::new();
        fu_by_qname.insert("::top".to_string(), &cu.top_level);
        for (qname, fu) in &cu.procedures {
            fu_by_qname.insert(qname.clone(), fu);
            if let Some(ir_proc) = cu.ir_module.procedures.get(qname) {
                func_ranges.push((qname.clone(), ir_proc.span.start(), ir_proc.span.end()));
            }
        }

        // Drain sites so we can borrow self.result mutably below.
        let sites = std::mem::take(&mut self.var_command_sites);
        let objdefined_vars = self.objdefined_vars.clone();
        // Object-factory locals: vars holding a factory result (`set x [Class
        // new]` / `set x [::ns::factory]` / `set x [object_returning_proc]`).
        // A `$x method` dispatch on one suppresses W307 (designed object usage).
        let factory_object_ranges = self.compute_factory_object_ranges(cu, registry);
        let is_factory_local = |var: &str, off: u32| -> bool {
            factory_object_ranges
                .iter()
                .any(|(s, e, names)| *s <= off && off <= *e && names.contains(var))
        };
        // Snit / OO instance-variable dispatch: `$mytree get` where `mytree` is
        // a class instance variable and the dispatch sits inside the class body
        // (including non-method helper `proc`s that `upvar` it). An instance var
        // holds a component / sub-object, so dispatching on it is designed usage
        // — suppress W307.  `snit_var_ranges` is built from every
        // `ClassDef`'s body span + declared `variables`.
        let snit_var_ranges: Vec<(u32, u32, &Vec<String>)> = self
            .result
            .all_classes
            .values()
            .filter(|cd| !cd.variables.is_empty())
            .map(|cd| (cd.body_span.start(), cd.body_span.end(), &cd.variables))
            .collect();
        let is_snit_member = |var: &str, off: u32| -> bool {
            snit_var_ranges
                .iter()
                .any(|(s, e, vars)| *s <= off && off <= *e && vars.iter().any(|v| v == var))
        };

        // **Proc-parameter / multi-dispatch object-dispatch suppression.**
        // A dispatch on a proc
        // *parameter* — `proc walk {tree} { $tree visit }` — is object
        // dispatch the user has documented as the proc's API contract, not a
        // static error.  A non-parameter local dispatched ≥2 times in the same
        // scope is likewise evidenced object usage (a single dispatch could be
        // a typo; repeated use is clearly designed).  Build, per enclosing
        // proc body, its parameter set and the per-var dispatch count, plus a
        // taint carve-out: a *tainted* var is never suppressed (dispatching a
        // user-controlled command name is an injection risk regardless of how
        // many times it appears).  `::top` is the sentinel for statements
        // outside any proc body.
        let mut proc_body_ranges: Vec<(u32, u32, String, HashSet<String>)> = self
            .result
            .all_procs
            .iter()
            .map(|(qname, pdef)| {
                let params: HashSet<String> = pdef.params.iter().map(|p| p.name.clone()).collect();
                (
                    pdef.body_span.start(),
                    pdef.body_span.end(),
                    qname.clone(),
                    params,
                )
            })
            .collect();
        // Innermost-enclosing wins: scan largest-start-first for a range that
        // contains the offset (procs don't nest, but `namespace eval` bodies
        // can wrap several, so this stays robust).  Returns the index into
        // `proc_body_ranges`, or `None` for the `::top` sentinel scope.
        proc_body_ranges.sort_by(|a, b| a.0.cmp(&b.0).then(b.1.cmp(&a.1)));
        let enclosing_idx = |off: u32| -> Option<usize> {
            proc_body_ranges
                .iter()
                .enumerate()
                .rev()
                .find(|(_, (s, e, _, _))| *s <= off && off <= *e)
                .map(|(i, _)| i)
        };
        let scope_qname = |idx: Option<usize>| -> &str {
            idx.map_or(W307_TOP_SCOPE, |i| proc_body_ranges[i].2.as_str())
        };
        let mut dispatch_counts: FxHashMap<(String, String), usize> = FxHashMap::default();
        for site in &sites {
            let qname = scope_qname(enclosing_idx(site.cmd_span.start()));
            *dispatch_counts
                .entry((qname.to_owned(), site.var_name.clone()))
                .or_insert(0) += 1;
        }
        // Per-scope tainted var names — any tainted SSA version of a name
        // disqualifies it from dispatcher-suppression.  Keyed by qname, with
        // `::top` for the top-level scope.
        let tainted_names_of = |fu: &crate::compilation_unit::FunctionUnit| -> HashSet<String> {
            fu.taints
                .iter()
                .filter(|(_, tl)| tl.is_tainted())
                .map(|((sym, _ver), _)| fu.ssa.var_name(*sym).to_owned())
                .collect()
        };
        let mut tainted_by_scope: FxHashMap<String, HashSet<String>> = FxHashMap::default();
        let top_tainted = tainted_names_of(&cu.top_level);
        if !top_tainted.is_empty() {
            tainted_by_scope.insert(W307_TOP_SCOPE.to_owned(), top_tainted);
        }
        for (qname, fu) in &cu.procedures {
            let names = tainted_names_of(fu);
            if !names.is_empty() {
                tainted_by_scope.insert(qname.clone(), names);
            }
        }

        for site in &sites {
            // **W308 path.**  Variable known to hold an Object
            // — validate the method name against the class
            // hierarchy.  When the method isn't found and the
            // class doesn't have an external superclass that
            // could carry it, emit W308.
            if let Some(class_names) = all_object_types.get(&site.var_name) {
                if let (Some(method_name), Some(hierarchy)) = (&site.method_name, &hierarchy) {
                    let mut found = false;
                    let mut has_local_class = false;
                    for cls in class_names {
                        if hierarchy.method_target(cls, method_name).is_some() {
                            found = true;
                            break;
                        }
                        if let Some(cd) = self.result.all_classes.get(cls) {
                            has_local_class = true;
                            if cd.methods.contains_key(method_name)
                                || cd.class_methods.contains_key(method_name)
                                || matches!(
                                    method_name.as_str(),
                                    "new" | "create" | "destroy" | "configure" | "cget"
                                )
                                || cd.methods.contains_key("unknown")
                            {
                                found = true;
                                break;
                            }
                        }
                    }
                    // Inherited ``unknown`` handler via MRO.
                    if !found && has_local_class {
                        for cls in class_names {
                            if hierarchy.method_target(cls, "unknown").is_some() {
                                found = true;
                                break;
                            }
                        }
                    }
                    // External superclass: a method might come
                    // from a class outside the current index.
                    if !found && has_local_class {
                        const OO_BASE: &[&str] = &["oo::object", "oo::class"];
                        'cls_loop: for cls in class_names {
                            if let Some(cd) = self.result.all_classes.get(cls) {
                                for s in &cd.superclasses {
                                    if !self.result.all_classes.contains_key(s)
                                        && !OO_BASE.contains(&s.as_str())
                                    {
                                        found = true;
                                        break 'cls_loop;
                                    }
                                }
                            }
                        }
                    }
                    // ``oo::objdefine`` adds per-instance
                    // methods we can't see at the class level.
                    if !found && objdefined_vars.contains(&site.var_name) {
                        found = true;
                    }
                    // snit instances route method calls through delegation /
                    // hull / options / built-ins (`$self`, `-option` cget/configure,
                    // `info`/`destroy`), none of which the analyser models — so
                    // method validation on a snit-typed receiver is unsound and
                    // never fires W308 (FP-OBJ-05).
                    if !found
                        && class_names.iter().any(|cls| {
                            self.result
                                .all_classes
                                .get(cls)
                                .is_some_and(|cd| cd.metaclass.contains("snit::"))
                        })
                    {
                        found = true;
                    }
                    if !found && has_local_class && !self.disabled_diagnostics.contains("W308") {
                        let mut classes_sorted: Vec<&str> =
                            class_names.iter().map(String::as_str).collect();
                        classes_sorted.sort_unstable();
                        let cls_display = classes_sorted.join(", ");
                        let message =
                            format!("Unknown method '{method_name}' on class '{cls_display}'");
                        self.result.diagnostics.push(super::types::Diagnostic {
                            code: DiagCode::W308,
                            span: site.cmd_span,
                            message,
                            severity: Severity::Warning,
                            fixes: Vec::new(),
                        });
                    }
                }
                // W307 path doesn't fire when the var is a
                // known Object — the method-name check is the
                // load-bearing piece.
                continue;
            }

            // **W307 path.**  Variable not a known Object.
            // Resolve the dispatch's value first: prefer the exact SSA
            // use-version, falling back to the merged constset.  This drops the
            // merged-set false positive on a variable reassigned from a
            // non-command to a known command before the dispatch
            // (`set c x; set c puts; $c ...`).
            let precise = w307_precise_cmd_values(
                &func_ranges,
                &fu_by_qname,
                site.cmd_span.start(),
                &site.var_name,
            );
            let effective = precise
                .as_ref()
                .or_else(|| all_constsets.get(&site.var_name));
            // SCCP concrete evidence the value IS a known command — suppress.
            if effective.is_some_and(|v| !v.is_empty() && v.iter().all(|x| is_known_command(x))) {
                continue;
            }
            // SCCP concrete evidence the value is NOT a command: every feasible
            // value is a literal and none is a known command.  When SCCP proves
            // this, the heuristic object-dispatch suppressions below (in-method,
            // proc-param / multi-dispatch) must not silence the real "invalid
            // command name" hazard (FP-OBJ-09 / FP-OBJ-D4-F5).
            let sccp_not_command =
                effective.is_some_and(|v| !v.is_empty() && v.iter().all(|x| !is_known_command(x)));
            // ``in_method`` short-circuits W307 because OO methods routinely use
            // ``$obj method`` patterns — unless SCCP proves a non-command value.
            if site.in_method && !sccp_not_command {
                continue;
            }
            // Proc-parameter / multi-dispatch object-dispatch suppression: a
            // dispatch on a parameter of the enclosing proc (any count), or on
            // a non-parameter local dispatched ≥2 times in the same scope, is
            // evidenced object usage — suppress unless the var is tainted.
            let idx = enclosing_idx(site.cmd_span.start());
            let encl_qname = scope_qname(idx);
            let is_param = idx.is_some_and(|i| proc_body_ranges[i].3.contains(&site.var_name));
            let dispatch_count = dispatch_counts
                .get(&(encl_qname.to_owned(), site.var_name.clone()))
                .copied()
                .unwrap_or(0);
            let dispatcher_suppressed = is_param || dispatch_count >= 2;
            let tainted = tainted_by_scope
                .get(encl_qname)
                .is_some_and(|s| s.contains(&site.var_name));
            if dispatcher_suppressed && !tainted && !sccp_not_command {
                continue;
            }
            // Namespaced-ensemble dispatch: `${ns}::tail` / `$ns::tail` where
            // `ns` holds a namespace prefix and `::tail` composes a qualified
            // command path (tcllib's logger / dns / irc modules use this).
            // When the prefix is an SCCP const and *every* composed name
            // `<value>::tail` resolves to a known command/proc/class, the
            // dispatch is statically resolvable — suppress.  A composition
            // that resolves to nothing (unknown proc) still fires.
            if let Some((prefix, tail)) = parse_namespaced_ensemble(&self.source, site.cmd_span)
                && let Some(values) = all_constsets.get(&prefix)
                && !values.is_empty()
                && values
                    .iter()
                    .all(|v| is_known_command(&format!("{v}::{tail}")))
            {
                continue;
            }
            // Object-factory provenance: `$var` holds a factory result in this
            // scope — a designed object handle, so the dispatch is not a static
            // error. W307 exemption.
            if is_factory_local(&site.var_name, site.cmd_span.start()) {
                continue;
            }
            // Class instance-variable dispatch inside the class body (component
            // / sub-object) — W307 exemption.
            if is_snit_member(&site.var_name, site.cmd_span.start()) {
                continue;
            }
            // Callback-registration array slot: `$state(-command)` /
            // `$state(doneCallback)` dispatches a command the user registered
            // into a switch-style option / callback slot. Unless SCCP has
            // concrete evidence the slot holds a non-command (handled above via
            // `sccp_not_command`, e.g. `array set state {-command notACommand}`
            // or `set state(-command) notACommand`), treat it as a designed
            // callback dispatch (FP-OBJ-10).
            if !sccp_not_command && is_callback_array_slot(&site.var_name) {
                continue;
            }
            self.result.diagnostics.push(super::types::Diagnostic {
                code: DiagCode::W307,
                span: site.cmd_span,
                message: "Non-literal command name — cannot statically analyze".to_string(),
                severity: Severity::Warning,
                fixes: Vec::new(),
            });
        }
        // Restore the sites list — snapshot/restore expects it
        // to round-trip independently of emission.
        self.var_command_sites = sites;

        // ``[cmd] method`` sites — emit
        // W307 only when the inner command's return type is
        // unknown AND the call isn't an OO self-dispatch
        // (``my`` / ``self``).  When the return type is a
        // known class, validate the method against the
        // hierarchy and emit W308 instead of W307.
        let cmd_sites = std::mem::take(&mut self.cmd_command_sites);
        for site in &cmd_sites {
            // `[cmd]::method` namespaced-ensemble dispatch (FP-OBJ-07): a
            // command-substitution head composed with a literal `::method` tail.
            // The literal tail is static method-name evidence — the dispatch is
            // well-formed (only the namespace prefix is computed at runtime), so
            // W307 must not fire. A bare `[cmd] arg` dispatch with no `::method`
            // tail has no such evidence and still fires.
            {
                let s = site.cmd_span.start() as usize;
                let e = (site.cmd_span.end() as usize).min(self.source.len());
                let word = &self.source[s..e];
                if word.starts_with('[')
                    && let Some(p) = word.find("]::")
                {
                    let tail = &word[p + 3..];
                    if !tail.is_empty()
                        && tail.chars().all(|c| c.is_alphanumeric() || c == '_' || c == ':')
                    {
                        continue;
                    }
                }
            }
            // No blanket `in_method` suppression: an in-method `[cmd] method`
            // dispatch must earn its silence from a positive signal (a known
            // OBJECT return type, or `my`/`self` self-dispatch resolving to a
            // method that returns an object).
            //
            // Parse the command-substitution text into
            // ``head ?args...``.  ``cmd_text`` is what the
            // analyser captured from
            // ``SourceMap::token_text``; the leading ``[`` /
            // trailing ``]`` are stripped already because
            // ``content_offset`` skipped them.
            let inner = site.cmd_text.trim();
            let inner = inner
                .strip_prefix('[')
                .map_or(inner, str::trim)
                .strip_suffix(']')
                .map_or(inner, str::trim);
            let mut parts = inner.split_whitespace();
            let Some(head) = parts.next() else {
                continue;
            };
            let arg_strs: Vec<&str> = parts.collect();

            // OO self-dispatch (`my <method>` / `self <method>`): by default
            // the return is treated as an object handle (suppress).  But when
            // the dispatched method resolves in the enclosing class and its
            // body is a simple `return <literal>`, the result is a plain
            // string, not an object — so the *outer* dispatch fires W307.
            if matches!(head, "my" | "self") {
                let returns_literal = arg_strs.first().is_some_and(|method| {
                    self.oo_self_method_returns_literal(site.cmd_span.start(), method)
                });
                if returns_literal {
                    self.result.diagnostics.push(super::types::Diagnostic {
                        code: DiagCode::W307,
                        span: site.cmd_span,
                        message: "Non-literal command name — cannot statically analyze".to_string(),
                        severity: Severity::Warning,
                        fixes: Vec::new(),
                    });
                }
                continue;
            }

            // ``[Dog new]`` / ``[Dog create
            // name]`` produce an Object whose class is ``Dog``.
            // The registry lookup for the bare class name
            // returns Overdefined (the class isn't a built-in
            // command) so we recognise the constructor pattern
            // explicitly here — ``known_class new/create`` maps to
            // ``TclType.OBJECT`` with the class name attached.
            let class_qn = self.canonicalise_class_name(head);
            let head_is_known_class = self.result.all_classes.contains_key(&class_qn)
                || self.result.all_classes.contains_key(head);
            let is_constructor_call = head_is_known_class
                && arg_strs
                    .first()
                    .is_some_and(|sub| matches!(*sub, "new" | "create"));

            // Look up the return type via the registry.  When
            // the head is a user proc / class, fall back to
            // ``Overdefined`` (matches the registry behaviour
            // for unknown commands).
            let ret_type = if is_constructor_call {
                crate::types::TypeLattice {
                    kind: crate::types::TypeKind::Known,
                    tcl_type: Some(tcl_registry::TclType::Object),
                    from_type: None,
                    class_name: Some(class_qn.clone()),
                }
            } else {
                // The constructor case is already handled inline above using the
                // analyser's authoritative class set, so the registry fallback
                // only needs to recognise registered built-ins here — pass an
                // empty class set / root namespace.
                crate::type_infer::return_type_for_command(
                    registry,
                    head,
                    &arg_strs,
                    &std::collections::HashSet::new(),
                    "::",
                )
            };

            // ``Object`` return type — suppress W307; if the
            // class is known, validate the method (W308).
            let is_object = ret_type.kind == crate::types::TypeKind::Known
                && matches!(ret_type.tcl_type, Some(tcl_registry::TclType::Object));
            if is_object {
                if !self.disabled_diagnostics.contains("W308")
                    && let (Some(method), Some(class_name)) =
                        (site.method_name.as_ref(), ret_type.class_name.as_ref())
                {
                    let cls_qn = self.canonicalise_class_name(class_name);
                    let cd = self.result.all_classes.get(&cls_qn).cloned();
                    let method_ok = self.validate_method_on_class(
                        &cls_qn,
                        method,
                        cd.as_ref(),
                        hierarchy.as_ref(),
                    );
                    if !method_ok {
                        self.result.diagnostics.push(super::types::Diagnostic {
                            code: DiagCode::W308,
                            span: site.cmd_span,
                            message: format!("Unknown method '{method}' on class '{class_name}'"),
                            severity: Severity::Warning,
                            fixes: Vec::new(),
                        });
                    }
                }
                continue;
            }

            // Type is unknown — emit W307 (only the emit-half
            // for the residual unknown-type case).
            self.result.diagnostics.push(super::types::Diagnostic {
                code: DiagCode::W307,
                span: site.cmd_span,
                message: "Non-literal command name — cannot statically analyze".to_string(),
                severity: Severity::Warning,
                fixes: Vec::new(),
            });
        }
        self.cmd_command_sites = cmd_sites;
    }

    /// Resolve a possibly-bare class name to its fully-qualified
    /// form keyed in `result.all_classes`.
    fn canonicalise_class_name(&self, name: &str) -> String {
        if name.starts_with("::") {
            return name.to_string();
        }
        let qualified = format!("::{name}");
        if self.result.all_classes.contains_key(&qualified) {
            qualified
        } else {
            name.to_string()
        }
    }

    /// Decide whether `method` is callable on `class_name`,
    /// consulting the class hierarchy + the class's local
    /// method tables.
    ///
    /// A method is OK when
    /// the class's MRO produces a concrete provider, or the
    /// class is external (no local `ClassDef`), or the method
    /// is one of the OO standard hooks (``new`` / ``create`` /
    /// ``destroy`` / ``configure`` / ``cget``), or the class
    /// declares an ``unknown`` method, or the class extends an
    /// external superclass we can't introspect.
    fn validate_method_on_class(
        &self,
        class_name: &str,
        method: &str,
        cd: Option<&super::types::ClassDef>,
        hierarchy: Option<&super::class_hierarchy::ClassHierarchy>,
    ) -> bool {
        if hierarchy.is_some_and(|h| h.method_target(class_name, method).is_some()) {
            return true;
        }
        let Some(cd) = cd else {
            // External class — can't validate.
            return true;
        };
        if cd.methods.contains_key(method) || cd.class_methods.contains_key(method) {
            return true;
        }
        if matches!(method, "new" | "create" | "destroy" | "configure" | "cget") {
            return true;
        }
        if cd.methods.contains_key("unknown") {
            return true;
        }
        if hierarchy.is_some_and(|h| h.method_target(class_name, "unknown").is_some()) {
            return true;
        }
        // External superclass ⇒ skip W308.
        if !cd.superclasses.is_empty() {
            for s in &cd.superclasses {
                if !self.result.all_classes.contains_key(s) && !OO_BASE.contains(&s.as_str()) {
                    return true;
                }
            }
        }
        false
    }

    /// Suppress W123 diagnostics whose command-name contains a
    /// `$` interpolation that resolves cleanly via SCCP.
    ///
    /// Walks every emitted W123, extracts the command name
    /// from the message, and runs
    /// [`crate::text::fold_interpolation_set`] over the
    /// aggregated SCCP results.  When every resolved value is
    /// a known command, proc, class, or class-tail name, the
    /// W123 is removed.
    ///
    /// **Simplification.**  This uses the union of
    /// every function's SCCP — slightly more permissive
    /// (over-suppresses if a same-named variable in a
    /// different function happens to resolve cleanly) but
    /// safe in practice.  Range-based per-function lookup
    /// could be added later.
    pub(super) fn resolve_interpolated_w123_diagnostics(
        &mut self,
        cu: &crate::compilation_unit::CompilationUnit,
    ) {
        use crate::analyses::{ConstValue, LatticeValue};
        use std::collections::HashMap;

        // Bail early when no W123 carries a ``$`` — the common
        // case for non-iRules code.
        let has_interpolated = self
            .result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W123 && d.message.contains('$'));
        if !has_interpolated {
            return;
        }

        // Aggregate SCCP-resolved string sets per variable name
        // across every function in the CU.  Same shape as
        // ``emit_var_command_diagnostics``.
        let mut all_constsets: HashMap<String, HashSet<String>> = HashMap::new();
        let collect_from = |fu: &crate::compilation_unit::FunctionUnit,
                            out: &mut HashMap<String, HashSet<String>>| {
            for ((sym, _ver), lv) in &fu.sccp.values {
                let values: Option<Vec<String>> = match lv {
                    LatticeValue::Const(ConstValue::String(s)) => Some(vec![s.clone()]),
                    LatticeValue::ConstSet(set) => set
                        .iter()
                        .map(|cv| match cv {
                            ConstValue::String(s) => Some(s.clone()),
                            _ => None,
                        })
                        .collect::<Option<Vec<_>>>(),
                    _ => None,
                };
                let Some(values) = values else { continue };
                let entry = out.entry(fu.ssa.var_name(*sym).to_owned()).or_default();
                for v in values {
                    entry.insert(v);
                }
            }
        };
        collect_from(&cu.top_level, &mut all_constsets);
        for fu in cu.procedures.values() {
            collect_from(fu, &mut all_constsets);
        }

        // Build the universe of names that count as "known
        // commands" for the resolution check.  Same set the
        // emitter used to skip suggestions in the first pass.
        let registry = tcl_registry::CommandRegistry::build_default();
        let known_cmds: HashSet<String> = registry.command_names().map(str::to_string).collect();
        let known_proc_tails: HashSet<String> = self
            .result
            .all_procs
            .keys()
            .filter_map(|qn| qn.rsplit_once("::").map(|(_, t)| t.to_string()))
            .filter(|s| !s.is_empty())
            .collect();

        // Walk W123 diagnostics and remove those whose
        // interpolated command name resolves cleanly.
        let drained = std::mem::take(&mut self.result.diagnostics);
        let mut kept: Vec<super::types::Diagnostic> = Vec::with_capacity(drained.len());
        for d in drained {
            if d.code != DiagCode::W123 {
                kept.push(d);
                continue;
            }
            let Some(cmd_name) = extract_quoted_word(&d.message) else {
                kept.push(d);
                continue;
            };
            if !cmd_name.contains('$') {
                kept.push(d);
                continue;
            }
            let Some(resolved) = crate::text::fold_interpolation_set(&cmd_name, &all_constsets)
            else {
                kept.push(d);
                continue;
            };
            // All resolved candidates must be known commands.
            let all_known = resolved.iter().all(|name| {
                known_cmds.contains(name)
                    || known_proc_tails.contains(name)
                    || self.result.all_procs.contains_key(&format!("::{name}"))
                    || self.result.all_procs.contains_key(name)
            });
            if all_known {
                // Suppress this W123 — the interpolated head
                // statically resolves to a known command set.
                continue;
            }
            kept.push(d);
        }
        self.result.diagnostics = kept;
    }
}

/// Parse a namespaced-ensemble dispatch head `${prefix}::tail` from the source
/// slice at `span`, returning `(prefix_var_name, tail)`.  Returns `None` when
/// the head isn't this shape.
///
/// Only the **braced** form composes a command path.  A bare `$prefix::tail`
/// is lexed by Tcl as a *single* variable named `prefix::tail` (the runtime
/// reads that variable — it is not `$prefix` followed by a literal `::tail`),
/// so it must NOT be treated as ensemble dispatch.  This only matters after
/// a `${…}` closing brace — the bare VAR token already swallows the `::tail`,
/// so the character after it is never `::`.
fn parse_namespaced_ensemble(source: &str, span: tcl_lexer::Span) -> Option<(String, String)> {
    let start = span.start() as usize;
    let end = (span.end() as usize).min(source.len());
    if start >= end {
        return None;
    }
    let head = &source[start..end];
    let braced = head.strip_prefix("${")?;
    let close = braced.find('}')?;
    let (prefix, after) = (&braced[..close], &braced[close + 1..]);
    let tail = after.strip_prefix("::")?;
    // Both prefix and tail must be non-empty; a `${arr(key)}` array element is
    // not an ensemble prefix.
    if prefix.is_empty() || tail.is_empty() || prefix.contains('(') {
        return None;
    }
    Some((prefix.to_string(), tail.to_string()))
}

/// Harvest `array set arr {k1 v1 k2 v2 …}` literal element values into the
/// constset map keyed by `arr(key)`, so the W307 callback-array suppression
/// can check the *actual* value of `$arr(-command)` against the known-command
/// set.  Without this, the dash-prefixed / callback-suffixed array-key
/// heuristic fires even when SCCP-equivalent literal evidence proves the value
/// is (or isn't) a command.
/// True when `var_name` is an array element `base(key)` whose key denotes a
/// switch-style callback / option registration slot: a dash-prefixed option
/// key (`-command`) or a callback-shaped suffix word
/// (`cmd`/`command`/`callback`/`handler`/`hook`/`proc`). Dispatching such a
/// slot is a designed callback invocation, not a stray non-literal command
/// (FP-OBJ-10); the caller still fires W307 when SCCP proves the slot holds a
/// concrete non-command value.
fn is_callback_array_slot(var_name: &str) -> bool {
    let Some((_base, rest)) = var_name.split_once('(') else {
        return false;
    };
    let Some(key) = rest.strip_suffix(')') else {
        return false;
    };
    if key.starts_with('-') {
        return true;
    }
    let k = key.to_ascii_lowercase();
    ["cmd", "command", "callback", "handler", "hook", "proc"]
        .iter()
        .any(|suffix| k.ends_with(suffix))
}

/// Harvest direct single-element array assignments — `set arr(key) <literal>`,
/// which lowers to an `AssignValue { name: "arr(key)", value }` (the scalar
/// `set_literal_body` path excludes `(`-bearing names) — into the constset map
/// keyed by `arr(key)`. The SSA const collector keys on scalar SSA variables
/// and does not track array elements, and `harvest_array_set_constants` only
/// covers the `array set` list form; this covers the single-element form so the
/// W307 callback-array suppression can see the slot's concrete value
/// (FP-OBJ-10 SCCP-evidence override). Also accepts the `AssignConst` / generic
/// `Call "set"` shapes defensively.
fn harvest_array_element_set_constants(
    cu: &crate::compilation_unit::CompilationUnit,
    out: &mut HashMap<String, HashSet<String>>,
) {
    use crate::ir::Statement;
    let is_literal = |s: &str| !s.contains('$') && !s.contains('[');
    let is_array_elem = |name: &str| name.contains('(') && name.ends_with(')');
    let units = std::iter::once(&cu.top_level).chain(cu.procedures.values());
    for fu in units {
        for block in fu.cfg.blocks.values() {
            for stmt in &block.statements {
                match stmt {
                    Statement::AssignValue { name, value, .. }
                        if is_array_elem(name) && is_literal(value) =>
                    {
                        out.entry(name.clone()).or_default().insert(value.clone());
                    }
                    Statement::AssignConst { name, value, .. } if is_array_elem(name) => {
                        out.entry(name.clone()).or_default().insert(value.clone());
                    }
                    Statement::Call { command, args, .. }
                        if command == "set"
                            && args.len() == 2
                            && is_array_elem(&args[0])
                            && is_literal(&args[1]) =>
                    {
                        out.entry(args[0].clone())
                            .or_default()
                            .insert(args[1].clone());
                    }
                    _ => {}
                }
            }
        }
    }
}

fn harvest_array_set_constants(
    cu: &crate::compilation_unit::CompilationUnit,
    out: &mut HashMap<String, HashSet<String>>,
) {
    use crate::ir::Statement;
    let units = std::iter::once(&cu.top_level).chain(cu.procedures.values());
    for fu in units {
        for block in fu.cfg.blocks.values() {
            for stmt in &block.statements {
                let (Statement::Call { command, args, .. }
                | Statement::Barrier { command, args, .. }) = stmt
                else {
                    continue;
                };
                let is_array =
                    command == "array" || stmt.canonical_command_or_source() == "::array";
                if !is_array || args.first().map(String::as_str) != Some("set") || args.len() < 3 {
                    continue;
                }
                let arr_name = &args[1];
                let items = crate::tcl_expr_eval::split_tcl_list(&args[2]);
                if !items.len().is_multiple_of(2) {
                    continue;
                }
                for pair in items.chunks_exact(2) {
                    let elem_name = format!("{arr_name}({})", pair[0]);
                    out.entry(elem_name).or_default().insert(pair[1].clone());
                }
            }
        }
    }
}

/// Harvest `dict with d { … }` unpacked variable values: when `d` is a known
/// literal dict (via SCCP CONST at param entry — usually from call-site
/// constant propagation), the body sees each dict key as a local variable
/// bound to its value.  Register those bindings so a `$cmd hi` dispatch inside
/// the body checks `cmd`'s value against the known-command set.
fn harvest_dict_with_constants(
    cu: &crate::compilation_unit::CompilationUnit,
    out: &mut HashMap<String, HashSet<String>>,
) {
    use crate::ir::Statement;
    let units = std::iter::once(&cu.top_level).chain(cu.procedures.values());
    for fu in units {
        for block in fu.cfg.blocks.values() {
            for stmt in &block.statements {
                let (Statement::Barrier { command, args, .. }
                | Statement::Call { command, args, .. }) = stmt
                else {
                    continue;
                };
                let is_dict = command == "dict" || stmt.canonical_command_or_source() == "::dict";
                if !is_dict || args.first().map(String::as_str) != Some("with") {
                    continue;
                }
                let Some(dict_var) = args.get(1) else {
                    continue;
                };
                let dvar = crate::naming::normalise_var_name(dict_var);
                // The call-site-propagated literal lands at the param entry (v0).
                let Some(crate::analyses::LatticeValue::Const(
                    crate::analyses::ConstValue::String(dict_text),
                )) = fu
                    .ssa
                    .var_symbol(dvar)
                    .and_then(|s| fu.sccp.values.get(&(s, 0)))
                else {
                    continue;
                };
                let items = crate::tcl_expr_eval::split_tcl_list(dict_text);
                if !items.len().is_multiple_of(2) {
                    continue;
                }
                for pair in items.chunks_exact(2) {
                    out.entry(pair[0].clone())
                        .or_default()
                        .insert(pair[1].clone());
                }
            }
        }
    }
}

/// Sentinel scope key for the W307 dispatcher-suppression maps covering
/// statements outside any proc body.
const W307_TOP_SCOPE: &str = "::top";

/// The variable named by a single `$var` / `${var}` substitution, or `None`.
///
/// The text must be exactly one bare or braced variable reference whose name
/// is made of word / namespace characters.  Anything else (literals, command
/// subs, composite words) yields `None`.
fn extract_dollar_var(value: &str) -> Option<String> {
    let v = value.trim();
    let rest = v.strip_prefix('$')?;
    let is_name = |s: &str| {
        !s.is_empty()
            && s.bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b':')
    };
    if let Some(inner) = rest.strip_prefix('{').and_then(|r| r.strip_suffix('}')) {
        // Braced `${name}` — reject nested braces.
        if !inner.contains('{') && is_name(inner) {
            return Some(inner.to_string());
        }
        return None;
    }
    is_name(rest).then(|| rest.to_string())
}

/// The variable returned by a proc's **last** `return $var`, or `None`.
///
/// Walks every block's statements and terminator (returns can lower to either
/// an `IRReturn` statement or a `Return` terminator) and keeps the last whose
/// value is a single `$var`.  Used by the object-returning-proc inference:
/// a proc returning `$X` where `X` was assigned from a factory is itself an
/// object factory.
fn last_return_var_of(cfg: &crate::cfg::Function) -> Option<String> {
    use crate::cfg::Terminator;
    use crate::ir::Statement;
    let mut last = None;
    for block in cfg.blocks.values() {
        for stmt in &block.statements {
            if let Statement::Return { value: Some(v), .. } = stmt
                && let Some(name) = extract_dollar_var(v)
            {
                last = Some(name);
            }
        }
        if let Some(Terminator::Return { value: Some(v), .. }) = &block.terminator
            && let Some(name) = extract_dollar_var(v)
        {
            last = Some(name);
        }
    }
    last
}

/// Every return value (statement + terminator) a proc body can produce, as raw
/// text.  Seeds the object-returning-proc inference.
fn return_values_of(cfg: &crate::cfg::Function) -> Vec<String> {
    use crate::cfg::Terminator;
    use crate::ir::Statement;
    let mut out = Vec::new();
    for block in cfg.blocks.values() {
        for stmt in &block.statements {
            if let Statement::Return { value: Some(v), .. } = stmt {
                out.push(v.clone());
            }
        }
        if let Some(Terminator::Return { value: Some(v), .. }) = &block.terminator {
            out.push(v.clone());
        }
    }
    out
}

/// External OO base classes that aren't in the per-document
/// ``ClassDef`` index but are recognised as legitimate
/// superclasses for W308 / W308-related gates.
const OO_BASE: [&str; 2] = ["oo::object", "oo::class"];

/// Extract the first single-quoted word from a diagnostic
/// message string, or `None` if the message has no quoted run.
///
/// Used by [`Analyser::resolve_interpolated_w123_diagnostics`]
/// to recover the command name from a "Unknown command 'NAME'"
/// W123 message.
fn extract_quoted_word(message: &str) -> Option<String> {
    let start = message.find('\'')?;
    let rest = &message[start + 1..];
    let end = rest.find('\'')?;
    Some(rest[..end].to_string())
}

/// Expand a CONST / CONSTSET lattice value into the flat set of its
/// string values, or `None` for any non-string-constant lattice state.
fn lattice_command_values(lv: &crate::analyses::LatticeValue) -> Option<Vec<String>> {
    use crate::analyses::{ConstValue, LatticeValue};
    match lv {
        LatticeValue::Const(ConstValue::String(s)) => Some(vec![s.clone()]),
        LatticeValue::ConstSet(set) => set
            .iter()
            .map(|cv| match cv {
                ConstValue::String(s) => Some(s.clone()),
                _ => None,
            })
            .collect::<Option<Vec<_>>>(),
        _ => None,
    }
}

/// The SCCP value set of `var_name` at the SSA use-version that reaches
/// the dispatch statement at `offset` (W307 per-SSA-version refinement).
///
/// The merged `all_constsets` map unions every version of a variable,
/// so `set c notacommand; set c parse; $c x` wrongly keeps
/// `notacommand` in the set even though only the `parse` version
/// reaches the dispatch. Reading the value at the use site's exact
/// version removes that false positive.
///
/// Purely additive: returns a set only when a CFG statement containing
/// `offset` that *uses* `var_name` is found and its version has a
/// concrete CONST / CONSTSET value — otherwise `None`, and the caller
/// falls back to the merged-set logic. Never broadens a fire into a
/// suppression unsoundly — the value is the exact one flowing into the
/// dispatch.
fn w307_precise_cmd_values(
    func_ranges: &[(String, u32, u32)],
    fu_by_qname: &std::collections::HashMap<String, &crate::compilation_unit::FunctionUnit>,
    offset: u32,
    var_name: &str,
) -> Option<HashSet<String>> {
    // Narrowest function range containing `offset`.
    let mut best: Option<(u32, &str)> = None;
    for (qname, start, end) in func_ranges {
        if *start <= offset && offset <= *end {
            let width = end - start;
            if best.is_none_or(|(bw, _)| width < bw) {
                best = Some((width, qname.as_str()));
            }
        }
    }
    let fu = fu_by_qname.get(best?.1)?;
    // A command-head variable that is not an SSA variable of `fu` has no
    // precise per-version value here.
    let sym = fu.ssa.var_symbol(var_name)?;

    // Narrowest CFG statement containing `offset` that uses `var_name`,
    // reading its SSA use-version (CFG / SSA blocks are parallel-indexed).
    let mut best_width: Option<u32> = None;
    let mut best_version: Option<u32> = None;
    for (block_name, block) in &fu.cfg.blocks {
        let Some(ssa_block) = fu.ssa.blocks.get(block_name) else {
            continue;
        };
        for (idx, stmt) in block.statements.iter().enumerate() {
            let span = fu.abs_span(stmt.span());
            if !(span.start() <= offset && offset <= span.end()) {
                continue;
            }
            let Some(ssa_stmt) = ssa_block.statements.get(idx) else {
                continue;
            };
            let Some(version) = ssa_stmt.uses.get(&sym) else {
                continue;
            };
            let width = span.end() - span.start();
            if best_width.is_none_or(|bw| width < bw) {
                best_width = Some(width);
                best_version = Some(*version);
            }
        }
    }
    let version = best_version?;
    let lv = fu.sccp.values.get(&(sym, version))?;
    Some(lattice_command_values(lv)?.into_iter().collect())
}
