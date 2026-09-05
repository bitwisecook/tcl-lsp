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

//! Commands and builtins.
//!
//! A [`Command`] is either a native builtin or a user procedure. Procs are
//! dispatched by the engine (not here) so a call pushes an activation rather
//! than recursing (steering §8b); builtins run synchronously and return a
//! completion.

use std::rc::Rc;

use tcl_bytecode::FunctionAsm;
use tcl_runtime_api::{Code, Completion};
use tcl_syntax::formal_params::{has_trailing_args, parse_formal_parameters};

use crate::error::TclError;
use crate::interp::{Vm, err, ok};
use crate::value::Value;

/// A native builtin: receives argv *without* the command name (Tcl's `objv[1..]`).
pub type BuiltinFn = fn(&mut Vm, &[Value]) -> Completion<Value>;

/// A command implemented by an **embedder**, not by the VM.
///
/// [`BuiltinFn`] is a bare function pointer, so it can only implement a
/// command whose whole state is the interpreter's. An embedder registering a
/// command — a `SpecTcl` emitter verb collecting what a hook body emitted, a
/// shimmed C extension holding its `ClientData` — needs to carry state of its
/// own, which is what this trait is for: the VM stores it behind an `Rc` and
/// hands it the same `(&mut Vm, argv)` a builtin gets.
///
/// `&self` rather than `&mut self` because the VM is re-entrant (a registered
/// command can evaluate a script that calls it again), so an implementation
/// that mutates keeps its state in a `Cell`/`RefCell` and decides for itself
/// what re-entry means.
pub trait NativeCommand {
    /// Run the command. `args` excludes the command name, exactly as
    /// [`BuiltinFn`] receives it.
    fn invoke(&self, vm: &mut Vm, args: &[Value]) -> Completion<Value>;
}

/// A procedure parameter: a name with an optional default.
#[derive(Clone)]
pub struct Param {
    /// Parameter name.
    pub name: String,
    /// Default value, if the parameter is optional.
    pub default: Option<Value>,
}

/// A user procedure: parameters plus the pre-compiled body.
#[derive(Clone)]
pub struct ProcDef {
    /// Canonical (namespace-qualified, no leading `::`) proc name.
    pub name: String,
    /// The namespace the body executes in (canonical; `""` = global).
    pub namespace: String,
    /// Formal parameters in order.
    pub params: Vec<Param>,
    /// Whether the last parameter is the `args` catch-all.
    pub has_args: bool,
    /// Pre-compiled body bytecode.
    pub body: Rc<FunctionAsm>,
    /// Original body source text — used by `info body`.
    pub body_src: Value,
    /// Overrides the leading token of the `wrong # args` usage message. `None`
    /// uses the (simple) proc name; `apply` sets `"apply lambdaExpr"` so the
    /// message reads `wrong # args: should be "apply lambdaExpr …"` rather than
    /// leaking the internal temp proc name (apply-4.*).
    pub usage_name: Option<String>,
    /// The `Vm::trace_deopt_epoch` this `body` was compiled under (`0` for
    /// every proc defined the ordinary way, before any step-capable
    /// execution trace exists). A call site compares this against the
    /// interp's current epoch and recompiles from `body_src` — traced or
    /// fast, matching whether a step trace is currently active — when they
    /// differ (issue #946 fault 3). See [`crate::interp::Vm::ensure_proc_traced`].
    pub compiled_epoch: u64,
    /// The VM dialect-profile generation under which `body` was compiled.
    /// Profile mutation recompiles stored bodies from [`Self::body_src`] before
    /// specialised bytecode can run under the new command surface.
    pub compiled_profile_generation: u64,
}

/// A registered command.
#[derive(Clone)]
pub enum Command {
    /// A native Rust handler.
    Builtin(BuiltinFn),
    /// An embedder-registered handler carrying its own state
    /// ([`NativeCommand`]) — the extension-registration path, used by the
    /// `SpecTcl` hook host's emitter verbs.
    Native(Rc<dyn NativeCommand>),
    /// A user procedure (dispatched by the engine, which pushes an activation).
    Proc(Rc<ProcDef>),
    /// An `interp alias` — invoking the command evaluates these target words
    /// (the target command plus any fixed prefix arguments) with the call's own
    /// arguments appended.
    Alias(Rc<Vec<Value>>),
    /// A cross-interp `interp alias` whose target runs in a DIFFERENT
    /// interpreter (parent, child, sibling, or any node reachable through the
    /// shared engine): invoking it switches the engine to `target` and
    /// evaluates the target words there at its global frame (C's
    /// `TclAliasObjCmd` → `Tcl_EvalObjv(targetInterp, …)`).  The target interp
    /// is addressed by stable [`crate::interp::InterpId`], so it can be a
    /// child, the owning parent, or a sibling reached via the parent, and it
    /// works whether the current interp is executing at top level or is
    /// re-entered deeper on the stack.
    CrossAlias {
        /// The interpreter the target words run in.
        target: crate::interp::InterpId,
        /// Target command + fixed prefix arguments.
        words: Rc<Vec<Value>>,
    },
    /// A child interpreter addressable as a command (`$child eval …`): the id
    /// addresses the child in the engine's interp arena. Invoking it dispatches
    /// the `interp` ensemble restricted to that child.
    ChildInterp(crate::interp::InterpId),
    /// A `namespace ensemble` — invoking `cmd sub args…` resolves `sub` against
    /// the ensemble's subcommands and dispatches to the mapped target.
    Ensemble(Rc<tcl_cmd_core::ensemble::EnsembleToken<EnsembleDef, String>>),
    /// A `TclOO` object or class (`Foo create obj` / `obj method …`): the name
    /// keys into the interp's `oo` state (`OoState::objects`/`classes`).
    /// Invoking it dispatches `method args…` against the object (`oo_dispatch`).
    /// Analogous to [`Command::ChildInterp`] — a command backed by a Vm-side
    /// table keyed by the (canonical) name.
    Object(String),
}

/// A `namespace ensemble create`d command (`tclEnsemble.c`).
#[derive(Clone)]
pub struct EnsembleDef {
    /// The namespace whose exported commands form the default subcommand set and
    /// against which an unmapped subcommand `sub` resolves to `namespace::sub`.
    pub namespace: String,
    /// `-map`: subcommand → target command prefix (already qualified).
    ///
    /// An association list, not a hash map: C stores the map as a Tcl dict and
    /// `namespace ensemble configure -map` reads it back in **insertion
    /// order**, so the order is observable and must round-trip. (The runtime's
    /// `EnsembleMap` is the same shape for the same reason.) Ensembles have a
    /// handful of subcommands, so linear lookup is not a concern.
    pub map: Vec<(String, Vec<Value>)>,
    /// `-subcommands`: the explicit subcommand list, or `None` to use the
    /// namespace's exported commands.
    pub subcommands: Option<Vec<String>>,
    /// `-prefixes`: whether an unambiguous prefix of a subcommand resolves.
    pub prefixes: bool,
    /// `-parameters`: formal parameter names that precede the subcommand word
    /// (`ens p1 p2 sub arg…`); their values thread in after the resolved
    /// target prefix (`target p1 p2 arg…`). Empty for an ordinary ensemble.
    pub parameters: Vec<String>,
    /// `-unknown`: a handler prefix invoked when no subcommand matches.
    pub unknown: Option<Vec<Value>>,
}

/// Register the builtin set on `vm`.
pub(crate) fn register_builtins(vm: &mut Vm) {
    vm.register("set", cmd_set);
    vm.register("puts", cmd_puts);
    vm.register("source", cmd_source);
    vm.register("tcl::build-info", cmd_build_info);
    vm.register("interp", cmd_interp);
    vm.register("rename", cmd_rename);
    vm.register("eval", cmd_eval);
    vm.register("apply", cmd_apply);
    vm.register("incr", cmd_incr);
    vm.register("expr", cmd_expr);
    vm.register("proc", cmd_proc);
    vm.register("return", cmd_return);
    vm.register("const", cmd_const);
    vm.register("tailcall", cmd_tailcall);
    vm.register("time", cmd_time);
    vm.register("encoding", cmd_encoding);
    vm.register("error", cmd_error);
    vm.register("break", cmd_break);
    vm.register("continue", cmd_continue);
    vm.register("catch", cmd_catch);
    vm.register("global", cmd_global);
    vm.register("upvar", cmd_upvar);
    vm.register("uplevel", cmd_uplevel);
    vm.register("variable", cmd_variable);
    vm.register("unset", cmd_unset);
    vm.register("subst", cmd_subst);
    vm.register("auto_load", cmd_auto_load);
    vm.register("auto_import", |_, _| ok(Value::empty()));
    vm.register("exit", cmd_exit);
    crate::cmd_array::register(vm);
    crate::cmd_chan::register(vm);
    crate::cmd_clock::register(vm);
    crate::cmd_control::register(vm);
    crate::cmd_list::register(vm);
    crate::cmd_dict::register(vm);
    crate::cmd_file::register(vm);
    crate::cmd_format::register(vm);
    crate::cmd_info::register(vm);
    crate::cmd_math::register(vm);
    crate::cmd_binary::register(vm);
    crate::cmd_mathop::register(vm);
    crate::cmd_lseq::register(vm);
    crate::cmd_prefix::register(vm);
    crate::cmd_namespace::register(vm);
    crate::cmd_package::register(vm);
    crate::cmd_regexp::register(vm);
    crate::cmd_switch::register(vm);
    crate::cmd_trace::register(vm);
    crate::cmd_try::register(vm);
    crate::cmd_oo::register(vm);
    crate::cmd_coro::register(vm);
    crate::cmd_event::register(vm);
    crate::cmd_thread::register(vm);
    // Last so the spec-derived intrinsic identities remain live after the
    // startup registration sweep's conservative command-epoch invalidations.
    crate::cmd_string::register(vm);
}

/// `exit ?returnCode?` — request process termination with `returnCode`
/// (default 0). Matches `tclsh`: a non-integer code is an error.
///
/// The VM library never calls `std::process::exit` itself — that would kill an
/// embedding host (the debugger, `tcl-irule-test`, `f5 explain-flow
/// --simulate`) on any guest script that runs `exit`. Instead it records the
/// code on the interpreter and returns an unwinding completion; the standalone
/// `tclvm` CLI checks [`Vm::take_exit`] and performs the real process exit,
/// while embedders see a completion they can handle. Like C Tcl's `Tcl_Exit`
/// it is **not** catchable — `catch` re-propagates while an exit is pending.
fn cmd_exit(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let code = match args {
        [] => 0,
        [c] => match c.as_int() {
            Ok(n) => i32::try_from(n).unwrap_or(0),
            Err(_) => {
                return err(format!("expected integer but got \"{}\"", c.to_str()));
            }
        },
        _ => return err("wrong # args: should be \"exit ?returnCode?\""),
    };
    vm.set_exit(code);
    // Unwind everything with an error-coded completion (so it propagates through
    // proc boundaries, unlike `return`); `catch` re-propagates while the exit is
    // pending, and the driver consumes the code before the message is surfaced.
    Completion::new(
        Code::Error,
        Value::string(format!("exit {code}")),
        Value::empty(),
    )
}

