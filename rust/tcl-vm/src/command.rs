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
use tcl_runtime_api::{Code, Completion, NsId, OoId};

#[cfg(test)]
use crate::error::TclError;
use crate::interp::{Vm, err, ok};
use crate::value::Value;

#[cfg(test)]
mod native_definition_order_tests;
mod native_encoding_source;
#[cfg(test)]
mod native_encoding_tests;
mod native_procedure;
#[cfg(test)]
mod native_procedure_name_tests;
pub use native_procedure::NativeProcedureCommand;
pub(crate) use native_procedure::{NativeProcedureReference, NativeProcedureResources};

mod apply_lambda;
mod carried_options;
mod catch;
mod child_interpreters;
mod encoding_command;
mod error_completion;
mod frame_level;
mod procedure_definition;
mod rename;
mod variable_links;

pub(crate) use apply_lambda::OriginalLambdaDefinition;
#[cfg(test)]
pub(crate) use apply_lambda::build_lambda_proc;
use apply_lambda::cmd_apply;
#[cfg(test)]
pub(crate) use carried_options::opt_get;
pub(crate) use carried_options::{
    OriginalCompletionOptions, apply_return_options, cmd_return, completion_option_failure,
    completion_option_rows_checked, completion_option_rows_resident, completion_options,
    opt_get_checked, opt_get_checked_using, opt_get_resident_checked, option_code_checked,
    option_integer_checked, resolved_error_code, settle_control_options, with_return_level,
    with_return_option,
};
use catch::cmd_catch;
#[cfg(test)]
use child_interpreters::skip_hidden_options;
use child_interpreters::{cmd_alias, cmd_interp};
pub(crate) use child_interpreters::{hidden_invocation_options, resolve_interp_option_bytes};
pub(crate) use encoding_command::refresh_encoding_profile;
use encoding_command::register_encoding;
use error_completion::cmd_error;
pub(crate) use error_completion::{
    completion_from_cmd_error, completion_from_tcl_error, err_with_code, options_dict,
};
#[cfg(test)]
use frame_level::OriginalFrameLevel;
#[cfg(test)]
pub(crate) use frame_level::runtime_explicit_frame_selection;
pub(crate) use frame_level::{cmd_uplevel, compiled_uplevel, runtime_frame_selection};
use procedure_definition::cmd_proc;
#[cfg(test)]
pub(crate) use procedure_definition::parse_params;
#[cfg(test)]
use procedure_definition::procedure_header_compilation;
pub(crate) use procedure_definition::{
    cmd_authored_rule_proc, parse_params_in, parse_params_value,
};
use rename::cmd_rename;
use variable_links::cmd_global;
pub(crate) use variable_links::{
    cmd_upvar, cmd_variable, upvar_link_error_bytes, upvar_link_error_bytes_in,
};

type CatchErrorMetadata = Option<(std::rc::Rc<[u8]>, Value)>;

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

/// Formal records built once from a declaration's parameters. A clone starts
/// empty, so a copied and then modified declaration builds its own.
#[derive(Default)]
pub(crate) struct FormalParameterCache(
    std::cell::OnceCell<Rc<[tcl_syntax::formal_params::ByteFormalParameter]>>,
);

impl Clone for FormalParameterCache {
    fn clone(&self) -> Self {
        Self::default()
    }
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
    /// Original body execution context, independent of the publication slot.
    pub(crate) execution_namespace: crate::interp::ProcedureNamespace,
    /// Formal parameters in order.
    pub params: Vec<Param>,
    /// Native formal parsing and activation policy captured at definition.
    pub parameter_grammar: tcl_dialect::ParameterGrammar,
    pub(crate) formals: FormalParameterCache,
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
    /// For an `apply` lambda, the lambda expression C names in the
    /// `(lambda term "…" line N)` errorInfo frame in place of a procedure's.
    pub(crate) lambda_term: Option<Value>,
}

impl ProcDef {
    pub(crate) fn formal_parameters(&self) -> Rc<[tcl_syntax::formal_params::ByteFormalParameter]> {
        Rc::clone(
            self.formals
                .0
                .get_or_init(|| formal_parameters(&self.params, self.parameter_grammar).into()),
        )
    }
}

