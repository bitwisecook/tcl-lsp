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

//! Conditional procedure completion under one genuine Logical source Module.
//!
//! The walk shares original declaration/allocation, effective argument and child
//! invocation owners with the deletion consumer. It models straight-line local
//! scalar stores, returns and selected calls whose argument counts and normal
//! source completion are proved. It preserves every original child horizon and
//! the independently selected formal grammar. Missing, foreign or Native-only
//! carriers provide no conditional completion. Runtime entry, traces, activation,
//! no-error and edit permission remain independent prerequisites.

use std::collections::{HashMap, HashSet};

use tcl_registry::CommandRegistry;
use tcl_registry::completion::{CompletionCode, CompletionCodeDomain};

use crate::ir::{CommandTokens, Module, Script, Statement, WordExpr, WordPart};

/// Conditional no-error source walk under one genuine Logical Module.
/// Every child retains its original point/vector; source syntax does not supply
/// Native normal completion, entered frames or executable erasure permission.
pub(crate) struct CompletionWalk<'w> {
    module: &'w Module,
    registry: &'w CommandRegistry,
    registry_commands: bool,
    pub(crate) calls: Vec<(String, usize)>,
    pub(crate) reads: Vec<String>,
}

impl<'w> CompletionWalk<'w> {
    pub(crate) fn for_module(
        module: &'w Module,
        registry: &'w CommandRegistry,
        registry_commands: bool,
    ) -> Option<Self> {
        if !module
            .retained_source_bindings
            .as_ref()?
            .matches_module(module, registry)
            || !crate::registry_invocation::InvocationMetadataContext::for_module(registry, module)?
                .permits_logical_source_names()
        {
            return None;
        }
        Some(Self {
            module,
            registry,
            registry_commands,
            calls: Vec::new(),
            reads: Vec::new(),
        })
    }

    /// One authentic written operand, evaluated in its original parent horizon.
    pub(crate) fn word_in(&mut self, parent: &CommandTokens, word: &WordExpr) -> bool {
        if !parent.words().contains(word) || !self.word(word) {
            return false;
        }
        self.substitutions(parent, Some(word.source().span))
    }

    fn command(&mut self, tokens: &CommandTokens) -> bool {
        self.select(tokens)
            && tokens.words().iter().all(|word| self.word(word))
            && self.substitutions(tokens, None)
    }

    /// Lexical variable-read obligations only. The shared child inventory
    /// separately authenticates every command substitution's geometry and lookup.
    fn word(&mut self, word: &WordExpr) -> bool {
        match word {
            WordExpr::Literal { .. }
            | WordExpr::BracedLiteral { .. }
            | WordExpr::CommandSubstitution { .. } => true,
            WordExpr::Variable { spelling, .. } => self.read(spelling),
            WordExpr::Template {
                rejected: Some(_), ..
            }
            | WordExpr::Expand { .. }
            | WordExpr::Opaque { .. } => false,
            WordExpr::Template { parts, .. } => parts.iter().all(|part| match part {
                WordPart::Text { .. } | WordPart::CommandSubstitution { .. } => true,
                WordPart::Variable { spelling, .. } => self.read(spelling),
                WordPart::Opaque { .. } => false,
            }),
        }
    }

    fn read(&mut self, spelling: &str) -> bool {
        let config = self.module.native_lexer_config().nested();
        let Ok(Some(reference)) = tcl_lexer::word_parts::whole_var_ref(spelling.as_bytes(), config)
        else {
            return false;
        };
        if reference.index.is_some()
            || tcl_syntax::naming::split_element_ref_bytes(reference.name).is_some()
        {
            return false;
        }
        let Ok(Some(name)) =
            tcl_syntax::naming::variable_reference_root_bytes(spelling.as_bytes(), config)
        else {
            return false;
        };
        if name.is_empty() {
            return false;
        }
        let Ok(name) = std::str::from_utf8(name) else {
            return false;
        };
        self.reads.push(name.to_owned());
        true
    }

