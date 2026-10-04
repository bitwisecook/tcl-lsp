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
//! set done 1]`) is read as the command it builds, and so is a command prefix
//! the registry places at a word that more words follow (`interp alias {} safe
//! {} string length` runs `string length` with the caller's words appended).
//! A callback spelled as several words, which the registry names no position
//! for (`after 100 set done 1`), is read as the one script they concatenate
//! into.
//!
//! A callback the scan cannot read may write any variable, as a trace on a
//! computed name may ([`DeferredWrites::any`]): a word the run time computes
//! (`after 100 $script`), a substitution of any other command (`after 100
//! [build]`), a `{*}` expansion, a command whose head is computed
//! (`after 100 {$cmd x}`), and a command that is neither a procedure of the
//! module nor one of the registry (`after 100 finish`, an `interp alias`),
//! whose code the module does not contain.

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
use crate::command_binding::ModuleCommandBindings;
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
        bindings: OnceCell::new(),
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

/// The text of each word that has one, and nothing for the rest.
fn spellings_of(args: &[Arg]) -> Vec<&str> {
    args.iter()
        .map(|arg| arg.literal().unwrap_or_default())
        .collect()
}

/// `concat`: each word trimmed of surrounding white space, the empty ones
/// dropped, the rest joined with one space.
fn concat_words(words: &[&str]) -> String {
    words
        .iter()
        .map(|word| word.trim_matches([' ', '\t', '\n', '\u{b}', '\u{c}', '\r']))
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
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
    /// What the module binds each command name to, taken the first time a
    /// callback command that is no procedure of the module needs placing.
    bindings: OnceCell<ModuleCommandBindings>,
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
            self.expanded_registration(head, words);
            return;
        };
        self.callbacks(head, &args, 0);
    }

    /// The callbacks the invocation `head args` stores, read, and the
    /// positions they sit at. A word the run time computes is a callback the
    /// scan cannot read.
    fn callbacks(&mut self, head: &str, args: &[Arg], depth: u32) -> Vec<usize> {
        let spellings = spellings_of(args);
        let indices = self.registry.callback_script_indices(
            head,
            &spellings,
            self.registry.own_surface_query(),
        );
        let prefixes = self.registry.command_prefixes(head, &spellings);
        for &index in &indices {
            let rest = args.get(index + 1..).unwrap_or_default();
            let builds_command = !rest.is_empty()
                && prefixes.iter().any(|&(at, _)| at == index)
                && !self.is_option_value(head, &spellings, index);
            match args.get(index) {
                Some(Arg::Literal(text)) if builds_command => {
                    self.prefix_command(text, rest, depth);
                }
                Some(Arg::Literal(text)) => self.script(text, depth),
                Some(Arg::Substitution(inner)) => self.command_prefix(inner, depth),
                Some(Arg::Dynamic) => self.out.any = true,
                None => {}
            }
        }
        if indices.is_empty() {
            self.concatenated_callback(head, args, &spellings, depth);
        }
        indices
    }

    /// An invocation with a `{*}` expansion stores any script it is given when
    /// its command can store one at all: which word is which is unknown.
    fn expanded_registration(&mut self, head: &str, words: &[Option<Arg>]) {
        let known: Vec<Arg> = words.iter().map_while(Option::clone).collect();
        let spellings = spellings_of(&known);
        let Some(call) =
            self.registry
                .resolve_call(head, &spellings, self.registry.own_surface_query())
        else {
            return;
        };
        let stores = |traits: Traits| {
            traits.contains(Traits::DEFERS_BODY) && !traits.contains(Traits::BODY_RUNS_IN_OWN_FRAME)
        };
        let may_store = match call.sub {
            Some(sub) => stores(call.spec.traits | sub.traits),
            None => {
                stores(call.spec.traits)
                    || call
                        .spec
                        .subcommands
                        .iter()
                        .any(|sub| stores(call.spec.traits | sub.traits))
            }
        };
        self.out.any |= may_store;
    }

    /// A script a command stores spelled as several words, which it joins as
    /// `concat` does (`after 100 set done 1`), has no position of its own in
    /// the registry, which names the callback of the form that has one word for
    /// it. The words from the first position at which that form names one are
    /// joined and read as that word is. No such position, and the invocation
    /// stores nothing (`after cancel …`, `after info`, a delay alone).
    fn concatenated_callback(&mut self, head: &str, args: &[Arg], spellings: &[&str], depth: u32) {
        let traits =
            self.registry
                .invocation_traits(head, spellings, self.registry.own_surface_query());
        if !traits.contains(Traits::DEFERS_BODY) || traits.contains(Traits::BODY_RUNS_IN_OWN_FRAME)
        {
            return;
        }
        for first in 0..args.len() {
            let words: Option<Vec<&str>> = args[first..].iter().map(Arg::literal).collect();
            let joined = words.as_deref().map(concat_words).unwrap_or_default();
            let mut probe = spellings[..first].to_vec();
            probe.push(&joined);
            if !self
                .registry
                .callback_script_indices(head, &probe, self.registry.own_surface_query())
                .contains(&first)
            {
                continue;
            }
            if words.is_some() {
                self.script(&joined, depth);
            } else {
                self.out.any = true;
            }
            return;
        }
    }

    /// A command prefix at a word that more words follow: Tcl runs the command
    /// those words make, with the caller's own words appended (`interp alias {}
    /// safe {} string length` makes `safe x` the call `string length x`), so the
    /// first is its command and the rest are its words, read as
    /// [`Self::command_prefix`] reads the command a `[list …]` word builds.
    fn prefix_command(&mut self, head: &str, rest: &[Arg], depth: u32) {
        if MAX_BRACKET_TEXT_DEPTH.exceeded(depth) {
            self.out.any = true;
            return;
        }
        self.invocation(head, rest, depth);
    }

    /// Whether word `index` of `head`'s invocation is the value of one of its
    /// options (`-command {.t yview} -orient vertical`), a prefix that is one
    /// word whatever follows it.
    fn is_option_value(&self, head: &str, spellings: &[&str], index: usize) -> bool {
        let Some(option) = index.checked_sub(1).and_then(|at| spellings.get(at)) else {
            return false;
        };
        let Some(call) =
            self.registry
                .resolve_call(head, spellings, self.registry.own_surface_query())
        else {
            return false;
        };
        let options = call.sub.map_or(call.spec.options, |sub| sub.options);
        tcl_registry::spec::resolve_option_prefix(options, option)
            .is_some_and(tcl_registry::hover::OptionSpec::takes_value)
    }

    /// A callback word that is `[cmd …]` holds the command `cmd` builds when the
    /// registry states it builds a command prefix (`[list tick $n]`): the
    /// words after `cmd` are that command.
    fn command_prefix(&mut self, text: &str, depth: u32) {
        let commands = tokenise_command_words(text, self.config);
        let [words] = commands.as_slice() else {
            self.out.any = true;
            return;
        };
        let Some(builder) = words.first().and_then(CommandWord::literal) else {
            self.out.any = true;
            return;
        };
        let spellings: Vec<&str> = words
            .iter()
            .skip(1)
            .map(|word| word.literal().unwrap_or_default())
            .collect();
        if words.iter().skip(1).any(|word| word.expanded)
            || !self
                .registry
                .invocation_traits(builder, &spellings, self.registry.own_surface_query())
                .contains(Traits::BUILDS_COMMAND_PREFIX)
        {
            self.out.any = true;
            return;
        }
        self.command(&words[1..], depth);
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
            // A computed command is any command at all.
            self.out.any = true;
            return;
        };
        if words.iter().skip(1).any(|word| word.expanded) {
            self.procedure(head);
            // Which word is which is unknown, so any variable may be written.
            self.out.any = true;
            return;
        }
        let args: Vec<Arg> = words
            .iter()
            .skip(1)
            .map(|word| self.substitution_word(word).unwrap_or(Arg::Dynamic))
            .collect();
        self.invocation(head, &args, depth);
    }

    /// The names the command `head args` writes, destroys or binds, and the
    /// scripts its words carry.
    fn invocation(&mut self, head: &str, args: &[Arg], depth: u32) {
        self.procedure(head);
        let spellings = spellings_of(args);
        let inputs: Vec<InvocationWord<'_>> = args.iter().map(Arg::registry_word).collect();
        self.variable_targets(head, args, &spellings, &inputs);
        self.aliases(head, &inputs);
        // A definition's body runs in a frame of its own, whose plain names
        // are its locals; anything else in this script runs where it does, and
        // a callback it stores runs later, read as a registration is.
        let invocation =
            self.registry
                .invocation_traits(head, &spellings, self.registry.own_surface_query());
        if !invocation.contains(Traits::BODY_RUNS_IN_OWN_FRAME) {
            let stored = self.callbacks(head, args, depth + 1);
            for index in self
                .registry
                .arg_indices_for_role(head, &spellings, ArgRole::Body)
            {
                if stored.contains(&index) {
                    continue;
                }
                match args.get(index) {
                    Some(Arg::Literal(body)) => self.script(body, depth + 1),
                    Some(Arg::Substitution(_) | Arg::Dynamic) => self.out.any = true,
                    None => {}
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
            // What a command that is neither a procedure of the module nor a
            // command of the registry runs is code the module does not contain.
            self.out.any |= !self.is_unbound_registry_command(head);
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

    /// Whether `head`, run at the global level, is a command of the registry
    /// the module leaves as it ships: the one target its bindings give it is
    /// the registry's own descriptor of that spelling, with no argument an
    /// `interp alias` prepends.
    fn is_unbound_registry_command(&self, head: &str) -> bool {
        let bindings = self
            .bindings
            .get_or_init(|| ModuleCommandBindings::analyse(self.module, self.registry));
        if bindings.target_resolution_may_be_unknown(head, "::") {
            return false;
        }
        let mut targets = bindings.targets(head, "::").into_iter();
        let (Some(target), None) = (targets.next(), targets.next()) else {
            return false;
        };
        target.registry_backed
            && target.prepended.is_empty()
            && target.command.trim_start_matches("::") == head.trim_start_matches("::")
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
        assert!(writes("after cancel {set done 1}").is_clear());
        assert!(writes("after info").is_clear());
        assert!(writes("after 100").is_clear());
        assert!(writes("after 100 {puts [clock seconds]}").is_clear());
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
    /// prefix, is read as the script it is.
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
    }

    /// A callback spelled as several words, which `after` concatenates like
    /// `concat`, is read as the script they make: the tclsh 8.4 to 9.1 output of
    /// each is the callback's `done`.
    #[test]
    fn a_callback_spelled_as_several_words_is_read_as_one_script() {
        for source in [
            "after 100 set done 1",
            "after idle set done 1",
            "after 100 incr done",
            "after 100 {set done} 1",
            "after 100 { set done 1 } { }",
            "set id [after 100 set done 1]",
            "proc p {} { after 100 set done 1 }",
            "after 1 {after 1 set done 1}",
        ] {
            assert_eq!(names(source), ["done"], "{source}");
        }
        assert_eq!(
            names("proc tick {} { set ::done 1 }\nafter 100 tick extra"),
            ["done"]
        );
    }

    /// A callback the scan cannot read may write any variable: a word the run
    /// time computes, a substitution of a command that builds no command
    /// prefix, a `{*}` expansion, a computed command head and a command the
    /// module cannot name.
    #[test]
    fn a_callback_the_scan_cannot_read_may_write_any_variable() {
        for source in [
            "after 100 $script",
            "after idle $script",
            "after 100 [build $n]",
            "after 100 set done $v $w",
            "after 100 {*}[list set done 1]",
            "after 100 {*}$words",
            "after 100 {$cmd x}",
            "after 100 [list $cmd x]",
            "after 100 {eval $script}",
            "after 100 {eval [build]}",
            "after 1 {after 1 $script}",
            "after 100 finish",
            "after 100 {finish now}",
            "interp alias {} fin {} set done 1\nafter 100 fin",
            "trace add variable x write $callback",
            "fileevent stdin readable $handler",
            "bind . <Key> $handler",
            "button .b -command $command",
            "after 100 {set {*}$pair}",
        ] {
            assert!(writes(source).any, "{source}");
        }
    }

    /// What it can name leaves the rest alone: a command of the registry or a
    /// procedure of the module, a callback that stores nothing, and a script
    /// that is only read.
    #[test]
    fn a_callback_it_can_read_leaves_every_other_variable_alone() {
        for source in [
            "after 100 update",
            "after 100 {puts hi}",
            "proc tick {} { puts hi }\nafter 100 tick",
            "proc tick {} { puts hi }\nafter 100 [list tick $n]",
            "namespace eval ns { proc tick {} { puts hi }; after 100 tick }",
            "after cancel $script",
            "after cancel {set done 1}",
            "after info",
            "after 100",
            "after $delay",
            "trace remove variable x write $callback",
            "trace info variable x",
            "bind . <Key>",
            "fileevent stdin readable",
            "puts {*}$words",
        ] {
            assert!(writes(source).is_clear(), "{source}: {:?}", writes(source));
        }
    }

    /// An alias stores the command its target word and the words after it
    /// make, read as that one command, however the target is spelled:
    /// `string length` writes nothing and `set done 1` writes `done`. A target
    /// word with nothing after it is read as the script it is, and `string`
    /// alone selects no subcommand, so it may write anything.
    #[test]
    fn an_alias_target_and_the_words_after_it_are_one_command() {
        for source in [
            "interp alias {} safe {} string length",
            "interp alias {} safe {} ::string length",
            "interp alias {} count {} llength",
        ] {
            assert!(writes(source).is_clear(), "{source}: {:?}", writes(source));
        }
        assert_eq!(names("interp alias {} fin {} set done 1"), ["done"]);
        assert!(writes("interp alias {} safe {} string").any);
    }

    /// A callback that writes a name it computes may change any variable.
    #[test]
    fn a_computed_name_in_a_callback_may_write_any_variable() {
        assert!(writes("after 1 { set $n 1 }").any);
        assert!(!writes("after 1 { set n 1 }").any);
    }
}
