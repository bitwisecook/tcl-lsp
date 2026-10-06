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

//! `TclOO` for the bytecode VM.
//!
//! Objects retain stable tokens and exact byte command names. Methods retain
//! counted byte keys separately from private storage identities and native
//! diagnostic reporting. Original invocation values supply method dispatch,
//! forwarding and argument-usage headers. Namespace and command publication
//! use the selected native name owner and actual holder tokens.
//!
//! Objects and method activations:
//!
//! - Objects and classes are commands ([`Command::Object`](crate::command::Command::Object)),
//!   keyed into [`OoState`]. **Every class is also an object** (an instance of
//!   `::oo::class`): a class FQN appears in both `classes` and `objects`.
//! - A method runs as a proc activation in the object's private namespace
//!   (`::oo::ObjN`), with the declared instance variables linked in and an
//!   [`OoFrame`] pushed so `self`/`my`/`next` can consult the resolved method
//!   chain ([`Vm::oo_run_method`](crate::interp::Vm)).
//! - Method resolution walks a keep-last linearisation (object mixins → the
//!   object → the class MRO), so a diamond defers a shared base until after
//!   everything deriving from it — the same order `next` follows.
//!
//! Declaration objects retain counted byte keys independently of `CString`
//! validation. Constructed command slots remain separate from reporting names.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use std::cell::RefCell;
use std::rc::Rc;

pub(crate) mod native_context;
pub(crate) mod native_method_cache;
use tcl_syntax::mro::{MroError, tcloo_linearise};

use tcl_core_types::NameBytes;
use tcl_registry::commands::tcl::{
    InfoOoEnsembleKind, InfoOoPropertiesOption, TclOoPropertyKind, TclOoPropertyOption,
    info_oo_subcommands, resolve_info_oo_properties_option, resolve_tcloo_property_kind,
    resolve_tcloo_property_option,
};
use tcl_runtime_api::{Code, Completion, NsId, OoId};

use crate::command::{Command, Param, ProcDef, parse_params_value};
use crate::interp::{CommandSidecarKey, Vm, err, ok};
use crate::value::Value;

/// A method (or constructor/destructor) body: a proc-like parameter list plus
/// an exact-object-namespace compiled body, and whether it is exported for public
/// (`$obj m`) dispatch. A class method has no single procedure namespace at
/// definition time, so compilation is deferred until an exact object-private
/// namespace is known. The cache is deliberately one bounded exact entry: an
/// object method is stable, while a shared class method recompiles when calls
/// move between object-private namespaces instead of retaining dead objects.
#[derive(Clone)]
struct Method {
    params: Vec<Param>,
    has_args: bool,
    compiled_body: Option<(String, crate::compiled::CompiledUnit)>,
    body_src: Value,
    /// `None` for an ordinary method; `Some(prefix)` for a `forward` (invoking
    /// the method evaluates `prefix args…`).
    forward: Option<Vec<Value>>,
    /// Native default accessor clientData, shared through reached Method snapshots.
    property: Option<Rc<native_properties::NativePropertyAccessor>>,
    /// Default public visibility (from the name / definition flags). The
    /// per-class/object `exported`/`unexported` override sets take precedence.
    exported: bool,
}

impl Method {
    /// A new native method declaration invokes `DetailsCloner`; reached snapshots
    /// use ordinary Clone and never acquire another clientData object role.
    fn duplicate_native_payload(&self) -> Self {
        let mut duplicate = self.clone();
        if let Some(accessor) = &self.property {
            duplicate.property = Some(Rc::new(native_properties::NativePropertyAccessor {
                original: accessor.original.clone(),
                writable: accessor.writable,
            }));
        }
        duplicate
    }
}

/// A retained declaration object and its exact counted native table key.
#[derive(Clone)]
struct DeclaredVariable {
    original: Value,
    name: NameBytes,
}

/// The class facet of an entity: what its *instances* inherit.
#[derive(Clone, Default)]
struct Class {
    /// Superclass FQNs (canonical). Empty means `::oo::object`.
    supers: Vec<OoId>,
    methods: native_method_cache::MethodTable,
    constructor: native_method_cache::MethodSlot,
    destructor: native_method_cache::MethodSlot,
    /// Declared instance-variable names, auto-linked into method frames.
    variables: Vec<DeclaredVariable>,
    /// TIP 500 private instance variables (`private variable`).
    ///
    /// Auto-linked into the frames of methods this class declares, but to a
    /// *mangled* storage name rather than the bare one, so two classes in one
    /// hierarchy may each declare `X` without sharing a slot. Hidden from the
    /// plain `info class variables`, reported by its `-private` form.
    private_variables: Vec<DeclaredVariable>,
    mixins: Vec<OoId>,
    /// `export`/`unexport` overrides applied to instance methods.
    exported: BTreeSet<String>,
    unexported: BTreeSet<String>,
    /// TIP 558 property slots — the readable/writable property names declared
    /// on this class via `::oo::configuresupport::{readable,writable}properties`
    /// (a `BTreeSet` keeps them sorted and unique, as `info class properties`
    /// reports them).
    readable_properties: BTreeSet<String>,
    writable_properties: BTreeSet<String>,
    /// Set when the class was created by `oo::configurable` (TIP 558): enables
    /// the `property` definition command in its body and the `configure`
    /// instance method on its objects.
    configurable: bool,
}

/// The object facet of an entity: its own (per-instance) state.
#[derive(Clone)]
struct Object {
    /// The original per-object `TclOO` name header, retired on rename/destruction.
    cached_name: Rc<RefCell<Option<Value>>>,
    /// FQN of the object's class (canonical).
    class: OoId,
    /// The namespace holding this object's instance variables (canonical).
    ns: String,
    /// Actual namespace incarnation selected during original object creation.
    namespace_token: Option<NsId>,
    /// Monotonic id (`info object creationid`), stable across rename.
    creation_id: u64,
    /// Per-object methods (`oo::objdefine … method`), incl. class-side methods
    /// (`oo::define C self method`) since a class is an object too.
    methods: native_method_cache::MethodTable,
    variables: Vec<DeclaredVariable>,
    /// TIP 500 per-object private instance variables
    /// (`oo::objdefine … private variable`). See `Class::private_variables`;
    /// here the declaring provider is the object itself.
    private_variables: Vec<DeclaredVariable>,
    mixins: Vec<OoId>,
    exported: BTreeSet<String>,
    unexported: BTreeSet<String>,
    /// TIP 558 per-object property slots
    /// (`::oo::configuresupport::obj{readable,writable}properties`).
    readable_properties: BTreeSet<String>,
    writable_properties: BTreeSet<String>,
    /// Set once the destructor chain has started (re-entrancy guard).
    destroyed: bool,
}

/// One resolved step in a method-call chain: the providing entity plus which
/// facet (object-side per-instance method, or class-side instance method).
#[derive(Clone)]
struct Step {
    method_owner: Option<native_method_cache::MethodOwner>,
    provider: OoId,
    is_object: bool,
}

/// What a running method's `self`/`my`/`next` operate on: the object, the
/// resolved chain of providers for the invoked method, the current index into
/// it, the public method name, and whether it was an external (`$obj m`) call.
pub(crate) struct OoFrame {
    name_owner: Rc<RefCell<Option<Value>>>,
    /// Issued identity of the actual method variable activation, never an ancestor.
    pub(crate) activation: Option<u64>,
    object: OoId,
    chain: native_method_cache::ReachedMethodChain,
    index: usize,
    /// The invoked method name (empty for a constructor, `<destructor>` for a
    /// destructor).
    method: String,
    external: bool,
    /// The object as the caller named it (`obj`, `::oo::Obj7`, …), reused by
    /// `my`/`next` to build a `wrong # args` usage that echoes the call site.
    invoked: Value,
    /// The `wrong # args` leading words for the running step (`obj m`, `C new`).
    usage_prefix: Vec<Value>,
}

/// The actual object facet selected by a definition worker's native client data.
#[derive(Clone, Copy)]
enum DefTarget {
    Class(OoId),
    Object(OoId),
}

/// The whole object system's runtime state, held by the [`Vm`].
#[derive(Default)]
pub(crate) struct OoState {
    native_methods: native_method_cache::NativeMethodWorld,
    property_foundation_epoch: u64,
    property_my_name: Option<Value>,
    property_members: BTreeMap<(OoId, bool, bool), BTreeMap<String, Value>>,
    property_caches: BTreeMap<(OoId, bool), native_properties::NativePropertyCache>,

    classes: BTreeMap<OoId, Class>,
    objects: BTreeMap<OoId, Object>,
    /// Current Tcl-facing command spelling, indexed one-way by stable token.
    names: BTreeMap<OoId, tcl_core_types::NameBytes>,
    /// Private command-table key, also one-way from the stable token. It is
    /// opaque and is never rendered or parsed as a Tcl name.
    command_keys: BTreeMap<OoId, CommandSidecarKey>,
    object_root: Option<OoId>,
    class_root: Option<OoId>,
    configurable_root: Option<OoId>,
    /// Monotonic counter for anonymous `::oo::ObjN` names and creation ids.
    counter: u64,
    /// Counted method keys index opaque private String storage identities.
    method_keys: BTreeMap<tcl_core_types::NameBytes, String>,
    method_names: BTreeMap<String, tcl_core_types::NameBytes>,
    method_counter: u64,
    /// Active method invocations (innermost last) — drives `self`/`my`/`next`.
    pub(crate) call_stack: Vec<OoFrame>,
    /// Non-zero while a `private { … }` definition block is being evaluated,
    /// so `variable` and `method` inside it declare privately (TIP 500).
    pub(crate) private_depth: usize,
    /// The Foundation's original constructor-to-define invocation head.
    define_name: Option<Value>,
    /// Active definition targets with issued variable activations and caller namespaces.
    def_stack: Vec<(DefTarget, u64, NsId, u64)>,
}

/// The per-flow OO execution stacks — the subset of [`OoState`] that a coroutine
/// owns and that is swapped in/out on suspend/resume (the class/object
/// registries stay shared). Mirrors `runtime/rust/src/cmd_oo.rs`'s `OoExec`.
#[derive(Default)]
pub(crate) struct OoExec {
    call_stack: Vec<OoFrame>,
    def_stack: Vec<(DefTarget, u64, NsId, u64)>,
    private_depth: usize,
}

impl OoState {
    fn allocate(&mut self, name: impl Into<tcl_core_types::NameBytes>) -> OoId {
        let id = OoId(self.counter);
        self.counter += 1;
        self.names.insert(id, name.into());
        id
    }

    fn intern_method_key(&mut self, original: tcl_core_types::NameBytes) -> String {
        if let Some(key) = self.method_keys.get(&original) {
            return key.clone();
        }
        let plain = original
            .try_utf8()
            .ok()
            .filter(|key| !self.method_names.contains_key(*key));
        let key = if let Some(plain) = plain {
            plain.to_owned()
        } else {
            loop {
                let candidate = format!("\0oo-method:{}", self.method_counter);
                self.method_counter = self
                    .method_counter
                    .checked_add(1)
                    .expect("method token counter exhausted");
                if !self.method_names.contains_key(&candidate) {
                    break candidate;
                }
            }
        };
        self.method_keys.insert(original.clone(), key.clone());
        self.method_names.insert(key.clone(), original);
        key
    }

    fn method_name_bytes<'a>(&'a self, key: &'a str) -> &'a [u8] {
        self.method_names.get(key).map_or_else(
            || {
                assert!(
                    !key.starts_with('\0'),
                    "opaque method token retains original bytes"
                );
                key.as_bytes()
            },
            tcl_core_types::NameBytes::as_bytes,
        )
    }

    /// Exchange this interpreter's live OO execution stacks with `e` — one half
    /// of a coroutine context switch. The registries are shared and untouched.
    pub(crate) fn swap_exec(&mut self, e: &mut OoExec) {
        std::mem::swap(&mut self.call_stack, &mut e.call_stack);
        std::mem::swap(&mut self.def_stack, &mut e.def_stack);
        std::mem::swap(&mut self.private_depth, &mut e.private_depth);
    }
}

/// Render a canonical FQN in display (`::`-qualified) form.
fn display(canonical: &str) -> String {
    format!("::{canonical}")
}

/// Resolve a class *reference* (`superclass`, `mixin`, `nextto`, `info … isa`)
/// to its canonical class key, honouring command-style scoping: an absolute
/// `::a::b` name as-is, else the current namespace then the global one. Unlike
/// [`Vm::qualify_name`] this does not blindly prepend the current namespace —
/// inside a method the current namespace is the object's private one, where the
/// class does not live.
fn resolve_object_value(vm: &mut Vm, name: &Value) -> Result<Option<OoId>, Completion<Value>> {
    let selected = vm
        .lookup_original_command_at(native_context::outer_namespace(vm), name)
        .map_err(|error| vm.refuse_host_command(error.to_string()))?;
    Ok(match selected {
        Some((_, Command::Object(id))) if vm.oo.objects.contains_key(&id) => Some(id),
        _ => None,
    })
}

fn resolve_class_value(vm: &mut Vm, name: &Value) -> Result<Option<OoId>, Completion<Value>> {
    Ok(resolve_object_value(vm, name)?.filter(|id| vm.oo.classes.contains_key(id)))
}

fn display_oo_bytes(vm: &Vm, id: OoId) -> Vec<u8> {
    let mut result = b"::".to_vec();
    result.extend_from_slice(
        vm.oo
            .names
            .get(&id)
            .expect("live object retains exact command reporting name")
            .as_bytes(),
    );
    result
}

fn object_message(vm: &Vm, id: OoId, before: &[u8], after: &[u8]) -> Vec<u8> {
    let mut message = before.to_vec();
    message.extend_from_slice(&display_oo_bytes(vm, id));
    message.extend_from_slice(after);
    message
}

fn native_method_key(vm: &mut Vm, original: &Value) -> Result<String, Completion<Value>> {
    let bytes = vm
        .native_name_operand_bytes(original)
        .map_err(|error| vm.refuse_host_command(error.to_string()))?;
    let policy = vm.name_policy_protocol().ok_or_else(|| {
        vm.refuse_host_command("native TclOO method protocol unavailable".to_owned())
    })?;
    let projected = policy
        .recipe()
        .oo_method_input(&bytes)
        .map_err(|error| vm.refuse_host_command(error.to_string()))?;
    Ok(vm
        .oo
        .intern_method_key(tcl_core_types::NameBytes::from(projected.selected())))
}

fn method_report_bytes(vm: &Vm, key: &str) -> Vec<u8> {
    let recipe = vm
        .name_policy_protocol()
        .expect("TclOO method tables require authenticated name policy")
        .recipe();
    tcl_syntax::naming::report_native_name_bytes(
        recipe,
        tcl_syntax::naming::NativeNameReportPurpose::OoMethodEnumeration,
        vm.oo.method_name_bytes(key),
    )
    .expect("TclOO method reporting requires its audited C recipe")
    .to_vec()
}

fn method_list_value(vm: &Vm, names: impl IntoIterator<Item = String>) -> Value {
    let mut names: Vec<_> = names
        .into_iter()
        .map(|key| method_report_bytes(vm, &key))
        .collect();
    names.sort();
    Value::list(
        names
            .into_iter()
            .map(Value::from_native_string_bytes)
            .collect(),
    )
}

pub(crate) fn register(vm: &mut Vm) {
    vm.oo.define_name = Some(Value::string("::oo::define"));
    vm.declare_namespace("::oo::define");
    vm.declare_namespace("::oo::objdefine");
    vm.register_stock_builtin("oo::define", cmd_oo_define);
    vm.register_stock_builtin("oo::objdefine", cmd_oo_objdefine);
    native_context::register_definition_workers(vm);
    // Native method namespaces resolve the genuine qualified helper slots.
    vm.declare_namespace("::oo::Helpers");
    vm.register_stock_builtin("::oo::Helpers::self", cmd_self);
    vm.register_stock_builtin("::oo::Helpers::next", cmd_next);
    vm.register_stock_builtin("::oo::Helpers::nextto", cmd_nextto);
    vm.register_stock_builtin("my", cmd_my);
    // TIP 558 property slots — the low-level readable/writable property setters
    // used inside `oo::define` / `oo::objdefine` bodies. `(obj, writable)`:
    // `obj` targets the per-object slot (`obj*properties`), else the class slot.
    vm.register_stock_builtin("::oo::configuresupport::readableproperties", |v, a| {
        property_slot(v, a, false, false)
    });
    vm.register_stock_builtin("::oo::configuresupport::writableproperties", |v, a| {
        property_slot(v, a, false, true)
    });
    vm.register_stock_builtin("::oo::configuresupport::objreadableproperties", |v, a| {
        property_slot(v, a, true, false)
    });
    vm.register_stock_builtin("::oo::configuresupport::objwritableproperties", |v, a| {
        property_slot(v, a, true, true)
    });
    // TIP 558 configurable layer: the `property` definition command (valid only
    // inside a configurable class's `oo::define`/`oo::objdefine` body) and the
    // `private` prefix that forwards to it (and the other def commands).
    vm.register_stock_builtin("property", cmd_property);
    vm.register_stock_builtin("private", cmd_private);
    bootstrap(vm);
}

fn declared_variable_error(error: tcl_syntax::naming::NativeOoVariableError) -> Completion<Value> {
    let code = Value::list(
        error
            .error_code
            .into_iter()
            .map(Value::from_native_string_bytes)
            .collect(),
    );
    Completion::new(
        Code::Error,
        Value::from_native_string_bytes(error.message),
        Value::list(vec![Value::string("-errorcode"), code]),
    )
}

