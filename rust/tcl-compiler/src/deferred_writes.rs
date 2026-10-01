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

//! The variables the callback scripts of a module write.
//!
//! A command that stores a script to run after it returns — `after`,
//! `fileevent`, `bind`, a variable trace — leaves the write in that script
//! outside the control flow of the code that registered it: `set done 0;
//! after 100 { set done 1 }; while {!$done} { update }` never sees the store
//! that ends its loop. The script runs at the global level, or in the frame
//! of whatever fires it, so a plain name in it can be a variable the
//! registering function holds.
//!
//! Which words are such scripts is the registry's answer
//! ([`CommandRegistry::callback_script_indices`]): the words it states as
//! deferred, less the body of a definition, which runs in a frame of its own.
//! This module holds no command name. It reads each such word's text as the
//! script it is, and collects every name a command of it writes, destroys or
//! binds — the variable-name roles the registry gives each command, and the
//! aliases a `global`, `upvar` or `variable` declares. A callback whose
//! command is a procedure of the module (`after 100 tick`) writes what that
//! procedure writes in the global frame
//! ([`crate::cfg_builder::global_write_info`]).
//!
//! A callback word that is one `[…]` substitution of a command the registry
//! states builds a command prefix (`after 100 [list tick $n]`, `-command [list
//! set done 1]`) is read as the command it builds. What it does not read: a
//! callback word that is otherwise computed (`after 100 $script`), a callback
//! spelled as several words, which the registry states no script position for
//! (`after 100 set done 1`), or one whose command head is computed
//! (`after 100 {$cmd x}`). The text is unknown here, as a call to a procedure
//! the module cannot see is.

use std::cell::OnceCell;
use std::collections::HashMap;

use tcl_lexer::LexerConfig;
use tcl_registry::{
    ArgRole, CommandRegistry, InvocationWord, InvocationWords, StateTransition, Traits,
    TransitionSubject, VariableAliasTarget,
};
use tcl_syntax::word_rules::WordValueRules;

use crate::cfg_builder::global_write_info::{
    GlobalWriteInfo, detect_global_write_procs_with_registry,
};
use crate::depth_guard::MAX_BRACKET_TEXT_DEPTH;
use crate::ir::{DeferredWrites, Module, Script, Statement, WordExpr};
use crate::ir_helpers::{CommandWord, evaluated_command_substitutions, tokenise_command_words};
use crate::registry_invocation::EffectiveInvocationWord;

/// The names the callback scripts of `module` write, destroy or bind.
///
/// Read over every statement of the top-level script, every procedure, every
/// method and every `namespace eval` / `apply` body, and over each `[…]`
/// substitution one of their words carries: a callback registered as
/// `set id [after 100 { … }]` is as real as a bare one.
#[must_use]
pub(crate) fn scan_module(module: &Module, registry: &CommandRegistry) -> DeferredWrites {
    let mut scan = Scan {
        module,
        registry,
        config: LexerConfig::for_profile(registry.profile()),
        procedure_writes: OnceCell::new(),
        out: DeferredWrites::default(),
    };
    let procedures = module.procedures.values().map(|proc| &proc.body);
    let methods = module.methods.values().map(|method| &method.body);
    let redefined = module
        .redefined_methods
        .values()
        .flatten()
        .map(|method| &method.body);
    let units = module.body_units.values().map(|unit| &unit.body);
    for script in std::iter::once(&module.top_level)
        .chain(procedures)
        .chain(methods)
        .chain(redefined)
        .chain(units)
    {
        scan.registrations(script);
    }
    scan.out
}

/// One word of an invocation, as far as the scan can read it.
#[derive(Clone)]
enum Arg {
    /// A word whose value is this text.
    Literal(String),
    /// A word that is one `[…]` substitution, holding the command it runs.
    Substitution(String),
    /// One word the run time computes.
    Dynamic,
}

impl Arg {
    fn literal(&self) -> Option<&str> {
        match self {
            Self::Literal(text) => Some(text),
            Self::Substitution(_) | Self::Dynamic => None,
        }
    }

    fn registry_word(&self) -> InvocationWord<'_> {
        match self {
            Self::Literal(text) => InvocationWord::Literal(text),
            Self::Substitution(_) | Self::Dynamic => InvocationWord::Dynamic,
        }
    }
}