/// `set varName ?newValue?` — read or write a scalar.
fn cmd_set(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    match args {
        [name] => {
            let n = name.to_str();
            // `set varName` is a variable read, so it fires read traces (like
            // the `$var` opcode path) — and a trace may create the variable
            // before the read resolves (tcltest's `SafeFetch` lazily
            // initialises a constraint this way).
            if let Err(c) = vm.fire_var_traces(&n, "read") {
                return c;
            }
            vm.var_get(&n).map_or_else(|| err(vm.read_miss_msg(&n)), ok)
        }
        [name, value] => match vm.var_set(&name.to_str(), value.clone()) {
            Ok(()) => ok(value.clone()),
            Err(e) => e,
        },
        _ => err("wrong # args: should be \"set varName ?newValue?\""),
    }
}

/// `source ?-encoding name? ?-nopkg? fileName` — read a file and evaluate it
/// as a script in the current context. `-encoding` is accepted and ignored
/// (files are read as UTF-8); Tcl 9's `-nopkg` suppresses package bookkeeping
/// that this VM does not otherwise perform, so it uses the same read path.
fn cmd_source(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let path = match args {
        [file] => file.to_str(),
        [flag, _enc, file] if &*flag.to_str() == "-encoding" => file.to_str(),
        [flag, file] if &*flag.to_str() == "-nopkg" => file.to_str(),
        _ => {
            return err("wrong # args: should be \"source ?-encoding name? ?-nopkg? fileName\"");
        }
    };
    let contents = match std::fs::read_to_string(&*path) {
        Ok(c) => c,
        Err(e) => return err(format!("couldn't read file \"{path}\": {e}")),
    };
    // Track the path so `info script` (and callers like `[file dirname [info
    // script]]`) resolve relative to the file being sourced.
    vm.push_script(path.to_string());
    let mut result = match vm.eval_source(&contents) {
        Ok(c) => c,
        Err(e) => err(e.message),
    };
    vm.pop_script();
    // A top-level `return` in the sourced file ends the *file*, not the caller:
    // `source` absorbs the `TCL_RETURN` and returns its value normally (like C
    // Tcl's `Tcl_FSEvalFileEx`). Without this a `return` — e.g. the early-out at
    // the top of a compatibility shim — unwinds the script that called `source`.
    if result.code == Code::Return {
        result.code = Code::Ok;
    }
    result
}

/// `tcl::build-info ?option?` — report build-time configuration. With no
/// argument it returns the full build string; `patchlevel` / `version` /
/// `commit` return their fields and any other word queries a build flag,
/// which is absent (`0`) on this VM because it is a plain release build.
///
/// Both the string and the field split come from
/// [`tcl_dialect::build_info`] keyed by this VM's pinned release, so the
/// answer tracks `--tcl-version` instead of a hardcoded `9.0.4`, and cannot
/// disagree with `runtime/rust` (ledger row B4). The registry gates the
/// command itself to `TCL90_PLUS`, matching `tclsh8.6`, where
/// `::tcl::build-info` is an invalid command name.
fn cmd_build_info(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let data = tcl_dialect::build_info::build_info(vm.runtime_version(), "tclvm");
    match args {
        [] => ok(Value::string(data)),
        [option] => ok(Value::string(tcl_dialect::build_info::query(
            &data,
            &option.to_str(),
        ))),
        _ => err("wrong # args: should be \"tcl::build-info ?option?\""),
    }
}

/// `eval arg ?arg ...?` — concatenate the arguments with spaces and evaluate the
/// result as a script in the current frame.
fn cmd_eval(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    if args.is_empty() {
        return err("wrong # args: should be \"eval arg ?arg ...?\"");
    }
    let script = if let [single] = args {
        single.to_str().to_string()
    } else {
        args.iter()
            .map(|v| v.to_str().to_string())
            .collect::<Vec<_>>()
            .join(" ")
    };
    // Defer the body to the *explicit* stack so a `yield` inside it stays
    // yieldable: compile it and hand it to the trampoline via
    // `pending_eval`, which pushes a transparent script frame whose result
    // replaces this placeholder. The script frame adds the `("eval" body line N)`
    // errorInfo frame itself on error (eval-2.5; see `Frame::body_label`).
    match vm.compile_source_cached(&script) {
        Ok(asm) => {
            vm.pending_eval = Some((asm, Some("eval"), None));
            ok(Value::empty())
        }
        Err(e) => err(e.message),
    }
}

/// Monotonic counter minting unique temporary command names for `apply`.
fn fresh_apply_name() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    // Canonical *unrooted* key form — `register_command` takes keys verbatim.
    format!("tcl::apply::lambda{n}")
}

/// Parse a lambda expression `{params body ?namespace?}` and define it as a
/// fresh internal proc, returning that proc's canonical name (`Err` with the
/// diagnostic on a malformed lambda).
///
/// Shared by `apply` and by `coroutine … apply …`. The caller owns the proc's
/// lifetime: `apply` deletes it right after the call; a coroutine keeps it alive
/// for the coroutine's life so the lambda body runs on the coroutine's *explicit
/// activation stack* (a `yield` inside it is then yieldable — the generic
/// `apply` path evaluates the body through a host-stack re-entry, which a
/// `yield` cannot cross).
pub(crate) fn build_lambda_proc(vm: &mut Vm, lambda: &Value) -> Result<String, Completion<Value>> {
    let parts = match lambda.as_list() {
        Ok(p) => p,
        Err(c) => return Err(err(c.message)),
    };
    if parts.len() < 2 || parts.len() > 3 {
        return Err(err(format!(
            "can't interpret \"{}\" as a lambda expression",
            lambda.to_str()
        )));
    }
    let (params_vec, has_args) = match parse_params(&parts[0].to_str()) {
        Ok(p) => p,
        Err(e) => {
            // A malformed parameter list errors "while parsing the lambda": add
            // the `(parsing lambda expression "<lambda>")` context frame so the
            // enclosing `apply` INVOKE logs "invoked from within" (apply-2.*).
            vm.seed_error_info_frame(
                &e,
                &format!("\n    (parsing lambda expression \"{}\")", lambda.to_str()),
            );
            return Err(err(e));
        }
    };
    let body = parts[1].clone();
    let Some(body_asm) = vm.compile_dynamic_body(&body.to_str()) else {
        return Err(err("apply: could not compile lambda body"));
    };
    // The optional third element is the namespace the body runs in (default
    // global). Strip a leading `::` to the canonical form. A named namespace
    // must already exist (apply-3.*); the message quotes the spelling given.
    let namespace = parts
        .get(2)
        .map(|v| v.to_str().trim_start_matches("::").to_string())
        .unwrap_or_default();
    if parts.len() == 3 && !vm.namespace_exists(&namespace) {
        // The message names the fully-qualified namespace: a relative third
        // element is resolved from the global namespace, so it is reported with a
        // leading `::` (apply-3.3 — `NONEXIST::…` → `::NONEXIST::…`).
        return Err(err(format!("namespace \"::{namespace}\" not found")));
    }

    let name = fresh_apply_name();
    vm.define_proc(ProcDef {
        name: name.clone(),
        namespace,
        params: params_vec,
        has_args,
        body: body_asm,
        body_src: body,
        usage_name: Some("apply lambdaExpr".to_string()),
        compiled_epoch: 0,
        compiled_profile_generation: vm.profile_generation(),
    });
    Ok(name)
}

/// `apply lambda ?arg ...?` — invoke an anonymous function `{params body ?ns?}`.
/// Implemented by binding the lambda to a temporary command and evaluating a
/// call, so parameter binding and `return` semantics match a normal proc.
///
/// Defers the call to the *explicit* stack via `pending_eval` (like
/// `eval`/`uplevel`) rather than `Vm::eval_source`'s nested drive, so a `yield`
/// inside the lambda body stays yieldable — `coroutine c apply {lambda}`
/// already got this treatment (`cmd_coroutine` binds the lambda to an internal
/// proc run on the coroutine's own stack); a bare `apply` called *from inside*
/// a coroutine body did not (issue #1311). `cleanup_proc` carries the
/// temporary proc's name so it is torn down once the deferred call completes,
/// mirroring the old `vm.take_command` that ran unconditionally after
/// `eval_source` returned.
fn cmd_apply(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some((lambda, call_args)) = args.split_first() else {
        return err("wrong # args: should be \"apply lambdaExpr ?arg ...?\"");
    };
    let name = match build_lambda_proc(vm, lambda) {
        Ok(n) => n,
        Err(c) => return c,
    };
    let mut words = Vec::with_capacity(call_args.len() + 1);
    words.push(Value::string(name.as_str()));
    words.extend_from_slice(call_args);
    let script = tcl_syntax::list::join_list(words.iter().map(Value::to_str));
    match vm.compile_source_cached(&script) {
        Ok(asm) => {
            vm.pending_eval = Some((asm, None, Some(name)));
            ok(Value::empty())
        }
        Err(e) => {
            vm.take_command_unchecked(&name);
            err(e.message)
        }
    }
}