/// A TIP 558 property slot (`::oo::configuresupport::{obj,}{readable,writable}properties`).
///
/// Callable only inside an `oo::define`/`oo::objdefine` body; applies a slot
/// operation to the target's readable or writable property set. Supported ops
/// are `-set` (replace, also the default for bare names), `-append`, `-remove`,
/// and `-clear`; every op keeps the set sorted and duplicate-free.
fn property_slot(vm: &mut Vm, args: &[Value], obj: bool, writable: bool) -> Completion<Value> {
    let Some((is_class, target)) = active_target(vm) else {
        return err_code(
            "this command may only be called from within the context of an ::oo::define or ::oo::objdefine command",
            &["TCL", "OO", "MONKEY_BUSINESS"],
        );
    };
    let (op, names): (String, &[Value]) = match args.split_first() {
        Some((first, rest)) if first.to_str().starts_with('-') => {
            (first.to_str().to_string(), rest)
        }
        _ => ("-set".to_owned(), args),
    };
    if vm
        .actual_native_invocation_dialect()
        .native_property_lookup_protocol()
        .is_some()
    {
        let class = is_class && !obj;
        let names =
            match native_properties::set_property_slot(vm, target, class, writable, &op, names) {
                Ok(names) => names,
                Err(error) => return crate::command::completion_from_cmd_error(vm, error.into()),
            };
        if class {
            if let Some(entry) = vm.oo.classes.get_mut(&target) {
                *slot_of_class(entry, writable) = names;
            }
        } else if let Some(entry) = vm.oo.objects.get_mut(&target) {
            *slot_of_obj(entry, writable) = names;
        }
        return ok(Value::empty());
    }
    let new_names: Vec<String> = names.iter().map(|v| v.to_str().to_string()).collect();

    // Resolve the target set: the per-object slot writes the object facet, the
    // class slot writes the class facet (a class is also an object).
    let set = if obj || !is_class {
        vm.oo
            .objects
            .get_mut(&target)
            .map(|o| slot_of_obj(o, writable))
    } else {
        vm.oo
            .classes
            .get_mut(&target)
            .map(|c| slot_of_class(c, writable))
    };
    let Some(set) = set else {
        return err(object_message(
            vm,
            target,
            b"",
            b" does not refer to an object",
        ));
    };
    match op.as_str() {
        "-set" => {
            set.clear();
            set.extend(new_names);
        }
        "-append" => set.extend(new_names),
        "-remove" => {
            for n in &new_names {
                set.remove(n);
            }
        }
        "-clear" => set.clear(),
        other => return err(format!("unsupported slot operation \"{other}\"")),
    }
    ok(Value::empty())
}

fn slot_of_class(c: &mut Class, writable: bool) -> &mut BTreeSet<String> {
    if writable {
        &mut c.writable_properties
    } else {
        &mut c.readable_properties
    }
}

fn slot_of_obj(o: &mut Object, writable: bool) -> &mut BTreeSet<String> {
    if writable {
        &mut o.writable_properties
    } else {
        &mut o.readable_properties
    }
}

// TIP 558 configurable layer: the `property` definition command, `private`
// prefix, and the `configure` instance method.

/// Build an `ERROR` completion carrying an explicit `-errorcode` list (the
/// `Tcl_GetIndexFromObj` / `TclOO` tags the `ooProp-4.x` tests check).
fn err_code(message: impl Into<String>, code: &[&str]) -> Completion<Value> {
    let ec = Value::list(
        code.iter()
            .map(|w| Value::string((*w).to_string()))
            .collect(),
    );
    let opts = Value::list(vec![Value::string("-errorcode"), ec]);
    Completion::new_error_metadata(Code::Error, Value::string(message.into()), opts)
}

/// The readable (`writable == false`) or writable property names visible to
/// `configure` on `obj_key`: the union of the object's own slots and every
/// class slot along its MRO (`info object properties -all`), sorted & unique.
fn configure_props(vm: &Vm, obj_key: OoId, writable: bool) -> Vec<String> {
    let mut names: BTreeSet<String> = BTreeSet::new();
    for step in vm.oo.object_precedence(obj_key) {
        if step.is_object {
            if let Some(o) = vm.oo.objects.get(&step.provider) {
                names.extend(slot_ref_obj(o, writable).iter().cloned());
            }
        } else if let Some(c) = vm.oo.classes.get(&step.provider) {
            names.extend(slot_ref_class(c, writable).iter().cloned());
        }
    }
    names.into_iter().collect()
}

/// `property name ?-kind K? ?-get body? ?-set body? …` inside a configurable
/// class's (or its object's) definition body. Each spec is a property name
/// followed by its options; options bind to the preceding name.
fn cmd_property(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    // Only a configurable definition target exposes `property`; anywhere else it
    // is simply not a command (C keeps it in the configurable define namespace).
    let Some((is_class, target)) = active_target(vm) else {
        return err("invalid command name \"property\"");
    };
    let allowed = if is_class {
        // The class must itself have been built by `oo::configurable` (a plain
        // subclass of a configurable class does *not* inherit the command).
        vm.oo.classes.get(&target).is_some_and(|c| c.configurable)
    } else {
        vm.oo.object_is_configurable(target)
    };
    if !allowed {
        return err("invalid command name \"property\"");
    }
    if vm
        .actual_native_invocation_dialect()
        .native_property_lookup_protocol()
        .is_some()
    {
        return native_properties::define_original_properties(vm, is_class, target, args);
    }
    define_properties(vm, is_class, target, args)
}

/// `private <def-command> …` — for `property`, the accessors are already
/// private; for the other definition commands, force unexported visibility.
fn cmd_private(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some((verb, rest)) = args.split_first() else {
        return ok(Value::empty());
    };
    // TIP 500 block form: a lone argument is a definition *script*, not a
    // command name, so `private {variable X}` declares X privately. Raise the
    // private depth for its evaluation; `variable` and `method` inside consult
    // it. The depth is restored on every path, including an error unwinding
    // out of the script.
    if rest.is_empty() {
        vm.oo.private_depth += 1;
        let result = match vm.eval_source(&verb.to_str()) {
            Ok(c) if c.code == Code::Return => ok(c.result),
            Ok(c) => c,
            Err(e) => crate::command::completion_from_tcl_error(vm, e),
        };
        vm.oo.private_depth = vm.oo.private_depth.saturating_sub(1);
        return result;
    }
    match verb.to_str().as_ref() {
        "property" => cmd_property(vm, rest),
        "method" => {
            let mut v = vec![Value::string("-private")];
            v.extend_from_slice(rest);
            def_body_cmd(vm, "method", &v)
        }
        // `private variable` is not the ordinary declaration with a visibility
        // flag: it declares into a separate set whose storage is mangled per
        // declaring provider (TIP 500), so it cannot route through the generic
        // definition-command switch the way the method forms do.
        "variable" => def_private_variable(vm, rest),
        other => def_body_cmd(vm, other, rest),
    }
}

/// Parse and apply a `property` argument sequence to `target`.
fn define_properties(
    vm: &mut Vm,
    is_class: bool,
    target: OoId,
    args: &[Value],
) -> Completion<Value> {
    let mut i = 0;
    while i < args.len() {
        let name = args[i].to_str().to_string();
        if let Err(e) = validate_prop_name(&name) {
            return e;
        }
        i += 1;
        let mut kind = TclOoPropertyKind::ReadWrite;
        let mut getter: Option<Value> = None;
        let mut setter: Option<Value> = None;
        // Options bind to the preceding name; a bare word (property names cannot
        // begin with `-`) starts the next spec.
        while i < args.len() && args[i].to_str().starts_with('-') {
            let opt = args[i].to_str().to_string();
            i += 1;
            match resolve_tcloo_property_option(opt.as_bytes()) {
                Ok(TclOoPropertyOption::Kind) => {
                    let Some(v) = args.get(i) else {
                        return err_code(
                            "missing kind value to go with -kind option",
                            &["TCL", "WRONGARGS"],
                        );
                    };
                    kind = match resolve_tcloo_property_kind(v.to_str().as_bytes()) {
                        Ok(k) => k,
                        Err(message) => {
                            return err_code(
                                String::from_utf8_lossy(&message).into_owned(),
                                &["TCL", "LOOKUP", "INDEX", "kind", &v.to_str()],
                            );
                        }
                    };
                    i += 1;
                }
                Ok(TclOoPropertyOption::Get) => {
                    let Some(v) = args.get(i) else {
                        return err_code(
                            "missing body to go with -get option",
                            &["TCL", "WRONGARGS"],
                        );
                    };
                    getter = Some(v.clone());
                    i += 1;
                }
                Ok(TclOoPropertyOption::Set) => {
                    let Some(v) = args.get(i) else {
                        return err_code(
                            "missing body to go with -set option",
                            &["TCL", "WRONGARGS"],
                        );
                    };
                    setter = Some(v.clone());
                    i += 1;
                }
                Err(message) => {
                    return err_code(
                        String::from_utf8_lossy(&message).into_owned(),
                        &["TCL", "LOOKUP", "INDEX", "option", &opt],
                    );
                }
            }
        }
        if let Err(e) = apply_property(vm, is_class, target, &name, kind, getter, setter) {
            return e;
        }
    }
    ok(Value::empty())
}

/// Validate a property name (must be a plain word, no leading `-`, no namespace
/// separators or parentheses). Errors are tagged `TCL OO PROPERTY_FORMAT`.
fn validate_prop_name(name: &str) -> Result<(), Completion<Value>> {
    let fail = |why: &str| {
        Err(err_code(
            format!("bad property name \"{name}\": {why}"),
            &["TCL", "OO", "PROPERTY_FORMAT"],
        ))
    };
    if name.starts_with('-') {
        return fail("must not begin with -");
    }
    if name.is_empty() || name.chars().any(char::is_whitespace) {
        return fail("must be a simple word");
    }
    if name.contains("::") {
        return fail("must not contain namespace separators");
    }
    if name.contains(['(', ')']) {
        return fail("must not contain parentheses");
    }
    Ok(())
}

/// Install property `name` on `target` (fully replacing any prior definition of
/// the same name): recompute its slot membership from `kind`, and (re)install or
/// clear its custom `<ReadProp-name>` / `<WriteProp-name>` accessor methods.
fn apply_property(
    vm: &mut Vm,
    is_class: bool,
    target: OoId,
    name: &str,
    kind: TclOoPropertyKind,
    getter: Option<Value>,
    setter: Option<Value>,
) -> Result<(), Completion<Value>> {
    apply_property_bytes(
        vm,
        is_class,
        target,
        (name.as_bytes(), None),
        kind,
        getter,
        setter,
    )
}

fn apply_property_bytes(
    vm: &mut Vm,
    is_class: bool,
    target: OoId,
    property_name: (&[u8], Option<&Value>),
    kind: TclOoPropertyKind,
    getter: Option<Value>,
    setter: Option<Value>,
) -> Result<(), Completion<Value>> {
    let (name, original) = property_name;
    let (dashed, read_method, write_method) = property_accessor_keys(vm, name);
    let (readable, writable) = match kind {
        TclOoPropertyKind::Readable => (true, false),
        TclOoPropertyKind::Writable => (false, true),
        TclOoPropertyKind::ReadWrite => (true, true),
    };
    let get_m = match getter {
        Some(body) => Some(build_method(vm, &Value::empty(), &body, false)?),
        None if readable => {
            original.map(|original| native_properties::default_method(original, false))
        }
        None => None,
    };
    let set_m = match setter {
        Some(body) => Some(build_method(vm, &Value::string("value"), &body, false)?),
        None if writable => {
            original.map(|original| native_properties::default_method(original, true))
        }
        None => None,
    };
    if readable {
        vm.native_property_method_created(target, is_class);
    }
    if writable {
        vm.native_property_method_created(target, is_class);
    }
    if let Err(error) =
        native_properties::register_property(vm, target, is_class, name, readable, writable)
    {
        return Err(crate::command::completion_from_cmd_error(vm, error.into()));
    }
    let apply = |methods: &mut native_method_cache::MethodTable,
                 rset: &mut BTreeSet<String>,
                 wset: &mut BTreeSet<String>| {
        rset.remove(&dashed);
        wset.remove(&dashed);
        if readable {
            rset.insert(dashed.clone());
        }
        if writable {
            wset.insert(dashed.clone());
        }
        match get_m {
            Some(m) => {
                methods.insert(read_method.clone(), m);
            }
            None => {
                methods.remove(&read_method);
            }
        }
        match set_m {
            Some(m) => {
                methods.insert(write_method.clone(), m);
            }
            None => {
                methods.remove(&write_method);
            }
        }
    };
    if is_class {
        if let Some(c) = vm.oo.classes.get_mut(&target) {
            let Class {
                methods,
                readable_properties,
                writable_properties,
                ..
            } = c;
            apply(methods, readable_properties, writable_properties);
        }
    } else if let Some(o) = vm.oo.objects.get_mut(&target) {
        let Object {
            methods,
            readable_properties,
            writable_properties,
            ..
        } = o;
        apply(methods, readable_properties, writable_properties);
    }
    Ok(())
}

/// The `configure` instance method of a configurable object. Forms:
/// `configure` (list every readable property as `-name value`), `configure
/// -name` (read one), `configure -name value …` (write pairs).
fn property_accessor_keys(vm: &mut Vm, name: &[u8]) -> (String, String, String) {
    let mut dashed_bytes = b"-".to_vec();
    dashed_bytes.extend_from_slice(tcl_core_types::c_string_extent(name));
    let dashed = vm
        .oo
        .intern_method_key(tcl_core_types::NameBytes::from(dashed_bytes.clone()));
    let mut read_bytes = b"<ReadProp".to_vec();
    read_bytes.extend_from_slice(&dashed_bytes);
    read_bytes.push(b'>');
    let mut write_bytes = b"<WriteProp".to_vec();
    write_bytes.extend_from_slice(&dashed_bytes);
    write_bytes.push(b'>');
    let read_method = vm
        .oo
        .intern_method_key(tcl_core_types::NameBytes::from(read_bytes));
    let write_method = vm
        .oo
        .intern_method_key(tcl_core_types::NameBytes::from(write_bytes));
    (dashed, read_method, write_method)
}

fn configure_method(vm: &mut Vm, obj_key: OoId, args: &[Value]) -> Completion<Value> {
    if vm
        .actual_native_invocation_dialect()
        .native_property_lookup_protocol()
        .is_some()
    {
        return native_properties::configure_native(vm, obj_key, args);
    }
    match args.len() {
        0 => {
            let mut out = Vec::new();
            for dashed in configure_props(vm, obj_key, false) {
                let val = match read_property(vm, obj_key, &dashed) {
                    Ok(v) => v,
                    Err(e) => return e,
                };
                out.push(Value::string(dashed));
                out.push(val);
            }
            ok(Value::list(out))
        }
        1 => {
            let dashed = match gate_property(vm, obj_key, &args[0], false, &mut None, false) {
                Ok(name) => name,
                Err(e) => return e,
            };
            match read_property(vm, obj_key, &dashed) {
                Ok(v) => ok(v),
                Err(e) => e,
            }
        }
        n if n % 2 == 0 => {
            let mut i = 0;
            let mut property_table = None;
            while i < n {
                let dashed =
                    match gate_property(vm, obj_key, &args[i], true, &mut property_table, true) {
                        Ok(name) => name,
                        Err(e) => return e,
                    };
                if let Err(e) = write_property(vm, obj_key, &dashed, args[i + 1].clone()) {
                    return e;
                }
                i += 2;
            }
            ok(Value::empty())
        }
        _ => {
            let original = vm
                .invoked_name_value()
                .unwrap_or_else(|| Value::from_native_string_bytes(display_oo_bytes(vm, obj_key)));
            let mut usage =
                match vm.native_argument_usage_header(&[original, Value::string("configure")]) {
                    Ok(header) => header,
                    Err(completion) => return completion,
                };
            usage.extend_from_slice(b" ?-option value ...?");
            crate::command::native_wrong_args_bytes(vm, &usage)
        }
    }
}

/// Resolve `dashed` to a property valid for the requested access, producing
/// the `bad`/`ambiguous property`/`is write only`/`is read only` errors
/// otherwise.
///
/// C's `oo::configuresupport` reaches this through
/// `tcl::prefix match -message property`, so an abbreviation resolves
/// (`-ye` → `-yellow`) and a word prefixing several — `""` and a lone `-`
/// included — is `ambiguous property`. The resolved *canonical* name is what
/// the accessor then reads or writes.
fn gate_property(
    vm: &mut Vm,
    obj_key: OoId,
    original: &Value,
    for_write: bool,
    _table: &mut Option<Value>,
    _retain_table: bool,
) -> Result<String, Completion<Value>> {
    let dashed = original.to_str();
    let wanted = configure_props(vm, obj_key, for_write);
    let miss = match tcl_cmd_core::prefix::OptionTable::abbreviating("property", &wanted)
        .index_of(dashed.as_bytes())
    {
        Ok(index) => return Ok(wanted[index].clone()),
        Err(message) => message,
    };
    // Present but in the opposite slot → a directionality error.
    let other = configure_props(vm, obj_key, !for_write);
    if tcl_cmd_core::prefix::OptionTable::abbreviating("property", &other)
        .index_of(dashed.as_bytes())
        .is_ok()
    {
        let why = if for_write { "read only" } else { "write only" };
        return Err(err(format!("property \"{dashed}\" is {why}")));
    }
    Err(err_code(
        String::from_utf8_lossy(&miss).into_owned(),
        &["TCL", "LOOKUP", "INDEX", "property", dashed.as_ref()],
    ))
}

/// Read property `dashed` from `obj_key`: invoke its `<ReadProp-name>` accessor
/// if one is defined, else read the backing instance variable directly.
fn read_property(vm: &mut Vm, obj_key: OoId, dashed: &str) -> Result<Value, Completion<Value>> {
    let name = dashed.strip_prefix('-').unwrap_or(dashed).to_string();
    let accessor = format!("<ReadProp-{name}>");
    if method_resolves(vm, obj_key, &accessor) {
        let invoked = Value::from_native_string_bytes(display_oo_bytes(vm, obj_key));
        let comp = oo_invoke(vm, obj_key, &accessor, &[], false, &invoked);
        return accessor_result(comp, dashed, false);
    }
    // Default getter: read the instance variable in the object's namespace.
    let ns = vm.oo.objects.get(&obj_key).map(|o| o.ns.clone());
    let Some(ns) = ns else {
        return Err(err(object_message(
            vm,
            obj_key,
            b"invalid command name \"",
            b"\"",
        )));
    };
    match vm.get_var(&format!("::{ns}::{name}")) {
        Some(v) => Ok(v),
        None => Err(err(format!("can't read \"{name}\": no such variable"))),
    }
}

