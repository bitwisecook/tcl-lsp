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

//! TclOO — the object system (`oo::class`, `oo::object`, `oo::define`, …).
//!
//! Covers (C ref `tclOO.c`): classes with single/multiple superclasses,
//! methods (incl. `forward`), constructor/destructor, instance variables,
//! object creation (`new`/`create`), per-object definitions (`oo::objdefine`,
//! per-object methods/mixins), class/object `mixin`s, `export`/`unexport`
//! visibility, `oo::copy`, method dispatch over a linearised chain (object →
//! object mixins → class mixins → class MRO), the method-context commands
//! `self`/`my`/`next`, `filter`s, classes-as-objects (`self` class methods), and
//! `info object`/`info class` introspection.
//!
//! The call chain is a list of [`CallStep`]s (`provider`, `method`); filters are
//! steps whose method is the filter name, prepended ahead of the target-method
//! steps, with `next` advancing through the chain. Each object/class is a
//! command (`Command::OoObject`); each object's instance variables live in a
//! private namespace, auto-linked into a method frame from the class's
//! `variable` declarations (reusing the proc machinery via [`Interp::run_proc`]
//! with `CallMeta::link_vars`).
//!
//! Declaration records retain their original objects and full counted names.
//! Native slot operations, validation and diagnostic extents are shared with
//! the syntax owner; method frames bind each field to its retained namespace.
//!
//! See `list.rs` for the module-level `not_unsafe_ptr_arg_deref` rationale.
#![allow(clippy::not_unsafe_ptr_arg_deref)]

/// Borrowed invocation views; the existing OriginalArguments owns validation.
struct OriginalOoInvocationView<'a> {
    invoked: Option<&'a [u8]>,
    original_argv: Option<&'a [*mut TclObj]>,
}

type OriginalOoStageCheck<'a> = &'a dyn Fn(&Interp) -> Result<(), tcl_cmd_core::CmdError>;

mod native_context;
pub(crate) mod native_method_cache;
mod native_object_info;
pub(crate) mod native_variables;
#[cfg(test)]
mod original_active_frame_tests;
#[cfg(test)]
mod original_call_report_tests;
#[cfg(test)]
mod original_classvariable_tests;
#[cfg(test)]
mod original_object_resolution_tests;
#[cfg(test)]
mod original_property_reporting_tests;
#[cfg(test)]
mod original_property_retirement_tests;
#[cfg(test)]
mod original_publication_tests;
#[cfg(test)]
mod original_record_retirement_tests;

use tcl_native_model::native_tcloo_method_definition::{
    NativeTclooMethodPayloadType, NativeTclooMethodReportProtocol, NativeTclooMethodReportRole,
};

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use tcl_core_types::{OoId, RecursionLimit};
#[cfg(test)]
use tcl_registry::commands::tcl::InfoOoEnsembleKind;

#[cfg(all(test, debug_assertions))]
use crate::interp::obj_bytes;
use crate::interp::{Code, Command, Interp};
use crate::list;
use crate::namespace::{NsId, GLOBAL};
use crate::obj::{self, TclObj};
use tcl_syntax::value::ValueOps;

/// Maximum superclass/mixin linearisation depth for [`Interp::linearize_class`]
/// / [`Interp::gather_class_props`]. `tcl_syntax::mro::MAX_MRO_DEPTH` fixes
/// the identical algorithm (TclOO's DFS + late-placement) in the
/// *diagnostics* lineariser, settling on 1024 there — but that pass runs on
/// a host-controlled analysis stack, not this runtime's live call stack.
/// Confirmed crash reproduction: a deep `mixin` chain (`oo::class create
/// C$i { mixin C[i-1] }`) SIGABRTs between depth 100-150 on a 256 KiB
/// stack, and still crashes at depth 2000 on a 1 MiB stack, so 1024 would
/// not actually stop the crash on a small-stack embedding. 64 — the same
/// cap `MAX_ARRAY_INDEX_DEPTH` /
/// `MAX_SCAN_PARTS_DEPTH` / `MAX_RESOLVE_PARTS_DEPTH` settled on for the same
/// crash class elsewhere in this crate — is comfortably under the measured
/// 100-150 floor, with margin for a smaller WASM host stack, and still far
/// past any real class hierarchy.
const MAX_MRO_DEPTH: RecursionLimit = RecursionLimit(64);

