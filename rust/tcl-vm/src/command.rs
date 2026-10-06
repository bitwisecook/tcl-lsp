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

use tcl_cmd_core::CmdError;
use tcl_runtime_api::completion_options::{self as shared_options, ErrorOptions, OptionValue};
use tcl_runtime_api::{Code, Completion, NsId, OoId};
use tcl_syntax::formal_params::{has_trailing_args, parse_formal_parameters_in};

use crate::error::TclError;
use crate::interp::{Vm, err, err_wrong_args, ok};
use crate::value::Value;

mod native_procedure;
#[cfg(test)]
mod native_procedure_name_tests;
pub use native_procedure::NativeProcedureCommand;
pub(crate) use native_procedure::{NativeProcedureReference, NativeProcedureResources};

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

    /// Release native-owned state while the retiring command generation is
    /// still bound. Reentrant publication gets an independent generation;
    /// callers remove only the generation whose retirement they initiated.
    fn retire(&self, _vm: &mut Vm) -> Completion<Value> {
        crate::interp::ok(Value::empty())
    }
}

/// A procedure parameter: a name with an optional default.
#[derive(Clone)]
pub struct Param {
    /// Parameter name.
    pub name: tcl_core_types::NameBytes,
    /// Default value, if the parameter is optional.
    pub default: Option<Value>,
}

/// A user procedure retaining its original source and optional prepared body.
#[derive(Clone)]
pub struct ProcDef {
    /// Actual native owners, independent of query and compiler lifetime handles.
    pub(crate) native_resources: Rc<NativeProcedureResources>,
    /// Canonical (namespace-qualified, no leading `::`) proc name.
    pub name: String,
    /// Stable namespace-table token and simple tail that bind the command.
    /// Kept separate from the body namespace because `apply`'s private proc is
    /// stored under `::tcl::apply` while its body runs in the lambda namespace.
    pub command_ns_id: tcl_core_types::NsId,
    /// Simple command-table name paired with [`Self::command_ns_id`].
    pub simple_name: tcl_core_types::NameBytes,
    /// The namespace the body executes in (canonical; `""` = global).
    pub namespace: tcl_core_types::ByteNamespacePath,
    /// That namespace's **token** — C's `procPtr->cmdPtr->nsPtr`, which is what
    /// `TclProcInterpProc` hands `Tcl_PushCallFrame`. The spelling alone is not
    /// the identity: a namespace deleted while this procedure's frame was
    /// running is retained under its old token while a wholly separate one may
    /// already answer to the same name, and the body belongs to the first.
    pub ns_id: tcl_core_types::NsId,
    /// Formal parameters in order.
    pub params: Vec<Param>,
    /// Native formal parsing and activation policy captured at definition.
    pub parameter_grammar: tcl_dialect::ParameterGrammar,
    /// Whether the selected native parameter grammar contains a variadic catch-all.
    pub has_args: bool,
    /// Original Jim formal-list object retained by the declaration. C owns
    /// its compiled formal records independently of this list object.
    pub(crate) native_parameters: Option<Value>,
    /// Original Jim declaration namespace object shared by actual call frames.
    pub(crate) native_jim_namespace: Option<Value>,
    /// Native command-header registration captured at procedure definition.
    pub(crate) native_header: tcl_dialect::NativeProcedureHeaderCompilation,
    /// Native persistent storage captured at definition, shared by activations.
    pub(crate) statics: Option<Rc<crate::vars::StaticVariables>>,
    /// Prepared body and its VM-local provenance. `None` is genuine source
    /// which has never been admitted for execution.
    pub(crate) body: Option<crate::compiled::CompiledUnit>,
    /// Original body source text — used by `info body`.
    pub body_src: Value,
    /// Overrides the leading token of the `wrong # args` usage message. `None`
    /// uses the (simple) proc name; `apply` sets `"apply lambdaExpr"` so the
    /// message reads `wrong # args: should be "apply lambdaExpr …"` rather than
    /// leaking the internal temp proc name (apply-4.*).
    pub usage_name: Option<Vec<Value>>,
    /// Exact invocation words for proc-like adapters whose registered command
    /// is an implementation detail (`apply` lambdas and `TclOO` method bodies).
    /// Ordinary procs derive them from the dispatched head and arguments.
    pub(crate) call_identity: Option<Vec<Value>>,
}

impl ProcDef {
    pub(crate) fn formal_parameters(&self) -> Vec<tcl_syntax::formal_params::ByteFormalParameter> {
        self.params
            .iter()
            .map(|parameter| tcl_syntax::formal_params::ByteFormalParameter {
                name: parameter.name.as_bytes().to_vec(),
                default: parameter.default.as_ref().map(|value| {
                    if self.parameter_grammar == tcl_dialect::ParameterGrammar::Jim
                        && parameter.name.as_bytes() == b"args"
                    {
                        value.string_bytes().to_vec()
                    } else {
                        Vec::new()
                    }
                }),
            })
            .collect()
    }
}

/// One entered procedure and its actual body execution lease. Jim Script
/// activations retain the original declaration without publishing a body cache.
pub(crate) struct PreparedProcedureActivation {
    pub(crate) proc: Rc<ProcDef>,
    pub(crate) body: crate::compiled::CompiledUnit,
    pub(crate) declaration_binding: NativeProcedureCommand,
}

pub(crate) type NativeArgumentUsageRewrite = tcl_cmd_core::ensemble::ArgumentUsageRewrite<Value>;

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
    Proc(NativeProcedureCommand),
    /// An `interp alias` — invoking the command evaluates these target words
    /// (the target command plus any fixed prefix arguments) with the call's own
    /// arguments appended.
    Alias(Rc<Vec<Value>>),
    /// Jim command-prefix alias; target lookup retains the caller namespace.
    CallerAlias(Rc<Vec<Value>>),
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
    Ensemble(Rc<tcl_cmd_core::ensemble::EnsembleToken<EnsembleDef, tcl_core_types::NameBytes>>),
    /// A `TclOO` object or class (`Foo create obj` / `obj method …`): the stable
    /// token keys into the interp's `oo` state (`OoState::objects`/`classes`).
    /// Invoking it dispatches `method args…` against the object (`oo_dispatch`).
    /// Analogous to [`Command::ChildInterp`] — a command backed by a Vm-side
    /// table keyed by identity rather than the mutable command name.
    Object(OoId),
}

/// A `namespace ensemble create`d command (`tclEnsemble.c`).
#[derive(Clone)]
pub struct EnsembleDef {
    /// Actual configured object roles and native prefix-table ownership.
    pub(crate) originals: native_ensemble_objects::NativeEnsembleObjects,
    /// Runtime-owned original implementation, independent of command spelling.
    pub(crate) native: Option<Rc<NativeEnsembleImplementation>>,
    /// The namespace whose exported commands form the default subcommand set and
    /// against which an unmapped subcommand `sub` resolves to `namespace::sub`.
    pub namespace: NsId,
    /// `-map`: subcommand → target command prefix (already qualified).
    ///
    /// An association list, not a hash map: C stores the map as a Tcl dict and
    /// `namespace ensemble configure -map` reads it back in **insertion
    /// order**, so the order is observable and must round-trip. (The runtime's
    /// `EnsembleMap` is the same shape for the same reason.) Ensembles have a
    /// handful of subcommands, so linear lookup is not a concern.
    pub map: Vec<(
        tcl_core_types::NameBytes,
        Vec<Option<tcl_core_types::NameBytes>>,
    )>,
    /// `-subcommands`: the explicit subcommand list, or `None` to use the
    /// namespace's exported commands.
    pub subcommands: Option<Vec<tcl_core_types::NameBytes>>,
    /// `-prefixes`: whether an unambiguous prefix of a subcommand resolves.
    pub prefixes: bool,
    /// `-parameters`: formal parameter names that precede the subcommand word
    /// (`ens p1 p2 sub arg…`); their values thread in after the resolved
    /// target prefix (`target p1 p2 arg…`). Empty for an ordinary ensemble.
    pub parameters: Vec<tcl_core_types::NameBytes>,
    /// `-unknown`: a handler prefix invoked when no subcommand matches.
    pub unknown: Option<Vec<tcl_core_types::NameBytes>>,
}

/// Original native ensemble configuration and its compiled-operation adapter.
pub(crate) struct NativeEnsembleImplementation {
    pub(crate) identity: &'static str,
    pub(crate) namespace: NsId,
    pub(crate) prefixes: bool,
    pub(crate) map: Vec<(String, Vec<String>)>,
}

impl EnsembleDef {
    /// Native compilation requires the actual original dispatch configuration.
    pub(crate) fn native_implementation(&self) -> Option<&NativeEnsembleImplementation> {
        let native = self.native.as_deref()?;
        if self.namespace != native.namespace
            || self.prefixes != native.prefixes
            || self.subcommands.is_some()
            || !self.parameters.is_empty()
            || self.unknown.is_some()
            || self.map.len() != native.map.len()
        {
            return None;
        }
        self.map
            .iter()
            .zip(&native.map)
            .all(|((name, words), (original_name, original_words))| {
                name == original_name
                    && words.len() == original_words.len()
                    && words.iter().zip(original_words).all(|(value, original)| {
                        value
                            .as_ref()
                            .is_some_and(|value| value.as_bytes() == original.as_bytes())
                    })
            })
            .then_some(native)
    }
}

pub(crate) mod native_ensemble_objects;

/// Register the builtin set on `vm`.
pub(crate) fn register_builtins(vm: &mut Vm) {
    register_builtins_with_native_core(vm, None);
}

/// Register this backend's commands under the explicitly selected constructor purpose.
pub(crate) fn register_builtins_with_native_core(
    vm: &mut Vm,
    native: Option<tcl_registry::special_vars::NativeBootstrapProtocol>,
) {
    vm.register_stock_builtin("set", cmd_set);
    refresh_jim_exists(vm);
    vm.register_stock_builtin("puts", cmd_puts);
    vm.register_stock_builtin("source", cmd_source);
    vm.register_stock_builtin("tcl::build-info", cmd_build_info);
    vm.register_stock_builtin("interp", cmd_interp);
    vm.register_stock_builtin("alias", cmd_alias);
    vm.register_stock_builtin("rename", cmd_rename);
    vm.register_stock_builtin("eval", cmd_eval);
    vm.register_stock_builtin("apply", cmd_apply);
    vm.register_stock_builtin("incr", cmd_incr);
    vm.register_stock_builtin("expr", cmd_expr);
    vm.register_stock_builtin("proc", cmd_proc);
    vm.register_stock_builtin("return", cmd_return);
    vm.register_stock_builtin("const", cmd_const);
    vm.register_stock_builtin("tailcall", cmd_tailcall);
    vm.register_stock_builtin("time", cmd_time);
    vm.register_stock_builtin("encoding", cmd_encoding);
    vm.register_stock_builtin("error", cmd_error);
    vm.register_stock_builtin("break", cmd_break);
    vm.register_stock_builtin("continue", cmd_continue);
    vm.register_stock_builtin("catch", cmd_catch);
    vm.register_stock_builtin("global", cmd_global);
    vm.register_stock_builtin("upvar", cmd_upvar);
    vm.register_stock_builtin("uplevel", cmd_uplevel);
    vm.register_stock_builtin("variable", cmd_variable);
    vm.register_stock_builtin("unset", cmd_unset);
    vm.register_stock_builtin("subst", cmd_subst);
    if native.is_none() {
        vm.register_stock_builtin("auto_load", cmd_auto_load);
        vm.register_stock_builtin("auto_import", |_, _| ok(Value::empty()));
    }
    vm.register_stock_builtin("exit", cmd_exit);
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
    if native.is_none_or(tcl_registry::special_vars::NativeBootstrapProtocol::registers_core_binary)
    {
        crate::cmd_binary::register(vm);
    }
    crate::cmd_mathop::register(vm);
    crate::cmd_lseq::register(vm);
    crate::cmd_prefix::register(vm);
    crate::cmd_namespace::register(vm);
    crate::cmd_package::register(vm);
    crate::cmd_regexp::register(vm);
    crate::cmd_switch::register(vm);
    crate::cmd_trace::register(vm);
    if native.is_some() {
        crate::cmd_try::register_for_bootstrap(vm, native);
    } else {
        crate::cmd_try::register(vm);
    }
    if native.is_none_or(tcl_registry::special_vars::NativeBootstrapProtocol::initializes_tcl_oo) {
        crate::cmd_oo::register(vm);
    }
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
    let dialect = vm.native_invocation_dialect();
    let code = match args {
        [] => 0,
        [c] => match dialect.process_exit_conversion.and_then(|conversion| {
            tcl_syntax::number::Numbers::Target(dialect.numbers)
                .parse_exit_status(&c.to_str(), conversion)
        }) {
            Some(code) => code,
            None => {
                return err(format!("expected integer but got \"{}\"", c.to_str()));
            }
        },
        _ => {
            return native_wrong_arguments_message(
                vm,
                "wrong # args: should be \"exit ?returnCode?\"",
            );
        }
    };
    vm.set_exit(code);
    // Unwind everything with an error-coded completion (so it propagates through
    // proc boundaries, unlike `return`); `catch` re-propagates while the exit is
    // pending, and the driver consumes the code before the message is surfaced.
    Completion::new(
        if dialect.family() == Some(tcl_dialect::model::Family::Jim) {
            Code::from_int(6)
        } else {
            Code::Error
        },
        if dialect.family() == Some(tcl_dialect::model::Family::Jim) {
            Value::int(i64::from(code))
        } else {
            Value::string(format!("exit {code}"))
        },
        Value::empty(),
    )
}

/// Publish an actual native arity validation without inferring identity from text.
pub(crate) fn native_wrong_args(vm: &mut Vm, usage: &str) -> Completion<Value> {
    native_wrong_args_bytes(vm, usage.as_bytes())
}

/// Publish authenticated arity with the presenter's original byte header.
pub(crate) fn native_wrong_args_bytes(vm: &mut Vm, usage: &[u8]) -> Completion<Value> {
    completion_from_cmd_error(vm, CmdError::wrong_args_bytes(usage))
}

/// Publish an already rendered authentic arity result, without text inference.
pub(crate) fn native_wrong_arguments_message(
    vm: &mut Vm,
    message: impl AsRef<[u8]>,
) -> Completion<Value> {
    completion_from_cmd_error(
        vm,
        CmdError::wrong_arguments_message_bytes(message.as_ref()),
    )
}

/// `set varName ?newValue?` — read or write a scalar.
fn cmd_set(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let [name, rest @ ..] = args else {
        return native_wrong_args(vm, "set varName ?newValue?");
    };
    if rest.len() > 1 {
        return native_wrong_args(vm, "set varName ?newValue?");
    }
    if let Some(value) = rest.first() {
        return match vm.store_original_named_variable(name, value.clone()) {
            Ok(value) => ok(value),
            Err(error) => error,
        };
    }
    match vm.read_original_named_variable(name) {
        Ok(value) => ok(value),
        Err(error) => error,
    }
}

/// Source original file bytes through the selected native file boundary.
fn cmd_source(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let dialect = vm.native_invocation_dialect();
    let Some(grammar) = dialect.source_file_grammar() else {
        return vm.refuse_host_command("source file protocol is unavailable".into());
    };
    let first = match args
        .first()
        .map(|value| vm.native_name_operand_bytes(value))
        .transpose()
    {
        Ok(bytes) => bytes,
        Err(error) => {
            return completion_from_cmd_error(
                vm,
                tcl_syntax::raw_string::NativeStringAccessError::Unavailable(error).into(),
            );
        }
    };
    let tcl_registry::source_file::SourceFileSelection::Selected(selected) =
        grammar.select_original(args.len(), first.as_deref())
    else {
        return native_wrong_args(vm, grammar.synopsis());
    };
    let path = match vm.native_name_operand_bytes(&args[selected.path_at]) {
        Ok(bytes) => bytes,
        Err(error) => {
            return completion_from_cmd_error(
                vm,
                tcl_syntax::raw_string::NativeStringAccessError::Unavailable(error).into(),
            );
        }
    };
    let path = tcl_core_types::c_string_extent(&path);
    let contents = match vm
        .host()
        .filesystem()
        .map_or(Err(tcl_platform::HostError::NotFound), |fs| {
            fs.read_bytes(path)
        }) {
        Ok(bytes) => bytes,
        Err(error) => {
            return err_with_code(
                [
                    b"couldn't read file \"".as_slice(),
                    path,
                    b"\": ",
                    error.reason().as_bytes(),
                ]
                .concat(),
                b"NONE",
            );
        }
    };
    let scope = selected.no_package.then(|| vm.take_package_file_scope());
    if let Err(error) = vm.record_package_source_file(path) {
        if let Some(scope) = scope {
            vm.restore_package_file_scope(scope);
        }
        return completion_from_cmd_error(vm, error.into());
    }
    vm.enter_package_source_path(path);
    let label = std::str::from_utf8(path).ok();
    if let Some(label) = label {
        vm.push_script(label.to_owned());
    }
    let image = tcl_lexer::SourceImage::native(std::sync::Arc::<[u8]>::from(contents));
    let completion = match vm.eval_source_image_at_internal(&image, None) {
        Ok(completion) => completion,
        Err(error) => completion_from_tcl_error(vm, error),
    };
    if label.is_some() {
        vm.pop_script();
    }
    vm.leave_package_source_path();
    if let Some(scope) = scope {
        vm.restore_package_file_scope(scope);
    }
    vm.settle_package_source_completion(completion)
}

/// `tcl::build-info ?option?` — report build-time configuration. With no
/// argument it returns the full build string; `patchlevel` / `version` /
/// `commit` return their fields and any other word queries a build flag,
/// which is absent (`0`) on this VM because it is a plain release build.
///
/// Both the string and the field split come from
/// [`tcl_dialect::build_info`] keyed by this VM's pinned release, so the
/// answer tracks `--tcl-version` instead of a hardcoded `9.0.4`, and cannot
/// disagree with `runtime/rust`. The registry gates the
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
        _ => native_wrong_arguments_message(
            vm,
            "wrong # args: should be \"tcl::build-info ?option?\"",
        ),
    }
}

/// Jim's variable query uses its actual named-variable door without C traces.
fn cmd_jim_exists(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    if vm.native_invocation_dialect().native_string_protocol()
        != Some(tcl_syntax::native_string::NativeStringProtocol::Jim084)
    {
        return vm.refuse_host_command("Jim exists primitive is unavailable".into());
    }
    let [name] = args else {
        return if args.len() == 2 {
            vm.refuse_host_command("Jim exists selector object protocol is unavailable".into())
        } else {
            native_wrong_args(vm, "?-command|-proc|-alias|-channel|-var? name")
        };
    };
    let name = match vm.native_name_operand_bytes(name) {
        Ok(name) => name,
        Err(error) => return vm.refuse_host_command(error.to_string()),
    };
    let exists = vm.get_var_bytes(&name).is_some();
    if let Some(refusal) = vm.refused_completion() {
        return refusal;
    }
    ok(Value::bool(exists))
}

