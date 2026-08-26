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

//! The **pack-evaluation host** — design E's execution model (`SpecTcl` 2.0,
//! `docs/design/spectcl-design-e-deep-dive.md` §1) running a whole pack file
//! as a sandboxed Tcl program.
//!
//! This module is the second consumer of the sandbox this crate already runs
//! hook bodies in, with two deliberate differences:
//!
//! - **A wider whitelist.** A pack templates its registrations, so it gets
//!   `proc`, `namespace`, `concat`, `catch`, and the rest of
//!   [`PACK_EVAL_EXTRA_COMMANDS`] on top of the hook whitelist
//!   ([`SANDBOX_COMMANDS`]). The determinism contract (§1.2) still holds:
//!   nothing on either list can read a clock, a file, a socket, the process
//!   environment, or the event loop, and `expr`'s `rand()`/`srand()` are
//!   removed from the maths-function table so the one nondeterministic maths
//!   function is unreachable.
//! - **Vocabulary words are the caller's.** The registration vocabulary
//!   (`command`, `option`, `available`, …) is supplied by the loader as
//!   [`WordHandler`] callbacks; this host only wires them in as native
//!   commands, tracks the current source line, and lets a handler evaluate a
//!   nested body ([`PackEvalCtx::eval_body`]) so `command NAME SYNOPSIS {…}`
//!   can run its block as Tcl in the caller's frame.
//!
//! Unlike the hook host, this module is deliberately **engine-specific**: it
//! works on the `tcl-vm` interpreter directly, because re-entrant body
//! evaluation from inside a registered command and per-command source-line
//! tracking are `Vm` facilities the engine-neutral interface does not carry.
//! It stays here — beside the sandbox it extends — rather than in the loader
//! crate, so `tcl-spectcl` keeps speaking to engines only through this crate.

use std::cell::Cell;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Rc;

use tcl_engine_api::{Budget, BudgetKind, CompileUnit, Engine, EngineError};
use tcl_engine_tclvm::TclVmEngine;
use tcl_vm::{Code, Completion, NativeCommand, Vm};

use crate::sandbox::SANDBOX_COMMANDS;

/// General Tcl a pack program may use **beyond** the hook-body whitelist.
///
/// Everything here is deterministic and world-contained: control flow,
/// procedure and namespace definition, list/state manipulation. Deliberately
/// absent, exactly as for hook bodies: anything that reads a clock, a file, a
/// socket, the environment, or the event loop — and additionally `info`,
/// `uplevel`, `upvar`, `trace`, `rename`, `subst`, and `interp`, which stay
/// denied so a pack cannot observe or rewrite the host's world
/// ([`denied_axis`] names each one's axis).
pub const PACK_EVAL_EXTRA_COMMANDS: &[&str] = &[
    "proc",
    "apply",
    "eval",
    "namespace",
    "variable",
    "global",
    "concat",
    "append",
    "unset",
    "catch",
    "error",
    "throw",
    "try",
    "lmap",
    "linsert",
    "lrepeat",
    "lreverse",
    "lset",
    "lpop",
    "ledit",
    "lseq",
    "array",
    "const",
    "fpclassify",
];

/// The prefixes of namespaced helper commands the whitelist keeps: the
/// ensemble targets of `string`/`dict`/`array`/`namespace` and the
/// deterministic maths functions.
const KEPT_PREFIXES: &[&str] = &[
    "tcl::string::",
    "tcl::dict::",
    "tcl::array::",
    "tcl::namespace::",
];

/// The two maths functions removed from the pack sandbox: the only source of
/// nondeterminism `expr` carries.
const DENIED_MATHFUNCS: &[&str] = &["tcl::mathfunc::rand", "tcl::mathfunc::srand"];

