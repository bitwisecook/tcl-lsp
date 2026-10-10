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

//! Whether a call to a procedure completes normally whatever its arguments
//! hold ([`super::ProcSummary::completes`]), and the walk over a word's
//! substitutions that the summary and the dead-store passes' raise proof
//! share.
//!
//! A store whose value runs a command can be deleted only where the command
//! cannot raise, and purity says a call changes nothing, not that it
//! completes. So the summary states completion apart: a
//! procedure completes where its body is straight-line — assignments to its
//! own scalars, calls and a `return` — every variable it reads is a parameter
//! or a scalar an earlier statement set, and every command it runs completes
//! whatever its words hold: a registry command whose resolved invocation
//! declares the normal completion alone, under a word count its arity
//! accepts, or a procedure of the module that completes in turn, called with
//! a word count its parameters accept. A recursion never completes, since the
//! interpreter's nesting limit can end it; for a call that does not recurse
//! that limit is set aside, as every fold sets it aside.

use std::collections::{HashMap, HashSet};

use tcl_registry::completion::{CompletionCode, CompletionCodeDomain};

use crate::command_binding::ModuleCommandMutations;
use crate::depth_guard::MAX_BRACKET_TEXT_DEPTH;
use crate::ir::{CommandTokens, Script, Statement, WordExpr, WordPart};

/// Whether a command head that names no procedure of the module, with its
/// arguments' spellings, is a registry command that completes normally
/// whatever its words hold.
pub(crate) type RegistryCompletes<'w> = &'w dyn Fn(&str, &[&str]) -> bool;

/// A walk over the words a statement evaluates: it collects the procedures
/// of the module the commands it substitutes call and the variables it
/// reads, and fails on a command that may raise whatever its words hold.
pub(crate) struct CompletionWalk<'w> {
    /// The grammar the words were lexed under.
    config: tcl_lexer::LexerConfig,
    /// The procedure of the module a command head names, where it names one.
    procedure: &'w dyn Fn(&str) -> Option<String>,
    /// What answers for a head that names no procedure of the module;
    /// `None` takes no registry command.
    registry: Option<RegistryCompletes<'w>>,
    /// Each procedure of the module a command calls, with the number of
    /// words after its head.
    pub(crate) calls: Vec<(String, usize)>,
    /// Each variable a word reads, as it names it.
    pub(crate) reads: Vec<String>,
}

impl<'w> CompletionWalk<'w> {
    /// A walk resolving heads through `procedure`, taking a registry command
    /// where `registry` answers for it.
    pub(crate) fn new(
        config: tcl_lexer::LexerConfig,
        procedure: &'w dyn Fn(&str) -> Option<String>,
        registry: Option<RegistryCompletes<'w>>,
    ) -> Self {
        Self {
            config,
            procedure,
            registry,
            calls: Vec::new(),
            reads: Vec::new(),
        }
    }

    /// Whether evaluating `word` cannot raise where every variable it reads
    /// is set as a scalar and every procedure it calls completes: its text is
    /// literal or substitutes a scalar variable or a command that completes.
    /// An element read, an expansion, a word the lexer could not model and a
    /// word the release's parser rejects may raise.
    pub(crate) fn word(&mut self, word: &WordExpr) -> bool {
        self.word_at(word, 0)
    }

    /// Whether running the command `words` spell cannot raise on those
    /// terms ([`Self::word`]).
    pub(crate) fn command(&mut self, words: &[WordExpr]) -> bool {
        self.command_at(words, 0)
    }

    fn word_at(&mut self, word: &WordExpr, depth: u32) -> bool {
        match word {
            WordExpr::Literal { .. } | WordExpr::BracedLiteral { .. } => true,
            WordExpr::Variable { spelling, .. } => self.read(spelling),
            WordExpr::CommandSubstitution { spelling, .. } => self.substitution(spelling, depth),
            WordExpr::Template {
                rejected: Some(_), ..
            }
            | WordExpr::Expand { .. }
            | WordExpr::Opaque { .. } => false,
            WordExpr::Template { parts, .. } => parts.iter().all(|part| match part {
                WordPart::Text { .. } => true,
                WordPart::Variable { spelling, .. } => self.read(spelling),
                WordPart::CommandSubstitution { spelling, .. } => {
                    self.substitution(spelling, depth)
                }
                WordPart::Opaque { .. } => false,
            }),
        }
    }