pub(crate) fn refresh_jim_exists(vm: &mut Vm) {
    let jim = vm.native_invocation_dialect().native_string_protocol()
        == Some(tcl_syntax::native_string::NativeStringProtocol::Jim084);
    let stock = vm.stock_native_identity("exists").as_deref() == Some("exists");
    if jim {
        if stock || vm.lookup_command("exists").is_none() {
            vm.register_stock_builtin("exists", cmd_jim_exists);
        }
    } else if stock {
        vm.remove_registered_command("exists");
    }
    crate::interp::jim_local::refresh(vm);
}

/// Select the native original-object dispatch door before string materialization.
pub(crate) fn original_script_list(
    vm: &mut Vm,
    script: &Value,
    purpose: tcl_registry::native_eval_object::EvalObjectPurpose,
) -> Result<Option<crate::NativeListItems>, Completion<Value>> {
    let Some(protocol) = vm.eval_object_protocol() else {
        return Err(
            vm.refuse_host_command("native script-object dispatch protocol is unavailable".into())
        );
    };
    if !protocol.dispatches_list(purpose, &script.native_object_snapshot()) {
        return Ok(None);
    }
    Ok(script.cached_list_representation().map(|(words, _)| words))
}

/// `eval arg ?arg ...?` — concatenate the arguments with spaces and evaluate the
/// result as a script in the current frame.
fn cmd_eval(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    if args.is_empty() {
        return native_wrong_arguments_message(
            vm,
            "wrong # args: should be \"eval arg ?arg ...?\"",
        );
    }
    let script = if let [single] = args {
        single.clone()
    } else {
        match tcl_cmd_core::list::concat_selected(vm, args) {
            Ok(value) => value,
            Err(error) => return completion_from_cmd_error(vm, error),
        }
    };
    let direct = match original_script_list(
        vm,
        &script,
        tcl_registry::native_eval_object::EvalObjectPurpose::Eval,
    ) {
        Ok(direct) => direct,
        Err(error) => return error,
    };
    if let Some(words) = direct {
        vm.pending.control = Some(crate::cmd_control::ControlState::object_eval(
            script, words, "eval", None,
        ));
        return ok(Value::empty());
    }
    // Defer the body to the *explicit* stack so a `yield` inside it stays
    // yieldable: compile it and hand it to the trampoline via
    // the pending eval request, which pushes a transparent script frame whose result
    // replaces this placeholder. The script frame adds the `("eval" body line N)`
    // errorInfo frame itself on error (eval-2.5; see `Frame::body_label`).
    // A body whose *later* commands do not parse still runs its clean prefix
    // before the error is raised (#1603): C parses one command at a time, so
    // the malformed tail is never reached until the commands ahead of it have
    // run.  `catch`/`try` already prepare their bodies this way.
    match vm.prepare_script_commands_value_for(
        &script,
        tcl_registry::native_eval_object::EvalObjectPurpose::Eval,
    ) {
        Ok(prepared) => match prepared.prefix {
            Some(unit) => {
                vm.pending.eval = Some(crate::exec::EvalReq {
                    selected_frame_restore: None,
                    script: unit.with_source_location(
                        args.first()
                            .filter(|_| args.len() == 1)
                            .and_then(Value::source_location),
                    ),
                    label: Some("eval"),
                    cleanup_proc: None,
                    fatal_tail: prepared.fatal_tail,
                });
                ok(Value::empty())
            }
            // Nothing in the body parses, so raising is all this `eval` does.
            None => prepared.fatal_tail.map_or_else(
                || ok(Value::empty()),
                |tail| {
                    vm.raise_script_parse_failure(
                        tail,
                        args.first()
                            .filter(|_| args.len() == 1)
                            .and_then(Value::source_location),
                    )
                },
            ),
        },
        Err(e) => completion_from_tcl_error(vm, e),
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
pub(crate) fn build_lambda_proc(
    vm: &mut Vm,
    lambda: &Value,
    call_identity: Vec<Value>,
) -> Result<String, Completion<Value>> {
    let Some(protocol) = vm.native_invocation_dialect().native_string_protocol() else {
        return Err(vm.refuse_host_command("native lambda list producer is unavailable".into()));
    };
    let parts = match vm.native_object_list_elements_in(lambda, protocol) {
        Ok(parts) => parts,
        Err(error) => return Err(completion_from_cmd_error(vm, error.into())),
    };
    if parts.len() < 2 || parts.len() > 3 {
        return Err(err(format!(
            "can't interpret \"{}\" as a lambda expression",
            lambda.to_str()
        )));
    }
    let Some(parameter_grammar) = vm.native_invocation_dialect().parameter_grammar() else {
        return Err(err("native parameter grammar is not selected"));
    };
    let body = match vm.capture_procedure_declaration_body(&parts[0], &parts[1]) {
        Ok(body) => body,
        Err(error) => return Err(completion_from_cmd_error(vm, error.into())),
    };
    let (params_vec, has_args) = match parse_params_value(vm, &parts[0], b"") {
        Ok(parsed) => parsed,
        Err(error) => {
            let mut frame = b"\n    (parsing lambda expression \"".to_vec();
            frame.extend_from_slice(&lambda.string_bytes());
            frame.extend_from_slice(b"\")");
            vm.seed_error_info_frame(error.result.string_bytes(), frame);
            return Err(error);
        }
    };
    // Resolve the original namespace object from the global namespace before
    // compiling the lambda; display spelling is not its namespace identity.
    let ns_id = if let Some(original) = parts.get(2) {
        let bytes = match vm.native_name_operand_bytes(original) {
            Ok(bytes) => bytes,
            Err(error) => {
                return Err(
                    vm.refuse_host_command(format!("lambda namespace is unavailable: {error}"))
                );
            }
        };
        match tcl_runtime_api::Namespaces::find_namespace_bytes_checked(
            vm,
            tcl_core_types::ROOT_NS,
            &bytes,
        ) {
            Ok(Some(namespace)) => namespace,
            Ok(None) => {
                let mut message = b"namespace \"".to_vec();
                if !bytes.starts_with(b"::") {
                    message.extend_from_slice(b"::");
                }
                message.extend_from_slice(&bytes);
                message.extend_from_slice(b"\" not found");
                return Err(err(message));
            }
            Err(error) => return Err(completion_from_cmd_error(vm, error.into())),
        }
    } else {
        tcl_core_types::ROOT_NS
    };
    let namespace = vm.namespace_path_for_token(ns_id);
    let native_jim_namespace = vm.uses_native_jim_lookup().then(|| {
        parts
            .get(2)
            .cloned()
            .unwrap_or_else(|| vm.new_jim_declaration_namespace(tcl_core_types::ROOT_NS))
    });
    let name = fresh_apply_name();
    let command_ns_id = vm.definition_namespace_token("tcl::apply");
    vm.define_proc(ProcDef {
        native_resources: Rc::default(),
        name: name.clone(),
        command_ns_id,
        simple_name: tcl_core_types::NameBytes::from(tcl_syntax::naming::written_command_tail(
            name.as_bytes(),
        )),
        namespace,
        ns_id,
        params: params_vec,
        parameter_grammar,
        has_args,
        native_jim_namespace,
        native_parameters: (parameter_grammar == tcl_dialect::ParameterGrammar::Jim)
            .then(|| parts[0].clone()),

        native_header: tcl_dialect::NativeProcedureHeaderCompilation::Absent,
        statics: None,
        body: None,
        body_src: body,
        usage_name: Some(vec![Value::string("apply"), Value::string("lambdaExpr")]),
        call_identity: Some(call_identity),
    });
    Ok(name)
}

/// `apply lambda ?arg ...?` — invoke an anonymous function `{params body ?ns?}`.
/// Implemented by binding the lambda to a temporary command and evaluating a
/// call, so parameter binding and `return` semantics match a normal proc.
///
/// Defers the call to the *explicit* stack via the pending eval request (like
/// `eval`/`uplevel`) rather than `Vm::eval_source`'s nested drive, so a `yield`
/// inside the lambda body stays yieldable, matching `coroutine c apply
/// {lambda}` (`cmd_coroutine` binds the lambda to an internal proc run on the
/// coroutine's own stack), including a bare `apply` called *from inside* a
/// coroutine body. `cleanup_proc` carries the temporary proc's name so it is
/// torn down once the deferred call completes, on every completion path.
fn cmd_apply(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some((lambda, call_args)) = args.split_first() else {
        return native_wrong_args(vm, "apply lambdaExpr ?arg ...?");
    };
    let mut call_identity = Vec::with_capacity(args.len() + 1);
    call_identity.push(Value::string(vm.invoked_name().unwrap_or("apply")));
    call_identity.extend_from_slice(args);
    let name = match build_lambda_proc(vm, lambda, call_identity) {
        Ok(n) => n,
        Err(c) => return c,
    };
    let mut words = Vec::with_capacity(call_args.len() + 1);
    words.push(Value::string(name.as_str()));
    words.extend_from_slice(call_args);
    let script = tcl_syntax::list::join_list(words.iter().map(Value::to_str));
    match vm.compile_script_cached(&script) {
        Ok(script) => {
            vm.pending.eval = Some(crate::exec::EvalReq {
                selected_frame_restore: None,
                script,
                label: None,
                cleanup_proc: Some(name),
                fatal_tail: None,
            });
            ok(Value::empty())
        }
        Err(e) => {
            vm.take_command_unchecked(&name);
            completion_from_tcl_error(vm, e)
        }
    }
}

/// `rename oldName newName` — rename a command, or delete it when `newName` is
/// empty.
fn cmd_rename(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let [old, new] = args else {
        return native_wrong_arguments_message(
            vm,
            "wrong # args: should be \"rename oldName newName\"",
        );
    };
    let old_name = match vm.native_name_operand_bytes(old) {
        Ok(bytes) => bytes,
        Err(error) => return vm.refuse_host_command(error.to_string()),
    };
    let new_name = match vm.native_name_operand_bytes(new) {
        Ok(bytes) => bytes,
        Err(error) => return vm.refuse_host_command(error.to_string()),
    };
    if let Some(error) = vm.jim_local_rename_error(&old_name, new_name.is_empty()) {
        return error;
    }
    // Tcl rejects a rename onto an existing command (leaving both intact), so
    // check the destination before removing the source.
    if !new_name.is_empty()
        && let Some(completion) = rename_destination_failure(vm, &new_name)
    {
        return completion;
    }
    let prepared = match vm.prepare_command_rename(&old_name) {
        Ok(prepared) => prepared,
        Err(error) => return vm.refuse_host_command(error.to_string()),
    };
    let Some((cmd, mut rename)) = prepared else {
        // Renaming to the empty name is a delete; C words the miss accordingly.
        let verb = if new_name.is_empty() {
            "delete"
        } else {
            "rename"
        };
        let reported = match vm.native_rename_reported_operand(&old_name) {
            Ok(name) => name,
            Err(error) => return vm.refuse_host_command(error.to_string()),
        };
        return completion_from_cmd_error(
            vm,
            tcl_cmd_core::CmdError::new_bytes(
                [
                    b"can't ".as_slice(),
                    verb.as_bytes(),
                    b" \"",
                    reported.as_slice(),
                    b"\": command doesn't exist",
                ]
                .concat(),
            ),
        );
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
        // `rename x {}` is a delete: fire the command's `delete` traces
        // (`callback ::old {} delete`, tclsh-pinned) and drop its traces.
        if !trace_handled {
            vm.on_command_removed(&old_key);
        }
    } else {
        // Reserve the exact `(namespace token, simple name)` destination.
        // Its Tcl display can collide with another legal command, so
        // every lifecycle map below uses this private injective key.
        let key = match vm.note_rename_destination(&new_name) {
            Ok(key) => key,
            Err(error) => return vm.refuse_host_command(error.to_string()),
        };
        // Procedure provenance keeps a display projection for compatibility,
        // but it must come from the resolved destination slot. The written
        // `b::y` inside `::a` names `::a::b::y`, not a raw `b::y` key.
        let display_key = key.clone();
        if is_coro {
            crate::cmd_coro::on_command_renamed(vm, &old_key, &key);
        }
        // A proc executes in the namespace it currently lives in, so renaming
        // it across namespaces re-homes its body (C `TclRenameCommand` updates
        // the command's `nsPtr`). `namespace current` inside the body then
        // reports the destination namespace (proc-3.4). The key is canonical,
        // so the shared qualifier split yields its namespace directly.
        let cmd = relocate_renamed_procedure(vm, cmd, &key, display_key);
        // C creates the destination's hash entry before `TclPreventAliasLoop`
        // and deletes it again on a refusal, so the resize that transient entry
        // triggers outlives the rejected rename.
        if let Err(completion) = check_renamed_alias_loop(vm, &key, &old_key, &cmd) {
            return completion;
        }
        vm.install_renamed_command(&mut rename, &key, cmd);
        vm.commit_renamed_command(&rename);
        let key = rename.new_key().to_owned();
        // The rename happened: move every trace to the new key so it keeps
        // firing under the new name, then fire the command's `rename` traces
        // (`callback ::old ::new rename`, both fully qualified — tclsh-pinned).
        // The source is still registered here, matching C's window between
        // creating the destination entry and `Tcl_DeleteHashEntry(oldHPtr)`:
        // a callback resolves *both* names, and reaches the one trace list
        // through either (tclsh-pinned on 8.6.16 and 9.0.4).
        vm.on_command_renamed_traces(&old_key, &key);
        vm.retire_renamed_command_source(&rename);
    }
    ok(Value::empty())
}

fn check_renamed_alias_loop(
    vm: &mut Vm,
    key: &str,
    old_key: &str,
    cmd: &Command,
) -> Result<(), Completion<Value>> {
    let is_alias = matches!(
        cmd,
        Command::Alias(_) | Command::CallerAlias(_) | Command::CrossAlias { .. }
    );
    let loops = if is_alias {
        match vm.alias_chain_loops_for_rename(key, cmd) {
            Ok(loops) => loops,
            Err(error) => {
                vm.forget_rename_destination(key);
                return Err(vm.refuse_host_command(error.to_string()));
            }
        }
    } else {
        false
    };
    if loops {
        let reported = vm
            .actual_native_invocation_dialect()
            .native_name_protocol()
            .and_then(|recipe| {
                let (_, source) = vm.command_slot_parts(old_key)?;
                let (_, destination) = vm.command_slot_parts(key)?;
                recipe
                    .rename_alias_loop_name(source.as_bytes(), destination.as_bytes())
                    .map(<[u8]>::to_vec)
            });
        vm.forget_rename_destination(key);
        let Some(reported) = reported else {
            return Err(
                vm.refuse_host_command("alias rename diagnostic binding is unavailable".into())
            );
        };
        return Err(completion_from_cmd_error(
            vm,
            tcl_cmd_core::CmdError::new_bytes(
                [
                    &b"cannot define or rename alias \""[..],
                    reported.as_slice(),
                    b"\": would create a loop",
                ]
                .concat(),
            ),
        ));
    }
    Ok(())
}

fn rename_destination_failure(vm: &mut Vm, new_name: &[u8]) -> Option<Completion<Value>> {
    match vm.rename_destination_exists(new_name) {
        Ok(true) => {
            let reported = match vm.native_rename_reported_operand(new_name) {
                Ok(name) => name,
                Err(error) => return Some(vm.refuse_host_command(error.to_string())),
            };
            Some(completion_from_cmd_error(
                vm,
                tcl_cmd_core::CmdError::new_bytes(
                    [
                        b"can't rename to \"".as_slice(),
                        reported.as_slice(),
                        b"\": command already exists",
                    ]
                    .concat(),
                ),
            ))
        }
        Ok(false) => None,
        Err(error) => Some(vm.refuse_host_command(error.to_string())),
    }
}

fn relocate_renamed_procedure(
    vm: &mut Vm,
    cmd: Command,
    key: &str,
    display_key: String,
) -> Command {
    match cmd {
        Command::Proc(def) => {
            let (new_ns_id, simple_name) = vm
                .command_slot_parts(key)
                .expect("rename destination carries an exact command slot");
            let new_ns = vm.namespace_path_for_token(new_ns_id);
            let declaration = def.declaration();
            let current = declaration.actual_command_slot();
            if new_ns_id != current.namespace || simple_name != current.simple {
                let (body_namespace_id, new_ns, jim_namespace) = if vm.uses_native_jim_lookup() {
                    let holder = vm
                        .name_policy_protocol()
                        .expect("actual Jim names")
                        .recipe()
                        .jim_procedure_relocation_namespace(simple_name.as_bytes())
                        .expect("actual Jim namespace recipe");
                    if let Some(holder) = holder {
                        let token = vm.intern_jim_namespace_object(holder);
                        (
                            token,
                            vm.namespace_path_for_token(token),
                            Some(vm.new_jim_declaration_namespace(token)),
                        )
                    } else {
                        (
                            declaration.actual_namespace_id(),
                            declaration.actual_namespace(),
                            declaration.retained_jim_namespace(),
                        )
                    }
                } else {
                    (new_ns_id, new_ns, None)
                };
                def.relocate_with_body_namespace(
                    display_key,
                    new_ns_id,
                    simple_name,
                    new_ns,
                    body_namespace_id,
                    jim_namespace,
                );
            }
            Command::Proc(def)
        }
        other => other,
    }
}

/// `interp create`'s option words (`createOptions[]`, `tclInterp.c`), resolved
/// with `Tcl_GetIndexFromObj(…, "option", 0)`: `-s` abbreviates `-safe` and the
/// lone `-` — a prefix of both entries — is `ambiguous option "-"`. Only a word
/// starting with `-` reaches the table, so the empty word is a path, not a miss.
const CREATE_OPTIONS: tcl_cmd_core::prefix::OptionTable<'static> =
    tcl_cmd_core::prefix::OptionTable::abbreviating("option", &["-safe", "--"]);

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
            match CREATE_OPTIONS.index_of_str(&a) {
                Ok(0) => {
                    safe = true;
                    i += 1;
                    continue;
                }
                Ok(_) => {
                    i += 1;
                    last = true;
                }
                Err(e) => return crate::command::completion_from_cmd_error(vm, e),
            }
        }
        if name.is_some() {
            return native_wrong_arguments_message(
                vm,
                "wrong # args: should be \"interp create ?-safe? ?--? ?path?\"",
            );
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
        _ => {
            return native_wrong_arguments_message(
                vm,
                "wrong # args: should be \"interp recursionlimit path ?newlimit?\"",
            );
        }
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
            return native_wrong_arguments_message(
                vm,
                "wrong # args: should be \"interp limit path limitType ?-option value ...?\"",
            );
        }
    };
    // Validate the limit type before the current-interp guard so that a bad
    // type is reported ahead of the inaccessibility error (interp-35.3 vs .23).
    if let Err(e) = crate::interp::LIMIT_TYPES.index_of_str(&ltype) {
        return crate::command::completion_from_cmd_error(vm, e);
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
        Err(error) => completion_from_cmd_error(vm, error),
    }
}