/// Maximum total mixin/superclass node visits across one linearisation call
/// ([`Interp::class_precedence`]'s `linearize_class` walk, or one
/// `Interp::class_property_list`'s `gather_class_props` walk). Mirrors
/// `tcl_syntax::mro::MAX_MRO_VISITS`: `linearize_class`'s cycle guard
/// (`path`) only blocks a class currently on the active DFS branch, so a
/// class reachable via multiple sibling branches (a diamond) is deliberately
/// re-explored once per reaching path — needed for the caller's keep-last
/// dedup to match TclOO's real "as late as possible" placement — which makes
/// a hierarchy of `k` stacked diamonds cost Θ(2^k) calls. Depth alone would
/// not bound that (a diamond is wide, not deep); this second, independent
/// cap does.
const MAX_MRO_VISITS: u32 = 200_000;

macro_rules! oo_original_bytes {
    ($interp:expr, $originals:expr, $index:expr) => {
        match $originals.bytes($index) {
            Ok(bytes) => bytes.to_vec(),
            Err(error) => return $interp.report_cmd_error(error),
        }
    };
}

macro_rules! oo_checked {
    ($interp:expr, $result:expr) => {
        match $result {
            Ok(value) => value,
            Err(error) => return $interp.report_cmd_error(error.into()),
        }
    };
}

macro_rules! oo_original_object {
    ($interp:expr, $originals:expr, $index:expr) => {{
        let original = oo_checked!($interp, $originals.original($index));
        oo_checked!($interp, $interp.oo_resolve_original_object(original))
    }};
}

macro_rules! oo_original_check {
    ($interp:expr, $originals:expr) => {
        if let Err(error) = $originals.check() {
            return $interp.report_cmd_error(error);
        }
    };
}

mod configurable;
mod copying;
mod definition;
mod engine;
mod installation;
mod introspection;
mod method_context;
mod slots;

#[cfg(test)]
use configurable::native_method_body;
use configurable::property_loopword_error;
#[cfg(test)]
use copying::oo_copy_cmd;
#[cfg(all(test, debug_assertions))]
use definition::{def_class, def_definitionnamespace, oo_define_cmd};
#[cfg(test)]
use definition::{
    def_constructor, def_deletemethod, def_export, def_forward, def_method, def_renamemethod,
};
pub use installation::install;
#[cfg(test)]
use introspection::list_procedure_body;
pub(crate) use introspection::{info_class, info_object};
#[cfg(test)]
use method_context::{classvariable_cmd, nextto_cmd};
pub(crate) use method_context::{my_cmd, myclass_cmd, object_namespace_command_names};
use method_context::{next_cmd, nextto_cmd_using, self_cmd};
#[cfg(test)]
use slots::{prepare_declared_variables, validate_declared_variables, write_declared_variables};

/// A native method handler `(interp, object, args) -> Code`, invoked without a
/// Tcl call frame (so `info level` inside any method it calls is unchanged) —
/// used for the `::oo::Slot` operations.
type NativeMethod = fn(&mut Interp, OoId, &[*mut TclObj]) -> Code;

/// A method: a normal body, a `forward` to a command prefix, or a native handler.
#[derive(Clone)]
enum Method {
    Body {
        procedure: crate::interp::NativeCallableProcedure,
        report_type: NativeTclooMethodPayloadType,
    },
    Forward {
        prefix: obj::Owned,
    },
    Builtin {
        handler: NativeMethod,
        report_type: NativeTclooMethodPayloadType,
    },
    /// Native C91 GetterType/SetterType clientData without a Tcl proc frame.
    Property(Rc<native_properties::NativePropertyAccessor>),
}