    /// A variable substitution: a whole scalar name, recorded as read.
    fn read(&mut self, spelling: &str) -> bool {
        crate::value_shapes::whole_word_scalar_var_name(spelling)
            .map(|name| self.reads.push(name.to_owned()))
            .is_some()
    }

    /// A command substitution: each command of its script, every one parsed
    /// whole.
    fn substitution(&mut self, spelling: &str, depth: u32) -> bool {
        if MAX_BRACKET_TEXT_DEPTH.exceeded(depth) {
            return false;
        }
        let Some(script) = spelling
            .strip_prefix('[')
            .and_then(|inner| inner.strip_suffix(']'))
        else {
            return false;
        };
        let config = self.config;
        let commands = crate::segmenter::segment_commands_with_offset_and_config(script, 0, config);
        if commands.iter().any(|command| command.is_partial) {
            return false;
        }
        let map = tcl_lexer::SourceMap::new(script);
        commands.iter().all(|command| {
            let tokens = CommandTokens::from_segmented(&map, config, command);
            self.command_at(&tokens.word_exprs, depth + 1)
        })
    }

    fn command_at(&mut self, words: &[WordExpr], depth: u32) -> bool {
        let Some((head, arguments)) = words.split_first() else {
            return true;
        };
        let (WordExpr::Literal { text, .. } | WordExpr::BracedLiteral { text, .. }) = head else {
            return false;
        };
        if arguments
            .iter()
            .any(|word| matches!(word, WordExpr::Expand { .. }))
        {
            return false;
        }
        if let Some(callee) = (self.procedure)(text) {
            self.calls.push((callee, arguments.len()));
        } else {
            let spellings: Vec<String> = arguments.iter().map(WordExpr::legacy_text).collect();
            let spellings: Vec<&str> = spellings.iter().map(String::as_str).collect();
            if !self
                .registry
                .is_some_and(|completes| completes(text, &spellings))
            {
                return false;
            }
        }
        arguments.iter().all(|word| self.word_at(word, depth))
    }
}

/// Whether each procedure of `ir_module` completes normally whatever its
/// arguments hold, for a call whose word count its parameters accept:
/// a least fixpoint over the calls each straight-line body makes, so
/// a recursion never completes. The registry commands a body runs are read
/// from the document's command surface as the module leaves them: the module
/// must trust the head's builtin binding, and a command the document
/// declares answers alone, with no completion stated. A procedure a body
/// calls must be defined where the body runs: `defined` answers
/// whether a callee's `proc` statement surely runs before the load may first
/// run the caller.
pub(super) fn procedures_complete(
    ir_module: &crate::ir::Module,
    surface: tcl_registry::model::DocumentCommandSurface<'_>,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    mutations: &ModuleCommandMutations,
    defined: &dyn Fn(&str, &str) -> bool,
) -> HashMap<String, bool> {
    let config = tcl_lexer::LexerConfig::for_profile(dialect);
    let registry = surface.commands();
    let registry_completes = |head: &str, arguments: &[&str]| {
        !surface.declares(head)
            && mutations.trusts(head)
            && registry
                .resolve_invocation(head, arguments, registry.own_surface_query())
                .is_some_and(|invocation| {
                    let semantics = invocation.semantics;
                    matches!(
                        semantics.completion.codes,
                        CompletionCodeDomain::Exact(codes) if codes == [CompletionCode::Ok]
                    ) && arguments
                        .len()
                        .checked_sub(semantics.argument_offset)
                        .and_then(|count| u16::try_from(count).ok())
                        .is_some_and(|count| semantics.arity.accepts(count))
                })
    };
    let trusts = |name: &str| mutations.trusts(name);
    let calls: HashMap<&str, Vec<(String, usize)>> = ir_module
        .procedures
        .iter()
        .filter_map(|(qname, proc)| {
            let procedure = |head: &str| {
                super::resolve_internal_call_with(head, qname, |candidate| {
                    ir_module.procedures.contains_key(candidate)
                })
            };
            let mut scan = BodyScan {
                walk: CompletionWalk::new(config, &procedure, Some(&registry_completes)),
                bound: proc.params.iter().cloned().collect(),
                trusts: &trusts,
            };
            scan.script(&proc.body, 0)
                .then_some((qname.as_str(), scan.walk.calls))
        })
        .collect();
    let stands = |qname: &str| {
        !ir_module.redefined_procedures.contains(qname) && mutations.trusts_proc_binding(qname)
    };
    let mut completes: HashSet<&str> = HashSet::new();
    loop {
        let before = completes.len();
        for (&qname, callees) in &calls {
            if completes.contains(qname) {
                continue;
            }
            let each_completes = callees.iter().all(|(callee, count)| {
                completes.contains(callee.as_str())
                    && stands(callee)
                    && defined(callee, qname)
                    && ir_module.procedures.get(callee).is_some_and(|proc| {
                        u16::try_from(*count)
                            .is_ok_and(|count| super::arity_from_names(&proc.params).accepts(count))
                    })
            });
            if each_completes {
                completes.insert(qname);
            }
        }
        if completes.len() == before {
            break;
        }
    }
    ir_module
        .procedures
        .keys()
        .map(|qname| (qname.clone(), completes.contains(qname.as_str())))
        .collect()
}