    fn substitutions(&mut self, parent: &CommandTokens, within: Option<tcl_lexer::Span>) -> bool {
        let Some(binding) = parent.source_binding.as_ref() else {
            return false;
        };
        let Some(metadata) =
            binding.original_invocation_metadata_for_module(parent, self.module, self.registry)
        else {
            return false;
        };
        let Some(children) = crate::word_subst::checked_original_lifted_calls_with_metadata_context(
            parent,
            self.module.native_lexer_config(),
            self.registry,
            metadata,
        ) else {
            return false;
        };
        children
            .into_iter()
            .filter(|child| {
                within.is_none_or(|span| {
                    span.start() <= child.span.start() && child.span.end() <= span.end()
                })
            })
            .all(|child| {
                child.tokens.as_ref().is_some_and(|tokens| {
                    self.select(tokens) && tokens.words().iter().all(|word| self.word(word))
                })
            })
    }

    fn select(&mut self, tokens: &CommandTokens) -> bool {
        if let Some(calls) = crate::registry_invocation::original_logical_procedure_calls_for_module(
            tokens,
            self.module,
            self.registry,
        ) {
            if calls.iter().any(|call| !call.accepts_arguments()) {
                return false;
            }
            for call in calls {
                let Some(count) = call.argument_count() else {
                    return false;
                };
                self.calls
                    .push((call.procedure().qualified_name.clone(), count));
            }
            return true;
        }
        self.registry_commands
            && self.invocation(tokens).is_some_and(|invocation| {
                invocation.facts.arity_accepts_frozen_arguments() == Some(true)
                    && matches!(invocation.facts.completion.codes,
                    CompletionCodeDomain::Exact(codes) if codes == [CompletionCode::Ok])
            })
    }

    fn invocation(
        &self,
        tokens: &CommandTokens,
    ) -> Option<crate::registry_invocation::ResolvedStatementInvocation> {
        let metadata = tokens
            .source_binding
            .as_ref()?
            .original_invocation_metadata_for_module(tokens, self.module, self.registry)?;
        crate::registry_invocation::original_logical_operation_invocation_with_metadata_context(
            self.registry,
            metadata,
            tokens,
        )
    }
}

/// Conditional normal-completion model for the Module's original Logical
/// procedure bodies. The complete source owner, selected formal grammar and
/// original point/child receipts are mandatory. Missing or Native-only owners
/// state no completion; physical Native entry consumes its independent proofs.
pub(super) fn procedures_complete(
    module: &Module,
    registry: &CommandRegistry,
    defined: &dyn Fn(&str, &str) -> bool,
) -> HashMap<String, bool> {
    let calls: HashMap<&str, Vec<(String, usize)>> = module
        .procedures
        .iter()
        .filter_map(|(qname, procedure)| {
            if qname != &procedure.qualified_name {
                return None;
            }
            let bound = crate::registry_invocation::original_procedure_scalar_bindings(
                module, procedure, registry,
            )?;
            if !module.traced_commands.is_empty()
                || module.has_dynamic_trace
                || !module.traced_variables.is_empty()
            {
                return None;
            }
            let mut scan = BodyScan {
                walk: CompletionWalk::for_module(module, registry, true)?,
                bound,
            };
            scan.script(&procedure.body)
                .then_some((qname.as_str(), scan.walk.calls))
        })
        .collect();
    let mut completes: HashSet<&str> = HashSet::new();
    loop {
        let before = completes.len();
        for (&qname, callees) in &calls {
            if completes.contains(qname) {
                continue;
            }
            if callees.iter().all(|(callee, count)| {
                completes.contains(callee.as_str())
                    && !module.redefined_procedures.contains(callee)
                    && defined(callee, qname)
                    && module
                        .procedures
                        .get(callee)
                        .and_then(|procedure| {
                            crate::registry_invocation::original_procedure_formal_count_shape(
                                module, procedure, registry,
                            )
                        })
                        .is_some_and(|shape| shape.accepts(*count))
            }) {
                completes.insert(qname);
            }
        }
        if completes.len() == before {
            break;
        }
    }
    module
        .procedures
        .keys()
        .map(|qname| (qname.clone(), completes.contains(qname.as_str())))
        .collect()
}