impl Method {
    fn builtin(handler: NativeMethod, report_type: NativeTclooMethodPayloadType) -> Self {
        Self::Builtin {
            handler,
            report_type,
        }
    }
    fn report_type(&self) -> NativeTclooMethodPayloadType {
        match self {
            Self::Body { report_type, .. } | Self::Builtin { report_type, .. } => *report_type,
            Self::Forward { .. } => NativeTclooMethodPayloadType::Forward,
            Self::Property(accessor) => {
                if accessor.writable {
                    NativeTclooMethodPayloadType::PropertySetter
                } else {
                    NativeTclooMethodPayloadType::PropertyGetter
                }
            }
        }
    }
    /// DetailsCloner acquires a clientData role only for a new declaration.
    /// Reached/cache snapshots share the current native method payload instead.
    fn duplicate_native_payload(&self) -> Result<Self, tcl_syntax::value::ValueError> {
        match self {
            Self::Property(accessor) => Ok(Self::Property(Rc::new(
                native_properties::NativePropertyAccessor::new(
                    accessor.original.as_ptr(),
                    accessor.writable,
                ),
            ))),
            Self::Body {
                procedure,
                report_type,
            } => Ok(Self::Body {
                procedure: procedure.duplicate_method()?,
                report_type: *report_type,
            }),
            _ => Ok(self.clone()),
        }
    }
}

/// An original declaration object and its full counted native table key.
#[derive(Clone)]
enum DeclaredVariableHeader {
    Stored(obj::Owned),
    Observed(obj::NativeObjectLifetime),
}
impl DeclaredVariableHeader {
    fn as_ptr(&self) -> *mut TclObj {
        match self {
            Self::Stored(value) => value.as_ptr(),
            Self::Observed(value) => value.as_ptr(),
        }
    }
    fn observation(&self) -> Self {
        match self {
            Self::Stored(value) => {
                Self::Observed(obj::NativeObjectLifetime::retain(value.as_ptr()))
            }
            Self::Observed(value) => Self::Observed(value.clone()),
        }
    }
    fn into_stored(self) -> obj::Owned {
        match self {
            Self::Stored(value) => value,
            Self::Observed(_) => unreachable!("neutral slot record owns its actual reference"),
        }
    }
}
#[derive(Clone)]
struct DeclaredVariableFullName {
    original: DeclaredVariableHeader,
    name: Vec<u8>,
}
#[derive(Clone)]
struct DeclaredVariable {
    original: DeclaredVariableHeader,
    name: Vec<u8>,
    full_name: Option<DeclaredVariableFullName>,
}
impl DeclaredVariable {
    fn check_originals(&self) -> Result<(), tcl_syntax::value::ValueError> {
        obj::check_native_liveness(self.original.as_ptr())?;
        if let Some(full) = &self.full_name {
            obj::check_native_liveness(full.original.as_ptr())?;
        }
        Ok(())
    }
}