/// `rename oldName newName` — rename a command, or delete it when `newName` is
/// empty.
fn cmd_rename(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let [old, new] = args else {
        return err("wrong # args: should be \"rename oldName newName\"");
    };
    let old_name = old.to_str();
    let new_name = new.to_str();
    // Tcl rejects a rename onto an existing command (leaving both intact), so
    // check the destination before removing the source.
    if !new_name.is_empty() && vm.lookup_command(&new_name).is_some() {
        return err(format!(
            "can't rename to \"{new_name}\": command already exists"
        ));
    }
    let Some((cmd, mut rename)) = vm.prepare_command_rename(&old_name) else {
        // Renaming to the empty name is a delete; C words the miss accordingly.
        let verb = if new_name.is_empty() {
            "delete"
        } else {
            "rename"
        };
        return err(format!(
            "can't {verb} \"{old_name}\": command doesn't exist"
        ));
    };
    let old_key = rename.old_key().to_owned();
    // A coroutine's state travels with its command: a delete (`rename $coro {}`)
    // drops it (no `finally` runs — the frozen continuation is inert data,
    // matching C Tcl); a rename re-keys it so it keeps working under the new name.
    // The prepared transaction holds the exact key resolved by Tcl's full
    // command lookup rule (including `namespace path`), rather than the
    // spelling rooted at the caller's current namespace.
    let is_coro = crate::cmd_coro::is_coroutine(vm, &old_key);
    if new_name.is_empty() {
        let trace_handled = vm.delete_prepared_renamed_command(&rename);
        if is_coro {
            crate::cmd_coro::on_command_deleted(vm, &old_key);
        }
        // `rename x {}` is a delete: fire the command's `delete` traces
        // (`callback ::old {} delete`, tclsh-pinned) and drop its traces.
        if !trace_handled {
            vm.on_command_removed(&old_key);
        }
    } else {
        // An unqualified target binds in the current namespace; a qualified one
        // is used as given, normalised to the key form (separator runs
        // collapse, the root drops — `rename p a:::q` creates `::a::q`,
        // tclsh8.6-verified; the raw name used to register a `a:::q` key).
        let key = if tcl_syntax::naming::is_qualified(new_name.as_bytes()) {
            crate::interp::canonical_cmd_key(&new_name).into_owned()
        } else {
            vm.qualify_name(&new_name)
        };
        if is_coro {
            crate::cmd_coro::on_command_renamed(vm, &old_key, &key);
        }
        // A proc executes in the namespace it currently lives in, so renaming
        // it across namespaces re-homes its body (C `TclRenameCommand` updates
        // the command's `nsPtr`). `namespace current` inside the body then
        // reports the destination namespace (proc-3.4). The key is canonical,
        // so the shared qualifier split yields its namespace directly.
        let cmd = match cmd {
            Command::Proc(def) => {
                // The destination namespace is the key's construction-inverse
                // holder — the written-name colon-run split would collapse a
                // lone-colon segment (#934).
                let (new_ns, _tail) = crate::interp::key_holder_and_tail_unrooted(&key);
                if new_ns == def.namespace && key == def.name {
                    Command::Proc(def)
                } else {
                    let mut relocated = (*def).clone();
                    relocated.namespace = new_ns;
                    relocated.name.clone_from(&key);
                    Command::Proc(Rc::new(relocated))
                }
            }
            other => other,
        };
        // C creates the destination's hash entry before `TclPreventAliasLoop`
        // and deletes it again on a refusal, so the resize that transient entry
        // triggers outlives the rejected rename.
        vm.note_rename_destination(&key);
        let is_alias = matches!(&cmd, Command::Alias(_) | Command::CrossAlias { .. });
        // C's `TclPreventAliasLoop` guards *rename* too: moving an alias onto
        // a name its own target chain resolves back to is refused, with the
        // command left untouched (tclsh-pinned: `interp alias {} a {} b;
        // rename a b` errors, `a` survives, `b` stays free).  This logical
        // check must precede registration: a hidden destination can carry
        // delete callbacks and trace/deoptimisation state that cannot be
        // rolled back after `register_command` observes an overwrite.
        if is_alias && vm.alias_chain_loops_for_rename(&key, &cmd) {
            vm.forget_rename_destination(&key);
            let tail = crate::interp::key_holder_and_tail_unrooted(&key).1;
            return err(format!(
                "cannot define or rename alias \"{tail}\": would create a loop"
            ));
        }
        vm.install_renamed_command(&mut rename, &key, cmd);
        vm.commit_renamed_command(&rename);
        // The rename happened: fire the command's `rename` traces
        // (`callback ::old ::new rename`, both fully qualified — tclsh-
        // pinned) and move every trace to the new key so it keeps firing
        // under the new name (also pinned).
        vm.on_command_renamed_traces(&old_key, &key);
    }
    ok(Value::empty())
}

/// `interp create ?-safe? ?--? ?name?` — make a child interpreter.
fn interp_create_cmd(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    // C's "weird historical rule": `-safe` is accepted anywhere before `--`
    // (`interp create a -safe` is valid), and the path is the lone non-option
    // word — so scan all args rather than stopping at the first non-flag.
    let mut safe = false;
    let mut last = false;
    let mut name: Option<String> = None;
    let mut i = 0;
    while i < rest.len() {
        let a = rest[i].to_str();
        if !last && a.starts_with('-') {
            match &*a {
                "-safe" => {
                    safe = true;
                    i += 1;
                    continue;
                }
                "--" => {
                    i += 1;
                    last = true;
                }
                s => return err(format!("bad option \"{s}\": must be -safe or --")),
            }
        }
        if name.is_some() {
            return err("wrong # args: should be \"interp create ?-safe? ?--? ?path?\"");
        }
        if let Some(n) = rest.get(i) {
            name = Some(n.to_str().to_string());
        }
        i += 1;
    }
    if let Some(n) = name.as_ref().filter(|n| vm.child_exists(n)) {
        return err(format!(
            "interpreter named \"{n}\" already exists, cannot create"
        ));
    }
    ok(Value::string(vm.create_child(name, safe)))
}

/// `interp recursionlimit path ?newlimit?` — get/set a (possibly child) interp's
/// recursion bound.
fn interp_recursionlimit_cmd(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    let (path, newlimit) = match rest {
        [path] => (path.to_str(), None),
        [path, nl] => (path.to_str(), Some(nl.to_str().to_string())),
        _ => return err("wrong # args: should be \"interp recursionlimit path ?newlimit?\""),
    };
    let result = if path.is_empty() {
        vm.recursion_limit_apply(newlimit.as_deref())
    } else if let Some(r) = vm.child_recursion_limit_apply(&path, newlimit.as_deref()) {
        r
    } else {
        return err(format!("could not find interpreter \"{path}\""));
    };
    match result {
        Ok(n) => ok(Value::int(n)),
        Err(m) => err(m),
    }
}

/// `interp limit path limitType ?-option value ...?` — query/configure the
/// `commands` or `time` limit on a child interp (stored, not enforced).
fn interp_limit_cmd(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    let (path, ltype, opts) = match rest {
        [path, ltype, opts @ ..] => (path.to_str(), ltype.to_str(), opts),
        _ => {
            return err(
                "wrong # args: should be \"interp limit path limitType ?-option value ...?\"",
            );
        }
    };
    // Validate the limit type before the current-interp guard so that a bad
    // type is reported ahead of the inaccessibility error (interp-35.3 vs .23).
    if &*ltype != "commands" && &*ltype != "time" {
        return err(format!(
            "bad limit type \"{ltype}\": must be commands or time"
        ));
    }
    if path.is_empty() {
        return err("limits on current interpreter inaccessible");
    }
    let id = match vm.resolve_interp_path(&path) {
        Ok(id) => id,
        Err(c) => return c,
    };
    let ltype = ltype.to_string();
    let opts = opts.to_vec();
    match vm.in_interp(id, |vm| vm.limit_apply(&ltype, &opts)) {
        Ok(v) => ok(v),
        Err(m) => err(m),
    }
}

/// `interp eval path arg ?arg ...?` — empty path evaluates like `eval`; a
/// named path (possibly multi-word, addressing a grandchild) routes into
/// that interpreter.
fn interp_eval_cmd(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    let [path, scripts @ ..] = rest else {
        return err("wrong # args: should be \"interp eval path arg ?arg ...?\"");
    };
    if scripts.is_empty() {
        return err("wrong # args: should be \"interp eval path arg ?arg ...?\"");
    }
    let p = path.to_str();
    let script = scripts
        .iter()
        .map(|v| v.to_str().to_string())
        .collect::<Vec<_>>()
        .join(" ");
    if p.is_empty() {
        return match vm.eval_source(&script) {
            Ok(c) => c,
            Err(e) => err(e.message),
        };
    }
    match vm.resolve_interp_path(&p) {
        Ok(id) => vm.eval_in_interp(id, &script),
        Err(c) => c,
    }
}

/// `interp delete ?path ...?` — destroy each named interpreter.
fn interp_delete_cmd(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    for path in rest {
        let p = path.to_str();
        if p.is_empty() {
            return err(format!("could not find interpreter \"{p}\""));
        }
        match vm.resolve_interp_path(&p) {
            Ok(id) if !vm.delete_interp(id) => {
                return err(format!("could not find interpreter \"{p}\""));
            }
            Ok(_) => {}
            Err(c) => return c,
        }
    }
    ok(Value::empty())
}

/// `interp bgerror path ?cmdPrefix?` — get/set the background-error handler.
/// `interp children ?path?` — children of the current interp, or (one level
/// down) of the named child.
fn interp_children_cmd(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    let names = match rest {
        [] => vm.child_names(),
        [path] if path.to_str().is_empty() => vm.child_names(),
        [path] => match vm.child_child_names(&path.to_str()) {
            Some(names) => names,
            None => return err(format!("could not find interpreter \"{}\"", path.to_str())),
        },
        _ => return err("wrong # args: should be \"interp children ?path?\""),
    };
    ok(Value::list(names.into_iter().map(Value::string).collect()))
}

fn interp_bgerror_cmd(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    let (path, bargs) = match rest {
        [path] | [path, _] => (path.to_str(), &rest[1..]),
        _ => return err("wrong # args: should be \"interp bgerror path ?cmdPrefix?\""),
    };
    if path.is_empty() {
        ok(vm.bgerror_apply(bargs))
    } else {
        match vm.resolve_interp_path(&path) {
            Ok(id) => {
                let bargs = bargs.to_vec();
                ok(vm.in_interp(id, |vm| vm.bgerror_apply(&bargs)))
            }
            Err(c) => c,
        }
    }
}

/// `interp debug path ?-frame ?bool??` — the per-interp frame-debug switch.
fn interp_debug_cmd(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    let Some((path, dargs)) = rest.split_first() else {
        return err("wrong # args: should be \"interp debug path ?-frame ?bool??\"");
    };
    let p = path.to_str();
    let res = if p.is_empty() {
        vm.debug_apply(dargs)
    } else {
        match vm.resolve_interp_path(&p) {
            Ok(id) => {
                let dargs = dargs.to_vec();
                vm.in_interp(id, |vm| vm.debug_apply(&dargs))
            }
            Err(c) => return c,
        }
    };
    match res {
        Ok(v) => ok(v),
        Err(m) => err(m),
    }
}

/// `interp hidden ?path?` — the hidden-command names of the current or named
/// interp.
fn interp_hidden_cmd(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    let path = rest
        .first()
        .map(|v| v.to_str().to_string())
        .unwrap_or_default();
    let names = if path.is_empty() {
        vm.own_hidden_names()
    } else {
        match vm.child_hidden_names(&path) {
            Some(n) => n,
            None => return err(format!("could not find interpreter \"{path}\"")),
        }
    };
    ok(Value::list(names.into_iter().map(Value::string).collect()))
}