/// Write `value` to property `dashed` on `obj_key`: invoke its `<WriteProp-name>`
/// accessor if defined, else set the backing instance variable directly.
fn write_property(
    vm: &mut Vm,
    obj_key: OoId,
    dashed: &str,
    value: Value,
) -> Result<(), Completion<Value>> {
    let name = dashed.strip_prefix('-').unwrap_or(dashed).to_string();
    let accessor = format!("<WriteProp-{name}>");
    if method_resolves(vm, obj_key, &accessor) {
        let invoked = Value::from_native_string_bytes(display_oo_bytes(vm, obj_key));
        let comp = oo_invoke(vm, obj_key, &accessor, &[value], false, &invoked);
        accessor_result(comp, dashed, true)?;
        return Ok(());
    }
    let ns = vm.oo.objects.get(&obj_key).map(|o| o.ns.clone());
    let Some(ns) = ns else {
        return Err(err(object_message(
            vm,
            obj_key,
            b"invalid command name \"",
            b"\"",
        )));
    };
    vm.set_var(&format!("::{ns}::{name}"), value)
}

/// Map a custom accessor's completion into a value (or propagate its error).
/// A `break`/`continue` from the accessor body becomes the `TclOO` diagnostic
/// `property {getter,setter} for -name did a {break,continue}`; other non-ok
/// codes (errors, `return -level N`) propagate unchanged.
fn accessor_result(
    comp: Completion<Value>,
    dashed: &str,
    is_set: bool,
) -> Result<Value, Completion<Value>> {
    match comp.code {
        Code::Ok => Ok(comp.result),
        Code::Break | Code::Continue => {
            let role = if is_set { "setter" } else { "getter" };
            let verb = if comp.code == Code::Break {
                "break"
            } else {
                "continue"
            };
            Err(err(format!("property {role} for {dashed} did a {verb}")))
        }
        // A residual `return -level N` (N>1 in the body) or an error propagates
        // unchanged, so `configure` transparently forwards it to its caller.
        _ => Err(comp),
    }
}

/// Whether `method` resolves on `obj_key` (any provider along its precedence
/// chain), without invoking it — used to detect custom property accessors.
fn method_resolves(vm: &Vm, obj_key: OoId, method: &str) -> bool {
    vm.oo
        .object_precedence(obj_key)
        .iter()
        .any(|s| vm.oo.step_method(s, method).is_some())
}

/// Seed the two root classes `::oo::object` and `::oo::class`. Each is a class,
/// an object (instance of `::oo::class`), and a command.
fn bootstrap(vm: &mut Vm) {
    // TclOO is compiled into the VM, so — like C Tcl's core at startup — provide
    // the `tcl::oo` package and seed the `::oo` version variables. Without the
    // package the `package require tcl::oo` at the top of every `oo*` test file
    // (and any real TclOO script) aborts with `can't find package tcl::oo`.
    vm.provide_package("tcl::oo", "1.3.1");
    vm.declare_namespace("::oo");
    let _ = vm.set_var("::oo::version", Value::string("1.3"));
    let _ = vm.set_var("::oo::patchlevel", Value::string("1.3.1"));

    let object_root = vm.oo.allocate("oo::object".to_string());
    let class_root = vm.oo.allocate("oo::class".to_string());
    vm.oo.property_foundation_epoch = 1;
    if vm
        .actual_native_invocation_dialect()
        .native_property_lookup_protocol()
        .is_some()
    {
        vm.oo.property_my_name = Some(Value::new_native_string_bytes(b"my".as_slice()));
    }
    vm.oo.object_root = Some(object_root);
    vm.oo.class_root = Some(class_root);
    for (root, root_id) in [("oo::object", object_root), ("oo::class", class_root)] {
        vm.oo.classes.insert(root_id, Class::default());
        vm.oo.objects.insert(
            root_id,
            Object {
                cached_name: Rc::new(RefCell::new(None)),
                class: class_root,
                ns: root.to_string(),
                namespace_token: None,
                creation_id: root_id.0,
                methods: native_method_cache::MethodTable::default(),
                variables: Vec::new(),
                private_variables: Vec::new(),
                mixins: Vec::new(),
                exported: BTreeSet::new(),
                unexported: BTreeSet::new(),
                readable_properties: BTreeSet::new(),
                writable_properties: BTreeSet::new(),
                destroyed: false,
            },
        );
        let namespace_token = native_context::install_object_helpers(vm, root);
        vm.oo.objects.get_mut(&root_id).unwrap().namespace_token = Some(namespace_token);
        let command_key = vm.register_command(root, Command::Object(root_id));
        vm.oo
            .command_keys
            .insert(root_id, CommandSidecarKey::visible(command_key));
        // Engine-installed, not script-created: the registry dates these
        // (TCL86_PLUS) and the availability gate must honour that.
        vm.declare_registry_object_root(root);
    }
    // `oo::object`'s only super is nothing (it is the root); `oo::class` extends
    // `oo::object`.
    if let Some(c) = vm.oo.classes.get_mut(&class_root) {
        c.supers = vec![object_root];
    }

    // `oo::configurable` (TIP 558): a metaclass (subclass of `oo::class`, so
    // `oo::configurable create C {…}` builds a *class*) whose `configurable`
    // flag propagates to the classes it creates — enabling their `property`
    // definition command and their instances' `configure` method.
    let configurable_root = vm.oo.allocate("oo::configurable".to_string());
    vm.oo.configurable_root = Some(configurable_root);
    vm.oo.classes.insert(
        configurable_root,
        Class {
            supers: vec![class_root],
            ..Class::default()
        },
    );
    vm.oo.objects.insert(
        configurable_root,
        Object {
            cached_name: Rc::new(RefCell::new(None)),
            class: class_root,
            ns: "oo::configurable".to_string(),
            namespace_token: None,
            creation_id: configurable_root.0,
            methods: native_method_cache::MethodTable::default(),
            variables: Vec::new(),
            private_variables: Vec::new(),
            mixins: Vec::new(),
            exported: BTreeSet::new(),
            unexported: BTreeSet::new(),
            readable_properties: BTreeSet::new(),
            writable_properties: BTreeSet::new(),
            destroyed: false,
        },
    );
    vm.declare_namespace("oo::configurable");
    let namespace_token = native_context::install_object_helpers(vm, "oo::configurable");
    vm.oo
        .objects
        .get_mut(&configurable_root)
        .unwrap()
        .namespace_token = Some(namespace_token);
    let command_key = vm.register_command("oo::configurable", Command::Object(configurable_root));
    vm.oo
        .command_keys
        .insert(configurable_root, CommandSidecarKey::visible(command_key));
    // TIP 558 is Tcl 9.0: real tclsh 8.6.16 has no `oo::configurable`.
    vm.declare_registry_object_root("oo::configurable");
}

// Dispatch entry (called from the engine for a `Command::Object`).

/// Dispatch `argv` against the object/class `key` (canonical). `invoked` is the
/// word the command was called under (for the `wrong # args` usage).
pub(crate) fn oo_dispatch(
    vm: &mut Vm,
    key: OoId,
    invoked: &Value,
    argv: &[Value],
) -> Completion<Value> {
    if argv.is_empty() {
        let mut usage = match vm.native_argument_usage_header(std::slice::from_ref(invoked)) {
            Ok(usage) => usage,
            Err(completion) => return completion,
        };
        usage.extend_from_slice(b" method ?arg ...?");
        return crate::command::native_wrong_args_bytes(vm, &usage);
    }
    let method = match native_method_key(vm, &argv[0]) {
        Ok(key) => key,
        Err(completion) => return completion,
    };
    let rest = &argv[1..];
    // A class responds to the factory built-ins first (create/new); anything
    // else is a class-side method (or `destroy`).
    if vm.oo.classes.contains_key(&key) {
        match method.as_str() {
            "create" => return factory(vm, key, invoked, &argv[0], false, rest),
            "new" if Some(key) != vm.oo.class_root => {
                return factory(vm, key, invoked, &argv[0], true, rest);
            }
            _ => {}
        }
    }
    oo_invoke_value(vm, key, &method, &argv[0], rest, true, invoked)
}

// Object / class creation.

/// The `create objectName ?arg…?` / `new ?arg…?` factory built-ins of a class.
/// `class_invoked` is the class as the caller named it (for the constructor's
/// `wrong # args` usage).
fn factory(
    vm: &mut Vm,
    class_key: OoId,
    class_invoked: &Value,
    original_factory: &Value,
    anon: bool,
    args: &[Value],
) -> Completion<Value> {
    // Instantiating a metaclass (a class whose MRO includes `::oo::class`)
    // builds a *class*, and its trailing arg is a definition script, not
    // constructor arguments.
    let is_meta = vm
        .oo
        .class_linear_of(class_key)
        .iter()
        .any(|s| Some(s.provider) == vm.oo.class_root);
    if is_meta {
        let (name, body) = if anon {
            (fresh_obj_name(vm).into_bytes(), args.first())
        } else {
            let Some((n, rest)) = args.split_first() else {
                let mut usage =
                    match vm.native_argument_usage_header(std::slice::from_ref(class_invoked)) {
                        Ok(header) => header,
                        Err(completion) => return completion,
                    };
                usage.extend_from_slice(b" create className ?definitionScript?");
                return crate::command::native_wrong_args_bytes(vm, &usage);
            };
            let n = match vm.native_name_operand_bytes(n) {
                Ok(bytes) => bytes,
                Err(error) => return vm.refuse_host_command(error.to_string()),
            };
            if n.is_empty() {
                return err("object name must not be empty");
            }
            (n.to_vec(), rest.first())
        };
        return make_class_bytes(vm, &name, body, class_key);
    }

    let (obj_key, obj_ns, ctor_args, ctor_usage, ctor_identity) = if anon {
        // An anonymous object's command *is* its instance namespace (`oo::ObjN`).
        let n = fresh_obj_name(vm);
        let identity =
            original_call_words([class_invoked, original_factory].into_iter().chain(args));
        (
            n.as_bytes().to_vec(),
            n,
            args,
            vec![class_invoked.clone(), original_factory.clone()],
            identity,
        )
    } else {
        let Some((name, rest)) = args.split_first() else {
            let mut usage =
                match vm.native_argument_usage_header(std::slice::from_ref(class_invoked)) {
                    Ok(header) => header,
                    Err(completion) => return completion,
                };
            usage.extend_from_slice(b" create objectName ?arg ...?");
            return crate::command::native_wrong_args_bytes(vm, &usage);
        };
        let name_bytes = match vm.native_name_operand_bytes(name) {
            Ok(bytes) => bytes,
            Err(error) => return vm.refuse_host_command(error.to_string()),
        };
        if name_bytes.is_empty() {
            return err("object name must not be empty");
        }
        // A named object's instance namespace is still a fresh `oo::ObjN`,
        // decoupled from its command name (C's `oo::Obj` counter).
        let identity = original_call_words(
            [class_invoked, original_factory, name]
                .into_iter()
                .chain(rest),
        );
        (
            name_bytes.to_vec(),
            fresh_obj_name(vm),
            rest,
            vec![
                class_invoked.clone(),
                original_factory.clone(),
                name.clone(),
            ],
            identity,
        )
    };
    oo_new(
        vm,
        class_key,
        &obj_key,
        &obj_ns,
        ctor_args,
        &ctor_usage,
        ctor_identity,
    )
}

/// A fresh `oo::ObjN` name (canonical) for an anonymous object.
fn fresh_obj_name(vm: &mut Vm) -> String {
    let n = vm.oo.counter;
    vm.oo.counter += 1;
    format!("oo::Obj{n}")
}

/// Create object `obj_key` as an instance of `class_key`, run its constructor
/// chain, and register its command. On a constructor error the half-built
/// object is torn down and the error re-raised.
fn oo_new(
    vm: &mut Vm,
    class_key: OoId,
    obj_key: &[u8],
    obj_ns: &str,
    ctor_args: &[Value],
    ctor_usage: &[Value],
    ctor_identity: Vec<Value>,
) -> Completion<Value> {
    let slot = match vm.native_object_publication_slot(obj_key) {
        Ok(selected) => selected,
        Err(error) => return vm.refuse_host_command(error.to_string()),
    };
    match vm.command_at_exact_slot_checked(&slot, true) {
        Ok(Some(_)) => {
            let mut message = b"can't create object \"".to_vec();
            message.extend_from_slice(tcl_core_types::c_string_extent(obj_key));
            message.extend_from_slice(b"\": command already exists with that name");
            return err(message);
        }
        Ok(None) => {}
        Err(error) => return vm.refuse_host_command(error.to_string()),
    }
    let object_display = tcl_core_types::NameBytes::from(vm.command_slot_display_bytes(&slot));
    let object_id = vm.oo.allocate(object_display);
    let ns = obj_ns.to_string();
    vm.declare_namespace(&format!("::{ns}"));
    let namespace_token = native_context::install_object_helpers(vm, &ns);
    vm.oo.objects.insert(
        object_id,
        Object {
            cached_name: Rc::new(RefCell::new(None)),
            class: class_key,
            ns: ns.clone(),
            namespace_token: Some(namespace_token),
            creation_id: object_id.0,
            methods: native_method_cache::MethodTable::default(),
            variables: Vec::new(),
            private_variables: Vec::new(),
            mixins: Vec::new(),
            exported: BTreeSet::new(),
            unexported: BTreeSet::new(),
            readable_properties: BTreeSet::new(),
            writable_properties: BTreeSet::new(),
            destroyed: false,
        },
    );
    // A class instantiated as an object is itself a class only when it is a
    // metaclass (its MRO includes `::oo::class`); v1 handles the direct
    // `oo::class create` case in `def_body`'s class creation, not here.
    let command_key = vm.register_command_in_slot(slot, Command::Object(object_id));
    vm.oo
        .command_keys
        .insert(object_id, CommandSidecarKey::visible(command_key));

    // Constructor chain: classes in the linearisation with a constructor,
    // most-derived first.
    let chain: Vec<Step> = vm
        .oo
        .class_linear_of(class_key)
        .into_iter()
        .filter(|s| {
            vm.oo
                .classes
                .get(&s.provider)
                .is_some_and(|c| c.constructor.is_some())
        })
        .collect();
    let chain: native_method_cache::ReachedMethodChain =
        native_method_cache::retain_owners(&vm.oo, chain, "").into();
    if !chain.is_empty() {
        let res = run_step(
            vm,
            object_id,
            &chain,
            0,
            String::new(),
            ctor_args,
            false,
            Value::from_native_string_bytes(display_oo_bytes(vm, object_id)),
            ctor_usage.to_vec(),
            ctor_identity,
        );
        drop(chain);
        if !res.code.is_ok() && res.code != Code::Return {
            // Tear the half-built object down and re-raise.
            teardown(vm, object_id);
            return res;
        }
    }
    ok(native_context::object_name(vm, object_id))
}

// Method resolution + execution.

impl OoState {
    /// Build the `(supers, mixins)` maps `tcloo_linearise` consumes from the
    /// live class registry, injecting `oo::object` as the implicit root of any
    /// class with no declared superclass (mirroring `TclOO`, where every class
    /// ultimately derives from `oo::object`).
    fn linearise_maps(&self) -> (HashMap<OoId, Vec<OoId>>, HashMap<OoId, Vec<OoId>>) {
        let mut supers = HashMap::with_capacity(self.classes.len());
        let mut mixins = HashMap::with_capacity(self.classes.len());
        for (name, c) in &self.classes {
            let mut sup = c.supers.clone();
            if sup.is_empty()
                && Some(*name) != self.object_root
                && let Some(root) = self.object_root
            {
                sup.push(root);
            }
            supers.insert(*name, sup);
            mixins.insert(*name, c.mixins.clone());
        }
        (supers, mixins)
    }

    /// The class-side linearisation, delegated to the compiler's faithful
    /// `TclOO` algorithm ([`tcloo_linearise`]: the two-pass mixin/super split with
    /// late-placement dedup from `tclOOCall.c`, correct for nested mixins and
    /// diamonds where a naive per-class DFS is not). Used for constructor /
    /// destructor chains and `info class properties -all`.
    fn class_linear_of(&self, class_key: OoId) -> Vec<Step> {
        let (supers, mixins) = self.linearise_maps();
        class_steps(tcloo_linearise(&class_key, &supers, &mixins), class_key)
    }

    /// The full method-search precedence for an object: its per-object mixins
    /// (each linearised as a class), then the object's own methods, then its
    /// class MRO — keep-last deduped so a shared class defers to its
    /// most-derived position.
    fn object_precedence(&self, obj_key: OoId) -> Vec<Step> {
        let Some(obj) = self.objects.get(&obj_key) else {
            return Vec::new();
        };
        let (supers, mixins) = self.linearise_maps();
        let mut out: Vec<Step> = Vec::new();
        for mx in &obj.mixins {
            out.extend(class_steps(tcloo_linearise(mx, &supers, &mixins), *mx));
        }
        out.push(Step {
            method_owner: None,
            provider: obj_key,
            is_object: true,
        });
        out.extend(class_steps(
            tcloo_linearise(&obj.class, &supers, &mixins),
            obj.class,
        ));
        keep_last(&out)
    }

    /// The `Method` a step provides for `name`, if any.
    fn step_method(&self, step: &Step, name: &str) -> Option<Method> {
        if step.is_object {
            self.objects.get(&step.provider)?.methods.get(name)
        } else {
            self.classes.get(&step.provider)?.methods.get(name)
        }
    }

    /// Whether `name` is exported for external dispatch at `step` (the
    /// per-provider `export`/`unexport` overrides beat the method's default).
    fn step_exported(&self, step: &Step, name: &str) -> bool {
        let (exported, unexported, dflt) = if step.is_object {
            let o = self.objects.get(&step.provider);
            (
                o.is_some_and(|o| o.exported.contains(name)),
                o.is_some_and(|o| o.unexported.contains(name)),
                o.and_then(|o| o.methods.get(name)).map(|m| m.exported),
            )
        } else {
            let c = self.classes.get(&step.provider);
            (
                c.is_some_and(|c| c.exported.contains(name)),
                c.is_some_and(|c| c.unexported.contains(name)),
                c.and_then(|c| c.methods.get(name)).map(|m| m.exported),
            )
        };
        if unexported {
            return false;
        }
        exported || dflt.unwrap_or(false)
    }