/// A class definition.
#[derive(Default, Clone)]
struct Class {
    /// Actual class-facet birth; a shallow observation shares this owner.
    facet_birth: Rc<()>,
    /// Stable superclass identities (defaults to `::oo::object`).
    supers: Vec<OoId>,
    methods: native_method_cache::MethodTable,
    constructor: native_method_cache::MethodSlot,
    destructor: native_method_cache::MethodSlot,
    /// Declared instance-variable names (auto-linked into every method frame).
    variables: Vec<DeclaredVariable>,
    /// TIP 500 private instance variables (`private variable`): listed by
    /// `info class variables -private`, hidden from the plain form.
    private_variables: Vec<DeclaredVariable>,
    /// TIP 558 readable / writable property name sets (stored uniqued,
    /// first-wins; sorted on introspection).
    readable_properties: Vec<Vec<u8>>,
    writable_properties: Vec<Vec<u8>>,
    /// Mixed-in classes (searched before the superclass MRO).
    mixins: Vec<OoId>,
    /// Methods marked non-exported (callable only via `my`).
    unexported: BTreeSet<Vec<u8>>,
    /// Names explicitly `export`ed — overrides the default-unexported built-in
    /// methods (`eval`/`variable`/`varname`/…) so they become publicly callable.
    exported: BTreeSet<Vec<u8>>,
    /// TIP 500 *private* methods (`private method`/`method -private`): a subset
    /// of the unexported methods that are additionally hidden from
    /// `info methods -private` and from subclasses' `my` dispatch.
    private: BTreeSet<Vec<u8>>,
    /// Filter method names applied to instances' method calls (`filter`).
    filters: Vec<Vec<u8>>,
    /// The namespace in which this class's *instances* are defined
    /// (`definitionnamespace`, TIP 524); `info class definitionnamespace
    /// … -instance`. `None` → the empty default. Used as the resolution
    /// namespace for an `oo::objdefine` body on an instance of this class.
    def_ns: Option<Vec<u8>>,
    /// The namespace in which *this class itself* is defined (`info class
    /// definitionnamespace … -class`). When this class is a metaclass, it is
    /// the resolution namespace for an `oo::define` body on its instances.
    class_def_ns: Option<Vec<u8>>,
}

/// An object instance.
#[derive(Clone)]
struct Object {
    cached_name: Rc<RefCell<Option<obj::Owned>>>,
    /// Stable identity of the object's class.
    class: OoId,
    /// The namespace holding this object's instance variables.
    var_ns: NsId,
    /// A unique, monotonic creation identifier (`info object creationid`),
    /// stable across rename.
    creation_id: u64,
    /// Per-object methods (`oo::objdefine method`).
    methods: native_method_cache::MethodTable,
    /// Per-object mixins.
    mixins: Vec<OoId>,
    unexported: BTreeSet<Vec<u8>>,
    /// Explicitly `export`ed names (see `Class::exported`).
    exported: BTreeSet<Vec<u8>>,
    /// TIP 500 private methods (see `Class::private`).
    private: BTreeSet<Vec<u8>>,
    /// Per-object filter method names (`oo::objdefine filter`).
    filters: Vec<Vec<u8>>,
    /// Per-object declared instance variables (`oo::objdefine variable`).
    variables: Vec<DeclaredVariable>,
    /// Per-object TIP 500 private instance variables.
    private_variables: Vec<DeclaredVariable>,
    /// Per-object TIP 558 readable / writable property sets.
    readable_properties: Vec<Vec<u8>>,
    writable_properties: Vec<Vec<u8>>,
    /// Set once the destructor chain has started, so teardown via `destroy`,
    /// `rename obj {}` and `namespace delete` does not re-run it (C's
    /// `DESTRUCTOR_CALLED`).
    destroyed: bool,
    /// Set once the destructor has finished and the object's method/class
    /// structures are being dismantled, but the command is still resolvable
    /// (C's `OBJECT_DESTRUCTING` after `ObjectNamespaceDeleted` guts the object).
    /// A method call on a torn-down object yields "impossible to invoke method"
    /// rather than running anything — what a nested-owned child sees when its
    /// destructor calls back into the parent mid-teardown (oo-35.7.1/2, oo-11.8).
    torn_down: bool,
}

impl Object {
    fn new(class: OoId, var_ns: NsId, creation_id: u64) -> Self {
        Self {
            cached_name: Rc::new(RefCell::new(None)),
            class,
            var_ns,
            creation_id,
            methods: native_method_cache::MethodTable::default(),
            mixins: Vec::new(),
            unexported: BTreeSet::new(),
            exported: BTreeSet::new(),
            private: BTreeSet::new(),
            filters: Vec::new(),
            variables: Vec::new(),
            private_variables: Vec::new(),
            readable_properties: Vec::new(),
            writable_properties: Vec::new(),
            destroyed: false,
            torn_down: false,
        }
    }
}