/// A straight-line body's statements in order, with the scalars set so far.
struct BodyScan<'s, 'w> {
    walk: CompletionWalk<'w>,
    /// The parameters, and each scalar an earlier statement set.
    bound: HashSet<String>,
    /// Whether the module trusts a command's builtin binding: the typed
    /// statements mean `set` and `return` only while it does.
    trusts: &'s dyn Fn(&str) -> bool,
}

impl BodyScan<'_, '_> {
    /// Whether every statement of `script` completes normally.
    fn script(&mut self, script: &Script, depth: u32) -> bool {
        if super::MAX_INTERPROCEDURAL_WALK_DEPTH.exceeded(depth) {
            return false;
        }
        script
            .statements
            .iter()
            .all(|statement| self.statement(statement, depth))
    }

    /// Whether `statement` completes normally: an assignment to one of the
    /// procedure's own scalars, a call, or a `return` — none an expression,
    /// an `incr`, a control-flow command or a barrier, each of which may
    /// raise on what it reads.
    fn statement(&mut self, statement: &Statement, depth: u32) -> bool {
        if statement.synthetic_marker().is_some() {
            return false;
        }
        match statement {
            Statement::Block { body, .. } => self.script(body, depth + 1),
            Statement::AssignConst { name, .. } => self.assign(name, None),
            Statement::AssignValue { name, tokens, .. } => {
                let Some([_, _, value]) =
                    tokens.as_ref().map(|tokens| tokens.word_exprs.as_slice())
                else {
                    return false;
                };
                self.assign(name, Some(value))
            }
            Statement::Call {
                tokens: Some(tokens),
                foreach_groups: None,
                ..
            } => self.reading(|walk| walk.command(&tokens.word_exprs)),
            Statement::Return { value: None, .. } => (self.trusts)("return"),
            Statement::Return {
                expr: None,
                value_word: Some(word),
                ..
            } => (self.trusts)("return") && self.reading(|walk| walk.word(word)),
            _ => false,
        }
    }

    /// An assignment of `value` (none for a literal) to `name`: a plain
    /// scalar of the procedure's own frame, under the builtin `set`.
    fn assign(&mut self, name: &str, value: Option<&WordExpr>) -> bool {
        if !(self.trusts)("set")
            || crate::naming::normalise_var_name(name) != name
            || name.contains("::")
            || value.is_some_and(|value| !self.reading(|walk| walk.word(value)))
        {
            return false;
        }
        self.bound.insert(name.to_owned());
        true
    }

    /// Whether `evaluate` succeeds and reads only scalars already set.
    fn reading(&mut self, evaluate: impl FnOnce(&mut CompletionWalk<'_>) -> bool) -> bool {
        let first = self.walk.reads.len();
        evaluate(&mut self.walk)
            && self.walk.reads[first..]
                .iter()
                .all(|name| self.bound.contains(name))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::compilation_unit::{CompilationUnit, UnitBuildOptions};

    const DIALECT: &str = "tcl8.6";

    /// Each procedure's summary of `source` under `dialect`, in a unit built
    /// as the optimiser builds one: the document's own declarations reach the
    /// lowering and the summary.
    fn summaries(
        source: &str,
        dialect: &str,
    ) -> HashMap<String, crate::interprocedural::ProcSummary> {
        let profile = tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile();
        let registry = tcl_registry::model::ingress::static_context_for(dialect).commands();
        let declared = crate::analyser::utils::document_declared_surface(source, None, dialect);
        CompilationUnit::build_with_options(
            source,
            UnitBuildOptions {
                registry,
                defer_top_level: false,
                config: tcl_lexer::LexerConfig::for_profile(Some(profile)),
                dialect: Some(profile),
                external_call_sites: None,
                declared_commands: Some(&declared),
            },
        )
        .with_interprocedural(registry, Some(profile))
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
}