/// `interp hide|expose path cmd` — move a command between the (current or
/// child) interp's visible and hidden tables.
fn interp_hidectl_cmd(vm: &mut Vm, hide: bool, rest: &[Value]) -> Completion<Value> {
    // `interp hide   path cmdName     ?hiddenCmdName?`
    // `interp expose path hiddenName  ?cmdName?`
    let (path, cmd, token) = match rest {
        [path, cmd] => (path.to_str(), cmd.to_str(), cmd.to_str()),
        [path, cmd, token] => (path.to_str(), cmd.to_str(), token.to_str()),
        _ => {
            let usage = if hide {
                "wrong # args: should be \"interp hide path cmdName ?hiddenCmdName?\""
            } else {
                "wrong # args: should be \"interp expose path hiddenCmdName ?cmdName?\""
            };
            return err(usage);
        }
    };
    // A safe interpreter may not touch the hidden-command table of itself or
    // any of its children (the check is on the *executing* interp).
    if vm.is_safe() {
        let verb = if hide { "hide" } else { "expose" };
        return err(format!(
            "permission denied: safe interpreter cannot {verb} commands"
        ));
    }
    if path.is_empty() {
        let result = if hide {
            vm.hide_command(&cmd, &token)
        } else {
            vm.expose_own_command(&cmd, &token)
        };
        match result {
            Ok(()) => ok(Value::empty()),
            Err(problem) => problem.into_completion(),
        }
    } else {
        match vm.child_hide(&path, &cmd, &token, hide) {
            Ok(true) => ok(Value::empty()),
            Ok(false) => err(format!("could not find interpreter \"{path}\"")),
            Err(problem) => problem.into_completion(),
        }
    }
}

/// `interp invokehidden path ?-namespace ns? ?--? cmd ?arg ...?` — invoke a
/// hidden command in the named child.
fn interp_invokehidden_cmd(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    let usage =
        "wrong # args: should be \"interp invokehidden path ?-namespace ns? ?--? cmd ?arg ..?\"";
    let [path, tail @ ..] = rest else {
        return err(usage);
    };
    if tail.is_empty() {
        return err(usage);
    }
    if vm.is_safe() {
        return err("not allowed to invoke hidden commands from safe interpreter");
    }
    // Skip the option flags we don't model (`-namespace ns`, `--`).
    let mut i = 0;
    while i < tail.len() {
        match &*tail[i].to_str() {
            "-namespace" => i += 2,
            "--" => {
                i += 1;
                break;
            }
            s if s.starts_with('-') => i += 1,
            _ => break,
        }
    }
    let Some(cmd) = tail.get(i) else {
        return err(usage);
    };
    let p = path.to_str();
    vm.invoke_hidden_in_child(&p, &cmd.to_str(), &tail[i + 1..])
        .unwrap_or_else(|| err(format!("could not find interpreter \"{p}\"")))
}

/// `interp` — child-interpreter creation, evaluation, and the single-interp
/// alias form. A child is a full `Vm` sharing the parent's output and compile
/// service (see [`Vm::create_child`]); `interp eval`/`delete`/`issafe`/… address
/// the current interp (empty path) or a named child.
fn cmd_interp(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some((sub, rest)) = args.split_first() else {
        return err("wrong # args: should be \"interp cmd ?arg ...?\"");
    };
    match &*sub.to_str() {
        "create" => interp_create_cmd(vm, rest),
        // interp alias srcPath srcCmd targetPath targetCmd ?arg ...?
        "alias" => match rest {
            [src_path, src_cmd, target_path, target @ ..] if !target.is_empty() => {
                // Routing (same-interp / parent→child / child→parent), the
                // written-name → key qualification (#934), and C's
                // `TclPreventAliasLoop` walk all live on the Vm.
                let res = vm.interp_alias_create(
                    &src_path.to_str(),
                    &src_cmd.to_str(),
                    &target_path.to_str(),
                    target.to_vec(),
                );
                if res.code.is_ok() {
                    ok(src_cmd.clone())
                } else {
                    res
                }
            }
            _ => err(
                "wrong # args: should be \"interp alias srcPath srcCmd targetPath targetCmd ?arg ...?\"",
            ),
        },
        "exists" => match rest {
            [] => ok(Value::int(1)),
            [path] => {
                let p = path.to_str();
                ok(Value::bool(p.is_empty() || vm.child_exists(&p)))
            }
            _ => err("wrong # args: should be \"interp exists ?path?\""),
        },
        // interp eval path arg ?arg ...? — empty path is the current interp
        // (evaluate like `eval`); a named path routes into that child.
        "eval" => interp_eval_cmd(vm, rest),
        // interp delete ?path ...?
        "delete" => interp_delete_cmd(vm, rest),
        "issafe" => match rest {
            [] => ok(Value::bool(vm.is_safe())),
            [path] => {
                let p = path.to_str();
                if p.is_empty() {
                    ok(Value::bool(vm.is_safe()))
                } else {
                    match vm.child_is_safe(&p) {
                        Some(s) => ok(Value::bool(s)),
                        None => err(format!("could not find interpreter \"{p}\"")),
                    }
                }
            }
            _ => err("wrong # args: should be \"interp issafe ?path?\""),
        },
        "slaves" | "children" => interp_children_cmd(vm, rest),
        "recursionlimit" => interp_recursionlimit_cmd(vm, rest),
        // interp hide path cmd / interp expose path cmd
        "hide" | "expose" => interp_hidectl_cmd(vm, &*sub.to_str() == "hide", rest),
        "hidden" => interp_hidden_cmd(vm, rest),
        "invokehidden" => interp_invokehidden_cmd(vm, rest),
        "debug" => interp_debug_cmd(vm, rest),
        "bgerror" => interp_bgerror_cmd(vm, rest),
        "limit" => interp_limit_cmd(vm, rest),
        // `interp marktrusted path` — clear a child's safe flag. A safe
        // interpreter may not mark anything trusted (checked on the executor).
        "marktrusted" => match rest {
            [path] => {
                if vm.is_safe() {
                    return err("permission denied: safe interpreter cannot mark trusted");
                }
                vm.child_mark_trusted(&path.to_str());
                ok(Value::empty())
            }
            _ => err("wrong # args: should be \"interp marktrusted path\""),
        },
        // Channel sharing/transfer between interps. Channels are per-interp in
        // this model; accept the operation so a script that wires up shared
        // channels at top level runs to completion (the dependent reads are
        // covered by the per-interp channel table, not a global one).
        "share" | "transfer" => ok(Value::empty()),
        other => err(format!(
            "bad option \"{other}\": must be alias, aliases, bgerror, cancel, \
             children, create, debug, delete, eval, exists, expose, hide, \
             hidden, issafe, invokehidden, limit, marktrusted, recursionlimit, \
             share, slaves, target, or transfer"
        )),
    }
}

/// The standard library `parray` proc, defined on demand by `auto_load`.
const PARRAY_SRC: &str = r#"proc ::parray {a {pattern *}} {
    upvar 1 $a array
    set maxl 0
    foreach name [lsort [array names array $pattern]] {
        if {[string length $name] > $maxl} {
            set maxl [string length $name]
        }
    }
    set maxl [expr {$maxl + [string length $a] + 2}]
    foreach name [lsort [array names array $pattern]] {
        set nameString [format %s(%s) $a $name]
        puts stdout [format "%-*s = %s" $maxl $nameString $array($name)]
    }
}"#;

/// `auto_load command` — there is no on-disk autoloader, but a few standard
/// library procs are provided on demand (so e.g. `info body ::parray` works).
/// Returns 1 if the command was made available, 0 otherwise.
fn cmd_auto_load(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let name = args
        .first()
        .map(|v| v.to_str().to_string())
        .unwrap_or_default();
    // The shared `namespace tail` op (separator-run-aware, matching C).
    let simple = std::str::from_utf8(tcl_cmd_core::namespace::tail(name.as_bytes()))
        .expect("subslice of valid UTF-8");
    let src = match simple {
        "parray" => PARRAY_SRC,
        _ => return ok(Value::int(0)),
    };
    match vm.eval_source(src) {
        Ok(c) if c.code.is_ok() => ok(Value::int(1)),
        Ok(c) => c,
        Err(e) => err(e.message),
    }
}

/// `subst`'s option words. C's `TclSubstOptions` (`tclCmdMZ.c:3341`) resolves
/// them with `Tcl_GetIndexFromObj` at flags `0`, so abbreviations match and the
/// *empty* word — which prefixes all three entries — is `ambiguous`, not `bad`.
/// Shared with the WASM runtime through the one `tcl-cmd-core::prefix` matcher.
const SUBST_OPTIONS: tcl_cmd_core::prefix::OptionTable<'static> =
    tcl_cmd_core::prefix::OptionTable::abbreviating(
        "option",
        &["-nobackslashes", "-nocommands", "-novariables"],
    );

/// `subst ?-nobackslashes? ?-nocommands? ?-novariables? string` — perform
/// backslash / command / variable substitution on a string.
fn cmd_subst(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    // C (`TclNRSubstObjCmd`): the *last* argument is the string and every
    // argument before it is an option, matched by unique abbreviation. So a
    // non-option word anywhere but last is a `bad option` error (subst-1.2/7.1),
    // and `-nov`/`-nob`/`-noc` are accepted as prefixes (subst-7.7).
    let Some((string, opts)) = args.split_last() else {
        return err(
            "wrong # args: should be \"subst ?-nobackslashes? ?-nocommands? ?-novariables? string\"",
        );
    };
    let (mut backslashes, mut commands, mut variables) = (true, true, true);
    for opt in opts {
        match SUBST_OPTIONS.index_of(opt.to_str().as_bytes()) {
            Ok(0) => backslashes = false,
            Ok(1) => commands = false,
            Ok(_) => variables = false,
            Err(m) => return err(String::from_utf8_lossy(&m).into_owned()),
        }
    }
    // Defer to the *explicit* stack so a `yield` inside a `[…]` stays yieldable:
    // park the template + switches in `pending_subst`, drained by
    // the trampoline into a scanner-driven subst frame (mirrors `cmd_catch`'s
    // `pending_catch`). The frame's accumulated result replaces this builtin's
    // placeholder; on the native `invoke_command` fallback it runs via a nested
    // drive (not yieldable, as before).
    vm.pending_subst = Some(crate::exec::SubstReq {
        template: string.to_str().to_string(),
        backslashes,
        commands,
        variables,
    });
    ok(Value::empty())
}