/// One step of a method-call chain: the provider (object or class identity) and the
/// method name to run there. Filters appear as steps whose `method` is the
/// filter's name, ahead of the steps for the actually-invoked method.
#[derive(Clone)]
struct CallStep {
    method_owner: Option<native_method_cache::MethodOwner>,
    provider: OoId,
    method: Vec<u8>,
    /// Whether this step resolves against the *object* facet of `provider`
    /// (its per-object methods) rather than the *class* facet (its instance
    /// methods). Normally `provider == object`, but a class mixed into its own
    /// instance (`self mixin <self>`) contributes both facets as distinct steps,
    /// so the facet can't be re-derived from the name alone (oo-13.7/23.1/41.2).
    is_object: bool,
    is_filter: bool,
    synthetic_type: Option<NativeTclooMethodPayloadType>,
    declaring_facet: Option<std::rc::Weak<()>>,
}

/// One active method invocation (for `self` / `my` / `next`).
struct OoFrame {
    name_owner: Rc<RefCell<Option<obj::Owned>>>,
    activation: Option<u64>,
    object: OoId,
    /// The full call chain (filter steps, then the target-method steps).
    chain: native_method_cache::ReachedMethodChain,
    /// Index into `chain` of the step currently running.
    index: usize,
    /// The originally-invoked method (the filters' target; empty for a
    /// constructor). `self method` reports the *current* step's method; `self
    /// target` reports this.
    target: Vec<u8>,
    /// Whether the original invocation was external (`$obj m`) vs internal
    /// (`my m`) — so a built-in reached via `next` (e.g. `eval`) labels its
    /// error frame with the object name vs the literal `my`.
    external: bool,
}

/// A definition target: a class (`oo::define`) or an object (`oo::objdefine`).
#[derive(Clone)]
enum DefTarget {
    Class(OoId),
    Object(OoId),
}

/// Actual publication geometry; its report is retained separately by the OO identity.
struct ObjectPublication {
    namespace: NsId,
    simple: Vec<u8>,
    report: Vec<u8>,
}

/// Continuation of the actual allocated object and its instance namespace.
/// It observes existing owners without granting command or namespace authority.
struct OriginalObjectPublication {
    target: native_context::OriginalOoTarget,
    object: OoId,
    namespace: NsId,
    holder: tcl_runtime_api::native_namespace_name::NativeNamespaceNameToken,
}

impl OriginalObjectPublication {
    fn capture(
        interpreter: &mut Interp,
        object: OoId,
        currency: &crate::interp::native_operation_currency::NativeOperationCurrency,
    ) -> Result<Self, tcl_cmd_core::CmdError> {
        currency.ensure_current_or_refuse()?;
        let class = interpreter.oo.borrow().classes.contains_key(&object);
        let target = native_context::OriginalOoTarget::capture(interpreter, object, class)?;
        let namespace = interpreter.oo.borrow().objects[&object].var_ns;
        let holder = interpreter
            .native_namespace_token_at(namespace)
            .ok_or_else(|| {
                tcl_cmd_core::CmdError::from(
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "original OO publication namespace",
                    ),
                )
            })?;
        let original = Self {
            target,
            object,
            namespace,
            holder,
        };
        original.check(interpreter, currency)?;
        Ok(original)
    }

    fn check(
        &self,
        interpreter: &mut Interp,
        currency: &crate::interp::native_operation_currency::NativeOperationCurrency,
    ) -> Result<(), tcl_cmd_core::CmdError> {
        currency.ensure_current_or_refuse()?;
        self.target.check(interpreter)?;
        let same_namespace = interpreter
            .oo
            .borrow()
            .objects
            .get(&self.object)
            .is_some_and(|owner| owner.var_ns == self.namespace);
        let live_holder = {
            let namespaces = interpreter.namespaces();
            namespaces.owns_namespace_name_token(&self.holder)
                && self.holder.lifecycle()
                    == tcl_syntax::native_namespace_name::NativeNamespaceLifecycle::Live
                && namespaces.namespace_is_live(self.namespace)
        };
        if same_namespace && live_holder {
            Ok(())
        } else {
            let refusal =
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "original OO publication namespace retired",
                );
            interpreter.refuse_native_access(refusal);
            Err(tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(refusal).into())
        }
    }
}