    /// Names callable on `obj_key` (for the unknown-method error), honouring
    /// export state. `internal` includes unexported names.
    fn visible_methods(&self, obj_key: OoId, internal: bool) -> BTreeSet<String> {
        let mut names = BTreeSet::new();
        for step in self.object_precedence(obj_key) {
            let provided: Vec<String> = if step.is_object {
                self.objects
                    .get(&step.provider)
                    .map(|o| o.methods.keys().cloned().collect())
                    .unwrap_or_default()
            } else {
                self.classes
                    .get(&step.provider)
                    .map(|c| c.methods.keys().cloned().collect())
                    .unwrap_or_default()
            };
            for name in provided {
                if internal || self.step_exported(&step, &name) {
                    names.insert(name);
                }
            }
        }
        // `destroy` is always a public built-in; a class also offers the
        // factory built-ins (`new` is unexported on the `::oo::class` root).
        names.insert("destroy".to_string());
        // A configurable object also exposes the public `configure` method.
        if self.object_is_configurable(obj_key) {
            names.insert("configure".to_string());
        }
        if self.classes.contains_key(&obj_key) {
            names.insert("create".to_string());
            if Some(obj_key) != self.class_root {
                names.insert("new".to_string());
            }
        }
        names
    }
}

/// Map a [`tcloo_linearise`] result into class-facet [`Step`]s, degrading to the
/// class itself when linearisation fails (a superclass cycle involving it, or a
/// pathological depth `tcloo_linearise` refuses to expand).
fn class_steps(chain: Result<Vec<OoId>, MroError>, fallback: OoId) -> Vec<Step> {
    match chain {
        Ok(classes) => classes
            .into_iter()
            .map(|c| Step {
                method_owner: None,
                provider: c,
                is_object: false,
            })
            .collect(),
        Err(_) => vec![Step {
            method_owner: None,
            provider: fallback,
            is_object: false,
        }],
    }
}

/// Keep-last dedup of a step list on `(provider, is_object)`: each entry
/// survives only at its last position (deferring shared bases past their
/// descendants), preserving order otherwise.
fn keep_last(steps: &[Step]) -> Vec<Step> {
    let mut out: Vec<Step> = Vec::new();
    for (i, s) in steps.iter().enumerate() {
        let later = steps[i + 1..]
            .iter()
            .any(|t| t.provider == s.provider && t.is_object == s.is_object);
        if !later {
            out.push(s.clone());
        }
    }
    out
}

/// Invoke `method` on object `obj_key`. `external` is a public `$obj m` call
/// (export-enforced); `false` is a `my m` internal call.
fn oo_invoke(
    vm: &mut Vm,
    obj_key: OoId,
    method: &str,
    args: &[Value],
    external: bool,
    invoked: &Value,
) -> Completion<Value> {
    oo_invoke_value(
        vm,
        obj_key,
        method,
        &Value::string(method),
        args,
        external,
        invoked,
    )
}

fn original_call_words<'a>(words: impl IntoIterator<Item = &'a Value>) -> Vec<Value> {
    words
        .into_iter()
        .map(|word| word.native_lifetime_lease().into_value())
        .collect()
}

fn oo_invoke_value(
    vm: &mut Vm,
    obj_key: OoId,
    method: &str,
    original_method: &Value,
    args: &[Value],
    external: bool,
    invoked: &Value,
) -> Completion<Value> {
    oo_invoke_value_with_head(
        vm,
        obj_key,
        method,
        original_method,
        args,
        external,
        (invoked, None),
    )
}

fn oo_invoke_value_with_head(
    vm: &mut Vm,
    obj_key: OoId,
    method: &str,
    original_method: &Value,
    args: &[Value],
    external: bool,
    invocation_heads: (&Value, Option<&Value>),
) -> Completion<Value> {
    let (invoked, private_head) = invocation_heads;
    if !vm.oo.objects.contains_key(&obj_key) {
        return err(object_message(
            vm,
            obj_key,
            b"invalid command name \"",
            b"\"",
        ));
    }
    // Object built-in methods (available internally, or when exported).
    if matches!(method, "variable" | "varname" | "eval")
        && (!external || is_exported_builtin(vm, obj_key, method))
    {
        return builtin_method(
            vm,
            obj_key,
            method,
            args,
            invoked,
            original_method,
            external,
        );
    }

    let chain = if let Some(chain) =
        native_method_cache::lookup(vm, obj_key, method, original_method, external)
    {
        chain
    } else {
        let precedence = vm.oo.object_precedence(obj_key);
        let chain: Vec<Step> = precedence
            .into_iter()
            .filter(|s| vm.oo.step_method(s, method).is_some())
            .collect();

        if chain.is_empty() {
            // `destroy` with no user override is the built-in destructor path.
            if method == "destroy" {
                return oo_destroy(vm, obj_key);
            }
            // `configure` with no user override is TIP 558's property accessor.
            if method == "configure" && vm.oo.object_is_configurable(obj_key) {
                return configure_method(vm, obj_key, args);
            }
            return unknown_method(vm, obj_key, method, external);
        }
        // External calls require the most-derived definer to be exported.
        if external && !vm.oo.step_exported(&chain[0], method) {
            return unknown_method(vm, obj_key, method, external);
        }
        let chain = native_method_cache::retain_owners(&vm.oo, chain, method);
        match native_method_cache::remember(
            vm,
            obj_key,
            method,
            original_method,
            external,
            Rc::new(chain),
        ) {
            Ok(chain) => chain,
            Err(error) => return crate::command::completion_from_cmd_error(vm, error.into()),
        }
    };
    let usage = vec![invoked.clone(), original_method.clone()];
    let original_head = if let Some(head) = private_head {
        head.native_lifetime_lease().into_value()
    } else if external {
        invoked.native_lifetime_lease().into_value()
    } else {
        let Some(head) = vm.invoked_name_value() else {
            return vm.refuse_host_command("original TclOO invocation head is unavailable".into());
        };
        head.native_lifetime_lease().into_value()
    };
    let call_identity =
        original_call_words([&original_head, original_method].into_iter().chain(args));
    run_step(
        vm,
        obj_key,
        &chain,
        0,
        method.to_string(),
        args,
        external,
        invoked.clone(),
        usage,
        call_identity,
    )
}

/// Whether an object built-in (`variable`/`varname`/`eval`) is exported — they
/// are unexported by default, so only reachable externally after `self export`.
fn is_exported_builtin(vm: &Vm, obj_key: OoId, method: &str) -> bool {
    vm.oo
        .objects
        .get(&obj_key)
        .is_some_and(|o| o.exported.contains(method))
}

/// The `unknown method "X": must be …` error (or "no visible methods").
fn unknown_method(vm: &Vm, obj_key: OoId, method: &str, external: bool) -> Completion<Value> {
    let names = vm.oo.visible_methods(obj_key, !external);
    if names.is_empty() {
        return err(object_message(
            vm,
            obj_key,
            b"object \"",
            b"\" has no visible methods",
        ));
    }
    let mut names: Vec<_> = names
        .iter()
        .map(|name| method_report_bytes(vm, name))
        .collect();
    names.sort();
    let mut message = b"unknown method \"".to_vec();
    message.extend_from_slice(&method_report_bytes(vm, method));
    message.extend_from_slice(b"\": must be ");
    message.extend_from_slice(&tcl_cmd_core::prefix::tcloo_choice_list_bytes(&names));
    err(message)
}

/// Execute step `index` of `chain` on `obj_key`, for the invoked `method`
/// (empty = constructor, `<destructor>` = destructor). Pushes an [`OoFrame`] so
/// `self`/`next` see the chain.
///
/// Native-stack safety net — see `interp::OO_DISPATCH_DEPTH_LIMIT`'s doc
/// comment. This is the single choke point every method-body
/// execution funnels through — `$obj method` (via [`oo_dispatch`] →
/// [`oo_invoke`]), `my method` (via `cmd_my` → [`oo_invoke`]), and
/// `next`/`nextto` (directly) — so guarding here covers every recursive
/// path, not just the `Command::Object` engine dispatch.
#[allow(clippy::too_many_arguments)] // A method activation genuinely carries this context.
fn run_step(
    vm: &mut Vm,
    obj_key: OoId,
    chain: &native_method_cache::ReachedMethodChain,
    index: usize,
    method: String,
    args: &[Value],
    external: bool,
    invoked: Value,
    usage_prefix: Vec<Value>,
    call_identity: Vec<Value>,
) -> Completion<Value> {
    if let Err(c) = vm.enter_oo_dispatch() {
        return c;
    }
    let result = run_step_inner(
        vm,
        obj_key,
        chain,
        index,
        method,
        args,
        external,
        invoked,
        usage_prefix,
        call_identity,
    );
    vm.exit_oo_dispatch();
    result
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)] // A method activation genuinely carries this context and persistence seam.
fn run_step_inner(
    vm: &mut Vm,
    obj_key: OoId,
    chain: &native_method_cache::ReachedMethodChain,
    index: usize,
    method: String,
    args: &[Value],
    external: bool,
    invoked: Value,
    usage_prefix: Vec<Value>,
    call_identity: Vec<Value>,
) -> Completion<Value> {
    let step = chain[index].clone();
    // Resolve the method body for this step.
    let m = if let Some(owner) = &step.method_owner {
        Some(owner.borrow().clone())
    } else if method.is_empty() {
        vm.oo
            .classes
            .get(&step.provider)
            .and_then(|c| c.constructor.get())
    } else if method == "<destructor>" {
        vm.oo
            .classes
            .get(&step.provider)
            .and_then(|c| c.destructor.get())
    } else {
        vm.oo.step_method(&step, &method)
    };
    let Some(m) = m else {
        return err("no such method");
    };

    if let Some(property) = &m.property {
        let frame = OoFrame {
            name_owner: Rc::clone(&vm.oo.objects[&obj_key].cached_name),
            activation: None,
            object: obj_key,
            chain: chain.clone(),
            index,
            method: method.clone(),
            external,
            invoked,
            usage_prefix,
        };
        vm.oo.call_stack.push(frame);
        let result = native_properties::invoke_default(vm, obj_key, property, args, &call_identity);
        vm.oo.call_stack.pop();
        return result;
    }

    // A forward: evaluate `prefix args…` with the OBJECT's namespace current,
    // so the target resolves object-ns → global exactly as C TclOO does —
    // never in the dispatch site's namespace (tclsh 8.6.16 / 9.0.4-pinned:
    // `$obj fw` called from a namespace that has its own same-named proc
    // still dispatches the global target).
    if let Some(prefix) = &m.forward {
        let mut words: Vec<Value> = prefix.clone();
        words.extend_from_slice(args);
        let ns = vm.oo.objects.get(&obj_key).map(|o| o.ns.clone());
        let frame = OoFrame {
            name_owner: Rc::clone(&vm.oo.objects[&obj_key].cached_name),
            activation: None,
            object: obj_key,
            chain: chain.clone(),
            index,
            method: method.clone(),
            external,
            invoked: invoked.clone(),
            usage_prefix: usage_prefix.clone(),
        };
        vm.oo.call_stack.push(frame);
        vm.push_ns(ns.unwrap_or_default());
        let context = vm.current_ns_id();
        let res = vm.invoke_command_value_at(
            context,
            &words[0],
            &words[1..],
            &[],
            tcl_registry::command_lookup::CommandLookupOrigin::Ordinary,
        );
        vm.pop_ns();
        vm.oo.call_stack.pop();
        return res;
    }

    let obj_ns = vm
        .oo
        .objects
        .get(&obj_key)
        .map(|o| o.ns.clone())
        .unwrap_or_default();
    // Instance variables to auto-link belong to the method's declaring
    // provider. An object-defined method sees the object's declarations; a
    // class-defined method sees that class's declarations.
    let mut decl: Vec<DeclaredVariable> = Vec::new();
    let mut private_decl: Vec<DeclaredVariable> = Vec::new();
    if step.is_object {
        if let Some(o) = vm.oo.objects.get(&step.provider) {
            decl.extend(o.variables.iter().cloned());
            private_decl.extend(o.private_variables.iter().cloned());
        }
    } else if let Some(c) = vm.oo.classes.get(&step.provider) {
        decl.extend(c.variables.iter().cloned());
        private_decl.extend(c.private_variables.iter().cloned());
    }
    // A class is an object too, so the declaring provider's creation id is in
    // `objects` either way; it names the private storage slot below.
    let provider_creation_id = vm
        .oo
        .objects
        .get(&step.provider)
        .map_or(0, |o| o.creation_id);
    // TclOO's automatic instance-variable resolver yields to a method formal of
    // the same name. Explicit `my variable x` still reaches the ordinary link
    // installer and reports the collision.
    let shadowed = |name: &NameBytes| m.params.iter().any(|param| param.name == *name);
    let ns_id = vm.definition_namespace_token(&obj_ns);
    let mut link_vars: Vec<(NameBytes, NameBytes)> = Vec::new();
    for variable in private_decl {
        if shadowed(&variable.name) || link_vars.iter().any(|(local, _)| *local == variable.name) {
            continue;
        }
        let storage = private_storage_name_bytes(provider_creation_id, variable.name.as_bytes());
        link_vars.push((variable.name, storage));
    }
    for variable in decl {
        if shadowed(&variable.name) || link_vars.iter().any(|(local, _)| *local == variable.name) {
            continue;
        }
        link_vars.push((variable.name.clone(), variable.name));
    }

    let body = m
        .compiled_body
        .as_ref()
        .filter(|(namespace, _)| namespace == &obj_ns)
        .map(|(_, body)| body.clone());
    let proc = ProcDef {
        native_resources: Rc::default(),
        name: format!("{obj_ns}::{method}"),
        command_ns_id: ns_id,
        simple_name: tcl_core_types::NameBytes::from(vm.oo.method_name_bytes(&method)),
        namespace: vm.namespace_path_for_token(ns_id),
        ns_id,
        params: m.params.clone(),
        parameter_grammar: tcl_dialect::ParameterGrammar::Tcl,
        has_args: m.has_args,
        native_jim_namespace: None,
        native_parameters: None,
        native_header: tcl_dialect::NativeProcedureHeaderCompilation::Absent,
        statics: None,
        body,
        body_src: m.body_src.clone(),
        usage_name: Some(usage_prefix.clone()),
        call_identity: Some(call_identity),
    };
    // Refresh through the shared proc owner, then persist the replacement on
    // its defining facet.  Recompiling on every method call after a profile
    // switch is both wasteful and makes the class cache lie about its body.
    let proc = match vm.ensure_proc_traced(crate::command::NativeProcedureCommand::new(proc)) {
        Ok(proc) => proc,
        Err(error) => return crate::command::completion_from_tcl_error(vm, error),
    };
    let cache_is_current = m.compiled_body.as_ref().is_some_and(|(namespace, cached)| {
        namespace == &obj_ns
            && Rc::ptr_eq(&proc.body.asm, &cached.asm)
            && proc.body.command_epoch == cached.command_epoch
            && proc.body.profile_generation == cached.profile_generation
            && proc.body.compiler == cached.compiler
    });
    if !cache_is_current {
        let mut refreshed = m.clone();
        refreshed.compiled_body = Some((obj_ns, proc.body.clone()));
        if let Some(owner) = &step.method_owner {
            let mut current = owner.borrow_mut();
            if current.body_src.native_object_identity() == m.body_src.native_object_identity()
                && current.forward.is_none()
                && current.property.is_none()
            {
                current.compiled_body = refreshed.compiled_body;
            }
        } else if method.is_empty() {
            if let Some(class) = vm.oo.classes.get_mut(&step.provider) {
                class.constructor.install(refreshed);
            }
        } else if method == "<destructor>" {
            if let Some(class) = vm.oo.classes.get_mut(&step.provider) {
                class.destructor.install(refreshed);
            }
        } else if step.is_object {
            if let Some(object) = vm.oo.objects.get_mut(&step.provider) {
                object.methods.insert(method.clone(), refreshed);
            }
        } else if let Some(class) = vm.oo.classes.get_mut(&step.provider) {
            class.methods.insert(method.clone(), refreshed);
        }
    }
    let frame = OoFrame {
        name_owner: Rc::clone(&vm.oo.objects[&obj_key].cached_name),
        activation: None,
        object: obj_key,
        chain: chain.clone(),
        index,
        method,
        external,
        invoked,
        usage_prefix,
    };
    vm.oo_run_method(proc, args, &link_vars, frame)
}

/// The object built-in methods `variable name…`, `varname name`, `eval script`.
fn builtin_method(
    vm: &mut Vm,
    obj_key: OoId,
    method: &str,
    args: &[Value],
    invoked: &Value,
    original_method: &Value,
    external: bool,
) -> Completion<Value> {
    let ns = match vm.oo.objects.get(&obj_key) {
        Some(o) => o.ns.clone(),
        None => {
            return err(object_message(
                vm,
                obj_key,
                b"invalid command name \"",
                b"\"",
            ));
        }
    };
    match method {
        "variable" => {
            for a in args {
                let name = a.to_str();
                if name.contains("::") {
                    return err(format!(
                        "variable name \"{name}\" illegal: must not contain namespace separator"
                    ));
                }
                let storage = frame_storage_name(vm, &ns, &name);
                if let Err(error) = vm.add_link(&name, 0, &storage) {
                    return crate::command::upvar_link_error(error, &storage, &name);
                }
            }
            ok(Value::empty())
        }
        "varname" => {
            let Some(a) = args.first() else {
                return crate::command::native_wrong_args(vm, "my varname varName");
            };
            ok(Value::string(display(&frame_storage_name(
                vm,
                &ns,
                &a.to_str(),
            ))))
        }
        "eval" => {
            let script = args
                .iter()
                .map(|v| v.to_str().to_string())
                .collect::<Vec<_>>()
                .join(" ");
            let namespace = vm.definition_namespace_token(&ns);
            let invocation =
                original_call_words([invoked, original_method].into_iter().chain(args.iter()));
            vm.push_ns_eval_token_frame(namespace, invocation);
            let frame = OoFrame {
                name_owner: Rc::clone(&vm.oo.objects[&obj_key].cached_name),
                activation: Some(vm.native_oo_variable_activation()),
                object: obj_key,
                chain: native_method_cache::ReachedMethodChain::from(vec![Step {
                    method_owner: None,
                    provider: vm.oo.object_root.expect("native object eval provider"),
                    is_object: false,
                }]),
                index: 0,
                method: "eval".into(),
                external,
                invoked: invoked.clone(),
                usage_prefix: original_call_words([invoked, original_method]),
            };
            vm.oo.call_stack.push(frame);
            let res = match vm.eval_source(&script) {
                Ok(c) => c,
                Err(e) => crate::command::completion_from_tcl_error(vm, e),
            };
            vm.oo.call_stack.pop();
            vm.pop_call_frame();
            vm.pop_ns();
            res
        }
        _ => unreachable!("builtin_method called with {method}"),
    }
}

