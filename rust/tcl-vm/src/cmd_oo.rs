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

use std::collections::{BTreeMap, BTreeSet};

use tcl_registry::native_tcloo_method_definition::{
    NativeTclooMethodReportProtocol, NativeTclooMethodReportRole,
};

use std::cell::RefCell;
use std::rc::Rc;

mod method_preparation;
mod native_bootstrap;
pub(crate) mod native_context;
pub(crate) mod native_method_cache;
pub(crate) mod native_variables;
mod original_name_operands;
mod variable_mapping;

use tcl_core_types::NameBytes;
#[cfg(test)]
use tcl_registry::commands::tcl::InfoOoEnsembleKind;
use tcl_registry::commands::tcl::TclOoPropertyKind;
use tcl_runtime_api::{Code, Completion, NsId, OoId};

use crate::command::{Command, Param};
use crate::interp::{CommandSidecarKey, Vm, err, ok};
use crate::value::Value;

mod activation_commands;
mod configurable_properties;
mod construction;
mod declared_variables;
mod definition;
mod destruction;
mod introspection;
mod method_invocation;
mod method_resolution;

use activation_commands::{cmd_my, cmd_self};
pub(crate) use activation_commands::{
    cmd_next, cmd_next_original, cmd_nextto, cmd_nextto_original, current_object_name, in_method,
};
use configurable_properties::{
    apply_property_bytes, cmd_property, property_slot, slot_ref_class, slot_ref_obj,
};
use construction::fresh_obj_name;
#[cfg(test)]
use construction::make_class_bytes;
use declared_variables::private_storage_name_bytes;
use definition::{
    active_target, append_define_frame, cmd_oo_define, cmd_oo_objdefine, cmd_private, def_body_cmd,
    run_define,
};
pub(crate) use destruction::{
    oo_command_deleted, oo_command_exposed, oo_command_hidden, oo_command_renamed,
    oo_command_replaced, original_oo_rename,
};
pub(crate) use introspection::{
    info_class, info_object, is_object, object_class_name, object_creation_id, object_key,
    object_namespace_name,
};
#[cfg(test)]
use method_invocation::builtin_method;
use method_invocation::{oo_invoke_value, oo_invoke_value_with_head};

#[derive(Clone)]
struct MethodParameters {
    values: Vec<Param>,
    has_args: bool,
}

/// A method (or constructor/destructor) body: a proc-like parameter list plus
/// an exact-object-namespace compiled body, and whether it is exported for public
/// (`$obj m`) dispatch. A class method has no single procedure namespace at
/// definition time, so compilation is deferred until an exact object-private
/// namespace is known. The cache is deliberately one bounded exact entry: an
/// object method is stable, while a shared class method recompiles when calls
/// move between object-private namespaces instead of retaining dead objects.
#[derive(Clone)]
struct Method {
    parameters: MethodParameters,
    compiled_body: Option<(String, crate::compiled::CompiledUnit)>,
    body_src: Value,
    /// `None` for an ordinary method; `Some(prefix)` for a `forward` (invoking
    /// the method evaluates `prefix args…`).
    forward: Option<Vec<Value>>,
    /// Native default accessor clientData, shared through reached Method snapshots.
    property: Option<Rc<native_properties::NativePropertyAccessor>>,
    /// Actual installed native method body, independent of its table spelling.
    intrinsic: Option<tcl_runtime_api::native_oo::NativeOoIntrinsicMethod>,
    /// A null-body visibility override continues to inherited method bodies.
    visibility_only: bool,
    /// Default public visibility (from the name / definition flags). The
    /// per-class/object `exported`/`unexported` override sets take precedence.
    exported: bool,
    /// True-private declarations are scoped to their declaring provider;
    /// ordinary unexported methods remain part of the general `my` chain.
    private: bool,
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
    /// The selected private map's actual stored full-name object reference.
    full_name: Option<DeclaredVariableFullName>,
}

/// Private map's actual constructor header and exact correspondence bytes.
#[derive(Clone)]
struct DeclaredVariableFullName {
    original: Value,
    name: NameBytes,
}
impl DeclaredVariable {
    /// A selected mapping keeps its stored private target across whole copies.
    /// Legacy authored records have no selected full-name role.
    fn private_storage(&self, creation: u64) -> NameBytes {
        self.full_name.as_ref().map_or_else(
            || private_storage_name_bytes(creation, self.name.as_bytes()),
            |full_name| full_name.name.clone(),
        )
    }
}