/// Commands the pack sandbox deliberately denies, each with the determinism
/// axis its absence protects. An unresolved dispatch of one of these is a
/// **sandbox denial** — a hard, transactional load failure — where an
/// unresolved word outside this table is unknown *vocabulary* and follows the
/// loader's §6.1 classification instead.
const DENIED_COMMANDS: &[(&str, &str)] = &[
    ("clock", "clock/time"),
    ("time", "clock/time"),
    ("after", "event loop"),
    ("update", "event loop"),
    ("vwait", "event loop"),
    ("open", "file/channel access"),
    ("close", "file/channel access"),
    ("read", "file/channel access"),
    ("gets", "file/channel access"),
    ("puts", "file/channel access"),
    ("seek", "file/channel access"),
    ("tell", "file/channel access"),
    ("eof", "file/channel access"),
    ("flush", "file/channel access"),
    ("fblocked", "file/channel access"),
    ("fconfigure", "file/channel access"),
    ("file", "file/channel access"),
    ("glob", "file/channel access"),
    ("cd", "file/channel access"),
    ("pwd", "file/channel access"),
    ("source", "file/channel access"),
    ("socket", "network"),
    ("exec", "process execution"),
    ("exit", "process control"),
    ("package", "package loading"),
    ("auto_load", "package loading"),
    ("auto_import", "package loading"),
    ("encoding", "host environment"),
    ("interp", "interpreter escape"),
    ("info", "host introspection"),
    ("uplevel", "sandbox containment"),
    ("upvar", "sandbox containment"),
    ("trace", "sandbox containment"),
    ("rename", "sandbox containment"),
    ("subst", "sandbox containment"),
    ("tailcall", "sandbox containment"),
    ("coroutine", "sandbox containment"),
    ("yield", "sandbox containment"),
    ("yieldto", "sandbox containment"),
];

/// The determinism axis `name` is denied on, when it is a denied command
/// rather than unknown vocabulary.
#[must_use]
pub fn denied_axis(name: &str) -> Option<&'static str> {
    let name = name.trim_start_matches("::");
    if name.starts_with("thread::") || name.starts_with("tpool::") || name.starts_with("tsv::") {
        return Some("threading");
    }
    DENIED_COMMANDS
        .iter()
        .find(|(denied, _)| *denied == name)
        .map(|(_, axis)| *axis)
}

/// The wall-clock half of the default pack-evaluation budget — and `None`
/// wherever there is no host clock a snapshot may depend on.
///
/// **Why this is target-gated rather than always armed.** A pack's snapshot is
/// meant to be a function of `(content, vocabulary, tier)` and nothing else
/// (design E §1.1, and the key `evaluate_pack_cached` memoises on). A
/// wall-clock deadline is the one budget axis that can fire differently for
/// the same three inputs, so it is only worth arming where the clock behind it
/// is a real, monotonic, unthrottled one — which on
/// `wasm32-unknown-unknown` it is not:
///
/// - the only clock there is the embedding page's `Date.now()`
///   (`tcl_vm::host_wasm::BrowserClock`), and a browser throttles or freezes
///   it in a backgrounded tab, so the same pack can load in a foreground tab
///   and fail its budget in a background one;
/// - with `tcl-vm`'s `js-clock` feature off — the import-free build — that
///   clock reports the epoch and the deadline can never be reached at all, so
///   the axis is silently inert rather than merely unreliable.
///
/// Neither is a budget worth having. What remains is the axis that *is* the
/// containment guarantee: the command-step budget, which `Vm` counts itself
/// and which needs no host at all (`Vm::set_wall_clock_budget`'s own
/// documentation says as much — the wall clock is "the belt to its braces").
/// The value-size cap is likewise host-free and stays armed. So a browser
/// evaluation is bounded by steps and value size, and is deterministic in
/// exactly the way the design asks for.
///
/// The residual gap, stated plainly: a pack that spends its whole evaluation
/// inside *one* command whose result stays small is bounded by nothing on this
/// target — a catastrophically backtracking `regexp` is the realistic shape,
/// since the whitelist admits one. That is a stalled tab rather than a
/// corrupted load (registration is transactional and the page is a worker's
/// call away from being reloaded), and it is the price of a snapshot that
/// means the same thing in every tab. A per-command step counter, which the
/// VM could measure without a host, is the honest fix if it ever bites.
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
const PACK_EVAL_WALL_CLOCK: Option<std::time::Duration> = None;
/// The wall-clock half of the default pack-evaluation budget: five seconds on
/// every target with a real clock under it. See the `wasm32` twin for why it
/// is target-gated.
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
const PACK_EVAL_WALL_CLOCK: Option<std::time::Duration> = Some(std::time::Duration::from_secs(5));