struct Scan<'r> {
    module: &'r Module,
    registry: &'r CommandRegistry,
    config: LexerConfig,
    /// What each procedure of the module writes in the global frame, taken
    /// the first time a callback names one.
    procedure_writes: OnceCell<HashMap<String, GlobalWriteInfo>>,
    out: DeferredWrites,
}

impl Scan<'_> {
    /// Every command `script` runs, its nested bodies and substitutions
    /// included, for the callback words among its own.
    fn registrations(&mut self, script: &Script) {
        crate::ir::for_each_statement(script, &mut |stmt| self.statement(stmt));
    }

    fn statement(&mut self, stmt: &Statement) {
        if let Statement::Call {
            command,
            canonical_command,
            args,
            tokens,
            ..
        }
        | Statement::Barrier {
            command,
            canonical_command,
            args,
            tokens,
            ..
        } = stmt
            && tokens
                .as_ref()
                .is_none_or(|tokens| tokens.synthetic.is_none())
        {
            let head = canonical_command.as_deref().unwrap_or(command.as_str());
            let words: Vec<Option<Arg>> = match tokens {
                Some(tokens) if tokens.words_align_with_argv_text() => tokens
                    .words()
                    .iter()
                    .skip(1)
                    .map(|word| self.source_word(word))
                    .collect(),
                _ => args
                    .iter()
                    .map(|arg| Some(Arg::Literal(arg.clone())))
                    .collect(),
            };
            self.registration(head, &words);
        }
        for words in evaluated_command_substitutions(stmt, self.registry).all_commands() {
            self.embedded(words);
        }
    }

    /// One word of a statement as written: its value when the source states
    /// one, the command a lone `[…]` substitution runs, and `None` for a `{*}`
    /// expansion or a word the token snapshot does not represent.
    fn source_word(&self, word: &WordExpr) -> Option<Arg> {
        if let WordExpr::CommandSubstitution { spelling, .. } = word {
            return Some(Arg::Substitution(
                spelling
                    .strip_prefix('[')
                    .map_or(spelling.as_str(), |rest| {
                        rest.strip_suffix(']').unwrap_or(rest)
                    })
                    .to_owned(),
            ));
        }
        match crate::registry_invocation::effective_invocation_word(
            word,
            self.config.escapes,
            WordValueRules::from_config(&self.config),
        ) {
            EffectiveInvocationWord::Literal(text) => Some(Arg::Literal(text)),
            EffectiveInvocationWord::Dynamic => Some(Arg::Dynamic),
            EffectiveInvocationWord::Expanded | EffectiveInvocationWord::Opaque => None,
        }
    }

    /// A command a `[…]` substitution of a statement runs.
    fn embedded(&mut self, words: &[CommandWord]) {
        let Some(head) = words.first().and_then(CommandWord::literal) else {
            return;
        };
        let args: Vec<Option<Arg>> = words
            .iter()
            .skip(1)
            .map(|word| self.substitution_word(word))
            .collect();
        self.registration(head, &args);
    }

    /// One word of a command read from a script's text, as [`Self::source_word`]
    /// reads one of a statement.
    fn substitution_word(&self, word: &CommandWord) -> Option<Arg> {
        if word.expanded {
            return None;
        }
        if let Some(text) = word.literal() {
            return Some(Arg::Literal(text.to_owned()));
        }
        // The word's written span stops at the last character inside a
        // delimiter, so its closing bracket may be absent.
        if let Some(rest) = word.raw.trim().strip_prefix('[') {
            let inner = rest.strip_suffix(']').unwrap_or(rest);
            let texts = crate::var_refs::command_subst_texts_with_config(
                &format!("[{inner}]"),
                self.config,
            );
            if texts.len() == 1 && texts[0] == inner {
                return Some(Arg::Substitution(inner.to_owned()));
            }
        }
        Some(Arg::Dynamic)
    }

    /// The callback words of one invocation, read as the scripts they are.
    /// A `None` word is a `{*}` expansion, whose argument count is unknown, so
    /// no position of the invocation can be told from another.
    fn registration(&mut self, head: &str, words: &[Option<Arg>]) {
        let Some(args) = words.iter().cloned().collect::<Option<Vec<Arg>>>() else {
            return;
        };
        let spellings: Vec<&str> = args
            .iter()
            .map(|arg| arg.literal().unwrap_or_default())
            .collect();
        for index in self.registry.callback_script_indices(
            head,
            &spellings,
            self.registry.own_surface_query(),
        ) {
            match args.get(index) {
                Some(Arg::Literal(text)) => self.script(text, 0),
                Some(Arg::Substitution(inner)) => self.command_prefix(inner, 0),
                Some(Arg::Dynamic) | None => {}
            }
        }
    }

    /// A callback word that is `[cmd …]` holds the command `cmd` builds when the
    /// registry states it builds a command prefix (`[list tick $n]`): the
    /// words after `cmd` are that command.
    fn command_prefix(&mut self, text: &str, depth: u32) {
        let commands = tokenise_command_words(text, self.config);
        let [words] = commands.as_slice() else {
            return;
        };
        let Some(builder) = words.first().and_then(CommandWord::literal) else {
            return;
        };
        if words.iter().skip(1).any(|word| word.expanded) {
            return;
        }
        let spellings: Vec<&str> = words
            .iter()
            .skip(1)
            .map(|word| word.literal().unwrap_or_default())
            .collect();
        if self
            .registry
            .invocation_traits(builder, &spellings, self.registry.own_surface_query())
            .contains(Traits::BUILDS_COMMAND_PREFIX)
        {
            self.command(&words[1..], depth);
        }
    }

    /// The names every command of `text`, run as a script, writes.
    fn script(&mut self, text: &str, depth: u32) {
        if MAX_BRACKET_TEXT_DEPTH.exceeded(depth) {
            self.out.any = true;
            return;
        }
        for words in tokenise_command_words(text, self.config) {
            self.command(&words, depth);
        }
    }

    /// The names one command of a callback script writes, destroys or binds,
    /// and the scripts and substitutions its words carry.
    fn command(&mut self, words: &[CommandWord], depth: u32) {
        // A substitution runs whatever the command is, the head's included.
        for word in words.iter().filter(|word| word.substituted) {
            for inner in crate::var_refs::command_subst_texts_with_config(&word.raw, self.config) {
                self.script(&inner, depth + 1);
            }
        }
        let Some(head) = words.first().and_then(CommandWord::literal) else {
            return;
        };
        self.procedure(head);
        if words.iter().skip(1).any(|word| word.expanded) {
            return;
        }
        let args: Vec<Arg> = words
            .iter()
            .skip(1)
            .map(|word| {
                word.literal()
                    .map_or(Arg::Dynamic, |text| Arg::Literal(text.to_owned()))
            })
            .collect();
        let spellings: Vec<&str> = args
            .iter()
            .map(|arg| arg.literal().unwrap_or_default())
            .collect();
        let inputs: Vec<InvocationWord<'_>> = args.iter().map(Arg::registry_word).collect();
        self.variable_targets(head, &args, &spellings, &inputs);
        self.aliases(head, &inputs);
        // A definition's body runs in a frame of its own, whose plain names
        // are its locals; anything else in this script runs where it does.
        let invocation =
            self.registry
                .invocation_traits(head, &spellings, self.registry.own_surface_query());
        if !invocation.contains(Traits::BODY_RUNS_IN_OWN_FRAME) {
            for index in self
                .registry
                .arg_indices_for_role(head, &spellings, ArgRole::Body)
            {
                if let Some(body) = args.get(index).and_then(Arg::literal) {
                    self.script(body, depth + 1);
                }
            }
        }
    }

    /// A callback whose command is a procedure of the module runs its body:
    /// what the body writes in the global frame changes when the callback
    /// fires.
    fn procedure(&mut self, head: &str) {
        let is_procedure = self.module.procedures.keys().any(|qualified| {
            crate::cfg_builder::qualified_lookup_keys(qualified)
                .iter()
                .any(|key| key == head)
        });
        if !is_procedure {
            return;
        }
        let writes = self
            .procedure_writes
            .get_or_init(|| detect_global_write_procs_with_registry(self.module, self.registry));
        let Some(info) = writes.get(head) else {
            return;
        };
        self.out.any |= info.opaque_global_frame;
        let names = info.names.clone();
        for name in &names {
            self.note(name);
        }
    }

    /// The variables the registry says a command writes, destroys, binds or
    /// iterates: every word in a variable-name role, and the value writes its
    /// resolution states.
    fn variable_targets(
        &mut self,
        head: &str,
        args: &[Arg],
        spellings: &[&str],
        inputs: &[InvocationWord<'_>],
    ) {
        for index in self
            .registry
            .arg_indices_for_role(head, spellings, ArgRole::VarWrite)
        {
            match args.get(index) {
                Some(Arg::Literal(name)) => self.note(name),
                Some(Arg::Dynamic | Arg::Substitution(_)) => self.out.any = true,
                None => {}
            }
        }
        let projection = self
            .registry
            .variable_write_projection(InvocationWords::structured(
                InvocationWord::Literal(head),
                inputs,
            ));
        for name in &projection.literal_names {
            self.note(name);
        }
        self.out.any |= projection.opaque_variable_frame;
        for index in self
            .registry
            .arg_indices_for_role(head, spellings, ArgRole::LoopVarList)
        {
            match args.get(index) {
                Some(Arg::Literal(list)) => {
                    let rules =
                        tcl_syntax::word_rules::WordValueRules::of_profile(self.registry.profile());
                    match rules.split_list(list) {
                        Ok(names) => names.iter().for_each(|name| self.note(name)),
                        Err(_) => self.out.any = true,
                    }
                }
                Some(Arg::Dynamic | Arg::Substitution(_)) => self.out.any = true,
                None => {}
            }
        }
    }

    /// The local and the cell of each alias a command declares (`global`,
    /// `upvar`, `variable`, `namespace upvar`): a write through the local is a
    /// write of the cell.
    fn aliases(&mut self, head: &str, inputs: &[InvocationWord<'_>]) {
        let words = InvocationWords::structured(InvocationWord::Literal(head), inputs);
        let Some(resolved) = self
            .registry
            .resolve_structured_invocation(words, self.registry.own_surface_query())
            .resolved()
        else {
            return;
        };
        for fact in resolved.state_transitions().facts() {
            let StateTransition::VariableCellAlias(alias) = &fact.transition else {
                continue;
            };
            self.subject(&alias.local);
            match &alias.target {
                VariableAliasTarget::Global { variable }
                | VariableAliasTarget::CurrentNamespace { variable }
                | VariableAliasTarget::CallerSelectedFrame { variable, .. }
                | VariableAliasTarget::Namespace { variable, .. } => self.subject(variable),
            }
        }
    }

    fn subject(&mut self, subject: &TransitionSubject) {
        match subject {
            TransitionSubject::Literal(name) => self.note(name),
            TransitionSubject::Unknown { .. } => self.out.any = true,
        }
    }

    /// Record a name: without a leading `::`, and with an element's array.
    fn note(&mut self, name: &str) {
        let name = name.strip_prefix("::").unwrap_or(name);
        if name.is_empty() {
            return;
        }
        self.out.names.insert(name.to_owned());
        let base = crate::sccp::place_base(name);
        if base != name {
            self.out.names.insert(base.to_owned());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn writes(source: &str) -> DeferredWrites {
        let registry = CommandRegistry::build_default();
        let module = crate::lowering::lower_to_ir_with_dialect(
            source,
            &registry,
            LexerConfig::for_profile(registry.profile()),
            registry.profile(),
        );
        scan_module(&module, &registry)
    }

    fn names(source: &str) -> Vec<String> {
        writes(source).names.into_iter().collect()
    }

    /// The callbacks `after`, `after idle`, `trace`, `bind`, `fileevent`,
    /// `chan event` and `interp bgerror` each write a name the scan reports,
    /// the leading `::` stripped.
    #[test]
    fn a_callback_script_names_what_it_writes() {
        for source in [
            "after 100 { set done 1 }",
            "after idle {set ::done 1}",
            "trace add variable x write { set ::done 1 ;# }",
            "bind . <Key> { set done 1 }",
            "fileevent stdin readable { set done 1 }",
            "chan event stdin readable { set done 1 }",
            "interp bgerror {} { set done 1 ;# }",
            "set id [after 100 { set done 1 }]",
            "proc p {} { after 100 { set done 1 } }",
        ] {
            assert_eq!(names(source), ["done"], "{source}");
        }
    }

    /// Whatever the callback writes counts — a command's variable-name words,
    /// a nested body, a substitution, an element, and the `global`, `upvar`
    /// and `variable` a script declares.
    #[test]
    fn a_callback_writes_through_every_shape_of_command() {
        assert_eq!(
            names("after 1 { incr n; append s x; lappend l 1; unset u; array set a {k v} }"),
            ["a", "l", "n", "s", "u"]
        );
        assert_eq!(names("after 1 { if {1} { set inner 1 } }"), ["inner"]);
        assert_eq!(names("after 1 { puts [set got 1] }"), ["got"]);
        assert_eq!(names("after 1 { set e(k) 1 }"), ["e", "e(k)"]);
        assert_eq!(names("after 1 { global g; set g 1 }"), ["g"]);
        assert_eq!(
            names("after 1 { upvar #0 target alias; set alias 1 }"),
            ["alias", "target"]
        );
        assert_eq!(
            names("after 1 { foreach {k v} $l { puts $k } }"),
            ["k", "v"]
        );
    }

    /// A definition's body runs in a frame of its own, and a script that only
    /// reads, or is not stored, writes nothing a callback could change.
    #[test]
    fn a_definition_a_read_and_a_script_run_now_are_no_callback_write() {
        assert!(writes("proc p {} { set local 1 }").is_clear());
        assert!(writes("after 1 { proc p {} { set local 1 } }").is_clear());
        assert!(writes("after 1 { puts $x }").is_clear());
        assert!(writes("catch { set x 1 }").is_clear());
        assert!(writes("trace remove variable x write cb").is_clear());
        assert!(writes("set script {set x 1}; after 1 $script").is_clear());
    }

    /// A callback that names a procedure of the module writes what the
    /// procedure writes in the global frame, however it names the variable;
    /// what it writes in its own frame is its own.
    #[test]
    fn a_callback_that_names_a_procedure_writes_what_the_procedure_writes_globally() {
        assert_eq!(
            names("proc tick {} { global done; set done 1 }\nafter 100 tick"),
            ["done"]
        );
        assert_eq!(
            names("proc tick {} { set ::done 1 }\nafter idle tick"),
            ["done"]
        );
        assert_eq!(
            names("proc tick {} { upvar #0 total t; incr t }\nfileevent stdin readable tick"),
            ["total"]
        );
        assert_eq!(
            names("proc tick {} { set ::done 1 }\nafter 1 { tick }"),
            ["done"]
        );
        assert!(writes("proc tick {} { set local 1 }\nafter 100 tick").is_clear());
        assert!(writes("proc tick {} { set ::done 1 }\ntick").is_clear());
    }

    /// A callback spelled as a quoted word with no substitution, or as one
    /// `[…]` substitution of a command the registry states builds a command
    /// prefix, is read as the script it is; a word computed some other way, or
    /// a callback spelled as several words, is not.
    #[test]
    fn a_quoted_callback_and_a_built_command_prefix_are_read() {
        for source in [
            "after 100 \"set done 1\"",
            "after 100 [list set done 1]",
            "after 100 [list set done $value]",
            "button .b -command [list set done 1]",
            "set id [after 100 [list set done 1]]",
            "proc p {} { after 100 [list set ::done 1] }",
        ] {
            assert_eq!(names(source), ["done"], "{source}");
        }
        assert_eq!(
            names("proc tick {n} { set ::done $n }\nafter 100 [list tick $n]"),
            ["done"]
        );
        for source in [
            "after 100 $script",
            "after 100 [build $n]",
            "after 100 set done 1",
        ] {
            assert!(writes(source).is_clear(), "{source}");
        }
    }

    /// A callback that writes a name it computes may change any variable.
    #[test]
    fn a_computed_name_in_a_callback_may_write_any_variable() {
        assert!(writes("after 1 { set $n 1 }").any);
        assert!(!writes("after 1 { set n 1 }").any);
    }
}