/// Straight-line local scalar bindings, independent of any Native activation.
struct BodyScan<'w> {
    walk: CompletionWalk<'w>,
    bound: HashSet<String>,
}

impl BodyScan<'_> {
    fn script(&mut self, script: &Script) -> bool {
        script.statements.iter().all(|statement| {
            script
                .retained_source_tokens_for_statement(statement)
                .is_some_and(|tokens| self.statement(statement, tokens))
        })
    }

    fn statement(&mut self, statement: &Statement, tokens: &CommandTokens) -> bool {
        if statement.synthetic_marker().is_some() {
            return false;
        }
        match statement {
            Statement::AssignConst { name, .. } => self.assign(name, tokens, None),
            Statement::AssignValue { name, .. } => {
                let [_, _, value] = tokens.words() else {
                    return false;
                };
                self.assign(name, tokens, Some(value))
            }
            Statement::Call {
                foreach_groups: None,
                ..
            } => self.reading(|walk| walk.command(tokens)),
            Statement::Return {
                expr: None,
                value_word,
                value,
                ..
            } => {
                let Some(invocation) = self.walk.invocation(tokens) else {
                    return false;
                };
                if invocation.facts.operation
                    != tcl_registry::SemanticOperationId::StructuredLowering(
                        tcl_registry::hooks::LoweringHookId::Return,
                    )
                    || invocation.facts.arity_accepts_frozen_arguments() != Some(true)
                    || invocation.effective.words.len() != 1 + usize::from(value.is_some())
                {
                    return false;
                }
                match value_word {
                    Some(word) => self.reading(|walk| walk.word_in(tokens, word)),
                    None => value.is_none() && self.walk.substitutions(tokens, None),
                }
            }
            _ => false,
        }
    }

    fn assign(&mut self, name: &str, tokens: &CommandTokens, value: Option<&WordExpr>) -> bool {
        let Some(invocation) = self.walk.invocation(tokens) else {
            return false;
        };
        if invocation.facts.operation
            != tcl_registry::SemanticOperationId::StructuredLowering(
                tcl_registry::hooks::LoweringHookId::Set,
            )
            || invocation.facts.arity_accepts_frozen_arguments() != Some(true)
            || invocation.effective.words.len() != 3
            || invocation.argument_literal(0).as_deref() != Some(name)
            || tcl_syntax::naming::split_element_ref_bytes(name.as_bytes()).is_some()
            || tcl_syntax::naming::is_qualified(name.as_bytes())
            || name.is_empty()
            || value.is_some_and(|value| !self.reading(|walk| walk.word_in(tokens, value)))
            || (value.is_none() && !self.walk.substitutions(tokens, None))
        {
            return false;
        }
        self.bound.insert(name.to_owned());
        true
    }

    fn reading(&mut self, evaluate: impl FnOnce(&mut CompletionWalk<'_>) -> bool) -> bool {
        let first = self.walk.reads.len();
        evaluate(&mut self.walk)
            && self.walk.reads[first..]
                .iter()
                .all(|name| self.bound.contains(name))
    }
}

#[cfg(test)]
pub(super) mod tests {
    use std::collections::HashMap;

    use crate::compilation_unit::{CompilationUnit, UnitBuildOptions};

    const DIALECT: &str = "tcl8.6";