// self / my / next / nextto

fn cmd_my(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some(fr) = native_context::current_method(vm) else {
        return err("invalid command name \"my\"");
    };
    let obj = fr.object;
    let invoked = fr.invoked.clone();
    let Some((method, rest)) = args.split_first() else {
        return crate::command::native_wrong_args(vm, "my methodName ?arg ...?");
    };
    let key = match native_method_key(vm, method) {
        Ok(key) => key,
        Err(completion) => return completion,
    };
    oo_invoke_value(vm, obj, &key, method, rest, false, &invoked)
}

/// The object a method frame reports as `self` — its command name (C's
/// `TclOOObjectName`).
fn frame_object_name(vm: &Vm, fr: &OoFrame) -> Value {
    native_context::object_name(vm, fr.object)
}

/// The running method's object as `self` / `self object` reports it — `None`
/// outside a method. The `tclooSelf` opcode's core.
pub(crate) fn current_object_name(vm: &Vm) -> Option<Value> {
    native_context::current_method(vm).map(|frame| frame_object_name(vm, frame))
}

/// Whether a method invocation is running — the context `self`/`next`/`nextto`
/// (and their opcodes) need.
pub(crate) fn in_method(vm: &Vm) -> bool {
    native_context::current_method(vm).is_some()
}

fn cmd_self(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some(fr) = native_context::current_method(vm) else {
        let Some(original) = vm.invoked_name_value() else {
            return vm.refuse_host_command("original self invocation head is unavailable".into());
        };
        let Some(protocol) = vm
            .actual_native_invocation_dialect()
            .native_string_protocol()
        else {
            return vm
                .refuse_host_command("native self invocation string getter is unavailable".into());
        };
        let head = match original.native_string_bytes(protocol) {
            Ok(head) => head,
            Err(error) => {
                return crate::command::completion_from_cmd_error(
                    vm,
                    tcl_syntax::raw_string::NativeStringAccessError::Unavailable(error).into(),
                );
            }
        };
        return native_context::helper_context_error(vm, &head);
    };
    let self_name = frame_object_name(vm, fr);
    let obj = fr.object;
    if args.len() > 1 {
        return crate::command::native_wrong_args(vm, "self subcommand");
    }
    let sub = match args.first() {
        Some(original) => match vm.native_static_option_index(
            original,
            tcl_registry::native_tcloo_compilation::SELF_SUBCOMMANDS,
            false,
            "subcommand",
        ) {
            Ok(index) => Some(tcl_registry::native_tcloo_compilation::SELF_SUBCOMMANDS[index]),
            Err(error) => return crate::command::completion_from_cmd_error(vm, error),
        },
        None => None,
    };
    match sub {
        None | Some("object") => ok(self_name),
        Some("namespace") => {
            let ns = vm
                .oo
                .objects
                .get(&obj)
                .map(|o| o.ns.clone())
                .unwrap_or_default();
            ok(Value::string(display(&ns)))
        }
        Some("class") => {
            // The class that declares the running method (its step provider).
            let step = &fr.chain[fr.index];
            if step.is_object {
                return err("method not defined by a class");
            }
            ok(Value::from_native_string_bytes(display_oo_bytes(
                vm,
                step.provider,
            )))
        }
        Some("method") => ok(Value::from_native_string_bytes(
            vm.oo.method_name_bytes(&fr.method).to_vec(),
        )),
        Some("call") => {
            let steps: Vec<Value> = fr
                .chain
                .iter()
                .map(|s| Value::from_native_string_bytes(display_oo_bytes(vm, s.provider)))
                .collect();
            ok(Value::list(vec![
                Value::list(steps),
                Value::int(i64::try_from(fr.index).unwrap_or(0)),
            ]))
        }
        Some("target") => {
            let step = &fr.chain[fr.index];
            ok(Value::list(vec![
                Value::from_native_string_bytes(display_oo_bytes(vm, step.provider)),
                Value::from_native_string_bytes(vm.oo.method_name_bytes(&fr.method).to_vec()),
            ]))
        }
        Some(other) => err(format!("unsupported self subcommand \"{other}\"")),
    }
}

/// `next ?arg…?` — run the next implementation on the method chain. Also the
/// `tclooNext` opcode's core (which checks the method context itself, so it can
/// report C's `INST_TCLOO_NEXT` message instead of the command's
/// `invalid command name`).
pub(crate) fn cmd_next(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some(head) = vm.invoked_name_value() else {
        return vm.refuse_host_command("original next invocation head is unavailable".into());
    };
    cmd_next_original(vm, &head, args)
}

pub(crate) fn cmd_next_original(vm: &mut Vm, head: &Value, args: &[Value]) -> Completion<Value> {
    let Some(fr) = native_context::current_method(vm) else {
        return native_context::helper_context_error(vm, b"next");
    };
    let obj = fr.object;
    let chain = fr.chain.clone();
    let index = fr.index;
    let method = fr.method.clone();
    let external = fr.external;
    let invoked = fr.invoked.clone();
    let usage = fr.usage_prefix.clone();
    if index + 1 >= chain.len() {
        // Past the end there is no further implementation of this kind.
        let kind = if method.is_empty() {
            "constructor"
        } else if method == "<destructor>" {
            "destructor"
        } else {
            "method"
        };
        return err(format!("no next {kind} implementation"));
    }
    let call_identity = original_call_words(std::iter::once(head).chain(args));
    run_step(
        vm,
        obj,
        &chain,
        index + 1,
        method,
        args,
        external,
        invoked,
        usage,
        call_identity,
    )
}

/// `nextto class ?arg…?` — resume the method chain at `class`. Also the
/// `tclooNextClass` opcode's core (see [`cmd_next`]).
pub(crate) fn cmd_nextto(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some(head) = vm.invoked_name_value() else {
        return vm.refuse_host_command("original nextto invocation head is unavailable".into());
    };
    cmd_nextto_original(vm, &head, args)
}

pub(crate) fn cmd_nextto_original(vm: &mut Vm, head: &Value, args: &[Value]) -> Completion<Value> {
    let Some(fr) = native_context::current_method(vm) else {
        return native_context::helper_context_error(vm, b"nextto");
    };
    let Some((cls, rest)) = args.split_first() else {
        return crate::command::native_wrong_args(vm, "nextto class ?arg...?");
    };
    let obj = fr.object;
    let chain = fr.chain.clone();
    let index = fr.index;
    let method = fr.method.clone();
    let external = fr.external;
    let invoked = fr.invoked.clone();
    let usage = fr.usage_prefix.clone();
    let target = match resolve_class_value(vm, cls) {
        Ok(target) => target,
        Err(completion) => return completion,
    };
    // Find the target class ahead of the current step.
    if let Some(pos) = chain
        .iter()
        .enumerate()
        .position(|(i, s)| i > index && !s.is_object && Some(s.provider) == target)
    {
        let call_identity = original_call_words(std::iter::once(head).chain(args));
        return run_step(
            vm,
            obj,
            &chain,
            pos,
            method,
            rest,
            external,
            invoked,
            usage,
            call_identity,
        );
    }
    err(format!(
        "method has no non-filter implementation by \"{}\"",
        cls.to_str()
    ))
}

// destroy / destructor

/// Run the destructor chain, then tear the object (and, if a class, its
/// descendants) down.
fn oo_destroy(vm: &mut Vm, obj_key: OoId) -> Completion<Value> {
    match vm.oo.objects.get_mut(&obj_key) {
        Some(o) if o.destroyed => return ok(Value::empty()),
        Some(o) => o.destroyed = true,
        None => return ok(Value::empty()),
    }
    // Destroying a class cascades to everything derived from it: its direct
    // instances and its direct subclasses (each recursion in turn reaches their
    // instances / subclasses), matching TclOO. The `destroyed` guard above makes
    // this safe against cycles.
    if vm.oo.classes.contains_key(&obj_key) {
        let derived: Vec<OoId> = vm
            .oo
            .objects
            .iter()
            .filter(|(k, o)| {
                **k != obj_key
                    && (o.class == obj_key
                        || vm
                            .oo
                            .classes
                            .get(*k)
                            .is_some_and(|c| c.supers.contains(&obj_key)))
            })
            .map(|(k, _)| *k)
            .collect();
        for d in derived {
            let _ = oo_destroy(vm, d);
        }
    }
    let class = vm.oo.objects.get(&obj_key).map(|o| o.class);
    let mut result = ok(Value::empty());
    if let Some(class) = class {
        let chain: Vec<Step> = vm
            .oo
            .class_linear_of(class)
            .into_iter()
            .filter(|s| {
                vm.oo
                    .classes
                    .get(&s.provider)
                    .is_some_and(|c| c.destructor.is_some())
            })
            .collect();
        let chain: native_method_cache::ReachedMethodChain =
            native_method_cache::retain_owners(&vm.oo, chain, "<destructor>").into();
        if !chain.is_empty() {
            let d = Value::from_native_string_bytes(display_oo_bytes(vm, obj_key));
            let call_identity = Vec::new();
            let res = run_step(
                vm,
                obj_key,
                &chain,
                0,
                "<destructor>".to_string(),
                &[],
                false,
                d.clone(),
                vec![d, Value::string("destroy")],
                call_identity,
            );
            drop(chain);
            if !res.code.is_ok() && res.code != Code::Return {
                result = res;
            }
        }
    }
    teardown(vm, obj_key);
    result
}

/// Remove an object's command and records (no destructor run).
fn teardown(vm: &mut Vm, obj_key: OoId) {
    if let Some(command_key) = vm.oo.command_keys.remove(&obj_key) {
        vm.retire_command_lifecycle_key(&command_key);
    }
    let retired_name = vm
        .oo
        .objects
        .get(&obj_key)
        .and_then(|owner| owner.cached_name.borrow_mut().take());
    drop(retired_name);
    let class = vm.oo.classes.contains_key(&obj_key);
    vm.native_method_structure_changed(obj_key, class);
    vm.oo.retire_native_properties(obj_key);
    vm.oo.objects.remove(&obj_key);
    vm.oo.classes.remove(&obj_key);
    // Reporting bytes remain attached to the never-reused retired object token.
}

/// Move the mutable command-name projection attached to one stable `TclOO`
/// token. All class/object/provider relationships remain on [`OoId`].
pub(crate) fn oo_command_renamed(
    vm: &mut Vm,
    object: OoId,
    new_key: String,
    new_display: tcl_core_types::NameBytes,
) {
    let retired = vm
        .oo
        .objects
        .get(&object)
        .and_then(|original| original.cached_name.borrow_mut().take());
    drop(retired);
    vm.oo
        .command_keys
        .insert(object, CommandSidecarKey::visible(new_key));
    vm.oo.names.insert(object, new_display);
}

/// Relocate the command-table sidecar without changing `TclOO`'s public object
/// name. Hiding is not a Tcl command rename: `self object` continues to report
/// the visible name the object had before it became hidden.
pub(crate) fn oo_command_hidden(vm: &mut Vm, object: OoId, token: String) {
    vm.oo
        .command_keys
        .insert(object, CommandSidecarKey::hidden(token));
}

pub(crate) fn oo_command_exposed(vm: &mut Vm, object: OoId, new_key: String) {
    vm.oo
        .command_keys
        .insert(object, CommandSidecarKey::visible(new_key));
}

/// Run the `TclOO` delete lifecycle after the command mutation owner's delete
/// trace. The ordinary command mutation owner removes the table entry through
/// [`Vm::remove_command_exact`]; [`teardown`] reaches that same exact-key seam.
pub(crate) fn oo_command_deleted(vm: &mut Vm, object: OoId) {
    vm.oo.command_keys.remove(&object);
    let _ = oo_destroy(vm, object);
}

/// Run the `TclOO` lifecycle for a command implementation being replaced in
/// place. Tcl keeps the command-table token for the new implementation, while
/// the old object/class is destroyed (including descendants and destructors).
/// Detaching only this root key makes [`teardown`] leave that table entry to
/// the registration owner; cascaded objects retain their keys and are deleted
/// normally.
pub(crate) fn oo_command_replaced(vm: &mut Vm, object: OoId) {
    vm.oo.command_keys.remove(&object);
    let _ = oo_destroy(vm, object);
}

// oo::define / oo::objdefine and the definition-body commands.

fn cmd_oo_define(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    run_define(vm, args, true)
}

fn cmd_oo_objdefine(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    run_define(vm, args, false)
}

/// `oo::define target …` / `oo::objdefine target …` — both the script form
/// (`target {body}`) and the single-command form (`target sub args…`).
fn run_define(vm: &mut Vm, args: &[Value], is_class: bool) -> Completion<Value> {
    let verb = if is_class {
        "oo::define"
    } else {
        "oo::objdefine"
    };
    if args.len() < 2 {
        return err(format!(
            "wrong # args: should be \"{verb} target ?arg ...?\""
        ));
    }
    let target = match object_key(vm, &args[0]) {
        Ok(target) => target,
        Err(completion) => return completion,
    };
    if is_class && !vm.oo.classes.contains_key(&target) {
        return err(object_message(
            vm,
            target,
            b"",
            b" does not refer to a class",
        ));
    }
    let dt = if is_class {
        DefTarget::Class(target)
    } else {
        DefTarget::Object(target)
    };
    let caller = vm.current_ns_id();
    let caller_activation = vm.native_oo_variable_activation();
    let mut invocation = vec![
        vm.invoked_name_value()
            .unwrap_or_else(|| Value::string(verb)),
    ];
    invocation.extend_from_slice(args);
    if let Err(completion) = native_context::enter_definition(vm, is_class, invocation) {
        return completion;
    }
    let activation = vm.native_oo_variable_activation();
    vm.oo
        .def_stack
        .push((dt, activation, caller, caller_activation));
    let result = (|| {
        if args.len() == 2 {
            // Script form: evaluate the body with the def target active. An error
            // unwinding out of it gains the `(in definition script for …)` frame.
            let source = match vm.native_name_operand_bytes(&args[1]) {
                Ok(bytes) => tcl_lexer::SourceImage::native(bytes.as_ref()),
                Err(error) => return vm.refuse_host_command(error.to_string()),
            };
            match vm.eval_source_image_at_internal(&source, args[1].source_location()) {
                Ok(c) if c.code == Code::Return => ok(c.result),
                Ok(c) if c.code == Code::Error => {
                    append_define_frame(vm, is_class, target, &c.result.string_bytes());
                    c
                }
                Ok(c) => c,
                Err(e) => definition_error(vm, is_class, target, e),
            }
        } else {
            // Single-command form: dispatch `sub args…` as one definition directive
            // (no definition-script frame — the directive is the command itself).
            native_context::magic_definition_invoke(vm, &args[1], &args[2..])
        }
    })();
    vm.oo.def_stack.pop();
    native_context::leave_definition(vm);
    result
}

/// Append the `(in definition script for {class,object} "::NAME" line N)` frame
/// to `errorInfo` as an error unwinds out of an `oo::define`/`oo::objdefine`
/// *script* body — the context frame C's `TclOODefineObjCmd` adds. `line` is the
/// body-relative source line of the failing directive (C's `iPtr->errorLine`).
fn append_define_frame(vm: &mut Vm, is_class: bool, target: OoId, msg: &[u8]) {
    let kind = if is_class { "class" } else { "object" };
    let line = vm.error_line();
    let mut frame = format!("\n    (in definition script for {kind} ").into_bytes();
    frame.push(b'"');
    frame.extend_from_slice(&display_oo_bytes(vm, target));
    frame.extend_from_slice(format!("\" line {line})").as_bytes());
    vm.seed_error_info_frame(msg, &frame);
}

fn definition_error(
    vm: &mut Vm,
    is_class: bool,
    target: OoId,
    error: crate::error::TclError,
) -> Completion<Value> {
    if let Some(completion) = error.guest_completion()
        && completion.code == Code::Error
    {
        append_define_frame(vm, is_class, target, &completion.result.string_bytes());
    }
    crate::command::completion_from_tcl_error(vm, error)
}

/// The active definition target at the exact selected variable activation.
fn active_target(vm: &Vm) -> Option<(bool, OoId)> {
    let (dt, _, _, _) = &vm.oo.def_stack[native_context::definition_index(vm)?];
    Some(match dt {
        DefTarget::Class(c) => (true, *c),
        DefTarget::Object(o) => (false, *o),
    })
}

