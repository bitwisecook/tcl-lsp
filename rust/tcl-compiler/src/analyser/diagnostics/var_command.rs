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

//! Variable-as-command and `TclOO` method-dispatch checks (the
//! cross-function post-pass).
//!
//! Resolves `$var`-as-command call sites collected during the walk: a
//! non-literal command word that cannot be proved safe (W307), an
//! unknown method invoked on an object whose class is known (W308), using
//! MRO-aware method resolution over the class hierarchy with the usual
//! suppression paths (inherited `unknown` handler, external superclass,
//! `oo::objdefine` per-instance methods), and a dispatch with no method
//! word at all on a known `TclOO` object (E001 — see
//! [`Analyser::e001_for_bare_object_dispatch`]). Tracks the object types
//! produced by constructors and factory procedures so a later `$obj
//! badMethod` resolves against the right class, and resolves
//! partially-interpolated command heads that fold to a finite
//! known-command set (W123 suppression).

use std::collections::{HashMap, HashSet};
use tcl_core_types::DiagCode;

use rustc_hash::{FxHashMap, FxHashSet};

use crate::analyser::state::Analyser;
use crate::analyser::types::Severity;

/// Dispatch reach and source anchors consumed together when W308 is built.
#[derive(Clone, Copy)]
struct W308DiagnosticSite {
    reach: tcl_registry::definer::MethodReach,
    method_span: Option<tcl_lexer::Span>,
    cmd_span: tcl_lexer::Span,
}

impl From<&super::state::VarCommandSite> for W308DiagnosticSite {
    fn from(site: &super::state::VarCommandSite) -> Self {
        Self {
            reach: site.receiver.method_reach(),
            method_span: site.method_span,
            cmd_span: site.cmd_span,
        }
    }
}

/// All the borrowed analysis data the W307 per-site suppression decision
/// reads, bundled so [`Analyser::w307_site_suppressed`] takes one context
/// argument instead of a dozen.
struct W307Ctx<'a> {
    all_constsets: &'a std::collections::HashMap<String, HashSet<String>>,
    func_ranges: &'a [W307FunctionRange<'a>],
    factory_object_ranges: &'a [(u32, u32, HashSet<String>)],
    snit_var_ranges: &'a [(u32, u32, &'a Vec<String>)],
    proc_body_ranges: &'a [(u32, u32, String, HashSet<String>)],
    dispatch_counts: &'a FxHashMap<(String, String), usize>,
    tainted_by_scope: &'a FxHashMap<String, HashSet<String>>,
}