struct FoundationMethodReports {
    constructor: obj::Owned,
    destructor: obj::Owned,
    unknown: obj::Owned,
}
impl FoundationMethodReports {
    fn new(protocol: NativeTclooMethodReportProtocol) -> Self {
        Self {
            unknown: obj::Owned::fresh(obj::new_string_bytes(b"unknown")),
            constructor: obj::Owned::fresh(obj::new_string_bytes(
                protocol
                    .foundation_name(NativeTclooMethodReportRole::Constructor)
                    .expect("selected constructor literal"),
            )),
            destructor: obj::Owned::fresh(obj::new_string_bytes(
                protocol
                    .foundation_name(NativeTclooMethodReportRole::Destructor)
                    .expect("selected destructor literal"),
            )),
        }
    }
    fn report(&self, role: NativeTclooMethodReportRole) -> Option<*mut TclObj> {
        match role {
            NativeTclooMethodReportRole::Named => None,
            NativeTclooMethodReportRole::Constructor => Some(self.constructor.as_ptr()),
            NativeTclooMethodReportRole::Destructor => Some(self.destructor.as_ptr()),
        }
    }
}

/// The interpreter's TclOO state.
#[derive(Default)]
pub struct OoState {
    native_methods: native_method_cache::NativeMethodWorld,
    method_reports: Option<FoundationMethodReports>,
    property_foundation_epoch: u64,
    variable_mapping_retirements: BTreeMap<(OoId, bool, bool), std::rc::Weak<()>>,
    property_my_name: Option<obj::Owned>,
    property_members: BTreeMap<(OoId, bool, bool), BTreeMap<Vec<u8>, obj::Owned>>,
    property_caches: BTreeMap<(OoId, bool), native_properties::NativePropertyCache>,
    /// Pure originating release recipe retained by the actual property producer.
    property_origins: BTreeMap<
        (OoId, bool),
        tcl_native_model::native_property_lookup::NativePropertyLookupProtocol,
    >,

    classes: BTreeMap<OoId, Class>,
    objects: BTreeMap<OoId, Object>,
    /// Current Tcl-facing command spelling, indexed one-way by stable token.
    names: BTreeMap<OoId, Vec<u8>>,
    object_root: Option<OoId>,
    class_root: Option<OoId>,
    configurable_root: Option<OoId>,
    next_oo_id: u64,
    counter: usize,
    /// Monotonic source of object creation IDs (`info object creationid`).
    next_id: u64,
    /// The Foundation's original constructor-to-define invocation head.
    define_name: Option<obj::Owned>,
    /// Original definition target, issued variable activation, creation identity
    /// and caller namespace. Nested procedures suspend this context; uplevel
    /// restores it only by selecting the retained activation.
    def_stack: Vec<(DefTarget, u64, Option<u64>, NsId, u64)>,
    call_stack: Vec<OoFrame>,
    /// Set while executing a filter (and everything it calls synchronously via
    /// `my`), so those nested calls are not re-wrapped by the same filters
    /// (C's `FILTER_HANDLING`). Cleared when a filter calls `next`, so the method
    /// it wraps runs with filters re-enabled for *its* own calls. Inherited down
    /// the call tree; save/restore at each boundary (oo-12.5/12.6/12.7).
    filter_handling: bool,
    /// Non-zero while inside a `private` definition modifier — methods defined
    /// then are marked unexported (callable only via `my`).
    private_depth: usize,
    /// The ensemble-rewrite prefix (`oo::define <class>` / `oo::objdefine
    /// <obj>`) active while dispatching the single-command form of a definition
    /// (`oo::define Foo method …`). A definition subcommand's `wrong # args`
    /// prepends it so the message names the whole original command, as C's
    /// ensemble rewrite does. `None` inside a `{ … }` definition body.
    def_rewrite: Option<Vec<u8>>,
    /// The caller's context scope (declaring class/object of the running method)
    /// for an in-progress unknown-method miss, captured at the original
    /// invocation so the `unknown`-handler frame does not mask it. Lets the
    /// unknown-method error list the in-scope private methods (TIP 500).
    unknown_scope: Option<(OoId, bool)>,
    /// Whether the unknown-method miss originated from an external (`$obj m`)
    /// call. An internal (`my m`) miss lists the unexported methods and the
    /// `oo::object` built-ins too; an external one lists only public methods.
    unknown_external: bool,
    /// The `wrong # args` command prefix a `forward` installs for the method it
    /// forwards to (the original invocation, e.g. `foo test`), consumed by the
    /// next method body's run_proc (C's ensemble-rewrite for `Tcl_WrongNumArgs`).
    fwd_usage: Option<Vec<u8>>,
}