/// `puts ?-nonewline? ?channelId? string` — write to the VM's output sink.
fn cmd_puts(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let mut rest = args;
    let mut newline = true;
    if let Some(first) = rest.first()
        && &*first.to_str() == "-nonewline"
    {
        newline = false;
        rest = &rest[1..];
    }
    let (channel, text) = match rest {
        [string] => ("stdout".to_string(), string.to_str().to_string()),
        // `puts ?-nonewline? channelId string`
        [channel, string] => (channel.to_str().to_string(), string.to_str().to_string()),
        _ => {
            return err("wrong # args: should be \"puts ?-nonewline? ?channelId? string\"");
        }
    };
    match crate::cmd_chan::chan_puts(vm, &channel, &text, newline) {
        Ok(()) => ok(Value::empty()),
        Err(e) => err(e),
    }
}

/// `incr varName ?increment?` — add to an integer variable (default 1; missing
/// variable starts at 0). The numeric-tower addition goes through the shared
/// [`ValueOps::int_add`](tcl_syntax::value::ValueOps::int_add) seam (the same one
/// the bytecode `incr` opcodes and the WASM runtime use), so the number-model
/// behaviour is identical everywhere: an overflowing `incr` promotes through
/// the integer tower (`i128`, then an arbitrary-precision bignum), matching
/// tclsh, instead of wrapping or erroring.
fn cmd_incr(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let (name, amount) = match args {
        [name] => (name.to_str(), Value::int(1)),
        [name, inc] => (name.to_str(), inc.clone()),
        _ => return err("wrong # args: should be \"incr varName ?increment?\""),
    };
    // `var_get`/`var_set` parse `base(key)` themselves; an unset variable reads
    // as `None`, which `int_add` treats as zero.
    let cur = vm.var_get(&name);
    let sum = match tcl_syntax::value::ValueOps::int_add(vm, cur.as_ref(), &amount) {
        Ok(v) => v,
        Err(e) => return err(e.message()),
    };
    if let Err(e) = vm.var_set(&name, sum.clone()) {
        return e;
    }
    ok(sum)
}

/// `expr arg ?arg ...?` — concatenate the args and evaluate as an expression.
fn cmd_expr(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    if args.is_empty() {
        return err("wrong # args: should be \"expr arg ?arg ...?\"");
    }
    let joined = args
        .iter()
        .map(|v| v.to_str().to_string())
        .collect::<Vec<_>>()
        .join(" ");
    match vm.eval_expr(&joined) {
        Ok(v) => ok(v),
        Err(e) => completion_from_tcl_error(e),
    }
}

/// Resolve a Tcl index spec against a length, returning a possibly out-of-range
/// signed index (callers clamp/empty as needed). Delegates to the canonical
/// `tcl_cmd_core::index` parser so the inline `lindex`/`string index` opcodes
/// accept every form the commands do — including the arithmetic
/// `integer?[+-]integer?` (`2+0`, `end-1+2`), which a bare `parse::<isize>` does
/// not (the inline `LIST_INDEX` opcode otherwise returned the empty string for
/// `lindex $l $i+1`).
pub(crate) fn resolve_index(spec: &str, len: usize) -> Option<isize> {
    tcl_cmd_core::index::resolve_opt(spec, len).and_then(|i| isize::try_from(i).ok())
}

/// Parse a proc parameter spec (`"a b {c 1} args"`) into params + `has_args`.
pub(crate) fn parse_params(spec: &str) -> Result<(Vec<Param>, bool), String> {
    let parsed = parse_formal_parameters(spec).map_err(|error| error.message())?;
    let has_args = has_trailing_args(&parsed);
    let params = parsed
        .into_iter()
        .map(|parameter| Param {
            name: parameter.name,
            default: parameter.default.map(Value::string),
        })
        .collect();
    Ok((params, has_args))
}

/// `proc name params body` — define a procedure (body is taken pre-compiled
/// from the module; dynamic bodies are not yet supported).
fn cmd_proc(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let [name, params, body_text] = args else {
        return err("wrong # args: should be \"proc name args body\"");
    };
    let name_s = name.to_str();
    // The registration / activation name is qualified with the *current*
    // namespace (so `namespace eval foo { proc bar … }` defines `foo::bar`).
    let reg_name = vm.qualify_name(&name_s);
    // The pre-compiled body cache is keyed by the *runtime-qualified* name. A
    // bare proc name resolves against the current namespace, so two procs that
    // share an unqualified name in different namespaces (`::a::p` vs `::b::p`)
    // must not collide on one cached body — keying by `reg_name` keeps them
    // distinct. Global procs (`reg_name == ::name`) still hit the compiler's
    // `::name`-keyed entry; namespaced bodies the compiler globalised under
    // `::name` simply miss and recompile via `compile_dynamic_body`.
    let body_key = reg_name.clone();
    // `reg_name` is canonical (single `::`s); the shared qualifier split keeps
    // the namespace correct for colon-run sources too (`proc foo:::baz` in an
    // existing `foo` defines `::foo::baz`, tclsh8.6-verified — the old
    // `rsplit_once` derived the bogus namespace `foo:` and errored).
    let namespace = std::str::from_utf8(tcl_cmd_core::namespace::qualifiers(reg_name.as_bytes()))
        .expect("subslice of valid UTF-8")
        .to_string();
    // A namespace-qualified proc name requires its namespace to already exist
    // (C's `TclGetNamespaceForQualName` → `nsPtr == NULL`). An unqualified name
    // lands in the current namespace, which always exists (proc-1.2).
    if name_s.contains("::") && !vm.namespace_accepts_command_definition(&namespace) {
        return err(format!(
            "can't create procedure \"{name_s}\": unknown namespace"
        ));
    }
    let (params_vec, has_args) = match parse_params(&params.to_str()) {
        Ok(p) => p,
        Err(e) => return err(e),
    };
    // The pre-compiled `module_procs` cache is keyed by name, and the compiler
    // keeps only the *first* definition of a name, so it is stale for a
    // *redefinition* (`proc p {} …` then `proc p {bar} …` — common in test
    // suites). A redefinition therefore compiles the body actually supplied to
    // this call; the first definition keeps the pre-compiled fast path (so
    // loading a library does not recompile every proc body).
    let body_str = body_text.to_str();
    let pre = (!vm.is_proc_defined(&reg_name))
        .then(|| vm.module_proc(&body_key))
        .flatten();
    let body = match pre {
        Some(b) => b,
        None => match vm.compile_dynamic_body(&body_str) {
            Some(b) => b,
            None => return err(format!("proc \"{name_s}\": could not compile body")),
        },
    };
    vm.define_proc(ProcDef {
        name: reg_name,
        namespace,
        params: params_vec,
        has_args,
        body,
        body_src: body_text.clone(),
        usage_name: None,
        compiled_epoch: 0,
        compiled_profile_generation: vm.profile_generation(),
    });
    ok(Value::empty())
}

/// Parse a `return -code` word: the named codes (`ok`/`error`/`return`/`break`/
/// `continue`) or an integer (`return -code N`, which becomes [`Code::Other`] for
/// N outside `0..=4`). An unparseable word falls back to `Ok`.
fn parse_code(s: &str) -> Code {
    match s {
        "ok" | "0" => Code::Ok,
        "error" => Code::Error,
        "return" => Code::Return,
        "break" => Code::Break,
        "continue" => Code::Continue,
        _ => Value::string(s)
            .as_int()
            .ok()
            .and_then(|n| i32::try_from(n).ok())
            .map_or(Code::Ok, Code::from_int),
    }
}

/// Build an options dict value `-code N -level L [-errorcode ..] [-errorinfo ..]`.
pub(crate) fn options_dict(code: Code, level: i64, extra: &[(&str, Value)]) -> Value {
    let mut items = vec![
        Value::string("-code"),
        Value::int(code.as_int()),
        Value::string("-level"),
        Value::int(level),
    ];
    for (k, v) in extra {
        items.push(Value::string(*k));
        items.push(v.clone());
    }
    Value::list(items)
}

/// An `ERROR` completion carrying an `-errorcode` (e.g. `TCL BINARY DECODE
/// INVALID`), so `$errorCode` is set when the error propagates.
pub(crate) fn err_with_code(message: impl Into<String>, code: &str) -> Completion<Value> {
    let options = options_dict(Code::Error, 0, &[("-errorcode", Value::string(code))]);
    Completion::new(Code::Error, Value::string(message.into()), options)
}

/// An `ERROR` completion carrying a structured Tcl lookup error code.
///
/// Error codes are Tcl lists, not space-delimited diagnostic strings: a
/// command or subcommand name may itself contain whitespace or list syntax.
/// Keep the encoding here, at the completion-construction boundary, and use
/// the shared list owner for quoting each element.
pub(crate) fn lookup_error(
    message: impl Into<String>,
    lookup_kind: &str,
    name: &str,
) -> Completion<Value> {
    let code = tcl_syntax::list::join_list(["TCL", "LOOKUP", lookup_kind, name]);
    err_with_code(message, &code)
}

/// The canonical error completion for a failed Tcl command lookup.
///
/// Attach this at lookup-producing sites instead of inferring identity from an
/// arbitrary error message later: a user is allowed to raise the same message
/// with the ordinary `NONE` error code.
pub(crate) fn command_lookup_error(name: &str) -> Completion<Value> {
    lookup_error(format!("invalid command name \"{name}\""), "COMMAND", name)
}

/// Convert an internal expression/helper failure into its Tcl completion
/// without losing either a propagating control code or an explicit
/// `-errorcode` supplied by the registry or command implementation.
pub(crate) fn completion_from_tcl_error(error: TclError) -> Completion<Value> {
    if let Some(code) = error.code {
        return Completion::new(code, Value::string(error.message), Value::empty());
    }
    match error.error_code {
        Some(code) => err_with_code(error.message, &code),
        None => err(error.message),
    }
}

/// The options dict a completion exposes to `catch`/`try` (`Tcl_GetReturnOptions`):
/// the carried dict when it has one, otherwise a faithful one built from the
/// code — every completion reads back at least `-code N -level 0`, so an OK body
/// yields `-code 0 -level 0` (not the empty value the bare completion carries).
pub(crate) fn completion_options(comp: &Completion<Value>) -> Value {
    let empty = comp.options.as_list().map_or(true, |l| l.is_empty());
    if empty {
        options_dict(comp.code, 0, &[])
    } else {
        comp.options.clone()
    }
}

/// Resolve the `-errorcode` an error completion publishes to `$errorCode`.
///
/// An explicit `-errorcode` in the completion's options wins (set by
/// `error` / `throw` / `return -errorcode`, even when it is the literal
/// `NONE`). A bare builtin error (`err(...)`, which carries no options) defaults
/// to `NONE` — except a "wrong # args" usage error, which C's
/// `Tcl_WrongNumArgs` tags `TCL WRONGARGS`. Matching that lets the many
/// `… errors` tests (join-2.1, split-2.1, …) see the right `$errorCode` without
/// retagging every wrong-args call site.
pub(crate) fn resolved_error_code(comp: &Completion<Value>) -> Value {
    if let Some(ec) = opt_get(&comp.options, "-errorcode") {
        return ec;
    }
    Value::string(default_error_code(&comp.result.to_str()))
}