/// Write one `variable` declaration into the target's public or private slot.
fn declare_variable(vm: &mut Vm, args: &[Value], private: bool) -> Completion<Value> {
    use tcl_syntax::naming::{
        NativeOoVariableSlotOperation, native_oo_variable_slot, validate_native_oo_variable,
    };
    let Some((is_class, target)) = active_target(vm) else {
        return err_code(
            "this command may only be called from within the context of an ::oo::define or ::oo::objdefine command",
            &["TCL", "OO", "MONKEY_BUSINESS"],
        );
    };
    let Some(policy) = vm.name_policy_protocol() else {
        return vm
            .refuse_host_command("TclOO variable declaration protocol is unavailable".to_owned());
    };
    let mut incoming = Vec::new();
    for original in args {
        let bytes = match vm.native_name_operand_bytes(original) {
            Ok(bytes) => bytes,
            Err(error) => return vm.refuse_host_command(error.to_string()),
        };
        incoming.push(DeclaredVariable {
            original: original.clone(),
            name: NameBytes::from(bytes.as_ref()),
        });
    }
    let selection = match native_oo_variable_slot(
        policy.recipe(),
        incoming.first().map(|entry| entry.name.as_bytes()),
    ) {
        Ok(Ok(selection)) => selection,
        Ok(Err(error)) => return declared_variable_error(error),
        Err(error) => return vm.refuse_host_command(format!("{error:?}")),
    };
    if selection.consumes_first {
        incoming.remove(0);
    }
    if selection.operation == NativeOoVariableSlotOperation::Clear && !incoming.is_empty() {
        let words = if private {
            vec![
                Value::string("private"),
                Value::string("variable"),
                args[0].clone(),
            ]
        } else {
            vec![Value::string("variable"), args[0].clone()]
        };
        let usage = match vm.native_argument_usage_header(&words) {
            Ok(usage) => usage,
            Err(completion) => return completion,
        };
        return crate::command::native_wrong_args_bytes(vm, &usage);
    }
    if incoming.is_empty()
        && !matches!(
            selection.operation,
            NativeOoVariableSlotOperation::Set | NativeOoVariableSlotOperation::Clear
        )
    {
        return ok(Value::empty());
    }
    let existing = declared_variable_snapshot(vm, is_class, target, private);
    let Some(mut existing) = existing else {
        return ok(Value::empty());
    };
    for entry in &mut existing {
        let bytes = match vm.native_name_operand_bytes(&entry.original) {
            Ok(bytes) => bytes,
            Err(error) => return vm.refuse_host_command(error.to_string()),
        };
        entry.name = NameBytes::from(bytes.as_ref());
    }
    let updated = tcl_syntax::naming::apply_native_oo_variable_slot(
        existing,
        incoming,
        selection.operation,
        |entry| entry.name.as_bytes(),
    );
    for entry in &updated {
        match validate_native_oo_variable(policy.recipe(), entry.name.as_bytes()) {
            Ok(None) => {}
            Ok(Some(error)) => return declared_variable_error(error),
            Err(error) => return vm.refuse_host_command(format!("{error:?}")),
        }
    }
    publish_declared_variables(vm, is_class, target, private, updated);
    ok(Value::empty())
}

fn publish_declared_variables(
    vm: &mut Vm,
    is_class: bool,
    target: OoId,
    private: bool,
    updated: Vec<DeclaredVariable>,
) {
    let slot = if is_class {
        vm.oo.classes.get_mut(&target).map(|class| {
            if private {
                &mut class.private_variables
            } else {
                &mut class.variables
            }
        })
    } else {
        vm.oo.objects.get_mut(&target).map(|object| {
            if private {
                &mut object.private_variables
            } else {
                &mut object.variables
            }
        })
    };
    if let Some(slot) = slot {
        *slot = updated;
    }
}

fn declared_variable_snapshot(
    vm: &Vm,
    is_class: bool,
    target: OoId,
    private: bool,
) -> Option<Vec<DeclaredVariable>> {
    if is_class {
        vm.oo.classes.get(&target).map(|class| {
            if private {
                &class.private_variables
            } else {
                &class.variables
            }
        })
    } else {
        vm.oo.objects.get(&target).map(|object| {
            if private {
                &object.private_variables
            } else {
                &object.variables
            }
        })
    }
    .cloned()
}

/// `private variable name …` inside a definition body.
fn def_private_variable(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    declare_variable(vm, args, true)
}

/// The storage name a TIP 500 private variable occupies in the object's
/// namespace: C's `PRIVATE_VARIABLE_PATTERN`, `"<creation id> : <name>"`,
/// where the id is the *declaring provider's*.
///
/// That is what keeps a `Base` and a `Derived` each declaring `X` from sharing
/// one slot — measured on tclsh 9.0.4 and 9.1b0, an object of such a `Derived`
/// holds both `20 : X` and `22 : X`.
fn private_storage_name_bytes(creation_id: u64, name: &[u8]) -> NameBytes {
    let mut storage = format!("{creation_id} : ").into_bytes();
    storage.extend_from_slice(name);
    NameBytes::from(storage)
}

fn private_storage_name(creation_id: u64, name: &str) -> String {
    format!("{creation_id} : {name}")
}

/// The object-namespace storage name for `name` as seen from the running
/// method, mangled when the method's own provider declares it private.
///
/// `my variable X` and `my varname X` must reach the same slot ordinary `$X`
/// access does. Without this they target the unmangled `{ns}::X`, so
/// `my variable X` would replace the private mapping with a public one and
/// `my varname X` would hand back a path to a different variable (#1933).
fn frame_storage_name(vm: &Vm, ns: &str, name: &str) -> String {
    let is_private = native_context::current_method(vm).is_some_and(|fr| {
        fr.chain.get(fr.index).is_some_and(|step| {
            let declared = if step.is_object {
                vm.oo
                    .objects
                    .get(&step.provider)
                    .map(|o| &o.private_variables)
            } else {
                vm.oo
                    .classes
                    .get(&step.provider)
                    .map(|c| &c.private_variables)
            };
            declared.is_some_and(|d| d.iter().any(|v| v.name.as_bytes() == name.as_bytes()))
        })
    });
    if !is_private {
        return format!("{ns}::{name}");
    }
    let creation_id = native_context::current_method(vm)
        .and_then(|fr| fr.chain.get(fr.index))
        .and_then(|step| vm.oo.objects.get(&step.provider))
        .map_or(0, |o| o.creation_id);
    format!("{ns}::{}", private_storage_name(creation_id, name))
}

/// A definition-body command (`method`, `superclass`, …). Resolves the active
/// target and applies the directive.
fn def_body_cmd(vm: &mut Vm, verb: &str, args: &[Value]) -> Completion<Value> {
    let Some((is_class, target)) = active_target(vm) else {
        return err_code(
            "this command may only be called from within the context of an ::oo::define or ::oo::objdefine command",
            &["TCL", "OO", "MONKEY_BUSINESS"],
        );
    };
    if !vm.oo.objects.contains_key(&target) {
        return err_code(
            "this command cannot be called when the object has been deleted",
            &["TCL", "OO", "MONKEY_BUSINESS"],
        );
    }
    match verb {
        "method" => def_method(vm, is_class, target, args),
        "deletemethod" => native_method_cache::delete_method(vm, is_class, target, args),
        "renamemethod" => native_method_cache::rename_method(vm, is_class, target, args),
        "constructor" => def_constructor(vm, is_class, target, args),
        "destructor" => def_destructor(vm, is_class, target, args),
        "superclass" => def_superclass(vm, is_class, target, args),
        "export" => def_export(vm, is_class, target, args, true),
        "unexport" => def_export(vm, is_class, target, args, false),
        "mixin" => def_mixin(vm, is_class, target, args),
        "forward" => def_forward(vm, is_class, target, args),
        "variable" => declare_variable(vm, args, vm.oo.private_depth > 0),
        _ => err(format!("unknown definition command \"{verb}\"")),
    }
}

/// Retain a method/constructor declaration with the native definition owner.
fn build_method(
    vm: &mut Vm,
    params_spec: &Value,
    body: &Value,
    exported: bool,
) -> Result<Method, Completion<Value>> {
    let body_src = vm
        .capture_procedure_declaration_body(params_spec, body)
        .map_err(|error| crate::command::completion_from_cmd_error(vm, error.into()))?;
    let (params, has_args) = parse_params_value(vm, params_spec, b"")?;
    Ok(Method {
        params,
        has_args,
        compiled_body: None,
        body_src,
        forward: None,
        property: None,
        exported,
    })
}

fn def_method(vm: &mut Vm, is_class: bool, target: OoId, args: &[Value]) -> Completion<Value> {
    // `method name ?-export|-unexport? args body` (a subset of the flag set).
    let mut idx = 0;
    let mut vis: Option<bool> = None;
    while let Some(a) = args.get(idx) {
        let bytes = match vm.native_name_operand_bytes(a) {
            Ok(bytes) => bytes,
            Err(error) => return vm.refuse_host_command(error.to_string()),
        };
        match bytes.as_ref() {
            b"-export" => vis = Some(true),
            b"-unexport" | b"-private" => vis = Some(false),
            _ => break,
        }
        idx += 1;
    }
    let (Some(name), Some(params), Some(body)) =
        (args.get(idx), args.get(idx + 1), args.get(idx + 2))
    else {
        return crate::command::native_wrong_args(vm, "method name ?option? args body");
    };
    let name = match native_method_key(vm, name) {
        Ok(name) => name,
        Err(completion) => return completion,
    };
    let exported = vis.unwrap_or_else(|| {
        vm.oo
            .method_name_bytes(&name)
            .first()
            .is_some_and(u8::is_ascii_lowercase)
    });
    let method = match build_method(vm, params, body, exported) {
        Ok(m) => m,
        Err(e) => return e,
    };
    if is_class {
        if let Some(c) = vm.oo.classes.get_mut(&target) {
            c.methods.insert(name, method);
        }
    } else if let Some(o) = vm.oo.objects.get_mut(&target) {
        o.methods.insert(name, method);
    }
    vm.native_property_method_created(target, is_class);
    ok(Value::empty())
}

fn def_constructor(vm: &mut Vm, is_class: bool, target: OoId, args: &[Value]) -> Completion<Value> {
    if !is_class {
        return err("constructors are only for classes");
    }
    let [params, body] = args else {
        return crate::command::native_wrong_args(vm, "constructor arguments body");
    };
    // An empty (whitespace-only) body installs *no* constructor — C TclOO's
    // optimisation, so `C new <extra args>` is then silently accepted.
    if body.to_str().is_empty() {
        if let Some(c) = vm.oo.classes.get_mut(&target) {
            let changed = c.constructor.take().is_some();
            if changed {
                vm.native_property_structure_changed(target, true);
            }
        }
        return ok(Value::empty());
    }
    let method = match build_method(vm, params, body, true) {
        Ok(m) => m,
        Err(e) => return e,
    };
    if let Some(c) = vm.oo.classes.get_mut(&target) {
        c.constructor.install(method);
    }
    vm.native_property_method_created(target, true);
    vm.native_property_structure_changed(target, true);
    ok(Value::empty())
}

fn def_destructor(vm: &mut Vm, is_class: bool, target: OoId, args: &[Value]) -> Completion<Value> {
    if !is_class {
        return err("destructors are only for classes");
    }
    let [body] = args else {
        return crate::command::native_wrong_args(vm, "destructor body");
    };
    if body.to_str().is_empty() {
        if let Some(c) = vm.oo.classes.get_mut(&target) {
            let changed = c.destructor.take().is_some();
            if changed {
                vm.native_property_structure_changed(target, true);
            }
        }
        return ok(Value::empty());
    }
    let method = match build_method(vm, &Value::empty(), body, true) {
        Ok(m) => m,
        Err(e) => return e,
    };
    if let Some(c) = vm.oo.classes.get_mut(&target) {
        c.destructor.install(method);
    }
    vm.native_property_method_created(target, true);
    vm.native_property_structure_changed(target, true);
    ok(Value::empty())
}

fn def_superclass(vm: &mut Vm, is_class: bool, target: OoId, args: &[Value]) -> Completion<Value> {
    if !is_class {
        return err("superclass is only for classes");
    }
    let mut supers = Vec::new();
    for a in args {
        let Some(s) = (match resolve_class_value(vm, a) {
            Ok(class) => class,
            Err(completion) => return completion,
        }) else {
            return err("only a class can be a superclass");
        };
        if s == target {
            return err("attempt to form circular dependency graph");
        }
        supers.push(s);
    }
    if let Some(c) = vm.oo.classes.get_mut(&target) {
        c.supers = supers;
    }
    vm.native_property_structure_changed(target, is_class);
    ok(Value::empty())
}

fn def_export(
    vm: &mut Vm,
    is_class: bool,
    target: OoId,
    args: &[Value],
    export: bool,
) -> Completion<Value> {
    let mut changed = false;
    if !is_class && !args.is_empty() {
        vm.oo.native_methods.instance_table_created(target);
    }
    for a in args {
        let name = match native_method_key(vm, a) {
            Ok(name) => name,
            Err(completion) => return completion,
        };
        if is_class {
            if let Some(c) = vm.oo.classes.get_mut(&target) {
                if export {
                    changed |= c.unexported.remove(&name);
                    changed |= c.exported.insert(name);
                } else {
                    changed |= c.exported.remove(&name);
                    changed |= c.unexported.insert(name);
                }
            }
        } else if let Some(o) = vm.oo.objects.get_mut(&target) {
            o.methods.mark_allocated();
            if export {
                changed |= o.unexported.remove(&name);
                changed |= o.exported.insert(name);
            } else {
                changed |= o.exported.remove(&name);
                changed |= o.unexported.insert(name);
            }
        }
    }
    if changed {
        vm.native_property_structure_changed(target, is_class);
    }
    ok(Value::empty())
}

fn def_mixin(vm: &mut Vm, is_class: bool, target: OoId, args: &[Value]) -> Completion<Value> {
    let mut mixins = Vec::new();
    for a in args {
        let Some(s) = (match resolve_class_value(vm, a) {
            Ok(class) => class,
            Err(completion) => return completion,
        }) else {
            return err("may only mix in classes");
        };
        mixins.push(s);
    }
    if is_class {
        if let Some(c) = vm.oo.classes.get_mut(&target) {
            c.mixins = mixins;
        }
    } else if let Some(o) = vm.oo.objects.get_mut(&target) {
        o.mixins = mixins;
    }
    if !is_class {
        vm.oo.recompute_native_method_class_cache(target);
    }
    vm.native_property_structure_changed(target, is_class);
    ok(Value::empty())
}

fn def_forward(vm: &mut Vm, is_class: bool, target: OoId, args: &[Value]) -> Completion<Value> {
    let Some((name, prefix)) = args.split_first().filter(|(_, prefix)| !prefix.is_empty()) else {
        return crate::command::native_wrong_args(vm, "forward name cmdName ?arg ...?");
    };
    let name = match native_method_key(vm, name) {
        Ok(name) => name,
        Err(completion) => return completion,
    };
    let exported = vm
        .oo
        .method_name_bytes(&name)
        .first()
        .is_some_and(u8::is_ascii_lowercase);
    let method = Method {
        params: Vec::new(),
        has_args: true,
        compiled_body: None,
        body_src: Value::empty(),
        forward: Some(prefix.to_vec()),
        property: None,
        exported,
    };
    if is_class {
        if let Some(c) = vm.oo.classes.get_mut(&target) {
            c.methods.insert(name, method);
        }
    } else if let Some(o) = vm.oo.objects.get_mut(&target) {
        o.methods.insert(name, method);
    }
    vm.native_property_method_created(target, is_class);
    ok(Value::empty())
}

// oo::class create — the metaclass factory, handled specially so the new
// class's definition body runs with a Class def target.

// info object / info class introspection (a common subset).

impl OoState {
    /// Whether `class_key` is reachable in the linearisation of object `obj`'s
    /// class (i.e. `obj` is an instance of `class_key`, honouring inheritance).
    fn is_a(&self, obj_key: OoId, class_key: OoId) -> bool {
        self.objects.get(&obj_key).is_some_and(|o| {
            self.class_linear_of(o.class)
                .iter()
                .any(|s| s.provider == class_key)
        })
    }

    /// Whether `class_key` is a metaclass (its instances are classes — its
    /// linearisation includes `::oo::class`).
    fn is_metaclass(&self, class_key: OoId) -> bool {
        self.classes.contains_key(&class_key)
            && self
                .class_linear_of(class_key)
                .iter()
                .any(|s| Some(s.provider) == self.class_root)
    }

    /// Whether `class_key`'s linearisation includes any `configurable` class —
    /// i.e. it was built by `oo::configurable` or inherits from such a class
    /// (TIP 558). Gates the `property` definition command.
    fn class_is_configurable(&self, class_key: OoId) -> bool {
        self.class_linear_of(class_key).iter().any(|s| {
            self.classes
                .get(&s.provider)
                .is_some_and(|c| c.configurable)
        })
    }

    /// Whether object `obj_key` should carry the `configure` method: its class
    /// (via the MRO) is configurable, or it has per-object property slots.
    fn object_is_configurable(&self, obj_key: OoId) -> bool {
        self.objects.get(&obj_key).is_some_and(|o| {
            !o.readable_properties.is_empty()
                || !o.writable_properties.is_empty()
                || self.class_is_configurable(o.class)
        })
    }
}

/// Whether `name` (as written at the call site) resolves to a `TclOO` object —
/// `info object isa object` and the `tclooIsObject` opcode, neither of which
/// ever errors.
pub(crate) fn is_object(vm: &mut Vm, name: &Value) -> Result<bool, Completion<Value>> {
    let present = resolve_object_value(vm, name)?.is_some();
    if !present {
        object_lookup_failure(vm, name)?;
    }
    Ok(present)
}

/// Resolve an original native object command operand at the current token.
/// Missing binding is a guest lookup failure; unavailable lookup remains a host refusal.
pub(crate) fn object_key(vm: &mut Vm, name: &Value) -> Result<OoId, Completion<Value>> {
    if let Some(object) = resolve_object_value(vm, name)? {
        return Ok(object);
    }
    Err(object_lookup_failure(vm, name)?)
}

fn object_lookup_failure(
    vm: &mut Vm,
    name: &Value,
) -> Result<Completion<Value>, Completion<Value>> {
    let original = vm
        .native_name_operand_bytes(name)
        .map_err(|error| vm.refuse_host_command(error.to_string()))?;
    let original = tcl_core_types::c_string_extent(&original);
    let dialect = vm.actual_native_invocation_dialect();
    let protocol = dialect
        .native_string_protocol()
        .filter(|protocol| protocol.tcl_version().is_some())
        .ok_or_else(|| vm.refuse_host_command("native TclOO object error producer".to_owned()))?;
    let materialization = dialect.native_string_materialization(None).ok_or_else(|| {
        vm.refuse_host_command("native TclOO object error String producer".to_owned())
    })?;
    let mut message = original.to_vec();
    message.extend_from_slice(b" does not refer to an object");
    let message = Value::new_native_string_bytes(message);
    message
        .retain_native_string_representation(materialization)
        .map_err(|error| vm.refuse_host_command(error.to_string()))?;
    vm.adopt_native_interp_result(message.clone())
        .map_err(|error| vm.refuse_host_command(error.to_string()))?;
    let code = Value::native_list_constructor(
        [b"TCL".as_slice(), b"LOOKUP", b"OBJECT", original]
            .into_iter()
            .map(Value::new_native_string_bytes)
            .collect(),
        protocol,
    );
    vm.retain_return_error_code(Some(&code), false);
    Ok(Completion::new_error_metadata(
        Code::Error,
        message,
        crate::command::options_dict(Code::Error, 0, &[("-errorcode", code)]),
    ))
}