impl OoState {
    fn allocate(&mut self, name: Vec<u8>) -> OoId {
        self.next_oo_id += 1;
        let id = OoId(self.next_oo_id);
        self.names.insert(id, name);
        id
    }

    fn name(&self, id: OoId) -> &[u8] {
        self.names.get(&id).map_or(b"", Vec::as_slice)
    }
}

fn oo_display(interp: &Interp, obj: OoId) -> Vec<u8> {
    let report = interp.oo_name(obj);
    tcl_syntax::naming::unroot_rooted_key_bytes(&report)
        .unwrap_or(&report)
        .to_vec()
}

/// Actual dispatch origin; a direct method named unknown does not issue fallback.
#[derive(Clone, Copy)]
struct MethodDispatch {
    external: bool,
    unknown: bool,
}

/// Exact borrowed command words for the actual method frame and CALL record.
struct MethodInvocation {
    external: bool,
    level_words: Option<Vec<Vec<u8>>>,
    // Borrowed pointer transport; the dispatch caller owns these objects.
    original_argv: Option<Vec<*mut TclObj>>,
}

impl MethodInvocation {
    fn internal() -> Self {
        Self {
            external: false,
            level_words: None,
            original_argv: Some(Vec::new()),
        }
    }

    fn with_words(external: bool, words: Vec<Vec<u8>>) -> Self {
        Self {
            external,
            level_words: Some(words),
            original_argv: None,
        }
    }
}

fn method_invocation(
    interp: &Interp,
    obj: OoId,
    invoked: Option<&[u8]>,
    method: &[u8],
    args: &[*mut TclObj],
    external: bool,
    original_argv: Option<&[*mut TclObj]>,
) -> Result<MethodInvocation, tcl_cmd_core::CmdError> {
    if let Some(argv) = original_argv {
        return Ok(MethodInvocation {
            external,
            level_words: None,
            original_argv: Some(argv.to_vec()),
        });
    }
    // Generated reporting heads are authored bytes, not original argv headers.
    let mut words = Vec::with_capacity(args.len() + 2);
    words.push(if external {
        invoked
            .map(<[u8]>::to_vec)
            .unwrap_or_else(|| oo_display(interp, obj))
    } else {
        b"my".to_vec()
    });
    words.push(method.to_vec());
    words.extend(native_context::original_word_bytes(interp, args)?);
    Ok(MethodInvocation::with_words(external, words))
}

/// The TclOO *execution* state (the per-flow stacks, not the shared class/object
/// definitions): swapped in/out when a coroutine suspends/resumes so a method
/// running inside a coroutine has its own `self`/`my`/`next` call chain and
/// definition context (`cmd_coro`).
#[derive(Default)]
pub struct OoExec {
    def_stack: Vec<(DefTarget, u64, Option<u64>, NsId, u64)>,
    call_stack: Vec<OoFrame>,
    private_depth: usize,
    def_rewrite: Option<Vec<u8>>,
    unknown_scope: Option<(OoId, bool)>,
    unknown_external: bool,
}