/// The class facet of an entity: what its *instances* inherit.
#[derive(Clone, Default)]
struct Class {
    /// Identity of this actual class facet; observational clones retain it.
    birth: Rc<()>,
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
    filters: Vec<NameBytes>,
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
    definition_namespace: Option<NsId>,
    object_definition_namespace: Option<NsId>,
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
    filters: Vec<NameBytes>,
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
    pub(crate) context_birth: Rc<()>,
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
struct FoundationMethodReports {
    constructor: Value,
    destructor: Value,
}
impl FoundationMethodReports {
    fn new(protocol: NativeTclooMethodReportProtocol) -> Self {
        Self {
            constructor: Value::new_native_string_bytes(
                protocol
                    .foundation_name(NativeTclooMethodReportRole::Constructor)
                    .expect("selected constructor literal"),
            ),
            destructor: Value::new_native_string_bytes(
                protocol
                    .foundation_name(NativeTclooMethodReportRole::Destructor)
                    .expect("selected destructor literal"),
            ),
        }
    }
    fn report(&self, role: NativeTclooMethodReportRole) -> Option<Value> {
        match role {
            NativeTclooMethodReportRole::Named => None,
            NativeTclooMethodReportRole::Constructor => Some(self.constructor.clone()),
            NativeTclooMethodReportRole::Destructor => Some(self.destructor.clone()),
        }
    }
}

#[derive(Default)]
pub(crate) struct OoState {
    native_methods: native_method_cache::NativeMethodWorld,
    method_reports: Option<FoundationMethodReports>,
    property_foundation_epoch: u64,
    property_my_name: Option<Value>,
    property_members: BTreeMap<(OoId, bool, bool), BTreeMap<String, Value>>,
    property_caches: BTreeMap<(OoId, bool), native_properties::NativePropertyCache>,
    /// Originating pure release recipe, independently of the current profile.
    property_origins:
        BTreeMap<(OoId, bool), tcl_registry::native_property_lookup::NativePropertyLookupProtocol>,

    /// Matching old-visible declaration retirement tickets, never lookup authority.
    variable_mapping_retirements: BTreeMap<(OoId, bool, bool), std::rc::Weak<()>>,
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
    configurable_support: Option<OoId>,
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
    /// Original single-directive invocation words for native usage rewriting.
    definition_usage: native_context::DefinitionUsage,
}

/// The per-flow OO execution stacks — the subset of [`OoState`] that a coroutine
/// owns and that is swapped in/out on suspend/resume (the class/object
/// registries stay shared). Mirrors `runtime/rust/src/cmd_oo.rs`'s `OoExec`.
#[derive(Default)]
pub(crate) struct OoExec {
    call_stack: Vec<OoFrame>,
    def_stack: Vec<(DefTarget, u64, NsId, u64)>,
    private_depth: usize,
    definition_usage: native_context::DefinitionUsage,
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
        std::mem::swap(&mut self.definition_usage, &mut e.definition_usage);
    }
}

/// Render a canonical FQN in display (`::`-qualified) form.
fn display(canonical: &str) -> String {
    tcl_syntax::naming::root_unrooted_key(canonical)
}

/// Resolve a class *reference* (`superclass`, `mixin`, `nextto`, `info … isa`)
/// through original command lookup in the actual outer namespace. A method's
/// private object namespace cannot replace that class-reference context.
fn resolve_object_value(vm: &mut Vm, name: &Value) -> Result<Option<OoId>, Completion<Value>> {
    let selected = vm
        .lookup_original_command_at(native_context::outer_namespace(vm), name)
        .map_err(|error| crate::command::completion_from_cmd_error(vm, error))?;
    Ok(match selected {
        Some((_, Command::Object(id))) if vm.oo.objects.contains_key(&id) => Some(id),
        _ => None,
    })
}

fn resolve_class_value(vm: &mut Vm, name: &Value) -> Result<Option<OoId>, Completion<Value>> {
    Ok(resolve_object_value(vm, name)?.filter(|id| vm.oo.classes.contains_key(id)))
}

fn display_oo_bytes(vm: &Vm, id: OoId) -> Vec<u8> {
    tcl_syntax::naming::root_unrooted_key_bytes(
        vm.oo
            .names
            .get(&id)
            .expect("live object retains exact command reporting name")
            .as_bytes(),
    )
}

fn object_message(vm: &Vm, id: OoId, before: &[u8], after: &[u8]) -> Vec<u8> {
    let mut message = before.to_vec();
    message.extend_from_slice(&display_oo_bytes(vm, id));
    message.extend_from_slice(after);
    message
}

