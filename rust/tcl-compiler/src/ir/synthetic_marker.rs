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

//! The statements the CFG builder synthesises to state an analysis effect or a codegen placeholder.

/// A statement the CFG builder **synthesised**: it models an analysis effect
/// or a codegen placeholder, and is never a Tcl command to run.
///
/// A [`Statement::Barrier`] normally *is* a command whose side effects defeat
/// static analysis (`eval`, a dynamic-body `catch`, `return -code …`), so
/// codegen dispatches it. The CFG builder also inserts statements that carry
/// only a `defs` list or a widening effect, and codegen must never emit an
/// invoke for one of *those*: a caller-frame widening that reused the
/// callee's own name would make
/// `proc p {} { upvar 1 {a b} v ; puts "u=$v" }; p` invoke `p` twice and
/// printed its body's output twice where tclsh 8.6.16 / 9.0.4 print it once.
///
/// The identity has to be **typed**, which is why this enum exists rather than
/// a list of reserved command spellings. A Tcl command name is an arbitrary
/// string, so every spelling the marker set used is a name a script may
/// legally define and call, and a name-based check silently dropped such a
/// call. Verified identical on tclsh 8.4.20, 8.5.19, 8.6.16, 9.0.4 and 9.1:
///
/// ```text
/// proc <cond> {} { puts "hit-cond" }
/// proc <caller-frame-opaque> {} { puts "hit-cfo" }
/// proc <global-frame-script> {} { puts "hit-gfs" }
/// proc <word-effects> {} { puts "hit-we" }
/// proc <empty_clause> {} { puts "hit-ec" }
/// <cond> ; <caller-frame-opaque> ; <global-frame-script>
/// <word-effects> ; <empty_clause>
/// ->  hit-cond / hit-cfo / hit-gfs / hit-we / hit-ec   (all five run)
/// ```
///
/// Not even the empty name is reservable — `proc {} {} { puts EMPTY-NAME-RAN
/// }; {}` prints `EMPTY-NAME-RAN` on all five releases — so no string sentinel
/// can carry this. A marker is recognised only through
/// [`CommandTokens::synthetic`], which both [`CommandTokens`] constructors
/// always leave `None`; nothing a script can write reaches
/// [`CommandTokens::marker`]. The statement keeps a readable `command`
/// spelling for the disassembly and the explorer — it is a label, not the
/// identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SyntheticMarker {
    /// Evaluate a proved wrapper's original words before its script phases.
    /// This boundary performs no direct command invocation effects.
    EvaluatedArguments,
    /// Residual wrapper effects/completion after its semantic script phases.
    /// Original words are provenance and must not substitute again.
    EvaluatedWrapper,
    /// Map frozen dictionary inputs into caller variables before the script.
    DictionaryScopeEntry,
    /// Perform completion-sensitive dictionary writeback without replaying argv.
    DictionaryScopeWriteback(tcl_registry::completion_route::InvocationCompletionRoute),
    /// Bind the next iteration's variables after a proved loop dispatch.
    /// The readable command label and value words are provenance; this node
    /// does not invoke the source command again. Its payload is the input
    /// representation minted by the genuine typed iterator producer; None
    /// means this iterator selects named storage rather than a value operand.
    IterationBindings(Option<tcl_registry::TclType>),
    /// Store a proved catch's output values after its body has completed.
    /// Original argv and binding evidence are retained without reevaluation.
    CapturedCatchOutputs,
    /// The `if` / `while` condition kill-set: a [`Statement::Call`] whose
    /// `defs` are the variables the condition's own substitutions write, so a
    /// value-motion pass cannot carry a stale value across the branch.
    Condition,
    /// An empty `for` init / next clause. Codegen emits the three `nop`s
    /// tclsh's bytecode has there, so the clause keeps its place in the
    /// instruction stream.
    EmptyClause,
    /// Caller-side may-definitions for an embedded upvar procedure. The
    /// original host statement performs the invocation; this node only
    /// invalidates the reached caller variables.
    UpvarInvalidate,
    /// A selected registry invocation can evaluate code or mutate command
    /// bindings. This separate node widens facts without repeating dispatch
    /// or claiming an original native invocation or handler.
    RegistryBarrier,
    /// What the `[…]` substitutions in a statement's words do to the frame —
    /// the names they write, and those they read before writing — as a
    /// definition point of its own: the statement's words read a name before
    /// and after a substitution writes it, and a statement's SSA records one
    /// version of each name it reads. It is paired with the statement whose
    /// words they are by construction, never by span ([`WordEffectsHost`]).
    WordEffects(WordEffectsHost),
    /// A callee that runs an unreadable script at the global frame
    /// (`uplevel #0 $body`): it can write any global or namespace
    /// name, so the site widens instead of enumerating defs.
    GlobalFrameScript,
    /// A callee whose caller-frame `upvar` alias cannot be placed
    /// (`upvar 1 $computed x`): it can write any caller variable, so the call
    /// site widens. It sits *beside* the call it widens for, which is why
    /// naming the callee on it would make codegen run the callee twice.
    CallerFrameOpaque,
    /// A call to code the module cannot see — a head the module cannot name
    /// (a literal it neither defines nor the registry ships, an alias to the
    /// unresolved-command handler, a binding the source-order timeline cannot
    /// name, a computed head), a `source`, or code of that kind a script a
    /// statement keeps inside itself runs. That code may write, unset or read
    /// any name of the frame it is called from: a plain name at the top level
    /// is the global `::name`, and a procedure's local is in the reach of a
    /// callee that runs `upvar 1` or `uplevel 1`, which an autoloaded or
    /// unknown-handled callee can do on every release. The marker sits beside
    /// the call (ahead of the statement for a `[…]` substitution's call), is
    /// no command to run, and widens once: the SSA gives every name live past
    /// it a fresh version ([`crate::ssa::SsaFunction::value_clobbers`]),
    /// whose value is unknown while every proof about the version before it
    /// stands, and records the version each name holds where it sits
    /// ([`crate::ssa::SsaFunction::is_observed_by_unseen_call`]), which the
    /// code it reaches may read.
    UnseenCall,
    /// The names a statement that keeps its scripts inside itself may write
    /// into the frame, carried on a statement of its own beside it because the
    /// scripts are not lowered: what a callee an arm of an opaque `switch`
    /// calls writes (`zero line` with `upvar 1`), after the `switch`, and every
    /// name the body of an opaque `catch` or `try` writes, ahead of the call.
    /// The names are *may*-definitions of the statement, as the writes the arms
    /// of a `switch` make themselves are ([`crate::ssa::switch_may_defs`]).
    ArmWrites,
}

/// Original argv inventory is separate from execution and mutable-world effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OriginalArgvInventoryDisposition {
    /// Original source must authenticate any lexical substitution inventory.
    SourceRequired,
    /// This canonical widening phase owns no argv. Its effects remain unknown.
    EffectOnly(SyntheticMarker),
}

/// Where the statement whose words a [`SyntheticMarker::WordEffects`]
/// definition point states the effects of stands, as the CFG builder puts
/// the two ([`crate::ssa::word_effects_host`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WordEffectsHost {
    /// The statement right after the definition point: an assignment, an
    /// expression or a command whose own words substitute.
    Statement,
    /// A control statement's words — a `return`'s value, a `switch`'s
    /// subject and patterns, a `catch` or `try` header — which run before the
    /// statement dispatches: no statement of the block holds them.
    Dispatch,
}