/// The class command name of the object `key` (`info object class`, the
/// `tclooClass` opcode). `key` must be a resolved object key — see
/// [`object_key`].
pub(crate) fn object_class_name(vm: &Vm, key: OoId) -> Value {
    native_context::object_name(vm, vm.oo.objects[&key].class)
}

/// The instance namespace of the object `key` (`info object namespace`, the
/// `tclooNamespace` opcode). `key` must be a resolved object key — see
/// [`object_key`].
pub(crate) fn object_namespace_name(vm: &mut Vm, key: OoId) -> Result<Value, Completion<Value>> {
    let Some(namespace) = vm.oo.objects[&key].namespace_token else {
        return Err(
            vm.refuse_host_command("original object namespace incarnation unavailable".to_owned())
        );
    };
    vm.native_namespace_result_object(
        namespace,
        tcl_syntax::native_namespace_name::NativeNamespaceObjectProducer::ObjectNamespace,
    )
    .map_err(|error| vm.refuse_host_command(error.to_string()))
}

/// Produce the actual wide creation epoch without guessing an overflowed value.
pub(crate) fn object_creation_id(vm: &mut Vm, key: OoId) -> Result<Value, Completion<Value>> {
    let epoch = i64::try_from(vm.oo.objects[&key].creation_id).map_err(|_| {
        vm.refuse_host_command(
            "original object creation epoch exceeds native wide integer".to_owned(),
        )
    })?;
    Ok(Value::int(epoch))
}

/// `info object subcommand object ?arg…?`.
pub(crate) fn info_object(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some((sub, rest)) = args.split_first() else {
        return crate::command::native_wrong_args(vm, "info object subcommand object ?arg ...?");
    };
    let sub = match vm.native_name_operand_bytes(sub) {
        Ok(bytes) => bytes,
        Err(error) => return vm.refuse_host_command(error.to_string()),
    };
    let sub = tcl_core_types::c_string_extent(&sub);
    let subcommands = info_oo_subcommands(InfoOoEnsembleKind::Object, vm.runtime_version());
    let sub = match subcommands.resolve(sub) {
        Ok(sub) => sub,
        Err(message) => return err(message),
    };
    // `isa` is the odd one out: `info object isa category object ?arg?` — the
    // object comes *after* the category.
    if sub == "isa" {
        return info_object_isa(vm, rest);
    }
    let Some(obj_arg) = rest.first() else {
        return err(format!(
            "wrong # args: should be \"info object {sub} object ?arg ...?\""
        ));
    };
    let obj = match object_key(vm, obj_arg) {
        Ok(k) => k,
        Err(c) => return c,
    };
    let extra = &rest[1..];
    match sub {
        "class" => {
            if let Some(cls) = extra.first() {
                let class = match resolve_class_value(vm, cls) {
                    Ok(class) => class,
                    Err(completion) => return completion,
                };
                let is_a = class.is_some_and(|class| vm.oo.is_a(obj, class));
                ok(Value::int(i64::from(is_a)))
            } else {
                ok(object_class_name(vm, obj))
            }
        }
        "namespace" => object_namespace_name(vm, obj).map_or_else(|completion| completion, ok),
        "creationid" => object_creation_id(vm, obj).map_or_else(|completion| completion, ok),
        "mixins" => ok(Value::list(
            vm.oo.objects[&obj]
                .mixins
                .iter()
                .map(|m| Value::from_native_string_bytes(display_oo_bytes(vm, *m)))
                .collect(),
        )),
        "variables" => {
            // TIP 500: the plain form reports only public declarations; the
            // `-private` form reports only the private ones.
            let private = extra.iter().any(|v| v.to_str().as_ref() == "-private");
            let o = &vm.oo.objects[&obj];
            let names = if private {
                &o.private_variables
            } else {
                &o.variables
            };
            ok(Value::list(
                names.iter().map(|entry| entry.original.clone()).collect(),
            ))
        }
        "vars" => info_object_variable_names(vm, obj, extra),
        "methods" => {
            let all = extra.iter().any(|v| v.to_str().as_ref() == "-all");
            let mut names: BTreeSet<String> = vm.oo.objects[&obj].methods.keys().cloned().collect();
            if all {
                names.extend(vm.oo.visible_methods(obj, false));
                names.remove("create");
                names.remove("new");
                names.remove("destroy");
            }
            ok(method_list_value(vm, names))
        }
        "properties" => info_object_properties(vm, obj, extra),
        _ => err(format!("unsupported info object subcommand \"{sub}\"")),
    }
}

fn info_object_variable_names(vm: &mut Vm, obj: OoId, extra: &[Value]) -> Completion<Value> {
    let namespace = vm.definition_namespace_token(&vm.oo.objects[&obj].ns.clone());
    let pattern = match extra
        .first()
        .map(|value| vm.native_name_operand_bytes(value))
        .transpose()
    {
        Ok(pattern) => pattern,
        Err(error) => return vm.refuse_host_command(error.to_string()),
    };
    let Some(policy) = vm.name_policy_protocol() else {
        return vm
            .refuse_host_command("TclOO variable enumeration protocol is unavailable".to_owned());
    };
    let matcher = tcl_syntax::native_glob::NativeGlobProtocol::from_name_policy(policy);
    let mut names = Vec::new();
    for name in tcl_runtime_api::Namespaces::vars_in_bytes(vm, namespace) {
        if !vm.tcloo_storage_variable_is_defined(namespace, &name) {
            continue;
        }
        if let Some(pattern) = pattern.as_ref() {
            match matcher.match_name_pattern(
                tcl_syntax::native_glob::NativeNameGlobPurpose::OoObjectNamesSearch,
                pattern,
                &name,
            ) {
                Ok(true) => {}
                Ok(false) => continue,
                Err(error) => return vm.refuse_host_command(error.to_string()),
            }
        }
        names.push(Value::from_native_string_bytes(name));
    }
    ok(Value::list(names))
}

/// `info object properties object ?-all? ?-readable|-writable?` (TIP 558).
///
/// Reports the readable (default) or writable properties declared directly on
/// the object; `-all` unions in the object's full precedence (its mixins, then
/// its class MRO). Sorted and duplicate-free.
fn info_object_properties(vm: &mut Vm, obj: OoId, extra: &[Value]) -> Completion<Value> {
    let (all, writable) = match info_properties_options(extra) {
        Ok(flags) => flags,
        Err(error) => return error,
    };
    if all
        && vm
            .actual_native_invocation_dialect()
            .native_property_lookup_protocol()
            .is_some()
    {
        return match vm.native_all_property_header(obj, false, writable) {
            Ok(header) => ok(header.value().clone()),
            Err(error) => crate::command::completion_from_cmd_error(vm, error.into()),
        };
    }
    if vm
        .actual_native_invocation_dialect()
        .native_property_lookup_protocol()
        .is_some()
    {
        return match vm.native_declared_property_header(obj, false, writable) {
            Ok(header) => ok(header),
            Err(error) => crate::command::completion_from_cmd_error(vm, error.into()),
        };
    }
    let mut names: BTreeSet<String> = BTreeSet::new();
    if all {
        for step in vm.oo.object_precedence(obj) {
            if step.is_object {
                if let Some(o) = vm.oo.objects.get(&step.provider) {
                    names.extend(slot_ref_obj(o, writable).iter().cloned());
                }
            } else if let Some(c) = vm.oo.classes.get(&step.provider) {
                names.extend(slot_ref_class(c, writable).iter().cloned());
            }
        }
    } else if let Some(o) = vm.oo.objects.get(&obj) {
        names.extend(slot_ref_obj(o, writable).iter().cloned());
    }
    ok(Value::list(names.into_iter().map(Value::string).collect()))
}

/// `info object isa`'s category word, in C table order (`categories[]`,
/// `tclOOInfo.c`): `Tcl_GetIndexFromObj(…, "category", 0)`, so `cl`/`ob`/`t`
/// abbreviate, `m` is ambiguous (metaclass/mixin), and the empty word — a
/// prefix of all five — is `ambiguous category ""`. The Oxford comma before
/// `or` is `Tcl_GetIndexFromObj`'s, not `TclOO`'s method-list style.
const ISA_CATEGORIES: tcl_cmd_core::prefix::OptionTable<'static> =
    tcl_cmd_core::prefix::OptionTable::abbreviating(
        "category",
        &["class", "metaclass", "mixin", "object", "typeof"],
    );

/// `info object isa category object ?arg?`.
fn info_object_isa(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    // C checks the arity in two stages (`InfoObjectIsACmd`, tclOOInfo.c).
    // First, `category objName` must both be present — *before* the category
    // word is resolved, so `info object isa object` is this message and not a
    // category error.
    let (Some(kind), Some(obj_arg)) = (rest.first(), rest.get(1)) else {
        return crate::command::native_wrong_args(vm, "info object isa category objName ?arg ...?");
    };
    let kind = match vm.native_static_option_index(kind, ISA_CATEGORIES.names(), false, "category")
    {
        Ok(i) => ISA_CATEGORIES.names()[i],
        Err(e) => return crate::command::completion_from_cmd_error(vm, e),
    };
    // Then each category pins an *exact* count — C's second stage,
    // `Tcl_WrongNumArgs(interp, 2, objv, …)`, whose noun carries the resolved
    // category word (an index-typed argument prints as its table entry). So
    // `info object isa t o` is `"info object isa typeof objName className"`,
    // and a trailing extra word is an error rather than being ignored.
    let (want, tail) = match kind {
        "mixin" | "typeof" => (3, " objName className"),
        // class / metaclass / object.
        _ => (2, " objName"),
    };
    if rest.len() != want {
        return err(format!(
            "wrong # args: should be \"info object isa {kind}{tail}\""
        ));
    }
    let obj = match resolve_object_value(vm, obj_arg) {
        Ok(object) => object,
        Err(completion) => return completion,
    };
    // `isa object` on a non-object is a plain false, not an error — the same
    // test the `tclooIsObject` opcode this form compiles to performs.
    if obj.is_none()
        && let Err(completion) = object_lookup_failure(vm, obj_arg)
    {
        return completion;
    }
    if kind == "object" {
        return ok(Value::int(i64::from(obj.is_some())));
    }
    let Some(obj) = obj else {
        return ok(Value::int(0));
    };
    let res = match kind {
        "class" => vm.oo.classes.contains_key(&obj),
        "metaclass" => vm.oo.is_metaclass(obj),
        "typeof" | "mixin" => {
            let class = match rest.get(2) {
                Some(original) => match resolve_class_value(vm, original) {
                    Ok(class) => class,
                    Err(completion) => return completion,
                },
                None => None,
            };
            class.is_some_and(|class| {
                if kind == "typeof" {
                    vm.oo.is_a(obj, class)
                } else {
                    vm.oo.objects[&obj].mixins.contains(&class)
                }
            })
        }
        // Unreachable: `ISA_CATEGORIES` has exactly these five names.
        _ => false,
    };
    ok(Value::int(i64::from(res)))
}

/// `info class subcommand class ?arg…?`.
pub(crate) fn info_class(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some((sub, rest)) = args.split_first() else {
        return crate::command::native_wrong_args(vm, "info class subcommand class ?arg ...?");
    };
    let sub = match vm.native_name_operand_bytes(sub) {
        Ok(bytes) => bytes,
        Err(error) => return vm.refuse_host_command(error.to_string()),
    };
    let sub = tcl_core_types::c_string_extent(&sub);
    let subcommands = info_oo_subcommands(InfoOoEnsembleKind::Class, vm.runtime_version());
    let sub = match subcommands.resolve(sub) {
        Ok(sub) => sub,
        Err(message) => return err(message),
    };
    let Some(cls_arg) = rest.first() else {
        return err(format!(
            "wrong # args: should be \"info class {sub} class ?arg ...?\""
        ));
    };
    let cls = match object_key(vm, cls_arg) {
        Ok(cls) => cls,
        Err(completion) => return completion,
    };
    if !vm.oo.classes.contains_key(&cls) {
        return err(object_message(vm, cls, b"\"", b"\" is not a class"));
    }
    let extra = &rest[1..];
    match sub {
        "superclasses" => ok(Value::list(
            vm.oo.classes[&cls]
                .supers
                .iter()
                .map(|s| Value::from_native_string_bytes(display_oo_bytes(vm, *s)))
                .collect(),
        )),
        "mixins" => ok(Value::list(
            vm.oo.classes[&cls]
                .mixins
                .iter()
                .map(|m| Value::from_native_string_bytes(display_oo_bytes(vm, *m)))
                .collect(),
        )),
        "subclasses" | "instances" => info_class_related_names(vm, cls, extra, sub),
        "methods" => info_class_methods(vm, cls, extra),
        "constructor" => match vm.oo.classes[&cls].constructor.get() {
            Some(m) => ok(Value::list(vec![
                params_value(&m.params),
                m.body_src.clone(),
            ])),
            None => ok(Value::empty()),
        },
        "destructor" => match vm.oo.classes[&cls].destructor.get() {
            Some(m) => ok(m.body_src.clone()),
            None => ok(Value::empty()),
        },
        "variables" => {
            // TIP 500, as for `info object variables`.
            let private = extra.iter().any(|v| v.to_str().as_ref() == "-private");
            let c = &vm.oo.classes[&cls];
            let names = if private {
                &c.private_variables
            } else {
                &c.variables
            };
            ok(Value::list(
                names.iter().map(|entry| entry.original.clone()).collect(),
            ))
        }
        "properties" => info_class_properties(vm, cls, extra),
        _ => err(format!("unsupported info class subcommand \"{sub}\"")),
    }
}

fn info_class_related_names(
    vm: &mut Vm,
    cls: OoId,
    extra: &[Value],
    sub: &str,
) -> Completion<Value> {
    let pattern = match extra.first() {
        Some(original) => match vm.native_name_operand_bytes(original) {
            Ok(bytes) => Some(bytes),
            Err(error) => return vm.refuse_host_command(error.to_string()),
        },
        None => None,
    };
    let tokens: Vec<_> = if sub == "subclasses" {
        vm.oo
            .classes
            .iter()
            .filter(|(key, class)| **key != cls && class.supers.contains(&cls))
            .map(|(key, _)| *key)
            .collect()
    } else {
        vm.oo
            .objects
            .iter()
            .filter(|(key, object)| object.class == cls && !vm.oo.classes.contains_key(*key))
            .map(|(key, _)| *key)
            .collect()
    };
    let Some(policy) = vm.name_policy_protocol() else {
        return vm.refuse_host_command("native TclOO enumeration protocol unavailable".to_owned());
    };
    let matcher = tcl_syntax::native_glob::NativeGlobProtocol::from_name_policy(policy);
    let mut names = Vec::new();
    for token in tokens {
        let name = display_oo_bytes(vm, token);
        if let Some(pattern) = pattern.as_ref() {
            match matcher.match_name_pattern(
                tcl_syntax::native_glob::NativeNameGlobPurpose::OoObjectNamesSearch,
                pattern,
                &name,
            ) {
                Ok(true) => {}
                Ok(false) => continue,
                Err(error) => return vm.refuse_host_command(error.to_string()),
            }
        }
        names.push(name);
    }
    names.sort();
    ok(Value::list(
        names
            .into_iter()
            .map(Value::from_native_string_bytes)
            .collect(),
    ))
}

/// `info class properties class ?-all? ?-readable|-writable?` (TIP 558).
///
/// Reports the readable (default) or writable property names declared on the
/// class; `-all` unions in every class up the MRO (mixins + superclasses). The
/// result is sorted and duplicate-free.
fn info_class_properties(vm: &mut Vm, cls: OoId, extra: &[Value]) -> Completion<Value> {
    let (all, writable) = match info_properties_options(extra) {
        Ok(flags) => flags,
        Err(error) => return error,
    };
    if all
        && vm
            .actual_native_invocation_dialect()
            .native_property_lookup_protocol()
            .is_some()
    {
        return match vm.native_all_property_header(cls, true, writable) {
            Ok(header) => ok(header.value().clone()),
            Err(error) => crate::command::completion_from_cmd_error(vm, error.into()),
        };
    }
    if vm
        .actual_native_invocation_dialect()
        .native_property_lookup_protocol()
        .is_some()
    {
        return match vm.native_declared_property_header(cls, true, writable) {
            Ok(header) => ok(header),
            Err(error) => crate::command::completion_from_cmd_error(vm, error.into()),
        };
    }
    let mut names: BTreeSet<String> = BTreeSet::new();
    if all {
        for step in vm.oo.class_linear_of(cls) {
            if let Some(c) = vm.oo.classes.get(&step.provider) {
                names.extend(slot_ref_class(c, writable).iter().cloned());
            }
        }
    } else if let Some(c) = vm.oo.classes.get(&cls) {
        names.extend(slot_ref_class(c, writable).iter().cloned());
    }
    ok(Value::list(names.into_iter().map(Value::string).collect()))
}

fn info_properties_options(extra: &[Value]) -> Result<(bool, bool), Completion<Value>> {
    let (mut all, mut writable) = (false, false);
    for value in extra {
        let word = value.to_str();
        match resolve_info_oo_properties_option(word.as_bytes()) {
            Ok(InfoOoPropertiesOption::All) => all = true,
            Ok(InfoOoPropertiesOption::Readable) => writable = false,
            Ok(InfoOoPropertiesOption::Writable) => writable = true,
            Err(message) => {
                return Err(err_code(
                    String::from_utf8_lossy(&message).into_owned(),
                    &["TCL", "LOOKUP", "INDEX", "option", &word],
                ));
            }
        }
    }
    Ok((all, writable))
}

fn slot_ref_class(c: &Class, writable: bool) -> &BTreeSet<String> {
    if writable {
        &c.writable_properties
    } else {
        &c.readable_properties
    }
}

fn slot_ref_obj(o: &Object, writable: bool) -> &BTreeSet<String> {
    if writable {
        &o.writable_properties
    } else {
        &o.readable_properties
    }
}