/// The `errorCode` a bare builtin error (no carried `-errorcode`) defaults to,
/// keyed off its message — the codes C's `Tcl_WrongNumArgs` / list parser
/// (`tclUtil.c`) attach. Only consulted when the completion carries no explicit
/// `-errorcode`, so a user `error msg` (which sets `-errorcode NONE`) is never
/// reclassified. Anything unrecognised is `NONE`.
fn default_error_code(message: &str) -> &'static str {
    if message.starts_with("wrong # args:") {
        "TCL WRONGARGS"
    } else if message == "unmatched open brace in list" {
        "TCL VALUE LIST BRACE"
    } else if message == "unmatched open quote in list" {
        "TCL VALUE LIST QUOTE"
    } else if message.starts_with("list element in braces followed by")
        || message.starts_with("list element in quotes followed by")
    {
        "TCL VALUE LIST JUNK"
    } else {
        "NONE"
    }
}

/// Rebuild a return-options dict with its `-level` replaced — the proc-boundary
/// countdown for `return -level N`. Every other key (`-code`
/// and any user options) is preserved; a missing `-level` is appended.
pub(crate) fn with_return_level(options: &Value, new_level: i64) -> Value {
    let mut items = Vec::new();
    let mut have_level = false;
    if let Ok(list) = options.as_list() {
        let mut i = 0;
        while i + 1 < list.len() {
            if &*list[i].to_str() == "-level" {
                items.push(Value::string("-level"));
                items.push(Value::int(new_level));
                have_level = true;
            } else {
                items.push(list[i].clone());
                items.push(list[i + 1].clone());
            }
            i += 2;
        }
    }
    if !have_level {
        items.push(Value::string("-level"));
        items.push(Value::int(new_level));
    }
    Value::list(items)
}

/// Look up a key in an options-dict value, returning the following element.
pub(crate) fn opt_get(options: &Value, key: &str) -> Option<Value> {
    let items = options.as_list().ok()?;
    let mut i = 0;
    while i + 1 < items.len() {
        if &*items[i].to_str() == key {
            return Some(items[i + 1].clone());
        }
        i += 2;
    }
    None
}

/// `return ?-code c? ?-level l? ?-errorcode ec? ?-errorinfo ei? ?value?`.
/// Shared with the `returnStk` opcode (C `INST_RETURN_STK` →
/// `Tcl_SetReturnOptions`), which behaves exactly like
/// `return -options $opts $result`.
pub(crate) fn cmd_return(_vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    // Apply one option pair, from a direct argument or an expanded `-options`
    // dict. `-code`/`-level` drive the completion; every other key (`-errorcode`,
    // `-errorinfo`, or a user option like `-foo`) is preserved in the options
    // dict, later occurrences overriding earlier ones (C's `TclMergeReturnOptions`).
    fn apply(
        key: &str,
        val: &Value,
        ret_code: &mut Code,
        level: &mut i64,
        extra: &mut Vec<(String, Value)>,
    ) {
        match key {
            "-code" => *ret_code = parse_code(&val.to_str()),
            "-level" => *level = val.as_int().unwrap_or(1),
            _ => match extra.iter_mut().find(|(k, _)| k == key) {
                Some(slot) => slot.1 = val.clone(),
                None => extra.push((key.to_string(), val.clone())),
            },
        }
    }

    let mut value = Value::empty();
    let mut ret_code = Code::Ok;
    let mut level = 1i64;
    let mut extra: Vec<(String, Value)> = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let a = args[i].to_str();
        if &*a == "-options" && i + 1 < args.len() {
            // Merge a return-options dict (TIP 90); a later explicit option
            // overrides it, and an odd-sized list is not a dict.
            let dict = &args[i + 1];
            let pairs = match dict.as_list() {
                Ok(p) if p.len() % 2 == 0 => p,
                _ => return err(format!("expected dict but got \"{}\"", dict.to_str())),
            };
            let mut j = 0;
            while j + 1 < pairs.len() {
                apply(
                    &pairs[j].to_str(),
                    &pairs[j + 1],
                    &mut ret_code,
                    &mut level,
                    &mut extra,
                );
                j += 2;
            }
            i += 2;
        } else if a.starts_with('-') && a.len() > 1 && i + 1 < args.len() {
            apply(&a, &args[i + 1], &mut ret_code, &mut level, &mut extra);
            i += 2;
        } else if i == args.len() - 1 {
            value = args[i].clone();
            i += 1;
        } else {
            return err(format!("bad option \"{a}\""));
        }
    }
    let extra_refs: Vec<(&str, Value)> =
        extra.iter().map(|(k, v)| (k.as_str(), v.clone())).collect();
    let options = options_dict(ret_code, level, &extra_refs);
    // `-level 0` makes the requested `-code` take effect *immediately* (the
    // completion IS that code, including `ok` — `return -level 0 -code N` is how
    // `try`/`catch` produce an arbitrary return code). A positive level raises
    // TCL_RETURN carrying `-code`/`-level` in its options; each proc boundary
    // decrements the level and only applies `-code` once it reaches 0, so
    // `return -level 2 -code error` is seen as TCL_RETURN one level up, not an
    // immediate error (the countdown lives at the proc boundary
    // in `exec.rs`).
    let final_code = if level == 0 { ret_code } else { Code::Return };
    Completion::new(final_code, value, options)
}

/// `const varName value` — define an immutable scalar (TIP 677). Re-declaring an
/// existing constant with any value is a silent no-op; declaring over a normal
/// variable or an array (element) is an error. The value is written through the
/// normal scalar path (so write traces fire) and only then flagged constant, so
/// a trace that vetoes the write leaves no constant (var-26.14).
///
/// Shared with the `constImm`/`constStk` opcodes (C's `doConst` block), which
/// pass the name from their LVT slot / the stack.
pub(crate) fn cmd_const(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let [name_v, value] = args else {
        return err("wrong # args: should be \"const varName value\"");
    };
    let name = name_v.to_str();
    let bad = |reason: &str| err(format!("can't make constant \"{name}\": {reason}"));
    // Through the one element-reference owner, not a local re-spelling of its
    // predicate (issue #1458): the two agree today, which is exactly when a
    // copy is cheapest to remove and most likely to drift later.
    if looks_like_element(&name) {
        return bad("name refers to an element in an array");
    }
    if vm.var_is_array(&name) {
        return bad("variable is array");
    }
    if vm.exists_var(&name) {
        // A constant re-`const` is a no-op; a normal variable is an error.
        return if vm.is_constant(&name) {
            ok(Value::empty())
        } else {
            bad("variable already exists")
        };
    }
    if let Err(e) = vm.set_var(&name, value.clone()) {
        return e;
    }
    vm.mark_constant(&name);
    ok(Value::empty())
}

/// `tailcall ?command ?arg …??` — the runtime fallback for forms the codegen
/// does not specialise into the `TAILCALL` opcode (an empty `tailcall`, or a
/// dynamically-built one reached via the generic invoke). No command terminates
/// the proc with an empty result (C: `tailcall` is a no-op return). For a
/// dynamic command the call runs now and its result becomes the proc's,
/// terminating it — a close approximation of the true caller-frame tail call the
/// opcode performs (this fallback runs in the proc's own frame).
fn cmd_tailcall(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    if args.is_empty() {
        return Completion::new(Code::Return, Value::empty(), Value::empty());
    }
    let name = args[0].to_str().to_string();
    let res = vm.invoke_command(&name, &args[1..]);
    if res.code.is_ok() {
        Completion::new(Code::Return, res.result, Value::empty())
    } else {
        res
    }
}

/// `time command ?count?` — evaluate `command` (in the current frame) `count`
/// times (default 1) and report the average duration as a 4-element list
/// `N microseconds per iteration` (C Tcl `Tcl_TimeObjCmd`). `N` is an integer for
/// `count <= 1` and a double otherwise; a body that does not complete `OK`
/// (error / break / continue / return) propagates.
// Averaging elapsed µs over the iteration count intentionally goes through f64
// and narrows back to i64, matching C Tcl's `Tcl_TimeObjCmd` arithmetic.
#[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
fn cmd_time(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    if args.is_empty() || args.len() > 2 {
        return err("wrong # args: should be \"time command ?count?\"");
    }
    let count = if args.len() == 2 {
        match args[1].as_int() {
            Ok(n) => n,
            Err(e) => return err(e.message),
        }
    } else {
        1
    };
    let script = args[0].to_str().to_string();
    let start = vm.host_rc().clock().now_micros();
    let mut i = count;
    while i > 0 {
        match vm.eval_source(&script) {
            Ok(c) if c.code.is_ok() => {}
            Ok(c) => return c,
            Err(e) => return err(e.message),
        }
        i -= 1;
    }
    let total = (vm.host_rc().clock().now_micros() - start) as f64;
    let num = if count <= 1 {
        Value::int(if count <= 0 { 0 } else { total as i64 })
    } else {
        Value::double(total / count as f64)
    };
    ok(Value::list(vec![
        num,
        Value::string("microseconds"),
        Value::string("per"),
        Value::string("iteration"),
    ]))
}

/// `encoding subcommand ?arg …?` — matches the tree-walking runtime
/// (`runtime/rust`): the internal string model is
/// UTF-8, so `convertto`/`convertfrom` pass the data through unchanged, `system`
/// reports `utf-8`, `names` lists the supported set, and `dirs` is accepted and
/// ignored (no encoding-file search). This is a documented simplification — real
/// codepage conversion (cp1252, shiftjis, …) is not implemented on either side.
fn cmd_encoding(_vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some(sub) = args.first() else {
        return err("wrong # args: should be \"encoding subcommand ?arg ...?\"");
    };
    match &*sub.to_str() {
        "dirs" => ok(Value::empty()),
        "system" => ok(Value::string("utf-8")),
        "names" => ok(Value::string("utf-8 unicode ascii iso8859-1")),
        "convertto" | "convertfrom" => {
            if args.len() < 2 {
                return err("wrong # args: should be \"encoding convertto ?encoding? data\"");
            }
            ok(args.last().expect("len >= 2").clone())
        }
        other => err(format!(
            "unknown or ambiguous subcommand \"{other}\": must be convertfrom, convertto, dirs, names, or system"
        )),
    }
}