fn native_method_key(vm: &mut Vm, original: &Value) -> Result<String, Completion<Value>> {
    let target = active_target(vm)
        .or_else(|| native_context::current_method(vm).map(|frame| (false, frame.object)));
    native_method_key_for_target(vm, original, target)
}
fn native_method_key_for_target(
    vm: &mut Vm,
    original: &Value,
    target: Option<(bool, OoId)>,
) -> Result<String, Completion<Value>> {
    original_name_operands::run_result(vm, target, |vm, original_names| {
        let bytes = original_names
            .read(vm, original)
            .map_err(|error| crate::command::completion_from_cmd_error(vm, error))?;
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
    })
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
    for identity in [
        "oo::configuresupport::configurableclass::property",
        "oo::configuresupport::configurableobject::property",
    ] {
        if tcl_registry::native_tcloo_registration::compilation(
            identity,
            vm.actual_native_invocation_dialect(),
        )
        .is_some()
        {
            vm.register_stock_builtin(identity, cmd_property);
        }
    }
    bootstrap(vm);
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

/// Seed the two root classes `::oo::object` and `::oo::class`. Each is a class,
/// an object (instance of `::oo::class`), and a command.
fn foundation_object(class: OoId, namespace: String, identity: OoId) -> Object {
    Object {
        cached_name: Rc::new(RefCell::new(None)),
        class,
        ns: namespace,
        namespace_token: None,
        creation_id: identity.0,
        methods: native_method_cache::MethodTable::default(),
        variables: Vec::new(),
        private_variables: Vec::new(),
        mixins: Vec::new(),
        filters: Vec::new(),
        exported: BTreeSet::new(),
        unexported: BTreeSet::new(),
        readable_properties: BTreeSet::new(),
        writable_properties: BTreeSet::new(),
        destroyed: false,
    }
}

fn bootstrap(vm: &mut Vm) {
    vm.oo.method_reports =
        NativeTclooMethodReportProtocol::select(vm.actual_native_invocation_dialect())
            .map(FoundationMethodReports::new);
    // TclOO is compiled into the VM, so — like C Tcl's core at startup — seed
    // the `::oo` version variables from the release's `tclOO.h`. The `TclOO` /
    // `tcl::oo` package entries are release data, provided with the core's own
    // (`provide_core_packages`).
    vm.declare_namespace("::oo");
    if let Some((version, patchlevel)) = vm
        .actual_native_invocation_dialect()
        .tcl_version
        .and_then(tcl_dialect::TclVersion::tcloo_version)
    {
        let _ = vm.set_var("::oo::version", Value::string(version));
        let _ = vm.set_var("::oo::patchlevel", Value::string(patchlevel));
    }

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
        let private_namespace = fresh_obj_name(vm);
        vm.declare_namespace_key(&private_namespace);
        vm.oo.classes.insert(root_id, Class::default());
        vm.oo.objects.insert(
            root_id,
            foundation_object(class_root, private_namespace.clone(), root_id),
        );
        let namespace_token = native_context::install_object_helpers(vm, &private_namespace);
        vm.oo.objects.get_mut(&root_id).unwrap().namespace_token = Some(namespace_token);
        let Some(command_key) = vm
            .register_command(root, Command::Object(root_id))
            .into_key()
        else {
            return;
        };
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
    let configurable_namespace = fresh_obj_name(vm);
    vm.declare_namespace_key(&configurable_namespace);
    vm.oo.objects.insert(
        configurable_root,
        foundation_object(
            class_root,
            configurable_namespace.clone(),
            configurable_root,
        ),
    );
    vm.declare_namespace("oo::configurable");
    let namespace_token = native_context::install_object_helpers(vm, &configurable_namespace);
    vm.oo
        .objects
        .get_mut(&configurable_root)
        .unwrap()
        .namespace_token = Some(namespace_token);
    let Some(command_key) = vm
        .register_command("oo::configurable", Command::Object(configurable_root))
        .into_key()
    else {
        return;
    };
    vm.oo
        .command_keys
        .insert(configurable_root, CommandSidecarKey::visible(command_key));
    // TIP 558 is Tcl 9.0: real tclsh 8.6.16 has no `oo::configurable`.
    vm.declare_registry_object_root("oo::configurable");
    native_bootstrap::install(vm);
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
    let scope = match crate::interp::native_operation_currency::NativeOperationScope::enter(vm) {
        Ok(scope) => scope,
        Err(error) => return crate::command::completion_from_cmd_error(vm, error),
    };
    let reached = oo_dispatch_entered(vm, key, invoked, argv);
    match scope.check(vm) {
        Ok(()) => reached,
        Err(error) => crate::command::completion_from_cmd_error(vm, error),
    }
}

fn oo_dispatch_entered(
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
    let method = match native_method_key_for_target(vm, &argv[0], Some((false, key))) {
        Ok(key) => key,
        Err(completion) => return completion,
    };
    let rest = &argv[1..];
    oo_invoke_value(vm, key, &method, &argv[0], rest, true, invoked)
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

#[cfg(test)]
mod native_eval_tests;

#[cfg(test)]
mod original_bootstrap_namespace_tests;

mod native_properties;

/// Same-entry OO observation captured from actual backend allocations and tables.
pub(crate) fn native_bootstrap_inventory(
    vm: &Vm,
    entry: &tcl_runtime_api::NativeCompilationEntry,
) -> Option<tcl_runtime_api::native_oo::NativeOoClassInventory> {
    native_bootstrap::capture(vm, entry)
}

#[cfg(test)]
mod native_info_option_tests;