/// What a pack program runs under. Wider than a hook body's budget — a pack
/// registers a whole library — but still a hard ceiling on every axis the
/// target can measure, so a runaway template fails the load instead of the
/// process. See [`PACK_EVAL_WALL_CLOCK`] for the one axis that is not
/// universal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PackEvalConfig {
    /// The budget the whole evaluation runs under.
    pub budget: Budget,
}

impl Default for PackEvalConfig {
    fn default() -> Self {
        let budget = Budget::of_commands(2_000_000).with_max_value_bytes(64 * 1024 * 1024);
        Self {
            budget: match PACK_EVAL_WALL_CLOCK {
                Some(wall_clock) => budget.with_wall_clock(wall_clock),
                None => budget,
            },
        }
    }
}

/// Why a pack program did not run to completion. Every variant is a
/// **transactional** failure: the caller discards everything the evaluation
/// staged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackEvalFailure {
    /// The file did not compile as Tcl.
    Compile(String),
    /// The program raised an uncaught Tcl error (a sandbox denial reaches the
    /// caller this way, carrying the denial message its handler produced).
    Script(String),
    /// The program outran its budget on the named axis.
    Budget(&'static str),
    /// The engine panicked; the payload is whatever the boundary recovered.
    Panic(String),
}

impl PackEvalFailure {
    /// The budget axis name for one exceeded [`BudgetKind`].
    #[must_use]
    pub const fn budget_axis(kind: BudgetKind) -> &'static str {
        match kind {
            BudgetKind::Commands => "command steps",
            BudgetKind::WallClock => "wall clock",
            BudgetKind::ValueSize => "value size",
        }
    }
}

impl std::fmt::Display for PackEvalFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Compile(message) => write!(f, "the pack does not compile as Tcl: {message}"),
            Self::Script(message) => write!(f, "{message}"),
            Self::Budget(axis) => {
                write!(f, "the evaluation budget was exhausted on the {axis} axis")
            }
            Self::Panic(payload) => write!(f, "the evaluation engine crashed: {payload}"),
        }
    }
}

/// What a vocabulary handler sees when its word is dispatched: the current
/// (unit-relative) source line, and the door back into the interpreter for
/// evaluating a declaration's body block.
pub struct PackEvalCtx<'a> {
    vm: &'a mut Vm,
    line: u32,
}

impl PackEvalCtx<'_> {
    /// The 1-based source line of the statement being dispatched, **relative
    /// to the unit currently executing** — the pack file for top-level
    /// statements, a body block inside [`Self::eval_body`], a `proc` body
    /// inside a helper. The caller keeps the base-line bookkeeping.
    #[must_use]
    pub fn line(&self) -> u32 {
        self.line
    }

    /// Evaluate `body` as a Tcl script in the **current frame** — the design E
    /// scope rule that lets a `command` body read the variables of the
    /// `foreach` that templated it.
    ///
    /// An error (including a budget exhaustion, whose message must be
    /// propagated verbatim so the top level can still name the axis) comes
    /// back as `Err`; the handler returns it from its own invocation so the
    /// failure unwinds the whole evaluation.
    pub fn eval_body(&mut self, body: &str) -> Result<(), String> {
        match self.vm.eval_source(body) {
            Ok(completion) => match completion.code {
                Code::Ok | Code::Return => Ok(()),
                _ => Err(completion.result.to_str().to_string()),
            },
            Err(error) => Err(error.message),
        }
    }
}