    pub(crate) fn logical_unit(
        source: &str,
        dialect: &str,
    ) -> (
        std::sync::Arc<tcl_registry::model::ContextRegistry>,
        CompilationUnit,
    ) {
        // Explicit conditional source model; this fixture grants no native
        // worker, actual entry, normal completion or executable replacement.
        let environment = tcl_registry::model::ingress::resolve_environment(dialect);
        let selected = environment.analyser_profile();
        let mut profile = tcl_dialect::DialectProfile::projected_from_point(
            "logical-procedure-completion-control",
            &[],
            "Logical procedure completion control",
            tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_79),
        );
        profile.runtime_base = selected.runtime_base;
        profile.grammar = selected.grammar;
        let profile = profile.intern();
        let context = environment.default_context_registry();
        let registry = context.commands();
        let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            std::sync::Arc::clone(&context),
            config,
        );
        assert!(input.has_logical_source_name_context());
        let declared = crate::analyser::utils::document_declared_surface(source, None, dialect);
        let entry = crate::command_binding::SourceAnalysisEntry {
            logical_source_input: Some(input.clone()),
            invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
            ..Default::default()
        };
        let unit = CompilationUnit::build_with_source_entry(
            source,
            UnitBuildOptions {
                registry,
                defer_top_level: false,
                config,
                dialect: Some(profile),
                external_call_sites: None,
                declared_commands: Some(&declared),
            },
            &entry,
        );
        (context, unit)
    }

    /// Conditional summaries under the actual source and selected grammar.
    fn summaries(
        source: &str,
        dialect: &str,
    ) -> HashMap<String, crate::interprocedural::ProcSummary> {
        let (context, unit) = logical_unit(source, dialect);
        let profile = unit.ir_module.dialect_profile;
        unit.with_interprocedural(context.commands(), profile)
            .interproc
            .expect("the summaries")
            .procedures
    }

    /// Whether each procedure of `source` completes, by its summary.
    fn completes(source: &str) -> HashMap<String, bool> {
        completes_under(source, DIALECT)
    }

    /// Whether each procedure of `source` completes under `dialect`.
    fn completes_under(source: &str, dialect: &str) -> HashMap<String, bool> {
        summaries(source, dialect)
            .into_iter()
            .map(|(qname, summary)| (qname, summary.completes))
            .collect()
    }

    /// Whether `procedure` is surely defined where `runner`'s calls run.
    fn defined_for(source: &str, runner: &str, procedure: &str) -> bool {
        summaries(source, DIALECT)[runner]
            .defined_callees
            .iter()
            .any(|name| name == procedure)
    }

    /// A procedure called before the `proc` statement that defines it has
    /// run raises `invalid command name`, so a callee counts as defined only
    /// where a direct top-level `proc` statement defines it before the load
    /// may first run the caller: the load runs `main` (as a statement,
    /// in a substitution, under `catch`, through `eval` or from a `namespace
    /// eval`) before `label`'s definition, or `label` is defined only under
    /// a condition, and `label` is not defined for `main`. Defined first, or
    /// with `main` run only by a callback, after the load, it is; so is a
    /// `namespace eval` body's direct definition, and the completion of a
    /// caller follows its callees' definitions.
    #[test]
    fn a_callee_counts_as_defined_only_where_its_definition_surely_ran() {
        // naming.interprocedural.original-procedure-completion-source-context
        // docs/design/analysis/name-resolution-proofs/interprocedural-original-procedure-completion-source-context.md
        // Conditional Logical model only; no Native invocation is asserted.
        let main = "proc main {} {set a [label abc]; return 1}\n";
        let label = "proc label {x} {return [string length $x]}\n";
        for run in [
            "main\n",
            "puts [main]\n",
            "puts [catch main]\n",
            "set s main\neval $s\n",
            "namespace eval ns {::main}\n",
            "puts [catch {time main}]\n",
        ] {
            let source = format!("{main}{run}{label}");
            assert!(
                !defined_for(&source, "::main", "::label"),
                "{source:?}: `main` runs before `label` is defined"
            );
        }
        let conditional = format!("if {{[info exists ::env(NOPE)]}} {{\n{label}}}\n{main}");
        assert!(!defined_for(&conditional, "::main", "::label"));
        let nested = format!("proc outer {{}} {{\n{label}}}\n{main}");
        assert!(!defined_for(&nested, "::main", "::label"));
        for source in [
            format!("{label}{main}main\n"),
            format!("{main}{label}"),
            format!("{main}{label}after 0 main\n"),
            format!("{main}namespace eval ns {{\n    {label}}}\n"),
        ] {
            let callee = if source.contains("namespace eval") {
                "::ns::label"
            } else {
                "::label"
            };
            let source = source.replace("[label abc]", &format!("[{callee} abc]"));
            assert!(
                defined_for(&source, "::main", callee),
                "{source:?}: `{callee}` is defined before `main` can run"
            );
        }
        let chained = format!(
            "proc wraps {{}} {{return [label abc]}}\nproc main {{}} {{return [wraps]}}\nmain\n{label}"
        );
        let completes = completes(&chained);
        assert!(completes["::label"], "`label` itself completes");
        assert!(
            !completes["::wraps"] && !completes["::main"],
            "`wraps` runs before `label` is defined: {completes:?}"
        );
    }

    /// A word the release's parser rejects is a compile error when the body
    /// is first compiled, so a body holding one never completes:
    /// content welded to a closing quote or brace under every release, and
    /// `{*}` under 8.4, which reads it as a braced word.
    #[test]
    fn a_word_the_release_rejects_never_completes() {
        // naming.interprocedural.original-procedure-completion-source-context
        // docs/design/analysis/name-resolution-proofs/interprocedural-original-procedure-completion-source-context.md
        // Conditional Logical model only; no Native invocation is asserted.
        for (body, dialects) in [
            (
                "return [string length \"a\"b]",
                &["tcl8.4", "tcl8.6", "tcl9.0"][..],
            ),
            (
                "return [string length {a}b]",
                &["tcl8.4", "tcl8.6", "tcl9.0"][..],
            ),
            ("string length {a}$x", &["tcl8.4", "tcl8.6", "tcl9.0"][..]),
            ("return [string length {*}$x]", &["tcl8.4"][..]),
        ] {
            let source = format!("proc lbl {{x}} {{{body}}}\n");
            for dialect in dialects {
                assert!(
                    !completes_under(&source, dialect)["::lbl"],
                    "{dialect}: {body:?} is a compile error"
                );
            }
        }
        let clean = "proc lbl {x} {return [string length \"a\"]}\n";
        assert!(completes_under(clean, "tcl8.4")["::lbl"]);
    }

    /// A straight-line body whose every command completes whatever its words
    /// hold completes, through the calls it makes to procedures that do: a
    /// literal return, an empty body, `string length` of a parameter or of a
    /// scalar the body set, and a call with the word count the callee takes.
    #[test]
    fn a_body_of_commands_that_complete_completes() {
        // naming.interprocedural.original-procedure-completion-source-context
        // docs/design/analysis/name-resolution-proofs/interprocedural-original-procedure-completion-source-context.md
        // Conditional Logical model only; no Native invocation is asserted.
        let completes = completes(
            "proc one {} {return 1}\n\
             proc empty {} {}\n\
             proc len {x} {return [string length $x]}\n\
             proc twice {x} {set n [string length $x]; set m $n; return $m$n}\n\
             proc wraps {y} {return [len $y]}\n\
             proc runs {y} {len $y}\n\
             proc chain {} {return [wraps [one]]}\n",
        );
        for name in [
            "::one", "::empty", "::len", "::twice", "::wraps", "::runs", "::chain",
        ] {
            assert!(completes[name], "{name} completes");
        }
    }

    /// A body that may raise does not complete: an expression, an `incr`, a
    /// registry command that states an error among its completions, a read of
    /// a variable the body never set, an element or a qualified store, an
    /// expansion, control flow, a link, a call whose word count the callee
    /// rejects, a callee that may raise, and a recursion, direct or mutual,
    /// which the nesting limit ends.
    #[test]
    fn a_body_that_may_raise_does_not_complete() {
        // naming.interprocedural.original-procedure-completion-source-context
        // docs/design/analysis/name-resolution-proofs/interprocedural-original-procedure-completion-source-context.md
        // Conditional Logical model only; no Native invocation is asserted.
        let completes = completes(
            "proc add {a b} {expr {$a + $b}}\n\
             proc sum {a} {return [expr {$a + 1}]}\n\
             proc bump {x} {incr x; return $x}\n\
             proc first {x} {return [lindex $x 0]}\n\
             proc unread {} {return $y}\n\
             proc element {} {set a(k) 1; return 1}\n\
             proc qualified {} {set ::g 1; return 1}\n\
             proc expand {l} {return [string length {*}$l]}\n\
             proc branch {x} {if {$x} {return 1}; return 0}\n\
             proc link {name} {upvar 1 $name v; return $v}\n\
             proc len {x} {return [string length $x]}\n\
             proc short {} {return [len]}\n\
             proc long {} {return [len a b]}\n\
             proc wraps {} {return [add 1 2]}\n\
             proc rec {n} {return [string length [rec $n]]}\n\
             proc ping {n} {return [pong $n]}\n\
             proc pong {n} {return [ping $n]}\n",
        );
        for name in [
            "::add",
            "::sum",
            "::bump",
            "::first",
            "::unread",
            "::element",
            "::qualified",
            "::expand",
            "::branch",
            "::link",
            "::short",
            "::long",
            "::wraps",
            "::rec",
            "::ping",
            "::pong",
        ] {
            assert!(!completes[name], "{name} may raise");
        }
        assert!(completes["::len"], "the callee itself completes");
    }

    /// A registry command's completion is the builtin's: a procedure of the
    /// module that takes its name answers for it, a `rename` leaves the name
    /// to another command, and a document's own declaration answers alone and
    /// states no completion — a stub's `-pure` says the command changes
    /// nothing, not that it completes.
    #[test]
    fn a_rebound_or_declared_head_states_no_completion() {
        // naming.interprocedural.original-procedure-completion-source-context
        // docs/design/analysis/name-resolution-proofs/interprocedural-original-procedure-completion-source-context.md
        // Conditional Logical model only; no Native invocation is asserted.
        let shadowed = completes(
            "proc string {args} {error boom}\nproc len {x} {return [string length $x]}\n",
        );
        assert!(
            !shadowed["::len"],
            "`string` is the module's procedure, which raises"
        );
        let renamed =
            completes("rename string _string\nproc len {x} {return [string length $x]}\n");
        assert!(!renamed["::len"], "`string` no longer names the builtin");
        let stub = "# tcl-lsp: stubs-begin\n# tcl-lsp: stub mylen {x} -pure\n# tcl-lsp: stubs-end\n\
                    proc len {x} {return [mylen $x]}\n";
        assert!(
            !completes(stub)["::len"],
            "a stub states purity, not completion"
        );
        let redeclared = "# tcl-lsp: stubs-begin\n# tcl-lsp: stub string {args} -pure\n\
                          # tcl-lsp: stubs-end\nproc len {x} {return [string length $x]}\n";
        assert!(
            !completes(redeclared)["::len"],
            "the document's declaration of `string` answers alone"
        );
    }

    /// The summary built from the IR alone holds no command trust, so it
    /// states no completion.
    #[test]
    fn the_ir_only_summary_states_no_completion() {
        // naming.interprocedural.original-procedure-completion-source-context
        // docs/design/analysis/name-resolution-proofs/interprocedural-original-procedure-completion-source-context.md
        // Conditional Logical model only; no Native invocation is asserted.
        let profile = tcl_registry::model::ingress::resolve_environment(DIALECT).analyser_profile();
        let registry = tcl_registry::model::ingress::static_context_for(DIALECT).commands();
        let ir = crate::lowering::lower_to_ir("proc one {} {return 1}\n", registry);
        let summaries = crate::interprocedural::build_interprocedural_analysis(
            &ir,
            registry,
            Some(profile),
            crate::interprocedural::ObjectTypeMap::none(),
            crate::realm::CommandBindingRealm::none(),
            None,
        );
        assert!(!summaries.procedures["::one"].completes);
        assert!(completes("proc one {} {return 1}\n")["::one"]);
    }
    #[test]
    fn original_completion_keeps_alias_prefixes_defaults_and_child_horizons() {
        // naming.interprocedural.original-procedure-completion-source-context
        // docs/design/analysis/name-resolution-proofs/interprocedural-original-procedure-completion-source-context.md
        // Source-model completion, independent of Native entry and edit licence.
        for source in [
            "proc leaf {{x VALUE}} {return $x}; proc p {} {return [leaf]}",
            "proc leaf {x y} {return $y}; interp alias {} fixed {} leaf FIRST; proc p {x} {return [fixed $x]}",
            "proc leaf {x} {return $x}; rename leaf moved; proc p {x} {return [moved $x]}",
            "interp alias {} length_of {} string length; proc p {x} {return [length_of $x]}",
            "proc leaf {x} {return $x}; proc p {} {return [leaf [string length VALUE]]}",
        ] {
            assert!(completes(source)["::p"], "{source}");
        }
        for source in [
            "proc leaf {x y} {return $y}; interp alias {} fixed {} leaf FIRST; proc p {} {return [fixed]}",
            "proc leaf {} {return OK}; proc other {} {error STOP}; interp alias {} called {} other; proc p {} {return [called]}",
            "proc leaf {x} {return $x}; proc p {} {return [leaf [error STOP]]}",
            "proc p {x} {return [string length $x]}; proc string args {error STOP}",
            "proc p {x} {return [unknown_worker $x]}",
        ] {
            assert!(!completes(source)["::p"], "{source}");
        }
    }

    #[test]
    fn original_completion_keeps_selected_scalar_reference_and_element_obligations() {
        // naming.interprocedural.original-procedure-completion-source-context
        // docs/design/analysis/name-resolution-proofs/interprocedural-original-procedure-completion-source-context.md
        for source in [
            "proc p {café} {return ${café}}",
            "proc p {{scalar(open}} {return ${scalar(open}}",
            "proc p {{scalar(k)tail}} {return ${scalar(k)tail}}",
            "proc p {{$literal}} {return ${$literal}}",
            "proc p {} {set {scalar(open} VALUE; return ${scalar(open}}",
        ] {
            assert!(completes(source)["::p"], "{source}");
        }
        for source in [
            "proc p {a} {return $a(k)}",
            "proc p {a} {return ${a(k)}}",
            "proc p {a i} {return $a($i)}",
            "proc p {} {return ${café}}",
        ] {
            assert!(!completes(source)["::p"], "{source}");
        }
        // The lexical obligation helper is tested independently here. These
        // spellings are not promoted into an original procedure source carrier.
        let (context8, unit8) = logical_unit("proc p {} {return OK}", "tcl8.6");
        let mut first_close =
            super::CompletionWalk::for_module(&unit8.ir_module, context8.commands(), true).unwrap();
        assert!(first_close.read("${a{b}"));
        assert_eq!(first_close.reads, ["a{b"]);
        let (context9, unit9) = logical_unit("proc p {} {return OK}", "tcl9.0");
        let mut nested =
            super::CompletionWalk::for_module(&unit9.ir_module, context9.commands(), true).unwrap();
        assert!(!nested.read("${a{b}"));
        assert!(nested.read("${a{b}c}"));
        assert_eq!(nested.reads, ["a{b}c"]);
    }

    #[test]
    fn original_completion_declines_missing_foreign_native_and_mutated_headers() {
        // naming.interprocedural.original-procedure-completion-source-context
        // docs/design/analysis/name-resolution-proofs/interprocedural-original-procedure-completion-source-context.md
        let source = "proc leaf {x} {return $x}; proc p {x} {return [leaf $x]}";
        let (context, unit) = logical_unit(source, DIALECT);
        let module = &unit.ir_module;
        assert!(super::procedures_complete(module, context.commands(), &|_, _| true)["::p"]);
        let mutations: &[fn(&mut crate::ir::Module)] = &[
            |module| {
                let procedure = module.procedures.remove("::leaf").unwrap();
                module.procedures.insert("::other".into(), procedure);
            },
            |module| module.source_metadata_input = None,
            |module| module.lexer_config.strict_quoting = !module.lexer_config.strict_quoting,
            |module| module.source = tcl_lexer::SourceImage::document("return OTHER"),
            |module| module.procedures.get_mut("::leaf").unwrap().qualified_name = "::other".into(),
            |module| module.procedures.get_mut("::leaf").unwrap().params_raw = "{x DEFAULT}".into(),
            |module| module.procedures.get_mut("::leaf").unwrap().params = vec!["y".into()],
            |module| module.procedures.get_mut("::leaf").unwrap().body_offset += 1,
            |module| {
                module.procedures.get_mut("::leaf").unwrap().body_source =
                    Some("return OTHER".into())
            },
            |module| {
                module
                    .procedures
                    .get_mut("::leaf")
                    .unwrap()
                    .body
                    .statements
                    .clear()
            },
        ];
        for change in mutations {
            let mut changed = module.clone();
            change(&mut changed);
            assert!(!super::procedures_complete(&changed, context.commands(), &|_, _| true)["::p"]);
        }
        let mut foreign_key = module.clone();
        let procedure = foreign_key.procedures.remove("::leaf").unwrap();
        foreign_key.procedures.insert("::other".into(), procedure);
        let summaries = super::procedures_complete(&foreign_key, context.commands(), &|_, _| true);
        assert!(!summaries["::other"]);
        assert!(!summaries["::p"]);
        let foreign =
            tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry();
        assert!(!super::procedures_complete(module, foreign.commands(), &|_, _| true)["::p"]);
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let context = tcl_registry::model::ingress::resolve_environment(dialect)
                .default_context_registry();
            let profile = context.commands().profile().unwrap();
            let input = crate::analyser::ResolvedAnalysisInput::new(
                profile,
                profile,
                std::sync::Arc::clone(&context),
                tcl_lexer::LexerConfig::for_profile(Some(profile)),
            );
            let unit = CompilationUnit::build_with_analysis_input(
                source,
                UnitBuildOptions {
                    registry: context.commands(),
                    defer_top_level: false,
                    config: input.lexer_config(),
                    dialect: Some(profile),
                    external_call_sites: None,
                    declared_commands: None,
                },
                None,
                &input,
            );
            assert!(
                super::procedures_complete(&unit.ir_module, context.commands(), &|_, _| true)
                    .values()
                    .all(|complete| !complete),
                "{dialect}: no Native entry/activation receipt"
            );
        }
    }

    #[test]
    fn original_completion_retains_selected_jim_formal_binding_strategy() {
        // naming.interprocedural.original-procedure-completion-source-context
        // docs/design/analysis/name-resolution-proofs/interprocedural-original-procedure-completion-source-context.md
        // Actual selected Jim syntax in an explicit Logical source model,
        // independently of native Jim activation and public normal completion.
        for (source, minimum, maximum, names) in [
            (
                "proc p {{x DEFAULT} y} {return $y}",
                1,
                Some(2),
                vec!["x", "y"],
            ),
            ("proc p {args y} {return $y}", 1, None, vec!["args", "y"]),
            (
                "proc p {{args tail} y} {return $tail}",
                1,
                None,
                vec!["tail", "y"],
            ),
        ] {
            let (context, unit) = logical_unit(source, "jim");
            let module = &unit.ir_module;
            assert_eq!(
                module.parameter_grammar(),
                Some(tcl_dialect::ParameterGrammar::Jim)
            );
            let procedure = &module.procedures["::p"];
            let shape = crate::registry_invocation::original_procedure_formal_count_shape(
                module,
                procedure,
                context.commands(),
            )
            .unwrap();
            assert_eq!((shape.minimum, shape.maximum), (minimum, maximum));
            let actual = crate::registry_invocation::original_procedure_scalar_bindings(
                module,
                procedure,
                context.commands(),
            )
            .unwrap();
            assert_eq!(actual, names.into_iter().map(str::to_owned).collect());
        }
        for source in [
            "proc p {&x} {return $x}",
            "proc p {{&x DEFAULT}} {return $x}",
            "proc p {{a(k)}} {return $a(k)}",
        ] {
            let (context, unit) = logical_unit(source, "jim");
            let procedure = &unit.ir_module.procedures["::p"];
            assert!(
                crate::registry_invocation::original_procedure_scalar_bindings(
                    &unit.ir_module,
                    procedure,
                    context.commands(),
                )
                .is_none()
            );
            assert!(
                !super::procedures_complete(&unit.ir_module, context.commands(), &|_, _| true,)["::p"]
            );
        }
    }
}