/// `interp eval path arg ?arg ...?` — empty path evaluates like `eval`; a
/// named path (possibly multi-word, addressing a grandchild) routes into
/// that interpreter.
fn interp_eval_cmd(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    let [path, scripts @ ..] = rest else {
        return native_wrong_arguments_message(
            vm,
            "wrong # args: should be \"interp eval path arg ?arg ...?\"",
        );
    };
    if scripts.is_empty() {
        return native_wrong_arguments_message(
            vm,
            "wrong # args: should be \"interp eval path arg ?arg ...?\"",
        );
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
            Err(e) => completion_from_tcl_error(vm, e),
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
        _ => {
            return native_wrong_arguments_message(
                vm,
                "wrong # args: should be \"interp children ?path?\"",
            );
        }
    };
    ok(Value::list(names.into_iter().map(Value::string).collect()))
}

fn interp_bgerror_cmd(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    let (path, bargs) = match rest {
        [path] | [path, _] => (path.to_str(), &rest[1..]),
        _ => {
            return native_wrong_arguments_message(
                vm,
                "wrong # args: should be \"interp bgerror path ?cmdPrefix?\"",
            );
        }
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
    const USAGE: &str = "interp debug path ?-frame ?bool??";
    let Some((path, dargs)) = rest.split_first() else {
        return native_wrong_arguments_message(vm, format!("wrong # args: should be \"{USAGE}\""));
    };
    let p = path.to_str();
    let res = if p.is_empty() {
        vm.debug_apply(dargs, USAGE)
    } else {
        match vm.resolve_interp_path(&p) {
            Ok(id) => {
                let dargs = dargs.to_vec();
                vm.in_interp(id, |vm| vm.debug_apply(&dargs, USAGE))
            }
            Err(c) => return c,
        }
    };
    match res {
        Ok(v) => ok(v),
        Err(error) => completion_from_cmd_error(vm, error),
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
    ok(Value::list(
        names
            .into_iter()
            .map(|name| Value::from_native_string_bytes(name.as_bytes().to_vec()))
            .collect(),
    ))
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

/// `interp invokehidden`'s leading option words (`hiddenOptions[]`,
/// `tclInterp.c`), resolved with `Tcl_GetIndexFromObj(…, "option", 0)`: `-g`
/// and `-n` abbreviate, and the lone `-` is `ambiguous option "-"`. Only a word
/// starting with `-` reaches the table, so an empty word is the command name.
const HIDDEN_OPTIONS: tcl_cmd_core::prefix::OptionTable<'static> =
    tcl_cmd_core::prefix::OptionTable::abbreviating("option", &["-global", "-namespace", "--"]);

/// Consume `interp invokehidden`'s leading options from `tail`, returning the
/// index of the command word. Mirrors C's loop: a word not starting with `-`
/// ends the scan, `-namespace` swallows the following word, and `--` ends it.
/// `-namespace`/`-global` select the invocation namespace, which this engine
/// does not model.
pub(crate) fn skip_hidden_options(vm: &mut Vm, tail: &[Value]) -> Result<usize, Completion<Value>> {
    let mut i = 0;
    while i < tail.len() {
        let word = tail[i].to_str();
        if !word.starts_with('-') {
            break;
        }
        match HIDDEN_OPTIONS.index_of_str(&word) {
            Ok(1) => {
                // `-namespace ns` — the namespace word, when there is one.
                i += 1;
                if i == tail.len() {
                    break;
                }
                i += 1;
            }
            Ok(2) => {
                i += 1;
                break;
            }
            Ok(_) => i += 1,
            Err(e) => return Err(completion_from_cmd_error(vm, e)),
        }
    }
    Ok(i)
}

/// `interp invokehidden path ?-namespace ns? ?-global? ?--? cmd ?arg ...?` —
/// invoke a hidden command in the named child.
fn interp_invokehidden_cmd(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    let usage = "wrong # args: should be \"interp invokehidden path \
                 ?-namespace ns? ?-global? ?--? cmd ?arg ..?\"";
    let [path, tail @ ..] = rest else {
        return native_wrong_arguments_message(vm, usage);
    };
    if tail.is_empty() {
        return native_wrong_arguments_message(vm, usage);
    }
    if vm.is_safe() {
        return err("not allowed to invoke hidden commands from safe interpreter");
    }
    let i = match skip_hidden_options(vm, tail) {
        Ok(i) => i,
        Err(completion) => return completion,
    };
    let Some(cmd) = tail.get(i) else {
        return native_wrong_arguments_message(vm, usage);
    };
    let p = path.to_str();
    vm.invoke_hidden_in_child(&p, &cmd.to_str(), &tail[i + 1..])
        .unwrap_or_else(|| err(format!("could not find interpreter \"{p}\"")))
}

/// Resolve an `interp`-family subcommand word through the shared owner:
/// `dispatch` is the table the word may resolve against, `advertised` the
/// (possibly shorter) table the miss message enumerates — C splits the two the
/// same way for `slaves`.
pub(crate) fn resolve_interp_option_bytes(
    dispatch: &'static [&'static str],
    advertised: &[&'static str],
    word: &[u8],
) -> Result<&'static str, tcl_cmd_core::CmdError> {
    match tcl_cmd_core::prefix::scan(dispatch, word, false) {
        tcl_cmd_core::prefix::Resolution::Exact(i)
        | tcl_cmd_core::prefix::Resolution::UniquePrefix(i) => Ok(dispatch[i]),
        miss => Err(tcl_cmd_core::CmdError::new_bytes(
            tcl_cmd_core::prefix::bad_key_message(
                advertised,
                b"option",
                word,
                matches!(miss, tcl_cmd_core::prefix::Resolution::Ambiguous),
            ),
        )),
    }
}

/// `interp` — child-interpreter creation, evaluation, and the single-interp
/// alias form. A child is a full `Vm` sharing the parent's output and compile
/// service (see [`Vm::create_child`]); `interp eval`/`delete`/`issafe`/… address
/// the current interp (empty path) or a named child.
/// Jim aliases retain the actual argv prefix and resolve it at each call.
fn cmd_alias(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let [name, target, prefix @ ..] = args else {
        return err_with_code(
            r#"wrong # args: should be "alias newname command ?args ...?""#,
            "NONE",
        );
    };
    let mut words = vec![target.clone()];
    words.extend_from_slice(prefix);
    let original = match vm.native_name_operand_bytes(name) {
        Ok(bytes) => bytes,
        Err(error) => return vm.refuse_host_command(error.to_string()),
    };
    let slot = match vm.native_alias_publication_slot(&original) {
        Ok(slot) => slot,
        Err(error) => return vm.refuse_host_command(error.to_string()),
    };
    vm.register_command_in_slot(slot, Command::CallerAlias(Rc::new(words)));
    ok(name.clone())
}

fn cmd_interp(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    if vm
        .actual_native_invocation_dialect()
        .native_jim_lookup_protocol()
        .is_some()
    {
        return if args.is_empty() {
            ok(Value::string(vm.create_jim_child()))
        } else {
            native_wrong_arguments_message(vm, "wrong # args: should be \"interp\"")
        };
    }
    let Some((sub, rest)) = args.split_first() else {
        return native_wrong_arguments_message(
            vm,
            "wrong # args: should be \"interp cmd ?arg ...?\"",
        );
    };
    let sub = match vm.native_interpreter_option_from_original(sub, false) {
        Ok(name) => name,
        Err(error) => return completion_from_cmd_error(vm, error),
    };
    match sub {
        "create" => interp_create_cmd(vm, rest),
        // interp alias srcPath srcCmd targetPath targetCmd ?arg ...?
        "alias" => match rest {
            [src_path, src_cmd, target_path, target @ ..] if !target.is_empty() => {
                // Routing (same-interp / parent→child / child→parent), the
                // written-name → key qualification, and C's
                // `TclPreventAliasLoop` walk all live on the Vm.
                let res = vm.interp_alias_create(
                    &src_path.to_str(),
                    src_cmd,
                    &target_path.to_str(),
                    target.to_vec(),
                );
                if res.code.is_ok() {
                    ok(src_cmd.clone())
                } else {
                    res
                }
            }
            _ => native_wrong_arguments_message(
                vm,
                "wrong # args: should be \"interp alias srcPath srcCmd targetPath targetCmd ?arg ...?\"",
            ),
        },
        "exists" => match rest {
            [] => ok(Value::int(1)),
            [path] => {
                let p = path.to_str();
                ok(Value::bool(p.is_empty() || vm.child_exists(&p)))
            }
            _ => native_wrong_arguments_message(
                vm,
                "wrong # args: should be \"interp exists ?path?\"",
            ),
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
            _ => native_wrong_arguments_message(
                vm,
                "wrong # args: should be \"interp issafe ?path?\"",
            ),
        },
        "slaves" | "children" => interp_children_cmd(vm, rest),
        "recursionlimit" => interp_recursionlimit_cmd(vm, rest),
        // interp hide path cmd / interp expose path cmd
        "hide" | "expose" => interp_hidectl_cmd(vm, sub == "hide", rest),
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
            _ => native_wrong_arguments_message(
                vm,
                "wrong # args: should be \"interp marktrusted path\"",
            ),
        },
        // Channel sharing/transfer between interps. Channels are per-interp in
        // this model; accept the operation so a script that wires up shared
        // channels at top level runs to completion (the dependent reads are
        // covered by the per-interp channel table, not a global one).
        "share" | "transfer" => ok(Value::empty()),
        _ => vm.refuse_host_command("native interpreter worker unavailable".to_owned()),
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
        Err(e) => completion_from_tcl_error(vm, e),
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
            Err(m) => return err(m),
        }
    }
    // Defer to the *explicit* stack so a `yield` inside a `[…]` stays yieldable:
    // park the template + switches in the pending subst request, drained by
    // the trampoline into a scanner-driven subst frame (mirrors `cmd_catch`'s
    // the pending catch request). The frame's accumulated result replaces this builtin's
    // placeholder; on the native `invoke_command` fallback it runs via a nested
    // drive (not yieldable, as before).
    let original = vm
        .actual_native_invocation_dialect()
        .native_string_protocol()
        .filter(|protocol| protocol.is_jim084())
        .map(|_| string.native_lifetime_lease());
    vm.pending.subst = Some(crate::exec::SubstReq {
        original,
        control: crate::subst::SubstitutionControl::Command,
        template: if vm
            .actual_native_invocation_dialect()
            .native_string_protocol()
            .is_some_and(tcl_syntax::native_string::NativeStringProtocol::is_jim084)
        {
            Vec::<u8>::new().into()
        } else {
            match vm.native_name_operand_bytes(string) {
                Ok(bytes) => bytes.to_vec().into(),
                Err(error) => {
                    return completion_from_tcl_error(
                        vm,
                        tcl_syntax::value::ValueError::NativeStringAccess(
                            tcl_syntax::raw_string::NativeStringAccessError::Unavailable(error),
                        )
                        .into(),
                    );
                }
            }
        },
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
    if let Some(first) = rest.first() {
        let nonewline = if vm
            .actual_native_invocation_dialect()
            .native_jim_enum_protocol()
            .is_some()
        {
            const FLAG: &[&str] = &["-nonewline"];
            let table =
                tcl_registry::native_index_lookup::NativeStaticIndexTable::supported_backend(FLAG);
            match vm.native_jim_compare_immediate(first, &table, 0) {
                Ok(matched) => matched,
                Err(error) => return completion_from_cmd_error(vm, error.into()),
            }
        } else {
            &*first.to_str() == "-nonewline"
        };
        if nonewline {
            newline = false;
            rest = &rest[1..];
        }
    }
    let (channel, text) = match rest {
        [string] => ("stdout".to_string(), string.to_str().to_string()),
        // `puts ?-nonewline? channelId string`
        [channel, string] => (channel.to_str().to_string(), string.to_str().to_string()),
        _ => {
            return native_wrong_arguments_message(
                vm,
                "wrong # args: should be \"puts ?-nonewline? ?channelId? string\"",
            );
        }
    };
    match crate::cmd_chan::chan_puts(vm, &channel, &text, newline) {
        Ok(()) => ok(Value::empty()),
        Err(error) => completion_from_cmd_error(vm, error),
    }
}

/// Increment an actual retained variable receiver using its selected native
/// amount, read, object mutation and publication protocols.
fn cmd_incr(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let default = Value::int(1);
    let (name, amount) = match args {
        [name] => (name, &default),
        [name, inc] => (name, inc),
        _ => return native_wrong_args(vm, "incr varName ?increment?"),
    };
    if vm.uses_native_jim_lookup() {
        return vm.increment_original_jim_name(name, amount);
    }
    let name = match vm.native_name_operand_bytes(name) {
        Ok(name) => name,
        Err(error) => {
            return vm
                .refuse_host_command(format!("native increment name is unavailable: {error:?}"));
        }
    };
    match vm.increment_captured_bytes(&name, None, amount) {
        Ok(stored) => Completion::new(Code::Ok, stored.value, stored.options),
        Err(error) => error,
    }
}

/// Evaluate the selected native expression argument protocol.
fn cmd_expr(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some(protocol) = vm.native_invocation_dialect().expression_arguments() else {
        return err("native expression argument protocol is not selected");
    };
    if !protocol.accepts_len(args.len()) {
        return native_wrong_args(vm, protocol.synopsis());
    }
    let source = if let [source] = args {
        source.clone()
    } else {
        let mut parts = Vec::with_capacity(args.len());
        for value in args {
            match vm.native_name_operand_bytes(value) {
                Ok(bytes) => parts.push(bytes),
                Err(refusal) => {
                    return completion_from_tcl_error(
                        vm,
                        tcl_syntax::value::ValueError::NativeStringAccess(
                            tcl_syntax::raw_string::NativeStringAccessError::Unavailable(refusal),
                        )
                        .into(),
                    );
                }
            }
        }
        Value::from_string_bytes(tcl_syntax::list::concat_bytes(
            parts.iter().map(std::convert::AsRef::as_ref),
        ))
    };
    match vm.prepare_expression_value(&source) {
        Ok(node) => {
            vm.pending.expression = Some(crate::exec::ExpressionReq {
                state: tcl_syntax::expr::ExprEvalState::new(node),
                awaiting_array: None,
                normalize: true,
                jim_objects: source.native_jim_expression_objects(),
                restore_primary: source.retain_expression_primary(),
            });
            ok(Value::empty())
        }
        Err(error) => completion_from_tcl_error(vm, error),
    }
}

/// Resolve a Tcl index spec against a length, returning a possibly out-of-range
/// signed index (callers clamp/empty as needed). Delegates to the canonical
/// `tcl_cmd_core::index` parser so the inline `lindex`/`string index` opcodes
/// accept every form the commands do — including the arithmetic
/// `integer?[+-]integer?` (`2+0`, `end-1+2`), which a bare `parse::<isize>` does
/// not (the inline `LIST_INDEX` opcode otherwise returned the empty string for
/// `lindex $l $i+1`).
pub(crate) fn resolve_index(vm: &mut Vm, spec: &str, len: usize) -> Option<isize> {
    tcl_cmd_core::index::resolve_for_ops(vm, spec, len)
        .ok()
        .and_then(|i| isize::try_from(i).ok())
}

pub(crate) fn bad_index(vm: &Vm, spec: &str) -> Completion<Value> {
    match vm.native_invocation_dialect().index_syntax() {
        Some(syntax) => err(tcl_cmd_core::index::bad_index_in(spec, syntax)
            .into_message()
            .expect("Unicode index produces a Unicode diagnostic")),
        None => err("container index dialect is not selected"),
    }
}

/// Parse a proc parameter spec (`"a b {c 1} args"`) into params + `has_args`.
#[cfg(test)]
pub(crate) fn parse_params(spec: &str) -> Result<(Vec<Param>, bool), String> {
    parse_params_in(spec, tcl_dialect::ParameterGrammar::Tcl).map_err(|error| error.message())
}

pub(crate) fn parse_params_in(
    spec: &str,
    grammar: tcl_dialect::ParameterGrammar,
) -> Result<(Vec<Param>, bool), tcl_syntax::formal_params::FormalParameterError> {
    let parsed = parse_formal_parameters_in(spec, grammar)?;
    let has_args = match grammar {
        tcl_dialect::ParameterGrammar::Tcl => has_trailing_args(&parsed),
        tcl_dialect::ParameterGrammar::Jim => {
            parsed.iter().any(|parameter| parameter.name == "args")
        }
    };
    let params = parsed
        .into_iter()
        .map(|parameter| Param {
            name: parameter.name.into(),
            default: parameter.default.map(Value::string),
        })
        .collect();
    Ok((params, has_args))
}

/// Parse original formal objects under the interpreter's selected native policy.
/// Old C releases use `CString` list input at both levels; modern engines keep
/// native list element objects, including the original default value.
fn split_formal_values(
    vm: &Vm,
    value: &Value,
    protocol: tcl_syntax::naming::NativeNameProtocol,
) -> Result<Vec<Value>, tcl_syntax::value::ValueError> {
    use tcl_syntax::value::ValueError;
    if matches!(
        protocol.tcl_version(),
        Some(tcl_dialect::TclVersion::V8_4 | tcl_dialect::TclVersion::V8_5)
    ) {
        let original = vm
            .native_name_operand_bytes(value)
            .map_err(|_| ValueError::CommandProtocolUnavailable("formal list string"))?;
        let bytes = &original[..original
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(original.len())];
        return tcl_syntax::list::split_native_list_bytes(bytes, protocol.string_protocol())
            .map(|items| {
                items
                    .into_iter()
                    .map(|item| Value::from_string_bytes(item.into_owned()))
                    .collect()
            })
            .map_err(|error| ValueError::ListParse {
                error,
                source: bytes.to_vec(),
            });
    }
    vm.native_object_list_elements_in(value, protocol.string_protocol())
        .map(|items| items.as_ref().clone())
}

pub(crate) fn parse_params_value(
    vm: &mut Vm,
    spec: &Value,
    procedure: &[u8],
) -> Result<(Vec<Param>, bool), Completion<Value>> {
    use tcl_syntax::formal_params::{FormalParameterValueError, parse_formal_parameter_values};
    use tcl_syntax::value::ValueError;
    let Some(policy) = vm.name_policy_protocol() else {
        return Err(vm.refuse_host_command("formal storage policy is unavailable".into()));
    };
    let protocol = policy.recipe();
    let split = |value: &Value| split_formal_values(vm, value, protocol);
    let outer = match split(spec) {
        Ok(outer) => outer,
        Err(error) => return Err(completion_from_cmd_error(vm, error.into())),
    };
    let parsed = parse_formal_parameter_values(
        &outer,
        protocol,
        |value, _| split(value),
        |value| {
            vm.native_name_operand_bytes(value)
                .map(|bytes| bytes.to_vec())
                .map_err(|_| ValueError::CommandProtocolUnavailable("formal name string"))
        },
    );
    let parsed = match parsed {
        Ok(parsed) => parsed,
        Err(FormalParameterValueError::Access(error)) => {
            return Err(completion_from_cmd_error(vm, error.into()));
        }
        Err(FormalParameterValueError::Format(error)) => {
            return Err(err(error.message_for_definition(protocol, procedure)));
        }
    };
    if protocol.is_jim084() {
        for parameter in &parsed {
            if parameter.name == b"args"
                && let Some(default) = &parameter.default
                && let Err(error) = vm.native_name_operand_bytes(default)
            {
                return Err(
                    vm.refuse_host_command(format!("variadic local name is unavailable: {error}"))
                );
            }
        }
    }
    let has_args = parsed.iter().enumerate().any(|(index, parameter)| {
        parameter.name == b"args" && (protocol.is_jim084() || index + 1 == parsed.len())
    });
    Ok((
        parsed
            .into_iter()
            .map(|parameter| Param {
                name: parameter.name.into(),
                default: parameter.default,
            })
            .collect(),
        has_args,
    ))
}

/// `proc name params body` — retain a procedure declaration. Its original
/// body is prepared by the activation owner when the procedure is called.
fn cmd_proc(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    use tcl_registry::native_procedure::{
        NativeProcedureDefinitionSelection, ProcedureDefinitionResult,
    };
    let words = vec![tcl_registry::InvocationWord::Dynamic; args.len()];
    let dialect = vm.native_invocation_dialect();
    let arguments = tcl_registry::InvocationArguments::structured(&words).with_dialect(dialect);
    let NativeProcedureDefinitionSelection::Valid(definition) =
        tcl_registry::native_procedure::select_native_procedure_definition(arguments)
    else {
        let usage = tcl_registry::native_procedure::NativeProcedureDefinitionSpec::Core
            .usage(Some(dialect))
            .unwrap_or("proc name args body");
        return native_wrong_args(vm, usage);
    };
    let name = &args[definition.name_at];
    let params = &args[definition.parameters_at];
    let body_text = &args[definition.body_at];
    let written = match vm.native_name_operand_bytes(name) {
        Ok(bytes) => bytes,
        Err(error) => return vm.refuse_host_command(error.to_string()),
    };
    if let Err(completion) = validate_procedure_publication_name(vm, &written) {
        return completion;
    }
    let Some(parameter_grammar) = vm.native_invocation_dialect().parameter_grammar() else {
        return err("native parameter grammar is not selected");
    };
    let body = match vm.capture_procedure_declaration_body(params, body_text) {
        Ok(body) => body,
        Err(error) => return completion_from_cmd_error(vm, error.into()),
    };
    let statics = match definition
        .statics_at
        .map(|index| vm.prepare_procedure_statics_value(&args[index]))
        .transpose()
    {
        Ok(statics) => statics,
        Err(error) => return error,
    };
    let (params_vec, has_args) = match parse_params_value(vm, params, &written) {
        Ok(parsed) => parsed,
        Err(error) => return error,
    };
    let (reg_name, slot, ns_id) = match vm.native_procedure_publication(&written) {
        Ok(publication) => publication,
        Err(error) => return vm.refuse_host_command(error.to_string()),
    };
    let namespace = vm.namespace_path_for_token(ns_id);
    let native_header = match procedure_header_compilation(vm, params, &body) {
        Ok(header) => header,
        Err(completion) => return completion,
    };
    vm.define_proc(ProcDef {
        native_resources: Rc::default(),
        name: reg_name,
        command_ns_id: slot.namespace,
        simple_name: slot.simple,
        namespace,
        ns_id,
        params: params_vec,
        parameter_grammar,
        has_args,
        native_jim_namespace: None,
        native_parameters: (parameter_grammar == tcl_dialect::ParameterGrammar::Jim)
            .then(|| params.clone()),

        native_header,
        statics,
        body: None,
        body_src: body,
        usage_name: None,
        call_identity: None,
    });
    ok(match definition.result {
        ProcedureDefinitionResult::Empty => Value::empty(),
        ProcedureDefinitionResult::NameArgument => name.clone(),
    })
}

/// Reach native header getters only when the original compiler needs them.
fn procedure_header_compilation(
    vm: &mut Vm,
    params: &Value,
    body_text: &Value,
) -> Result<tcl_dialect::NativeProcedureHeaderCompilation, Completion<Value>> {
    let dialect = vm.actual_native_invocation_dialect();
    let mut native_header = tcl_registry::native_procedure::procedure_header_compilation_bytes(
        dialect,
        None,
        None,
        Some(false),
    );
    if native_header == tcl_dialect::NativeProcedureHeaderCompilation::Unknown {
        let Some(protocol) = dialect.native_string_protocol() else {
            return Err(vm.refuse_host_command(
                "native procedure header string producer is unavailable".into(),
            ));
        };
        let parameter_bytes = match params.native_string_bytes_with_integer_formatter(
            protocol,
            vm.host().native_integer_formatter(),
        ) {
            Ok(bytes) => bytes,
            Err(error) => return Err(vm.refuse_host_command(error.to_string())),
        };
        native_header = tcl_registry::native_procedure::procedure_header_compilation_bytes(
            dialect,
            Some(&parameter_bytes),
            None,
            Some(false),
        );
        if native_header == tcl_dialect::NativeProcedureHeaderCompilation::Unknown {
            let body_bytes = match body_text.native_string_bytes(protocol) {
                Ok(bytes) => bytes,
                Err(error) => return Err(vm.refuse_host_command(error.to_string())),
            };
            native_header = tcl_registry::native_procedure::procedure_header_compilation_bytes(
                dialect,
                Some(&parameter_bytes),
                Some(&body_bytes),
                Some(false),
            );
        }
    }
    Ok(native_header)
}

fn validate_procedure_publication_name(
    vm: &mut Vm,
    written: &[u8],
) -> Result<(), Completion<Value>> {
    match vm.native_procedure_holder_exists(written) {
        Ok(true) => {}
        Ok(false) => {
            let Some((message, code)) =
                tcl_registry::native_procedure::procedure_unknown_namespace_error(
                    vm.native_invocation_dialect(),
                    written,
                )
            else {
                return Err(vm.refuse_host_command(
                    "native procedure namespace diagnostic is unavailable".into(),
                ));
            };
            let error = match code {
                Some(code) => tcl_cmd_core::CmdError::with_error_code_bytes(message, code),
                None => tcl_cmd_core::CmdError::new_bytes(message),
            };
            return Err(completion_from_cmd_error(vm, error));
        }
        Err(error) => return Err(vm.refuse_host_command(error.to_string())),
    }
    let Some(policy) = vm.name_policy_protocol() else {
        return Err(vm.refuse_host_command("procedure name validation is unavailable".into()));
    };
    if !policy.recipe().is_jim084() {
        let path = vm.namespace_path_for_token(vm.current_ns_id());
        let slot = match policy
            .recipe()
            .command_lookup_slot(tcl_syntax::naming::NativeNameContext::new(&path), written)
        {
            Ok(slot) => slot,
            Err(error) => return Err(vm.refuse_host_command(error.to_string())),
        };
        match tcl_registry::native_procedure::procedure_name_creation_error_for_policy(
            policy,
            slot.namespace.as_segments().is_empty(),
            slot.simple.as_bytes(),
        ) {
            Ok(()) => {}
            Err(message) => {
                let mut error = tcl_cmd_core::CmdError::new_bytes(message);
                if policy.authority() == tcl_syntax::naming::NamePolicyAuthority::Native {
                    error = error.with_native_string_result(policy.recipe().string_protocol());
                }
                return Err(completion_from_cmd_error(vm, error));
            }
        }
    }
    Ok(())
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
pub(crate) fn err_with_code(
    message: impl AsRef<[u8]>,
    code: impl AsRef<[u8]>,
) -> Completion<Value> {
    let options = options_dict(
        Code::Error,
        0,
        &[("-errorcode", Value::from_string_bytes(code.as_ref()))],
    );
    Completion::new_error_metadata(
        Code::Error,
        Value::from_string_bytes(message.as_ref()),
        options,
    )
}

/// Convert a portable command-layer error without losing its Tcl identity.
pub(crate) fn completion_from_cmd_error(vm: &mut Vm, error: CmdError) -> Completion<Value> {
    if let Some(error) = error.native_access_refusal() {
        return vm.refuse_host_command(error.to_string());
    }
    let details = error.into_byte_details();
    let update = match details.error_code.resolve(|| {
        vm.native_invocation_dialect()
            .wrong_arguments_protocol(Some(tcl_registry::native_wrong_arguments::LogicalWrongArgumentsProvider::Tcl84CoreSimulation))
            .map(|protocol| protocol.error_code().as_bytes().to_vec())
            .ok_or(tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable("wrong arguments"))
    }) {
        Ok(update) => update,
        Err(refusal) => return vm.refuse_host_command(refusal.to_string()),
    };
    let string_result = match details.string_result {
        Some(expected) => match vm
            .actual_native_invocation_dialect()
            .native_string_materialization(None)
        {
            Some(actual) if actual.protocol() == expected => Some(actual),
            _ => return vm.refuse_host_command("selected error String producer".to_owned()),
        },
        None => None,
    };
    let code = vm.apply_primitive_error_code(update);
    let message = details
        .primitive_getter
        .as_ref()
        .map_or(details.message.as_slice(), |getter| {
            getter.eval_result_bytes(&details.message)
        });
    let result = Value::from_string_bytes(message);
    if let Some(materialization) = string_result
        && let Err(error) = result.retain_native_string_representation(materialization)
    {
        return vm.refuse_host_command(error.to_string());
    }
    let mut extra = Vec::with_capacity(3);
    extra.push(("-errorcode", code));
    if let Some(info) = details.error_info {
        extra.push(("-errorinfo", Value::from_string_bytes(info)));
    }
    if let Some(line) = details.error_line {
        extra.push(("-errorline", Value::int(line)));
    }
    Completion::new_error_metadata(Code::Error, result, options_dict(Code::Error, 0, &extra))
}

/// Convert an internal expression/helper failure into its Tcl completion
/// without losing either a propagating control code or an explicit
/// `-errorcode` supplied by the registry or command implementation.
pub(crate) fn completion_from_tcl_error(vm: &mut Vm, error: TclError) -> Completion<Value> {
    match error.into_completion() {
        Ok(completion) => completion,
        Err(error) => vm.refuse_tcl_host_failure(error),
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

/// Settle an adapter-owned control completion under the shared option policy.
///
/// A native completion's empty `options` value normally means “this command did
/// not replace the surrounding carried options”. Fresh control activations need
/// to distinguish that from an explicitly empty option set, so a successful
/// fresh/settled completion materialises the standard `-code 0 -level 0` dict.
/// The dispatcher can then replace the prior state without command-name logic.
pub(crate) fn settle_control_options(
    mut completion: Completion<Value>,
    policy: tcl_runtime_api::completion_options::ControlOptionPolicy,
) -> Completion<Value> {
    if completion.code != Code::Ok {
        return completion;
    }
    let empty = completion
        .options
        .as_list()
        .is_ok_and(|options| options.is_empty());
    if policy.settles_success() || (policy.begins_fresh() && empty) {
        completion.options = options_dict(Code::Ok, 0, &[]);
    }
    completion
}

/// Explicit completion metadata wins; untagged arbitrary guest errors use NONE.
pub(crate) fn resolved_error_code(comp: &Completion<Value>) -> Value {
    opt_get(&comp.options, "-errorcode").unwrap_or_else(|| Value::string("NONE"))
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

/// Rebuild a return-options dict with one key replaced, preserving every
/// unrelated option. A missing key is appended.
pub(crate) fn with_return_option(options: &Value, key: &str, value: Value) -> Value {
    let mut items = Vec::new();
    let mut replaced = false;
    if let Ok(list) = options.as_list() {
        let mut i = 0;
        while i + 1 < list.len() {
            items.push(list[i].clone());
            if list[i].string_bytes().as_ref() == key.as_bytes() {
                items.push(value.clone());
                replaced = true;
            } else {
                items.push(list[i + 1].clone());
            }
            i += 2;
        }
    }
    if !replaced {
        items.push(Value::string(key));
        items.push(value);
    }
    Value::list(items)
}

/// Look up a key in an options-dict value, returning the following element.
pub(crate) fn opt_get(options: &Value, key: &str) -> Option<Value> {
    if let Some(value) = options.with_cached_dictionary_representation(|pairs, _| {
        pairs
            .iter()
            .find(|(name, _)| {
                name.resident_string_bytes()
                    .is_some_and(|bytes| bytes.as_ref() == key.as_bytes())
            })
            .map(|(_, value)| value.clone())
    }) {
        return value;
    }
    let items = options.as_list().ok()?;
    let mut i = 0;
    while i + 1 < items.len() {
        if items[i].string_bytes().as_ref() == key.as_bytes() {
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
pub(crate) fn cmd_return(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    crate::return_options::apply(
        vm,
        args,
        tcl_cmd_core::return_options::ReturnOptionsPurpose::User,
    )
}

/// Apply an internal original-object options dictionary, independently of the
/// exposed return command's argv grammar.
pub(crate) fn apply_return_options(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    crate::return_options::apply(
        vm,
        args,
        tcl_cmd_core::return_options::ReturnOptionsPurpose::InternalDictionary,
    )
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
    let [name, value] = args else {
        return native_wrong_arguments_message(
            vm,
            "wrong # args: should be \"const varName value\"",
        );
    };
    let name = match vm.native_name_operand_bytes(name) {
        Ok(name) => name,
        Err(error) => return vm.refuse_host_command(error.to_string()),
    };
    match vm.declare_constant_bytes(&name, value.clone(), true) {
        Ok(()) => ok(Value::empty()),
        Err(error) => error,
    }
}

/// Queue the same replacement activation used by the native tailcall opcode.
fn cmd_tailcall(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let request = crate::exec::TailcallReq {
        namespace: tcl_cmd_core::namespace::current(vm),
        words: args.to_vec(),
    };
    if let Err(completion) = vm.schedule_tailcall(request) {
        return completion;
    }
    Completion::new(Code::Return, Value::empty(), Value::empty())
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
        return native_wrong_arguments_message(
            vm,
            "wrong # args: should be \"time command ?count?\"",
        );
    }
    let count = if args.len() == 2 {
        match tcl_syntax::value::ValueOps::as_int(vm, &args[1]) {
            Ok(n) => n,
            Err(e) => return completion_from_cmd_error(vm, CmdError::from(e)),
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
            Err(e) => return completion_from_tcl_error(vm, e),
        }
        i -= 1;
    }
    let total = (vm.host_rc().clock().now_micros() - start) as f64;
    let num = if count <= 1 {
        Value::int(if count <= 0 { 0 } else { total as i64 })
    } else {
        Value::native_double(total / count as f64, vm.native_invocation_dialect())
    };
    settle_control_options(
        ok(Value::list(vec![
            num,
            Value::string("microseconds"),
            Value::string("per"),
            Value::string("iteration"),
        ])),
        tcl_runtime_api::completion_options::ControlOptionPolicy::FRESH_SETTLED,
    )
}

/// `encoding subcommand ?arg …?` — matches the tree-walking runtime
/// (`runtime/rust`): the internal string model is UTF-8, so
/// `convertto`/`convertfrom` pass the data through unchanged, `system` reports
/// the host's typed locale fact, `names` lists the supported channel encodings,
/// and `dirs` is accepted and ignored (no encoding-file search). This is a
/// documented simplification — real codepage conversion (cp1252, shiftjis, …)
/// is not implemented on either side.
/// `encoding`'s subcommand set, alphabetical as `TclMakeEnsemble` sorts it.
/// 9.0's table also carries `profiles` and `user`, which need the encoding
/// machinery this engine does not model; like its other ensembles it names
/// only what it dispatches.
const ENCODING_SUBS: &[&str] = &["convertfrom", "convertto", "dirs", "names", "system"];

fn cmd_encoding(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some(sub) = args.first() else {
        return native_wrong_arguments_message(
            vm,
            "wrong # args: should be \"encoding subcommand ?arg ...?\"",
        );
    };
    // `dirs` arrives in 8.5, so the table follows the emulated release.
    let subs = crate::environment::release_subcommands(
        vm.runtime_version().dialect_profile_name(),
        "encoding",
        ENCODING_SUBS,
    );
    let canon =
        match tcl_cmd_core::ensemble::resolve_subcommand(subs, sub.to_str().as_bytes(), true) {
            Some(index) => subs[index],
            None => {
                return err(tcl_cmd_core::ensemble::unknown_subcommand_message(
                    subs,
                    sub.to_str().as_bytes(),
                    true,
                    b"::tcl::encoding",
                ));
            }
        };
    match canon {
        "dirs" => ok(Value::empty()),
        "system" => match args {
            [_] => ok(Value::string(vm.system_encoding().as_str())),
            [_, value] => match tcl_cmd_core::channel::resolve_system_encoding(&value.to_str()) {
                Ok(encoding) => {
                    vm.set_system_encoding(encoding);
                    ok(Value::empty())
                }
                Err(error) => completion_from_cmd_error(vm, error),
            },
            _ => err_wrong_args(vm, "encoding system ?encoding?"),
        },
        "names" => ok(Value::string("utf-8 unicode ascii iso8859-1")),
        // Unreachable: `ENCODING_SUBS` has exactly these five names.
        _ => {
            if args.len() < 2 {
                return native_wrong_arguments_message(
                    vm,
                    "wrong # args: should be \"encoding convertto ?encoding? data\"",
                );
            }
            ok(args.last().expect("len >= 2").clone())
        }
    }
}

/// `error message ?info? ?code?`.
fn cmd_error(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some(protocol) = vm.native_invocation_dialect().error_arguments() else {
        return vm.refuse_host_command("native error argument grammar is unresolved".to_owned());
    };
    if !protocol.accepts_len(args.len()) {
        return native_wrong_args(vm, protocol.synopsis());
    }
    let msg = &args[0];
    if protocol == tcl_registry::invocation_words::NativeErrorArguments::JimStackTrace {
        let trace = args.get(1).cloned().unwrap_or_else(Value::empty);
        if args.len() == 2 {
            vm.jim_errors.stack.adopt_explicit(trace.clone());
        }
        return Completion::new_error_metadata(
            Code::Error,
            msg.clone(),
            options_dict(
                Code::Error,
                0,
                &[("-errorcode", Value::string("NONE")), ("-errorinfo", trace)],
            ),
        );
    }
    if vm
        .native_invocation_dialect()
        .native_error_variable_protocol()
        .is_some()
    {
        let mut options = vec![
            Value::string("-code"),
            Value::string("error"),
            Value::string("-level"),
            Value::string("0"),
        ];
        if let Some(info) = args.get(1) {
            options.extend([Value::string("-errorinfo"), info.clone()]);
        }
        if let Some(code) = args.get(2) {
            options.extend([Value::string("-errorcode"), code.clone()]);
        }
        options.push(msg.clone());
        return crate::return_options::apply(
            vm,
            &options,
            tcl_cmd_core::return_options::ReturnOptionsPurpose::User,
        );
    }
    let ecode = args
        .get(2)
        .cloned()
        .unwrap_or_else(|| Value::string("NONE"));
    // A *non-empty* `info` argument *is* the errorInfo trace: seed it directly and
    // suppress the `error` command's own `while executing` frame (C's
    // `ERR_ALREADY_LOGGED`). An empty (or absent) `info` is treated as absent —
    // the trace seeds from the message and the invoke site logs the frame as for
    // any other command error (error-4.2/4.3).
    let info = args.get(1).map(super::value::Value::string_bytes);
    let info_nonempty = info.as_deref().is_some_and(|s| !s.is_empty());
    if info_nonempty {
        vm.seed_error_info(info.clone().unwrap_or_default());
    }
    let einfo = if info_nonempty {
        info.unwrap_or_default()
    } else {
        msg.string_bytes()
    };
    let options = options_dict(
        Code::Error,
        0,
        &[
            ("-errorcode", ecode),
            ("-errorinfo", Value::from_string_bytes(einfo)),
        ],
    );
    Completion::new_error_metadata(Code::Error, msg.clone(), options)
}

/// `break`.
fn cmd_break(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    if !args.is_empty() {
        return native_wrong_arguments_message(vm, "wrong # args: should be \"break\"");
    }
    Completion::new(Code::Break, Value::empty(), Value::empty())
}

/// `continue`.
fn cmd_continue(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    if !args.is_empty() {
        return native_wrong_arguments_message(vm, "wrong # args: should be \"continue\"");
    }
    Completion::new(Code::Continue, Value::empty(), Value::empty())
}

/// `catch script ?resultVarName? ?optionsVarName?`.
fn cmd_catch(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    if vm.uses_jim_error_stack() {
        let _ = vm.set_var_bytes(b"::errorCode", Value::string("NONE"));
        if let Some(refusal) = vm.refused_completion() {
            return refusal;
        }
    }
    let dialect = vm.native_invocation_dialect();
    let positional = dialect.catch_positional_arity().is_some();
    let unknown_values = vec![tcl_registry::InvocationWord::Dynamic; args.len()];
    let strings: Vec<String> = if positional {
        Vec::new()
    } else {
        args.iter()
            .map(|value| value.to_str().to_string())
            .collect()
    };
    let words: Vec<&str> = strings.iter().map(String::as_str).collect();
    let selected = match tcl_registry::catch_invocation::select_catch_invocation(
        if positional {
            tcl_registry::InvocationArguments::structured(&unknown_values)
        } else {
            tcl_registry::InvocationArguments::literals(&words)
        },
        dialect,
    ) {
        tcl_registry::catch_invocation::CatchInvocationSelection::Valid(selected) => selected,
        _ if dialect.completion_options_policy()
            == Some(tcl_registry::CompletionOptionsPolicy::Legacy) =>
        {
            return native_wrong_arguments_message(
                vm,
                "wrong # args: should be \"catch command ?varName?\"",
            );
        }
        _ => {
            return native_wrong_arguments_message(
                vm,
                "wrong # args: should be \"catch script ?resultVarName? ?optionVarName?\"",
            );
        }
    };
    let script = &args[selected.script_at];
    let resvar = selected.result_var_at.map(|index| &args[index]);
    let optvar = selected.options_var_at.map(|index| &args[index]);
    // Defer the body to the *explicit* stack so a `yield` inside it stays
    // yieldable: compile it and hand it to the trampoline via
    // the pending catch request. A catch frame runs the body and its completion — of any
    // code — is absorbed by `finish_catch` (which binds the result/options vars
    // and yields the status code). Mirrors `cmd_eval`'s pending request, but
    // catch's completion is caught rather than propagated.
    match vm.prepare_script_commands_value(script) {
        Ok(prepared) if prepared.prefix.is_some() => {
            vm.pending.catch = Some(crate::exec::CatchReq {
                script: prepared
                    .prefix
                    .expect("checked above")
                    .with_source_location(script.source_location()),
                resvar: resvar.map(|v| (*v).clone()),
                optvar: optvar.map(|v| (*v).clone()),
                fatal_tail: prepared.fatal_tail,
                ignored_codes: selected.ignored_codes,
            });
            ok(Value::empty())
        }
        Ok(prepared) => {
            let comp = prepared.fatal_tail.map_or_else(
                || ok(Value::empty()),
                |tail| vm.raise_script_parse_failure(tail, script.source_location()),
            );
            vm.finish_catch(comp, resvar, optvar, selected.ignored_codes)
        }
        // A body that fails to *parse* is itself a catchable error: run the
        // epilogue directly with the parse-error completion (no body to execute).
        Err(e) => {
            let comp = completion_from_tcl_error(vm, e);
            vm.finish_catch(comp, resvar, optvar, selected.ignored_codes)
        }
    }
}

impl Vm {
    /// Snapshot a completion through the shared standard-options planner.
    /// `catch`, compiled catch ranges, and `try` all use this adapter, so live
    /// error metadata and carried return options cannot drift between them.
    pub(crate) fn completion_options_snapshot(&self, comp: &Completion<Value>) -> Value {
        let merged = match comp.option_origin {
            tcl_core_types::CompletionOptionOrigin::MergedReturnOptions { code, level } => {
                Some((&comp.options, code, level))
            }
            _ => self.native_merged_return_options().map(|original| {
                let (code, level) = self.native_return_controls(comp.code);
                (original, code, level)
            }),
        };
        if let Some((options, code, level)) = merged {
            return self.merged_completion_options_snapshot(comp, options, code, level);
        }
        if self.uses_jim_error_stack() {
            return self.jim_return_options(comp.code);
        }
        let mut carried = comp.options.as_list().map_or_else(
            |_| Vec::new(),
            |items| {
                items
                    .as_slice()
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|pair| (pair[0].string_bytes().to_vec(), pair[1].clone()))
                    .collect()
            },
        );
        let jim_quote = self.native_invocation_dialect().expression_quote_control()
            == Some(tcl_registry::invocation_words::ExpressionQuoteControl::Jim084);
        if jim_quote && comp.code == Code::Ok {
            carried.retain(|(key, _)| {
                !matches!(
                    key.as_slice(),
                    b"-errorcode" | b"-errorinfo" | b"-errorline" | b"-errorstack"
                )
            });
        }
        let (code, level) = if comp.code == Code::Return {
            let code = opt_get(&comp.options, "-code")
                .and_then(|value| value.as_int().ok())
                .and_then(|value| i32::try_from(value).ok())
                .map_or(Code::Ok, Code::from_int);
            let level = opt_get(&comp.options, "-level")
                .and_then(|value| value.as_int().ok())
                .unwrap_or(1);
            (code, level)
        } else {
            let level = if jim_quote && comp.code == Code::Ok {
                opt_get(&comp.options, "-level")
                    .and_then(|value| value.as_int().ok())
                    .unwrap_or(0)
            } else {
                0
            };
            (comp.code, level)
        };
        let active_error = code == Code::Error && comp.code == Code::Error && level == 0;
        let error = (code == Code::Error).then(|| ErrorOptions {
            error_code: Some(resolved_error_code(comp)),
            error_info: active_error.then(|| {
                if self.uses_jim_error_stack() {
                    return self.jim_stacktrace();
                }
                self.error_info_value().map_or_else(
                    || opt_get(&comp.options, "-errorinfo").unwrap_or_else(|| comp.result.clone()),
                    Value::from_string_bytes,
                )
            }),
            error_stack: (active_error && self.supports_error_stack())
                .then(|| self.error_stack_for_completion(opt_get(&comp.options, "-errorstack"))),
            error_line: (active_error && !self.uses_jim_error_stack())
                .then(|| i64::from(self.error_line())),
            during: None,
        });
        let rows = shared_options::plan_with_origin(
            self.runtime_version(),
            code,
            level,
            comp.option_origin,
            &carried,
            error.as_ref(),
        );
        Value::list(
            rows.into_iter()
                .flat_map(|(key, value)| {
                    let value = match value {
                        OptionValue::Integer(value) => Value::int(value),
                        OptionValue::Value(value) => value,
                    };
                    [Value::from_string_bytes(key), value]
                })
                .collect(),
        )
    }

    fn merged_completion_options_snapshot(
        &self,
        comp: &Completion<Value>,
        options: &Value,
        code: i32,
        level: i64,
    ) -> Value {
        let strings = self
            .native_invocation_dialect()
            .native_string_protocol()
            .expect("merged C return origin");
        // Tcl_GetReturnOptions always duplicates the retained private header.
        let original = options.duplicate_native_object_in(strings);
        let mut copied = original
            .prepare_native_dictionary(strings)
            .expect("original merged Dictionary");
        drop(original);
        let active_error = comp.code == Code::Error && level == 0;
        let error = (code == 1).then(|| ErrorOptions {
            error_code: Some(
                self.native_return_error_code()
                    .cloned()
                    .unwrap_or_else(|| resolved_error_code(comp)),
            ),
            error_stack: (active_error && self.supports_error_stack())
                .then(|| self.error_stack_for_completion(opt_get(&comp.options, "-errorstack"))),
            error_info: active_error.then(|| {
                self.native_return_error_info().cloned().unwrap_or_else(|| {
                    self.error_info_value()
                        .map_or_else(|| comp.result.clone(), Value::new_native_string_bytes)
                })
            }),
            error_line: active_error.then(|| i64::from(self.error_line())),
            during: None,
        });
        // Apply the shared native overlay to the duplicate. Existing keys
        // keep their positions; new errorStack precedes errorCode/info/line.
        let overlay = shared_options::plan_with_origin(
            self.runtime_version(),
            Code::from_int(code),
            level,
            tcl_core_types::CompletionOptionOrigin::ErrorMetadata,
            &[],
            error.as_ref(),
        );
        for (key, value) in overlay {
            let value = match value {
                OptionValue::Integer(value) => Value::int(value),
                OptionValue::Value(value) => value,
            };
            copied
                .set_member(Value::new_native_string_bytes(key), value)
                .expect("native return-options overlay");
        }
        copied.into_value()
    }

    /// Restore a frozen error completion after a successful `finally` body so
    /// subsequent procedure unwinding extends the original stack.
    pub(crate) fn restore_completion_error_state(&mut self, comp: &Completion<Value>) {
        if comp.code != Code::Error {
            return;
        }
        if let Some(original) = opt_get(&comp.options, "-errorcode") {
            // This is an explicit frozen-completion restoration, not a fresh
            // primitive conversion or an inferred code from diagnostic bytes.
            self.restore_guest_error_code(original);
        }
        let original = opt_get(&comp.options, "-errorinfo").unwrap_or_else(|| comp.result.clone());
        let info = original.string_bytes();
        self.seed_error_info_original(&original, &info);
        if let Some(stack) = opt_get(&comp.options, "-errorstack") {
            self.seed_error_stack(&stack);
        }
        if let Some(line) = opt_get(&comp.options, "-errorline")
            .and_then(|value| value.as_int().ok())
            .and_then(|value| u32::try_from(value).ok())
        {
            self.set_error_line(line);
        }
    }

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
        ignored_codes: u64,
    ) -> Completion<Value> {
        if let Some(refused) = self.refused_completion() {
            return refused;
        }
        let code = u32::try_from(comp.code.as_int()).ok();
        if code.is_some_and(|code| code < 64 && ignored_codes & (1_u64 << code) != 0) {
            return comp;
        }
        if self.exit_pending() {
            if self.native_invocation_dialect().family() == Some(tcl_dialect::model::Family::Jim)
                && comp.code == Code::from_int(6)
            {
                let _ = self.take_exit();
            } else {
                return comp;
            }
        }
        let opts = self.completion_options_snapshot(&comp);
        let error_meta = (comp.code == Code::Error).then(|| {
            let einfo = opt_get(&opts, "-errorinfo")
                .map_or_else(|| comp.result.string_bytes(), |value| value.string_bytes());
            let ecode = opt_get(&opts, "-errorcode").unwrap_or_else(|| resolved_error_code(&comp));
            (einfo, ecode)
        });
        let _ = self.take_error_info();
        if let Some(r) = resvar
            && let Err(e) = self.set_var(&r.to_str(), comp.result.clone())
        {
            return e;
        }
        if let Some(o) = optvar
            && let Err(e) = self.set_var(&o.to_str(), opts)
        {
            return e;
        }
        if let Some((einfo, ecode)) = &error_meta {
            self.publish_error(einfo, ecode);
        }
        // The catch epilogue replaces the interpreter's body result with its
        // status integer (JimCatchTryHelper's Jim_SetResultInt). A deferred
        // Script invocation reads that actual result owner after this returns;
        // the completion carries only a lifetime view of the same object.
        match self.publish_native_interp_completion(ok(Value::int(comp.code.as_int()))) {
            Ok(completion) => completion,
            Err(error) => completion_from_tcl_error(self, error.into()),
        }
    }

    /// The options dict a *compiled* catch range records when it absorbs
    /// `comp` — the same digestion [`Vm::finish_catch`] performs (consume the
    /// accumulated `errorInfo` trace, resolve the error code and line, publish
    /// the `$errorInfo`/`$errorCode` globals) minus the variable binding,
    /// which the bytecode epilogue does itself from
    /// `PUSH_RESULT`/`PUSH_RETURN_CODE`/`PUSH_RETURN_OPTS`.
    pub(crate) fn digest_catch_options(&mut self, comp: &Completion<Value>) -> Value {
        if comp.code == Code::Error {
            let opts = self.completion_options_snapshot(comp);
            let einfo = opt_get(&opts, "-errorinfo")
                .map_or_else(|| comp.result.string_bytes(), |value| value.string_bytes());
            let ecode = opt_get(&opts, "-errorcode").unwrap_or_else(|| resolved_error_code(comp));
            let _ = self.take_error_info();
            self.publish_error(&einfo, &ecode);
            opts
        } else {
            let _ = self.take_error_info();
            completion_options(comp)
        }
    }
}

/// `unset ?-nocomplain? ?--? name ...` — remove variables / array elements.
fn cmd_unset(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some(protocol) = vm.native_invocation_dialect().unset_option_protocol() else {
        return vm.refuse_host_command("unset option grammar is unavailable".into());
    };
    let options = match protocol.parse(args.len(), |index| {
        vm.native_name_operand_bytes(&args[index])
            .map_err(|error| vm.refuse_host_command(error.to_string()))
    }) {
        Ok(options) => options,
        Err(error) => return error,
    };
    let rest = &args[options.names_from..];
    for n in rest {
        if let Err(c) = vm.unset_original_named_variable(n, options.complain) {
            return c;
        }
    }
    ok(Value::empty())
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
/// `global a(b)` are hard errors rather than silent mislinks.
fn bad_link_name(name: &str) -> Completion<Value> {
    err_with_code(
        format!(
            "bad variable name \"{name}\": can't create a scalar variable that looks like an array element"
        ),
        "TCL UPVAR LOCAL_ELEMENT",
    )
}

/// Render the typed variable-resolver failure from [`Vm::link_upvar_bytes`].
pub(crate) fn upvar_link_error(
    error: crate::interp::UpvarLinkError,
    other: &str,
    local: &str,
) -> Completion<Value> {
    match error {
        crate::interp::UpvarLinkError::TargetNamespace => err_with_code(
            format!("can't access \"{other}\": parent namespace doesn't exist"),
            lookup_var_error_code(other),
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
            lookup_var_error_code(local),
        ),
        crate::interp::UpvarLinkError::Exists => err_with_code(
            format!("variable \"{local}\" already exists"),
            "TCL UPVAR EXISTS",
        ),
        crate::interp::UpvarLinkError::Traced => err_with_code(
            format!("variable \"{local}\" has traces: can't use for upvar"),
            "TCL UPVAR TRACED",
        ),
        crate::interp::UpvarLinkError::SelfLink => {
            err_with_code("can't upvar from variable to itself", "TCL UPVAR SELF")
        }
    }
}

/// Render a native link failure using exact original operand bytes.
pub(crate) fn upvar_link_error_bytes(
    error: crate::interp::UpvarLinkError,
    other: &[u8],
    local: &[u8],
) -> Completion<Value> {
    use crate::interp::UpvarLinkError;
    let (prefix, name, suffix, code): (&[u8], &[u8], &[u8], &[u8]) = match error {
        UpvarLinkError::TargetNamespace => (
            b"can't access \"",
            other,
            b"\": parent namespace doesn't exist",
            b"TCL LOOKUP VARNAME",
        ),
        UpvarLinkError::LocalNamespace => (
            b"can't create \"",
            local,
            b"\": parent namespace doesn't exist",
            b"TCL LOOKUP VARNAME",
        ),
        UpvarLinkError::Inverted => (
            b"bad variable name \"",
            local,
            b"\": can't create namespace variable that refers to procedure variable",
            b"TCL UPVAR INVERTED",
        ),
        UpvarLinkError::LocalElement => (
            b"bad variable name \"",
            local,
            b"\": can't create a scalar variable that looks like an array element",
            b"TCL UPVAR LOCAL_ELEMENT",
        ),
        UpvarLinkError::Exists => (
            b"variable \"",
            local,
            b"\" already exists",
            b"TCL UPVAR EXISTS",
        ),
        UpvarLinkError::Traced => (
            b"variable \"",
            local,
            b"\" has traces: can't use for upvar",
            b"TCL UPVAR TRACED",
        ),
        UpvarLinkError::SelfLink => {
            return err_with_code("can't upvar from variable to itself", "TCL UPVAR SELF");
        }
    };
    let mut message = prefix.to_vec();
    message.extend_from_slice(name);
    message.extend_from_slice(suffix);
    let mut error_code = code.to_vec();
    if matches!(
        error,
        UpvarLinkError::TargetNamespace | UpvarLinkError::LocalNamespace
    ) {
        let base = tcl_syntax::naming::split_element_ref_bytes(name).map_or(name, |(base, _)| base);
        error_code.push(b' ');
        tcl_syntax::list::append_list_element(&mut error_code, base, false);
    }
    err_with_code(message, error_code)
}

/// `global name ?name ...?` — link names to the global frame.
///
/// Outside a procedure `global` is a documented no-op: `Tcl_GlobalObjCmd`
/// returns `TCL_OK` before touching its arguments when the current frame *is*
/// the global frame. So the element-name guard below is reached only from a
/// proc — at top level and inside `namespace eval`, even `global (x)` and
/// `global a(b)` are accepted. Verified on 8.6.16 and 9.0.4, which agree.
fn cmd_global(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    if tcl_registry::VariableAliasDestination::ProcedureLocal
        .is_active_in_frame(vm.variable_alias_frame(), vm.native_invocation_dialect())
        != Some(true)
    {
        return ok(Value::empty());
    }
    let Some(policy) = vm.name_policy_protocol() else {
        return vm.refuse_host_command("global alias policy is unavailable".into());
    };
    for value in args {
        let name = match vm.native_name_operand_bytes(value) {
            Ok(name) => name,
            Err(error) => {
                return vm.refuse_host_command(format!("global name is unavailable: {error}"));
            }
        };
        let Some(local) = tcl_syntax::naming::global_local_name_bytes(policy.recipe(), &name)
        else {
            continue;
        };
        if vm.native_c_variable_name_protocol().is_some() {
            let local = if policy
                .recipe()
                .variable_alias_local_input(&name)
                .qualification()
                == tcl_syntax::naming::NativeNameQualification::Unqualified
            {
                value.native_lifetime_lease().into_value()
            } else {
                Value::new_native_string_bytes(local)
            };
            if let Err(error) = vm.link_original_c_variable_objects(value, 0, None, &local) {
                return error;
            }
            continue;
        }
        if let Err(error) = vm.add_link_original(&local, 0, value) {
            return error;
        }
    }
    ok(Value::empty())
}

/// `upvar ?level? otherVar localVar ?otherVar localVar ...?`.
pub(crate) fn cmd_upvar(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let usage = vm.actual_native_invocation_dialect().native_namespace_upvar_protocol()
        .and_then(tcl_registry::native_namespace_upvar::NativeNamespaceUpvarProtocol::forwarded_wrong_arguments_usage)
        .unwrap_or(b"upvar ?level? otherVar localVar ?otherVar localVar ...?");
    if args.len() < 2 {
        return native_wrong_args_bytes(vm, usage);
    }
    let (width, target) =
        match runtime_frame_selection(vm, tcl_registry::FrameEffectSpec::UPVAR, args) {
            Ok(selected) => selected,
            Err(error) => return error,
        };
    let rest = &args[width..];
    if rest.is_empty() || !rest.len().is_multiple_of(2) {
        return native_wrong_args_bytes(vm, usage);
    }
    let mut i = 0;
    while i + 1 < rest.len() {
        if vm.native_c_variable_name_protocol().is_some() {
            if let Err(error) =
                vm.link_original_c_variable_objects(&rest[i], target, None, &rest[i + 1])
            {
                return error;
            }
            i += 2;
            continue;
        }
        if let Err(error) = vm.link_upvar_original(target, &rest[i], &rest[i + 1]) {
            return error;
        }
        i += 2;
    }
    ok(Value::empty())
}

struct OriginalFrameLevel<'a> {
    value: &'a Value,
    dialect: tcl_registry::InvocationDialect,
}

impl<'a> OriginalFrameLevel<'a> {
    fn selected(vm: &Vm, value: &'a Value) -> Result<Self, tcl_syntax::value::ValueError> {
        let dialect = vm.native_scalar_carrier_dialect();
        if dialect.native_string_protocol()
            == Some(tcl_syntax::native_string::NativeStringProtocol::Jim084)
        {
            value.bind_native_jim_context(
                &crate::interp::InterpState::native_jim_object_context(vm)?,
            )?;
        }
        Ok(Self { value, dialect })
    }
}

impl tcl_registry::frame_effect::NativeFrameLevelObject for OriginalFrameLevel<'_> {
    type Error = tcl_syntax::value::ValueError;

    fn native_frame_string(&mut self) -> Result<Vec<u8>, Self::Error> {
        let protocol = self.dialect.native_string_protocol().ok_or(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable("native frame string"),
        )?;
        self.value
            .native_string_bytes(protocol)
            .map(|bytes| bytes.to_vec())
            .map_err(|_| {
                tcl_syntax::value::ValueError::CommandProtocolUnavailable("native frame string")
            })
    }

    fn native_frame_probe(
        &mut self,
        kind: tcl_syntax::scalar_getter::NativeScalarGetterKind,
    ) -> Result<Result<i64, tcl_syntax::scalar_getter::NativeScalarGetterFailure>, Self::Error>
    {
        use tcl_syntax::scalar_getter::NativeScalarGetterValue;
        self.value
            .native_scalar_probe(self.dialect, kind)
            .map(|outcome| {
                outcome.map(|value| match value {
                    NativeScalarGetterValue::Wide(integer) => integer,
                    _ => unreachable!("integer frame getter"),
                })
            })
    }

    fn native_frame_is_integer(&self) -> bool {
        matches!(
            self.value.native_scalar_cache(),
            Some(tcl_syntax::scalar_getter::NativeScalarCache::Number(
                tcl_syntax::number::Number::Int(_) | tcl_syntax::number::Number::Big { .. }
            ))
        )
    }

    fn native_frame_is_machine_integer(&self) -> bool {
        matches!(
            self.value.native_scalar_cache(),
            Some(tcl_syntax::scalar_getter::NativeScalarCache::Number(
                tcl_syntax::number::Number::Int(_)
            ))
        )
    }

    fn native_frame_cache(
        &self,
    ) -> Result<Option<tcl_registry::NativeFrameLevelCache>, Self::Error> {
        self.value.native_frame_level_cache_in(self.dialect)
    }

    fn native_frame_set_cache(
        &mut self,
        cache: tcl_registry::NativeFrameLevelCache,
    ) -> Result<(), Self::Error> {
        self.value
            .install_native_frame_level_cache(cache, self.dialect)
    }
}

fn native_frame_failure(
    vm: &mut Vm,
    failure: tcl_registry::frame_effect::NativeFrameLevelFailure,
) -> Completion<Value> {
    use tcl_registry::frame_effect::NativeFrameLevelFailure;
    match failure {
        NativeFrameLevelFailure::Primitive(record) => completion_from_cmd_error(
            vm,
            tcl_syntax::value::ValueError::NativeScalarGetter(record).into(),
        ),
        NativeFrameLevelFailure::BadLevel { name, lookup_code } => {
            let mut message = b"bad level \"".to_vec();
            message.extend_from_slice(&name);
            message.push(b'"');
            if lookup_code {
                let mut code = b"TCL LOOKUP LEVEL".to_vec();
                tcl_syntax::list::append_list_element(&mut code, &name, false);
                err_with_code(message, code)
            } else {
                err(message)
            }
        }
    }
}

pub(crate) fn runtime_frame_selection(
    vm: &mut Vm,
    effect: tcl_registry::FrameEffectSpec,
    args: &[Value],
) -> Result<(usize, usize), Completion<Value>> {
    let dialect = vm.native_scalar_carrier_dialect();
    let Some(protocol) = dialect.native_frame_level_protocol() else {
        return Err(
            vm.refuse_host_command("native original-object frame protocol is unavailable".into())
        );
    };
    let current = vm.current_level();
    let count_width = effect.level_word_len_for_native_bytes(args.len(), None, dialect);
    let default_target = |vm: &mut Vm| {
        current
            .checked_sub(1)
            .map(|target| (0, target))
            .ok_or_else(|| {
                native_frame_failure(
                    vm,
                    tcl_registry::frame_effect::NativeFrameLevelFailure::BadLevel {
                        name: b"1".to_vec(),
                        lookup_code: protocol
                            .tcl_version()
                            .is_some_and(|version| version >= tcl_dialect::TclVersion::V8_6),
                    },
                )
            })
    };
    if count_width == Some(0) {
        return default_target(vm);
    }
    let original = args.first().expect("frame command arity checked");
    if count_width == Some(1) {
        return runtime_explicit_frame_selection(vm, original).map(|target| (1, target));
    }
    if effect == tcl_registry::FrameEffectSpec::UPLEVEL
        && args.len() == 1
        && protocol.probes_single_script_list_first()
        && original.resident_string_bytes().is_none()
    {
        match tcl_syntax::value::ValueOps::list_len(vm, original) {
            Ok(length) if length > 1 => return default_target(vm),
            Err(error) if error.native_access_refusal().is_some() => {
                return Err(completion_from_cmd_error(vm, error.into()));
            }
            _ => {}
        }
    }
    let mut operand = OriginalFrameLevel::selected(vm, original)
        .map_err(|error| completion_from_cmd_error(vm, error.into()))?;
    let resolution = protocol.resolve_leading_object(current, &mut operand);
    match resolution {
        Ok(Ok(result)) => Ok((
            count_width.unwrap_or(usize::from(result.explicit)),
            result.target,
        )),
        Ok(Err(failure)) => Err(native_frame_failure(vm, failure)),
        Err(error) => Err(completion_from_cmd_error(vm, error.into())),
    }
}

/// Resolve an already-explicit original level through the selected native getter.
/// This does not decide whether the operand belongs to the script word vector.
pub(crate) fn runtime_explicit_frame_selection(
    vm: &mut Vm,
    original: &Value,
) -> Result<usize, Completion<Value>> {
    let dialect = vm.native_scalar_carrier_dialect();
    let protocol = dialect.native_frame_level_protocol().ok_or_else(|| {
        vm.refuse_host_command("native original-object frame protocol is unavailable".into())
    })?;
    let mut operand = OriginalFrameLevel::selected(vm, original)
        .map_err(|error| completion_from_cmd_error(vm, error.into()))?;
    match protocol.resolve_object(vm.current_level(), &mut operand) {
        Ok(Ok(selected)) => Ok(selected.target),
        Ok(Err(failure)) => Err(native_frame_failure(vm, failure)),
        Err(error) => Err(completion_from_cmd_error(vm, error.into())),
    }
}

/// The instruction already consumed an original level and one original script.
/// Preserve that operand boundary even when the level getter chooses its default.
pub(crate) fn compiled_uplevel(vm: &mut Vm, level: &Value, script: Value) -> Completion<Value> {
    let target = match runtime_explicit_frame_selection(vm, level) {
        Ok(target) => target,
        Err(error) => return error,
    };
    let location = script.source_location();
    eval_original_uplevel(vm, target, script, location)
}

/// `uplevel ?level? arg ?arg ...?` — evaluate the concatenated args as a script
/// in the call frame `level` up (default 1, the caller). `#N` selects an
/// absolute level.
pub(crate) fn cmd_uplevel(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    if args.is_empty() {
        return native_wrong_args(vm, "uplevel ?level? command ?arg ...?");
    }
    let (width, target) =
        match runtime_frame_selection(vm, tcl_registry::FrameEffectSpec::UPLEVEL, args) {
            Ok(selected) => selected,
            Err(error) => return error,
        };
    let rest = &args[width..];
    if rest.is_empty() {
        return native_wrong_args(vm, "uplevel ?level? command ?arg ...?");
    }
    let script = if rest.len() == 1 {
        rest[0].clone()
    } else {
        match tcl_cmd_core::list::concat_selected(vm, rest) {
            Ok(value) => value,
            Err(error) => return crate::command::completion_from_cmd_error(vm, error),
        }
    };
    let location = rest
        .first()
        .filter(|_| rest.len() == 1)
        .and_then(Value::source_location);
    eval_original_uplevel(vm, target, script, location)
}

/// Enter the selected variable frame before inspecting or preparing the source.
/// The actual evaluation activation owns restoration across suspension/unwind.
fn eval_original_uplevel(
    vm: &mut Vm,
    target: usize,
    script: Value,
    location: Option<tcl_runtime_api::script_source_location::ScriptSourceLocation>,
) -> Completion<Value> {
    let selected = match vm.select_execution_frame(target) {
        Ok(selected) => selected,
        Err(message) => return err(message),
    };
    let direct = match original_script_list(
        vm,
        &script,
        tcl_registry::native_eval_object::EvalObjectPurpose::UpLevel,
    ) {
        Ok(direct) => direct,
        Err(error) => {
            vm.restore_execution_frame(selected);
            return error;
        }
    };
    if let Some(words) = direct {
        vm.pending.control = Some(crate::cmd_control::ControlState::object_eval(
            script, words, "uplevel", selected,
        ));
        return ok(Value::empty());
    }
    // Compile against the selected namespace and physical frame. The deferred
    // activation retains the hidden caller owners until it completes, so every
    // level uses the same yieldable execution protocol.
    match vm.prepare_script_commands_value_for(
        &script,
        tcl_registry::native_eval_object::EvalObjectPurpose::UpLevel,
    ) {
        Ok(prepared) => {
            if let Some(script) = prepared.prefix {
                vm.pending.eval = Some(crate::exec::EvalReq {
                    script: script.with_source_location(location.clone()),
                    label: Some("uplevel"),
                    cleanup_proc: None,
                    fatal_tail: prepared.fatal_tail,
                    selected_frame_restore: selected,
                });
                ok(Value::empty())
            } else {
                let completion = prepared.fatal_tail.map_or_else(
                    || ok(Value::empty()),
                    |tail| vm.raise_script_parse_failure(tail, location),
                );
                vm.restore_execution_frame(selected);
                completion
            }
        }
        Err(error) => {
            vm.restore_execution_frame(selected);
            completion_from_tcl_error(vm, error)
        }
    }
}

/// `variable ?name value ...? name ?value?` — namespace variables (currently global).
pub(crate) fn cmd_variable(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    if args.is_empty() {
        return native_wrong_args(vm, "variable ?name value ...? name ?value?");
    }
    if vm.native_c_variable_name_protocol().is_some() {
        for pair in args.chunks(2) {
            if let Err(error) = vm.define_original_c_namespace_variable(&pair[0], pair.get(1)) {
                return error;
            }
        }
        return ok(Value::empty());
    }
    let Some(policy) = vm.name_policy_protocol() else {
        return vm.refuse_host_command("namespace variable policy is unavailable".into());
    };
    let mut i = 0;
    while i < args.len() {
        let name = match vm.native_name_operand_bytes(&args[i]) {
            Ok(name) => name,
            Err(error) => {
                return vm.refuse_host_command(format!("variable name is unavailable: {error}"));
            }
        };
        let local = tcl_syntax::naming::variable_local_name_bytes(policy.recipe(), &name);
        if !vm.var_parent_exists_bytes(&name) {
            return upvar_link_error_bytes(
                crate::interp::UpvarLinkError::TargetNamespace,
                &name,
                &local,
            );
        }
        if !policy.recipe().is_jim084()
            && policy
                .recipe()
                .combined_variable_input(&name)
                .element()
                .is_some()
        {
            let mut message = b"can't define \"".to_vec();
            message.extend_from_slice(&name);
            message.extend_from_slice(b"\": name refers to an element in an array");
            return err_with_code(message, "TCL UPVAR LOCAL_ELEMENT");
        }
        if let Err(error) = vm.link_namespace_variable_bytes(&local, &name) {
            return upvar_link_error_bytes(error, &name, &local);
        }
        if i + 1 < args.len() {
            if let Err(error) = vm.set_var_bytes(&local, args[i + 1].clone()) {
                return error;
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
    use tcl_syntax::value::ValueOps;

    #[test]
    fn physical_procedure_capture_and_header_remain_separate_from_authored_names() {
        use super::{Value, Vm};
        use std::rc::Rc;
        use tcl_dialect::NativeProcedureHeaderCompilation as Header;
        let host = tcl_registry::model::ingress::resolve_environment("tcl9.0").unit_profile();
        let authored = tcl_dialect::DialectProfile::irules();
        let mut vm = Vm::with_native_core(
            Box::new(Vec::<u8>::new()),
            Rc::new(crate::host_native::NativeHost::new()),
            host,
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        vm.set_dialect_profile(authored);
        assert!(vm.set_command_surface_profile(host));
        assert!(vm.set_logical_name_provider(
            tcl_syntax::naming::NamePolicyProtocol::authored_tcl(tcl_dialect::TclVersion::V8_4,)
        ));
        let parameters = Value::new_native_string_bytes(b"args".as_slice());
        let original = Value::new_native_string_bytes(b"\0".as_slice());
        let alias = original.clone();
        let captured = vm
            .capture_procedure_declaration_body(&parameters, &original)
            .unwrap();
        assert_ne!(
            captured.native_object_identity(),
            original.native_object_identity()
        );
        assert_eq!(captured.string_bytes().as_ref(), b"\0");
        assert_eq!(
            alias.native_object_identity(),
            original.native_object_identity()
        );
        // These exact original headers are independently recorded in the
        // native_procedure_headers corpus (case 7 of each selected release).
        let native = include_str!("../../tcl-syntax/tests/data/native_procedure_headers/9.0.4.txt");
        assert!(native.lines().nth(7).unwrap().ends_with("header=0"));
        assert_eq!(
            super::procedure_header_compilation(&mut vm, &parameters, &captured).unwrap(),
            Header::Absent
        );
        assert_eq!(
            tcl_registry::native_procedure::procedure_header_compilation_bytes(
                tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_4),
                Some(b"args"),
                Some(b"\0"),
                Some(false),
            ),
            Header::NoOp,
        );
        let mut missing = Vm::new();
        missing.set_dialect_profile(authored);
        assert!(!missing.set_native_engine_profile(authored));
        assert!(
            missing
                .capture_procedure_declaration_body(&parameters, &original)
                .is_err()
        );
        assert!(super::procedure_header_compilation(&mut missing, &parameters, &original).is_err());
    }

    #[test]
    fn c84_procedure_parse_failures_match_7_native_original_object_completions() {
        use super::{Value, Vm};
        let decode = |text: &str| {
            assert_eq!(text.len() % 2, 0);
            text.as_bytes()
                .as_chunks::<2>()
                .0
                .iter()
                .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
                .collect::<Vec<_>>()
        };
        let profile = tcl_registry::model::ingress::resolve_environment("tcl8.4").unit_profile();
        let mut compared = 0;
        for row in include_str!("../tests/data/native_procedure_parse_context/8.4.20.tsv").lines() {
            let fields = row.split('\t').collect::<Vec<_>>();
            assert_eq!(fields.len(), 8);
            let mut vm = Vm::new();
            vm.set_dialect_profile(profile);
            vm.set_compiler(Box::new(
                tcl_compiler::compile_service::BytecodeCompileService::for_profile(profile),
            ));
            let name = Value::new_native_string_bytes(decode(fields[3]));
            let definition = super::cmd_proc(
                &mut vm,
                &[
                    name.clone(),
                    Value::new_native_string_bytes(&b"x"[..]),
                    Value::new_native_string_bytes(decode(fields[4])),
                ],
            );
            assert_eq!(definition.code.as_int().to_string(), fields[1], "{row}");
            let arguments = if fields[0] == "5" {
                vec![Value::new_native_string_bytes(&b"OK"[..])]
            } else {
                Vec::new()
            };
            let completion = vm.invoke_host_original_object_vector(&name, &arguments);
            assert_eq!(completion.code.as_int().to_string(), fields[2], "{row}");
            assert_eq!(
                completion.result.string_bytes().as_ref(),
                decode(fields[5]),
                "{row}"
            );
            assert_eq!(vm.error_info_value().unwrap(), decode(fields[6]), "{row}");
            assert_eq!(
                super::opt_get(&completion.options, "-errorcode")
                    .unwrap()
                    .string_bytes()
                    .as_ref(),
                decode(fields[7]),
                "{row}"
            );
            assert!(
                vm.get_var_bytes(b"side").is_none(),
                "C84 must reject before the valid prefix: {row}"
            );
            compared += 1;
        }
        assert_eq!(compared, 7);
    }

    fn procedure_definition_object_state(value: &super::Value) -> (&'static str, usize) {
        use tcl_syntax::native_object::NativeObjectCacheSnapshot;
        let snapshot = value.native_object_snapshot();
        let kind = match snapshot.cache {
            NativeObjectCacheSnapshot::None => "none",
            NativeObjectCacheSnapshot::List { .. } => "list",
            NativeObjectCacheSnapshot::ByteArray { .. } => "bytearray",
            other => panic!("unexpected definition cache {other:?}"),
        };
        (kind, usize::from(snapshot.resident.is_some()))
    }

    #[test]
    fn procedure_body_capture_matches_58_native_definition_windows() {
        use super::{Command, Value, Vm};
        let engines = [
            (
                "tcl8.4",
                include_str!("../tests/data/native_procedure_body_capture/8.4.20.tsv"),
            ),
            (
                "tcl8.5",
                include_str!("../tests/data/native_procedure_body_capture/8.5.19.tsv"),
            ),
            (
                "tcl8.6",
                include_str!("../tests/data/native_procedure_body_capture/8.6.18.tsv"),
            ),
            (
                "tcl9.0",
                include_str!("../tests/data/native_procedure_body_capture/9.0.4.tsv"),
            ),
            (
                "tcl9.1",
                include_str!("../tests/data/native_procedure_body_capture/9.1.0.tsv"),
            ),
            (
                "jim",
                include_str!("../tests/data/native_procedure_body_capture/Jim.tsv"),
            ),
        ];
        let mut compared = 0;
        for (engine, fixture) in engines {
            for row in fixture.lines() {
                let expected = row.split('\t').map(str::to_owned).collect::<Vec<_>>();
                assert_eq!(expected.len(), 12);
                let kind = expected[0].parse::<usize>().unwrap();
                let refs = expected[1].parse::<usize>().unwrap();
                let profile =
                    tcl_registry::model::ingress::resolve_environment(engine).unit_profile();
                let mut vm = Vm::new();
                vm.set_dialect_profile(profile);
                let dialect = vm.native_invocation_dialect();
                let protocol = dialect.native_string_protocol().unwrap();
                let body = match kind {
                    0 => Value::new_native_string_bytes(&b"return OK"[..]),
                    1 => Value::native_list_constructor(
                        vec![
                            Value::new_native_string_bytes(&b"return"[..]),
                            Value::new_native_string_bytes(&b"OK"[..]),
                        ],
                        protocol,
                    ),
                    2 => {
                        Value::from_native_byte_array(std::rc::Rc::from(&b"return OK"[..]), dialect)
                            .unwrap()
                    }
                    3 => Value::new_native_string_bytes(&b"set value \""[..]),
                    4 => Value::new_native_string_bytes(&b"return OK\0error BAD"[..18]),
                    _ => panic!("unknown fixture kind"),
                };
                let argv = [
                    Value::new_native_string_bytes(&b"p"[..]),
                    Value::new_native_string_bytes(&b""[..]),
                    body,
                ];
                let _alias = (refs == 2).then(|| argv[2].clone());
                assert_eq!(argv[2].native_object_reference_count(), refs);
                let before = procedure_definition_object_state(&argv[2]);
                let completion = super::cmd_proc(&mut vm, &argv);
                assert_eq!(
                    completion.code,
                    tcl_runtime_api::Code::Ok,
                    "{engine}: {row}"
                );
                let original_refs = argv[2].native_object_reference_count();
                let after = procedure_definition_object_state(&argv[2]);
                let Some(Command::Proc(proc)) = vm.lookup_command("p") else {
                    panic!("missing native declaration")
                };
                assert!(proc.body.is_none());
                let stored = procedure_definition_object_state(&proc.body_src);
                let actual = vec![
                    kind.to_string(),
                    refs.to_string(),
                    "0".into(),
                    usize::from(
                        proc.body_src.native_object_identity() == argv[2].native_object_identity(),
                    )
                    .to_string(),
                    original_refs.to_string(),
                    before.0.into(),
                    before.1.to_string(),
                    after.0.into(),
                    after.1.to_string(),
                    stored.0.into(),
                    stored.1.to_string(),
                    proc.body_src.native_object_reference_count().to_string(),
                ];
                assert_eq!(actual, expected, "{engine}: {row}");
                compared += 1;
            }
        }
        assert_eq!(compared, 58);
    }

    #[test]
    fn procedure_definition_and_activation_match_42_native_observations() {
        use super::{Command, Value, Vm};
        let engines = [
            (
                "tcl8.4",
                include_str!("../tests/data/native_procedure_activation/8.4.txt"),
            ),
            (
                "tcl8.5",
                include_str!("../tests/data/native_procedure_activation/8.5.txt"),
            ),
            (
                "tcl8.6",
                include_str!("../tests/data/native_procedure_activation/8.6.txt"),
            ),
            (
                "tcl9.0",
                include_str!("../tests/data/native_procedure_activation/9.0.txt"),
            ),
            (
                "tcl9.1",
                include_str!("../tests/data/native_procedure_activation/9.1.txt"),
            ),
            (
                "jim",
                include_str!("../tests/data/native_procedure_activation/Jim.txt"),
            ),
        ];
        let mut compared = 0;
        for (engine, fixture) in engines {
            let profile = tcl_registry::model::ingress::resolve_environment(engine).unit_profile();
            let mut vm = Vm::new();
            vm.set_dialect_profile(profile);
            vm.set_compiler(Box::new(
                tcl_compiler::compile_service::BytecodeCompileService::for_profile(profile),
            ));
            let protocol = vm
                .native_invocation_dialect()
                .native_string_protocol()
                .unwrap();
            let rows = fixture
                .lines()
                .map(|line| {
                    tcl_syntax::list::split_native_list_bytes(line.as_bytes(), protocol)
                        .unwrap()
                        .into_iter()
                        .map(std::borrow::Cow::into_owned)
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();
            assert_eq!(rows.len(), 7);
            let mut actual = Vec::new();
            for body in ["set value \"", "error BODY"] {
                actual.push(vm.invoke_command(
                    "proc",
                    &[Value::string("p"), Value::string("x"), Value::string(body)],
                ));
                assert!(
                    matches!(vm.lookup_command("p"), Some(Command::Proc(proc)) if proc.body.is_none()),
                    "{engine}"
                );
                actual.push(vm.invoke_command("p", &[]));
                actual.push(vm.invoke_command("p", &[Value::string("OK")]));
            }
            let definition = vm.invoke_command(
                "proc",
                &[
                    Value::string("q"),
                    Value::string("x"),
                    Value::string("set side BODY;return OK"),
                ],
            );
            assert_eq!(definition.code, tcl_runtime_api::Code::Ok);
            vm.set_var_bytes(b"side", Value::string("BEFORE")).unwrap();
            actual.push(vm.invoke_command("q", &[]));
            assert_eq!(
                vm.get_var_bytes(b"side").unwrap().string_bytes().as_ref(),
                b"BEFORE"
            );
            for (row, completion) in rows.iter().zip(actual) {
                assert_eq!(
                    completion.code.as_int().to_string().as_bytes(),
                    row[1],
                    "{engine}/{row:?}"
                );
                assert_eq!(
                    vm.native_string_bytes(&completion.result).unwrap().as_ref(),
                    row[2],
                    "{engine}/{row:?}"
                );
                compared += 1;
            }
        }
        assert_eq!(compared, 42);
    }

    #[test]
    fn original_frame_selectors_match_native_cache_and_failure_order() {
        let fixtures = [
            (
                tcl_dialect::TclVersion::V8_4,
                include_str!("../../tcl-syntax/tests/data/native_frame_levels/8.4.20.jsonl"),
            ),
            (
                tcl_dialect::TclVersion::V8_5,
                include_str!("../../tcl-syntax/tests/data/native_frame_levels/8.5.19.jsonl"),
            ),
            (
                tcl_dialect::TclVersion::V8_6,
                include_str!("../../tcl-syntax/tests/data/native_frame_levels/8.6.18.jsonl"),
            ),
            (
                tcl_dialect::TclVersion::V9_0,
                include_str!("../../tcl-syntax/tests/data/native_frame_levels/9.0.4.jsonl"),
            ),
            (
                tcl_dialect::TclVersion::V9_1,
                include_str!("../../tcl-syntax/tests/data/native_frame_levels/9.1.0.jsonl"),
            ),
        ];
        let inputs: [&[u8]; 13] = [
            b"1",
            b"1\0X",
            b"#1",
            b"#1\0X",
            b"-1",
            b"+1",
            b"01",
            b"1.0",
            b"NaN",
            b"2147483648",
            b"4294967295",
            b"0",
            b"bad",
        ];
        let mut count = 0;
        let jim = tcl_registry::InvocationDialect::of_point(
            tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_84),
        );
        let fixtures = fixtures
            .into_iter()
            .map(|(version, fixture)| {
                (
                    tcl_registry::InvocationDialect::for_version(version),
                    fixture,
                )
            })
            .chain(std::iter::once((
                jim,
                include_str!("../../tcl-syntax/tests/data/native_frame_levels/jim0.84.jsonl"),
            )));
        for (dialect, fixture) in fixtures {
            let protocol = dialect.native_frame_level_protocol().unwrap();
            let jim_inputs: [&[u8]; 18] = [
                b"1",
                b"1\0X",
                b"#1",
                b"#1\0X",
                b"-1",
                b"+1",
                b"01",
                b"1.0",
                b"NaN",
                b"2147483648",
                b"4294967295",
                b"0",
                b"bad",
                b"#1 ",
                b"#9223372036854775808",
                b"#-0x8000000000000000",
                b"#-0",
                b"#0o1",
            ];
            let inputs = if protocol.is_jim084() {
                &jim_inputs[..]
            } else {
                &inputs[..]
            };
            for (case, line) in fixture.lines().enumerate() {
                assert_original_frame_selector(dialect, inputs, case, line);
                count += 1;
            }
        }
        assert_eq!(count, 101);
    }

    fn native_frame_field<'a>(line: &'a str, name: &str) -> &'a str {
        let marker = format!("\"{name}\":\"");
        line.split_once(&marker)
            .unwrap()
            .1
            .split('"')
            .next()
            .unwrap()
    }

    fn decode_native_frame_hex(value: &str) -> Vec<u8> {
        value
            .as_bytes()
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect()
    }

    fn assert_original_frame_selector(
        dialect: tcl_registry::InvocationDialect,
        inputs: &[&[u8]],
        case: usize,
        line: &str,
    ) {
        use super::OriginalFrameLevel;
        use tcl_registry::frame_effect::{NativeFrameLevelFailure, NativeFrameLevelResolution};
        use tcl_syntax::number::Number;
        use tcl_syntax::scalar_getter::NativeScalarCache;
        let version = dialect;
        let protocol = dialect.native_frame_level_protocol().unwrap();
        let original = match case.cmp(&inputs.len()) {
            std::cmp::Ordering::Less => crate::Value::from_string_bytes(inputs[case]),
            std::cmp::Ordering::Equal => crate::Value::int(1),
            std::cmp::Ordering::Greater => crate::Value::native_double(
                if case == inputs.len() + 1 {
                    1.0
                } else {
                    f64::NAN
                },
                dialect,
            ),
        };
        let alias = original.clone();
        let outcome = protocol
            .resolve_leading_object(
                2,
                &mut OriginalFrameLevel {
                    value: &original,
                    dialect,
                },
            )
            .unwrap();
        let expected = decode_native_frame_hex(native_frame_field(line, "result"));
        match outcome {
            Ok(NativeFrameLevelResolution {
                explicit: true,
                target,
            }) => {
                assert!(line.contains("\"code\":0"), "{version:?} {case}");
                assert_eq!(
                    expected,
                    target.to_string().as_bytes(),
                    "{version:?} {case}"
                );
            }
            Ok(NativeFrameLevelResolution {
                explicit: false, ..
            }) => {
                assert!(line.contains("\"code\":1"), "{version:?} {case}");
                assert!(
                    expected.starts_with(b"invalid command name"),
                    "{version:?} {case}"
                );
            }
            Err(NativeFrameLevelFailure::Primitive(record)) => {
                assert_eq!(expected, record.eval_message_bytes(), "{version:?} {case}");
            }
            Err(NativeFrameLevelFailure::BadLevel { name, .. }) => {
                let mut actual = b"bad level \"".to_vec();
                actual.extend_from_slice(&name);
                actual.push(b'"');
                assert_eq!(expected, actual, "{version:?} {case}");
            }
        }
        let actual_cache = if alias.native_frame_level_cache().is_some() {
            "levelReference"
        } else {
            match alias.native_scalar_cache() {
                Some(NativeScalarCache::Number(Number::Int(_))) => "integer",
                Some(NativeScalarCache::Number(Number::Double(_) | Number::Nan { .. })) => "double",
                None => "string",
                other => panic!("unexpected frame cache {other:?}"),
            }
        };
        let expected_cache = match native_frame_field(line, "after") {
            "int" | "wideInt" => "integer",
            other => other,
        };
        assert_eq!(actual_cache, expected_cache, "{version:?} {case}");
    }

    #[test]
    fn expression_source_objects_preserve_native_shared_representation_timing() {
        let cases = [
            ("set x [list 3]; expr $x; concat $x [list]", "3", "3 "),
            (
                "set x [list nope]; catch {expr $x}; concat $x [list]",
                "nope",
                "nope ",
            ),
            (
                "set x [list 1/0]; catch {expr $x}; concat $x [list]",
                "1/0",
                "1/0 ",
            ),
            (
                "set x [list 3]; set y $x; expr $y; concat $x [list]",
                "3",
                "3 ",
            ),
            (
                "set x [list 3]; expr {0 ? $x+0 : 1}; concat $x [list]",
                "3",
                "3",
            ),
            ("set x [list 3]; expr {$x+0}; concat $x [list]", "3", "3 "),
        ];
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let profile = tcl_registry::model::ingress::resolve_environment(dialect).unit_profile();
            for (source, c_result, jim_result) in cases {
                let mut vm = crate::Vm::new();
                vm.set_dialect_profile(profile);
                vm.set_compiler(Box::new(
                    tcl_compiler::compile_service::BytecodeCompileService::for_profile(profile),
                ));
                let completion = vm
                    .try_eval_source(source)
                    .unwrap_or_else(|error| panic!("{dialect}: {source}: {error:?}"));
                assert_eq!(
                    completion.code,
                    tcl_runtime_api::Code::Ok,
                    "{dialect}: {source}"
                );
                let expected = if dialect == "jim" {
                    jim_result
                } else {
                    c_result
                };
                assert_eq!(
                    &*completion.result.to_str(),
                    expected,
                    "{dialect}: {source}"
                );
            }
        }
    }

    #[test]
    fn expression_argv_validation_precedes_shared_source_coercion() {
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let profile = tcl_registry::model::ingress::resolve_environment(dialect).unit_profile();
            let mut vm = crate::Vm::new();
            vm.set_dialect_profile(profile);
            vm.set_compiler(Box::new(
                tcl_compiler::compile_service::BytecodeCompileService::for_profile(profile),
            ));
            let result = vm
                .try_eval_source(
                    "set x [list 3]; set code [catch {expr $x + 1} result]; list $code $result [concat $x [list]]",
                )
                .unwrap_or_else(|error| panic!("{dialect}: expression argv: {error:?}"));
            assert_eq!(result.code, tcl_runtime_api::Code::Ok);
            let expected = if dialect == "jim" {
                "1 {wrong # args: should be \"expr expression\"} 3"
            } else {
                "0 4 3"
            };
            assert_eq!(&*result.result.to_str(), expected, "{dialect}");
        }
    }

    #[test]
    fn expression_source_cache_uses_the_actual_native_grammar() {
        let mut modern = crate::Vm::new();
        modern.set_dialect_profile(tcl_dialect::DialectProfile::find("tcl9.0").unwrap());
        let source = crate::Value::string("0d10");
        assert!(modern.prepare_expression_value(&source).is_ok());
        assert!(
            modern.prepare_expression_value(&source).is_ok(),
            "warm cache"
        );
        let mut legacy = crate::Vm::new();
        legacy.set_dialect_profile(tcl_dialect::DialectProfile::find("tcl8.4").unwrap());
        assert!(
            legacy.prepare_expression_value(&source).is_err(),
            "an expression object cannot retain another engine's numeral grammar"
        );
        assert!(
            modern.prepare_expression_value(&source).is_ok(),
            "a failed conversion must not corrupt the original source bytes"
        );
    }

    #[test]
    fn checked_expression_cache_retains_operator_base_independently_of_lexing() {
        use tcl_runtime_api::{NativeExecutionError, NativeExpressionRefusal};
        let modern = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
        let mut restricted = modern.clone();
        restricted.expr_grammar_base = Some(tcl_dialect::TclVersion::V8_4);
        let restricted = restricted.intern();
        assert_eq!(modern.name, restricted.name);
        assert_eq!(modern.grammar, restricted.grammar);
        let source = crate::Value::string("2 ** 3");
        let mut vm = crate::Vm::new();
        vm.set_dialect_profile(modern);
        assert_eq!(
            {
                let invocation_argument = source.clone();
                vm.try_invoke_command("expr", std::slice::from_ref(&invocation_argument))
            }
            .unwrap()
            .result
            .to_str()
            .as_ref(),
            "8"
        );
        vm.set_dialect_profile(restricted);
        let NativeExecutionError::ExpressionRefusal(failure) = {
            let invocation_argument = source.clone();
            vm.try_invoke_command("expr", std::slice::from_ref(&invocation_argument))
        }
        .unwrap_err() else {
            panic!("operator syntax needs an expression provider");
        };
        assert_eq!(
            failure.reason,
            NativeExpressionRefusal::UnpresentedSyntaxFailure
        );
        assert_eq!(failure.source.bytes(), b"2 ** 3");
        assert_eq!(failure.source_profile, restricted.cache_key());
        assert_eq!(
            failure.source_profile.profile().expr_grammar_base,
            Some(tcl_dialect::TclVersion::V8_4)
        );
        assert_eq!(
            failure.native_profile.profile().runtime_version(),
            Some(tcl_dialect::TclVersion::V9_0)
        );
        vm.set_dialect_profile(modern);
        assert_eq!(
            vm.try_invoke_command("expr", &[source])
                .unwrap()
                .result
                .to_str()
                .as_ref(),
            "8"
        );
    }

    #[test]
    fn legacy_bareword_errors_use_actual_fixed_function_name_resolution() {
        for (dialect, source, expected) in [
            (
                "tcl8.4",
                "nope",
                "syntax error in expression \"nope\": variable references require preceding $",
            ),
            (
                "tcl8.4",
                "abs",
                "syntax error in expression \"abs\": expected parenthesis enclosing function arguments",
            ),
            ("jim", "nope", "syntax error in expression: \"nope\""),
            (
                "jim",
                "abs",
                "syntax error in expression: \"abs\": function requires parentheses",
            ),
        ] {
            let profile = tcl_registry::model::ingress::resolve_environment(dialect).unit_profile();
            let mut vm = crate::Vm::new();
            vm.set_dialect_profile(profile);
            let completion = vm.try_eval_expr(source).unwrap();
            assert_eq!(completion.code, tcl_runtime_api::Code::Error);
            assert_eq!(completion.result.to_str().as_ref(), expected);
            assert_eq!(
                super::opt_get(&completion.options, "-errorcode")
                    .unwrap()
                    .to_str()
                    .as_ref(),
                "NONE"
            );
        }
    }

    fn vm_without_expression_provider() -> (crate::Vm, &'static tcl_dialect::DialectProfile) {
        let profile = tcl_dialect::DialectProfile::irules();
        let host = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
        let mut vm = crate::Vm::new();
        vm.set_dialect_profile(profile);
        assert!(vm.set_native_engine_profile(host));
        assert!(vm.set_command_surface_profile(host));
        assert!(vm.set_logical_name_provider(
            tcl_syntax::naming::NamePolicyProtocol::authored_tcl(tcl_dialect::TclVersion::V8_4)
        ));
        assert!(vm.set_logical_numeric_provider(
            tcl_syntax::logical_numeric_simulation::AuthoredLogicalNumericSimulation::Tcl84Core
        ));
        assert!(vm.set_logical_compiled_variable_provider(tcl_registry::native_compiled_variables::LogicalCompiledVariableProvider::Tcl84CoreSimulation));
        assert!(vm.set_logical_source_word_provider(
            tcl_registry::invocation_words::LogicalSourceWordProvider::Tcl84CoreSimulation
        ));
        vm.set_compiler(Box::new(
            tcl_compiler::compile_service::BytecodeCompileService::for_profile(profile),
        ));
        (vm, profile)
    }

    #[test]
    fn unsupported_expression_is_neutral_and_preserves_only_prior_effects() {
        use tcl_runtime_api::{NativeExecutionError, NativeExpressionRefusal};
        let cases = [
            "catch {expr $source; set ::late 1} caught options; set ::handler 1",
            "try {expr $source; set ::late 1} on error {} {set ::handler 1} finally {set ::cleanup 1}",
            "set ::target [expr $source]; set ::late 1",
            "proc p {} {set local 1; trace add variable local unset observe; expr $::source; set ::late 1}; p",
        ];
        for body in cases {
            let (mut vm, profile) = vm_without_expression_provider();
            let setup = "set early 0; set late 0; set handler 0; set cleanup 0; set observer 0; set caught ORIGINAL; set options ORIGINAL; set target ORIGINAL; set errorInfo ORIGINAL; set errorCode ORIGINAL; set source {1 @ 2}; proc observe args {set ::observer 1}; trace add execution expr leave observe";
            let result = vm.try_eval_source(&format!("{setup}; set ::early 1; {body}"));
            let refusal = result.unwrap_err();
            let NativeExecutionError::ExpressionRefusal(failure) = refusal else {
                panic!("{body}: expected expression refusal, got {refusal:?}");
            };
            assert_eq!(failure.reason, NativeExpressionRefusal::UnsupportedGrammar);
            assert_eq!(failure.source.bytes(), b"1 @ 2");
            assert_eq!(failure.source_profile, profile.cache_key());
            assert_eq!(failure.native_profile, profile.cache_key());
            assert_eq!(failure.interpreter.owner, vm.owner_nonce);
            assert_eq!(failure.interpreter.interpreter, 0);
            assert_eq!(failure.frame, usize::from(body.starts_with("proc p")));
            assert_eq!(failure.namespace.as_ref(), "");
            assert_eq!(failure.namespace_token, 0);
            for (name, expected) in [
                ("early", "1"),
                ("late", "0"),
                ("handler", "0"),
                ("cleanup", "0"),
                ("observer", "0"),
                ("caught", "ORIGINAL"),
                ("options", "ORIGINAL"),
                ("target", "ORIGINAL"),
                ("errorInfo", "ORIGINAL"),
                ("errorCode", "ORIGINAL"),
            ] {
                assert_eq!(
                    vm.get_var(name).unwrap().to_str().as_ref(),
                    expected,
                    "{body}: {name}"
                );
            }
            assert_eq!(vm.current_level(), 0, "host refusal retires proc storage");
            assert_eq!(
                vm.try_eval_source("set resumed YES")
                    .unwrap()
                    .result
                    .to_str()
                    .as_ref(),
                "YES",
                "a new independent host entry can run after receiving the typed refusal"
            );
        }
    }

    #[test]
    fn unsupported_expression_refusal_survives_child_interpreter_switches() {
        use tcl_runtime_api::{NativeExecutionError, NativeExpressionRefusal};
        let (mut vm, _) = vm_without_expression_provider();
        let result = vm.try_eval_source(
            "set parentLate 0; set child [interp create]; interp eval $child {set early 1; set late 0; set source {1 @ 2}; catch {expr $source} result; set late 1}; set parentLate 1",
        );
        let error = result.unwrap_err();
        let NativeExecutionError::ExpressionRefusal(failure) = error else {
            panic!("child expression needs a provider: {error:?}");
        };
        assert_eq!(failure.reason, NativeExpressionRefusal::UnsupportedGrammar);
        assert_eq!(failure.source.bytes(), b"1 @ 2");
        assert_eq!(failure.interpreter.owner, vm.owner_nonce);
        assert_ne!(failure.interpreter.interpreter, 0);
        assert_eq!(failure.frame, 0);
        assert_eq!(vm.get_var("parentLate").unwrap().to_str().as_ref(), "0");
        let resumed = vm
            .try_eval_source("interp eval $child {list $early $late [info exists result]}")
            .unwrap();
        assert_eq!(resumed.result.to_str().as_ref(), "1 0 0");
    }

    #[test]
    fn jim_static_storage_matches_the_fixed_native_fixture() {
        let profile = tcl_registry::model::ingress::resolve_environment("jim").unit_profile();
        let source =
            include_str!("../../tcl-syntax/tests/data/body_execution/native-static-storage.tcl");
        let expected = include_str!(
            "../../tcl-syntax/tests/data/body_execution/native-static-storage-jim.txt"
        );
        for (line, expected) in source.lines().skip(1).zip(expected.lines()) {
            let words = tcl_syntax::list::split_list_jim(line);
            let wanted = tcl_syntax::list::split_list_jim(expected);
            assert_eq!(words.len(), 3);
            assert_eq!(wanted.len(), 3);
            assert_eq!(words[1], wanted[0]);
            let mut vm = crate::Vm::new();
            vm.set_dialect_profile(profile);
            vm.set_compiler(Box::new(
                tcl_compiler::compile_service::BytecodeCompileService::for_profile(profile),
            ));
            let completion = vm.eval_source(&words[2]).unwrap();
            assert_eq!(
                completion.code.as_int().to_string(),
                &*wanted[1],
                "{}",
                words[1]
            );
            assert_eq!(&*completion.result.to_str(), &*wanted[2], "{}", words[1]);
        }
    }

    #[test]
    fn jim_captured_aliases_follow_the_selected_logical_level() {
        let cases = [
            (
                "proc maker {} {set a KEEP; upvar 0 a x; proc p {} {&x} {set x}}; maker; list [catch p r] $r",
                "1 {can't read \"x\": no such variable}",
            ),
            (
                "proc maker {} {set a KEEP; upvar 0 a x; proc p {} {&x} {set x}}; maker; proc unrelated {} {set a OTHER; list [catch p r] $r}; unrelated",
                "0 OTHER",
            ),
            (
                "proc maker {} {set a OLD; upvar 0 a x; proc p {} {&x} {set x MODIFIED}; proc reader {} {&x} {set x}}; maker; list [p] [catch reader r] $r",
                "MODIFIED 1 {can't read \"x\": no such variable}",
            ),
        ];
        let profile = tcl_registry::model::ingress::resolve_environment("jim").unit_profile();
        for (source, wanted) in cases {
            let mut vm = crate::Vm::new();
            vm.set_dialect_profile(profile);
            vm.set_compiler(Box::new(
                tcl_compiler::compile_service::BytecodeCompileService::for_profile(profile),
            ));
            let completion = vm.eval_source(source).unwrap();
            assert_eq!(
                completion.code,
                tcl_runtime_api::Code::Ok,
                "{}",
                completion.result.to_str()
            );
            assert_eq!(completion.result.to_str().as_ref(), wanted, "{source}");
        }
    }

    #[test]
    fn procedure_definition_result_follows_the_native_protocol() {
        let mut profiles: Vec<_> = tcl_dialect::TclVersion::ALL
            .iter()
            .map(|version| {
                tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap()
            })
            .collect();
        profiles.push(tcl_registry::model::ingress::resolve_environment("jim").unit_profile());
        for profile in profiles {
            let mut vm = crate::Vm::new();
            vm.set_dialect_profile(profile);
            vm.set_compiler(Box::new(
                tcl_compiler::compile_service::BytecodeCompileService::for_profile(profile),
            ));
            let completion = vm.eval_source("proc written {} {}").unwrap();
            assert_eq!(completion.code, tcl_runtime_api::Code::Ok);
            let wanted = match profile.parameter_grammar().unwrap() {
                tcl_dialect::ParameterGrammar::Tcl => "",
                tcl_dialect::ParameterGrammar::Jim => "written",
            };
            assert_eq!(&*completion.result.to_str(), wanted, "{}", profile.name);
        }
    }

    #[test]
    fn formal_activation_obeys_the_selected_native_plan() {
        let cases = [
            ("proc p {x x} {set x}; p FIRST SECOND", "FIRST", "SECOND"),
            (
                "proc p {args x} {list $args $x}; set c [catch {p A B C} r]; list $c $r",
                "1 {wrong # args: should be \"p args x\"}",
                "0 {{A B} C}",
            ),
            (
                "proc p {{x DEF} y} {list $x $y}; set c [catch {p VAL} r]; list $c $r",
                "1 {wrong # args: should be \"p ?x? y\"}",
                "0 {DEF VAL}",
            ),
            (
                "set v OLD; proc p {&x} {set x NEW}; p v; set v",
                "OLD",
                "NEW",
            ),
            ("proc p {{&x DEFAULT}} {set {&x}}; p", "DEFAULT", "DEFAULT"),
            (
                "proc p {&x} {}; set c [catch {p MISSING} r]; list $c $r",
                "0 {}",
                "0 {}",
            ),
            (
                "set c [catch {proc p {a(k)} {set a(k)}} r]; if {$c} {set c} else {list $c [p ARG]}",
                "1",
                "0 ARG",
            ),
            (
                "namespace eval param {}; set c [catch {proc p {::param::x} {set ::param::x}} r]; if {$c} {set c} else {list $c [p ARG]}",
                "1",
                "0 ARG",
            ),
            (
                "proc p {{args rest} x} {list $rest $x}; set c [catch {p A B C} r]; list $c $r",
                "1 {wrong # args: should be \"p ?args? x\"}",
                "0 {{A B} C}",
            ),
        ];
        let mut profiles: Vec<_> = tcl_dialect::TclVersion::ALL
            .iter()
            .map(|version| {
                tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap()
            })
            .collect();
        profiles.push(tcl_registry::model::ingress::resolve_environment("jim").unit_profile());
        for profile in profiles {
            for (source, tcl, jim) in cases {
                let mut vm = crate::Vm::new();
                vm.set_dialect_profile(profile);
                vm.set_compiler(Box::new(
                    tcl_compiler::compile_service::BytecodeCompileService::for_profile(profile),
                ));
                let result = vm.eval_source(source).unwrap();
                assert_eq!(
                    result.code,
                    tcl_runtime_api::Code::Ok,
                    "{}: {source}: {}",
                    profile.name,
                    result.result.to_str()
                );
                let expected =
                    if profile.parameter_grammar() == Some(tcl_dialect::ParameterGrammar::Jim) {
                        jim
                    } else {
                        tcl
                    };
                assert_eq!(
                    &*result.result.to_str(),
                    expected,
                    "{}: {source}",
                    profile.name
                );
            }
        }
    }

    #[test]
    fn script_object_materialization_keeps_opaque_counted_source() {
        use crate::{Value, Vm};
        for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let mut vm = Vm::new();
            vm.set_dialect_profile(
                tcl_registry::model::ingress::resolve_environment(engine).unit_profile(),
            );
            let original =
                Value::new_native_string_bytes(&b"set opaque {\xff\0TAIL}; set opaque"[..]);
            let source = vm
                .native_script_source_image(&original)
                .unwrap_or_else(|_| panic!("physical script issuer {engine}"));
            assert_eq!(source.bytes(), b"set opaque {\xff\0TAIL}; set opaque");
            assert_eq!(
                original.resident_string_bytes().unwrap().as_ref(),
                source.bytes()
            );
        }
    }

    #[test]
    fn original_list_eval_retains_member_identity_without_materialising_source() {
        use crate::{Value, Vm};
        use tcl_runtime_api::Code;
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let profile = tcl_registry::model::ingress::resolve_environment(dialect).unit_profile();
            let mut vm = Vm::new();
            vm.set_dialect_profile(profile);
            vm.set_compiler(Box::new(
                tcl_compiler::compile_service::BytecodeCompileService::for_profile(profile),
            ));
            let result = Value::new_native_string_bytes(&b"\xff\0TAIL"[..]);
            let script = Value::list(vec![Value::string("return"), result.clone()]);
            let completion = {
                let invocation_argument = script.clone();
                vm.try_invoke_command("eval", std::slice::from_ref(&invocation_argument))
            }
            .unwrap();
            assert_eq!(completion.code, Code::Return, "{dialect}");
            assert_eq!(
                completion.result.native_object_identity(),
                result.native_object_identity(),
                "{dialect}"
            );
            assert_eq!(completion.result.string_bytes().as_ref(), b"\xff\0TAIL");
            assert!(script.resident_string_bytes().is_none(), "{dialect}");
        }
    }

    #[test]
    fn jim_native_catch_preserves_its_flag_and_exit_protocol() {
        use crate::{Value, Vm};
        use tcl_runtime_api::Code;
        let profile = tcl_registry::model::ingress::resolve_environment("jim").unit_profile();
        let mut vm = Vm::new();
        vm.set_dialect_profile(profile);
        vm.set_compiler(Box::new(
            tcl_compiler::compile_service::BytecodeCompileService::for_profile(profile),
        ));
        let words = |args: &[&str]| {
            args.iter()
                .map(|word| Value::string(*word))
                .collect::<Vec<_>>()
        };
        vm.invoke_command("set", &words(&["r", "ORIGINAL"]));
        vm.invoke_command("set", &words(&["o", "OPTIONS"]));
        let completion = vm.invoke_command("catch", &words(&["-nook", "set x VALUE", "r", "o"]));
        assert_eq!(completion.code, Code::Ok);
        assert_eq!(&*completion.result.to_str(), "VALUE");
        assert_eq!(
            &*vm.invoke_command("set", &words(&["r"])).result.to_str(),
            "ORIGINAL"
        );
        let completion = vm.invoke_command("catch", &words(&["-ok", "set x VALUE", "r", "o"]));
        assert_eq!(&*completion.result.to_str(), "0");
        assert_eq!(
            &*vm.invoke_command("set", &words(&["r"])).result.to_str(),
            "VALUE"
        );
        let completion = vm.invoke_command("catch", &words(&["-exit", "exit 12", "r", "o"]));
        assert_eq!(completion.code, Code::Ok);
        assert_eq!(&*completion.result.to_str(), "6");
        assert_eq!(
            &*vm.invoke_command("set", &words(&["r"])).result.to_str(),
            "12"
        );
        assert!(vm.take_exit().is_none());
        let completion = vm.invoke_command("catch", &words(&["-noerror", "error BAD", "r", "o"]));
        assert_eq!(completion.code, Code::Error);
        assert_eq!(&*completion.result.to_str(), "BAD");
        let completion = vm.invoke_command("catch", &words(&["exit 12", "r", "o"]));
        assert_eq!(completion.code, Code::from_int(6));
        assert_eq!(vm.take_exit(), Some(12));
    }

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

#[cfg(test)]
#[path = "command/native_frame_context_tests.rs"]
mod native_frame_context_tests;