/// `error message ?info? ?code?`.
fn cmd_error(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    if args.is_empty() || args.len() > 3 {
        return err("wrong # args: should be \"error message ?errorInfo? ?errorCode?\"");
    }
    let msg = &args[0];
    let ecode = args
        .get(2)
        .cloned()
        .unwrap_or_else(|| Value::string("NONE"));
    // A *non-empty* `info` argument *is* the errorInfo trace: seed it directly and
    // suppress the `error` command's own `while executing` frame (C's
    // `ERR_ALREADY_LOGGED`). An empty (or absent) `info` is treated as absent —
    // the trace seeds from the message and the invoke site logs the frame as for
    // any other command error (error-4.2/4.3).
    let info = args.get(1).map(|v| v.to_str().to_string());
    let info_nonempty = info.as_deref().is_some_and(|s| !s.is_empty());
    if info_nonempty {
        vm.seed_error_info(info.clone().unwrap_or_default());
    }
    let einfo = if info_nonempty {
        info.unwrap_or_default()
    } else {
        msg.to_str().to_string()
    };
    let options = options_dict(
        Code::Error,
        0,
        &[("-errorcode", ecode), ("-errorinfo", Value::string(einfo))],
    );
    Completion::new(Code::Error, msg.clone(), options)
}

/// `break`.
fn cmd_break(_vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    if !args.is_empty() {
        return err("wrong # args: should be \"break\"");
    }
    Completion::new(Code::Break, Value::empty(), Value::empty())
}

/// `continue`.
fn cmd_continue(_vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    if !args.is_empty() {
        return err("wrong # args: should be \"continue\"");
    }
    Completion::new(Code::Continue, Value::empty(), Value::empty())
}

/// `catch script ?resultVarName? ?optionsVarName?`.
fn cmd_catch(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let (script, resvar, optvar) = match args {
        [s] => (s, None, None),
        [s, r] => (s, Some(r), None),
        [s, r, o] => (s, Some(r), Some(o)),
        _ => {
            return err("wrong # args: should be \"catch script ?resultVarName? ?optionVarName?\"");
        }
    };
    // Defer the body to the *explicit* stack so a `yield` inside it stays
    // yieldable: compile it and hand it to the trampoline via
    // `pending_catch`. A catch frame runs the body and its completion — of any
    // code — is absorbed by `finish_catch` (which binds the result/options vars
    // and yields the status code). Mirrors `cmd_eval`'s `pending_eval`, but
    // catch's completion is caught rather than propagated.
    match vm.compile_source_cached(&script.to_str()) {
        Ok(asm) => {
            vm.pending_catch = Some(crate::exec::CatchReq {
                asm,
                resvar: resvar.map(|v| (*v).clone()),
                optvar: optvar.map(|v| (*v).clone()),
            });
            ok(Value::empty())
        }
        // A body that fails to *parse* is itself a catchable error: run the
        // epilogue directly with the parse-error completion (no body to execute).
        Err(e) => {
            let comp = Completion::new(Code::Error, Value::string(e.message), Value::empty());
            vm.finish_catch(comp, resvar, optvar)
        }
    }
}

impl Vm {
    /// The `catch` epilogue, shared by the explicit-stack catch frame
    /// ([`crate::exec`]'s `unwind`) and the `invoke_command` / parse-error
    /// fallbacks: from the body's completion `comp`, bind the result and options
    /// variables and return the status code as an integer (this `catch` command's
    /// result). An uncatchable `exit` unwind passes straight through; a failure to
    /// write a bind variable propagates as this catch's own error.
    ///
    /// `take_error_info` consumes the accumulated trace, so the same values feed
    /// both the options dict and the `$errorInfo`/`$errorCode` globals; a
    /// non-error completion still clears any in-flight trace so the next error
    /// starts fresh.
    pub(crate) fn finish_catch(
        &mut self,
        comp: Completion<Value>,
        resvar: Option<&Value>,
        optvar: Option<&Value>,
    ) -> Completion<Value> {
        if self.exit_pending() {
            return comp;
        }
        let error_meta = if comp.code == Code::Error {
            let einfo = self.take_error_info().unwrap_or_else(|| {
                opt_get(&comp.options, "-errorinfo").map_or_else(
                    || comp.result.to_str().to_string(),
                    |v| v.to_str().to_string(),
                )
            });
            Some((einfo, resolved_error_code(&comp), self.error_line()))
        } else {
            let _ = self.take_error_info();
            None
        };
        if let Some(r) = resvar
            && let Err(e) = self.set_var(&r.to_str(), comp.result.clone())
        {
            return e;
        }
        if let Some(o) = optvar {
            let opts = match &error_meta {
                Some((einfo, ecode, eline)) => catch_error_options(&comp, ecode, einfo, *eline),
                None => completion_options(&comp),
            };
            if let Err(e) = self.set_var(&o.to_str(), opts) {
                return e;
            }
        }
        if let Some((einfo, ecode, _)) = &error_meta {
            self.publish_error(einfo, ecode);
        }
        ok(Value::int(comp.code.as_int()))
    }

    /// The options dict a *compiled* catch range records when it absorbs
    /// `comp` — the same digestion [`Vm::finish_catch`] performs (consume the
    /// accumulated `errorInfo` trace, resolve the error code and line, publish
    /// the `$errorInfo`/`$errorCode` globals) minus the variable binding,
    /// which the bytecode epilogue does itself from
    /// `PUSH_RESULT`/`PUSH_RETURN_CODE`/`PUSH_RETURN_OPTS`.
    pub(crate) fn digest_catch_options(&mut self, comp: &Completion<Value>) -> Value {
        if comp.code == Code::Error {
            let einfo = self.take_error_info().unwrap_or_else(|| {
                opt_get(&comp.options, "-errorinfo").map_or_else(
                    || comp.result.to_str().to_string(),
                    |v| v.to_str().to_string(),
                )
            });
            let ecode = resolved_error_code(comp);
            let opts = catch_error_options(comp, &ecode, &einfo, self.error_line());
            self.publish_error(&einfo, &ecode);
            opts
        } else {
            let _ = self.take_error_info();
            completion_options(comp)
        }
    }
}

/// The options dict `catch` binds for an error completion. C always attaches
/// `-errorcode`, `-errorinfo`, and `-errorline` to an error's options — even a
/// bare builtin error such as `catch {llength} m opts` — so
/// `dict get $opts -errorcode` never fails. The resolved code
/// and trace already fold in any values a user `error`/`throw`/`return` carried;
/// any *other* carried option (a custom `-foo`) is preserved verbatim.
fn catch_error_options(comp: &Completion<Value>, ecode: &Value, einfo: &str, eline: u32) -> Value {
    let mut items = vec![
        Value::string("-code"),
        Value::int(comp.code.as_int()),
        Value::string("-level"),
        Value::int(0),
        Value::string("-errorcode"),
        ecode.clone(),
        Value::string("-errorinfo"),
        Value::string(einfo),
        Value::string("-errorline"),
        Value::int(i64::from(eline)),
    ];
    if let Ok(carried) = comp.options.as_list() {
        let mut i = 0;
        while i + 1 < carried.len() {
            let k = carried[i].to_str();
            if !matches!(
                &*k,
                "-code" | "-level" | "-errorcode" | "-errorinfo" | "-errorline"
            ) {
                items.push(carried[i].clone());
                items.push(carried[i + 1].clone());
            }
            i += 2;
        }
    }
    Value::list(items)
}

/// `unset ?-nocomplain? ?--? name ...` — remove variables / array elements.
fn cmd_unset(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let mut rest = args;
    let mut nocomplain = false;
    while let Some(first) = rest.first() {
        match &*first.to_str() {
            "-nocomplain" => {
                nocomplain = true;
                rest = &rest[1..];
            }
            "--" => {
                rest = &rest[1..];
                break;
            }
            s if s.starts_with('-') => rest = &rest[1..],
            _ => break,
        }
    }
    for n in rest {
        let name = n.to_str();
        if let Err(c) = vm.unset_one(&name, !nocomplain) {
            return c;
        }
    }
    ok(Value::empty())
}

/// Whether `name` looks like an array element (`arr(k)`, `(k)`) — the test C's
/// `TclObjLookupVarEx` applies, through the one shared owner (issue #1458).
fn looks_like_element(name: &str) -> bool {
    tcl_syntax::naming::split_element_ref(name).is_some()
}

/// The unqualified tail of a possibly `::`-qualified variable name, splitting
/// at the last `::` **anywhere** in the name.
///
/// This is `global`'s rule, and the `::` genuinely may sit inside what looks
/// like an array index: `global v(x::y)` splits to the tail `y)`, which is not
/// an element, so C accepts it — while `global ::x::a(b)` splits to `a(b)` and
/// is refused. Both verified on 8.6.16 and 9.0.4.
fn name_tail(name: &str) -> &str {
    name.rsplit("::").next().unwrap_or(name)
}

/// The tail `variable` checks — qualifier scanning stops at the **first `(`**,
/// so a `::` inside an array index never splits the name.
///
/// `variable` and `global` really do differ here, which is why they cannot
/// share [`name_tail`]: `variable v(x::y)` is `can't define ...: name refers to
/// an element in an array` (the whole word is one element reference, index
/// `x::y`), whereas the identical `global v(x::y)` is accepted because its
/// scan splits at the `::` and lands on the non-element tail `y)`. Splitting
/// `variable` the same way made all five `::`-in-index spellings silently
/// succeed (issue #1458).
fn variable_name_tail(name: &str) -> &str {
    let scan_end = name.find('(').unwrap_or(name.len());
    match name[..scan_end].rfind("::") {
        Some(i) => &name[i + 2..],
        None => name,
    }
}

/// C's refusal when a qualified variable name's parent namespace is absent —
/// `TclObjLookupVar`'s `TCL_LEAVE_ERR_MSG` path, which takes the operation
/// word (`define` for `variable`, `access` for `global`) and reports the name
/// **as written**, with `errorCode` `TCL LOOKUP VARNAME`.
///
/// This check runs *before* the element-name guard: `variable ::nosuch::v(k)`
/// is a missing-namespace error, not an element error, on 8.6.16 and 9.0.4
/// alike. It closes a slice of #1588 (the VM had no parent-namespace check at
/// all); the remaining `upvar` surface of that issue is untouched.
fn missing_parent_ns(vm: &Vm, op: &str, name: &str) -> Option<Completion<Value>> {
    if vm.var_parent_exists(name) {
        return None;
    }
    Some(err_with_code(
        format!("can't {op} \"{name}\": parent namespace doesn't exist"),
        &lookup_var_error_code(name),
    ))
}

/// C's `TCL LOOKUP VARNAME` detail is the scalar/array **base**, while the
/// human-readable message retains the name exactly as written. Keep that
/// split at the command adapter boundary so `variable`, `global`, and `upvar`
/// cannot drift (`::missing::v(k)` reports detail `::missing::v`).
fn lookup_var_error_code(name: &str) -> String {
    let base = tcl_syntax::naming::split_element_ref(name).map_or(name, |(base, _)| base);
    format!("TCL LOOKUP VARNAME {base}")
}