/// Exact declaration owner retained with its original range.
struct W307FunctionRange<'a> {
    function: &'a crate::compilation_unit::FunctionUnit,
    start: u32,
    end: u32,
}

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
        let Some(class_def) = self.enclosing_class_at_offset(site_offset) else {
            return false;
        };
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
        let Some(mut bt) = Analyser::source_slice(&self.source, start, end).map(str::trim) else {
            return false;
        };
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
        !ret_arg.is_empty() && !ret_arg.contains('[') && !ret_arg.contains('$')
    }

    /// The `ClassDef` whose body contains `offset` — the enclosing `TclOO`
    /// class for a call site inside a method body, if any. Naive
    /// first-match over [`AnalysisResult::all_classes`] (class bodies don't
    /// nest in practice); shared by [`Self::oo_self_method_returns_literal`]
    /// and the `[self]`/`[self object]` self-receiver W308 check, so the two
    /// "what class is this dispatch inside" answers
    /// cannot drift apart.
    fn enclosing_class_at_offset(&self, offset: u32) -> Option<&super::types::ClassDef> {
        self.result.all_classes.values().find(|class_def| {
            let body = class_def.body_span;
            body.start() <= offset && offset <= body.end()
        })
    }

    /// An absent method name can be diagnosed only from the immutable method
    /// inventory captured by the actual instance receipt. A name in that
    /// inventory supplies no visibility, arity, or dispatch implementation proof.
    fn w308_for_object_var(
        &self,
        site: &crate::analyser::state::VarCommandSite,
        instance: &crate::command_binding::SourceObjectInstanceProof,
    ) -> Option<super::types::Diagnostic> {
        let class = &instance.class_target().command;
        let Some(method) = site.method_name.as_ref() else {
            return self.e001_for_bare_object_dispatch(site, &HashSet::from([class.clone()]));
        };
        let input = self
            .head_identities
            .source_bindings_ref()
            .original_written_name_input_at_span(site.method_span?, self.lexer_config())?;
        input
            .policy()
            .recipe()
            .oo_method_input(input.bytes())
            .ok()?;
        if instance.instance_method_names()?.contains(input.bytes())
            || !Self::method_word_is_literal(method)
            || self.disabled_diagnostics.contains("W308")
        {
            return None;
        }
        // The receipt includes private names only to avoid false absence
        // claims. It cannot safely supply public-method suggestions either.
        Some(super::types::Diagnostic::new(
            DiagCode::W308,
            site.method_span.unwrap_or(site.cmd_span),
            format!("Unknown method '{method}' on class '{class}'"),
            Severity::Warning,
        ))
    }

    /// Build the W308 (unknown method) diagnostic: anchored on the method
    /// *word* when the dispatch site recorded one (falling back to the
    /// command-head span), with a "did you mean…?" suggestion drawn from
    /// every method callable on the candidate classes — their MRO-resolved
    /// methods plus the implicit `TclOO` object builtins — and a replace
    /// fix targeting the same word.
    fn w308_diagnostic(
        &self,
        method: &str,
        cls_display: &str,
        class_names: &[&str],
        hierarchy: Option<&super::class_hierarchy::ClassHierarchy>,
        site: W308DiagnosticSite,
    ) -> super::types::Diagnostic {
        let reach = site.reach;
        // Candidate methods for the suggestion: MRO methods of every
        // candidate class, locally-declared methods (for classes the
        // hierarchy may not index), and the implicit object builtins.
        let mut candidates: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        for cls in class_names {
            if let Some(h) = hierarchy {
                candidates.extend(h.known_methods(self.registry.as_deref(), cls));
            }
            if let Some(cd) = self.result.all_classes.get(*cls) {
                candidates.extend(cd.methods.keys().cloned());
                candidates.extend(cd.class_methods.keys().cloned());
                if let Some(grammar) = self.class_definer_grammar(cls) {
                    candidates.extend(
                        grammar
                            .builtin_object_methods
                            .iter()
                            .filter(|builtin| {
                                grammar.builtin_object_method(builtin.name, reach).is_some()
                            })
                            .map(|builtin| builtin.name.to_string()),
                    );
                }
            }
        }
        let suggestions = crate::text::suggest_similar(
            method,
            candidates.iter().map(String::as_str),
            1,
            crate::text::scaled_max_distance(method),
        );
        let mut message = format!("Unknown method '{method}' on class '{cls_display}'");
        let mut fixes: Vec<super::types::CodeFix> = Vec::new();
        let span = site.method_span.unwrap_or(site.cmd_span);
        if let Some(best) = suggestions.first() {
            use std::fmt::Write as _;
            let _ = write!(message, "; did you mean '{best}'?");
            if let Some(fix_span) = site.method_span {
                fixes.push(super::types::CodeFix {
                    span: fix_span,
                    new_text: (*best).to_string(),
                    description: format!("Replace with '{best}'"),
                    // W308: an edit-distance guess at the intended method.
                    safety: crate::irules_checks::FixSafety::RequiresReview,
                });
            }
        }
        crate::analyser::types::Diagnostic::new(DiagCode::W308, span, message, Severity::Warning)
            .with_fixes(fixes)
    }

    /// **E001** (`TclOO` form) — `$obj` invoked with no method word at all.
    ///
    /// `TclOO`'s per-object command dispatcher requires a method name
    /// before it even attempts method resolution: `set o [C new]; $o`
    /// fails at run time with `wrong # args: should be "o method ?arg
    /// ...?"` regardless of whether the class declares an `unknown`
    /// handler, since the argument-count check runs before any method
    /// lookup — `unknown` is itself only reachable as the *result* of a
    /// failed lookup, and there is no name here to look up (confirmed
    /// against tclsh 9.0.4). This is the object-dispatch analogue of the
    /// registry ensemble's "missing subcommand" E001
    /// ([`Analyser::emit_arity_diagnostics`]) — same failure shape (a
    /// dispatcher invoked with no dispatch word), a different mechanism.
    ///
    /// Fires only when every candidate class is both locally known
    /// (`self.result.all_classes` — an external/unindexed class can't be
    /// vouched for) and a genuine `TclOO` metaclass
    /// (`DefinerFamily::TclOo` — `oo::class`, `oo::abstract`,
    /// `oo::configurable`, `oo::singleton`, …, resolved via the class's
    /// recorded `metaclass` and the registry's `definition_body`, never
    /// a hardcoded name). snit's generated dispatcher proc and `[incr
    /// Tcl]` are different mechanisms this analyser does not model
    /// precisely enough to make the same guarantee, so both abstain
    /// here — the same carve-out the with-method path already applies
    /// to snit (FP-OBJ-05).
    fn e001_for_bare_object_dispatch(
        &self,
        site: &crate::analyser::state::VarCommandSite,
        class_names: &HashSet<String>,
    ) -> Option<super::types::Diagnostic> {
        if site.argc != 0
            || class_names.is_empty()
            || self.site_in_child_interp(site.cmd_span.start())
        {
            return None;
        }
        let all_tcloo = class_names.iter().all(|cls| {
            self.result
                .all_classes
                .get(cls)
                .is_some_and(|cd| super::validity::is_tcloo_source_class(self, cd))
        });
        if !all_tcloo {
            return None;
        }
        Some(crate::analyser::types::Diagnostic::new(
            DiagCode::E001,
            site.cmd_span,
            format!("'{}' requires a method", site.var_name),
            Severity::Error,
        ))
    }

    /// **E001** (`TclOO` form) for a command-substitution head: a bare
    /// `[Dog new]` — or `[make]` where the lattice proves `make` an
    /// object-returning factory — invoked with no method word at all.
    ///
    /// Same failure and same gates as
    /// [`Self::e001_for_bare_object_dispatch`]: `TclOO`'s per-object
    /// dispatcher rejects a zero-word invocation before any method lookup
    /// (tclsh 9.0.4: `wrong # args: should be "::oo::Obj… method ?arg
    /// ...?"`, `-errorcode {TCL WRONGARGS}`), so an `unknown` handler
    /// cannot save it.  Fires only when the produced class is locally
    /// known **and** a genuine `TclOO` metaclass — snit / itcl dispatchers
    /// and external classes abstain, exactly as on the `$var` path.  The
    /// class identity comes from the type lattice / object-handle facts,
    /// never from a spelling match on the head.
    fn e001_for_bare_cmd_dispatch(
        &self,
        site: &crate::analyser::state::CmdCommandSite,
        class_name: Option<&str>,
    ) -> Option<super::types::Diagnostic> {
        if self.site_in_child_interp(site.cmd_span.start()) {
            return None;
        }
        let class_qn = self.canonicalise_class_name(class_name?)?;
        let is_tcloo = self
            .result
            .all_classes
            .get(&class_qn)
            .is_some_and(|cd| super::validity::is_tcloo_source_class(self, cd));
        if !is_tcloo {
            return None;
        }
        let inner = site.cmd_text.trim();
        let inner = inner
            .strip_prefix('[')
            .and_then(|w| w.strip_suffix(']'))
            .map_or(inner, str::trim);
        Some(crate::analyser::types::Diagnostic::new(
            DiagCode::E001,
            site.cmd_span,
            format!("'{inner}' requires a method"),
            Severity::Error,
        ))
    }

    /// Whether the dispatch site at `off` runs inside a **child
    /// interpreter's** evaluation body (the analyser's synthetic `@interp@…`
    /// scope domain).  The object classes the E001
    /// gates vouch for live in the *main* interpreter's command table; a
    /// child interpreter has its own, in which the class command does not
    /// exist at all (`interp create sub; interp eval sub {[Dog new]}` fails
    /// `invalid command name "Dog"` in the child — a different error the
    /// unresolved-command machinery owns), so the `TclOO` zero-word E001
    /// abstains there rather than assert main-interp object semantics.
    fn site_in_child_interp(&self, off: u32) -> bool {
        crate::analyser::scope::command_resolution_namespace_at(&self.result.global_scope, off)
            .contains("@interp@")
    }

    /// The source-owned allocation and current dispatch receipt. Aggregate
    /// type labels and bare instance-name assistance supply no runtime proof.
    fn live_instance_at_dispatch(
        &self,
        site: &crate::analyser::state::VarCommandSite,
    ) -> Option<&crate::command_binding::SourceObjectInstanceProof> {
        self.result
            .object_handle_facts
            .instance_in_scope(site.cmd_span.start(), &site.var_name)
    }

    /// Command lookup at the actual dispatch point, including namespace,
    /// imports, aliases, provider state and command allocation lifetime.
    fn command_slot_is_present(&self, value: &str, call_offset: u32) -> bool {
        self.head_identities
            .invocation_at_source("", call_offset)
            .lookup_command_word(value)
            .selected_slot_presence()
            == crate::command_binding::SourceCommandSlotPresence::Present
    }

    /// Positioned absence advice, retaining uncertainty about custom fallback.
    fn command_slot_has_absence_advice(&self, value: &str, call_offset: u32) -> bool {
        self.head_identities
            .invocation_at_source("", call_offset)
            .lookup_command_word(value)
            .selected_slot_diagnostic_presence()
            == crate::command_binding::SourceCommandSlotPresence::Absent
    }

    /// A formal's original caller operands are diagnostic candidates only.
    /// Unknown external calls remain open; this never changes runtime lookup,
    /// SSA parameter values or the independently selected receiver protocol.
    fn formal_caller_has_absence_advice(&self, offset: u32) -> bool {
        let Some(registry) = self.registry.as_deref() else {
            return false;
        };
        let binding = self.head_identities.invocation_at_source("", offset);
        let Some(site) = binding.invocation_site() else {
            return false;
        };
        let Some(candidates) = self
            .head_identities
            .source_bindings_ref()
            .declaration_formal_call_values(offset, registry)
        else {
            return false;
        };
        !candidates.is_empty()
            && candidates.iter().all(|candidate| {
                candidate
                    .value_in_source(&site.source)
                    .is_some_and(|value| self.command_slot_has_absence_advice(value, offset))
            })
    }

    /// Decide whether a `$var <method>` dispatch site is suppressed (no W307).
    ///
    /// Suppressed when SCCP proves the value is a known command; when an
    /// `in_method` dispatch lacks a proven non-command value; when the var is a
    /// proc parameter or is dispatched ≥2 times in scope (and not tainted /
    /// SCCP-non-command); when a namespaced-ensemble composition resolves; when
    /// the var is an object-factory local or a class instance member; or when
    /// it's a callback-array slot without SCCP non-command evidence.
    fn w307_site_suppressed(
        &self,
        site: &crate::analyser::state::VarCommandSite,
        ctx: &W307Ctx<'_>,
    ) -> bool {
        // Prefer the source owner's executed lookup before advisory value
        // candidates or the diagnostic's object-usage heuristics.
        if self
            .head_identities
            .invocation_at_source("", site.cmd_span.start())
            .selected_slot_presence()
            == crate::command_binding::SourceCommandSlotPresence::Present
        {
            return true;
        }
        // Resolve the dispatch's value first: prefer the exact SSA use-version,
        // falling back to the merged constset.  This drops the merged-set false
        // positive on a variable reassigned from a non-command to a known
        // command before the dispatch (`set c x; set c puts; $c ...`).
        let precise = w307_precise_cmd_values(
            ctx.func_ranges,
            site.cmd_span.start(),
            &site.var_name,
            self.registry.as_deref(),
        );
        let effective = precise
            .as_ref()
            .or_else(|| ctx.all_constsets.get(&site.var_name));
        // Method-local advice needs this declaration's exact original read;
        // aggregate values from another function cannot establish its value.
        if site.in_method && precise.as_ref().is_none_or(HashSet::is_empty) {
            return true;
        }
        let call_off = site.cmd_span.start();
        // SCCP concrete evidence the value IS a known command — suppress.
        if effective.is_some_and(|v| {
            !v.is_empty() && v.iter().all(|x| self.command_slot_is_present(x, call_off))
        }) {
            return true;
        }
        // Closed absence advice for every feasible literal overrides the
        // object-usage suppressions below. Initial autoload may still handle
        // the command; a custom fallback or incomplete lookup declines this
        // advice through the shared source owner.
        let sccp_absence_advice = effective.is_some_and(|v| {
            !v.is_empty()
                && v.iter()
                    .all(|x| self.command_slot_has_absence_advice(x, call_off))
        });
        // Proc-parameter / multi-dispatch object-dispatch suppression: a
        // dispatch on a parameter of the enclosing proc (any count), or on a
        // non-parameter local dispatched ≥2 times in the same scope, is
        // evidenced object usage — suppress unless the var is tainted.
        let idx = w307_enclosing_idx(ctx.proc_body_ranges, site.cmd_span.start());
        let encl_qname = idx.map_or(W307_TOP_SCOPE, |i| ctx.proc_body_ranges[i].2.as_str());
        let is_param = idx.is_some_and(|i| ctx.proc_body_ranges[i].3.contains(&site.var_name));
        let dispatch_count = ctx
            .dispatch_counts
            .get(&(encl_qname.to_owned(), site.var_name.clone()))
            .copied()
            .unwrap_or(0);
        let dispatcher_suppressed = is_param || dispatch_count >= 2;
        let tainted = ctx
            .tainted_by_scope
            .get(encl_qname)
            .is_some_and(|s| s.contains(&site.var_name));
        let original_caller_absence = is_param && self.formal_caller_has_absence_advice(call_off);
        if dispatcher_suppressed && !tainted && !sccp_absence_advice && !original_caller_absence {
            return true;
        }
        // Namespaced-ensemble dispatch: `${ns}::tail` / `$ns::tail` where `ns`
        // holds a namespace prefix and `::tail` composes a qualified command
        // path (tcllib's logger / dns / irc modules use this).  When the prefix
        // is an SCCP const and *every* composed name `<value>::tail` resolves to
        // a known command/proc/class, the dispatch is statically resolvable —
        // suppress.  A composition that resolves to nothing still fires.
        if let Some((prefix, tail)) =
            parse_namespaced_ensemble(&self.source, site.cmd_span, self.braced_var())
            && let Some(values) = ctx.all_constsets.get(&prefix)
            && !values.is_empty()
            && values
                .iter()
                .all(|v| self.command_slot_is_present(&format!("{v}::{tail}"), call_off))
        {
            return true;
        }
        // Object-factory provenance: `$var` holds a factory result in this scope
        // — a designed object handle, so the dispatch is not a static error.
        if ctx
            .factory_object_ranges
            .iter()
            .any(|(s, e, names)| *s <= call_off && call_off <= *e && names.contains(&site.var_name))
        {
            return true;
        }
        // Class instance-variable dispatch inside the class body (component /
        // sub-object) — W307 exemption.
        if ctx
            .snit_var_ranges
            .iter()
            .any(|(s, e, vars)| *s <= call_off && call_off <= *e && vars.contains(&site.var_name))
        {
            return true;
        }
        // Callback-registration array slot: `$state(-command)` /
        // `$state(doneCallback)` dispatches a command the user registered into a
        // switch-style option / callback slot. Unless SCCP has concrete evidence
        // the slot has absence advice (handled above via `sccp_absence_advice`,
        // e.g. `array set state {-command notACommand}` or
        // `set state(-command) notACommand`), treat it as a designed callback
        // dispatch (FP-OBJ-10).
        if !sccp_absence_advice && is_callback_array_slot(&site.var_name) {
            return true;
        }
        false
    }

    /// Nominal object results are assistance. Actual lookup identity and
    /// catalogue applicability come from the shared point owner.
    fn nominal_object_result(
        &self,
        word: &crate::ir::WordExpr,
        parent: Option<&crate::ir::CommandTokens>,
        registry: &tcl_registry::CommandRegistry,
    ) -> (bool, Option<String>) {
        let Some(commands) =
            crate::value_shapes::command_substitution_tokens(word, parent, self.lexer_config())
        else {
            return (false, None);
        };
        let Some(tokens) = commands.last() else {
            return (false, None);
        };
        self.nominal_tokens_object_result(tokens, registry)
    }

    fn nominal_tokens_object_result(
        &self,
        tokens: &crate::ir::CommandTokens,
        registry: &tcl_registry::CommandRegistry,
    ) -> (bool, Option<String>) {
        use crate::registry_invocation::registry_invocation_assistance_in_context;
        let context = self.analysis_context();
        let object_metadata = registry_invocation_assistance_in_context(&context, tokens)
            .is_some_and(|assistance| {
                assistance
                    .candidates
                    .iter()
                    .any(|shape| shape.nominal_return_type == Some(tcl_registry::TclType::Object))
            });
        let Some(binding) = &tokens.source_binding else {
            return (object_metadata, None);
        };
        let dispatcher_name = binding.nominal_definition_name_result(registry).is_some();
        let object_class = !binding.class_factory_candidates(registry).is_empty();
        let callee = {
            let candidates = binding.declared_procedure_result_candidates(tokens);
            candidates
                .first()
                .filter(|first| {
                    candidates.iter().all(|candidate| {
                        candidate.implementation_allocation == first.implementation_allocation
                            && candidate.command == first.command
                    })
                })
                .map(|target| target.command.clone())
        }
        .or_else(|| {
            let candidates = binding.declared_self_method_candidates(registry);
            let first = candidates.first()?;
            let class = first.declaring_class()?;
            candidates
                .iter()
                .all(|entry| entry.declaration() == first.declaration())
                .then(|| format!("{}::{}", class.command, first.name()))
        });
        (object_metadata || object_class || dispatcher_name, callee)
    }

    fn compute_factory_object_ranges(
        &self,
        cu: &crate::compilation_unit::CompilationUnit,
        registry: &tcl_registry::CommandRegistry,
    ) -> FactoryObjectAdvice {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        // Possible result advice uses the same source and actual availability
        // generation as the supplied unit, without a completed result grant.
        let Some(input) = self.result.resolved_input.as_ref() else {
            return FactoryObjectAdvice::default();
        };
        let Some(context) =
            crate::registry_invocation::retained_module_metadata_context(registry, &cu.ir_module)
        else {
            return FactoryObjectAdvice::default();
        };
        if cu.ir_module.source_metadata_input.as_ref() != Some(input)
            || cu.source != self.source
            || !self.result.matches_original_source_image(
                &tcl_lexer::SourceImage::document(&self.source),
                input.lexer_config(),
            )
        {
            return FactoryObjectAdvice::default();
        }
        // Original procedure and method units contribute possible result advice.
        let units: Vec<(&str, &crate::compilation_unit::FunctionUnit)> =
            std::iter::once(("::top", &cu.top_level))
                .chain(cu.procedures.iter().map(|(q, fu)| (q.as_str(), fu)))
                .chain(cu.methods.iter().map(|(q, fu)| (q.as_str(), fu)))
                .collect();

        // Per-proc: factory-local vars (non-user-proc factory heads), the last
        // returned var, and the `{var -> rhs command head}` assignment map.
        let mut maps = FactoryMaps::default();
        for (qname, fu) in &units {
            seed_factory_maps(
                qname,
                fu,
                &context,
                &|word, parent| self.nominal_object_result(word, parent, registry),
                &mut maps,
                self.lexer_config(),
            );
        }
        let FactoryMaps {
            mut factory_locals,
            assigns,
            return_var,
            mut object_returning,
            returned_callees,
        } = maps;
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

        propagate_object_returning(
            &return_var,
            &returned_callees,
            &assigns,
            &bare_to_qnames,
            &mut object_returning,
            &mut factory_locals,
        );

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
        FactoryObjectAdvice {
            ranges,
            object_returning,
        }
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
    pub(super) fn emit_var_command_diagnostics(
        &mut self,
        cu: &crate::compilation_unit::CompilationUnit,
        registry: &tcl_registry::CommandRegistry,
    ) {
        if self.var_command_sites.is_empty() && self.cmd_command_sites.is_empty() {
            return;
        }
        // Proc-name literals held as dispatch-table values
        // (`array set` pairs, `dict create`/`dict set` values, `set arr(k) v`)
        // become command references when the table is consumed by one of the
        // dispatch sites this pass walks — so find-references reaches the
        // table entry, and a rename rewrites it alongside the proc (keeping
        // the dispatch working).
        self.emit_dispatch_table_command_references(cu);
        // Build the class hierarchy once for W308 method
        // resolution (uses the ``ClassHierarchy``).
        let hierarchy = if self.result.all_classes.is_empty() {
            None
        } else {
            Some(super::class_hierarchy::build_class_hierarchy(
                self.result.all_classes.clone(),
            ))
        };

        // Aggregate constant-string knowledge (var name → flat CONST/CONSTSET
        // value set) across every function in the CompilationUnit.
        let all_constsets = aggregate_constsets(cu, &self.analysis_context());

        // Per-SSA-version refinement: map each
        // function to its source range + FunctionUnit so the W307
        // suppression can read the value at the dispatch's *exact* SSA
        // use-version instead of the merged set.  ``::top`` covers the
        // whole source; each original procedure or method owns its narrower
        // declaration range and its own symbolic value facts.
        let mut func_ranges = vec![W307FunctionRange {
            function: &cu.top_level,
            start: 0,
            end: u32::MAX,
        }];
        for (qname, fu) in &cu.procedures {
            if let Some(ir_proc) = cu.ir_module.procedures.get(qname) {
                func_ranges.push(W307FunctionRange {
                    function: fu,
                    start: ir_proc.span.start(),
                    end: ir_proc.span.end(),
                });
            }
        }
        for (qname, fu) in &cu.methods {
            if let Some(span) = cu
                .ir_module
                .methods
                .get(qname)
                .and_then(|method| method.span)
            {
                func_ranges.push(W307FunctionRange {
                    function: fu,
                    start: span.start(),
                    end: span.end(),
                });
            }
        }

        // Drain sites so we can borrow self.result mutably below.
        let sites = std::mem::take(&mut self.var_command_sites);
        // Object-factory locals: vars holding a factory result (`set x [Class
        // new]` / `set x [::ns::factory]` / `set x [object_returning_proc]`).
        // A `$x method` dispatch on one suppresses W307 (designed object usage).
        let FactoryObjectAdvice {
            ranges: factory_object_ranges,
            object_returning,
        } = self.compute_factory_object_ranges(cu, registry);
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
        let mut dispatch_counts: FxHashMap<(String, String), usize> = FxHashMap::default();
        for site in &sites {
            let idx = w307_enclosing_idx(&proc_body_ranges, site.cmd_span.start());
            let qname = idx.map_or(W307_TOP_SCOPE, |i| proc_body_ranges[i].2.as_str());
            *dispatch_counts
                .entry((qname.to_owned(), site.var_name.clone()))
                .or_insert(0) += 1;
        }
        // Per-scope tainted var names — any tainted SSA version of a name
        // disqualifies it from dispatcher-suppression.
        let tainted_by_scope = build_tainted_by_scope(cu);

        let w307_ctx = W307Ctx {
            all_constsets: &all_constsets,
            func_ranges: &func_ranges,
            factory_object_ranges: &factory_object_ranges,
            snit_var_ranges: &snit_var_ranges,
            proc_body_ranges: &proc_body_ranges,
            dispatch_counts: &dispatch_counts,
            tainted_by_scope: &tainted_by_scope,
        };

        // Collected rather than pushed in the loop: `w307_ctx` borrows out of
        // `self.result`, so the per-site decision has to run against `&self`.
        let emitted: Vec<super::types::Diagnostic> = sites
            .iter()
            .filter_map(|site| self.diagnose_var_command_site(site, hierarchy.as_ref(), &w307_ctx))
            .collect();
        self.result.diagnostics.extend(emitted);
        // Restore the sites list — snapshot/restore expects it
        // to round-trip independently of emission.
        self.var_command_sites = sites;

        // ``[cmd] method`` sites — W307/W308 on command-substitution heads.
        self.emit_cmd_command_diagnostics(registry, hierarchy.as_ref(), &object_returning);
    }

    /// The diagnostic one recorded dispatch site draws, if any — the body of
    /// [`Self::emit_var_command_diagnostics`]'s per-site loop, split out so
    /// the three mutually exclusive paths a site can take are visible as
    /// three arms rather than buried in a page of setup.
    ///
    /// Exactly one arm claims each site:
    ///
    /// * **self-dispatch** (`my <method>`) — the receiver is the
    ///   enclosing object, so it is neither a variable nor a resolvable
    ///   command name and W307's "non-literal command name" question never
    ///   arises. This arm always consumes the site;
    /// * **W308** — a current instance receipt has a closed method-name
    ///   inventory, so an absent literal method may be diagnosed;
    /// * **W307** — no usable object type, and the head could not be proved
    ///   to reach a finite set of known command names.
    fn diagnose_var_command_site(
        &self,
        site: &crate::analyser::state::VarCommandSite,
        hierarchy: Option<&super::class_hierarchy::ClassHierarchy>,
        w307_ctx: &W307Ctx<'_>,
    ) -> Option<super::types::Diagnostic> {
        if site.receiver == crate::analyser::state::DispatchReceiver::SelfDispatch {
            return self.w308_for_self_dispatch(site, hierarchy);
        }

        if let Some(instance) = self.live_instance_at_dispatch(site) {
            return self.w308_for_object_var(site, instance);
        }

        (!self.w307_site_suppressed(site, w307_ctx)).then(|| {
            crate::analyser::types::Diagnostic::new(
                DiagCode::W307,
                site.cmd_span,
                "Non-literal command name — cannot statically analyse".to_string(),
                Severity::Warning,
            )
        })
    }

    /// Whether `site` is a `[cmd]::method` namespaced-ensemble dispatch
    /// (FP-OBJ-07): a command-substitution head composed with a literal
    /// `::method` tail.
    ///
    /// The literal tail is static method-name evidence — the dispatch is
    /// well-formed, with only the namespace prefix computed at run time — so
    /// W307 must not fire. A bare `[cmd] arg` dispatch with no `::method`
    /// tail has no such evidence and still fires.
    fn is_namespaced_ensemble_dispatch(
        &self,
        site: &crate::analyser::state::CmdCommandSite,
    ) -> bool {
        let start = site.cmd_span.start() as usize;
        let end = (site.cmd_span.end() as usize).min(self.source.len());
        let Some(word) = self.source.get(start..end) else {
            return false;
        };
        if !word.starts_with('[') {
            return false;
        }
        let Some(sep) = word.find("]::") else {
            return false;
        };
        let tail = &word[sep + 3..];
        !tail.is_empty()
            && tail
                .chars()
                .all(|c| c.is_alphanumeric() || c == '_' || c == ':')
    }

    /// Emit W307/W308 for `[cmd] method` command-substitution dispatch sites.
    ///
    /// W307 fires only when the inner command's return type is unknown AND the
    /// call isn't an OO self-dispatch (`my` / `self`).  When the return type is
    /// a known class, the method is validated against the hierarchy and W308 is
    /// emitted instead.  Restores `cmd_command_sites` on exit.
    #[allow(
        clippy::too_many_lines,
        reason = "the diagnostic's closely coupled dispatch and type-validation cases share one restored-site lifecycle"
    )]
    fn emit_cmd_command_diagnostics(
        &mut self,
        registry: &tcl_registry::CommandRegistry,
        hierarchy: Option<&super::class_hierarchy::ClassHierarchy>,
        object_returning: &FxHashSet<String>,
    ) {
        let mut cmd_sites = std::mem::take(&mut self.cmd_command_sites);
        // A per-item body initially owns an isolated command world. Its
        // carriers have since been rebased into this document, but rebasing
        // cannot establish the surrounding class, alias, or mutation state.
        // Restore exact original-site proofs through the shared source owner
        // before either analysis strategy classifies a result. Missing sites
        // remain explicitly unknown; no catalogue or spelling fallback is used.
        let bindings = self.head_identities.source_bindings();
        for site in &mut cmd_sites {
            for tokens in &mut site.commands {
                bindings.stamp_original_tokens(tokens);
            }
        }
        // The walk below takes `&mut self`, so this analysis's own registry
        // is held as a handle rather than re-borrowed from `self` per site.
        let context = self.analysis_context();
        let attached = self.registry.clone();
        let attached = attached.as_deref();
        for site in &cmd_sites {
            if self.is_namespaced_ensemble_dispatch(site) {
                continue;
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
            let Some(tokens) = site.commands.first().filter(|_| site.commands.len() == 1) else {
                continue;
            };
            let effective = crate::registry_invocation::effective_command_words(tokens);
            let words = effective
                .as_ref()
                .map_or(&[][..], |effective| effective.words.as_slice());
            let spellings = words
                .iter()
                .map(crate::ir::WordExpr::legacy_text)
                .collect::<Vec<_>>();
            let head = spellings.first().map_or("", String::as_str);
            let arg_strs = spellings
                .iter()
                .skip(1)
                .map(String::as_str)
                .collect::<Vec<_>>();
            let invocation =
                crate::registry_invocation::resolved_tokens_invocation_in_context(&context, tokens);

            // OO self-dispatch (`my <method>` / `self <method>`): by default
            // the return is treated as an object handle (suppress).  But when
            // the dispatched method resolves in the enclosing class and its
            // body is a simple `return <literal>`, the result is a plain
            // string, not an object — so the *outer* dispatch fires W307.
            // `my <method>` (self-dispatch) and `self <subcommand>`
            // (introspection) both return something the analyser treats as an
            // object handle by default. Both kinds come from the registry;
            // `next`/`nextto` are deliberately *not* included — they return
            // the next implementation's result, not a handle.
            if invocation
                .as_ref()
                .and_then(|invocation| {
                    attached.and_then(|registry| {
                        registry.method_dispatch_keyword(&invocation.facts.canonical_command)
                    })
                })
                .is_some_and(|kind| {
                    matches!(
                        kind,
                        tcl_registry::MethodDispatchKind::SelfDispatch
                            | tcl_registry::MethodDispatchKind::Introspection
                    )
                })
            {
                let returns_literal = arg_strs.first().is_some_and(|method| {
                    self.oo_self_method_returns_literal(site.cmd_span.start(), method)
                });
                if returns_literal {
                    self.result
                        .diagnostics
                        .push(crate::analyser::types::Diagnostic::new(
                            DiagCode::W307,
                            site.cmd_span,
                            "Non-literal command name — cannot statically analyse".to_string(),
                            Severity::Warning,
                        ));
                    continue;
                }
                // `[self]` / `[self object]` is not just *some* self-dispatch
                // or introspection call whose return type happens to be
                // unknowable — the registry (`is_self_receiver_call`)
                // says this exact head/arg pair denotes the *current*
                // receiver, the same target `my <method>` dispatches on. So
                // the outer method word is validated against the enclosing
                // class (W308) instead of falling through as an opaque
                // object handle of unresolvable class.
                if attached
                    .is_some_and(|r| r.is_self_receiver_call(head, arg_strs.first().copied()))
                    && let Some(diag) = self.w308_for_self_receiver(site, hierarchy)
                {
                    self.result.diagnostics.push(diag);
                }
                continue;
            }

            let ret_type = self.cmd_head_return_type(tokens, registry);

            // ``Object`` return type — suppress W307; if the
            // class is known, validate the method (W308), and a dispatch
            // with *no* method word at all is the unconditional `TclOO`
            // "wrong # args" failure (E001).
            let is_object = ret_type.kind() == crate::types::TypeKind::Known
                && matches!(ret_type.tcl_type(), Some(tcl_registry::TclType::Object));
            if is_object {
                if site.method_name.is_none() {
                    if let Some(diag) = self.e001_for_bare_cmd_dispatch(site, ret_type.class_name())
                    {
                        self.result.diagnostics.push(diag);
                    }
                    continue;
                }
                if !self.disabled_diagnostics.contains("W308")
                    && let (Some(method), Some(class_name)) =
                        (site.method_name.as_ref(), ret_type.class_name().as_ref())
                {
                    let Some(cls_qn) = self.canonicalise_class_name(class_name) else {
                        continue;
                    };
                    let cd = self.result.all_classes.get(&cls_qn).cloned();
                    // `[Dog new] m` dispatches through the produced object's
                    // own command, so only its exported surface is reachable.
                    let method_ok = self.validate_method_on_class(
                        &cls_qn,
                        method,
                        cd.as_ref(),
                        hierarchy,
                        tcl_registry::definer::MethodReach::ObjectCommand,
                    );
                    if !method_ok && Self::method_word_is_literal(method) {
                        let diag = self.w308_diagnostic(
                            method,
                            class_name,
                            &[cls_qn.as_str()],
                            hierarchy,
                            W308DiagnosticSite {
                                reach: tcl_registry::definer::MethodReach::ObjectCommand,
                                method_span: site.method_span,
                                cmd_span: site.cmd_span,
                            },
                        );
                        self.result.diagnostics.push(diag);
                    }
                }
                continue;
            }

            // Nominal candidate metadata can suppress a type warning; it
            // cannot establish an object class or validate an outer method.
            let (object, callee) = self.nominal_tokens_object_result(tokens, registry);
            if object || callee.is_some_and(|callee| object_returning.contains(&callee)) {
                continue;
            }

            // Type is unknown — emit W307 (only the emit-half
            // for the residual unknown-type case).
            self.result
                .diagnostics
                .push(crate::analyser::types::Diagnostic::new(
                    DiagCode::W307,
                    site.cmd_span,
                    "Non-literal command name — cannot statically analyse".to_string(),
                    Severity::Warning,
                ));
        }
        self.cmd_command_sites = cmd_sites;
    }

    /// W308 for a `[self]` / `[self object]` dispatch head confirmed by
    /// [`tcl_registry::CommandRegistry::is_self_receiver_call`] — the
    /// receiver is the *enclosing* class, found via
    /// [`Self::enclosing_class_at_offset`], exactly as a bareword `my
    /// <method>` dispatch's target is.
    ///
    /// The substitution yields the object's own **command**, so it reaches
    /// only exported methods —
    /// [`MethodReach::ObjectCommand`](tcl_registry::definer::MethodReach::ObjectCommand),
    /// not `SelfDispatch`. tclsh 9.0.4 and 8.6.16 agree: `[self] varname v`
    /// fails with `unknown method "varname"` where `my varname v` succeeds.
    fn w308_for_self_receiver(
        &self,
        site: &crate::analyser::state::CmdCommandSite,
        hierarchy: Option<&super::class_hierarchy::ClassHierarchy>,
    ) -> Option<super::types::Diagnostic> {
        self.w308_for_enclosing_receiver(
            site.method_name.as_deref()?,
            site.method_span,
            site.cmd_span,
            tcl_registry::definer::MethodReach::ObjectCommand,
            hierarchy,
        )
    }

    /// W308 for a bareword **self-dispatch keyword** head — `my <method>`,
    /// the commonest same-object spelling in `TclOO`.
    ///
    /// The receiver is the same enclosing object `[self]` names, so the
    /// class lookup is shared with [`Self::w308_for_self_receiver`]; what
    /// differs is the reach. `my` bypasses export filtering, so it can call
    /// `oo::object`'s unexported members — which is why `my variable v` and
    /// `my varname v`, both idiomatic, must stay silent while `$obj
    /// variable v` does not.
    fn w308_for_self_dispatch(
        &self,
        site: &crate::analyser::state::VarCommandSite,
        hierarchy: Option<&super::class_hierarchy::ClassHierarchy>,
    ) -> Option<super::types::Diagnostic> {
        if self.self_dispatch_keyword_disturbed(&site.var_name) {
            return None;
        }
        self.w308_for_enclosing_receiver(
            site.method_name.as_deref()?,
            site.method_span,
            site.cmd_span,
            tcl_registry::definer::MethodReach::SelfDispatch,
            hierarchy,
        )
    }

    /// Whether this document renames, aliases, deletes or shadows the
    /// self-dispatch keyword `head` anywhere — in which case a bare `my`
    /// may not be `TclOO`'s dispatcher at all and every conclusion about
    /// the word after it is unfounded, so W308 abstains.
    ///
    /// `rename my mine`, `interp alias {} my {} …`, `rename my {}`, and a
    /// user `proc my {…}` all qualify. Asked here rather than in the walker
    /// because it is **order-independent by design**: a `rename` written
    /// *after* the method body still governs, since the body only runs
    /// later, and a `proc my` in a deferred fragment is invisible to the
    /// walk that records the site. Post-walk, every table is complete and
    /// the answer is the same on the whole-file and per-item paths.
    ///
    /// Deliberately coarse — "disturbed anywhere" rather than "disturbed
    /// before this offset". Modelling which redefinition wins at each
    /// offset would buy back warnings only in files that redefine a core
    /// `TclOO` keyword, and would risk a confident wrong answer in exactly
    /// the files least able to afford one.
    ///
    /// Both spellings are checked, since each table keys bare or
    /// `::`-qualified depending on how the disturbing command was written.
    fn self_dispatch_keyword_disturbed(&self, head: &str) -> bool {
        let bare = head.trim_start_matches("::");
        let qualified = format!("::{bare}");
        [bare, qualified.as_str()].iter().any(|name| {
            self.command_aliases.contains_key(*name)
                || self.renamed_commands.contains_key(*name)
                || self.renamed_commands.values().any(|old| old == *name)
                || self.deleted_commands.contains_key(*name)
                || self.result.all_procs.contains_key(*name)
                || self.result.all_classes.contains_key(*name)
        })
    }

    /// The shared body of the two "the receiver is the object whose method
    /// body encloses this call" W308 checks — `[self] <method>` and bareword
    /// `my <method>`.
    ///
    /// Unlike the `Object`-return-type path, the class here is never
    /// *inferred* from a type lattice: the registry says the head denotes
    /// the current object outright, so the only question is which class's
    /// body the offset falls in.
    ///
    /// # What abstains, and why that is the right answer
    ///
    /// * **No enclosing class** — a top-level `my`, a `my` inside a plain
    ///   `proc`, or one inside an `oo::define` / `oo::objdefine` body whose
    ///   members are not folded into a recorded `ClassDef`. There is no
    ///   receiver to check against, and guessing at one would invent errors
    ///   in code that runs fine.
    /// * **A non-literal method word** (`my $action`, `my get$suffix`) —
    ///   the name dispatched is computed at run time, so no static method
    ///   set can contradict it ([`Self::method_word_is_literal`]).
    /// * **An unprovable method set** — a `mixin` or `superclass` outside
    ///   the local index, a class manufactured by an unreadable metaclass,
    ///   or a body that installs members reflectively. Handled inside
    ///   [`Self::validate_method_on_class`] via
    ///   [`Self::method_set_unknowable`], so a dynamically-extended class
    ///   never draws this warning.
    /// * **An `unknown` handler** anywhere in the MRO — every unresolved
    ///   name is then legal by construction.
    /// * **A template method, on `my` dispatch only** — the method resolves
    ///   nowhere on the enclosing class's MRO but a known subclass defines
    ///   it ([`ClassHierarchy::subclass_provides_method`](super::class_hierarchy::ClassHierarchy::subclass_provides_method)).
    ///   `my` late-binds on the actual receiver — always a
    ///   subclass instance when the base is abstract — and reaches
    ///   unexported members, so the call is the deliberate pattern, not a
    ///   typo.  `[self] M` keeps the warning: it dispatches through the
    ///   object's command, where an unexported subclass method really is
    ///   unreachable.
    ///
    /// A `forward`ed method needs no carve-out: `forward NAME …` records a
    /// real member under `NAME`, so `my NAME` resolves through the ordinary
    /// member tables. Nor do inherited methods, which the MRO lookup finds.
    ///
    /// `None` when the method resolves, when any of the above abstains, or
    /// when the diagnostic is disabled.
    fn w308_for_enclosing_receiver(
        &self,
        method: &str,
        method_span: Option<tcl_lexer::Span>,
        cmd_span: tcl_lexer::Span,
        reach: tcl_registry::definer::MethodReach,
        hierarchy: Option<&super::class_hierarchy::ClassHierarchy>,
    ) -> Option<super::types::Diagnostic> {
        if self.disabled_diagnostics.contains("W308") {
            return None;
        }
        if !Self::method_word_is_literal(method) {
            return None;
        }
        if self.site_in_child_interp(cmd_span.start()) {
            return None;
        }
        let class_def = self.enclosing_class_at_offset(cmd_span.start())?;
        let cls_qn = class_def.qualified_name.as_str();
        if self.validate_method_on_class(cls_qn, method, Some(class_def), hierarchy, reach) {
            return None;
        }
        // Template-method pattern: a base-class body calling
        // `my M` where `M` is written only by subclasses runs fine — `my`
        // late-binds on the actual receiver, always a subclass instance,
        // and bypasses export filtering.  A known defining subclass is the
        // evidence; with none anywhere in the index the warning stands.
        // `SelfDispatch` only: `[self] M` goes through the object's own
        // command, where an unexported subclass method really is
        // unreachable (tclsh 9.0.4: `[self] varname v` fails where
        // `my varname v` succeeds — same reach split as the doc above).
        if reach == tcl_registry::definer::MethodReach::SelfDispatch
            && (hierarchy.is_some_and(|h| h.subclass_provides_method(cls_qn, method))
                || self.workspace_subclass_methods.as_ref().is_some_and(|m| {
                    m.get(cls_qn)
                        .is_some_and(|provided| provided.contains(method))
                }))
        {
            return None;
        }
        Some(self.w308_diagnostic(
            method,
            cls_qn,
            &[cls_qn],
            hierarchy,
            W308DiagnosticSite {
                reach,
                method_span,
                cmd_span,
            },
        ))
    }

    /// Whether a recorded method word is a **literal** name the static
    /// method set can actually contradict.
    ///
    /// A word carrying a variable or command substitution (`$action`,
    /// `get$suffix`, `[pick]`) or a `{*}` expansion names something chosen
    /// at run time; "unknown method" is not a claim any static table is
    /// entitled to make about it. The recorded text is the reconstructed
    /// word, in which the walker re-brackets a substitution (`${x}` /
    /// `[…]`), so testing for the introducers covers both the written and
    /// the reconstructed spelling.
    ///
    /// Shared by every W308 emitter so a fix on one path cannot leave
    /// another false-positiving on the same input.
    fn method_word_is_literal(method: &str) -> bool {
        !method.is_empty() && !method.contains(['$', '[']) && !method.contains("{*}")
    }

    /// Result type selected from the retained executed implementation.
    /// Concrete class results require construction closure or the actual
    /// procedure's return proof; unresolved results stay unknown.
    fn cmd_head_return_type(
        &self,
        tokens: &crate::ir::CommandTokens,
        registry: &tcl_registry::CommandRegistry,
    ) -> crate::types::TypeLattice {
        if let Some(binding) = &tokens.source_binding {
            if let Some(class) = binding.proved_construction_result(registry) {
                return crate::types::TypeLattice::object_of(class);
            }
            if let Some(target) = binding
                .proved_execution_target()
                .filter(|target| target.kind == crate::command_binding::BindingKind::Proc)
                && let Some(result) = self
                    .result
                    .object_handle_facts
                    .normal_procedure_result(target)
            {
                return result.clone();
            }
        }
        crate::registry_invocation::normal_representation_invocation_in_context(
            &self.analysis_context(),
            tokens,
        )
        .and_then(|invocation| invocation.result_representation_type())
        .map_or_else(
            crate::types::TypeLattice::overdefined,
            crate::types::TypeLattice::of,
        )
    }

    /// W250 — instantiating an `oo::abstract` class.
    ///
    /// `TclOO`'s `oo::abstract` removes `new` / `create` from the class, so
    /// `AbstractClass new` / `AbstractClass create obj` is a runtime error.
    /// Uses the retained live class incarnation and its original factory,
    /// including alias prefixes and nested invocations in generic calls.
    /// Factory metadata supplies the diagnostic grammar independently of
    /// constructor completion and native opcode eligibility.
    pub(super) fn emit_abstract_instantiation_diagnostics(
        &mut self,
        cu: &crate::compilation_unit::CompilationUnit,
    ) {
        if self.disabled_diagnostics.contains("W250") {
            return;
        }
        let Some(registry) = self.registry.as_deref() else {
            return;
        };
        let diagnostic = |tokens: &crate::ir::CommandTokens, span: tcl_lexer::Span| {
            let binding = tokens.source_binding.as_ref()?;
            let (factory, target) = binding.proved_class_definition_factory()?;
            let query = binding
                .variable_context
                .invocation_dialect?
                .authoring_query()?
                .with_realm(binding.invocation_realm()?);
            let spec = registry.get_for_surface(factory, Some(query))?;
            if !spec
                .traits
                .contains(tcl_registry::Traits::ABSTRACT_CLASS_FACTORY)
            {
                return None;
            }
            let member = if let Some(prefix) = target.prepended.first() {
                prefix.as_registry_word().literal()?
            } else {
                binding.evaluated_argument_values.first()?.as_deref()?
            };
            spec.definition_body?.manufacturer(member)?;
            Some(crate::analyser::types::Diagnostic::new(
                DiagCode::W250,
                span,
                format!(
                    "Instantiating abstract class '{}' — use a concrete subclass",
                    target.command
                ),
                super::types::Severity::Warning,
            ))
        };
        let mut diags = Vec::new();
        for fu in cu.analysable_body_function_units() {
            for statement in fu.cfg.blocks.values().flat_map(|block| &block.statements) {
                let Some(tokens) = statement.tokens() else {
                    continue;
                };
                if let Some(diag) = diagnostic(tokens, fu.abs_span(statement.span())) {
                    diags.push(diag);
                }
                // A normal assignment can remain a generic call. Lift its
                // original evaluated words through the shared substitution
                // owner; no string reparse or AssignValue-only branch can
                // preserve all nested invocation identities.
                for call in crate::word_subst::lifted_calls(Some(tokens), cu.ir_module.lexer_config)
                {
                    if let Some(tokens) = call.tokens
                        && let Some(diag) = diagnostic(&tokens, fu.abs_span(call.span))
                        && !diags.contains(&diag)
                    {
                        diags.push(diag);
                    }
                }
            }
        }

        self.result.diagnostics.extend(diags);
    }

    /// Emit a command reference for each proc-name **literal**
    /// held as a dispatch-table value, provided the table is actually
    /// *consumed* by a `$table(...)` / `[dict get $table …]` dispatch site
    /// this pass walks (the W307 shapes).  The reference anchors at the
    /// literal value token itself — the span **is** the written command name,
    /// so rename may rewrite the table entry, keeping the dispatch alive.
    /// Values with no recoverable span (pure SCCP folds, e.g. through
    /// `string map`) abstain, as does an unconsumed table (a config array
    /// whose values merely look like command names must not gain phantom
    /// references).
    fn emit_dispatch_table_command_references(
        &mut self,
        cu: &crate::compilation_unit::CompilationUnit,
    ) {
        let consumed = self
            .var_command_sites
            .iter()
            .filter(|site| site.receiver != crate::analyser::state::DispatchReceiver::SelfDispatch)
            .map(|site| site.cmd_span.start())
            .chain(
                self.cmd_command_sites
                    .iter()
                    .map(|site| site.cmd_span.start()),
            )
            .collect::<HashSet<_>>();
        #[cfg(test)]
        if std::env::var_os("TCL_TABLE_PROOF_DEBUG").is_some() {
            eprintln!("table consumption sites={consumed:?}");
        }
        let Some(input) = self.result.resolved_input.as_ref() else {
            return;
        };
        let harvested = harvest_table_command_value_spans(
            cu,
            &self.source,
            &input.context_registry(),
            input.lexer_config(),
            &consumed,
        );
        for (value, span, reference) in harvested {
            if !reference.is_user_command()
                || self.result.command_invocations.iter().any(|existing| {
                    existing.range == span
                        && existing.resolved_command_reference.as_ref() == Some(&reference)
                        && existing.resolved_user_definition == reference.is_direct_definition()
                })
            {
                continue;
            }
            let mut invocation = crate::signature_scan::types::SignatureCommandInvocation::written(
                value, span, None,
            );
            invocation.retain_reference(&reference);
            self.result.command_invocations.push(invocation);
        }
    }

    /// Resolve a possibly-bare class name to its fully-qualified form keyed
    /// in `result.all_classes` — the shared call-site resolver
    /// ([`super::class_hierarchy::resolve_written_class_name`]), so this
    /// keying can never diverge from the LSP's. Missing source identity withdraws.
    fn canonicalise_class_name(&self, name: &str) -> Option<String> {
        super::class_hierarchy::resolve_written_class_name(name, &self.result.all_classes)
    }

    /// Decide whether `method` is callable on `class_name` through a
    /// dispatch of the given `reach`, consulting the class hierarchy, the
    /// class's local method tables, and the class system's own built-in
    /// object methods.
    ///
    /// A method is OK when the class's MRO produces a concrete provider, or
    /// the class is external (no local `ClassDef`), or the method is one the
    /// *class system itself* supplies (see
    /// [`Self::builtin_object_method_reachable`]), or the class declares an
    /// `unknown` method, or the class extends or mixes in an external class
    /// we can't introspect.
    ///
    /// `reach` matters only for the built-in set, and it is the difference
    /// between a true positive and a false one: `my variable v` reaches
    /// `oo::object`'s unexported `variable`, while `$obj variable v` and
    /// `[self] variable v` do not and really are errors.
    fn validate_method_on_class(
        &self,
        class_name: &str,
        method: &str,
        cd: Option<&super::types::ClassDef>,
        hierarchy: Option<&super::class_hierarchy::ClassHierarchy>,
        reach: tcl_registry::definer::MethodReach,
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
        if self.builtin_object_method_reachable(cd, method, reach) {
            return true;
        }
        // A method the class system generates from declared properties —
        // written by no `method` body, so neither the member tables nor the
        // MRO above can see it.
        if hierarchy
            .is_some_and(|h| h.is_property_accessor(self.registry.as_deref(), class_name, method))
        {
            return true;
        }
        if let Some(fallback) = self
            .class_definer_grammar(class_name)
            .and_then(|grammar| grammar.unknown_dispatch_method)
        {
            if cd.methods.contains_key(fallback) {
                return true;
            }
            if hierarchy.is_some_and(|h| h.method_target(class_name, fallback).is_some()) {
                return true;
            }
        }
        if self
            .class_definer_grammar(class_name)
            .is_some_and(|grammar| grammar.dynamic_method_dispatch)
        {
            return true;
        }
        // Unknowable method set (external base, opaque inheritance, or
        // reflectively-installed members) ⇒ skip W308.
        self.method_set_unknowable(cd)
    }

    /// Whether `method` is one `cd`'s **class system** gives every object,
    /// reachable by a dispatch of this `reach`.
    ///
    /// Registry data, not a name list here: the class's recorded metaclass
    /// is itself a definer command, whose
    /// [`DefinitionBodyGrammar`](tcl_registry::definer::DefinitionBodyGrammar)
    /// carries `builtin_object_methods` — so `TclOO` gets `oo::object`'s
    /// inherited surface, snit gets `configure` / `cget` / `info`, itcl gets
    /// its own, and a class system the registry gains later is covered
    /// without touching this function.
    ///
    /// # Deliberate abstentions
    ///
    /// * **Receiver kind is not filtered.** `new` / `create` are declared
    ///   [`BuiltinMethodReceiver::ClassObject`](tcl_registry::definer::BuiltinMethodReceiver::ClassObject)
    ///   and really are absent from an instance (tclsh 9.0.4: `[C new] new`
    ///   → `unknown method "new"`), but a receiver here may equally be a
    ///   *class* command held in a variable (`set cls C; $cls new`), which
    ///   the analyser cannot always tell apart from one of its instances.
    ///   Accepting both is the abstention; narrowing it needs receiver-kind
    ///   facts this pass does not have.
    /// * **`oo::configurable`'s `configure` is not in the shared `TclOO`
    ///   grammar**, because it is contributed by a specific metaclass rather
    ///   than by `oo::object` — every `TclOO` metaclass shares the one
    ///   grammar, so listing it here would hand it to plain `oo::class`
    ///   instances too, which really do fail with `unknown method
    ///   "configure"`. It rides on `oo::configurable`'s own
    ///   `TCLOO_CONFIGURABLE_GRAMMAR` instead, and is settled one level up in
    ///   [`Self::validate_method_on_class`] via
    ///   [`ClassHierarchy::is_property_accessor`](super::class_hierarchy::ClassHierarchy::is_property_accessor),
    ///   which asks the *metaclass command's* registry traits; a class that
    ///   inherits from an *unindexed* configurable base is covered by
    ///   [`Self::method_set_unknowable`].
    fn builtin_object_method_reachable(
        &self,
        cd: &super::types::ClassDef,
        method: &str,
        reach: tcl_registry::definer::MethodReach,
    ) -> bool {
        self.class_definer_grammar(&cd.qualified_name)
            .is_some_and(|grammar| grammar.builtin_object_method(method, reach).is_some())
    }

    /// True when `cd`'s callable method set cannot be enumerated from what
    /// the analyser recorded, so W308 must abstain rather than call a method
    /// missing.  Three independent reasons, all "the tables are a lower
    /// bound", never "this method does not exist":
    ///
    /// * a superclass **or mixin** outside the local class index (the
    ///   `TclOO` bases in [`OO_BASE`] excepted) — `TclOO` mixins contribute
    ///   to the MRO exactly as superclasses do, so an unresolvable name in
    ///   either list hides an unknown set of inherited methods;
    /// * [`ClassDef::inheritance_unknown`] — manufactured by a user
    ///   metaclass whose `create` override could not be read, so the
    ///   spliced superclass list itself is unknown;
    /// * [`ClassDef::member_set_incomplete`] — the class's own body installs
    ///   members reflectively.
    ///
    /// Used by the class-definition assistance path; source-owned instance
    /// receipts retain their independent method-inventory completeness.
    ///
    /// [`ClassDef::inheritance_unknown`]: super::types::ClassDef::inheritance_unknown
    /// [`ClassDef::member_set_incomplete`]: super::types::ClassDef::member_set_incomplete
    fn method_set_unknowable(&self, cd: &super::types::ClassDef) -> bool {
        // A class manufactured by a user-defined metaclass whose `create`
        // override could not be read has an unknown superclass list, which is
        // the same situation as a superclass outside the index: a method it
        // inherits is not a method it is missing.
        //
        // A class whose own body installs members through reflection
        // (`constructor {*}[info class constructor ::Base]`, `foreach m {…}
        // { method $m … }`) is the same judgement one level in: the recorded
        // member tables are a lower bound, so a method that is absent from
        // them is not thereby missing from the class.
        cd.inheritance_unknown
            || cd.member_set_incomplete
            || cd
                .superclasses
                .iter()
                .chain(&cd.mixins)
                .any(|s| !self.result.all_classes.contains_key(s) && !OO_BASE.contains(&s.as_str()))
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
fn parse_namespaced_ensemble(
    source: &str,
    span: tcl_lexer::Span,
    braced_var: tcl_dialect::BracedVarStyle,
) -> Option<(String, String)> {
    let start = span.start() as usize;
    let end = (span.end() as usize).min(source.len());
    if start >= end {
        return None;
    }
    let head = &source[start..end];
    head.strip_prefix("${")?;
    // The closer comes from the shared owner under this document's release
    // rule — `2` is the byte just past the `${`. A first-`}` scan split
    // `${a{b}c}::tail` at the wrong brace, so the prefix this reports is a
    // variable the source never names.
    let tcl_lexer::BracedVarEnd::Closed(close) =
        tcl_lexer::braced_var_name_end(head.as_bytes(), 2, braced_var)
    else {
        return None;
    };
    let (prefix, after) = (&head[2..close], &head[close + 1..]);
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
    // This is a resolved name, so a leading dollar is literal.
    // naming.diagnostics.original-variable-name-anchor
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-variable-name-anchor.md
    let Some((_base, key)) = crate::naming::split_element_ref(var_name) else {
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

/// Values are harvested only through original reached consumption carriers and
/// exact physical contents ancestry. A literal's command lookup occurs in the
/// consuming invocation's post-argv world, rather than its producer's namespace.
fn harvest_table_command_value_spans(
    cu: &crate::compilation_unit::CompilationUnit,
    source: &str,
    context: &tcl_registry::model::ContextRegistry,
    config: tcl_lexer::LexerConfig,
    consumed: &HashSet<u32>,
) -> Vec<(
    String,
    tcl_lexer::Span,
    crate::command_binding::SourceCommandReference,
)> {
    use crate::ir::WordExpr;
    if consumed.is_empty() {
        return Vec::new();
    }
    let mut selected = HashMap::new();
    for unit in cu.all_body_function_units() {
        for (&block, body) in &unit.ssa.blocks {
            for index in 0..body.statements.len() {
                let view = crate::ssa::SsaSourceView::at_statement(&unit.ssa, block, index);
                let Some(tokens) = view
                    .source_tokens()
                    .filter(|tokens| tokens.synthetic.is_none())
                else {
                    continue;
                };
                let Some(head) = tokens.words().first() else {
                    continue;
                };
                let expanded = matches!(head, WordExpr::Expand { .. });
                let written_start = unit.abs_span(head.source().span).start();
                let head = if let WordExpr::Expand { word, .. } = head {
                    word.as_ref()
                } else {
                    head
                };
                if !consumed.contains(&written_start)
                    && !consumed.contains(&unit.abs_span(head.source().span).start())
                {
                    continue;
                }
                let Some(binding) = &tokens.source_binding else {
                    continue;
                };
                let read = dispatch_table_values(cu, unit, tokens, head, source, context, config);
                for value in read.into_iter().flatten() {
                    let Some(value) = dispatch_table_head(value, expanded, binding) else {
                        continue;
                    };
                    let Some(span) = value.literal_span else {
                        continue;
                    };
                    let reference = binding.command_reference(&value.value);
                    let key = (span.start(), span.end());
                    let entry = selected
                        .entry(key)
                        .or_insert_with(|| (value.value.clone(), span, reference.clone()));
                    if entry.0 != value.value || entry.2 != reference {
                        entry.2 = None;
                    }
                }
            }
        }
    }
    selected
        .into_values()
        .filter_map(|(value, span, reference)| Some((value, span, reference?)))
        .collect()
}

fn dispatch_table_values(
    compilation: &crate::compilation_unit::CompilationUnit,
    unit: &crate::compilation_unit::FunctionUnit,
    tokens: &crate::ir::CommandTokens,
    head: &crate::ir::WordExpr,
    source: &str,
    context: &tcl_registry::model::ContextRegistry,
    config: tcl_lexer::LexerConfig,
) -> Option<Vec<crate::table_value_provenance::TableValueContributor>> {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Availability and lexer overrides remain separate from reached cell contents.
    let registry = context.commands();
    if head.sole_variable_substitution().is_some() {
        return crate::table_value_provenance::table_values(
            compilation,
            unit,
            tokens,
            head,
            None,
            source,
            registry,
        );
    }
    tokens
        .source_binding
        .as_ref()?
        .variable_context
        .invocation_dialect?;
    let mut nested =
        crate::word_subst::whole_word_command_tokens(head, tokens.native_lexer_config(config))?;
    nested.inherit_nested_bindings(tokens);
    let (input, keys) =
        crate::registry_invocation::normal_representation_invocation_in_context(context, &nested)?
            .dictionary_lookup()?;
    if keys.is_empty() {
        return None;
    }
    crate::table_value_provenance::table_values(
        compilation,
        unit,
        &nested,
        &input,
        Some(&keys),
        source,
        registry,
    )
}

fn dispatch_table_head(
    value: crate::table_value_provenance::TableValueContributor,
    expanded: bool,
    binding: &crate::command_binding::SourceInvocationBinding,
) -> Option<crate::value_provenance::ValueContributor> {
    let mut value = match value {
        crate::table_value_provenance::TableValueContributor::ListHead(value) => {
            return expanded.then_some(value);
        }
        crate::table_value_provenance::TableValueContributor::SingletonList(value) => {
            return (expanded
                || tcl_syntax::list::join_list([value.value.as_str()]) == value.value)
                .then_some(value);
        }
        crate::table_value_provenance::TableValueContributor::Prefix(value) => value,
    };
    if !expanded {
        return Some(value);
    }
    let rules = binding.variable_context.invocation_dialect?.word_values;
    if rules.split_list(&value.value).ok()?.is_empty() {
        return None;
    }
    let element =
        tcl_syntax::list::find_element_with_syntax(&value.value, 0, rules.list).ok()??;
    if !element.literal {
        return None;
    }
    let base = value.literal_span?.start();
    value.literal_span = Some(tcl_lexer::Span::new(
        base.checked_add(u32::try_from(element.value.start).ok()?)?,
        base.checked_add(u32::try_from(element.value.end).ok()?)?,
    ));
    value.value.get(element.value.clone())?;
    value.value.truncate(element.value.end);
    value.value.replace_range(..element.value.start, "");
    Some(value)
}

/// Literal element assignment candidates from the retained source May
/// envelope. They suppress W307 only; they are not SSA or executed-store facts.
fn harvest_array_element_set_constants(
    cu: &crate::compilation_unit::CompilationUnit,
    out: &mut HashMap<String, HashSet<String>>,
    registry: &tcl_registry::CommandRegistry,
) {
    let units = std::iter::once(&cu.top_level).chain(cu.procedures.values());
    for fu in units {
        let Some(metadata) = fu.invocation_metadata_context_for_module(registry, &cu.ir_module)
        else {
            continue;
        };
        let rules = tcl_syntax::word_rules::WordValueRules::from_config(&fu.source_lexer_config());
        for (&block, body) in &fu.cfg.blocks {
            for (index, _) in body.statements.iter().enumerate() {
                let Some(tokens) =
                    crate::ssa::SsaSourceView::at_statement(&fu.ssa, block, index).source_tokens()
                else {
                    continue;
                };
                for assignment in
                    crate::registry_invocation::advisory_value_assignments_with_metadata_context(
                        registry,
                        Some(metadata),
                        tokens,
                    )
                {
                    if tcl_syntax::naming::split_element_ref(&assignment.name).is_none() {
                        continue;
                    }
                    if let crate::registry_invocation::EffectiveInvocationWord::Literal(value) =
                        crate::registry_invocation::effective_invocation_word(
                            &assignment.value,
                            fu.source_lexer_config().escapes,
                            rules,
                        )
                    {
                        out.entry(assignment.name).or_default().insert(value);
                    }
                }
            }
        }
    }
}

fn harvest_array_set_constants(
    cu: &crate::compilation_unit::CompilationUnit,
    out: &mut HashMap<String, HashSet<String>>,
    context: &tcl_registry::model::ContextRegistry,
) {
    let units = std::iter::once(&cu.top_level).chain(cu.procedures.values());
    for fu in units {
        let Some(metadata) =
            fu.invocation_metadata_context_for_module(context.commands(), &cu.ir_module)
        else {
            continue;
        };
        let rules = tcl_syntax::word_rules::WordValueRules::from_config(&fu.source_lexer_config());
        for (&block, body) in &fu.cfg.blocks {
            for (index, _) in body.statements.iter().enumerate() {
                let Some(tokens) =
                    crate::ssa::SsaSourceView::at_statement(&fu.ssa, block, index).source_tokens()
                else {
                    continue;
                };
                let Some(invocation) =
                    crate::registry_invocation::original_callback_invocation_with_metadata_context(
                        context.commands(),
                        metadata,
                        tokens,
                    )
                else {
                    continue;
                };
                let Some(tcl_registry::VarElementsEffect::SetsArrayElementsFromList { values_at }) =
                    invocation.facts.var_elements_effect
                else {
                    continue;
                };
                let target = invocation
                    .facts
                    .arg_roles
                    .iter()
                    .find(|(_, role)| *role == tcl_registry::ArgRole::VarWrite)
                    .map(|(index, _)| invocation.facts.argument_offset + usize::from(*index));
                let Some(arr_name) = target.and_then(|index| invocation.argument_literal(index))
                else {
                    continue;
                };
                let Some(list) = invocation
                    .argument_literal(invocation.facts.argument_offset + usize::from(values_at))
                else {
                    continue;
                };
                let items = crate::tcl_expr_eval::split_tcl_list(&list, rules);
                if !items.len().is_multiple_of(2) {
                    continue;
                }
                for pair in items.as_chunks::<2>().0 {
                    let elem_name = format!("{arr_name}({})", pair[0]);
                    out.entry(elem_name).or_default().insert(pair[1].clone());
                }
            }
        }
    }
}

/// Conditional dictionary-body values from the statement's original
/// dictionary read and its represented SSA use version. The binder plan keeps
/// alias captures and the actual retained availability and grammar together.
fn harvest_dict_with_constants(
    cu: &crate::compilation_unit::CompilationUnit,
    out: &mut HashMap<String, HashSet<String>>,
    registry: &tcl_registry::CommandRegistry,
) {
    use crate::value_transfer::{
        DictBinder, dict_body_operand_with_metadata_context, dict_body_with_metadata_context,
    };
    let units = std::iter::once(&cu.top_level).chain(cu.procedures.values());
    for fu in units {
        let Some(metadata) = fu.invocation_metadata_context_for_module(registry, &cu.ir_module)
        else {
            continue;
        };
        for (&block_id, block) in &fu.cfg.blocks {
            let Some(ssa_block) = fu.ssa.blocks.get(&block_id) else {
                continue;
            };
            for (index, _) in block.statements.iter().enumerate() {
                let Some(tokens) =
                    crate::ssa::SsaSourceView::at_statement(&fu.ssa, block_id, index)
                        .source_tokens()
                else {
                    continue;
                };
                let Some((_, name)) =
                    dict_body_operand_with_metadata_context(registry, metadata, tokens)
                else {
                    continue;
                };
                let root = tcl_syntax::naming::split_element_ref(&name)
                    .map_or(name.as_str(), |(root, _)| root);
                let dictionary = fu
                    .ssa
                    .var_symbol_at(block_id, index, root)
                    .and_then(|symbol| {
                        let version = *ssa_block.statements.get(index)?.uses.get(&symbol)?;
                        match fu
                            .diagnostic_value_facts()
                            .values()
                            .get(&(symbol, version))?
                        {
                            crate::analyses::LatticeValue::Const(value) => {
                                crate::value_transfer::const_text(value)
                            }
                            _ => None,
                        }
                    });
                let Some(binders) = dictionary.as_deref().and_then(|dictionary| {
                    dict_body_with_metadata_context(registry, metadata, tokens, dictionary)
                }) else {
                    continue;
                };
                for binder in binders {
                    if let DictBinder::Key { name, value } = binder {
                        out.entry(name).or_default().insert(value);
                    }
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
/// a `Statement::Return` or a `Terminator::Return`) and keeps the last whose
/// value is a single `$var`.  Used by the object-returning-proc inference:
/// a proc returning `$X` where `X` was assigned from a factory is itself an
/// object factory.
/// Per-scope tainted variable names (`::top` for the top-level scope): any
/// tainted SSA version of a name disqualifies it from W307 dispatcher
/// suppression.
fn build_tainted_by_scope(
    cu: &crate::compilation_unit::CompilationUnit,
) -> FxHashMap<String, HashSet<String>> {
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
    tainted_by_scope
}

/// Aggregate constant-string knowledge (var name → flat CONST/CONSTSET value
/// set) across the top level, every proc, and every method body of `cu`,
/// then fold in the element writes each statement states and the keys a
/// `dict with` binds.  Used by the W307 non-literal-command-name check.
fn aggregate_constsets(
    cu: &crate::compilation_unit::CompilationUnit,
    context: &tcl_registry::model::ContextRegistry,
) -> std::collections::HashMap<String, HashSet<String>> {
    let mut all_constsets: std::collections::HashMap<String, HashSet<String>> =
        std::collections::HashMap::new();
    let collect_from =
        |fu: &crate::compilation_unit::FunctionUnit,
         out: &mut std::collections::HashMap<String, HashSet<String>>| {
            for ((sym, _ver), lv) in fu.diagnostic_value_facts().values() {
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
    // blanket `in_method` W307 suppression.
    for fu in cu.methods.values() {
        collect_from(fu, &mut all_constsets);
    }

    harvest_array_set_constants(cu, &mut all_constsets, context);
    harvest_array_element_set_constants(cu, &mut all_constsets, context.commands());
    harvest_dict_with_constants(cu, &mut all_constsets, context.commands());
    all_constsets
}

/// The per-proc maps built by the factory-object analysis in
/// [`Analyser::compute_factory_object_ranges`]: factory-local vars, the
/// `{var -> rhs command head}` assignment map, the last returned var, and the
/// set of procedures with a possible object result.
#[derive(Default)]
struct FactoryObjectAdvice {
    ranges: Vec<(u32, u32, HashSet<String>)>,
    object_returning: FxHashSet<String>,
}

#[derive(Default)]
struct FactoryMaps {
    factory_locals: FxHashMap<String, HashSet<String>>,
    assigns: FxHashMap<String, FxHashMap<String, String>>,
    return_var: FxHashMap<String, Option<String>>,
    object_returning: FxHashSet<String>,
    returned_callees: FxHashMap<String, FxHashSet<String>>,
}

/// Populate [`FactoryMaps`] for one analysable unit (`qname` / `fu`): record
/// every `set X [head …]` assignment, mark `X` a factory local when `head` is
/// object-returning and not a user proc, capture the last returned var, and
/// retain possible factory returns independently of completion and dispatch
/// closure. Unknown return paths never become strict result-type evidence.
fn seed_factory_maps(
    qname: &str,
    fu: &crate::compilation_unit::FunctionUnit,
    context: &tcl_registry::model::ContextRegistry,
    nominal_result: &impl Fn(
        &crate::ir::WordExpr,
        Option<&crate::ir::CommandTokens>,
    ) -> (bool, Option<String>),
    maps: &mut FactoryMaps,
    config: tcl_lexer::LexerConfig,
) {
    let mut names = HashSet::new();
    let mut amap = FxHashMap::default();
    let mut returns = Vec::new();
    let mut returned_variables = Vec::new();
    for block in fu.cfg.blocks.values() {
        for stmt in &block.statements {
            let assignments = match stmt {
                crate::ir::Statement::AssignValue {
                    name,
                    value,
                    tokens,
                    ..
                } => tokens
                    .as_ref()
                    .and_then(|tokens| tokens.words().get(2))
                    .cloned()
                    .or_else(|| crate::value_shapes::value_word_with_config(value, config))
                    .map(|value| {
                        vec![crate::registry_invocation::AdvisoryValueAssignment {
                            name: name.clone(),
                            value,
                        }]
                    })
                    .unwrap_or_default(),
                _ => stmt
                    .tokens()
                    .map(|tokens| {
                        crate::registry_invocation::advisory_value_assignments_in_context(
                            context, tokens,
                        )
                    })
                    .unwrap_or_default(),
            };
            for word in stmt.tokens().into_iter().flat_map(|tokens| {
                crate::registry_invocation::advisory_return_values_in_context(context, tokens)
            }) {
                returned_variables.push(match &word {
                    crate::ir::WordExpr::Variable { spelling, .. } => extract_dollar_var(spelling),
                    _ => None,
                });
                let (object, callee) = nominal_result(&word, stmt.tokens());
                returns.push(object);
                if let Some(callee) = callee {
                    maps.returned_callees
                        .entry(qname.to_owned())
                        .or_default()
                        .insert(callee);
                }
            }
            for assignment in assignments {
                let (object, callee) = nominal_result(&assignment.value, stmt.tokens());
                if object {
                    names.insert(assignment.name.clone());
                }
                if let Some(callee) = callee {
                    amap.insert(assignment.name, callee);
                }
            }
        }
        if let Some(crate::cfg::Terminator::Return {
            value_word, tokens, ..
        }) = &block.terminator
        {
            let (object, callee) = value_word.as_ref().map_or((false, None), |word| {
                nominal_result(word, tokens.as_deref())
            });
            returns.push(object);
            if let Some(callee) = callee {
                maps.returned_callees
                    .entry(qname.to_owned())
                    .or_default()
                    .insert(callee);
            }
        } else if block.terminator.is_none() {
            returns.push(false);
        }
    }
    maps.factory_locals.insert(qname.to_string(), names);
    maps.assigns.insert(qname.to_string(), amap);
    let advisory_variable = returned_variables
        .first()
        .cloned()
        .flatten()
        .filter(|first| {
            returned_variables
                .iter()
                .all(|candidate| candidate.as_ref() == Some(first))
        });
    maps.return_var.insert(
        qname.to_string(),
        advisory_variable.or_else(|| last_return_var_of(&fu.cfg)),
    );
    if returns.iter().any(|value| *value) {
        maps.object_returning.insert(qname.to_string());
    }
}

/// Fixpoint over the factory-object analysis maps for
/// [`Analyser::compute_factory_object_ranges`].
///
/// First propagates `object_returning`: a proc whose returned var is assigned
/// `[other]` where `other` is a proven object-returning user proc is itself
/// object-returning — iterated to a fixpoint.  Then extends `factory_locals`:
/// a `set X [user_proc]` whose proc is now proven object-returning makes `X` a
/// factory local too.  `bare_to_qnames` resolves relative call heads to the
/// qualified names the analysis is keyed on.
fn propagate_object_returning(
    return_var: &FxHashMap<String, Option<String>>,
    returned_callees: &FxHashMap<String, FxHashSet<String>>,
    assigns: &FxHashMap<String, FxHashMap<String, String>>,
    bare_to_qnames: &FxHashMap<&str, Vec<&str>>,
    object_returning: &mut FxHashSet<String>,
    factory_locals: &mut FxHashMap<String, HashSet<String>>,
) {
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
        for (qname, callees) in returned_callees {
            if !object_returning.contains(qname)
                && callees.iter().any(|callee| {
                    resolve_candidates(callee)
                        .iter()
                        .any(|candidate| object_returning.contains(candidate))
                })
            {
                object_returning.insert(qname.clone());
                changed = true;
            }
        }
        for (qname, rv) in return_var {
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
    for (qname, amap) in assigns {
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
}

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

/// External OO base classes that aren't in the per-document
/// ``ClassDef`` index but are recognised as legitimate
/// superclasses / mixins for W308 / W308-related gates.
const OO_BASE: [&str; 2] = ["oo::object", "oo::class"];

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
/// Index of the innermost proc body range containing `off`, or `None` for the
/// `::top` sentinel scope.  `proc_body_ranges` is sorted largest-start-first,
/// so the reverse scan finds the innermost enclosing range (procs don't nest,
/// but `namespace eval` bodies can wrap several, so this stays robust).
fn w307_enclosing_idx(
    proc_body_ranges: &[(u32, u32, String, HashSet<String>)],
    off: u32,
) -> Option<usize> {
    proc_body_ranges
        .iter()
        .enumerate()
        .rev()
        .find(|(_, (s, e, _, _))| *s <= off && off <= *e)
        .map(|(i, _)| i)
}

fn w307_precise_cmd_values(
    func_ranges: &[W307FunctionRange<'_>],
    offset: u32,
    var_name: &str,
    registry: Option<&tcl_registry::CommandRegistry>,
) -> Option<HashSet<String>> {
    // Narrowest function range containing `offset`.
    let mut best = None;
    for range in func_ranges {
        if range.start <= offset && offset <= range.end {
            let width = range.end - range.start;
            if best.is_none_or(|(bw, _)| width < bw) {
                best = Some((width, range.function));
            }
        }
    }
    let fu = best?.1;
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
            let version = ssa_stmt.uses.get(&sym).copied().or_else(|| {
                declaration_head_value_version(
                    fu,
                    *block_name,
                    idx,
                    offset,
                    var_name,
                    sym,
                    registry?,
                )
            });
            let Some(version) = version else {
                continue;
            };
            let width = span.end() - span.start();
            if best_width.is_none_or(|bw| width < bw) {
                best_width = Some(width);
                best_version = Some(version);
            }
        }
    }
    let version = best_version?;
    let lv = fu.diagnostic_value_facts().values().get(&(sym, version))?;
    Some(lattice_command_values(lv)?.into_iter().collect())
}

/// Conditional declaration-local value advice for this exact original head read.
/// No represented SSA use, executed lookup or physical read is created.
fn declaration_head_value_version(
    fu: &crate::compilation_unit::FunctionUnit,
    block: crate::cfg::BlockId,
    index: usize,
    offset: u32,
    name: &str,
    symbol: crate::ssa::Symbol,
    registry: &tcl_registry::CommandRegistry,
) -> Option<crate::ssa::Version> {
    let tokens = crate::ssa::SsaSourceView::at_statement(&fu.ssa, block, index).source_tokens()?;
    let head = tokens.words().first()?;
    if head.source().span.start() != offset {
        return None;
    }
    let crate::ir::WordExpr::Variable { spelling, .. } = head else {
        return None;
    };
    let binding = tokens.source_binding.as_ref()?;
    let reads = binding.declaration_read_occurrences(registry, tokens)?;
    let mut exact = reads.iter().filter(|read| {
        read.name() == name && read.source() == head.source() && read.spelling() == spelling
    });
    let read = exact.next()?;
    if exact.next().is_some() {
        return None;
    }
    let (selected, version) = read.diagnostic_version(&fu.ssa, block, index, registry)?;
    (selected == symbol).then_some(version)
}

#[cfg(test)]
mod retained_constant_context_tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn array_constant_harvest_uses_retained_operand_grammar_over_catalogue_profile() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        // Original source-value advice; no executed store or Native argv claim.
        let source = r#"array set state "callback \U0001F600""#;
        let catalogue = tcl_registry::model::ingress::static_context_for("tcl9.0");
        for (environment, expected) in [("tcl8.4", "U0001F600"), ("tcl9.0", "\u{1F600}")] {
            let profile = tcl_dialect::DialectProfile::find(environment).unwrap();
            let context = Arc::new(
                tcl_registry::model::ingress::static_context_for(environment)
                    .with_command_store(Arc::clone(catalogue.commands())),
            );
            let dialect = tcl_registry::InvocationDialect::of_profile(profile);
            // The source-authoring entry supplies its explicit dialect below.
            // A Driver entry would require independent Native operand/handler
            // premises and would test a different admission boundary.
            let cu = crate::compilation_unit::CompilationUnit::build_with_context_registry(
                source,
                crate::compilation_unit::UnitBuildOptions {
                    registry: context.commands(),
                    defer_top_level: false,
                    config: tcl_lexer::LexerConfig::for_profile(Some(profile)),
                    dialect: Some(profile),
                    external_call_sites: None,
                    declared_commands: None,
                },
                None,
                Arc::clone(&context),
            );
            let selected: Vec<_> = cu
                .top_level
                .cfg
                .blocks
                .values()
                .flat_map(|block| &block.statements)
                .filter_map(|statement| {
                    crate::registry_invocation::resolved_statement_invocation_in_context(
                        &context, statement,
                    )
                })
                .collect();
            assert_eq!(
                selected.len(),
                1,
                "positive original metadata: {environment}"
            );
            assert_eq!(selected[0].dialect, Some(dialect));
            assert!(matches!(
                selected[0].facts.var_elements_effect,
                Some(tcl_registry::VarElementsEffect::SetsArrayElementsFromList { .. })
            ));
            let mut values = HashMap::new();
            harvest_array_set_constants(&cu, &mut values, &context);
            assert_eq!(
                values.get("state(callback)"),
                Some(&HashSet::from([expected.to_owned()])),
                "retained source grammar: {environment}"
            );
        }
    }
}

#[cfg(test)]
mod original_callback_slot_name_tests {
    use super::*;

    #[test]
    fn callback_slot_hints_use_shared_resolved_element_name_boundaries() {
        // naming.diagnostics.original-variable-name-anchor
        // docs/design/analysis/name-resolution-proofs/diagnostic-original-variable-name-anchor.md
        // Callback-shaped names are suppression hints only; these controls
        // assert neither current callback occupancy nor native variable reads.
        for name in [
            "state(-command)",
            "$state(-command)",
            "é(doneCallback)",
            "(-command)",
            "state(a)(-command)",
        ] {
            assert!(is_callback_array_slot(name), "{name}");
        }
        for name in [
            "state(-command",
            "state(doneCallback",
            "state()",
            "doneCallback",
            "state(-command)tail",
            "state(a)(data)",
        ] {
            assert!(!is_callback_array_slot(name), "{name}");
        }
    }
}

#[cfg(test)]
mod factory_metadata_context_tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn factory_may_advice_uses_actual_availability_and_exact_supplied_source() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        // These Registry declarations supply possible source result shapes,
        // never an actual factory implementation, allocation or return value.
        let source = "gated-store handle [possible-object]";
        let mut registry = tcl_registry::CommandRegistry::build_default();
        registry.insert(tcl_registry::CommandSpec {
            name: "gated-store",
            surface: registry.get("dict").unwrap().surface,
            ..registry.get("set").unwrap().clone()
        });
        registry.insert(tcl_registry::CommandSpec {
            name: "possible-object",
            return_type: Some(tcl_registry::TclType::Object),
            ..tcl_registry::CommandSpec::DEFAULT
        });
        let store = Arc::new(registry);
        for (environment, expected) in [("tcl8.4", false), ("tcl9.0", true)] {
            let profile = tcl_dialect::DialectProfile::find(environment).unwrap();
            let context = Arc::new(
                tcl_registry::model::ingress::static_context_for(environment)
                    .with_command_store(Arc::clone(&store)),
            );
            let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
            let input = crate::analyser::ResolvedAnalysisInput::new(
                profile,
                profile,
                Arc::clone(&context),
                config,
            );
            let mut analyser = Analyser::new().with_resolved_input(input);
            let analysis = analyser.analyse(source, environment);
            let mut cu = crate::compilation_unit::CompilationUnit::build_with_context_registry(
                source,
                crate::compilation_unit::UnitBuildOptions {
                    registry: context.commands(),
                    defer_top_level: false,
                    config,
                    dialect: Some(profile),
                    external_call_sites: None,
                    declared_commands: None,
                },
                None,
                Arc::clone(&context),
            );
            let advice = analyser.compute_factory_object_ranges(&cu, context.commands());
            assert_eq!(
                advice
                    .ranges
                    .iter()
                    .any(|(_, _, names)| names.contains("handle")),
                expected,
                "{environment}"
            );
            let foreign = tcl_registry::CommandRegistry::build_default();
            assert!(
                analyser
                    .compute_factory_object_ranges(&cu, &foreign)
                    .ranges
                    .is_empty()
            );
            analyser.result.resolved_input = None;
            assert!(
                analyser
                    .compute_factory_object_ranges(&cu, context.commands())
                    .ranges
                    .is_empty()
            );
            analyser.result = analysis.clone();
            cu.ir_module.source_metadata_input = None;
            assert!(
                analyser
                    .compute_factory_object_ranges(&cu, context.commands())
                    .ranges
                    .is_empty()
            );
            cu.ir_module.source_metadata_input = analysis.resolved_input.clone();
            let original_profile = cu.ir_module.dialect_profile;
            cu.ir_module.dialect_profile = Some(tcl_dialect::DialectProfile::find("tcl").unwrap());
            assert!(
                analyser
                    .compute_factory_object_ranges(&cu, context.commands())
                    .ranges
                    .is_empty(),
                "changed actual Module profile cannot borrow the input"
            );
            cu.ir_module.dialect_profile = original_profile;
            let original_config = cu.ir_module.lexer_config;
            cu.ir_module.lexer_config.expand_syntax = !original_config.expand_syntax;
            assert!(
                analyser
                    .compute_factory_object_ranges(&cu, context.commands())
                    .ranges
                    .is_empty(),
                "changed actual Module syntax cannot borrow the input"
            );
            cu.ir_module.lexer_config = original_config;
            cu.source.push(' ');
            assert!(
                analyser
                    .compute_factory_object_ranges(&cu, context.commands())
                    .ranges
                    .is_empty()
            );
        }
    }
}