pub(crate) fn formal_parameters(
    params: &[Param],
    grammar: tcl_dialect::ParameterGrammar,
) -> Vec<tcl_syntax::formal_params::ByteFormalParameter> {
    params
        .iter()
        .map(|parameter| tcl_syntax::formal_params::ByteFormalParameter {
            name: parameter.name.as_bytes().to_vec(),
            default: parameter.default.as_ref().map(|value| {
                if grammar == tcl_dialect::ParameterGrammar::Jim
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
    Alias {
        /// Target command + fixed prefix arguments.
        words: Rc<Vec<Value>>,
        /// Selected original C alias-record spelling, independent of lookup slots.
        publication_name: tcl_core_types::NameBytes,
        /// C retains its original registration object; Jim retains no caller name.
        registration_name: Option<Value>,
    },
    /// Jim's original prefix List header; target lookup retains the caller namespace.
    CallerAlias(Value),
    /// C evaluates target words in the target's global frame. Jim retains an
    /// original parent prefix List and copies child arguments and the result
    /// across that parent's current evaluation context.
    CrossAlias {
        /// The interpreter the target words run in.
        target: crate::interp::InterpId,
        /// C target words, or a neutral snapshot of Jim's original prefix members.
        words: Rc<Vec<Value>>,
        /// Original parent-owned Jim List; C retains its independent word vector.
        original_jim_prefix: Option<Rc<Value>>,
        /// Selected child registration report in the source interpreter's realm.
        publication_name: tcl_core_types::NameBytes,
        /// C retains its original registration object; Jim retains no caller name.
        registration_name: Option<Value>,
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

/// Register this backend's commands under the explicitly selected constructor purpose.
pub(crate) fn register_builtins_with_native_core(
    vm: &mut Vm,
    native: tcl_registry::special_vars::NativeBootstrapProtocol,
) {
    vm.register_stock_builtin("set", cmd_set);
    refresh_jim_exists(vm);
    vm.register_stock_builtin("puts", cmd_puts);
    vm.register_stock_builtin("source", cmd_source);
    vm.register_stock_builtin("tcl::build-info", cmd_build_info);
    vm.register_stock_builtin("interp", cmd_interp);
    if vm
        .actual_native_invocation_dialect()
        .native_bootstrap_protocol()
        .is_some_and(
            tcl_registry::special_vars::NativeBootstrapProtocol::registers_core_init_auto_path,
        )
    {
        vm.register_stock_builtin("::tcl::InitAutoPath", Vm::init_auto_path);
    }
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
    register_encoding(vm);
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
    crate::cmd_math::register(vm, native);
    if native.registers_core_binary() {
        crate::cmd_binary::register(vm);
    }
    if native.registers_core_mathop() {
        crate::cmd_mathop::register(vm);
    }
    crate::cmd_lseq::register(vm);
    crate::cmd_prefix::register(vm);
    crate::cmd_namespace::register(vm);
    crate::cmd_package::register(vm);
    crate::cmd_regexp::register(vm);
    crate::cmd_switch::register(vm);
    crate::cmd_trace::register(vm);
    crate::cmd_try::register_for_bootstrap(vm, native);
    if native.initializes_tcl_oo() {
        crate::cmd_oo::register(vm);
    }
    crate::cmd_coro::register(vm);
    crate::cmd_event::register(vm);
    crate::cmd_thread::register(vm);
    // `string` last, as the registration order has always had it.
    crate::cmd_string::register(vm);
    // With the command table complete, attach the registry's identities to the
    // builtins in it.
    vm.attach_identities();
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
                return vm.cmd_error(CmdError::with_error_code(
                    format!("expected integer but got \"{}\"", c.to_str()),
                    "TCL VALUE NUMBER",
                ));
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
    native_encoding_source::source(vm, args)
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
    let (kind, name) = match args {
        [name] => (
            tcl_registry::commands::tcl::NativeJimExistsKind::Variable,
            name,
        ),
        [option, name] => match jim_exists_kind(vm, option) {
            Ok(kind) => (kind, name),
            Err(completion) => return completion,
        },
        _ => return native_wrong_args(vm, "?-command|-proc|-alias|-channel|-var? name"),
    };
    if kind != tcl_registry::commands::tcl::NativeJimExistsKind::Variable {
        return jim_command_exists(vm, kind, name);
    }
    let name = match vm.native_name_operand_bytes_checked(name) {
        Ok(name) => name,
        Err(error) => return completion_from_cmd_error(vm, error),
    };
    let exists = vm.get_var_bytes(&name).is_some();
    if let Some(refusal) = vm.refused_completion() {
        return refusal;
    }
    ok(Value::bool(exists))
}

/// Jim's `exists` selector, through its original enum lookup.
fn jim_exists_kind(
    vm: &mut Vm,
    option: &Value,
) -> Result<tcl_registry::commands::tcl::NativeJimExistsKind, Completion<Value>> {
    let Some(protocol) = vm.native_invocation_dialect().native_jim_exists_protocol() else {
        return Err(vm.refuse_host_command("Jim exists selector protocol is unavailable".into()));
    };
    match vm.native_jim_enum_from_original_checked(
        option,
        &protocol.table(),
        tcl_registry::native_jim_enum::NativeJimEnumFlags::options(false),
        None,
    ) {
        Ok(Ok(ordinal)) => Ok(protocol
            .selected(ordinal)
            .expect("original exists enum ordinal")),
        Ok(Err(message)) => Err(completion_from_cmd_error(
            vm,
            CmdError::with_error_code_bytes(message.unwrap_or_default(), b"NONE"),
        )),
        Err(error) => Err(completion_from_cmd_error(vm, error.into())),
    }
}

/// Jim's command-record flags: any command, a procedure, or an alias.
fn jim_command_exists(
    vm: &mut Vm,
    kind: tcl_registry::commands::tcl::NativeJimExistsKind,
    name: &Value,
) -> Completion<Value> {
    use tcl_registry::commands::tcl::NativeJimExistsKind as Kind;
    let name = match vm.native_name_operand_bytes_checked(name) {
        Ok(name) => name,
        Err(error) => return completion_from_cmd_error(vm, error),
    };
    let command = match vm.lookup_command_bytes_checked(vm.current_ns_id(), &name) {
        Ok(found) => found.map(|(_, command)| command),
        Err(_) => {
            return vm.refuse_host_command("Jim exists command lookup is unavailable".into());
        }
    };
    if let Some(refusal) = vm.refused_completion() {
        return refusal;
    }
    let alias = command.as_ref().is_some_and(|command| {
        matches!(
            command,
            Command::Alias { .. } | Command::CallerAlias(_) | Command::CrossAlias { .. }
        )
    });
    let exists = match (kind, command) {
        (_, None) => false,
        (Kind::Command, Some(_)) => true,
        (Kind::Procedure, Some(command)) => matches!(command, Command::Proc(_)),
        (Kind::Alias, Some(_)) => alias,
        (Kind::Channel, Some(command)) if alias || matches!(command, Command::Proc(_)) => false,
        (Kind::Channel | Kind::Variable, Some(_)) => {
            return vm.refuse_host_command("Jim channel command flag is unavailable".into());
        }
    };
    if kind == Kind::Alias
        && let Err(refusal) = vm.observe_original_jim_import_alias(&name, exists)
    {
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
    if !{
        let original = script.native_object_snapshot();
        let dispatch = protocol.dispatches_list(purpose, &original);
        #[cfg(debug_assertions)]
        if matches!(
            purpose,
            tcl_registry::native_eval_object::EvalObjectPurpose::UpLevel
        ) && std::env::var("TCL_LSP_NATIVE_ADMISSION_DIAGNOSTIC").as_deref()
            == Ok("first-decline-v1")
        {
            let list = match &original.cache {
                tcl_syntax::native_object::NativeObjectCacheSnapshot::List {
                    length,
                    canonical,
                } => Some((*length, *canonical)),
                _ => None,
            };
            eprintln!(
                "TCL_NATIVE_UPLEVEL_ORIGINAL_HEADER selected={protocol:?} resident={} list={list:?} dispatch={dispatch}",
                original.resident.is_some(),
            );
        }
        dispatch
    } {
        return Ok(None);
    }
    let words = script.cached_list_representation().map(|(words, _)| words);
    #[cfg(debug_assertions)]
    if matches!(
        purpose,
        tcl_registry::native_eval_object::EvalObjectPurpose::UpLevel
    ) {
        observe_original_uplevel_caller(vm, words.as_ref());
    }
    Ok(words)
}

#[cfg(debug_assertions)]
fn observe_original_uplevel_caller(vm: &Vm, words: Option<&crate::NativeListItems>) {
    use std::sync::atomic::{AtomicU64, Ordering};
    static CALLS: AtomicU64 = AtomicU64::new(0);
    if std::env::var("TCL_LSP_NATIVE_ADMISSION_DIAGNOSTIC").as_deref() != Ok("first-decline-v1") {
        return;
    }
    let Ok(previous) = CALLS.try_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
    else {
        return;
    };
    let ordinal = previous + 1;
    if ordinal > 32 && !ordinal.is_power_of_two() {
        return;
    }
    let head = words
        .and_then(|words| words.first())
        .and_then(Value::resident_string_bytes);
    let head_bytes = head.as_deref().map(|bytes| &bytes[..bytes.len().min(32)]);
    let caller = vm.native_diagnostic_proc_name_prefix();
    let info_words = words.filter(|_| head.as_deref() == Some(b"::tmm::_orig_info".as_slice()));
    let subcommand = info_words
        .and_then(|words| words.get(1))
        .and_then(Value::resident_string_bytes);
    let receiver = info_words
        .and_then(|words| words.get(2))
        .and_then(Value::resident_string_bytes);
    let subcommand_bytes = subcommand
        .as_deref()
        .map(|bytes| &bytes[..bytes.len().min(32)]);
    let receiver_bytes = receiver
        .as_deref()
        .map(|bytes| &bytes[..bytes.len().min(128)]);
    eprintln!(
        "TCL_NATIVE_UPLEVEL_CALLER ordinal={ordinal} level={} caller_key_prefix={:?} caller_key_len={:?} head={head_bytes:?} head_len={:?} info_subcommand={subcommand_bytes:?} info_subcommand_len={:?} info_receiver={receiver_bytes:?} info_receiver_len={:?}",
        vm.current_level(),
        caller.map(|(prefix, _)| prefix),
        caller.map(|(_, length)| length),
        head.as_ref().map(|bytes| bytes.len()),
        subcommand.as_ref().map(|bytes| bytes.len()),
        receiver.as_ref().map(|bytes| bytes.len()),
    );
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
                let unit = match unit.with_deferred_script_original(script) {
                    Ok(unit) => unit,
                    Err(error) => return completion_from_cmd_error(vm, error.into()),
                };
                vm.pending.eval = Some(crate::exec::EvalReq {
                    selected_frame_restore: None,
                    package: None,
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

/// `subst ?-nobackslashes? ?-nocommands? ?-novariables? string` — perform
/// backslash / command / variable substitution on a string.
fn cmd_subst(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    // C (`TclNRSubstObjCmd`): the *last* argument is the string and every
    // argument before it is an option, matched by unique abbreviation. So a
    // non-option word anywhere but last is a `bad option` error (subst-1.2/7.1),
    // and `-nov`/`-nob`/`-noc` are accepted as prefixes (subst-7.7).
    use tcl_registry::substitution::{NativeSubstitutionOptions, SubstitutionOptionError};
    let Some(options) = NativeSubstitutionOptions::select(vm.actual_native_invocation_dialect())
    else {
        return vm.refuse_host_command("original Subst option declaration is unavailable".into());
    };
    let Some((string, opts)) = args.split_last() else {
        return native_wrong_args(vm, options.usage());
    };
    let table = tcl_cmd_core::prefix::OptionTable::abbreviating(options.noun(), options.names());
    let mut selected = Vec::with_capacity(opts.len());
    for opt in opts {
        match table.index_of_original(vm, opt) {
            Ok(index) => selected.push(index),
            Err(error) => return completion_from_cmd_error(vm, error),
        }
    }
    let kinds = match options.kinds(&selected) {
        Ok(kinds) => kinds,
        Err(SubstitutionOptionError::MixedFamilies) => {
            return err("cannot combine positive and negative options");
        }
        Err(SubstitutionOptionError::UnknownOption) => {
            return vm.refuse_host_command("foreign Subst option ordinal".into());
        }
    };
    let (backslashes, commands, variables) = (kinds.backslashes, kinds.commands, kinds.variables);
    let compiled = match vm.prepare_original_substitution(
        string,
        tcl_runtime_api::native_substitution::NativeSubstitutionFlags::new(
            backslashes,
            commands,
            variables,
        ),
    ) {
        Ok(compiled) => compiled,
        Err(error) => return completion_from_tcl_error(vm, error),
    };
    // Defer to the *explicit* stack so a `yield` inside a `[…]` stays yieldable:
    // park the template + switches in the pending subst request, drained by
    // the trampoline into a scanner-driven subst frame (mirrors `cmd_catch`'s
    // the pending catch request). The frame's accumulated result replaces this builtin's
    // placeholder; on the native `invoke_command` fallback it runs via a nested
    // drive (not yieldable, as before).
    let original = vm
        .actual_native_string_protocol()
        .filter(|protocol| protocol.is_jim084())
        .map(|_| string.native_lifetime_lease());
    let uses_compiled_template = compiled.is_some();
    vm.pending.subst = Some(crate::exec::SubstReq {
        compiled,
        original,
        control: crate::subst::SubstitutionControl::Command,
        template: if uses_compiled_template
            || vm
                .actual_native_string_protocol()
                .is_some_and(tcl_syntax::native_string::NativeStringProtocol::is_jim084)
        {
            Vec::<u8>::new().into()
        } else {
            match vm.native_name_operand_bytes_checked(string) {
                Ok(bytes) => bytes.to_vec().into(),
                Err(error) => {
                    return completion_from_cmd_error(vm, error);
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
            match vm.native_jim_compare_immediate_checked(first, &table, 0) {
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
    // The words are read as the interpreter reads them: a list or dict whose
    // members are numbers produces its string through the interpreter's
    // numeric operation, which an unchecked read cannot reach.
    let text_of = |vm: &mut Vm, value: &Value| {
        tcl_syntax::value::ValueOps::native_string_bytes(vm, value)
            .map(|bytes| {
                // A Tcl NUL is stored as `C0 80` and leaves the channel as a NUL.
                tcl_syntax::raw_string::RawString::from_bytes(bytes.as_ref())
                    .unicode()
                    .map_or_else(
                        |_| String::from_utf8_lossy(&bytes).into_owned(),
                        |text| text.to_string(),
                    )
            })
            .map_err(|error| completion_from_cmd_error(vm, error.into()))
    };
    let (channel, text) = match rest {
        [string] => match text_of(vm, string) {
            Ok(text) => ("stdout".to_string(), text),
            Err(completion) => return completion,
        },
        // `puts ?-nonewline? channelId string`
        [channel, string] => match (text_of(vm, channel), text_of(vm, string)) {
            (Ok(channel), Ok(text)) => (channel, text),
            (Err(completion), _) | (_, Err(completion)) => return completion,
        },
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
    let name = match vm.native_name_operand_bytes_checked(name) {
        Ok(name) => name,
        Err(error) => {
            return completion_from_cmd_error(vm, error);
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
            match vm.native_name_operand_bytes_checked(value) {
                Ok(bytes) => parts.push(bytes),
                Err(refusal) => {
                    return completion_from_cmd_error(vm, refusal);
                }
            }
        }
        Value::from_string_bytes(tcl_syntax::list::concat_bytes(
            parts.iter().map(std::convert::AsRef::as_ref),
        ))
    };
    match vm.prepare_original_expression_value(&source) {
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
    let name = match vm.native_name_operand_bytes_checked(name) {
        Ok(name) => name,
        Err(error) => return completion_from_cmd_error(vm, error),
    };
    match vm.declare_constant_bytes(&name, value.clone(), true) {
        Ok(()) => ok(Value::empty()),
        Err(error) => error,
    }
}

/// Queue the same replacement activation used by the native tailcall opcode.
fn cmd_tailcall(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some(protocol) = vm.actual_native_invocation_dialect().tailcall_protocol() else {
        return vm.refuse_host_command("native tailcall control issuer".into());
    };
    if args.is_empty() && !protocol.schedules_empty() {
        return ok(Value::empty());
    }
    if protocol == tcl_registry::invocation_words::NativeTailcallProtocol::Tcl {
        if let Err(completion) = vm.cancel_original_tailcall() {
            return completion;
        }
        if args.is_empty() {
            return Completion::new(Code::Return, Value::empty(), Value::empty());
        }
        let request = match vm.original_tailcall_request(args) {
            Ok(request) => request,
            Err(completion) => return completion,
        };
        if let Err(completion) = vm.schedule_tailcall(request) {
            return completion;
        }
        return Completion::new(Code::Return, Value::empty(), Value::empty());
    }
    let original_scope =
        match crate::interp::native_operation_currency::NativeOperationScope::enter(vm) {
            Ok(scope) => scope,
            Err(error) => return completion_from_cmd_error(vm, error),
        };
    let request = crate::exec::TailcallReq {
        original_scope: crate::exec::OriginalExecutionOwner::Entered(original_scope),
        namespace: tcl_cmd_core::namespace::current(vm),
        words: args.to_vec(),
        original_list: None,
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
    // C reads the count as a C `int`, so `4294967295` is -1 iterations.
    let count = if args.len() == 2 {
        match tcl_syntax::value::ValueOps::as_int(vm, &args[1])
            .and_then(tcl_syntax::value::narrow_to_c_int)
        {
            Ok(n) => i64::from(n),
            Err(e) => return completion_from_cmd_error(vm, CmdError::from(e)),
        }
    } else {
        1
    };
    let start = vm.host_rc().clock().now_micros();
    let mut i = count;
    while i > 0 {
        match vm.eval_original_script_value(
            &args[0],
            tcl_registry::native_eval_object::EvalObjectPurpose::ControlBody,
            args[0].source_location(),
        ) {
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
        vm,
        ok(Value::list(vec![
            num,
            Value::string("microseconds"),
            Value::string("per"),
            Value::string("iteration"),
        ])),
        tcl_runtime_api::completion_options::ControlOptionPolicy::FRESH_SETTLED,
    )
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

/// `unset ?-nocomplain? ?--? name ...` — remove variables / array elements.
fn cmd_unset(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some(protocol) = vm.native_invocation_dialect().unset_option_protocol() else {
        return vm.refuse_host_command("unset option grammar is unavailable".into());
    };
    let options = match protocol.parse(args.len(), |index| {
        vm.native_name_operand_bytes_checked(&args[index])
            .map_err(|error| completion_from_cmd_error(vm, error))
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

#[cfg(test)]
mod tests;

#[cfg(test)]
#[path = "command/native_frame_context_tests.rs"]
mod native_frame_context_tests;

#[cfg(test)]
mod native_substitution_tests;

#[cfg(test)]
#[path = "command/native_catch_formal_alias_tests.rs"]
mod native_catch_formal_alias_tests;

#[cfg(test)]
#[path = "command/native_formal_alpha_tests.rs"]
mod native_formal_alpha_tests;

#[cfg(test)]
mod native_subst_operand_tests;

#[cfg(test)]
#[path = "command/native_lambda_tests.rs"]
mod native_lambda_tests;

#[cfg(test)]
#[path = "command/native_apply_original_tests.rs"]
mod native_apply_original_tests;

#[cfg(test)]
#[path = "command/native_static_original_tests.rs"]
mod native_static_original_tests;

#[cfg(test)]
mod native_completion_option_tests;

#[cfg(test)]
#[path = "command/native_string_stage_tests.rs"]
mod original_string_stage_tests;

mod native_frame_numeric;

#[cfg(test)]
mod physical_variable_declaration_tests;