/// One registration word's implementation. `Ok(Some(text))` is the command's
/// Tcl result (`available?` answers a boolean this way); `Ok(None)` is the
/// empty result; `Err` raises a Tcl error that fails the whole load.
pub type WordHandler =
    Rc<dyn Fn(&mut PackEvalCtx<'_>, &[String]) -> Result<Option<String>, String>>;

/// The handler for a command name nothing resolves: the attempted name plus
/// its arguments. This is where the loader applies §6.1 vocabulary
/// classification and where [`denied_axis`] denials become hard errors.
pub type UnknownHandler =
    Rc<dyn Fn(&mut PackEvalCtx<'_>, &str, &[String]) -> Result<Option<String>, String>>;

/// Adapts a [`WordHandler`] to the VM's native-command seam, carrying the
/// shared current-line cell the debug hook keeps up to date.
struct WordAdapter {
    handler: WordHandler,
    line: Rc<Cell<u32>>,
}

fn completion_of(outcome: Result<Option<String>, String>) -> Completion<tcl_vm::Value> {
    match outcome {
        Ok(result) => Completion::new(
            Code::Ok,
            tcl_vm::Value::string(result.unwrap_or_default()),
            tcl_vm::Value::string(String::new()),
        ),
        Err(message) => Completion::new(
            Code::Error,
            tcl_vm::Value::string(message),
            tcl_vm::Value::string(String::new()),
        ),
    }
}

impl NativeCommand for WordAdapter {
    fn invoke(&self, vm: &mut Vm, args: &[tcl_vm::Value]) -> Completion<tcl_vm::Value> {
        let words: Vec<String> = args
            .iter()
            .map(|value| value.to_str().to_string())
            .collect();
        let mut ctx = PackEvalCtx {
            vm,
            line: self.line.get(),
        };
        completion_of((self.handler)(&mut ctx, &words))
    }
}

/// The `unknown` fallback, adapted the same way: the VM invokes it as
/// `unknown NAME arg…`, so the first argument is the unresolved name.
struct UnknownAdapter {
    handler: UnknownHandler,
    line: Rc<Cell<u32>>,
}

impl NativeCommand for UnknownAdapter {
    fn invoke(&self, vm: &mut Vm, args: &[tcl_vm::Value]) -> Completion<tcl_vm::Value> {
        let words: Vec<String> = args
            .iter()
            .map(|value| value.to_str().to_string())
            .collect();
        let Some((name, rest)) = words.split_first() else {
            return completion_of(Ok(None));
        };
        let mut ctx = PackEvalCtx {
            vm,
            line: self.line.get(),
        };
        completion_of((self.handler)(&mut ctx, name, rest))
    }
}

/// Run one pack file as a sandboxed, budgeted, deterministic Tcl program.
///
/// `vocabulary` names the registration words and their handlers;
/// `unknown` receives every dispatch nothing resolves. The program runs to
/// completion (`Ok`) or fails transactionally with the reason.
///
/// # Panics
///
/// Never — an engine panic is caught and reported as
/// [`PackEvalFailure::Panic`].
pub fn run_pack_program(
    source: &str,
    vocabulary: &[(&str, WordHandler)],
    unknown: &UnknownHandler,
    config: &PackEvalConfig,
) -> Result<(), PackEvalFailure> {
    let mut engine = TclVmEngine::new();
    if let Err(error) = engine.set_budget(config.budget) {
        return Err(PackEvalFailure::Compile(error.to_string()));
    }

    // The current-line seam: the VM keeps the watched cell on the source
    // line of the instruction being dispatched, so at the moment a
    // vocabulary word's native command is invoked the cell holds that
    // statement's unit-relative line. This is `Vm::set_line_watch`, the
    // cheap sibling of the step-debugger hook.
    let line = Rc::new(Cell::new(1_u32));
    engine.vm_mut().set_line_watch(Some(Rc::clone(&line)));

    let mut registered: Vec<String> = Vec::with_capacity(vocabulary.len() + 1);
    for (word, handler) in vocabulary {
        engine.vm_mut().register_native_command(
            word,
            Rc::new(WordAdapter {
                handler: Rc::clone(handler),
                line: Rc::clone(&line),
            }),
        );
        registered.push((*word).to_owned());
    }
    engine.vm_mut().register_native_command(
        "unknown",
        Rc::new(UnknownAdapter {
            handler: Rc::clone(unknown),
            line: Rc::clone(&line),
        }),
    );
    registered.push("unknown".to_owned());

    // The whitelist sweep, after every registration so the sweep keeps them:
    // hook-sandbox commands, the pack extras, the ensemble helper namespaces,
    // and the maths functions minus `rand`/`srand`.
    engine.vm_mut().retain_commands(&|name| {
        let name = name.trim_start_matches("::");
        if DENIED_MATHFUNCS.contains(&name) {
            return false;
        }
        SANDBOX_COMMANDS.contains(&name)
            || PACK_EVAL_EXTRA_COMMANDS.contains(&name)
            || registered.iter().any(|kept| kept == name)
            || KEPT_PREFIXES.iter().any(|prefix| name.starts_with(prefix))
            || name.starts_with("tcl::mathfunc::")
            || name == "tcl::prefix"
            || name == "tcl::build-info"
    });

    let unit = CompileUnit {
        name: "pack",
        parameters: &[],
        body: source,
    };
    let handle = match engine.compile(unit) {
        Ok(handle) => handle,
        Err(error) => return Err(PackEvalFailure::Compile(error.to_string())),
    };

    let invoked = catch_unwind(AssertUnwindSafe(|| engine.invoke(&handle, &[])));
    match invoked {
        Ok(Ok(_)) => Ok(()),
        Ok(Err(EngineError::BudgetExceeded(kind))) => {
            Err(PackEvalFailure::Budget(PackEvalFailure::budget_axis(kind)))
        }
        Ok(Err(EngineError::Compile(message))) => Err(PackEvalFailure::Compile(message)),
        Ok(Err(EngineError::Script { message, .. })) => Err(PackEvalFailure::Script(message)),
        Ok(Err(other)) => Err(PackEvalFailure::Script(other.to_string())),
        Err(payload) => Err(PackEvalFailure::Panic(panic_text(payload.as_ref()))),
    }
}

fn panic_text(payload: &(dyn std::any::Any + Send)) -> String {
    payload.downcast_ref::<&str>().map_or_else(
        || {
            payload
                .downcast_ref::<String>()
                .cloned()
                .unwrap_or_else(|| "panic with an unreadable payload".to_owned())
        },
        |message| (*message).to_owned(),
    )
}

#[cfg(test)]
mod tests {
    use super::{
        PACK_EVAL_EXTRA_COMMANDS, PACK_EVAL_WALL_CLOCK, PackEvalConfig, PackEvalFailure,
        UnknownHandler, WordHandler, denied_axis, run_pack_program,
    };
    use std::cell::RefCell;
    use std::rc::Rc;

    type Captured = Rc<RefCell<Vec<Vec<String>>>>;

    fn capture_all() -> (Captured, Vec<(&'static str, WordHandler)>, UnknownHandler) {
        let seen: Captured = Rc::new(RefCell::new(Vec::new()));
        let make = |word: &'static str, seen: &Captured| -> WordHandler {
            let seen = Rc::clone(seen);
            Rc::new(move |_ctx, args| {
                let mut row = vec![word.to_owned()];
                row.extend(args.iter().cloned());
                seen.borrow_mut().push(row);
                Ok(None)
            })
        };
        let vocabulary = vec![("register", make("register", &seen))];
        let unknown: UnknownHandler = {
            let seen = Rc::clone(&seen);
            Rc::new(move |_ctx, name, args| {
                if let Some(axis) = denied_axis(name) {
                    return Err(format!("`{name}` denied ({axis})"));
                }
                let mut row = vec!["?".to_owned(), name.to_owned()];
                row.extend(args.iter().cloned());
                seen.borrow_mut().push(row);
                Ok(None)
            })
        };
        (seen, vocabulary, unknown)
    }

    #[test]
    fn templated_registrations_run_and_capture_in_order() {
        let (seen, vocabulary, unknown) = capture_all();
        run_pack_program(
            "foreach x {a b c} { register $x }\n",
            &vocabulary,
            &unknown,
            &PackEvalConfig::default(),
        )
        .expect("runs");
        let rows: Vec<Vec<String>> = seen.borrow().clone();
        assert_eq!(rows.len(), 3, "{rows:?}");
        assert_eq!(rows[0], vec!["register".to_owned(), "a".to_owned()]);
        assert_eq!(rows[2], vec!["register".to_owned(), "c".to_owned()]);
    }

    #[test]
    fn a_denied_command_fails_the_whole_program() {
        let (_seen, vocabulary, unknown) = capture_all();
        let failure = run_pack_program(
            "register ok\nclock seconds\nregister never\n",
            &vocabulary,
            &unknown,
            &PackEvalConfig::default(),
        )
        .expect_err("clock is denied");
        assert!(
            matches!(&failure, PackEvalFailure::Script(message) if message.contains("clock")),
            "{failure:?}"
        );
    }

    /// The default budget arms every axis the target can measure, and the
    /// wall clock only where a real clock backs it (see
    /// [`PACK_EVAL_WALL_CLOCK`]).
    #[test]
    fn the_default_budget_arms_the_axes_this_target_can_measure() {
        let budget = PackEvalConfig::default().budget;
        assert_eq!(budget.commands, Some(2_000_000));
        assert_eq!(budget.max_value_bytes, Some(64 * 1024 * 1024));
        assert_eq!(budget.wall_clock, PACK_EVAL_WALL_CLOCK);
        assert_eq!(
            budget.wall_clock.is_none(),
            cfg!(all(target_arch = "wasm32", target_os = "unknown")),
            "the browser evaluates a pack under the step budget alone"
        );
    }

    #[test]
    fn the_command_budget_names_its_axis() {
        let (_seen, vocabulary, unknown) = capture_all();
        let failure = run_pack_program(
            "set i 0\nwhile {1} { register [incr i] }\n",
            &vocabulary,
            &unknown,
            &PackEvalConfig {
                budget: tcl_engine_api::Budget::of_commands(500)
                    .with_wall_clock(std::time::Duration::from_secs(5)),
            },
        )
        .expect_err("the budget stops it");
        assert_eq!(failure, PackEvalFailure::Budget("command steps"));
    }

    #[test]
    fn rand_is_unreachable_from_expr() {
        let (_seen, vocabulary, unknown) = capture_all();
        let failure = run_pack_program(
            "register [expr {rand()}]\n",
            &vocabulary,
            &unknown,
            &PackEvalConfig::default(),
        )
        .expect_err("rand is removed");
        assert!(
            matches!(&failure, PackEvalFailure::Script(message)
                if message.contains("tcl::mathfunc::rand")),
            "{failure:?}"
        );
    }

    #[test]
    fn the_extra_whitelist_keeps_the_hook_denials_that_matter() {
        for denied in [
            "open", "exec", "source", "socket", "after", "interp", "info",
        ] {
            assert!(
                !PACK_EVAL_EXTRA_COMMANDS.contains(&denied),
                "{denied} must stay denied for pack programs"
            );
            assert!(denied_axis(denied).is_some(), "{denied} must name an axis");
        }
    }
}