/// `info class methods class ?-all? ?-private?`.
fn info_class_methods(vm: &mut Vm, cls: OoId, extra: &[Value]) -> Completion<Value> {
    let all = extra.iter().any(|v| v.to_str().as_ref() == "-all");
    let private = extra.iter().any(|v| v.to_str().as_ref() == "-private");
    let mut names: Vec<String> = if all {
        let mut s = vm.oo.visible_methods(cls, private);
        s.remove("create");
        s.remove("new");
        s.remove("destroy");
        s.into_iter().collect()
    } else {
        let c = &vm.oo.classes[&cls];
        c.methods
            .iter()
            .filter(|(n, m)| private || m.borrow().exported || c.exported.contains(*n))
            .filter(|(n, _)| private || !c.unexported.contains(*n))
            .map(|(n, _)| n.clone())
            .collect()
    };
    names.sort();
    ok(method_list_value(vm, names))
}

/// Render a param list as an `info class constructor`-style arg spec (`args` is
/// already the last element when present).
fn params_value(params: &[Param]) -> Value {
    Value::list(
        params
            .iter()
            .map(|p| match &p.default {
                Some(d) => {
                    Value::list(vec![Value::from_string_bytes(p.name.as_bytes()), d.clone()])
                }
                None => Value::from_string_bytes(p.name.as_bytes()),
            })
            .collect(),
    )
}

/// Create a class named `class_key` (canonical), optionally running `body` as
/// its definition script. `metaclass` is the class being instantiated to build
/// it (`oo::class`, `oo::configurable`, or a user metaclass): it becomes the new
/// class's object-facet class, and — when it is `oo::configurable` or descends
/// from it — marks the new class `configurable` (TIP 558).
fn make_class_bytes(
    vm: &mut Vm,
    class_key: &[u8],
    body: Option<&Value>,
    metaclass: OoId,
) -> Completion<Value> {
    let slot = match vm.native_object_publication_slot(class_key) {
        Ok(selected) => selected,
        Err(error) => return vm.refuse_host_command(error.to_string()),
    };
    match vm.command_at_exact_slot_checked(&slot, true) {
        Ok(Some(_)) => {
            let mut message = b"can't create object \"".to_vec();
            message.extend_from_slice(tcl_core_types::c_string_extent(class_key));
            message.extend_from_slice(b"\": command already exists with that name");
            return err(message);
        }
        Ok(None) => {}
        Err(error) => return vm.refuse_host_command(error.to_string()),
    }
    // A class is configurable when instantiated *from* `oo::configurable` (or a
    // metaclass descending from it) — an identity test, distinct from the MRO
    // walk used for `configure` availability (which inherits down subclasses).
    let configurable = Some(metaclass) == vm.oo.configurable_root
        || vm
            .oo
            .class_linear_of(metaclass)
            .iter()
            .any(|s| Some(s.provider) == vm.oo.configurable_root);
    let class_display = tcl_core_types::NameBytes::from(vm.command_slot_display_bytes(&slot));
    let class_namespace = fresh_obj_name(vm);
    let class_id = vm.oo.allocate(class_display);
    vm.oo.classes.insert(
        class_id,
        Class {
            configurable,
            ..Class::default()
        },
    );
    vm.oo.objects.insert(
        class_id,
        Object {
            cached_name: Rc::new(RefCell::new(None)),
            class: metaclass,
            ns: class_namespace.clone(),
            namespace_token: None,
            creation_id: class_id.0,
            methods: native_method_cache::MethodTable::default(),
            variables: Vec::new(),
            private_variables: Vec::new(),
            mixins: Vec::new(),
            exported: BTreeSet::new(),
            unexported: BTreeSet::new(),
            readable_properties: BTreeSet::new(),
            writable_properties: BTreeSet::new(),
            destroyed: false,
        },
    );
    vm.activate_namespace_written(&format!("::{class_namespace}"));
    let namespace_token = native_context::install_object_helpers(vm, &class_namespace);
    vm.oo.objects.get_mut(&class_id).unwrap().namespace_token = Some(namespace_token);
    let command_key = vm.register_command_in_slot(slot, Command::Object(class_id));
    vm.oo
        .command_keys
        .insert(class_id, CommandSidecarKey::visible(command_key));
    if let Some(body) = body {
        let res = evaluate_class_definition(vm, class_id, body);
        if !res.code.is_ok() && res.code != Code::Return {
            teardown(vm, class_id);
            return res;
        }
    }
    ok(native_context::object_name(vm, class_id))
}

fn evaluate_class_definition(vm: &mut Vm, class_id: OoId, body: &Value) -> Completion<Value> {
    let caller = vm.current_ns_id();
    let caller_activation = vm.native_oo_variable_activation();
    let argv = vec![
        vm.oo
            .define_name
            .as_ref()
            .expect("native OO Foundation define head")
            .clone(),
        native_context::object_name(vm, class_id),
        body.clone(),
    ];
    if let Err(completion) = native_context::enter_definition(vm, true, argv) {
        return completion;
    }
    let activation = vm.native_oo_variable_activation();
    vm.oo.def_stack.push((
        DefTarget::Class(class_id),
        activation,
        caller,
        caller_activation,
    ));
    let source = match vm.native_name_operand_bytes(body) {
        Ok(bytes) => tcl_lexer::SourceImage::native(bytes.as_ref()),
        Err(error) => {
            vm.oo.def_stack.pop();
            native_context::leave_definition(vm);
            return vm.refuse_host_command(error.to_string());
        }
    };
    let res = match vm.eval_source_image_at_internal(&source, body.source_location()) {
        Ok(c) if c.code == Code::Return => ok(c.result),
        Ok(c) if c.code == Code::Error => {
            append_define_frame(vm, true, class_id, &c.result.string_bytes());
            c
        }
        Ok(c) => c,
        Err(e) => definition_error(vm, true, class_id, e),
    };
    vm.oo.def_stack.pop();
    native_context::leave_definition(vm);
    res
}

#[cfg(test)]
mod native_byte_name_tests {
    use super::*;
    use tcl_core_types::ROOT_NS;

    #[test]
    fn declared_variables_retain_original_values_and_counted_object_fields() {
        for version in [
            tcl_dialect::TclVersion::V8_6,
            tcl_dialect::TclVersion::V9_0,
            tcl_dialect::TclVersion::V9_1,
        ] {
            let profile = crate::environment::profile_for_dialect(version.dialect_profile_name());
            let mut vm = Vm::with_output(Box::new(std::io::sink()));
            vm.set_dialect_profile(profile);
            vm.set_compiler(Box::new(
                tcl_compiler::compile_service::BytecodeCompileService::for_profile(profile),
            ));
            let root = vm_class_root(&vm);
            assert_eq!(make_class_bytes(&mut vm, b"C", None, root).code, Code::Ok);
            let class = Value::string("C");
            let original = Value::from_native_string_bytes(b"a\0z".as_slice());
            let declaration = vm.invoke_command(
                "oo::define",
                &[class.clone(), Value::string("variable"), original.clone()],
            );
            assert_eq!(declaration.code, Code::Ok);
            let listed = vm.invoke_command(
                "info",
                &[
                    Value::string("class"),
                    Value::string("variables"),
                    class.clone(),
                ],
            );
            assert_eq!(listed.code, Code::Ok);
            let listed = listed.result.as_list().unwrap();
            assert_eq!(listed.len(), 1);
            assert_eq!(
                listed[0].native_object_identity(),
                original.native_object_identity()
            );
            assert_eq!(listed[0].string_bytes().as_ref(), b"a\0z");
            let mut body = b"set {".to_vec();
            body.extend_from_slice(b"a\0z");
            body.extend_from_slice(b"} HIT; return [info object vars [self]]");
            let definition = vm.invoke_command(
                "oo::define",
                &[
                    class.clone(),
                    Value::string("method"),
                    Value::string("write"),
                    Value::empty(),
                    Value::from_native_string_bytes(body),
                ],
            );
            assert_eq!(definition.code, Code::Ok);
            assert_eq!(
                vm.invoke_command("C", &[Value::string("create"), Value::string("o")])
                    .code,
                Code::Ok
            );
            let fields = vm.invoke_command("o", &[Value::string("write")]);
            assert_eq!(
                fields.code,
                Code::Ok,
                "{} {:?}",
                profile.name,
                fields.result.string_bytes()
            );
            let fields = fields.result.as_list().unwrap();
            assert_eq!(fields.len(), 1);
            assert_eq!(fields[0].string_bytes().as_ref(), b"a\0z");
            let rejected = vm.invoke_command(
                "oo::define",
                &[
                    class.clone(),
                    Value::string("variable"),
                    Value::from_native_string_bytes(b"bad::name".as_slice()),
                ],
            );
            assert_eq!(rejected.code, Code::Error);
            let retained = vm.invoke_command(
                "info",
                &[Value::string("class"), Value::string("variables"), class],
            );
            assert_eq!(retained.result.as_list().unwrap().len(), 1);
        }
    }

    #[test]
    fn native_object_and_method_keys_keep_independent_extents_and_original_usage() {
        for version in [
            tcl_dialect::TclVersion::V8_6,
            tcl_dialect::TclVersion::V9_0,
            tcl_dialect::TclVersion::V9_1,
        ] {
            let profile = crate::environment::profile_for_dialect(version.dialect_profile_name());
            let mut vm = Vm::with_output(Box::new(std::io::sink()));
            vm.set_dialect_profile(profile);
            vm.set_compiler(Box::new(
                tcl_compiler::compile_service::BytecodeCompileService::for_profile(profile),
            ));
            let class = Value::from_native_string_bytes(b"C\xff".as_slice());
            let metaclass = vm_class_root(&vm);
            let created = make_class_bytes(&mut vm, b"C\xff", None, metaclass);
            assert_eq!(created.code, Code::Ok, "{}", profile.name);
            assert_eq!(created.result.string_bytes().as_ref(), b"::C\xff");
            for method in [
                b"a\0z".as_slice(),
                b"a\xffz".as_slice(),
                b"a\xc0\x80z".as_slice(),
            ] {
                let original = Value::from_native_string_bytes(method);
                let defined = vm.invoke_command(
                    "oo::define",
                    &[
                        class.clone(),
                        Value::string("method"),
                        original.clone(),
                        Value::empty(),
                        Value::string("return HIT"),
                    ],
                );
                assert_eq!(defined.code, Code::Ok, "{} {method:?}", profile.name);
                let key = vm
                    .oo
                    .method_keys
                    .get(method)
                    .expect("exact counted method key");
                assert_eq!(vm.oo.method_name_bytes(key), method);
            }
            let object = Value::from_native_string_bytes(b"O\xff".as_slice());
            let created = vm.invoke_command_value_at(
                ROOT_NS,
                &class,
                &[Value::string("create"), object.clone()],
                &[],
                tcl_registry::command_lookup::CommandLookupOrigin::Ordinary,
            );
            assert_eq!(created.code, Code::Ok);
            assert_eq!(created.result.string_bytes().as_ref(), b"::O\xff");
            for method in [
                b"a\0z".as_slice(),
                b"a\xffz".as_slice(),
                b"a\xc0\x80z".as_slice(),
            ] {
                let hit = vm.invoke_command_value_at(
                    ROOT_NS,
                    &object,
                    &[Value::from_native_string_bytes(method)],
                    &[],
                    tcl_registry::command_lookup::CommandLookupOrigin::Ordinary,
                );
                assert_eq!(hit.code, Code::Ok, "{} {method:?}", profile.name);
                assert_eq!(hit.result.string_bytes().as_ref(), b"HIT");
            }
            let miss = vm.invoke_command_value_at(
                ROOT_NS,
                &object,
                &[Value::string("a")],
                &[],
                tcl_registry::command_lookup::CommandLookupOrigin::Ordinary,
            );
            assert_eq!(miss.code, Code::Error);
            let raw_name = Value::from_native_string_bytes(b"N\0z".as_slice());
            let created = vm.invoke_command_value_at(
                ROOT_NS,
                &class,
                &[Value::string("create"), raw_name.clone()],
                &[],
                tcl_registry::command_lookup::CommandLookupOrigin::Ordinary,
            );
            assert_eq!(created.code, Code::Ok);
            assert_eq!(created.result.string_bytes().as_ref(), b"::N");
            let invocation = vm.invoke_command_value_at(
                ROOT_NS,
                &raw_name,
                &[],
                &[],
                tcl_registry::command_lookup::CommandLookupOrigin::Ordinary,
            );
            assert_eq!(invocation.code, Code::Error);
            // C9 uses native list quoting for usage; the original input remains counted.
            assert_eq!(raw_name.string_bytes().as_ref(), b"N\0z");
            assert!(
                invocation
                    .result
                    .string_bytes()
                    .as_ref()
                    .windows(3)
                    .any(|part| part == b"N\0z")
            );
        }
    }

    fn vm_class_root(vm: &Vm) -> OoId {
        vm.oo.class_root.expect("native TclOO root")
    }
}

#[cfg(test)]
mod original_call_argv_tests {
    use super::*;
    use std::cell::RefCell;
    struct Observe(Rc<RefCell<Vec<Vec<usize>>>>);
    impl crate::command::NativeCommand for Observe {
        fn invoke(&self, vm: &mut Vm, _arguments: &[Value]) -> Completion<Value> {
            let words = vm.frame_argv(vm.current_level()).expect("active OO frame");
            self.0
                .borrow_mut()
                .push(words.iter().map(Value::native_object_identity).collect());
            ok(Value::empty())
        }
    }
    fn instance(
        version: tcl_dialect::TclVersion,
        setup: &str,
    ) -> (Vm, Rc<RefCell<Vec<Vec<usize>>>>) {
        let profile = crate::environment::profile_for_dialect(version.dialect_profile_name());
        let mut vm = Vm::with_native_core(
            Box::new(std::io::sink()),
            Rc::new(crate::host_native::NativeHost::new()),
            profile,
            tcl_registry::special_vars::NativeBootstrapInputs {
                package_path: Vec::new(),
                default_library: None,
            },
        )
        .expect("selected actual C constructor");
        vm.set_compiler(Box::new(
            tcl_compiler::compile_service::BytecodeCompileService::for_profile(profile),
        ));
        let observed = Rc::new(RefCell::new(Vec::new()));
        vm.register_command_in_slot(
            tcl_runtime_api::CommandSlot {
                namespace: tcl_core_types::ROOT_NS,
                simple: NameBytes::from(b"observe_original".as_slice()),
            },
            Command::Native(Rc::new(Observe(Rc::clone(&observed)))),
        );
        let result = vm.eval_source(setup).unwrap();
        assert_eq!(result.code, Code::Ok, "{:?}", result.result.string_bytes());
        (vm, observed)
    }
    #[test]
    fn constructor_and_method_frames_borrow_original_dispatch_children() {
        for version in [
            tcl_dialect::TclVersion::V8_6,
            tcl_dialect::TclVersion::V9_0,
            tcl_dialect::TclVersion::V9_1,
        ] {
            for constructor in [false, true] {
                let setup = if constructor {
                    "oo::class create C {constructor {x} {observe_original; error FAILURE}}"
                } else {
                    "oo::class create C {method m {x} {observe_original; error FAILURE}}; C create o"
                };
                let (mut vm, observed) = instance(version, setup);
                let head =
                    Value::new_native_string_bytes(if constructor { &b"C"[..] } else { &b"o"[..] });
                let argument = Value::native_list_constructor(
                    vec![Value::string("ORIGINAL")],
                    tcl_syntax::native_string::NativeStringProtocol::C(version),
                );
                let arguments = if constructor {
                    vec![Value::string("create"), Value::string("obj"), argument]
                } else {
                    vec![Value::string("m"), argument]
                };
                let expected: Vec<_> = std::iter::once(&head)
                    .chain(&arguments)
                    .map(Value::native_object_identity)
                    .collect();
                let result = vm.invoke_host_original_object_vector(&head, &arguments);
                assert_eq!(result.code, Code::Error);
                assert_eq!(observed.borrow().as_slice(), &[expected.clone()]);
                let stack = vm.error_stack_value();
                let flat = stack.as_list().unwrap();
                let calls: Vec<_> = flat
                    .chunks_exact(2)
                    .filter(|pair| pair[0].string_bytes().as_ref() == b"CALL")
                    .map(|pair| {
                        pair[1]
                            .as_list()
                            .unwrap()
                            .iter()
                            .map(Value::native_object_identity)
                            .collect::<Vec<_>>()
                    })
                    .collect();
                assert_eq!(calls, vec![expected]);
            }
        }
    }
    #[test]
    fn next_nextto_and_destructor_frames_use_native_invocation_vectors() {
        for version in [
            tcl_dialect::TclVersion::V8_6,
            tcl_dialect::TclVersion::V9_0,
            tcl_dialect::TclVersion::V9_1,
        ] {
            for next in ["next $x", "nextto Base $x"] {
                let setup = format!(
                    "oo::class create Base {{method m {{x}} {{observe_original; error FAILURE}}}}; oo::class create Derived {{superclass Base; method m {{x}} {{{next}}}}}; Derived create o"
                );
                let (mut vm, observed) = instance(version, &setup);
                let argument = Value::native_list_constructor(
                    vec![Value::string("ORIGINAL")],
                    tcl_syntax::native_string::NativeStringProtocol::C(version),
                );
                let identity = argument.native_object_identity();
                let result = vm.invoke_host_original_object_vector(
                    &Value::string("o"),
                    &[Value::string("m"), argument],
                );
                assert_eq!(result.code, Code::Error);
                let observed = observed.borrow();
                assert_eq!(observed.len(), 1);
                assert_eq!(
                    observed[0].len(),
                    if next.starts_with("nextto") { 3 } else { 2 }
                );
                assert_eq!(observed[0].last(), Some(&identity));
            }
            let (mut vm, observed) = instance(
                version,
                "oo::class create C {destructor {observe_original; error FAILURE}}; C create o",
            );
            let result = vm.invoke_host_original_object_vector(
                &Value::string("o"),
                &[Value::string("destroy")],
            );
            assert_eq!(result.code, Code::Error);
            assert_eq!(observed.borrow().as_slice(), &[Vec::<usize>::new()]);
            let stack = vm.error_stack_value();
            assert!(
                stack
                    .as_list()
                    .unwrap()
                    .chunks_exact(2)
                    .all(|pair| pair[0].string_bytes().as_ref() != b"CALL")
            );
        }
    }
}

mod native_properties;