/// C's `MakeUpvar` refusal for a link *target name* that looks like an array
/// element — a link is always to a scalar cell, so `upvar 0 zz (v)` and
/// `global a(b)` are hard errors rather than silent mislinks (issue #1458's
/// companion guard).
fn bad_link_name(name: &str) -> Completion<Value> {
    err_with_code(
        format!(
            "bad variable name \"{name}\": can't create a scalar variable that looks like an array element"
        ),
        "TCL UPVAR LOCAL_ELEMENT",
    )
}

/// Render the typed variable-resolver failure from [`Vm::link_upvar`].
pub(crate) fn upvar_link_error(
    error: crate::interp::UpvarLinkError,
    other: &str,
    local: &str,
) -> Completion<Value> {
    match error {
        crate::interp::UpvarLinkError::TargetNamespace => err_with_code(
            format!("can't access \"{other}\": parent namespace doesn't exist"),
            &lookup_var_error_code(other),
        ),
        crate::interp::UpvarLinkError::Inverted => err_with_code(
            format!(
                "bad variable name \"{local}\": can't create namespace variable that refers to procedure variable"
            ),
            "TCL UPVAR INVERTED",
        ),
        crate::interp::UpvarLinkError::LocalElement => bad_link_name(local),
        crate::interp::UpvarLinkError::LocalNamespace => err_with_code(
            format!("can't create \"{local}\": parent namespace doesn't exist"),
            &lookup_var_error_code(local),
        ),
    }
}

/// `global name ?name ...?` — link names to the global frame.
///
/// Outside a procedure `global` is a documented no-op: `Tcl_GlobalObjCmd`
/// returns `TCL_OK` before touching its arguments when the current frame *is*
/// the global frame. So the element-name guard below is reached only from a
/// proc — at top level and inside `namespace eval`, even `global (x)` and
/// `global a(b)` are accepted. Verified on 8.6.16 and 9.0.4, which agree.
fn cmd_global(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let in_proc = vm.in_proc_frame();
    for n in args {
        let nm = n.to_str();
        if in_proc {
            // Namespace resolution comes first: `global ::nosuch::v(k)` is a
            // missing-namespace error, not an element one.
            if let Some(e) = missing_parent_ns(vm, "access", &nm) {
                return e;
            }
            // C links under the *tail* and reports the tail: `global ::x::a(b)`
            // says `bad variable name "a(b)"`. Verified on 8.6.16 and 9.0.4.
            let tail = name_tail(&nm);
            if looks_like_element(tail) {
                return bad_link_name(tail);
            }
        }
        vm.add_link(&nm, 0, &nm);
    }
    ok(Value::empty())
}

/// `upvar ?level? otherVar localVar ?otherVar localVar ...?`.
fn cmd_upvar(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let mut rest = args;
    // Default level 1 (the caller).
    let mut target = vm.current_level().saturating_sub(1);
    if let Some(first) = rest.first() {
        let s = first.to_str();
        if let Some(abs) = s.strip_prefix('#')
            && let Ok(n) = abs.parse::<usize>()
        {
            target = n;
            rest = &rest[1..];
        } else if !s.is_empty()
            && s.bytes().all(|b| b.is_ascii_digit())
            && let Ok(n) = s.parse::<usize>()
        {
            target = vm.current_level().saturating_sub(n);
            rest = &rest[1..];
        }
    }
    if rest.is_empty() || !rest.len().is_multiple_of(2) {
        return err(
            "wrong # args: should be \"upvar ?level? otherVar localVar ?otherVar localVar ...?\"",
        );
    }
    let mut i = 0;
    while i + 1 < rest.len() {
        let other = rest[i].to_str();
        let local = rest[i + 1].to_str();
        if let Err(error) = vm.link_upvar(target, &other, &local) {
            return upvar_link_error(error, &other, &local);
        }
        i += 2;
    }
    ok(Value::empty())
}

/// The outcome of parsing an `uplevel`/`upvar` level word (C's `TclObjGetFrame`).
enum FrameLevel {
    /// A valid absolute frame level.
    Level(usize),
    /// The word is not a level — the caller treats it as the start of the
    /// command, using the default level.
    NotLevel,
    /// A number-like word that is not a usable level (carries the spelling for
    /// the `bad level "…"` error).
    Bad(String),
}

/// Parse an `uplevel`/`upvar` level word against the current level `cur`,
/// mirroring C's `TclObjGetFrame`: a plain integer is *relative* (`cur - n`, and
/// must land in `0..=cur`); `#n` is *absolute*; a number wider than the C `int`
/// range, or any other purely-numeric word, is a `bad level`; a non-numeric word
/// is not a level at all (the command, run one level up).
fn parse_frame_level(cur: usize, obj: &Value) -> FrameLevel {
    let s = obj.to_str();
    let cur = i64::try_from(cur).unwrap_or(i64::MAX);
    // A plain integer is a relative level. C parses with the `int` range, so a
    // value outside it (e.g. `-0xffffffff` / `[expr -0xffffffff]`) is a bad level
    // rather than a huge relative offset.
    let bad = || FrameLevel::Bad(s.to_string());
    if let Ok(w) = obj.as_int() {
        let Ok(n) = i32::try_from(w) else {
            return bad();
        };
        let level = cur - i64::from(n);
        if n < 0 || level < 0 || level > cur {
            return bad();
        }
        usize::try_from(level).map_or_else(|_| bad(), FrameLevel::Level)
    } else if let Some(rest) = s.strip_prefix('#') {
        // `#n` is an absolute level. `#-0` is level 0; any other negative
        // spelling is bad.
        let parsed = Value::string(rest)
            .as_int()
            .ok()
            .and_then(|w| i32::try_from(w).ok());
        match parsed {
            Some(n) if n >= 0 && !(n > 0 && rest.starts_with('-')) && i64::from(n) <= cur => {
                usize::try_from(n).map_or_else(|_| bad(), FrameLevel::Level)
            }
            _ => bad(),
        }
    } else {
        // Anything non-numeric is the command, not a level.
        FrameLevel::NotLevel
    }
}

/// `uplevel ?level? arg ?arg ...?` — evaluate the concatenated args as a script
/// in the call frame `level` up (default 1, the caller). `#N` selects an
/// absolute level.
fn cmd_uplevel(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let cur = vm.current_level();
    let mut rest = args;
    let mut target = cur.saturating_sub(1);
    let mut explicit = false;
    if let Some(first) = rest.first() {
        match parse_frame_level(cur, first) {
            FrameLevel::Level(level) => {
                target = level;
                rest = &rest[1..];
                explicit = true;
            }
            FrameLevel::NotLevel => {}
            FrameLevel::Bad(name) => return err(format!("bad level \"{name}\"")),
        }
    }
    // A non-`#`/non-numeric first word is the command, run one level up; at the
    // global level there is no such frame, so an implicit level is `bad level "1"`
    // (uplevel-4.0.1/4.0.2).
    if !explicit && cur == 0 {
        return err("bad level \"1\"");
    }
    if rest.is_empty() {
        return err("wrong # args: should be \"uplevel ?level? command ?arg ...?\"");
    }
    let script = rest
        .iter()
        .map(|v| v.to_str().to_string())
        .collect::<Vec<_>>()
        .join(" ");
    // `uplevel 0` (and `uplevel #<current level>`) runs in the *current* frame —
    // no frame swap — so defer it to the explicit stack (yieldable), like `eval`
    // (coroutine-1.7/1.8/1.12). Every other level swaps to a
    // different frame that must be restored afterwards, which a transparent
    // script frame cannot carry, so those keep the nested-drive `eval_at_level`.
    if target == cur {
        return match vm.compile_source_cached(&script) {
            Ok(asm) => {
                vm.pending_eval = Some((asm, Some("uplevel"), None));
                ok(Value::empty())
            }
            Err(e) => err(e.message),
        };
    }
    vm.eval_at_level(target, &script)
}

/// `variable ?name value ...? name ?value?` — namespace variables (currently global).
pub(crate) fn cmd_variable(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    // Inside an `oo::define`/`oo::objdefine` body, `variable` *declares* instance
    // variables rather than linking a namespace variable (C's `TclOODefineVariablesObjCmd`).
    if let Some(res) = crate::cmd_oo::maybe_declare_variable(vm, args) {
        return res;
    }
    if args.is_empty() {
        return err("wrong # args: should be \"variable ?name value ...? name ?value?\"");
    }
    let mut i = 0;
    while i < args.len() {
        let name = args[i].to_str();
        // Namespace variables live in the global frame keyed by their canonical
        // qualified name; alias the unqualified local the body uses to it.
        let qual = vm.qualify_name(&name);
        let local = name_tail(&name).to_owned();
        // C's `TclNRVariableObjCmd` rejects an element-looking target, checking
        // the tail but naming the *given* spelling: `variable ::x::v(k)` says
        // `can't define "::x::v(k)"`. Verified on 8.6.16 and 9.0.4.
        //
        // The check uses `variable`'s own tail rule, NOT the link tail above:
        // qualifier scanning stops at the first `(`, so `v(x::y)` stays one
        // element reference. Only the *check* uses it — the link name keeps its
        // existing derivation, so a name that passes the check links exactly as
        // before.
        //
        // Namespace resolution runs first, so `variable ::nosuch::v(k)` is a
        // missing-namespace error rather than an element one.
        if let Some(e) = missing_parent_ns(vm, "define", &name) {
            return e;
        }
        if looks_like_element(variable_name_tail(&name)) {
            return err_with_code(
                format!("can't define \"{name}\": name refers to an element in an array"),
                "TCL UPVAR LOCAL_ELEMENT",
            );
        }
        vm.declare_namespace_variable(&qual);
        vm.add_link(&local, 0, &qual);
        if i + 1 < args.len() {
            // `set_var` follows the alias just installed to the real cell.
            if let Err(e) = vm.set_var(&local, args[i + 1].clone()) {
                return e;
            }
            i += 2;
        } else {
            i += 1;
        }
    }
    ok(Value::empty())
}

#[cfg(test)]
mod tests {
    use super::parse_params;

    #[test]
    fn shared_proc_parameter_parser_adapts_to_vm_values() {
        let (params, has_args) = parse_params("a {b {hello world}} args").unwrap();
        assert!(has_args);
        assert_eq!(params[1].name, "b");
        let default = params[1].default.as_ref().unwrap().to_str();
        assert_eq!(&*default, "hello world");
        assert_eq!(
            parse_params("{{} default}").err().unwrap(),
            "argument with no name"
        );
        assert_eq!(
            parse_params("{ns::name}").err().unwrap(),
            "formal parameter \"ns::name\" is not a simple name"
        );
    }
}