impl OoState {
    /// Swap this interp's OO execution stacks with `e` (the class/object
    /// *definitions* — `classes`/`objects`/`counter`/`next_id` — stay shared).
    pub(crate) fn swap_exec(&mut self, e: &mut OoExec) {
        std::mem::swap(&mut self.def_stack, &mut e.def_stack);
        std::mem::swap(&mut self.call_stack, &mut e.call_stack);
        std::mem::swap(&mut self.private_depth, &mut e.private_depth);
        std::mem::swap(&mut self.def_rewrite, &mut e.def_rewrite);
        std::mem::swap(&mut self.unknown_scope, &mut e.unknown_scope);
        std::mem::swap(&mut self.unknown_external, &mut e.unknown_external);
    }
}

fn err(interp: &mut Interp, msg: &[u8]) -> Code {
    interp.set_error(msg)
}

/// `wrong # args` for the OO definition commands — a deliberate variant of
/// [`Interp::wrong_args`]: the single-command definition forms
/// (`oo::define Foo method …`) report the *whole* original command via the
/// active ensemble-rewrite prefix (C's `Tcl_WrongNumArgs` rewrite path), so
/// this prepends `def_rewrite` before delegating to the shared helper.
fn wrong_args(interp: &mut Interp, usage: &[u8]) -> Code {
    let rewrite = interp.oo.borrow().def_rewrite.clone();
    if let Some(prefix) = rewrite {
        let mut u = prefix;
        u.push(b' ');
        u.extend_from_slice(usage);
        return Interp::wrong_args(interp, &u);
    }
    Interp::wrong_args(interp, usage)
}

#[cfg(test)]
mod native_private_tests;

#[cfg(test)]
mod native_method_rename_tests;

impl Interp {
    /// Retire the allocated partial object without dispatching later creation work.
    fn oo_refused_publication(&mut self, object: OoId, error: tcl_cmd_core::CmdError) -> Code {
        let code = self.report_cmd_error(error);
        let result = obj::Owned::retain(self.get_obj_result());
        let snapshot = self.snapshot_error();
        self.oo_destroy_bg(object);
        self.set_result(result.as_ptr());
        let restored = self.restore_error(snapshot);
        drop(result);
        if let Err(first) = restored {
            return self.refuse_native_execution(first);
        }
        if let Err(first) = self.check_entered_native_operation() {
            return self.refuse_native_execution(first);
        }
        code
    }

    /// The definition target at the actual selected variable activation.
    fn active_def_target(&self) -> Option<DefTarget> {
        let activation = self.frames.borrow().current_activation();
        self.oo
            .borrow()
            .def_stack
            .iter()
            .rev()
            .find(|(_, active, _, _, _)| *active == activation)
            .map(|(t, _, _, _, _)| t.clone())
    }

    /// Allocate the next monotonic object creation ID.
    fn oo_next_id(&self) -> u64 {
        let mut oo = self.oo.borrow_mut();
        oo.next_id += 1;
        oo.next_id
    }
}

#[cfg(test)]
mod native_eval_tests;

#[cfg(test)]
mod native_method_report_tests;

#[cfg(test)]
mod original_declaration_retirement_tests;

#[cfg(test)]
mod original_definition_stage_tests;

#[cfg(test)]
mod original_class_facet_tests;

#[cfg(test)]
mod original_info_target_tests;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod original_call_argv_tests;

mod native_properties;

#[cfg(test)]
mod native_info_option_tests;

#[cfg(test)]
mod native_explicit_variable_tests;

#[cfg(test)]
mod native_method_info_tests;

#[cfg(test)]
mod native_property_counted_tests;

#[cfg(test)]
mod native_original_holder_tests;

#[cfg(test)]
mod original_argument_tests;

#[cfg(test)]
mod original_namespace_copy_tests;

#[path = "cmd_oo/variable_mapping.rs"]
mod variable_mapping;

#[cfg(test)]
mod native_abstract_bootstrap_tests;
