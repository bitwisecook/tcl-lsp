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
//! Deferred: private methods/variables (8.7+), the full C3 mixin linearisation,
//! and `oo::define`'s rarer subcommands / internal introspection.
//!
//! See `list.rs` for the module-level `not_unsafe_ptr_arg_deref` rationale.
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use tcl_core_types::RecursionLimit;

use crate::interp::{
    obj_bytes, CallMeta, Code, Command, Interp, MethodFrameWhat, Param, ProcFrame,
};
use crate::list;
use crate::namespace::NsId;
use crate::obj::{self, TclObj};

/// Maximum superclass/mixin linearisation depth for [`Interp::linearize_class`]
/// / [`Interp::gather_class_props`] (issue #996). `tcl_syntax::mro::MAX_MRO_DEPTH`
/// fixed the identical algorithm (TclOO's DFS + late-placement) in the
/// *diagnostics* linearizer under the internal tracking label,
/// settling on 1024 there — but that pass runs on a host-controlled analysis
/// stack, not this runtime's live call stack. Confirmed crash reproduction
/// (this sweep): a deep `mixin` chain (`oo::class create C$i { mixin C[i-1]
/// }`) SIGABRTs between depth 100-150 on a 256 KiB stack, and still crashes
/// at depth 2000 on a 1 MiB stack, so 1024 would not actually stop the crash
/// on a small-stack embedding (a caught P1 review finding on the fix that
/// first introduced this guard). 64 — the same cap `MAX_ARRAY_INDEX_DEPTH` /
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

/// A native method handler `(interp, object, args) -> Code`, invoked without a
/// Tcl call frame (so `info level` inside any method it calls is unchanged) —
/// used for the `::oo::Slot` operations.
type NativeMethod = fn(&mut Interp, &[u8], &[*mut TclObj]) -> Code;

/// A method: a normal body, a `forward` to a command prefix, or a native handler.
#[derive(Clone)]
enum Method {
    Body {
        params: Vec<Param>,
        body: Vec<u8>,
        /// Source provenance (TIP 280) when the method was defined while a file
        /// was being sourced: `(file, body_line_base)`. Its `info frame` is then
        /// `type source` with file-absolute lines (`line_base` = the body's
        /// starting file line minus one). `None` for an eval-defined method.
        src: Option<(Rc<[u8]>, u32)>,
    },
    Forward {
        prefix: Vec<Vec<u8>>,
    },
    Builtin(NativeMethod),
}

/// A class definition.
#[derive(Default, Clone)]
struct Class {
    /// Superclass FQNs (defaults to `::oo::object` when none are declared).
    supers: Vec<Vec<u8>>,
    methods: BTreeMap<Vec<u8>, Method>,
    constructor: Option<Method>,
    destructor: Option<Vec<u8>>,
    /// Declared instance-variable names (auto-linked into every method frame).
    variables: Vec<Vec<u8>>,
    /// TIP 500 private instance variables (`private variable`): listed by
    /// `info class variables -private`, hidden from the plain form.
    private_variables: Vec<Vec<u8>>,
    /// TIP 558 readable / writable property name sets (stored uniqued,
    /// first-wins; sorted on introspection).
    readable_properties: Vec<Vec<u8>>,
    writable_properties: Vec<Vec<u8>>,
    /// Mixed-in classes (searched before the superclass MRO).
    mixins: Vec<Vec<u8>>,
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
#[derive(Default, Clone)]
struct Object {
    /// FQN of the object's class.
    class: Vec<u8>,
    /// The namespace holding this object's instance variables.
    var_ns: NsId,
    /// A unique, monotonic creation identifier (`info object creationid`),
    /// stable across rename.
    creation_id: u64,
    /// Per-object methods (`oo::objdefine method`).
    methods: BTreeMap<Vec<u8>, Method>,
    /// Per-object mixins.
    mixins: Vec<Vec<u8>>,
    unexported: BTreeSet<Vec<u8>>,
    /// Explicitly `export`ed names (see `Class::exported`).
    exported: BTreeSet<Vec<u8>>,
    /// TIP 500 private methods (see `Class::private`).
    private: BTreeSet<Vec<u8>>,
    /// Per-object filter method names (`oo::objdefine filter`).
    filters: Vec<Vec<u8>>,
    /// Per-object declared instance variables (`oo::objdefine variable`).
    variables: Vec<Vec<u8>>,
    /// Per-object TIP 500 private instance variables.
    private_variables: Vec<Vec<u8>>,
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
    /// Current FQNs of the per-object `my`/`myclass` commands. Tracked across
    /// `rename` so that destroying the object also deletes a `my` renamed out
    /// of the instance namespace (C ties `my`'s lifetime to the object).
    my_aliases: Vec<Vec<u8>>,
}

/// One step of a method-call chain: the provider (object or class FQN) and the
/// method name to run there. Filters appear as steps whose `method` is the
/// filter's name, ahead of the steps for the actually-invoked method.
#[derive(Clone)]
struct CallStep {
    provider: Vec<u8>,
    method: Vec<u8>,
    /// Whether this step resolves against the *object* facet of `provider`
    /// (its per-object methods) rather than the *class* facet (its instance
    /// methods). Normally `provider == object`, but a class mixed into its own
    /// instance (`self mixin <self>`) contributes both facets as distinct steps,
    /// so the facet can't be re-derived from the name alone (oo-13.7/23.1/41.2).
    is_object: bool,
}

/// One active method invocation (for `self` / `my` / `next`).
struct OoFrame {
    object: Vec<u8>,
    /// The full call chain (filter steps, then the target-method steps).
    chain: Vec<CallStep>,
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
    Class(Vec<u8>),
    Object(Vec<u8>),
}

/// The interpreter's TclOO state.
#[derive(Default)]
pub struct OoState {
    classes: BTreeMap<Vec<u8>, Class>,
    objects: BTreeMap<Vec<u8>, Object>,
    counter: usize,
    /// Monotonic source of object creation IDs (`info object creationid`).
    next_id: u64,
    /// The definition-target stack, each entry tagged with the call-frame level
    /// it is active at. The definition commands (`method`/`variable`/…) are in
    /// scope only when evaluation is *directly* at that level; a nested
    /// proc/method call suspends the context (and `uplevel` back into the body
    /// restores it), matching C's scoping of the `::oo::define` namespace.
    def_stack: Vec<(DefTarget, usize, Option<u64>)>,
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
    unknown_scope: Option<Vec<u8>>,
    /// Whether the unknown-method miss originated from an external (`$obj m`)
    /// call. An internal (`my m`) miss lists the unexported methods and the
    /// `oo::object` built-ins too; an external one lists only public methods.
    unknown_external: bool,
    /// The `wrong # args` command prefix a `forward` installs for the method it
    /// forwards to (the original invocation, e.g. `foo test`), consumed by the
    /// next method body's run_proc (C's ensemble-rewrite for `Tcl_WrongNumArgs`).
    fwd_usage: Option<Vec<u8>>,
    /// The full command words of the `create`/`new` invocation that is about to
    /// run a constructor (e.g. `oo::object create foo`), so the constructor's
    /// `info level 0` reports the instantiation rather than the synthetic
    /// `<constructor>` name (oo-2.1). Consumed by the constructor's run_proc.
    ctor_words: Option<Vec<Vec<u8>>>,
}

/// The object command name as named from the global scope (strip a single
/// leading `::`) — the `wrong # args` prefix for a method (approximating C's
/// literal `objv[0]` for the common, global-scope case).
fn object_display(obj: &[u8]) -> Vec<u8> {
    obj.strip_prefix(b"::")
        .map(<[u8]>::to_vec)
        .unwrap_or_else(|| obj.to_vec())
}

/// The TclOO *execution* state (the per-flow stacks, not the shared class/object
/// definitions): swapped in/out when a coroutine suspends/resumes so a method
/// running inside a coroutine has its own `self`/`my`/`next` call chain and
/// definition context (`cmd_coro`).
#[derive(Default)]
pub struct OoExec {
    def_stack: Vec<(DefTarget, usize, Option<u64>)>,
    call_stack: Vec<OoFrame>,
    private_depth: usize,
    def_rewrite: Option<Vec<u8>>,
    unknown_scope: Option<Vec<u8>>,
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

/// Register the `oo::*` commands and the definition / context commands.
pub fn install(interp: &mut Interp) {
    interp.register_builtin(b"oo::define", oo_define_cmd);
    interp.register_builtin(b"oo::objdefine", oo_objdefine_cmd);
    interp.register_builtin(b"oo::copy", oo_copy_cmd);
    // Definition-script commands (valid inside an `oo::define`/`oo::objdefine`).
    interp.register_builtin(b"method", def_method);
    interp.register_builtin(b"constructor", def_constructor);
    interp.register_builtin(b"destructor", def_destructor);
    interp.register_builtin(b"superclass", def_superclass);
    interp.register_builtin(b"variable", def_variable);
    interp.register_builtin(b"export", |i, a| def_export(i, a, true));
    interp.register_builtin(b"unexport", |i, a| def_export(i, a, false));
    interp.register_builtin(b"mixin", def_mixin);
    interp.register_builtin(b"forward", def_forward);
    interp.register_builtin(b"filter", def_filter);
    interp.register_builtin(b"private", def_private);
    // Method-context commands. `my` is created *per object* (in each object's
    // namespace) like C TclOO — never global — so a test's `rename ::my {}`
    // can't break it. `self`/`next` resolve via the call stack.
    interp.register_builtin(b"self", self_cmd);
    interp.register_builtin(b"next", next_cmd);
    interp.register_builtin(b"nextto", nextto_cmd);
    interp.register_builtin(b"classvariable", classvariable_cmd);
    // Root classes (so `superclass`-less classes inherit `object` and
    // `superclass oo::class`/`oo::object` validate). Both are themselves
    // objects (instances of `::oo::class`) and dispatch through `oo_dispatch`
    // like any other class command — so `create`/`new`/`destroy`, unknown-
    // method errors and the empty-name check are all handled uniformly.
    interp
        .oo
        .borrow_mut()
        .classes
        .insert(b"::oo::object".to_vec(), Class::default());
    interp.oo.borrow_mut().classes.insert(
        b"::oo::class".to_vec(),
        Class {
            supers: vec![b"::oo::object".to_vec()],
            ..Class::default()
        },
    );
    // `oo::object`'s instance-definition namespace (TIP 524, `-instance`) is
    // `::oo::objdefine` — defining any object (via `oo::objdefine`) runs there.
    // (`oo::class`'s `-class` namespace, `::oo::define`, is handled in the
    // `definitionnamespace` introspection.)
    if let Some(c) = interp
        .oo
        .borrow_mut()
        .classes
        .get_mut(b"::oo::object".as_slice())
    {
        c.def_ns = Some(b"::oo::objdefine".to_vec());
    }
    // Defining a class (an instance of `oo::class`) happens in `::oo::define`.
    if let Some(c) = interp
        .oo
        .borrow_mut()
        .classes
        .get_mut(b"::oo::class".as_slice())
    {
        c.class_def_ns = Some(b"::oo::define".to_vec());
    }
    for fqn in [b"::oo::object".as_slice(), b"::oo::class".as_slice()] {
        let var_ns = interp.ensure_namespace(fqn);
        let creation_id = interp.oo_next_id();
        interp.oo.borrow_mut().objects.insert(
            fqn.to_vec(),
            Object {
                class: b"::oo::class".to_vec(),
                var_ns,
                creation_id,
                ..Object::default()
            },
        );
        interp.ns_register(fqn, Command::OoObject(fqn.to_vec()));
        // Engine-installed, not script-created: the registry dates these
        // (TCL86_PLUS) and the availability gate must honour that (#1463).
        interp.declare_registry_object_root(fqn);
        interp.oo_register_my(fqn);
    }
    // `oo::object` has a built-in (unexported) `unknown` method — the standard
    // "unknown method" error, and the terminus of the `unknown` call chain.
    if let Some(c) = interp
        .oo
        .borrow_mut()
        .classes
        .get_mut(b"::oo::object".as_slice())
    {
        c.methods
            .insert(b"unknown".to_vec(), Method::Builtin(oo_object_unknown));
        c.unexported.insert(b"unknown".to_vec());
        // `<cloned>` copies the instance namespace during `oo::copy`; reachable
        // via `next` from a user-defined `<cloned>` override.
        c.methods
            .insert(b"<cloned>".to_vec(), Method::Builtin(oo_object_cloned));
        c.unexported.insert(b"<cloned>".to_vec());
    }
    let _ =
        interp.eval_str(b"namespace eval ::oo {variable version 1.3.1; variable patchlevel 1.3.1}");
    // The definition namespaces exist as real namespaces (TIP 524 lets user
    // code put them on a `namespace path`); the actual definition subcommands
    // are resolved by the global builtins + the define-context fallback.
    let _ = interp.eval_str(b"namespace eval ::oo::define {}; namespace eval ::oo::objdefine {}");
    install_slot_class(interp);
    let _ = interp.eval_str(b"namespace eval ::oo::configuresupport {}");
    // The seven definition slots and the four TIP 558 property slots are real
    // `::oo::Slot` instances (C's `TclOODefineSlots`), invoked e.g. as
    // `oo::define c ::oo::configuresupport::readableproperties -set …`. Their
    // per-instance `Get`/`Set` read/write the active definition target's lists.
    install_slot_instances(interp);
    install_configurable(interp);
    register_define_ns_commands(interp);
}

/// Register the definition subcommands as real commands in `::oo::define` /
/// `::oo::objdefine` (e.g. `::oo::define::method`). Invoked outside a definition
/// context they report C's "this command may only be called …" error; inside
/// one they dispatch like the bare definition subcommand. The list-valued slots
/// (`filter`/`mixin`/`superclass`/`variable`) are *not* listed here: they are
/// real `::oo::Slot` instances installed by `install_slot_instances`.
fn register_define_ns_commands(interp: &mut Interp) {
    const CLASS: &[&[u8]] = &[
        b"constructor",
        b"definitionnamespace",
        b"deletemethod",
        b"destructor",
        b"export",
        b"forward",
        b"method",
        b"private",
        b"renamemethod",
        b"self",
        b"unexport",
    ];
    const OBJ: &[&[u8]] = &[
        b"class",
        b"deletemethod",
        b"export",
        b"forward",
        b"method",
        b"private",
        b"renamemethod",
        b"self",
        b"unexport",
    ];
    for sub in CLASS {
        let mut fqn = b"::oo::define::".to_vec();
        fqn.extend_from_slice(sub);
        interp.ns_register(&fqn, Command::Builtin(oo_ns_define_class_cmd));
    }
    for sub in OBJ {
        let mut fqn = b"::oo::objdefine::".to_vec();
        fqn.extend_from_slice(sub);
        interp.ns_register(&fqn, Command::Builtin(oo_ns_objdefine_cmd));
    }
}

/// `::oo::define::<sub>` command (class context).
fn oo_ns_define_class_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    oo_ns_define_cmd(interp, argv, false)
}

/// `::oo::objdefine::<sub>` command (object context).
fn oo_ns_objdefine_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    oo_ns_define_cmd(interp, argv, true)
}

/// A `::oo::define::<sub>` / `::oo::objdefine::<sub>` command: errors outside a
/// definition context, else dispatches the subcommand on the current target.
/// `is_objdefine` is fixed by the namespace the command was registered in (the
/// invoked `argv[0]` is the bare subcommand name when resolved directly in a
/// definition-script body, so it can't be used to tell the two apart).
fn oo_ns_define_cmd(interp: &mut Interp, argv: &[*mut TclObj], is_objdefine_cmd: bool) -> Code {
    let target = match interp.active_def_target() {
        Some(t) => t,
        None => {
            return interp.set_error(
                b"this command may only be called from within the context of an \
                  ::oo::define or ::oo::objdefine command",
            );
        }
    };
    let cmd = obj_bytes(argv[0]);
    // A `::oo::define::*` command requires a class context and `::oo::objdefine::*`
    // an object context; the wrong pairing is an API misuse (oo-1.7).
    let target_is_object = matches!(target, DefTarget::Object(_));
    if is_objdefine_cmd != target_is_object {
        return interp.set_error(b"attempt to misuse API");
    }
    // The subcommand is the segment after the final `::` (when invoked by its
    // qualified name), else the bare invoked name.
    let sub: Vec<u8> = (0..cmd.len().saturating_sub(1))
        .rev()
        .find(|&i| &cmd[i..i + 2] == b"::")
        .map(|i| cmd[i + 2..].to_vec())
        .unwrap_or_else(|| cmd.clone());
    interp
        .oo_define_command(&sub, argv)
        .unwrap_or_else(|| interp.invalid_command(&cmd))
}

/// TIP 558 `oo::configurable` metaclass + the `property` definition command and
/// the `configure` method (C's tclOOProp.c / configurable wiring). A class made
/// with `oo::configurable create` mixes in `::oo::configuresupport::configurable`
/// (for `configure`) and resolves `property` in the configurableclass /
/// configurableobject definition namespaces.
fn install_configurable(interp: &mut Interp) {
    // The two definition namespaces hold the `property` command (class /
    // instance variants), reached via TIP 524 definition-namespace resolution.
    let _ = interp.eval_str(
        b"namespace eval ::oo::configuresupport::configurableclass {}; \
          namespace eval ::oo::configuresupport::configurableobject {}",
    );
    interp.ns_register(
        b"::oo::configuresupport::configurableclass::property",
        Command::Builtin(prop_define_class),
    );
    interp.ns_register(
        b"::oo::configuresupport::configurableobject::property",
        Command::Builtin(prop_define_object),
    );
    // The mixin that gives configurable objects their `configure` method.
    let _ = interp.eval_str(b"oo::class create ::oo::configuresupport::configurable {}");
    if let Some(c) = interp
        .oo
        .borrow_mut()
        .classes
        .get_mut(b"::oo::configuresupport::configurable".as_slice())
    {
        c.methods
            .insert(b"configure".to_vec(), Method::Builtin(oo_configure));
    }
    // The `oo::configurable` metaclass. Its instances (configurable classes) get
    // the configurable support set up natively in `oo_new_ns` — mixing in the
    // support class and pointing instance definitions at configurableobject —
    // so the definition script still resolves class-name arguments in the
    // caller's namespace. Its own definition namespace is configurableclass so
    // `property` resolves while defining those classes.
    let _ = interp.eval_str(
        b"oo::class create ::oo::configurable { superclass ::oo::class }\n\
          oo::define ::oo::configurable definitionnamespace ::oo::configuresupport::configurableclass",
    );
    install_abstract_singleton(interp);
    // The 9.0 metaclasses are engine-installed on the registry's behalf too,
    // so the release gate hides them below their introducing release the way
    // it hides a builtin (#1463) — real tclsh 8.6.16 has no `oo::configurable`.
    for root in [
        b"::oo::configurable".as_slice(),
        b"::oo::singleton",
        b"::oo::abstract",
    ] {
        interp.declare_registry_object_root(root);
    }
    // NOTE: `::oo::Slot` and `::oo::SingletonInstance` are engine-installed
    // too, but marking them roots here would be inert — the release gate dates
    // a root through the registry, and the registry has no spec for either
    // name, so `profile_admits_registry_builtin` admits them on every surface.
    // They are therefore still present (and consistently callable) on an 8.4
    // surface that should have no TclOO at all. That is a registry-content
    // gap, not a gate gap; #1463's gate is mirrored correctly without them.
}

/// TIP-less foundation metaclasses created by `InitFoundation` in C
/// (`tclOO.c`): `oo::singleton`, `oo::SingletonInstance`, and `oo::abstract`.
/// `oo-1.21` introspects them, so they must exist in every fresh interp with
/// the same class/superclass relationships the C core sets up.
fn install_abstract_singleton(interp: &mut Interp) {
    // `oo::singleton`: a metaclass (superclass oo::class) whose instances only
    // permit a single instance; it unexports `create`/`createWithNamespace`.
    let _ = interp.eval_str(
        b"oo::class create ::oo::singleton { superclass ::oo::class }\n\
          oo::define ::oo::singleton unexport create createWithNamespace",
    );
    // `oo::SingletonInstance`: a plain class (superclass oo::object) mixed into
    // singleton instances so they can't easily be destroyed or cloned.
    let _ = interp.eval_str(b"oo::class create ::oo::SingletonInstance {}");
    // `oo::abstract`: a metaclass (superclass oo::class) whose instances can't
    // be directly instantiated; it unexports `create`/`createWithNamespace`/`new`.
    let _ = interp.eval_str(
        b"oo::class create ::oo::abstract { superclass ::oo::class }\n\
          oo::define ::oo::abstract unexport create createWithNamespace new",
    );
}

fn prop_define_class(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    prop_define(interp, argv, false)
}
fn prop_define_object(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    prop_define(interp, argv, true)
}

/// `property name ?-get body? ?-set body? ?-kind k? …` (TIP 558,
/// `TclOODefinePropertyCmd`): declare configurable properties on the current
/// definition target. Default props read/write the like-named instance
/// variable through generated `<ReadProp-name>` / `<WriteProp-name>` methods;
/// `-get`/`-set` supply custom bodies. Registers `-name` in the readable /
/// writable property set per `-kind` (readable / readwrite [default] / writable).
fn prop_define(interp: &mut Interp, argv: &[*mut TclObj], use_instance: bool) -> Code {
    let target = match def_target(interp) {
        Ok(t) => t,
        Err(code) => return code,
    };
    let mut i = 1;
    while i < argv.len() {
        let prop = obj_bytes(argv[i]);
        i += 1;
        // Validate the property name (C's TclOOInstallStdPropertyImpls). Order
        // matters: the `-` check precedes the simple-word check.
        let bad = if prop.first() == Some(&b'-') {
            Some(&b"must not begin with -"[..])
        } else if prop.is_empty() || prop.iter().any(|c| c.is_ascii_whitespace()) {
            Some(&b"must be a simple word"[..])
        } else if prop.windows(2).any(|w| w == b"::") {
            Some(&b"must not contain namespace separators"[..])
        } else if prop.contains(&b'(') || prop.contains(&b')') {
            Some(&b"must not contain parentheses"[..])
        } else {
            None
        };
        if let Some(reason) = bad {
            let mut m = b"bad property name \"".to_vec();
            m.extend_from_slice(&prop);
            m.extend_from_slice(b"\": ");
            m.extend_from_slice(reason);
            return interp.error_with_code(&m, b"TCL OO PROPERTY_FORMAT");
        }
        // Parse the property's options.
        let (mut kind_ro, mut kind_wo) = (false, false);
        let mut getter: Option<Vec<u8>> = None;
        let mut setter: Option<Vec<u8>> = None;
        while i < argv.len() && obj_bytes(argv[i]).first() == Some(&b'-') {
            let opt = obj_bytes(argv[i]);
            // Options accept unambiguous prefixes (`-g`/`-k`/`-s`).
            let optname: &[u8] = match prefix_match(&opt, &[b"-get", b"-kind", b"-set"]) {
                Some(o) => o,
                None => {
                    let mut m = b"bad option \"".to_vec();
                    m.extend_from_slice(&opt);
                    m.extend_from_slice(b"\": must be -get, -kind, or -set");
                    let mut code = b"TCL LOOKUP INDEX option ".to_vec();
                    code.extend_from_slice(&opt);
                    return interp.error_with_code(&m, &code);
                }
            };
            if i + 1 >= argv.len() {
                let what: &[u8] = if optname == b"-kind" {
                    b"kind value"
                } else {
                    b"body"
                };
                let mut m = b"missing ".to_vec();
                m.extend_from_slice(what);
                m.extend_from_slice(b" to go with ");
                m.extend_from_slice(optname);
                m.extend_from_slice(b" option");
                return interp.error_with_code(&m, b"TCL WRONGARGS");
            }
            let val = obj_bytes(argv[i + 1]);
            i += 2;
            match optname {
                b"-get" => getter = Some(val),
                b"-set" => setter = Some(val),
                b"-kind" => match prefix_match(&val, &[b"readable", b"readwrite", b"writable"]) {
                    Some(b"readable") => {
                        kind_ro = true;
                        kind_wo = false;
                    }
                    Some(b"writable") => {
                        kind_wo = true;
                        kind_ro = false;
                    }
                    Some(b"readwrite") => {
                        kind_ro = false;
                        kind_wo = false;
                    }
                    _ => {
                        let mut m = b"bad kind \"".to_vec();
                        m.extend_from_slice(&val);
                        m.extend_from_slice(b"\": must be readable, readwrite, or writable");
                        let mut code = b"TCL LOOKUP INDEX kind ".to_vec();
                        code.extend_from_slice(&val);
                        return interp.error_with_code(&m, &code);
                    }
                },
                _ => unreachable!(),
            }
        }
        let readable = !kind_wo;
        let writable = !kind_ro;
        // Install the accessor methods (`<ReadProp-name>` / `<WriteProp-name>`),
        // unexported (their `<…>` names are non-lowercase). Default impls read /
        // write the like-named instance variable.
        if readable {
            let mname = property_method_name(b"<ReadProp-", &prop);
            let body = getter.clone().unwrap_or_else(|| std_getter_body(&prop));
            if let Code::Error = install_property_method(interp, &mname, &[], &body) {
                return Code::Error;
            }
        }
        if writable {
            let mname = property_method_name(b"<WriteProp-", &prop);
            let body = setter.clone().unwrap_or_else(|| std_setter_body(&prop));
            if let Code::Error = install_property_method(interp, &mname, b"value", &body) {
                return Code::Error;
            }
        }
        // Register `-name`: add to (or remove from) each set per the kind, so a
        // later declaration with a different `-kind` replaces the membership
        // (C's TclOORegisterProperty add/remove; ooProp-3.16).
        let mut hyph = b"-".to_vec();
        hyph.extend_from_slice(&prop);
        set_property_membership(interp, &target, use_instance, false, &hyph, readable);
        set_property_membership(interp, &target, use_instance, true, &hyph, writable);
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

/// Unambiguous-prefix match of `arg` against `cands` (Tcl option-table style):
/// the exact name, else the unique candidate it prefixes, else `None` — the
/// shared `Tcl_GetIndexFromObjStruct` matcher.
fn prefix_match<'a>(arg: &[u8], cands: &'a [&'a [u8]]) -> Option<&'a [u8]> {
    use tcl_cmd_core::prefix::Resolution;
    match tcl_cmd_core::prefix::scan(cands, arg, false) {
        Resolution::Exact(i) | Resolution::UniquePrefix(i) => Some(cands[i]),
        Resolution::Ambiguous | Resolution::NoMatch => None,
    }
}

fn property_method_name(prefix: &[u8], prop: &[u8]) -> Vec<u8> {
    let mut m = prefix.to_vec();
    m.extend_from_slice(prop);
    m.push(b'>');
    m
}

/// The default getter body: read the like-named instance variable.
fn std_getter_body(prop: &[u8]) -> Vec<u8> {
    let mut b = b"::variable ".to_vec();
    b.extend_from_slice(prop);
    b.extend_from_slice(b"; return [::set ");
    b.extend_from_slice(prop);
    b.push(b']');
    b
}

/// The default setter body: write the like-named instance variable.
fn std_setter_body(prop: &[u8]) -> Vec<u8> {
    let mut b = b"::variable ".to_vec();
    b.extend_from_slice(prop);
    b.extend_from_slice(b"; ::set ");
    b.extend_from_slice(prop);
    b.extend_from_slice(b" $value");
    b
}

fn install_property_method(interp: &mut Interp, name: &[u8], params: &[u8], body: &[u8]) -> Code {
    let params = match crate::cmd_proc::parse_params(params) {
        Ok(p) => p,
        Err(e) => return err(interp, &e),
    };
    install_method_vis(
        interp,
        name.to_vec(),
        Method::Body {
            params,
            body: body.to_vec(),
            src: None,
        },
        MethodVis::Unexported,
    )
}

/// Add or remove `prop` (already hyphenated) in a definition target's
/// readable/writable property set (C's `BuildPropertyList`).
fn set_property_membership(
    interp: &mut Interp,
    target: &DefTarget,
    use_instance: bool,
    writable: bool,
    prop: &[u8],
    member: bool,
) {
    let mut set = read_property_set(interp, target, use_instance, writable);
    let present = set.iter().any(|p| p.as_slice() == prop);
    if member && !present {
        set.push(prop.to_vec());
    } else if !member && present {
        set.retain(|p| p.as_slice() != prop);
    } else {
        return; // no change
    }
    write_property_set(interp, target, use_instance, writable, set);
}

/// `configure` method (TIP 558, `TclOO_Configurable_Configure`): with no args
/// returns the readable properties as an `-opt value …` dict; with one `-opt`
/// reads it; with `-opt value …` pairs writes them.
fn oo_configure(interp: &mut Interp, obj: &[u8], args: &[*mut TclObj]) -> Code {
    let n = args.len();
    if n != 1 && n % 2 == 1 {
        let mut u = obj.to_vec();
        u.extend_from_slice(b" configure ?-option value ...?");
        return wrong_args(interp, &u);
    }
    if n == 0 {
        // Read every readable property into a dict (sorted by name).
        let props = interp.object_property_list(obj, false, true);
        let mut pairs: Vec<Vec<u8>> = Vec::with_capacity(props.len() * 2);
        for p in props {
            let code = read_property(interp, obj, &p);
            if code != Code::Ok {
                return code;
            }
            pairs.push(p);
            pairs.push(interp.result_bytes());
        }
        set_list(interp, &pairs);
        return Code::Ok;
    }
    if n == 1 {
        let name = match get_property_name(interp, obj, args[0], false) {
            Ok(n) => n,
            Err(c) => return c,
        };
        return read_property(interp, obj, &name);
    }
    let mut i = 0;
    while i < n {
        let name = match get_property_name(interp, obj, args[i], true) {
            Ok(n) => n,
            Err(c) => return c,
        };
        let code = write_property(interp, obj, &name, args[i + 1]);
        if code != Code::Ok {
            return code;
        }
        i += 2;
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

/// Read a property by invoking the object's `<ReadProp-name>` method via `my`.
/// A `break`/`continue` from the getter is turned into an error (C's
/// ReadProperty).
fn read_property(interp: &mut Interp, obj: &[u8], prop_hyph: &[u8]) -> Code {
    let mname = property_method_name(b"<ReadProp", prop_hyph);
    let code = interp.oo_invoke(obj, &mname, &[], false);
    property_loopword_error(interp, code, b"getter", prop_hyph)
}

fn write_property(interp: &mut Interp, obj: &[u8], prop_hyph: &[u8], value: *mut TclObj) -> Code {
    let mname = property_method_name(b"<WriteProp", prop_hyph);
    let code = interp.oo_invoke(obj, &mname, &[value], false);
    property_loopword_error(interp, code, b"setter", prop_hyph)
}

/// Map a property accessor's `break`/`continue` to the C error message
/// (`property getter|setter for -x did a break|continue`).
fn property_loopword_error(interp: &mut Interp, code: Code, role: &[u8], prop_hyph: &[u8]) -> Code {
    let word: &[u8] = match code {
        Code::Break => b"break",
        Code::Continue => b"continue",
        _ => return code,
    };
    let mut m = b"property ".to_vec();
    m.extend_from_slice(role);
    m.extend_from_slice(b" for ");
    m.extend_from_slice(prop_hyph);
    m.extend_from_slice(b" did a ");
    m.extend_from_slice(word);
    interp.set_error(&m)
}

/// Resolve a `configure` option to a property name in the object's readable /
/// writable set, or produce the C error (`bad property` / `is read|write only`).
fn get_property_name(
    interp: &mut Interp,
    obj: &[u8],
    given: *mut TclObj,
    writable: bool,
) -> Result<Vec<u8>, Code> {
    let given = obj_bytes(given);
    let cands = interp.object_property_list(obj, writable, true);
    if cands.contains(&given) {
        return Ok(given);
    }
    // Accessible the other way? Report read-only / write-only.
    let other = interp.object_property_list(obj, !writable, true);
    if other.contains(&given) {
        let mut m = b"property \"".to_vec();
        m.extend_from_slice(&given);
        m.extend_from_slice(b"\" is ");
        m.extend_from_slice(if writable { b"read" } else { b"write" });
        m.extend_from_slice(b" only");
        return Err(interp.set_error(&m));
    }
    // Otherwise a plain bad-property error listing the candidates (Oxford comma).
    let mut m = b"bad property \"".to_vec();
    m.extend_from_slice(&given);
    m.extend_from_slice(b"\": must be ");
    m.extend_from_slice(&oxford_join(&cands));
    let mut code = b"TCL LOOKUP INDEX property ".to_vec();
    code.extend_from_slice(&given);
    Err(interp.error_with_code(&m, &code))
}

/// Join names as `a`, `a or b`, or `a, b, or c` (Tcl's option-table style).
fn oxford_join(items: &[Vec<u8>]) -> Vec<u8> {
    let mut out = Vec::new();
    let n = items.len();
    for (i, it) in items.iter().enumerate() {
        if i > 0 {
            if n > 2 {
                out.extend_from_slice(b", ");
            } else {
                out.push(b' ');
            }
            if i == n - 1 {
                out.extend_from_slice(b"or ");
            }
        }
        out.extend_from_slice(it);
    }
    out
}

/// Parse the `info … properties` option flags (`-all`, `-readable` [default],
/// `-writable`) from `opts`, returning `(all, writable)` or an error code.
fn parse_property_opts(interp: &mut Interp, opts: &[*mut TclObj]) -> Result<(bool, bool), Code> {
    let (mut all, mut writable) = (false, false);
    for &a in opts {
        match obj_bytes(a).as_slice() {
            b"-all" => all = true,
            b"-readable" => writable = false,
            b"-writable" => writable = true,
            other => {
                let mut m = b"bad option \"".to_vec();
                m.extend_from_slice(other);
                m.extend_from_slice(b"\": must be -all, -readable, or -writable");
                return Err(interp.set_error(&m));
            }
        }
    }
    Ok((all, writable))
}

/// The current value of a definition target's readable/writable property set.
/// `want_object` reads the per-object set (the object's entry); otherwise the
/// class set (the class entry). A class is registered in both maps, so the
/// target FQN is the key in either case.
fn read_property_set(
    interp: &Interp,
    target: &DefTarget,
    want_object: bool,
    writable: bool,
) -> Vec<Vec<u8>> {
    let fqn = match target {
        DefTarget::Class(c) | DefTarget::Object(c) => c,
    };
    let oo = interp.oo.borrow();
    if want_object {
        oo.objects.get(fqn).map(|o| {
            if writable {
                o.writable_properties.clone()
            } else {
                o.readable_properties.clone()
            }
        })
    } else {
        oo.classes.get(fqn).map(|c| {
            if writable {
                c.writable_properties.clone()
            } else {
                c.readable_properties.clone()
            }
        })
    }
    .unwrap_or_default()
}

fn write_property_set(
    interp: &mut Interp,
    target: &DefTarget,
    want_object: bool,
    writable: bool,
    set: Vec<Vec<u8>>,
) {
    let fqn = match target {
        DefTarget::Class(c) | DefTarget::Object(c) => c.clone(),
    };
    let mut oo = interp.oo.borrow_mut();
    if want_object {
        if let Some(o) = oo.objects.get_mut(&fqn) {
            if writable {
                o.writable_properties = set;
            } else {
                o.readable_properties = set;
            }
        }
    } else if let Some(c) = oo.classes.get_mut(&fqn) {
        if writable {
            c.writable_properties = set;
        } else {
            c.readable_properties = set;
        }
    }
}

/// Define `::oo::Slot` (TIP 380): the overridable `Get`/`Set`/`Resolve` and the
/// `unknown`-driven defaulting are pure Tcl; the `-set`/`-append`/… operations
/// are native so they add no call frame (the `info level` the ops observe
/// matches C). The default operation is `-append` (subclasses override it).
fn install_slot_class(interp: &mut Interp) {
    // `--default-operation` is a forward (`my -append`) and `unknown` is a
    // native method (`slot_m_unknown`): like C's `TclOONewForwardMethod` /
    // `Slot_Unknown`, neither adds a Tcl call frame, so the `info level` the
    // overridable `Get`/`Set`/`Resolve` observe is the same as for a direct
    // operation. `destroy` is hidden too, so calling it on a slot is just
    // another datum routed through the default operation.
    let _ = interp.eval_str(
        b"oo::class create ::oo::Slot {\n\
            method Get {} { return {} }\n\
            method Set list { return }\n\
            method Resolve x { return $x }\n\
            forward --default-operation my -append\n\
            unexport Get Set Resolve --default-operation unknown destroy\n\
          }",
    );
    // Inject the native list operations + unknown handler as class methods.
    let ops: &[(&[u8], NativeMethod)] = &[
        (b"-set", slot_m_set),
        (b"-append", slot_m_append),
        (b"-prepend", slot_m_prepend),
        (b"-remove", slot_m_remove),
        (b"-appendifnew", slot_m_appendifnew),
        (b"-clear", slot_m_clear),
        (b"unknown", slot_m_unknown),
    ];
    if let Some(cl) = interp
        .oo
        .borrow_mut()
        .classes
        .get_mut(b"::oo::Slot".as_slice())
    {
        for (name, f) in ops {
            cl.methods.insert(name.to_vec(), Method::Builtin(*f));
        }
    }
}

/// The eleven built-in slot objects (`::oo::Slot` instances) that C's
/// `TclOODefineSlots` installs in every interp: the list-valued definition
/// slots and the TIP 558 property slots. `oo-1.21` / `oo-34.*` introspect them
/// (`info class instances ::oo::Slot`, per-slot method lists). Each gets
/// per-instance `Get`/`Set` (native, operating on the active definition target)
/// plus, for the class-reference slots, a `Resolve` (resolves a class name) and
/// a `--default-operation` forwarding to `-set`.
fn install_slot_instances(interp: &mut Interp) {
    // (slot object name, is a class-reference slot: mixin/superclass)
    const SLOTS: &[(&[u8], bool)] = &[
        (b"::oo::define::filter", false),
        (b"::oo::define::mixin", true),
        (b"::oo::define::superclass", true),
        (b"::oo::define::variable", false),
        (b"::oo::objdefine::filter", false),
        (b"::oo::objdefine::mixin", true),
        (b"::oo::objdefine::variable", false),
        (b"::oo::configuresupport::readableproperties", false),
        (b"::oo::configuresupport::writableproperties", false),
        (b"::oo::configuresupport::objreadableproperties", false),
        (b"::oo::configuresupport::objwritableproperties", false),
    ];
    for (name, class_ref) in SLOTS {
        let mut script = b"::oo::Slot create ".to_vec();
        script.extend_from_slice(name);
        let _ = interp.eval_str(&script);
        if let Some(o) = interp.oo.borrow_mut().objects.get_mut(*name) {
            o.methods
                .insert(b"Get".to_vec(), Method::Builtin(slot_inst_get));
            o.methods
                .insert(b"Set".to_vec(), Method::Builtin(slot_inst_set));
            o.unexported.insert(b"Get".to_vec());
            o.unexported.insert(b"Set".to_vec());
            if *class_ref {
                o.methods
                    .insert(b"Resolve".to_vec(), Method::Builtin(slot_inst_resolve));
                o.unexported.insert(b"Resolve".to_vec());
                // Class-reference slots default to `-set` (not the base `-append`).
                o.methods.insert(
                    b"--default-operation".to_vec(),
                    Method::Forward {
                        prefix: vec![b"my".to_vec(), b"-set".to_vec()],
                    },
                );
                o.unexported.insert(b"--default-operation".to_vec());
            }
        }
    }
}

/// Classify a built-in slot object by name into the definition-target field it
/// reads/writes. `None` for the property slots (handled via `read/write_
/// property_set`) or an unknown name.
fn slot_property_kind(slot: &[u8]) -> Option<(bool, bool)> {
    // (want_object, writable)
    match slot {
        b"::oo::configuresupport::readableproperties" => Some((false, false)),
        b"::oo::configuresupport::writableproperties" => Some((false, true)),
        b"::oo::configuresupport::objreadableproperties" => Some((true, false)),
        b"::oo::configuresupport::objwritableproperties" => Some((true, true)),
        _ => None,
    }
}

/// Read the current contents of a built-in slot from the active definition
/// target (C's per-slot `*_Get`).
fn slot_field_read(interp: &Interp, slot: &[u8], target: &DefTarget) -> Vec<Vec<u8>> {
    if let Some((want_object, writable)) = slot_property_kind(slot) {
        return read_property_set(interp, target, want_object, writable);
    }
    let oo = interp.oo.borrow();
    match (slot, target) {
        (b"::oo::define::filter", DefTarget::Class(c)) => {
            oo.classes.get(c).map(|x| x.filters.clone())
        }
        (b"::oo::define::mixin", DefTarget::Class(c)) => {
            oo.classes.get(c).map(|x| x.mixins.clone())
        }
        (b"::oo::define::superclass", DefTarget::Class(c)) => {
            oo.classes.get(c).map(|x| x.supers.clone())
        }
        (b"::oo::define::variable", DefTarget::Class(c)) => {
            oo.classes.get(c).map(|x| x.variables.clone())
        }
        (b"::oo::objdefine::filter", DefTarget::Object(o)) => {
            oo.objects.get(o).map(|x| x.filters.clone())
        }
        (b"::oo::objdefine::mixin", DefTarget::Object(o)) => {
            oo.objects.get(o).map(|x| x.mixins.clone())
        }
        (b"::oo::objdefine::variable", DefTarget::Object(o)) => {
            oo.objects.get(o).map(|x| x.variables.clone())
        }
        _ => None,
    }
    .unwrap_or_default()
}

/// Write a built-in slot's contents back to the active definition target (C's
/// per-slot `*_Set`). The class-reference and filter/variable list slots store
/// the list verbatim; the property slots unique it (first occurrence wins).
fn slot_field_write(interp: &mut Interp, slot: &[u8], target: &DefTarget, list: Vec<Vec<u8>>) {
    if let Some((want_object, writable)) = slot_property_kind(slot) {
        let mut uniq: Vec<Vec<u8>> = Vec::with_capacity(list.len());
        for v in list {
            if !uniq.contains(&v) {
                uniq.push(v);
            }
        }
        write_property_set(interp, target, want_object, writable, uniq);
        return;
    }
    let mut oo = interp.oo.borrow_mut();
    match (slot, target) {
        (b"::oo::define::filter", DefTarget::Class(c)) => {
            if let Some(x) = oo.classes.get_mut(c) {
                x.filters = list;
            }
        }
        (b"::oo::define::mixin", DefTarget::Class(c)) => {
            if let Some(x) = oo.classes.get_mut(c) {
                x.mixins = list;
            }
        }
        (b"::oo::define::superclass", DefTarget::Class(c)) => {
            if let Some(x) = oo.classes.get_mut(c) {
                x.supers = if list.is_empty() {
                    vec![b"::oo::object".to_vec()]
                } else {
                    list
                };
            }
        }
        (b"::oo::define::variable", DefTarget::Class(c)) => {
            if let Some(x) = oo.classes.get_mut(c) {
                x.variables = list;
            }
        }
        (b"::oo::objdefine::filter", DefTarget::Object(o)) => {
            if let Some(x) = oo.objects.get_mut(o) {
                x.filters = list;
            }
        }
        (b"::oo::objdefine::mixin", DefTarget::Object(o)) => {
            if let Some(x) = oo.objects.get_mut(o) {
                x.mixins = list;
            }
        }
        (b"::oo::objdefine::variable", DefTarget::Object(o)) => {
            if let Some(x) = oo.objects.get_mut(o) {
                x.variables = list;
            }
        }
        _ => {}
    }
}

/// A built-in slot's `Get` method: returns its current contents from the active
/// definition target.
fn slot_inst_get(interp: &mut Interp, slot: &[u8], _args: &[*mut TclObj]) -> Code {
    let Some(target) = interp.active_def_target() else {
        return err(
            interp,
            b"this command may only be called from within the context of an \
              ::oo::define or ::oo::objdefine command",
        );
    };
    let list = slot_field_read(interp, slot, &target);
    set_list(interp, &list);
    Code::Ok
}

/// A built-in slot's `Set` method: stores the (single list) argument into the
/// active definition target.
fn slot_inst_set(interp: &mut Interp, slot: &[u8], args: &[*mut TclObj]) -> Code {
    let Some(target) = interp.active_def_target() else {
        return err(
            interp,
            b"this command may only be called from within the context of an \
              ::oo::define or ::oo::objdefine command",
        );
    };
    let list = args
        .first()
        .map(|&a| parse_list(&obj_bytes(a)))
        .unwrap_or_default();
    slot_field_write(interp, slot, &target, list);
    interp.set_result_bytes(b"");
    Code::Ok
}

/// A class-reference slot's `Resolve` method (C's `Slot_ResolveClass`): resolve
/// the item as a class in the active definition context (current namespace then
/// global), returning it unchanged if it does not name a class.
fn slot_inst_resolve(interp: &mut Interp, _slot: &[u8], args: &[*mut TclObj]) -> Code {
    let raw = args.first().map(|&a| obj_bytes(a)).unwrap_or_default();
    let resolved = interp.oo_resolve_object(&raw);
    if interp.oo.borrow().classes.contains_key(&resolved) {
        interp.set_result(obj::new_string_bytes(&resolved));
    } else {
        interp.set_result(obj::new_string_bytes(&raw));
    }
    Code::Ok
}

/// C's `Slot_Unknown`: the method-miss handler for slots. `args[0]` is the
/// missed method name (prepended by the dispatcher). With no args, or a first
/// arg that does not start with `-`, dispatch the slot's default operation over
/// all the args; a leading `-flag` that is not a known operation chains (as C's
/// `next` would) to the standard unknown-method error.
fn slot_m_unknown(interp: &mut Interp, obj: &[u8], args: &[*mut TclObj]) -> Code {
    let first = args.first().map(|&a| obj_bytes(a)).unwrap_or_default();
    if !args.is_empty() && first.first() == Some(&b'-') {
        return interp.oo_unknown_method(obj, &first);
    }
    interp.oo_invoke(obj, b"--default-operation", args, false)
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

impl Interp {
    /// `unknown method "X": must be <public methods + built-ins>` — the C TclOO
    /// error, listing the callable methods (sorted, `a, b or c` style).
    fn oo_unknown_method(&mut self, obj: &[u8], requested: &[u8]) -> Code {
        // `destroy` is a real (overridable, unexportable) method on every
        // object; it is visible unless unexported anywhere in the method chain
        // (e.g. `::oo::Slot`) without a re-export.
        let (destroy_exp, destroy_unexp, _) = self.method_visibility_flags(obj, b"destroy");
        let destroy_hidden = destroy_unexp && !destroy_exp;
        let mut names: BTreeSet<Vec<u8>> = BTreeSet::new();
        if self.oo.borrow().classes.contains_key(obj) {
            // A class object responds to the class built-ins. `::oo::class`
            // is a singleton with `new` unexported, so it lists only `create`.
            // A class that unexports its own `create`/`new` (e.g. a metaclass
            // with a custom factory) hides them from an external miss.
            let ext = self.oo.borrow().unknown_external;
            let (cre_unexp, new_unexp) = {
                let oo = self.oo.borrow();
                let o = oo.objects.get(obj);
                (
                    o.is_some_and(|o| o.unexported.contains(b"create".as_slice())),
                    o.is_some_and(|o| o.unexported.contains(b"new".as_slice())),
                )
            };
            if !(ext && cre_unexp) {
                names.insert(b"create".to_vec());
            }
            if obj != b"::oo::class" && !(ext && new_unexp) {
                names.insert(b"new".to_vec());
            }
        }
        if !destroy_hidden {
            names.insert(b"destroy".to_vec());
        }
        // The miss's origin: an internal (`my m`) call also lists unexported
        // methods and the `oo::object` built-ins; an external one lists only
        // public (exported) methods.
        let external = self.oo.borrow().unknown_external;
        // An internal call reaches the unexported `oo::object` built-ins too.
        if !external {
            for b in [
                &b"<cloned>"[..],
                b"eval",
                b"unknown",
                b"variable",
                b"varname",
            ] {
                names.insert(b.to_vec());
            }
        }
        // Collect method names from each provider: public always; unexported
        // (non-private) only for an internal call; explicitly-exported names
        // (a promoted built-in) always.
        let collect = |methods: &BTreeMap<Vec<u8>, Method>,
                       unexp: &BTreeSet<Vec<u8>>,
                       exp: &BTreeSet<Vec<u8>>,
                       priv_set: &BTreeSet<Vec<u8>>,
                       names: &mut BTreeSet<Vec<u8>>| {
            for m in methods.keys() {
                if priv_set.contains(m) {
                    continue; // private: handled separately (scope-gated)
                }
                if !unexp.contains(m) || !external {
                    names.insert(m.clone());
                }
            }
            // An explicitly-exported name is listed only if it is a real method
            // or a promotable built-in — not a name merely `export`ed without a
            // backing implementation (oo-4.3).
            const PROMOTABLE: &[&[u8]] =
                &[b"eval", b"variable", b"varname", b"unknown", b"<cloned>"];
            for m in exp {
                if methods.contains_key(m) || PROMOTABLE.contains(&m.as_slice()) {
                    names.insert(m.clone());
                }
            }
        };
        // A private method named is visible (and so listed) only from its own
        // declaring scope — the scope captured at the original invocation (the
        // `unknown` handler's own frame must not mask it; TIP 500). From
        // non-method code there is no scope, so none are listed.
        let caller_scope: Option<Vec<u8>> = self.oo.borrow().unknown_scope.clone();
        for p in self.method_chain(obj) {
            let is_object = p == obj;
            let in_scope = caller_scope.as_deref() == Some(p.as_slice());
            let oo = self.oo.borrow();
            if is_object {
                if let Some(o) = oo.objects.get(&p) {
                    collect(
                        &o.methods,
                        &o.unexported,
                        &o.exported,
                        &o.private,
                        &mut names,
                    );
                    if in_scope {
                        names.extend(o.private.iter().cloned());
                    }
                }
            } else if let Some(c) = oo.classes.get(&p) {
                collect(
                    &c.methods,
                    &c.unexported,
                    &c.exported,
                    &c.private,
                    &mut names,
                );
                if in_scope {
                    names.extend(c.private.iter().cloned());
                }
            }
        }
        let names: Vec<Vec<u8>> = names.into_iter().collect();
        if names.is_empty() {
            // C: an object with no callable methods reports differently.
            let mut msg = b"object \"".to_vec();
            msg.extend_from_slice(obj);
            msg.extend_from_slice(b"\" has no visible methods");
            return self.error(&msg);
        }
        let mut msg = b"unknown method \"".to_vec();
        msg.extend_from_slice(requested);
        msg.extend_from_slice(b"\": must be ");
        for (i, n) in names.iter().enumerate() {
            if i > 0 {
                msg.extend_from_slice(if i == names.len() - 1 { b" or " } else { b", " });
            }
            msg.extend_from_slice(n);
        }
        self.error(&msg)
    }
}

// -- oo::class / oo::object / oo::define / oo::objdefine ---------------------

/// `oo::define class script` or `oo::define class subcommand ?arg ...?`.
fn oo_define_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 3 {
        return wrong_args(interp, b"oo::define target ?arg ...?");
    }
    let fqn = interp.oo_resolve_object(&obj_bytes(argv[1]));
    // C resolves the object first (`Tcl_GetObjectFromObj`): a name that is not
    // an object at all reports differently from an object that is not a class.
    if !interp.oo.borrow().objects.contains_key(&fqn) {
        return not_object(interp, &obj_bytes(argv[1]));
    }
    if !interp.oo.borrow().classes.contains_key(&fqn) {
        let mut m = obj_bytes(argv[1]);
        m.extend_from_slice(b" does not refer to a class");
        return err(interp, &m);
    }
    interp.oo_run_def(DefTarget::Class(fqn), argv)
}

/// `oo::objdefine object script` / `oo::objdefine object subcommand ?arg ...?`.
fn oo_objdefine_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 3 {
        return wrong_args(interp, b"oo::objdefine target ?arg ...?");
    }
    let fqn = interp.oo_resolve_object(&obj_bytes(argv[1]));
    if !interp.oo.borrow().objects.contains_key(&fqn) {
        return not_object(interp, &obj_bytes(argv[1]));
    }
    interp.oo_run_def(DefTarget::Object(fqn), argv)
}

/// `<cloned>` object built-in (C's `TclOO_Object_Cloned`): copy the procedures
/// and variables of the origin object's instance namespace into this object's.
/// Invoked by `oo::copy`; a user `<cloned>` runs first and reaches this via
/// `next`.
fn oo_object_cloned(interp: &mut Interp, obj: &[u8], args: &[*mut TclObj]) -> Code {
    if args.len() != 1 {
        return wrong_args(interp, b"my <cloned> originObject");
    }
    let src = interp.oo_resolve_object(&obj_bytes(args[0]));
    let pair = {
        let oo = interp.oo.borrow();
        match (oo.objects.get(&src), oo.objects.get(obj)) {
            (Some(s), Some(d)) => Some((s.var_ns, d.var_ns)),
            _ => None,
        }
    };
    if let Some((src_ns, dst_ns)) = pair {
        interp.oo_clone_namespace(src_ns, dst_ns);
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

/// `oo::copy srcObject ?targetObject?` — clone an object (its class + per-object
/// methods/mixins).
fn oo_copy_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 || argv.len() > 4 {
        return wrong_args(
            interp,
            b"oo::copy sourceName ?targetName? ?targetNamespace?",
        );
    }
    // The source resolves like a command (current namespace then global), so a
    // copy from inside a namespace still finds a global object (oo-15.1).
    let src = interp.oo_resolve_object(&obj_bytes(argv[1]));
    let Some(src_obj) = interp.oo.borrow().objects.get(&src).cloned() else {
        let mut m = b"\"".to_vec();
        m.extend_from_slice(&obj_bytes(argv[1]));
        m.extend_from_slice(b"\" does not refer to an object");
        return err(interp, &m);
    };
    // An empty (or omitted) target name auto-generates an anonymous name.
    let dst = match argv.get(2).map(|&a| obj_bytes(a)) {
        Some(ref n) if !n.is_empty() => interp.fqn_for(n),
        _ => {
            let n = interp.oo.borrow().counter;
            interp.oo.borrow_mut().counter += 1;
            format!("::oo::Obj{n}").into_bytes()
        }
    };
    // If the source is also a class, clone the class definition too, so the
    // copy is a working class (TclOO copies both the object and class facets).
    let src_cls = interp.oo.borrow().classes.get(&src).cloned();
    // An explicit target namespace becomes the copy's instance-variable
    // namespace; otherwise it defaults to the object's own name.
    let var_ns = match argv.get(3).map(|&a| obj_bytes(a)) {
        Some(ref ns) if !ns.is_empty() => {
            // The target namespace is *created*; an existing one is an error
            // (C's `TclOO_Copy`; oo-15.12).
            if interp.resolve_namespace_name(ns).is_some() {
                let mut m = ns.clone();
                m.extend_from_slice(b" refers to an existing namespace");
                return err(interp, &m);
            }
            let ns = interp.fqn_for(ns);
            interp.ensure_namespace(&ns)
        }
        _ => interp.ensure_namespace(&dst),
    };
    let creation_id = interp.oo_next_id();
    interp.oo.borrow_mut().objects.insert(
        dst.clone(),
        Object {
            class: src_obj.class,
            var_ns,
            creation_id,
            methods: src_obj.methods,
            mixins: src_obj.mixins,
            unexported: src_obj.unexported,
            exported: src_obj.exported,
            private: src_obj.private,
            filters: src_obj.filters,
            variables: src_obj.variables,
            private_variables: src_obj.private_variables,
            readable_properties: src_obj.readable_properties,
            writable_properties: src_obj.writable_properties,
            destroyed: false,
            torn_down: false,
            my_aliases: Vec::new(),
        },
    );
    if let Some(cls) = src_cls {
        interp.oo.borrow_mut().classes.insert(dst.clone(), cls);
    }
    interp.ns_register(&dst, Command::OoObject(dst.clone()));
    interp.oo_register_my(&dst);
    // Run the `<cloned>` method (a user override first, then oo::object's native
    // implementation via `next`) to copy the instance namespace's procedures
    // and variables from the source (oo-15.6, oo-15.8).
    let src_obj = obj::new_string_bytes(&src);
    unsafe { obj::incr_ref_count(src_obj) };
    // A `wrong # args` from a user `<cloned>` names the new object (C reports the
    // copy as `<obj> <cloned> …`), not the internal `my` (oo-15.9). Prime the
    // usage prefix the method body's run_proc consumes.
    let mut usage = dst.clone();
    usage.extend_from_slice(b" <cloned>");
    interp.oo.borrow_mut().fwd_usage = Some(usage);
    let code = interp.oo_invoke(&dst, b"<cloned>", &[src_obj], false);
    interp.oo.borrow_mut().fwd_usage = None;
    unsafe { obj::decr_ref_count(src_obj) };
    if code == Code::Error {
        return Code::Error;
    }
    interp.set_result(obj::new_string_bytes(&dst));
    Code::Ok
}

// -- definition-script commands ---------------------------------------------

/// The current definition target, or an error if outside a definition body.
fn def_target(interp: &mut Interp) -> Result<DefTarget, Code> {
    let target = interp.active_def_target();
    match target {
        Some(t) => Ok(t),
        None => Err(interp
            .set_error(b"this command can only be called from within the body of a definition")),
    }
}

/// A method's declared visibility.
#[derive(Clone, Copy, PartialEq)]
enum MethodVis {
    /// Callable externally (a lowercase name or `-export`).
    Public,
    /// Hidden from external calls but listed by `info methods -private`
    /// (a non-lowercase name or `-unexport`).
    Unexported,
    /// TIP 500 private (`-private`/`private` block): hidden everywhere.
    Private,
}

/// Install `method` into the current definition target (class or object) with
/// the default visibility for its name (or `private` block).
fn install_method(interp: &mut Interp, name: Vec<u8>, m: Method) -> Code {
    let vis = default_vis(interp, &name);
    install_method_vis(interp, name, m, vis)
}

/// Default visibility for a method name in the current definition context.
fn default_vis(interp: &Interp, name: &[u8]) -> MethodVis {
    if interp.oo.borrow().private_depth > 0 {
        MethodVis::Private
    } else if exported_by_default(name) {
        MethodVis::Public
    } else {
        MethodVis::Unexported
    }
}

fn install_method_vis(interp: &mut Interp, name: Vec<u8>, m: Method, vis: MethodVis) -> Code {
    let apply = |methods: &mut BTreeMap<Vec<u8>, Method>,
                 unexp: &mut BTreeSet<Vec<u8>>,
                 priv_set: &mut BTreeSet<Vec<u8>>| {
        methods.insert(name.clone(), m);
        match vis {
            MethodVis::Public => {
                unexp.remove(&name);
                priv_set.remove(&name);
            }
            MethodVis::Unexported => {
                unexp.insert(name.clone());
                priv_set.remove(&name);
            }
            MethodVis::Private => {
                unexp.insert(name.clone());
                priv_set.insert(name.clone());
            }
        }
    };
    let ok = match def_target(interp) {
        Ok(DefTarget::Class(c)) => {
            let mut oo = interp.oo.borrow_mut();
            oo.classes.get_mut(&c).is_some_and(|cl| {
                apply(&mut cl.methods, &mut cl.unexported, &mut cl.private);
                true
            })
        }
        Ok(DefTarget::Object(o)) => {
            let mut oo = interp.oo.borrow_mut();
            oo.objects.get_mut(&o).is_some_and(|ob| {
                apply(&mut ob.methods, &mut ob.unexported, &mut ob.private);
                true
            })
        }
        Err(code) => return code,
    };
    if !ok {
        return interp.set_error(b"no current class/object to define on");
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

/// `private cmd ?arg ...?` / `private { script }` — a definition modifier that
/// marks the methods/variables it defines as private (`my`-only). TIP 500.
fn def_private(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 {
        return wrong_args(interp, b"private cmd ?arg ...?");
    }
    interp.oo.borrow_mut().private_depth += 1;
    let code = if argv.len() == 2 {
        interp.eval_str(&obj_bytes(argv[1]))
    } else {
        interp.dispatch(&argv[1..])
    };
    interp.oo.borrow_mut().private_depth -= 1;
    code
}

/// `deletemethod name ?name ...?` — remove method(s) from the current target.
fn def_deletemethod(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 {
        return wrong_args(interp, b"deletemethod name ?name ...?");
    }
    let target = match def_target(interp) {
        Ok(t) => t,
        Err(c) => return c,
    };
    for &a in &argv[1..] {
        let name = obj_bytes(a);
        let removed = {
            let mut oo = interp.oo.borrow_mut();
            match &target {
                DefTarget::Class(c) => oo.classes.get_mut(c).is_some_and(|cl| {
                    cl.unexported.remove(&name);
                    cl.methods.remove(&name).is_some()
                }),
                DefTarget::Object(o) => oo.objects.get_mut(o).is_some_and(|ob| {
                    ob.unexported.remove(&name);
                    ob.methods.remove(&name).is_some()
                }),
            }
        };
        if !removed {
            let mut m = b"method ".to_vec();
            m.extend_from_slice(&name);
            m.extend_from_slice(b" does not exist");
            return err(interp, &m);
        }
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

/// `renamemethod oldName newName` — rename a method on the current target.
fn def_renamemethod(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 3 {
        return wrong_args(interp, b"renamemethod oldName newName");
    }
    let from = obj_bytes(argv[1]);
    let to = obj_bytes(argv[2]);
    let target = match def_target(interp) {
        Ok(t) => t,
        Err(c) => return c,
    };
    let renamed = {
        let mut oo = interp.oo.borrow_mut();
        match &target {
            DefTarget::Class(c) => oo.classes.get_mut(c).is_some_and(|cl| {
                if let Some(m) = cl.methods.remove(&from) {
                    cl.methods.insert(to.clone(), m);
                    if cl.unexported.remove(&from) {
                        cl.unexported.insert(to.clone());
                    }
                    true
                } else {
                    false
                }
            }),
            DefTarget::Object(o) => oo.objects.get_mut(o).is_some_and(|ob| {
                if let Some(m) = ob.methods.remove(&from) {
                    ob.methods.insert(to.clone(), m);
                    if ob.unexported.remove(&from) {
                        ob.unexported.insert(to.clone());
                    }
                    true
                } else {
                    false
                }
            }),
        }
    };
    if !renamed {
        let mut m = b"method ".to_vec();
        m.extend_from_slice(&from);
        m.extend_from_slice(b" does not exist");
        return err(interp, &m);
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

/// `class className` (objdefine only) — change the object's class.
fn def_class(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 2 {
        return wrong_args(interp, b"class className");
    }
    let obj = match def_target(interp) {
        Ok(DefTarget::Object(o)) => o,
        Ok(DefTarget::Class(_)) => {
            return err(interp, b"attempt to misuse class as object");
        }
        Err(c) => return c,
    };
    // The root classes' class may not be changed (each with its own message).
    if obj == b"::oo::object" {
        return err(interp, b"may not modify the class of the root object class");
    }
    if obj == b"::oo::class" {
        return err(interp, b"may not modify the class of the class of classes");
    }
    let new_cls = interp.oo_resolve_object(&obj_bytes(argv[1]));
    if !interp.oo.borrow().objects.contains_key(&new_cls) {
        return not_object(interp, &obj_bytes(argv[1]));
    }
    if !interp.oo.borrow().classes.contains_key(&new_cls) {
        let mut m = obj_bytes(argv[1]);
        m.extend_from_slice(b" does not refer to a class");
        return err(interp, &m);
    }
    // An object may not become an instance of itself.
    if new_cls == obj {
        return err(
            interp,
            b"may not change classes into an instance of themselves",
        );
    }
    // Changing the class can turn a plain object into a class (its new class is
    // `oo::class` or a metaclass) or back; keep the class-map membership in sync
    // so it gains/loses `create`/`new` and class introspection.
    let becomes_class = interp
        .mro(&new_cls)
        .iter()
        .any(|c| c.as_slice() == b"::oo::class");
    if let Some(ob) = interp.oo.borrow_mut().objects.get_mut(&obj) {
        ob.class = new_cls;
    }
    if becomes_class {
        if !interp.oo.borrow().classes.contains_key(&obj) {
            interp.oo.borrow_mut().classes.insert(
                obj.clone(),
                Class {
                    supers: vec![b"::oo::object".to_vec()],
                    ..Class::default()
                },
            );
        }
    } else if interp.oo.borrow().classes.contains_key(&obj) {
        // A class demoted to a non-class can no longer support its subclasses
        // or instances, so they are torn down (C's `TclOODeleteDescendants`
        // when the class facet goes away; oo-13.6). The object itself survives
        // as a plain object — only its class facet is removed.
        interp.oo_destroy_class_descendants(&obj);
        interp.oo.borrow_mut().classes.remove(&obj);
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

/// `definitionnamespace ?-class|-instance? namespace` (TIP 524) — set the
/// namespace in which this class (or its instances) are defined. An empty
/// namespace name resets to the default.
fn def_definitionnamespace(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let cls = match def_target(interp) {
        Ok(DefTarget::Class(c)) => c,
        Ok(DefTarget::Object(_)) => {
            return err(
                interp,
                b"this command may only be called in a class definition",
            );
        }
        Err(c) => return c,
    };
    if cls == b"::oo::object" || cls == b"::oo::class" {
        return err(
            interp,
            b"may not modify the definition namespace of the root classes",
        );
    }
    if argv.len() != 2 && argv.len() != 3 {
        return wrong_args(interp, b"definitionnamespace ?kind? namespace");
    }
    // Default kind is `-class`.
    let instance = if argv.len() == 3 {
        match obj_bytes(argv[1]).as_slice() {
            b"-class" => false,
            b"-instance" => true,
            other => {
                let mut m = b"bad kind \"".to_vec();
                m.extend_from_slice(other);
                m.extend_from_slice(b"\": must be -class or -instance");
                return err(interp, &m);
            }
        }
    } else {
        false
    };
    let ns_arg = obj_bytes(argv[argv.len() - 1]);
    let stored = if ns_arg.is_empty() {
        None
    } else {
        match interp.resolve_namespace_name(&ns_arg) {
            Some(qn) => Some(qn),
            None => {
                let mut m = b"namespace \"".to_vec();
                m.extend_from_slice(&ns_arg);
                m.extend_from_slice(b"\" not found");
                return err(interp, &m);
            }
        }
    };
    if let Some(c) = interp.oo.borrow_mut().classes.get_mut(&cls) {
        if instance {
            c.def_ns = stored;
        } else {
            c.class_def_ns = stored;
        }
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

fn def_method(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    // `method name ?-export|-unexport|-private? args body` (TIP 500 flags).
    let name = obj_bytes(argv[1]);
    let (flag_vis, rest): (Option<MethodVis>, &[*mut TclObj]) =
        match argv.get(2).map(|&a| obj_bytes(a)) {
            Some(f) if f == b"-private" => (Some(MethodVis::Private), &argv[3..]),
            Some(f) if f == b"-export" => (Some(MethodVis::Public), &argv[3..]),
            Some(f) if f == b"-unexport" => (Some(MethodVis::Unexported), &argv[3..]),
            Some(f) if f.starts_with(b"-") => {
                let mut m = b"bad export flag \"".to_vec();
                m.extend_from_slice(&f);
                m.extend_from_slice(b"\": must be -export, -private, or -unexport");
                return err(interp, &m);
            }
            _ => (None, &argv[2..]),
        };
    if rest.len() != 2 {
        return wrong_args(interp, b"method name ?option? args body");
    }
    let params = match crate::cmd_proc::parse_params(&obj_bytes(rest[0])) {
        Ok(p) => p,
        Err(e) => return err(interp, &e),
    };
    let vis = flag_vis.unwrap_or_else(|| default_vis(interp, &name));
    // Source provenance for `info frame` (TIP 280): the body word is the last
    // argument; its file-absolute line (minus one) is the body's line base.
    let src = method_body_src(interp, argv.len() - 1);
    install_method_vis(
        interp,
        name,
        Method::Body {
            params,
            body: obj_bytes(rest[1]),
            src,
        },
        vis,
    )
}

/// The source provenance `(file, body_line_base)` for a method/constructor body
/// at argument index `body_idx`, when defined while sourcing a file; `None`
/// otherwise (an eval-defined body keeps body-relative lines).
fn method_body_src(interp: &Interp, body_idx: usize) -> Option<(Rc<[u8]>, u32)> {
    interp
        .current_source_file()
        .map(|f| (f, interp.arg_line(body_idx).saturating_sub(1)))
}

/// A method/forward is exported by default only when its name begins with an
/// ASCII lowercase letter (TclOO's naming convention).
fn exported_by_default(name: &[u8]) -> bool {
    name.first().is_some_and(u8::is_ascii_lowercase)
}

/// `forward name cmdPrefix ?arg ...?` — a method that calls a command prefix.
fn def_forward(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 3 {
        return wrong_args(interp, b"forward name cmdName ?arg ...?");
    }
    let prefix: Vec<Vec<u8>> = argv[2..].iter().map(|&a| obj_bytes(a)).collect();
    install_method(interp, obj_bytes(argv[1]), Method::Forward { prefix })
}

fn def_constructor(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 3 {
        return wrong_args(interp, b"constructor arguments body");
    }
    let class = match def_target(interp) {
        Ok(DefTarget::Class(c)) => c,
        Ok(DefTarget::Object(_)) => return err(interp, b"constructors are only for classes"),
        Err(code) => return code,
    };
    let params = match crate::cmd_proc::parse_params(&obj_bytes(argv[1])) {
        Ok(p) => p,
        Err(e) => return err(interp, &e),
    };
    let src = method_body_src(interp, 2);
    interp
        .oo
        .borrow_mut()
        .classes
        .get_mut(&class)
        .unwrap()
        .constructor = Some(Method::Body {
        params,
        body: obj_bytes(argv[2]),
        src,
    });
    interp.set_result_bytes(b"");
    Code::Ok
}

fn def_destructor(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 2 {
        return wrong_args(interp, b"destructor body");
    }
    let class = match def_target(interp) {
        Ok(DefTarget::Class(c)) => c,
        Ok(DefTarget::Object(_)) => return err(interp, b"destructors are only for classes"),
        Err(code) => return code,
    };
    interp
        .oo
        .borrow_mut()
        .classes
        .get_mut(&class)
        .unwrap()
        .destructor = Some(obj_bytes(argv[1]));
    interp.set_result_bytes(b"");
    Code::Ok
}

fn def_superclass(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let class = match def_target(interp) {
        Ok(DefTarget::Class(c)) => c,
        Ok(DefTarget::Object(_)) => return err(interp, b"superclass is only for classes"),
        Err(code) => return code,
    };
    let raw: Vec<Vec<u8>> = argv[1..].iter().map(|&a| obj_bytes(a)).collect();
    let (op, vals) = match slot_op_split(&raw, SlotOp::Set) {
        Ok(x) => x,
        Err(m) => return err(interp, &m),
    };
    // TIP 516: resolve each name as a class (current namespace, then global —
    // C's `Slot_ResolveClass`), storing the canonical FQN so the slot tracks
    // classes, not strings. An unresolvable name is kept as-is; validation runs
    // in the setter, so only the items being *added* are checked.
    let resolved: Vec<Vec<u8>> = vals.iter().map(|v| interp.oo_resolve_object(v)).collect();
    // C resolves each via Tcl_GetObjectFromObj (→ `X does not refer to an
    // object`, as-written) then requires it be a class (→ `only a class can be
    // a superclass`).
    if op_validates(&op) {
        for (s, raw) in resolved.iter().zip(vals.iter()) {
            if !interp.oo.borrow().objects.contains_key(s) {
                return not_object(interp, raw);
            }
            if !interp.oo.borrow().classes.contains_key(s) {
                return err(interp, b"only a class can be a superclass");
            }
        }
    }
    let current = interp
        .oo
        .borrow()
        .classes
        .get(&class)
        .map(|c| c.supers.clone())
        .unwrap_or_default();
    let mut new = slot_apply(&op, &current, &resolved);
    // A class may be a direct superclass at most once.
    if has_duplicate(&new) {
        return err(interp, b"class should only be a direct superclass once");
    }
    // A class may not (transitively) be its own superclass.
    if new.iter().any(|s| s == &class) || self_reachable(interp, &class, &new) {
        return err(interp, b"attempt to form circular dependency graph");
    }
    if new.is_empty() {
        // Zero superclasses defaults to a single root: `oo::class` for a class
        // that is itself a class (a metaclass — reachable from `oo::class`), so
        // its instances stay classes; otherwise `oo::object` (C's
        // ClassSuperclassSet, Bug 9d61624b3d; oo-35.2).
        let is_metaclass = self_reachable(interp, b"::oo::class", &current);
        new = vec![if is_metaclass {
            b"::oo::class".to_vec()
        } else {
            b"::oo::object".to_vec()
        }];
    }
    interp
        .oo
        .borrow_mut()
        .classes
        .get_mut(&class)
        .unwrap()
        .supers = new;
    interp.set_result_bytes(b"");
    Code::Ok
}

/// Whether `class` is reachable through the proposed superclass list (a cycle).
fn self_reachable(interp: &Interp, class: &[u8], supers: &[Vec<u8>]) -> bool {
    let mut stack: Vec<Vec<u8>> = supers.to_vec();
    let mut seen: Vec<Vec<u8>> = Vec::new();
    while let Some(s) = stack.pop() {
        if s == class {
            return true;
        }
        if seen.contains(&s) {
            continue;
        }
        seen.push(s.clone());
        if let Some(cl) = interp.oo.borrow().classes.get(&s) {
            stack.extend(cl.supers.clone());
        }
    }
    false
}

/// A TIP 380 slot operation (`superclass`/`mixin`/`variable`/`filter` accept
/// these instead of a bare value list).
enum SlotOp {
    Set,
    Append,
    Prepend,
    Remove,
    AppendIfNew,
    Clear,
}

/// Split a slot command's args into `(operation, values)`: a leading
/// `-set`/`-append`/… selects the op; any other leading `-flag` is an unknown
/// slot operation (`Err` with the C unknown-method message); a non-`-` first
/// arg uses the slot's `default` operation over all args.
fn slot_op_split(args: &[Vec<u8>], default: SlotOp) -> Result<(SlotOp, &[Vec<u8>]), Vec<u8>> {
    Ok(match args.first().map(|a| a.as_slice()) {
        Some(b"-set") => (SlotOp::Set, &args[1..]),
        Some(b"-append") => (SlotOp::Append, &args[1..]),
        Some(b"-prepend") => (SlotOp::Prepend, &args[1..]),
        Some(b"-remove") => (SlotOp::Remove, &args[1..]),
        Some(b"-appendifnew") => (SlotOp::AppendIfNew, &args[1..]),
        Some(b"-clear") => (SlotOp::Clear, &[]),
        Some(op) if op.first() == Some(&b'-') => {
            let mut m = b"unknown method \"".to_vec();
            m.extend_from_slice(op);
            m.extend_from_slice(
                b"\": must be -append, -appendifnew, -clear, -prepend, -remove or -set",
            );
            return Err(m);
        }
        _ => (default, args),
    })
}

/// Whether a slot op *adds* items (so the class/object slots must validate the
/// supplied values). `-remove`/`-clear` only drop items, so a now-deleted name
/// is harmless and is not validated (C validates in the setter, over the
/// resulting list — which for these ops is a subset of the already-valid set).
fn op_validates(op: &SlotOp) -> bool {
    matches!(
        op,
        SlotOp::Set | SlotOp::Append | SlotOp::Prepend | SlotOp::AppendIfNew
    )
}

/// Apply a slot op to the current list, yielding the new list.
fn slot_apply(op: &SlotOp, current: &[Vec<u8>], values: &[Vec<u8>]) -> Vec<Vec<u8>> {
    match op {
        SlotOp::Set => values.to_vec(),
        SlotOp::Clear => Vec::new(),
        SlotOp::Append => current.iter().chain(values).cloned().collect(),
        SlotOp::Prepend => values.iter().chain(current).cloned().collect(),
        SlotOp::Remove => current
            .iter()
            .filter(|c| !values.contains(c))
            .cloned()
            .collect(),
        SlotOp::AppendIfNew => {
            let mut v = current.to_vec();
            for x in values {
                if !v.contains(x) {
                    v.push(x.clone());
                }
            }
            v
        }
    }
}

// -- the `::oo::Slot` class (TIP 380) ----------------------------------------
//
// The public operations are *native* methods so they add no Tcl call frame:
// the overridable `Get`/`Set`/`Resolve` they invoke therefore run at the same
// `info level` the C implementation reports.

/// Invoke `obj`'s `method` with byte-string `args` (internally, so private
/// `Get`/`Set`/`Resolve` are reachable), returning its result bytes.
fn slot_call(
    interp: &mut Interp,
    obj: &[u8],
    method: &[u8],
    args: &[Vec<u8>],
) -> Result<Vec<u8>, Code> {
    let argv: Vec<*mut TclObj> = args.iter().map(|a| obj::new_string_bytes(a)).collect();
    for &a in &argv {
        unsafe { obj::incr_ref_count(a) };
    }
    let code = interp.oo_invoke(obj, method, &argv, false);
    for &a in &argv {
        unsafe { obj::decr_ref_count(a) };
    }
    if code == Code::Ok {
        Ok(interp.result_bytes())
    } else {
        Err(code)
    }
}

/// Parse a Tcl list string into its elements.
fn parse_list(s: &[u8]) -> Vec<Vec<u8>> {
    let o = obj::new_string_bytes(s);
    unsafe { obj::incr_ref_count(o) };
    let out = match list::list_elements(o) {
        Ok(elems) => elems.iter().map(|&e| obj_bytes(e)).collect(),
        Err(_) => Vec::new(),
    };
    unsafe { obj::decr_ref_count(o) };
    out
}

/// Build a Tcl list string from elements.
fn build_list(elems: &[Vec<u8>]) -> Vec<u8> {
    let objs: Vec<*mut TclObj> = elems.iter().map(|e| obj::new_string_bytes(e)).collect();
    let l = list::new_list_obj(&objs);
    let s = obj_bytes(l);
    crate::interp::drop_fresh(l);
    s
}

/// Run a slot operation: `Resolve` each arg, `Get` the current list (except for
/// `-set`/`-clear`), apply the op, then `Set` the result.
fn slot_run_op(interp: &mut Interp, obj: &[u8], op: &SlotOp, args: &[*mut TclObj]) -> Code {
    let new = match op {
        SlotOp::Clear => Vec::new(),
        _ => {
            // Resolve each argument in turn.
            let mut resolved: Vec<Vec<u8>> = Vec::with_capacity(args.len());
            for &a in args {
                match slot_call(interp, obj, b"Resolve", &[obj_bytes(a)]) {
                    Ok(r) => resolved.push(r),
                    Err(c) => return c,
                }
            }
            if matches!(op, SlotOp::Set) {
                resolved
            } else {
                let cur = match slot_call(interp, obj, b"Get", &[]) {
                    Ok(r) => parse_list(&r),
                    Err(c) => return c,
                };
                slot_apply(op, &cur, &resolved)
            }
        }
    };
    match slot_call(interp, obj, b"Set", &[build_list(&new)]) {
        Ok(_) => {
            interp.set_result_bytes(b"");
            Code::Ok
        }
        Err(c) => c,
    }
}

fn slot_m_set(i: &mut Interp, o: &[u8], a: &[*mut TclObj]) -> Code {
    slot_run_op(i, o, &SlotOp::Set, a)
}
fn slot_m_append(i: &mut Interp, o: &[u8], a: &[*mut TclObj]) -> Code {
    slot_run_op(i, o, &SlotOp::Append, a)
}
fn slot_m_prepend(i: &mut Interp, o: &[u8], a: &[*mut TclObj]) -> Code {
    slot_run_op(i, o, &SlotOp::Prepend, a)
}
fn slot_m_remove(i: &mut Interp, o: &[u8], a: &[*mut TclObj]) -> Code {
    slot_run_op(i, o, &SlotOp::Remove, a)
}
fn slot_m_appendifnew(i: &mut Interp, o: &[u8], a: &[*mut TclObj]) -> Code {
    slot_run_op(i, o, &SlotOp::AppendIfNew, a)
}
fn slot_m_clear(i: &mut Interp, o: &[u8], a: &[*mut TclObj]) -> Code {
    slot_run_op(i, o, &SlotOp::Clear, a)
}

/// `oo::object`'s built-in `unknown` method: the standard "unknown method"
/// error. It is the end of the `unknown` call chain, so a user `unknown` that
/// does `next` lands here. `args[0]` is the originally-requested method name.
/// With *no* args it was reached via a no-method invocation (`$obj`), whose
/// default outcome is the `wrong # args` usage (C's `FORCE_UNKNOWN` path).
fn oo_object_unknown(interp: &mut Interp, obj: &[u8], args: &[*mut TclObj]) -> Code {
    let Some(&first) = args.first() else {
        let mut u = obj.to_vec();
        u.extend_from_slice(b" method ?arg ...?");
        return wrong_args(interp, &u);
    };
    let method = obj_bytes(first);
    interp.oo_unknown_method(obj, &method)
}

/// `mixin ?class ...?` — set the mixins of the current class/object.
/// `filter ?methodName ...?` — set the filter methods on the def target (class
/// or object). Filters wrap every public method call on instances.
fn def_filter(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let raw: Vec<Vec<u8>> = argv[1..].iter().map(|&a| obj_bytes(a)).collect();
    // The `filter` slot's default operation is `-append` (its DeclaredSlot
    // defOp is NULL, so it inherits the base Slot default).
    let (op, vals) = match slot_op_split(&raw, SlotOp::Append) {
        Ok(x) => x,
        Err(m) => return err(interp, &m),
    };
    let target = match def_target(interp) {
        Ok(t) => t,
        Err(c) => return c,
    };
    let current = match &target {
        DefTarget::Class(c) => interp
            .oo
            .borrow()
            .classes
            .get(c)
            .map(|cl| cl.filters.clone())
            .unwrap_or_default(),
        DefTarget::Object(o) => interp
            .oo
            .borrow()
            .objects
            .get(o)
            .map(|ob| ob.filters.clone())
            .unwrap_or_default(),
    };
    let filters = slot_apply(&op, &current, vals);
    match target {
        DefTarget::Class(c) => {
            if let Some(cl) = interp.oo.borrow_mut().classes.get_mut(&c) {
                cl.filters = filters;
            }
        }
        DefTarget::Object(o) => {
            if let Some(ob) = interp.oo.borrow_mut().objects.get_mut(&o) {
                ob.filters = filters;
            }
        }
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

fn def_mixin(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let raw: Vec<Vec<u8>> = argv[1..].iter().map(|&a| obj_bytes(a)).collect();
    let (op, vals) = match slot_op_split(&raw, SlotOp::Set) {
        Ok(x) => x,
        Err(m) => return err(interp, &m),
    };
    // Resolve each value to a class FQN (TIP 516: current namespace then
    // global, C's `Slot_ResolveClass` — an unresolvable name is kept as-is).
    // Validation (`X does not refer to an object`, as-written, then `may only
    // mix in classes`) happens in the setter, so only the items being *added*
    // are checked; `-remove`/`-clear` may name a now-deleted class harmlessly.
    let resolved: Vec<Vec<u8>> = vals.iter().map(|v| interp.oo_resolve_object(v)).collect();
    if op_validates(&op) {
        for (mx, raw) in resolved.iter().zip(vals.iter()) {
            if !interp.oo.borrow().objects.contains_key(mx) {
                return not_object(interp, raw);
            }
            if !interp.oo.borrow().classes.contains_key(mx) {
                return err(interp, b"may only mix in classes");
            }
        }
    }
    let target = match def_target(interp) {
        Ok(t) => t,
        Err(c) => return c,
    };
    // A class may not mix itself in.
    if let DefTarget::Class(c) = &target {
        if resolved.iter().any(|mx| mx == c) {
            return err(interp, b"may not mix a class into itself");
        }
    }
    let current = match &target {
        DefTarget::Class(c) => interp
            .oo
            .borrow()
            .classes
            .get(c)
            .map(|cl| cl.mixins.clone()),
        DefTarget::Object(o) => interp
            .oo
            .borrow()
            .objects
            .get(o)
            .map(|ob| ob.mixins.clone()),
    }
    .unwrap_or_default();
    let new = slot_apply(&op, &current, &resolved);
    // A class may be a direct mixin at most once.
    if has_duplicate(&new) {
        return err(interp, b"class should only be a direct mixin once");
    }
    match target {
        DefTarget::Class(c) => {
            if let Some(cl) = interp.oo.borrow_mut().classes.get_mut(&c) {
                cl.mixins = new;
            }
        }
        DefTarget::Object(o) => {
            if let Some(ob) = interp.oo.borrow_mut().objects.get_mut(&o) {
                ob.mixins = new;
            }
        }
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

/// Whether `list` contains a duplicate element.
fn has_duplicate(list: &[Vec<u8>]) -> bool {
    list.iter().enumerate().any(|(i, x)| list[..i].contains(x))
}

fn def_variable(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    // Outside a definition body (incl. a nested proc/method call from one), this
    // is the ordinary `variable` command.
    if interp.active_def_target().is_none() {
        return crate::cmd_var::variable(interp, argv);
    }
    // The `variable` slot's default operation is `-append` (its DeclaredSlot
    // defOp is NULL, so it inherits the base Slot default).
    let raw: Vec<Vec<u8>> = argv[1..].iter().map(|&a| obj_bytes(a)).collect();
    let (op, names) = match slot_op_split(&raw, SlotOp::Append) {
        Ok(x) => x,
        Err(m) => return err(interp, &m),
    };
    // A declared variable name must be a plain scalar: no namespace separator,
    // no array element.
    for n in names {
        if n.windows(2).any(|w| w == b"::") {
            let mut m = b"invalid declared variable name \"".to_vec();
            m.extend_from_slice(n);
            m.extend_from_slice(b"\": must not contain namespace separators");
            return err(interp, &m);
        }
        if n.contains(&b'(') || n.last() == Some(&b')') {
            let mut m = b"invalid declared variable name \"".to_vec();
            m.extend_from_slice(n);
            m.extend_from_slice(b"\": must not refer to an array element");
            return err(interp, &m);
        }
    }
    let target = match def_target(interp) {
        Ok(t) => t,
        Err(code) => return code,
    };
    // Inside a `private { … }` block the names are TIP 500 private variables,
    // tracked separately so introspection can distinguish them.
    let private = interp.oo.borrow().private_depth > 0;
    let current = match (&target, private) {
        (DefTarget::Class(c), false) => interp
            .oo
            .borrow()
            .classes
            .get(c)
            .map(|cl| cl.variables.clone())
            .unwrap_or_default(),
        (DefTarget::Class(c), true) => interp
            .oo
            .borrow()
            .classes
            .get(c)
            .map(|cl| cl.private_variables.clone())
            .unwrap_or_default(),
        (DefTarget::Object(o), false) => interp
            .oo
            .borrow()
            .objects
            .get(o)
            .map(|ob| ob.variables.clone())
            .unwrap_or_default(),
        (DefTarget::Object(o), true) => interp
            .oo
            .borrow()
            .objects
            .get(o)
            .map(|ob| ob.private_variables.clone())
            .unwrap_or_default(),
    };
    // De-duplicate the appended result (a variable is declared once), mirroring
    // C's uniquifying Set.
    let mut applied = slot_apply(&op, &current, names);
    let mut seen: Vec<Vec<u8>> = Vec::with_capacity(applied.len());
    applied.retain(|n| {
        if seen.contains(n) {
            false
        } else {
            seen.push(n.clone());
            true
        }
    });
    match (target, private) {
        (DefTarget::Class(c), false) => {
            if let Some(cl) = interp.oo.borrow_mut().classes.get_mut(&c) {
                cl.variables = applied;
            }
        }
        (DefTarget::Class(c), true) => {
            if let Some(cl) = interp.oo.borrow_mut().classes.get_mut(&c) {
                cl.private_variables = applied;
            }
        }
        (DefTarget::Object(o), false) => {
            if let Some(ob) = interp.oo.borrow_mut().objects.get_mut(&o) {
                ob.variables = applied;
            }
        }
        (DefTarget::Object(o), true) => {
            if let Some(ob) = interp.oo.borrow_mut().objects.get_mut(&o) {
                ob.private_variables = applied;
            }
        }
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

/// `export`/`unexport name ...` — set method visibility on the current target.
/// Tracks both the `unexported` set (a method hidden from public dispatch) and
/// the `exported` set (a default-unexported built-in promoted to public).
fn def_export(interp: &mut Interp, argv: &[*mut TclObj], export: bool) -> Code {
    let names: Vec<Vec<u8>> = argv[1..].iter().map(|&a| obj_bytes(a)).collect();
    // Export/unexport also clears the TIP 500 private flag: the method becomes
    // plain public/unexported (oo-40.2/40.3).
    let apply = |unexp: &mut BTreeSet<Vec<u8>>,
                 exp: &mut BTreeSet<Vec<u8>>,
                 priv_set: &mut BTreeSet<Vec<u8>>| {
        for n in &names {
            priv_set.remove(n);
            if export {
                unexp.remove(n);
                exp.insert(n.clone());
            } else {
                exp.remove(n);
                unexp.insert(n.clone());
            }
        }
    };
    match def_target(interp) {
        Ok(DefTarget::Class(c)) => {
            let mut oo = interp.oo.borrow_mut();
            if let Some(cl) = oo.classes.get_mut(&c) {
                apply(&mut cl.unexported, &mut cl.exported, &mut cl.private);
            }
        }
        Ok(DefTarget::Object(o)) => {
            let mut oo = interp.oo.borrow_mut();
            if let Some(ob) = oo.objects.get_mut(&o) {
                apply(&mut ob.unexported, &mut ob.exported, &mut ob.private);
            }
        }
        Err(code) => return code,
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

// -- method context: self / my / next ---------------------------------------

fn self_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let ctx = interp.oo.borrow().call_stack.last().map(|frame| {
        let step = frame.chain.get(frame.index);
        (
            frame.object.clone(),
            step.map(|s| s.provider.clone()).unwrap_or_default(),
            step.map(|s| s.method.clone()).unwrap_or_default(),
            frame.target.clone(),
        )
    });
    let Some((object, class, method, target)) = ctx else {
        // Define-context `self`: inside an `oo::define`/`oo::class create` body,
        // `self ?subcmd …?` applies objdefine-style directives to the object
        // being defined (the classes-as-objects model). `self` alone returns it.
        let def_target = interp.active_def_target();
        if let Some(target) = def_target {
            // C has two `self` commands: `oo::objdefine`'s takes no arguments
            // (`TclOODefineObjSelfObjCmd`), while `oo::define`'s applies
            // class-side directives (`TclOODefineSelfObjCmd`).
            let (tfqn, is_class) = match target {
                DefTarget::Class(c) => (c, true),
                DefTarget::Object(o) => (o, false),
            };
            if argv.len() == 1 {
                interp.set_result(obj::new_string_bytes(&tfqn));
                return Code::Ok;
            }
            if !is_class {
                return wrong_args(interp, b"self");
            }
            let lvl = interp.current_level();
            // The class-as-object's creation id comes from the enclosing
            // `oo::define` entry (its name may have been renamed in the body, so
            // a lookup by `tfqn` could already miss).
            let creation_id = interp
                .oo
                .borrow()
                .def_stack
                .last()
                .and_then(|(_, _, c)| *c)
                .or_else(|| interp.oo.borrow().objects.get(&tfqn).map(|o| o.creation_id));
            // The object being defined may have been deleted earlier in the body
            // (e.g. `rename ::foo {}`); `self` then can't run (oo-18.11).
            let alive = creation_id.is_some_and(|id| {
                interp
                    .oo
                    .borrow()
                    .objects
                    .values()
                    .any(|o| o.creation_id == id)
            });
            if !alive {
                return err(
                    interp,
                    b"this command cannot be called when the object has been deleted",
                );
            }
            interp.oo.borrow_mut().def_stack.push((
                DefTarget::Object(tfqn.clone()),
                lvl,
                creation_id,
            ));
            // `self { script }` runs a definition body; `self subcmd …` is one
            // directive — mirror `oo_run_def`'s script-vs-subcommand split.
            let is_body = argv.len() == 2;
            let code = if is_body {
                interp.eval_str(&obj_bytes(argv[1]))
            } else {
                interp.dispatch(&argv[1..])
            };
            interp.oo.borrow_mut().def_stack.pop();
            // The `self { script }` body is a class-side definition: on error it
            // adds its own `(in definition script for class object "X")` frame.
            if is_body && code == Code::Error {
                interp.add_def_script_frame(b"class object", &tfqn, creation_id);
            }
            return code;
        }
        return err(interp, b"self may only be called from inside a method");
    };
    match argv.get(1).map(|&a| obj_bytes(a)).as_deref() {
        None | Some(b"object") => interp.set_result(obj::new_string_bytes(&object)),
        // `self class` is the *declaring class* of the running method; a method
        // defined directly on the object (objdefine / class-side `self method`)
        // has no declaring class (C: `declaringClassPtr == NULL`).
        Some(b"class") if class == object => return err(interp, b"method not defined by a class"),
        Some(b"class") => interp.set_result(obj::new_string_bytes(&class)),
        Some(b"method") => interp.set_result(obj::new_string_bytes(&method)),
        Some(b"namespace") => {
            let ns = interp.oo.borrow().objects.get(&object).map(|o| o.var_ns);
            let name = ns
                .map(|n| interp.namespaces().qualified_name(n))
                .unwrap_or_default();
            interp.set_result(obj::new_string_bytes(&name));
        }
        // `self target` — the actual (filtered) method as a `{class method}`
        // pair: the provider/method of the first non-filter step.
        Some(b"target") => {
            let pair = interp.oo.borrow().call_stack.last().and_then(|f| {
                f.chain
                    .iter()
                    .find(|s| s.method == target)
                    .map(|s| (s.provider.clone(), s.method.clone()))
            });
            // A filter wrapping a built-in target (e.g. `destroy`) has no chain
            // step for it — the built-in is the implicit terminus — so name its
            // declaring class directly (`::oo::object` for the object built-ins,
            // `::oo::class` for the instantiation ones; oo-12.8).
            let pair = pair.or_else(|| {
                let decl: &[u8] = match target.as_slice() {
                    b"destroy" | b"eval" | b"variable" | b"varname" | b"unknown" | b"<cloned>" => {
                        b"::oo::object"
                    }
                    b"create" | b"new" | b"createWithNamespace" => b"::oo::class",
                    _ => return None,
                };
                Some((decl.to_vec(), target.clone()))
            });
            match pair {
                Some((p, m)) => {
                    let objs = [obj::new_string_bytes(&p), obj::new_string_bytes(&m)];
                    interp.set_result(crate::list::new_list_obj(&objs));
                }
                None => interp.set_result_bytes(b""),
            }
        }
        // `self call` — `{chain index}`: the full call chain (each step a
        // `{callType method declarer methodType}` element) and the current index.
        Some(b"call") => {
            let (chain, index, target) = interp
                .oo
                .borrow()
                .call_stack
                .last()
                .map(|f| (f.chain.clone(), f.index, f.target.clone()))
                .unwrap_or_default();
            let elems: Vec<Vec<u8>> = chain
                .iter()
                .map(|s| {
                    let is_object = s.provider == object;
                    // A step whose method differs from the invoked target is a
                    // filter wrapping the call.
                    let is_filter = !s.method.is_empty() && s.method != target;
                    // Constructors carry an empty method name, rendered
                    // `<constructor>`; destructors use `<destructor>`.
                    let display: &[u8] = if s.method.is_empty() {
                        b"<constructor>"
                    } else {
                        &s.method
                    };
                    let call_type: &[u8] = if is_filter {
                        b"filter"
                    } else if interp.method_is_private(&s.provider, &s.method, is_object) {
                        b"private"
                    } else {
                        b"method"
                    };
                    call_chain_elem(
                        interp,
                        call_type,
                        display,
                        &s.method,
                        &s.provider,
                        is_object,
                    )
                })
                .collect();
            let inner_objs: Vec<*mut TclObj> =
                elems.iter().map(|e| obj::new_string_bytes(e)).collect();
            let inner = crate::list::new_list_obj(&inner_objs);
            let inner_str = obj_bytes(inner);
            crate::interp::drop_fresh(inner);
            let outer = [
                obj::new_string_bytes(&inner_str),
                obj::new_string_bytes(index.to_string().as_bytes()),
            ];
            interp.set_result(crate::list::new_list_obj(&outer));
        }
        Some(other) => {
            let mut m = b"unsupported self subcommand \"".to_vec();
            m.extend_from_slice(other);
            m.push(b'"');
            return err(interp, &m);
        }
    }
    Code::Ok
}

impl Interp {
    /// Register the per-object `my` command in the object's namespace (`<fqn>::my`).
    pub(crate) fn oo_register_my(&mut self, fqn: &[u8]) {
        let mut name = fqn.to_vec();
        name.extend_from_slice(b"::my");
        self.ns_register(&name, Command::Builtin(my_cmd));
        // `myclass` (TIP 478): a per-object command dispatching on the object's
        // class, for invoking class-side (`self method`) methods.
        let mut mc = fqn.to_vec();
        mc.extend_from_slice(b"::myclass");
        self.ns_register(&mc, Command::Builtin(myclass_cmd));
        // Remember these names so the object's teardown deletes them even if
        // they are later renamed out of the instance namespace.
        if let Some(o) = self.oo.borrow_mut().objects.get_mut(fqn) {
            o.my_aliases = vec![name, mc];
        }
    }

    /// The `info cmdtype` classification of the per-object private command `fqn`
    /// (a `my`/`myclass` builtin): `privateObject` for an object's `my`,
    /// `privateClass` for its `myclass`, else `None`. Matched against the
    /// object's tracked `my_aliases` so a renamed command is still classified.
    pub(crate) fn oo_private_cmd_kind(&self, fqn: &[u8]) -> Option<&'static [u8]> {
        let oo = self.oo.borrow();
        for o in oo.objects.values() {
            match o.my_aliases.iter().position(|a| a.as_slice() == fqn) {
                Some(0) => return Some(b"privateObject"),
                Some(1) => return Some(b"privateClass"),
                _ => {}
            }
        }
        None
    }

    /// Whether evaluation is currently inside an `oo::define`/`oo::objdefine`
    /// body (so `dispatch` should try definition-subcommand resolution on a miss).
    pub(crate) fn in_oo_define(&self) -> bool {
        self.active_def_target().is_some()
    }

    /// The definition target whose body is being evaluated *directly* at the
    /// current call-frame level — `None` inside a nested proc/method called from
    /// a definition body (where the definition commands are out of scope).
    fn active_def_target(&self) -> Option<DefTarget> {
        let lvl = self.current_level();
        self.oo
            .borrow()
            .def_stack
            .last()
            .filter(|(_, l, _)| *l == lvl)
            .map(|(t, _, _)| t.clone())
    }

    /// Allocate the next monotonic object creation ID.
    fn oo_next_id(&self) -> u64 {
        let mut oo = self.oo.borrow_mut();
        oo.next_id += 1;
        oo.next_id
    }

    /// Resolve an unknown command inside an `oo::define`/`oo::objdefine` body as
    /// a definition subcommand, matching C's ensemble: an exact name or a unique
    /// prefix (`super` → `superclass`, `forw` → `forward`). Returns `None` when
    /// the name is not a (unique) define subcommand, so dispatch falls through to
    /// the normal `unknown`/`invalid command name` path (an ambiguous prefix like
    /// `m` is likewise left for that error). The full-name subcommands are also
    /// registered globally, so this only fires for abbreviations and the
    /// subcommands without a global builtin (`class`/`deletemethod`/
    /// `renamemethod`).
    pub(crate) fn oo_define_command(&mut self, name: &[u8], argv: &[*mut TclObj]) -> Option<Code> {
        const CLASS_CMDS: &[&[u8]] = &[
            b"constructor",
            b"definitionnamespace",
            b"deletemethod",
            b"destructor",
            b"export",
            b"filter",
            b"forward",
            b"method",
            b"mixin",
            b"private",
            b"renamemethod",
            b"self",
            b"superclass",
            b"unexport",
            b"variable",
        ];
        const OBJ_CMDS: &[&[u8]] = &[
            b"class",
            b"deletemethod",
            b"export",
            b"filter",
            b"forward",
            b"method",
            b"mixin",
            b"private",
            b"renamemethod",
            b"self",
            b"unexport",
            b"variable",
        ];
        let is_object = matches!(self.active_def_target(), Some(DefTarget::Object(_)));
        let cands: &[&[u8]] = if is_object { OBJ_CMDS } else { CLASS_CMDS };
        // Exact name wins; otherwise a unique prefix (ambiguous → not resolved).
        let matched: Option<&[u8]> = if let Some(c) = cands.iter().find(|c| **c == name) {
            Some(c)
        } else {
            let mut it = cands.iter().filter(|c| c.starts_with(name));
            match (it.next(), it.next()) {
                (Some(c), None) => Some(c),
                _ => None,
            }
        };
        let Some(full) = matched else {
            // Not a standard definition subcommand: a user-set definition
            // namespace (TIP 524) contributes commands too — resolve `name`
            // there (exact or unique prefix) and dispatch the qualified form.
            return self.oo_define_ns_command(name, argv);
        };
        Some(match full {
            b"method" => def_method(self, argv),
            b"constructor" => def_constructor(self, argv),
            b"destructor" => def_destructor(self, argv),
            b"superclass" => def_superclass(self, argv),
            b"variable" => def_variable(self, argv),
            b"export" => def_export(self, argv, true),
            b"unexport" => def_export(self, argv, false),
            b"mixin" => def_mixin(self, argv),
            b"forward" => def_forward(self, argv),
            b"filter" => def_filter(self, argv),
            b"private" => def_private(self, argv),
            b"self" => self_cmd(self, argv),
            b"deletemethod" => def_deletemethod(self, argv),
            b"renamemethod" => def_renamemethod(self, argv),
            b"class" => def_class(self, argv),
            b"definitionnamespace" => def_definitionnamespace(self, argv),
            _ => return None,
        })
    }

    /// Resolve a definition-body command in the target's TIP-524 definition
    /// namespace (exact name or unique prefix), dispatching the qualified form.
    /// `None` when there is no custom namespace or no match.
    fn oo_define_ns_command(&mut self, name: &[u8], argv: &[*mut TclObj]) -> Option<Code> {
        let target = self.active_def_target()?;
        // A custom TIP 524 definition namespace, else the built-in default
        // (`::oo::define` / `::oo::objdefine`) — user procs there are reachable
        // as definition commands too (oo-36.9/36.10).
        let def_ns = self
            .definition_namespace_for(&target)
            .unwrap_or_else(|| match target {
                DefTarget::Object(_) => b"::oo::objdefine".to_vec(),
                DefTarget::Class(_) => b"::oo::define".to_vec(),
            });
        let cmds = self.commands_in_namespace(&def_ns);
        let full: &[u8] = if let Some(c) = cmds.iter().find(|c| c.as_slice() == name) {
            c
        } else {
            let mut it = cmds.iter().filter(|c| c.starts_with(name));
            match (it.next(), it.next()) {
                (Some(c), None) => c,
                _ => return None,
            }
        };
        // Dispatch `<def_ns>::<full> <args…>`.
        let mut qualified = def_ns.clone();
        qualified.extend_from_slice(b"::");
        qualified.extend_from_slice(full);
        let head = obj::new_string_bytes(&qualified);
        unsafe { obj::incr_ref_count(head) };
        let mut new_argv: Vec<*mut TclObj> = Vec::with_capacity(argv.len());
        new_argv.push(head);
        for &a in &argv[1..] {
            unsafe { obj::incr_ref_count(a) };
            new_argv.push(a);
        }
        let code = self.dispatch(&new_argv);
        for a in new_argv {
            unsafe { obj::decr_ref_count(a) };
        }
        Some(code)
    }
}

fn my_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 {
        return wrong_args(interp, b"my methodName ?arg ...?");
    }
    // Inside a method, `my` dispatches on the current object; invoked directly
    // (e.g. `[info object namespace o]::my m`) it dispatches on the object that
    // owns this `my` command, found via its tracked alias (oo-16.13).
    let object = match interp
        .oo
        .borrow()
        .call_stack
        .last()
        .map(|f| f.object.clone())
    {
        Some(o) => Some(o),
        None => {
            let fqn = interp.fqn_for(&obj_bytes(argv[0]));
            interp
                .oo
                .borrow()
                .objects
                .iter()
                .find(|(_, o)| o.my_aliases.first().is_some_and(|a| *a == fqn))
                .map(|(k, _)| k.clone())
        }
    };
    let Some(object) = object else {
        return err(interp, b"my may only be called from inside a method");
    };
    let method = obj_bytes(argv[1]);
    interp.oo_invoke(&object, &method, &argv[2..], false)
}

fn myclass_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 {
        return wrong_args(interp, b"myclass methodName ?arg ...?");
    }
    // Inside a method, `myclass` dispatches on the current object's class; invoked
    // directly (e.g. a renamed `myclass`) it dispatches on the class of the object
    // that owns this command, found via its tracked alias (`my_aliases[1]`;
    // oo-41.2). The class can morph, so it is read live.
    let object = match interp
        .oo
        .borrow()
        .call_stack
        .last()
        .map(|f| f.object.clone())
    {
        Some(o) => Some(o),
        None => {
            let fqn = interp.fqn_for(&obj_bytes(argv[0]));
            interp
                .oo
                .borrow()
                .objects
                .iter()
                .find(|(_, o)| o.my_aliases.get(1).is_some_and(|a| *a == fqn))
                .map(|(k, _)| k.clone())
        }
    };
    let class = object.and_then(|o| {
        interp
            .oo
            .borrow()
            .objects
            .get(&o)
            .map(|ob| ob.class.clone())
    });
    let Some(class) = class else {
        return err(interp, b"myclass may only be called from inside a method");
    };
    let method = obj_bytes(argv[1]);
    interp.oo_invoke(&class, &method, &argv[2..], false)
}

fn next_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let ctx = interp.oo.borrow().call_stack.last().map(|frame| {
        (
            frame.object.clone(),
            frame.chain.clone(),
            frame.index,
            frame.target.clone(),
            frame.external,
        )
    });
    let Some((object, chain, index, target, external)) = ctx else {
        return err(interp, b"next may only be called from inside a method");
    };
    // The chain is pre-built (filters then the method steps), so `next` simply
    // advances to the following step. A `next` *from a filter step* clears
    // `filter_handling`, so the wrapped method re-enables filters for its own
    // `my` calls (C's FILTER_HANDLING; oo-12.7); save/restore around the call.
    let from_filter = chain
        .get(index)
        .is_some_and(|s| !s.method.is_empty() && s.method != target);
    if index + 1 < chain.len() {
        let saved_fh = interp.oo.borrow().filter_handling;
        if from_filter {
            interp.oo.borrow_mut().filter_handling = false;
        }
        let code = interp.oo_run(&object, chain, index + 1, &target, &argv[1..], external);
        interp.oo.borrow_mut().filter_handling = saved_fh;
        code
    } else if target.is_empty() {
        // Past the last constructor in the chain — a no-op (C's default).
        interp.set_result_bytes(b"");
        Code::Ok
    } else if let Some(code) = interp.oo_builtin_method(&object, &target, &argv[1..], external) {
        // Past the last user method: the `oo::object` built-ins (`eval`/
        // `variable`/`varname`) are the terminal implementations of those names,
        // so a user override that calls `next` reaches them (oo-18.5).
        code
    } else if target == b"destroy" {
        // `destroy` likewise has a built-in terminus (a filter/override chain
        // ending in the real teardown).
        let code = interp.oo_destroy(&object);
        if code == Code::Ok {
            interp.set_result_bytes(b"");
        }
        code
    } else {
        err(interp, b"no next method implementation")
    }
}

/// `nextto class ?arg ...?` (C's `TclOONextToObjCmd`): like `next`, but resumes
/// the call chain at the non-filter step declared by `class`, which must lie
/// *ahead* of the current step (no jumping backwards).
fn nextto_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let ctx = interp.oo.borrow().call_stack.last().map(|frame| {
        (
            frame.object.clone(),
            frame.chain.clone(),
            frame.index,
            frame.target.clone(),
            frame.external,
        )
    });
    let Some((object, chain, index, target, external)) = ctx else {
        return err(interp, b"nextto may only be called from inside a method");
    };
    if argv.len() < 2 {
        return wrong_args(interp, b"nextto class ?arg...?");
    }
    let raw = obj_bytes(argv[1]);
    // The class argument resolves like a command: current namespace then the
    // global fallback (C's Tcl_GetObjectFromObj), not relative to the object's
    // instance namespace that the method runs in.
    let class = match interp.namespaces().resolve_fqn(interp.current_ns(), &raw) {
        Some(c) if interp.oo.borrow().objects.contains_key(&c) => c,
        _ => interp.fqn_for(&raw),
    };
    if !interp.oo.borrow().objects.contains_key(&class) {
        return not_object(interp, &raw);
    }
    if !interp.oo.borrow().classes.contains_key(&class) {
        let mut m = b"\"".to_vec();
        m.extend_from_slice(&raw);
        m.extend_from_slice(b"\" is not a class");
        return err(interp, &m);
    }
    // A non-filter step has the same method name as the invoked target.
    let is_target = |s: &CallStep| s.method == target;
    // Search forward (past the current step) for the class's implementation.
    for i in index + 1..chain.len() {
        if chain[i].provider == class && is_target(&chain[i]) {
            return interp.oo_run(&object, chain, i, &target, &argv[2..], external);
        }
    }
    // Not reachable ahead: distinguish "behind us" from "not in the chain".
    let method_type: &[u8] = if target.is_empty() {
        b"constructor"
    } else if target == b"<destructor>" {
        b"destructor"
    } else {
        b"method"
    };
    let reachable_behind = (0..=index).any(|i| chain[i].provider == class && is_target(&chain[i]));
    let mut m = method_type.to_vec();
    if reachable_behind {
        m.extend_from_slice(b" implementation by \"");
        m.extend_from_slice(&raw);
        m.extend_from_slice(b"\" not reachable from here");
    } else {
        m.extend_from_slice(b" has no non-filter implementation by \"");
        m.extend_from_slice(&raw);
        m.push(b'"');
    }
    err(interp, &m)
}

/// `classvariable name ...` — the method-context command that links each `name`
/// in the running method's frame to the like-named variable in the *declaring
/// class's* namespace, so every instance of that class shares it. Distinct from
/// the object `variable` method (`oo_builtin_method`), which targets the object's
/// own instance namespace: here the target is the class object's namespace, and
/// it is resolved per *declaring* class, so a method reached via `next` links to
/// its own class rather than the leaf object's.
fn classvariable_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    // Method-context only. C exposes `classvariable` through `::oo::Helpers` on a
    // method's namespace path; the runtime models the helpers as global builtins
    // that consult the call stack (as `self`/`my`/`next` do), so outside a method
    // it reports the same "from inside a method" error those use.
    let step = interp
        .oo
        .borrow()
        .call_stack
        .last()
        .and_then(|f| f.chain.get(f.index).cloned());
    let Some(step) = step else {
        return err(
            interp,
            b"classvariable may only be called from inside a method",
        );
    };
    // The providing method must belong to a *class*: a per-object method (a plain
    // object's `objdefine` method, or a class-side `self method`) has no class
    // namespace to share into (C: "method not defined by a class").
    if step.is_object {
        return err(interp, b"method not defined by a class");
    }
    if argv.len() < 2 {
        return wrong_args(interp, b"classvariable name ...");
    }
    // The declaring class is itself an object (classes-as-objects); its instance
    // namespace holds the shared variables.
    let Some(class_ns) = interp
        .oo
        .borrow()
        .objects
        .get(&step.provider)
        .map(|o| o.var_ns)
    else {
        return err(interp, b"method not defined by a class");
    };
    for &a in &argv[1..] {
        let name = obj_bytes(a);
        // C validates the local name as it would any `upvar` target, in this
        // order: an array-element look-alike first, then a namespace separator.
        if crate::frame::split_array_ref(&name).1.is_some() {
            let mut m = b"bad variable name \"".to_vec();
            m.extend_from_slice(&name);
            m.extend_from_slice(
                b"\": can't create a scalar variable that looks like an array element",
            );
            return interp.set_error(&m);
        }
        if name.windows(2).any(|w| w == b"::") {
            let mut m = b"bad variable name \"".to_vec();
            m.extend_from_slice(&name);
            m.extend_from_slice(
                b"\": can't create a local variable with a namespace separator in it",
            );
            return interp.set_error(&m);
        }
        interp.make_variable(class_ns, &name);
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

// -- info object / info class (called from cmd_info) -------------------------

/// Canonical `info object`/`info class` subcommands (in C's listing order, used
/// for prefix resolution and the `unknown or ambiguous subcommand` message).
const INFO_OBJECT_SUBS: &[&[u8]] = &[
    b"call",
    b"class",
    b"creationid",
    b"definition",
    b"filters",
    b"forward",
    b"isa",
    b"methods",
    b"methodtype",
    b"mixins",
    b"namespace",
    b"properties",
    b"variables",
    b"vars",
];
const INFO_CLASS_SUBS: &[&[u8]] = &[
    b"call",
    b"constructor",
    b"definition",
    b"definitionnamespace",
    b"destructor",
    b"filters",
    b"forward",
    b"instances",
    b"methods",
    b"methodtype",
    b"mixins",
    b"properties",
    b"subclasses",
    b"superclasses",
    b"variables",
];

/// Resolve a (possibly abbreviated) `info object`/`info class` subcommand to its
/// canonical name — exact name or unique prefix, matching the C ensemble. On a
/// miss or an ambiguous prefix, set the `unknown or ambiguous subcommand` error.
fn info_sub_resolve<'a>(
    interp: &mut Interp,
    sub: &[u8],
    cands: &'a [&'a [u8]],
) -> Result<&'a [u8], Code> {
    if let Some(c) = cands.iter().find(|c| **c == sub) {
        return Ok(c);
    }
    let mut it = cands.iter().filter(|c| c.starts_with(sub));
    if let (Some(c), None) = (it.next(), it.next()) {
        return Ok(c);
    }
    let mut m = b"unknown or ambiguous subcommand \"".to_vec();
    m.extend_from_slice(sub);
    m.extend_from_slice(b"\": must be ");
    for (i, c) in cands.iter().enumerate() {
        if i > 0 {
            // Tcl ensemble style: ", or " before the last of 3+, " or " for 2.
            m.extend_from_slice(if i == cands.len() - 1 {
                if cands.len() == 2 {
                    b" or " as &[u8]
                } else {
                    b", or "
                }
            } else {
                b", "
            });
        }
        m.extend_from_slice(c);
    }
    Err(interp.set_error(&m))
}

/// `info object subcommand object ?arg?`.
pub(crate) fn info_object(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 3 {
        return wrong_args(interp, b"info object subcommand ?arg ...?");
    }
    let sub = match info_sub_resolve(interp, &obj_bytes(argv[2]), INFO_OBJECT_SUBS) {
        Ok(s) => s,
        Err(c) => return c,
    };
    // `creationid` has its own arg-count message (it errors at 0 *or* 2+ names).
    if sub == b"creationid" && argv.len() != 4 {
        return wrong_args(interp, b"info object creationid objName");
    }
    // `call` takes exactly objName + methodName.
    if sub == b"call" && argv.len() != 5 {
        return wrong_args(interp, b"info object call objName methodName");
    }
    if argv.len() < 4 {
        return wrong_args(interp, b"info object subcommand objName ?arg ...?");
    }
    let obj = interp.fqn_for(&obj_bytes(argv[3]));
    match sub {
        b"class" => {
            let class = interp
                .oo
                .borrow()
                .objects
                .get(&obj)
                .map(|o| o.class.clone());
            let Some(class) = class else {
                return not_object(interp, &obj_bytes(argv[3]));
            };
            // Two-arg form `info object class obj className`: a membership test —
            // whether `className` is in the object's precedence (its class MRO +
            // mixins), like `info object isa typeof` (C's `InfoObjectClassCmd`).
            if let Some(&want_arg) = argv.get(4) {
                let want = interp.oo_resolve_object(&obj_bytes(want_arg));
                let yes = interp.method_chain(&obj).iter().skip(1).any(|p| *p == want);
                interp.set_result_bytes(if yes { b"1" } else { b"0" });
                return Code::Ok;
            }
            interp.set_result(obj::new_string_bytes(&class));
            Code::Ok
        }
        b"isa" => {
            // info object isa category objName ?arg?
            let cat = obj_bytes(argv[3]);
            // Resolve the object name following import aliases to the origin
            // command, so an imported object command is recognised (oo-1.10).
            let target = interp.oo_resolve_object(&obj_bytes(argv[4]));
            let yes = match cat.as_slice() {
                b"object" => interp.oo.borrow().objects.contains_key(&target),
                b"class" => interp.oo.borrow().classes.contains_key(&target),
                // A metaclass is a class whose own hierarchy includes
                // `::oo::class` (so its instances are themselves classes).
                b"metaclass" => {
                    let is_class = interp.oo.borrow().classes.contains_key(&target);
                    is_class
                        && interp
                            .mro(&target)
                            .iter()
                            .any(|c| c.as_slice() == b"::oo::class")
                }
                // `mixin`: `mixinClass` is mixed into the object directly or via
                // its class.
                b"mixin" => {
                    let mixin = interp.fqn_for(&obj_bytes(argv[5]));
                    let oo = interp.oo.borrow();
                    oo.objects.get(&target).is_some_and(|o| {
                        o.mixins.contains(&mixin)
                            || oo
                                .classes
                                .get(&o.class)
                                .is_some_and(|c| c.mixins.contains(&mixin))
                    })
                }
                b"typeof" => {
                    // The object's full precedence list (mixins included), not
                    // just its class's superclass MRO (oo-16.9). The chain leads
                    // with the object itself, so drop it before matching classes.
                    let want = interp.fqn_for(&obj_bytes(argv[5]));
                    interp.oo.borrow().objects.contains_key(&target)
                        && interp
                            .method_chain(&target)
                            .iter()
                            .skip(1)
                            .any(|p| *p == want)
                }
                _ => false,
            };
            interp.set_result_bytes(if yes { b"1" } else { b"0" });
            Code::Ok
        }
        // `vars` lists *set* instance variables (glob-filtered); `variables`
        // lists the object's *declared* variables (the `variable` slot).
        b"vars" => {
            let ns = interp.oo.borrow().objects.get(&obj).map(|o| o.var_ns);
            let Some(ns) = ns else {
                return not_object(interp, &obj_bytes(argv[3]));
            };
            // `info object vars obj ?pattern?` — the optional glob filters the
            // (simple) variable names (oo-16.10).
            let pat = argv.get(4).map(|&a| obj_bytes(a));
            let mut names: Vec<Vec<u8>> = interp
                .namespaces()
                .var_names(ns)
                .into_iter()
                .filter(|n| fqn_glob_ok(pat.as_deref(), n))
                .collect();
            names.sort();
            set_list(interp, &names);
            Code::Ok
        }
        b"variables" => {
            // `info object variables obj ?-private?` — `-private` lists the
            // TIP 500 private instance variables instead of the public ones.
            let private = argv.get(4).map(|&a| obj_bytes(a)).as_deref() == Some(b"-private");
            let v = interp.oo.borrow().objects.get(&obj).map(|o| {
                if private {
                    o.private_variables.clone()
                } else {
                    o.variables.clone()
                }
            });
            match v {
                Some(v) => {
                    set_list(interp, &v);
                    Code::Ok
                }
                None => not_object(interp, &obj_bytes(argv[3])),
            }
        }
        b"properties" => {
            // `info object properties obj ?-all? ?-readable|-writable?` (TIP 558).
            if !interp.oo.borrow().objects.contains_key(&obj) {
                return not_object(interp, &obj_bytes(argv[3]));
            }
            let (all, writable) = match parse_property_opts(interp, &argv[4..]) {
                Ok(x) => x,
                Err(code) => return code,
            };
            let props = interp.object_property_list(&obj, writable, all);
            set_list(interp, &props);
            Code::Ok
        }
        b"methods" => {
            if !interp.oo.borrow().objects.contains_key(&obj) {
                return not_object(interp, &obj_bytes(argv[3]));
            }
            let all = argv[4..].iter().any(|&a| obj_bytes(a) == b"-all");
            let private = argv[4..].iter().any(|&a| obj_bytes(a) == b"-private");
            // `-scope public|unexported|private` (TIP 500) selects exactly one
            // visibility class (and ignores `-all`).
            let scope: Option<Vec<u8>> = argv[4..]
                .iter()
                .position(|&a| obj_bytes(a) == b"-scope")
                .and_then(|p| argv.get(4 + p + 1).map(|&a| obj_bytes(a)));
            let mut names: Vec<Vec<u8>> = Vec::new();
            // The object's own methods, plus (with `-all`, no `-scope`) the chain.
            let chain = if all && scope.is_none() {
                interp.method_chain(&obj)
            } else {
                vec![obj.clone()]
            };
            for p in &chain {
                let is_object = p.as_slice() == obj;
                let oo = interp.oo.borrow();
                let entry = if is_object {
                    oo.objects
                        .get(p)
                        .map(|o| (&o.methods, &o.unexported, &o.private))
                } else {
                    oo.classes
                        .get(p)
                        .map(|c| (&c.methods, &c.unexported, &c.private))
                };
                if let Some((methods, unexp, priv_set)) = entry {
                    for n in methods.keys() {
                        let show = if let Some(sc) = &scope {
                            let s: &[u8] = if priv_set.contains(n) {
                                b"private"
                            } else if unexp.contains(n) {
                                b"unexported"
                            } else {
                                b"public"
                            };
                            s == sc.as_slice()
                        } else if private {
                            !priv_set.contains(n)
                        } else {
                            !unexp.contains(n)
                        };
                        if show && !names.contains(n) {
                            names.push(n.clone());
                        }
                    }
                }
            }
            // `-all` also surfaces the inherited `oo::object` built-in methods,
            // honouring any `export`/`unexport` applied to them (but `-scope`
            // restricts to the object's own methods).
            if all && scope.is_none() {
                for b in [
                    b"<cloned>".as_slice(),
                    b"destroy",
                    b"eval",
                    b"unknown",
                    b"variable",
                    b"varname",
                ] {
                    let (exp, unexp, priv_) = interp.method_visibility_flags(&obj, b);
                    // `destroy` is exported by default; the rest are unexported.
                    let eff_unexp = !exp && (unexp || b != b"destroy");
                    let show = if private { !priv_ } else { !eff_unexp };
                    if show && !names.iter().any(|n| n == b) {
                        names.push(b.to_vec());
                    }
                }
            }
            names.sort();
            set_list(interp, &names);
            Code::Ok
        }
        b"namespace" => {
            let ns = interp.oo.borrow().objects.get(&obj).map(|o| o.var_ns);
            let Some(ns) = ns else {
                return not_object(interp, &obj_bytes(argv[3]));
            };
            let name = interp.namespaces().qualified_name(ns);
            interp.set_result(obj::new_string_bytes(&name));
            Code::Ok
        }
        b"mixins" => {
            let mx = interp
                .oo
                .borrow()
                .objects
                .get(&obj)
                .map(|o| o.mixins.clone());
            match mx {
                Some(m) => {
                    set_list(interp, &m);
                    Code::Ok
                }
                None => not_object(interp, &obj_bytes(argv[3])),
            }
        }
        b"creationid" => {
            // Exactly one object name; the prior `argv.len() < 4` gate already
            // rejected the no-name case, but this also rejects extra args.
            if argv.len() != 4 {
                return wrong_args(interp, b"info object creationid objName");
            }
            let id = interp.oo.borrow().objects.get(&obj).map(|o| o.creation_id);
            match id {
                Some(id) => {
                    interp.set_result_bytes(id.to_string().as_bytes());
                    Code::Ok
                }
                None => not_object(interp, &obj_bytes(argv[3])),
            }
        }
        b"filters" => {
            let f = interp
                .oo
                .borrow()
                .objects
                .get(&obj)
                .map(|o| o.filters.clone());
            match f {
                Some(f) => {
                    set_list(interp, &f);
                    Code::Ok
                }
                None => not_object(interp, &obj_bytes(argv[3])),
            }
        }
        b"forward" => info_forward(interp, &obj, argv, false),
        b"definition" => info_definition(interp, &obj, argv, false),
        b"methodtype" => info_methodtype(interp, &obj, argv, false),
        b"call" => info_call(interp, &obj, argv, false),
        // `properties` is not yet modelled.
        other => {
            let mut m = b"unsupported info object subcommand \"".to_vec();
            m.extend_from_slice(other);
            m.push(b'"');
            err(interp, &m)
        }
    }
}

/// `info object|class forward objName methodName` — the forward prefix list.
fn info_forward(interp: &mut Interp, fqn: &[u8], argv: &[*mut TclObj], class: bool) -> Code {
    if argv.len() != 5 {
        let u: &[u8] = if class {
            b"info class forward className methodName"
        } else {
            b"info object forward objName methodName"
        };
        return wrong_args(interp, u);
    }
    let name = obj_bytes(argv[4]);
    let m = {
        let oo = interp.oo.borrow();
        if class {
            oo.classes
                .get(fqn)
                .and_then(|c| c.methods.get(&name).cloned())
        } else {
            oo.objects
                .get(fqn)
                .and_then(|o| o.methods.get(&name).cloned())
        }
    };
    match m {
        Some(Method::Forward { prefix }) => {
            set_list(interp, &prefix);
            Code::Ok
        }
        Some(_) => err(
            interp,
            b"prefix argument list not available for this kind of method",
        ),
        None => unknown_method_named(interp, &name),
    }
}

/// `info object|class definition fqn methodName` — `{params} {body}`.
fn info_definition(interp: &mut Interp, fqn: &[u8], argv: &[*mut TclObj], class: bool) -> Code {
    if argv.len() != 5 {
        let u: &[u8] = if class {
            b"info class definition className methodName"
        } else {
            b"info object definition objName methodName"
        };
        return wrong_args(interp, u);
    }
    let name = obj_bytes(argv[4]);
    let m = {
        let oo = interp.oo.borrow();
        if class {
            oo.classes
                .get(fqn)
                .and_then(|c| c.methods.get(&name).cloned())
        } else {
            oo.objects
                .get(fqn)
                .and_then(|o| o.methods.get(&name).cloned())
        }
    };
    match m {
        Some(Method::Body { params, body, .. }) => {
            let out = list_params_body(&params, &body);
            interp.set_result(obj::new_string_bytes(&out));
            Code::Ok
        }
        Some(_) => err(interp, b"definition not available for this kind of method"),
        None => unknown_method_named(interp, &name),
    }
}

/// `info object|class methodtype fqn methodName` — `method` or `forward`.
fn info_methodtype(interp: &mut Interp, fqn: &[u8], argv: &[*mut TclObj], class: bool) -> Code {
    if argv.len() != 5 {
        let u: &[u8] = if class {
            b"info class methodtype className methodName"
        } else {
            b"info object methodtype objName methodName"
        };
        return wrong_args(interp, u);
    }
    let name = obj_bytes(argv[4]);
    let m = {
        let oo = interp.oo.borrow();
        if class {
            oo.classes
                .get(fqn)
                .and_then(|c| c.methods.get(&name).cloned())
        } else {
            oo.objects
                .get(fqn)
                .and_then(|o| o.methods.get(&name).cloned())
        }
    };
    match m {
        Some(Method::Body { .. } | Method::Builtin(_)) => {
            interp.set_result_bytes(b"method");
            Code::Ok
        }
        Some(Method::Forward { .. }) => {
            interp.set_result_bytes(b"forward");
            Code::Ok
        }
        None => unknown_method_named(interp, &name),
    }
}

/// The `unknown method "X"` error (no method-list suffix — used by the `info`
/// introspection subcommands that name a specific method).
fn unknown_method_named(interp: &mut Interp, name: &[u8]) -> Code {
    let mut m = b"unknown method \"".to_vec();
    m.extend_from_slice(name);
    m.push(b'"');
    interp.set_error(&m)
}

/// `info object|class call fqn methodName` — the method-resolution chain for
/// `methodName`, each step a `{callType methodName declarer methodType}` list.
fn info_call(interp: &mut Interp, fqn: &[u8], argv: &[*mut TclObj], class: bool) -> Code {
    if argv.len() != 5 {
        let u: &[u8] = if class {
            b"info class call className methodName"
        } else {
            b"info object call objName methodName"
        };
        return wrong_args(interp, u);
    }
    // The target must exist (oo-call-1.17): a non-object reports "X does not
    // refer to an object"; a non-class object reports `"X" is not a class`.
    if !interp.oo.borrow().objects.contains_key(fqn) {
        return not_object(interp, &obj_bytes(argv[3]));
    }
    if class && !interp.oo.borrow().classes.contains_key(fqn) {
        let mut m = b"\"".to_vec();
        m.extend_from_slice(&obj_bytes(argv[3]));
        m.extend_from_slice(b"\" is not a class");
        return err(interp, &m);
    }
    let method = obj_bytes(argv[4]);
    let elems = build_call_chain(interp, fqn, &method, class);
    set_list(interp, &elems);
    Code::Ok
}

/// Build the rendered call chain (`{callType name declarer methodType}` per
/// step) for `info object/class call`: filter steps (one per provider that
/// implements an active filter, in precedence order), then either the method's
/// own steps or — if the method is undefined — the `unknown`-handler chain.
fn build_call_chain(interp: &Interp, fqn: &[u8], method: &[u8], class: bool) -> Vec<Vec<u8>> {
    let providers = if class {
        interp.class_precedence(fqn)
    } else {
        interp.method_chain(fqn)
    };
    let is_obj = |p: &[u8]| !class && p == fqn;
    let mut elems: Vec<Vec<u8>> = Vec::new();
    // Active filter method names: the object's own filters then each provider's.
    let mut filter_names: Vec<Vec<u8>> = Vec::new();
    let add_name = |n: &Vec<u8>, v: &mut Vec<Vec<u8>>| {
        if !v.contains(n) {
            v.push(n.clone());
        }
    };
    if !class {
        if let Some(o) = interp.oo.borrow().objects.get(fqn) {
            for f in &o.filters {
                add_name(f, &mut filter_names);
            }
        }
    }
    for p in &providers {
        if let Some(c) = interp.oo.borrow().classes.get(p) {
            for f in &c.filters {
                add_name(f, &mut filter_names);
            }
        }
    }
    // Each filter contributes a step per implementing provider (precedence order).
    for fname in &filter_names {
        for p in &providers {
            if interp.oo_has_method(p, fname, is_obj(p)) {
                elems.push(call_chain_elem(
                    interp,
                    b"filter",
                    fname,
                    fname,
                    p,
                    is_obj(p),
                ));
            }
        }
    }
    // The target method's steps, or the unknown-handler chain if undefined.
    let method_steps: Vec<Vec<u8>> = providers
        .iter()
        .filter(|p| interp.oo_has_method(p, method, is_obj(p)))
        .cloned()
        .collect();
    // A core built-in (`destroy`/`eval`/`variable`/`varname`) is the terminal
    // implementation of its name on `oo::object`, so it appears as a chain step
    // even with no user method (C lists e.g. `{method destroy ::oo::object
    // {core method: "destroy"}}`). `destroy` is public; the rest are private.
    let core_builtin: Option<&[u8]> = match method {
        b"destroy" => Some(b"method"),
        b"eval" | b"variable" | b"varname" => Some(b"private"),
        _ => None,
    };
    if method_steps.is_empty() {
        if let Some(ct) = core_builtin {
            if providers.iter().any(|p| p.as_slice() == b"::oo::object") {
                elems.push(call_chain_elem(
                    interp,
                    ct,
                    method,
                    method,
                    b"::oo::object",
                    false,
                ));
            }
        } else {
            for p in &providers {
                if interp.oo_has_method(p, b"unknown", is_obj(p)) {
                    elems.push(call_chain_elem(
                        interp,
                        b"unknown",
                        b"unknown",
                        b"unknown",
                        p,
                        is_obj(p),
                    ));
                }
            }
        }
    } else {
        for p in &method_steps {
            let ct: &[u8] = if interp.method_is_private(p, method, is_obj(p)) {
                b"private"
            } else {
                b"method"
            };
            elems.push(call_chain_elem(interp, ct, method, method, p, is_obj(p)));
        }
    }
    elems
}

/// One `info call` / `self call` chain element: a 4-element list `{callType
/// displayName declarer methodType}`. `type_key` is the method-table key used
/// to determine the method type (proc `method`, `forward`, or a core builtin
/// rendered `core method: "NAME"`); the declarer is `object` for a per-object
/// method, else the class FQN.
fn call_chain_elem(
    interp: &Interp,
    call_type: &[u8],
    display_name: &[u8],
    type_key: &[u8],
    provider: &[u8],
    is_object: bool,
) -> Vec<u8> {
    let mtype = method_type_name(interp, provider, type_key, is_object);
    let declarer: Vec<u8> = if is_object {
        b"object".to_vec()
    } else {
        provider.to_vec()
    };
    build_list(&[call_type.to_vec(), display_name.to_vec(), declarer, mtype])
}

/// The `methodType` word for a method: `forward`, `core method: "NAME"` for a
/// native builtin, or `method` for a proc body (incl. constructor/destructor).
fn method_type_name(interp: &Interp, provider: &[u8], key: &[u8], is_object: bool) -> Vec<u8> {
    if key == b"<constructor>" || key == b"<destructor>" || key.is_empty() {
        return b"method".to_vec();
    }
    // The `oo::object` core built-ins are rendered `core method: "NAME"` even
    // though they are not in the method table (they are dispatched natively).
    if provider == b"::oo::object"
        && matches!(
            key,
            b"destroy" | b"eval" | b"variable" | b"varname" | b"unknown" | b"<cloned>"
        )
    {
        let mut s = b"core method: \"".to_vec();
        s.extend_from_slice(key);
        s.push(b'"');
        return s;
    }
    let oo = interp.oo.borrow();
    let m = if is_object {
        oo.objects.get(provider).and_then(|o| o.methods.get(key))
    } else {
        oo.classes.get(provider).and_then(|c| c.methods.get(key))
    };
    match m {
        Some(Method::Forward { .. }) => b"forward".to_vec(),
        Some(Method::Builtin(_)) => {
            let mut s = b"core method: \"".to_vec();
            s.extend_from_slice(key);
            s.push(b'"');
            s
        }
        _ => b"method".to_vec(),
    }
}

/// `info class subcommand class ?arg?`.
pub(crate) fn info_class(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 3 {
        return wrong_args(interp, b"info class subcommand ?arg ...?");
    }
    let sub = match info_sub_resolve(interp, &obj_bytes(argv[2]), INFO_CLASS_SUBS) {
        Ok(s) => s,
        Err(c) => return c,
    };
    if sub == b"call" && argv.len() != 5 {
        return wrong_args(interp, b"info class call className methodName");
    }
    if argv.len() < 4 {
        return wrong_args(interp, b"info class subcommand className ?arg ...?");
    }
    let cls = interp.fqn_for(&obj_bytes(argv[3]));
    // C (`TclOOGetClassFromObj`) resolves the object first: a non-object name
    // reports `X does not refer to an object` (no quotes); an object that is
    // not a class reports `"X" is not a class`.
    if !interp.oo.borrow().objects.contains_key(&cls) {
        return not_object(interp, &obj_bytes(argv[3]));
    }
    if !interp.oo.borrow().classes.contains_key(&cls) {
        let mut m = b"\"".to_vec();
        m.extend_from_slice(&obj_bytes(argv[3]));
        m.extend_from_slice(b"\" is not a class");
        return err(interp, &m);
    }
    match sub {
        b"superclasses" => {
            let s = interp.oo.borrow().classes[&cls].supers.clone();
            set_list(interp, &s);
            Code::Ok
        }
        b"mixins" => {
            let m = interp.oo.borrow().classes[&cls].mixins.clone();
            set_list(interp, &m);
            Code::Ok
        }
        b"variables" | b"variable" => {
            // `-private` lists the TIP 500 private instance variables.
            let private = argv.get(4).map(|&a| obj_bytes(a)).as_deref() == Some(b"-private");
            let cl = interp.oo.borrow();
            let v = if private {
                cl.classes[&cls].private_variables.clone()
            } else {
                cl.classes[&cls].variables.clone()
            };
            drop(cl);
            set_list(interp, &v);
            Code::Ok
        }
        b"properties" => {
            // `info class properties cls ?-all? ?-readable|-writable?` (TIP 558).
            let (all, writable) = match parse_property_opts(interp, &argv[4..]) {
                Ok(x) => x,
                Err(code) => return code,
            };
            let props = interp.class_property_list(&cls, writable, all);
            set_list(interp, &props);
            Code::Ok
        }
        b"instances" => {
            // `info class instances class ?pattern?` — direct instances, the
            // optional pattern glob-matching the instance command FQN.
            let pat = argv.get(4).map(|&a| obj_bytes(a));
            let mut insts: Vec<Vec<u8>> = interp
                .oo
                .borrow()
                .objects
                .iter()
                .filter(|(k, o)| o.class == cls && fqn_glob_ok(pat.as_deref(), k))
                .map(|(k, _)| k.clone())
                .collect();
            insts.sort();
            set_list(interp, &insts);
            Code::Ok
        }
        b"subclasses" => {
            // `info class subclasses class ?pattern?` — direct subclasses, the
            // optional pattern glob-matching the subclass FQN (oo-17.8).
            let pat = argv.get(4).map(|&a| obj_bytes(a));
            let mut subs: Vec<Vec<u8>> = interp
                .oo
                .borrow()
                .classes
                .iter()
                .filter(|(k, c)| {
                    **k != cls && c.supers.contains(&cls) && fqn_glob_ok(pat.as_deref(), k)
                })
                .map(|(k, _)| k.clone())
                .collect();
            subs.sort();
            set_list(interp, &subs);
            Code::Ok
        }
        b"methods" => {
            let all = argv[4..].iter().any(|&a| obj_bytes(a) == b"-all");
            let private = argv[4..].iter().any(|&a| obj_bytes(a) == b"-private");
            // `-scope public|unexported|private` (TIP 500) selects exactly one
            // visibility class.
            let scope: Option<Vec<u8>> = argv[4..]
                .iter()
                .position(|&a| obj_bytes(a) == b"-scope")
                .and_then(|p| argv.get(4 + p + 1).map(|&a| obj_bytes(a)));
            let mut names: Vec<Vec<u8>> = Vec::new();
            // `-all` traverses the class's full precedence (its mixins and the
            // mixins of its superclasses too), not just the superclass MRO
            // (oo-35.5).
            let chain = if all {
                interp.class_precedence(&cls)
            } else {
                vec![cls.clone()]
            };
            for c in &chain {
                if let Some(cl) = interp.oo.borrow().classes.get(c) {
                    for n in cl.methods.keys() {
                        let show = if let Some(sc) = &scope {
                            let s: &[u8] = if cl.private.contains(n) {
                                b"private"
                            } else if cl.unexported.contains(n) {
                                b"unexported"
                            } else {
                                b"public"
                            };
                            s == sc.as_slice()
                        } else if private {
                            // `-private` lists unexported methods too, but never
                            // the TIP-500 private ones; default lists exported.
                            !cl.private.contains(n)
                        } else {
                            !cl.unexported.contains(n)
                        };
                        if show && !names.contains(n) {
                            names.push(n.clone());
                        }
                    }
                }
            }
            // `-all` reaches oo::object's built-ins: `destroy` (public, unless
            // unexported in the chain), plus the unexported set under `-private`
            // (oo-17.9, oo-17.10).
            if all {
                let add = |n: &[u8], names: &mut Vec<Vec<u8>>| {
                    if !names.iter().any(|x| x == n) {
                        names.push(n.to_vec());
                    }
                };
                let destroy_unexported = chain.iter().any(|c| {
                    interp
                        .oo
                        .borrow()
                        .classes
                        .get(c)
                        .is_some_and(|cl| cl.unexported.contains(b"destroy".as_slice()))
                });
                if private || !destroy_unexported {
                    add(b"destroy", &mut names);
                }
                if private {
                    for b in [
                        &b"<cloned>"[..],
                        b"eval",
                        b"unknown",
                        b"variable",
                        b"varname",
                    ] {
                        add(b, &mut names);
                    }
                }
            }
            names.sort();
            set_list(interp, &names);
            Code::Ok
        }
        b"constructor" => {
            let body = match &interp.oo.borrow().classes[&cls].constructor {
                Some(Method::Body { params, body, .. }) => list_params_body(params, body),
                _ => Vec::new(),
            };
            interp.set_result(obj::new_string_bytes(&body));
            Code::Ok
        }
        b"destructor" => {
            let body = interp.oo.borrow().classes[&cls]
                .destructor
                .clone()
                .unwrap_or_default();
            interp.set_result(obj::new_string_bytes(&body));
            Code::Ok
        }
        b"definitionnamespace" => {
            // `info class definitionnamespace className ?kind?` (TIP 524).
            if argv.len() > 5 {
                return wrong_args(interp, b"info class definitionnamespace className ?kind?");
            }
            let kind = if argv.len() == 5 {
                obj_bytes(argv[4])
            } else {
                b"-class".to_vec()
            };
            let ns = match kind.as_slice() {
                // The namespace used to define this class itself.
                b"-class" => interp.oo.borrow().classes[&cls].class_def_ns.clone(),
                // The namespace used to define this class's instances.
                b"-instance" => interp.oo.borrow().classes[&cls].def_ns.clone(),
                _ => {
                    let mut m = b"bad kind \"".to_vec();
                    m.extend_from_slice(&kind);
                    m.extend_from_slice(b"\": must be -class or -instance");
                    return err(interp, &m);
                }
            };
            interp.set_result_bytes(&ns.unwrap_or_default());
            Code::Ok
        }
        b"filters" => {
            let f = interp.oo.borrow().classes[&cls].filters.clone();
            set_list(interp, &f);
            Code::Ok
        }
        b"forward" => info_forward(interp, &cls, argv, true),
        b"definition" => info_definition(interp, &cls, argv, true),
        b"methodtype" => info_methodtype(interp, &cls, argv, true),
        b"call" => info_call(interp, &cls, argv, true),
        // `properties` is not yet modelled.
        other => {
            let mut m = b"unsupported info class subcommand \"".to_vec();
            m.extend_from_slice(other);
            m.push(b'"');
            err(interp, &m)
        }
    }
}

/// Whether `name` passes an optional introspection glob `pattern` (matching the
/// fully-qualified name). `None` (no pattern) always passes.
fn fqn_glob_ok(pattern: Option<&[u8]>, name: &[u8]) -> bool {
    match pattern {
        None => true,
        Some(p) => match (core::str::from_utf8(p), core::str::from_utf8(name)) {
            (Ok(p), Ok(n)) => tcl_syntax::glob::string_match(p, n),
            _ => false,
        },
    }
}

fn not_object(interp: &mut Interp, name: &[u8]) -> Code {
    // C (`Tcl_GetObjectFromObj`) reports without surrounding quotes.
    let mut m = name.to_vec();
    m.extend_from_slice(b" does not refer to an object");
    interp.set_error(&m)
}

/// `{params} body` as a 2-element list (for `info class constructor`).
fn list_params_body(params: &[Param], body: &[u8]) -> Vec<u8> {
    // The argument spec is a list whose elements are either a bare parameter
    // name or a `{name default}` pair — so `{a {b c} args}` round-trips.
    let param_objs: Vec<*mut TclObj> = params
        .iter()
        .map(|p| match &p.default {
            Some(d) => {
                let pair = [obj::new_string_bytes(&p.name), obj::new_string_bytes(d)];
                let l = list::new_list_obj(&pair);
                let s = obj_bytes(l);
                crate::interp::drop_fresh(l);
                obj::new_string_bytes(&s)
            }
            None => obj::new_string_bytes(&p.name),
        })
        .collect();
    let plist = list::new_list_obj(&param_objs);
    let spec = obj_bytes(plist);
    crate::interp::drop_fresh(plist);
    let elems = [obj::new_string_bytes(&spec), obj::new_string_bytes(body)];
    let l = list::new_list_obj(&elems); // rc 0, owns its (now rc-1) elements
    let out = obj_bytes(l);
    crate::interp::drop_fresh(l); // frees the list and, with it, its elements
    out
}

fn set_list(interp: &mut Interp, names: &[Vec<u8>]) {
    let elems: Vec<*mut TclObj> = names.iter().map(|n| obj::new_string_bytes(n)).collect();
    // `new_list_obj` retains each element; `set_result` retains the list. The
    // rc-0 temporaries are now owned by the list — no manual release.
    interp.set_result(list::new_list_obj(&elems));
}

// -- the object-system engine (impl Interp) ----------------------------------

impl Interp {
    /// An OO object's/class's command was renamed or deleted (e.g. `rename obj
    /// {}`): keep the OO registry in sync so the name frees up / follows the
    /// command. Tcl ties an object's lifetime to its command.
    pub(crate) fn oo_command_renamed(&mut self, old_fqn: &[u8], new_fqn: Option<&[u8]>) {
        match new_fqn {
            // Rename: the object/class (and its `my`) follow to the new name.
            Some(nf) => {
                let mut oo = self.oo.borrow_mut();
                // A class is registered in *both* maps (classes-as-objects), so
                // move from both to avoid a dangling half-entry.
                let obj = oo.objects.remove(old_fqn);
                let cls = oo.classes.remove(old_fqn);
                // Only an actual OO object/class command gets re-bound below (not
                // an incidental rename of `my` or some unrelated command).
                let was_oo_object = obj.is_some() || cls.is_some();
                if let Some(o) = obj {
                    oo.objects.insert(nf.to_vec(), o);
                }
                if let Some(c) = cls {
                    oo.classes.insert(nf.to_vec(), c);
                }
                // A renamed per-object `my`/`myclass` follows in the owning
                // object's alias list, so teardown still finds and deletes it.
                // References to the renamed object/class in other records' class,
                // superclass, and mixin lists follow too — C tracks objects by
                // pointer, so a rename is transparent to its dependents (e.g. a
                // class mixed into its own renamed instance; oo-23.1).
                for o in oo.objects.values_mut() {
                    for a in &mut o.my_aliases {
                        if a.as_slice() == old_fqn {
                            *a = nf.to_vec();
                        }
                    }
                    if o.class.as_slice() == old_fqn {
                        o.class = nf.to_vec();
                    }
                    for m in &mut o.mixins {
                        if m.as_slice() == old_fqn {
                            *m = nf.to_vec();
                        }
                    }
                }
                for c in oo.classes.values_mut() {
                    for s in &mut c.supers {
                        if s.as_slice() == old_fqn {
                            *s = nf.to_vec();
                        }
                    }
                    for m in &mut c.mixins {
                        if m.as_slice() == old_fqn {
                            *m = nf.to_vec();
                        }
                    }
                }
                drop(oo);
                // The object command embeds its own FQN (returned by `self` and
                // used to resolve the object); re-bind it under the new name so a
                // renamed object reports and dispatches as its new name (oo-23.1).
                if was_oo_object {
                    self.ns_register(nf, Command::OoObject(nf.to_vec()));
                }
            }
            // Deletion (`rename obj {}`): C ties the object's lifetime to its
            // command, so the destructor fires before the registry entry goes.
            None => {
                if self.oo.borrow().classes.contains_key(old_fqn) {
                    self.oo_destroy_class(old_fqn);
                } else if self.oo.borrow().objects.contains_key(old_fqn) {
                    // Implicit teardown swallows the destructor result.
                    self.oo_destroy_bg(old_fqn);
                }
            }
        }
    }

    /// Destroy every OO object whose instance-variable namespace lies within the
    /// namespace `ns` (or a descendant) — invoked just before `namespace delete`
    /// clears it, so destructors run with the namespace still intact.
    pub(crate) fn oo_namespace_deleted(&mut self, ns: NsId) {
        if self.oo_is_empty() {
            return;
        }
        let ids: std::collections::HashSet<NsId> =
            self.namespaces().descendant_ids(ns).into_iter().collect();
        let victims: Vec<Vec<u8>> = self
            .oo
            .borrow()
            .objects
            .iter()
            // Skip objects already being torn down (e.g. the object whose own
            // namespace deletion triggered this cascade) — only the not-yet-
            // destroyed descendants are destroyed here.
            .filter(|(_, o)| ids.contains(&o.var_ns) && !o.destroyed)
            .map(|(k, _)| k.clone())
            .collect();
        for v in victims {
            self.oo_destroy_bg(&v);
        }
    }

    /// Whether the OO registry is empty (the rename hot-path early-out).
    pub(crate) fn oo_is_empty(&self) -> bool {
        let oo = self.oo.borrow();
        oo.objects.is_empty() && oo.classes.is_empty()
    }

    /// Create class `fqn` (running its optional definition script).
    fn oo_make_class(&mut self, fqn: &[u8], display: &[u8], script: Option<&[u8]>) -> Code {
        let taken = self.oo.borrow().classes.contains_key(fqn)
            || self.oo.borrow().objects.contains_key(fqn);
        // A root this release does not have is not a collision: the engine
        // seeds the 9.0 metaclasses unconditionally and lets the gate hide
        // them, so on an 8.6 surface the name is the script's to take.
        if taken && !self.is_gate_hidden_object_root(fqn) {
            // C reports `object` (creation funnels through object creation) and
            // the name *as written*, not the resolved FQN.
            let mut m = b"can't create object \"".to_vec();
            m.extend_from_slice(display);
            m.extend_from_slice(b"\": command already exists with that name");
            return self.error(&m);
        }
        self.oo.borrow_mut().classes.insert(
            fqn.to_vec(),
            Class {
                supers: vec![b"::oo::object".to_vec()],
                ..Class::default()
            },
        );
        // A class is also an object (an instance of `::oo::class`), so it can
        // carry its own methods (`oo::define … self method`) — the TclOO
        // classes-as-objects model.
        let var_ns = self.ensure_namespace(fqn);
        let creation_id = self.oo_next_id();
        self.oo.borrow_mut().objects.insert(
            fqn.to_vec(),
            Object {
                class: b"::oo::class".to_vec(),
                var_ns,
                creation_id,
                ..Object::default()
            },
        );
        self.ns_register(fqn, Command::OoObject(fqn.to_vec()));
        self.oo_register_my(fqn);
        if let Some(script) = script {
            let code = self.oo_define_body(DefTarget::Class(fqn.to_vec()), script, None);
            if code != Code::Ok {
                // Roll back a failed definition so the name frees up (C destroys
                // a partially-created class whose definition script errors).
                self.oo.borrow_mut().classes.remove(fqn);
                self.oo.borrow_mut().objects.remove(fqn);
                self.delete_command(fqn);
                return code;
            }
        }
        self.set_result(obj::new_string_bytes(fqn));
        Code::Ok
    }

    /// Run an `oo::define`/`oo::objdefine` body or single subcommand on `target`.
    fn oo_run_def(&mut self, target: DefTarget, argv: &[*mut TclObj]) -> Code {
        if argv.len() == 3 {
            let body = obj_bytes(argv[2]);
            // The body word is argv[2]; capture its source provenance (TIP 280)
            // now, while the argument lines are still those of this command.
            let body_src = method_body_src(self, 2);
            return self.oo_define_body(target, &body, body_src);
        }
        // Single-command form: a subcommand's `wrong # args` names the whole
        // original command (`oo::define Foo method …`), via the rewrite prefix
        // `<oo::define|oo::objdefine> <as-written target>`.
        let mut prefix = obj_bytes(argv[0]);
        prefix.push(b' ');
        prefix.extend_from_slice(&obj_bytes(argv[1]));
        let saved = self.oo.borrow_mut().def_rewrite.replace(prefix);
        let lvl = self.current_level();
        let tfqn = match &target {
            DefTarget::Class(c) | DefTarget::Object(c) => c.clone(),
        };
        let cid = self.oo.borrow().objects.get(&tfqn).map(|o| o.creation_id);
        self.oo.borrow_mut().def_stack.push((target, lvl, cid));
        // Re-base the TIP 280 argument lines onto the dispatched subcommand
        // (drop the `oo::define <target>` prefix) so a defined body word's line
        // is found at the subcommand's own index, then restore.
        let saved_lines = self.arg_lines_snapshot();
        if saved_lines.len() >= 2 {
            self.set_arg_lines(saved_lines[2..].to_vec());
        }
        let code = self.dispatch(&argv[2..]);
        self.set_arg_lines(saved_lines);
        self.oo.borrow_mut().def_stack.pop();
        self.oo.borrow_mut().def_rewrite = saved;
        code
    }

    /// Evaluate a definition `body` on `target`. The target's definition
    /// namespace (TIP 524) contributes commands via `oo_define_command` (a path-
    /// style lookup on a command miss), so bare procs there are reachable while
    /// class-name *arguments* still resolve in the caller's namespace. Shared by
    /// `oo_run_def` and the `oo::class` constructor (a metaclass instance body).
    fn oo_define_body(
        &mut self,
        target: DefTarget,
        body: &[u8],
        body_src: Option<(Rc<[u8]>, u32)>,
    ) -> Code {
        let (kind, fqn): (&[u8], Vec<u8>) = match &target {
            DefTarget::Class(c) => (b"class", c.clone()),
            DefTarget::Object(o) => (b"object", o.clone()),
        };
        // The errorInfo frame names the object by its *current* command name, so
        // a `rename` inside the body is reflected (oo-18.6/18.7). The creation id
        // is stable across rename, so we re-find the entry by it at error time.
        let creation_id = self.oo.borrow().objects.get(&fqn).map(|o| o.creation_id);
        let lvl = self.current_level();
        self.oo
            .borrow_mut()
            .def_stack
            .push((target, lvl, creation_id));
        // When sourced, run the body in a `type source` frame so the methods it
        // defines record file-absolute body lines (TIP 280).
        let code = self.eval_def_body(body, body_src);
        self.oo.borrow_mut().def_stack.pop();
        // On error, add the `(in definition script for class/object "X" line N)`
        // errorInfo frame (C's GenerateErrorInfo).
        if code == Code::Error {
            self.add_def_script_frame(kind, &fqn, creation_id);
        }
        code
    }

    /// Append the `(in definition script for <kind> "<name>" line N)` errorInfo
    /// frame (C's `GenerateErrorInfo`). The object is named by its *current*
    /// command (re-found by `creation_id` so a `rename` in the body shows), the
    /// name quoted and truncated to 30 bytes (`OBJNAME_LENGTH_IN_ERRORINFO_LIMIT`).
    fn add_def_script_frame(&mut self, kind: &[u8], fqn: &[u8], creation_id: Option<u64>) {
        let current = creation_id
            .and_then(|id| {
                self.oo
                    .borrow()
                    .objects
                    .iter()
                    .find(|(_, o)| o.creation_id == id)
                    .map(|(k, _)| k.clone())
            })
            .unwrap_or_else(|| fqn.to_vec());
        let mut inner = b"in definition script for ".to_vec();
        inner.extend_from_slice(kind);
        inner.extend_from_slice(b" \"");
        if current.len() > 30 {
            inner.extend_from_slice(&current[..30]);
            inner.extend_from_slice(b"...");
        } else {
            inner.extend_from_slice(&current);
        }
        inner.push(b'"');
        self.append_frame_line(&inner);
        // A frame boundary: let the enclosing command log its own `invoked from
        // within` frame.
        self.clear_error_logged();
    }

    /// The custom definition-resolution namespace for a definition `target`, or
    /// `None` for the built-in default (`::oo::define`/`::oo::objdefine`, which
    /// our global definition builtins already serve). For a class, it is the
    /// metaclass's `-class` namespace; for an object, its class's `-instance`.
    fn definition_namespace_for(&self, target: &DefTarget) -> Option<Vec<u8>> {
        let oo = self.oo.borrow();
        let ns = match target {
            DefTarget::Class(c) => {
                let meta = oo.objects.get(c).map(|o| o.class.clone())?;
                oo.classes.get(&meta)?.class_def_ns.clone()
            }
            DefTarget::Object(o) => {
                let cls = oo.objects.get(o).map(|ob| ob.class.clone())?;
                oo.classes.get(&cls)?.def_ns.clone()
            }
        }?;
        // The built-in defaults are handled by the global definition commands.
        if ns == b"::oo::define" || ns == b"::oo::objdefine" {
            None
        } else {
            Some(ns)
        }
    }

    /// The class instantiation built-ins (`create`/`new`/`createWithNamespace`).
    /// Shared by external `$cls method …` dispatch and internal `my method …`
    /// dispatch, so a metaclass method can `my create …`. `args` are the
    /// arguments *after* the method name; `cmd` names the command for
    /// `wrong # args`. `block_unexported` honours an `unexport` of `create`/
    /// `new` (and the default-unexported `createWithNamespace`) for an external
    /// call; an internal call sees them all. Returns `None` when `method` is not
    /// an applicable built-in, so the caller falls through.
    fn oo_class_factory(
        &mut self,
        fqn: &[u8],
        cmd: &[u8],
        method: &[u8],
        args: &[*mut TclObj],
        block_unexported: bool,
    ) -> Option<Code> {
        if !self.oo.borrow().classes.contains_key(fqn) {
            return None;
        }
        // The metaclass `::oo::class` is a singleton: instantiating it builds a
        // *class* (`oo_make_class`), and `new` is unexported on it.
        let is_meta = fqn == b"::oo::class";
        let (cre_unexp, new_unexp, cwn_exp) = {
            let oo = self.oo.borrow();
            match oo.objects.get(fqn) {
                Some(o) => (
                    o.unexported.contains(b"create".as_slice()),
                    o.unexported.contains(b"new".as_slice()),
                    o.exported.contains(b"createWithNamespace".as_slice()),
                ),
                None => (false, false, false),
            }
        };
        let cre_ok = !block_unexported || !cre_unexp;
        let new_ok = !block_unexported || !new_unexp;
        // `createWithNamespace` is unexported by default; an external call needs
        // it `self export`ed, an internal call reaches it regardless.
        let cwn_ok = !block_unexported || cwn_exp;
        // Record the instantiation words (`cmd method ?args...?`) for the
        // constructor's `info level 0` (oo-2.1). The constructor's run_proc
        // consumes this; a no-constructor path leaves it for the next create.
        if matches!(method, b"new" | b"create" | b"createWithNamespace") {
            let mut words = vec![cmd.to_vec(), method.to_vec()];
            words.extend(args.iter().map(|&a| obj_bytes(a)));
            self.oo.borrow_mut().ctor_words = Some(words);
        }
        match method {
            b"new" if !is_meta && new_ok => Some(self.oo_new(fqn, None, b"", args)),
            b"createWithNamespace" if cwn_ok => {
                if args.len() < 2 {
                    let mut u = cmd.to_vec();
                    u.extend_from_slice(b" createWithNamespace objectName namespaceName ?arg ...?");
                    return Some(wrong_args(self, &u));
                }
                let raw = obj_bytes(args[0]);
                if raw.is_empty() {
                    return Some(self.error(b"object name must not be empty"));
                }
                let name = self.fqn_for(&raw);
                let ns_raw = obj_bytes(args[1]);
                // The namespace is *created*; an existing one is an error.
                if self.resolve_namespace_name(&ns_raw).is_some() {
                    let mut m = b"can't create namespace \"".to_vec();
                    m.extend_from_slice(&ns_raw);
                    m.extend_from_slice(b"\": already exists");
                    return Some(self.error(&m));
                }
                let ns = self.fqn_for(&ns_raw);
                Some(self.oo_new_ns(fqn, Some(name), &raw, Some(ns), &args[2..]))
            }
            b"create" if cre_ok => {
                if args.is_empty() {
                    let mut u = cmd.to_vec();
                    u.extend_from_slice(b" create objectName ?arg ...?");
                    return Some(wrong_args(self, &u));
                }
                let raw = obj_bytes(args[0]);
                if raw.is_empty() {
                    return Some(self.error(b"object name must not be empty"));
                }
                let name = self.fqn_for(&raw);
                Some(if is_meta {
                    self.oo_make_class(&name, &raw, args.get(1).map(|&a| obj_bytes(a)).as_deref())
                } else {
                    self.oo_new(fqn, Some(name), &raw, &args[1..])
                })
            }
            _ => None,
        }
    }

    /// Whether `obj` has any active filter (its own object filters, or a class
    /// filter anywhere along its precedence) — so a built-in target like
    /// `destroy` must be dispatched through the filter chain, not directly.
    fn object_has_filters(&self, obj: &[u8]) -> bool {
        if self
            .oo
            .borrow()
            .objects
            .get(obj)
            .is_some_and(|o| !o.filters.is_empty())
        {
            return true;
        }
        self.method_chain(obj).iter().any(|p| {
            self.oo
                .borrow()
                .classes
                .get(p)
                .is_some_and(|c| !c.filters.is_empty())
        })
    }

    /// Dispatch a command bound to the OO object/class FQN `fqn`.
    pub(crate) fn oo_dispatch(&mut self, fqn: &[u8], argv: &[*mut TclObj]) -> Code {
        if self.oo.borrow().classes.contains_key(fqn) {
            let cmd = obj_bytes(argv[0]);
            // The class instantiation built-ins honour the class's own
            // `export`/`unexport` for this external call.
            if let Some(sub) = argv.get(1).map(|&a| obj_bytes(a)) {
                if let Some(code) = self.oo_class_factory(fqn, &cmd, &sub, &argv[2..], true) {
                    return code;
                }
            }
            match argv.get(1).map(|&a| obj_bytes(a)).as_deref() {
                Some(b"destroy") => {
                    self.oo_destroy_class(fqn);
                    self.set_result_bytes(b"");
                    Code::Ok
                }
                // Any other subcommand is a class-object method (defined via
                // `oo::define … self method`); dispatch it on the class object.
                // An unknown method funnels through `oo_invoke` →
                // `oo_unknown_method` for the C error text.
                Some(other) => self.oo_invoke(fqn, other, &argv[2..], true),
                // No method name: C forces the unknown handler (`FORCE_UNKNOWN`)
                // with an empty method, so a *user* `unknown` runs with no args.
                // With only the default handler, report the `wrong # args` usage
                // naming the command as invoked (`argv[0]`, not the FQN).
                None if self.has_user_unknown(fqn) => self.oo_invoke(fqn, b"unknown", &[], false),
                None => {
                    let mut u = obj_bytes(argv[0]);
                    u.extend_from_slice(b" method ?arg ...?");
                    wrong_args(self, &u)
                }
            }
        } else if self.oo.borrow().objects.contains_key(fqn) {
            match argv.get(1).map(|&a| obj_bytes(a)) {
                // Explicit `obj destroy` propagates a destructor error to its
                // caller; on success the method yields the empty string. A
                // class that unexports `destroy` (e.g. `::oo::Slot`) hides it
                // from external calls, which then funnel through `oo_invoke` to
                // the unknown handler instead of tearing the object down.
                Some(m)
                    if m == b"destroy"
                        && (self.method_exported(fqn, b"destroy")
                            || !self.method_visibility_flags(fqn, b"destroy").1)
                        // An active filter must wrap `destroy` too, so route it
                        // through oo_invoke (which prepends the filter chain) in
                        // that case rather than tearing down directly (oo-12.2).
                        && !self.object_has_filters(fqn) =>
                {
                    let code = self.oo_destroy(fqn);
                    if code == Code::Ok {
                        self.set_result_bytes(b"");
                    }
                    code
                }
                Some(method) => self.oo_invoke(fqn, &method, &argv[2..], true),
                // No method name: force a *user* `unknown` (C's `FORCE_UNKNOWN`)
                // with empty args; else the `wrong # args` usage (as invoked).
                None if self.has_user_unknown(fqn) => self.oo_invoke(fqn, b"unknown", &[], false),
                None => {
                    let mut u = obj_bytes(argv[0]);
                    u.extend_from_slice(b" method ?arg ...?");
                    wrong_args(self, &u)
                }
            }
        } else {
            self.invalid_command(fqn)
        }
    }

    fn oo_new(
        &mut self,
        class: &[u8],
        name: Option<Vec<u8>>,
        display: &[u8],
        args: &[*mut TclObj],
    ) -> Code {
        self.oo_new_ns(class, name, display, None, args)
    }

    /// `oo_new` with an optional explicit instance-variable namespace
    /// (`createWithNamespace`); `None` defaults to the object's own name.
    /// `display` is the object name *as written* (for the dup error).
    fn oo_new_ns(
        &mut self,
        class: &[u8],
        name: Option<Vec<u8>>,
        display: &[u8],
        ns_override: Option<Vec<u8>>,
        args: &[*mut TclObj],
    ) -> Code {
        let fqn = name.unwrap_or_else(|| {
            let n = format!("::oo::Obj{}", self.oo.borrow().counter);
            self.oo.borrow_mut().counter += 1;
            n.into_bytes()
        });
        let taken = self.oo.borrow().objects.contains_key(&fqn)
            || self.oo.borrow().classes.contains_key(&fqn);
        // A root this release does not have is not a collision: the engine
        // seeds the 9.0 metaclasses unconditionally and lets the gate hide
        // them, so on an 8.6 surface the name is the script's to take.
        if taken && !self.is_gate_hidden_object_root(&fqn) {
            let mut m = b"can't create object \"".to_vec();
            m.extend_from_slice(display);
            m.extend_from_slice(b"\": command already exists with that name");
            return self.error(&m);
        }
        let var_ns = match ns_override {
            Some(ns) => self.ensure_namespace(&ns),
            None => self.ensure_namespace(&fqn),
        };
        let creation_id = self.oo_next_id();
        self.oo.borrow_mut().objects.insert(
            fqn.clone(),
            Object {
                class: class.to_vec(),
                var_ns,
                creation_id,
                ..Object::default()
            },
        );
        // Instantiating a *metaclass* (one whose MRO includes `::oo::class`)
        // produces a class: register the new instance in the class map too, so
        // it responds to `create`/`new` and is `info object isa class`.
        let mro = self.mro(class);
        let is_metaclass = mro.iter().any(|c| c.as_slice() == b"::oo::class");
        if is_metaclass {
            // A TIP 558 configurable class (instance of `::oo::configurable`)
            // mixes in the configurable support (for `configure`) and points its
            // instance definitions at the configurableobject namespace (so
            // `oo::objdefine $inst property …` resolves). Done before the
            // definition script runs, which still resolves class-name arguments
            // in the caller's namespace.
            let configurable = mro.iter().any(|c| c.as_slice() == b"::oo::configurable");
            self.oo.borrow_mut().classes.insert(
                fqn.clone(),
                Class {
                    supers: vec![b"::oo::object".to_vec()],
                    mixins: if configurable {
                        vec![b"::oo::configuresupport::configurable".to_vec()]
                    } else {
                        Vec::new()
                    },
                    def_ns: if configurable {
                        Some(b"::oo::configuresupport::configurableobject".to_vec())
                    } else {
                        None
                    },
                    ..Class::default()
                },
            );
        }
        self.ns_register(&fqn, Command::OoObject(fqn.clone()));
        self.oo_register_my(&fqn);

        // Constructor dispatch runs along the *class* MRO (objects can't define
        // constructors), with the object as `self`; the chain is the
        // constructor-providing classes in MRO order (so `next` chains). For a
        // metaclass instance, `::oo::class` contributes a synthetic constructor
        // that applies the (optional) definition-script argument.
        let chain: Vec<CallStep> = mro
            .iter()
            .filter(|c| self.class_has_ctor(c) || (is_metaclass && c.as_slice() == b"::oo::class"))
            .map(|c| CallStep {
                is_object: c.as_slice() == fqn,
                provider: c.clone(),
                method: Vec::new(),
            })
            .collect();
        if !chain.is_empty() {
            let code = self.oo_run(&fqn, chain, 0, b"", args, false);
            if code == Code::Error {
                // A failed constructor tears the partially-built object down,
                // running its destructor (C: the object is deleted, firing the
                // destructor chain), then re-raises the constructor's error.
                let snap = self.error_snapshot();
                if self.oo.borrow().objects.contains_key(&fqn) {
                    self.oo_destroy_bg(&fqn);
                }
                // `oo_destroy_bg` removes the object; clean up the class facet
                // (a failed metaclass instantiation) and the command too.
                self.oo.borrow_mut().objects.remove(&fqn);
                self.oo.borrow_mut().classes.remove(&fqn);
                self.delete_command(&fqn);
                self.error_restore(snap);
                return Code::Error;
            }
            // A constructor that destroys its own object (`[self] destroy`)
            // leaves nothing to return: `new`/`create` fails (oo-30.1/30.2,
            // Bug 2903011).
            if !self.oo.borrow().objects.contains_key(&fqn) {
                return self.error(b"object deleted in constructor");
            }
        }
        // With no constructor in the MRO, extra construction arguments are
        // silently ignored (matching tclsh9.0 — the default constructor is a
        // no-op that does not validate its argument count).
        self.set_result(obj::new_string_bytes(&fqn));
        Code::Ok
    }

    /// Invoke method `method` on `obj`. `external` enforces export visibility.
    fn oo_invoke(
        &mut self,
        obj: &[u8],
        method: &[u8],
        args: &[*mut TclObj],
        external: bool,
    ) -> Code {
        if !self.oo.borrow().objects.contains_key(obj) {
            return self.invalid_command(obj);
        }
        // A torn-down object (its destructor finished, structures dismantled, but
        // command still resolvable) has no call chain for anything — C's
        // `TclOOGetCallContext` returns NULL. This is what a nested-owned child's
        // destructor hits when it calls back into the parent mid-teardown
        // (oo-35.7.1/2, oo-11.8).
        if self
            .oo
            .borrow()
            .objects
            .get(obj)
            .is_some_and(|o| o.torn_down)
        {
            let mut m = b"impossible to invoke method \"".to_vec();
            m.extend_from_slice(method);
            m.extend_from_slice(b"\": no defined method or unknown method");
            let mut code = b"TCL LOOKUP METHOD ".to_vec();
            code.extend_from_slice(method);
            return self.error_with_code(&m, &code);
        }
        // TIP 500: a private (unexported) method is still visible to an external
        // call that originates from *within the same object* (e.g. `[self]
        // priv`), since the caller belongs to the object.
        let caller_is_self = self
            .oo
            .borrow()
            .call_stack
            .last()
            .is_some_and(|f| f.object.as_slice() == obj);
        let enforce = external && !caller_is_self;
        let providers = self.method_chain_faceted(obj);
        // An `export` of the method anywhere in the chain (e.g. on the object)
        // makes every step callable, overriding a class-level unexport.
        let exported_anywhere = self.method_exported(obj, method);
        // TIP 500: a TRUE_PRIVATE method is scoped to its declaring class/object.
        // The general call chain skips private methods (C's
        // `AddSimpleClassChainToCallContext`: `if (!IS_PRIVATE(mPtr))`); a
        // private step is added only when the caller's context scope — the
        // declaring entity of the currently-running method — is that same
        // provider. From non-method (external) code there is no scope, so all
        // private methods are invisible.
        let caller_scope: Option<Vec<u8>> = self
            .oo
            .borrow()
            .call_stack
            .last()
            .and_then(|f| f.chain.get(f.index).map(|s| s.provider.clone()));
        // The target-method steps: every provider that defines `method`. For an
        // external call, skip steps the provider unexports (unless overridden by
        // an export) — so a public override still runs while a private one is
        // skipped.
        let mut steps: Vec<CallStep> = providers
            .iter()
            .filter(|(p, is_obj)| {
                let is_obj = *is_obj;
                if !self.oo_has_method(p, method, is_obj) {
                    return false;
                }
                // A private method is in scope only from its own declarer.
                if self.method_is_private(p, method, is_obj) {
                    return caller_scope.as_deref() == Some(p.as_slice());
                }
                !(enforce && !exported_anywhere && self.method_unexported(p, method, is_obj))
            })
            .map(|(p, is_obj)| CallStep {
                provider: p.clone(),
                method: method.to_vec(),
                is_object: *is_obj,
            })
            .collect();
        // A caller-scope private shadows any public override further down the
        // chain, so move it to the front (it is the most-specific step).
        if !external {
            if let Some(c) = caller_scope.clone() {
                let is_obj = c.as_slice() == obj;
                if self.oo_has_method(&c, method, is_obj)
                    && self.method_is_private(&c, method, is_obj)
                {
                    if let Some(pos) = steps.iter().position(|s| s.provider == c) {
                        let s = steps.remove(pos);
                        steps.insert(0, s);
                    }
                }
            }
        }
        if steps.is_empty() {
            // An active object/class filter wraps even a built-in target (e.g.
            // `destroy`, `eval`): C's filters sit ahead of the real method, which
            // for these is the `oo::object` built-in. Route the filter chain
            // through `oo_run`; the filter's `next` reaches the built-in terminus
            // in `next_cmd` (oo-12.2/12.3). Only when the built-in is reachable —
            // otherwise the call still misses and falls to the unknown handler.
            if external {
                let destroy_ok = method == b"destroy"
                    && (exported_anywhere || !self.method_visibility_flags(obj, b"destroy").1);
                let objbuiltin_ok =
                    matches!(method, b"eval" | b"variable" | b"varname") && exported_anywhere;
                if destroy_ok || objbuiltin_ok {
                    let filters = self.active_filters(obj, &providers);
                    if !filters.is_empty() {
                        return self.oo_run(obj, filters, 0, method, args, external);
                    }
                }
            }
            // `destroy` is a built-in (overridable, PUBLIC) method on every
            // object; with no user override in the chain it tears the object
            // down. Reachable both externally (`$obj destroy`) and internally
            // (`my destroy`). When a class unexports it (e.g. `::oo::Slot`), an
            // external call misses and falls through to the unknown handler.
            if method == b"destroy"
                && (!enforce
                    || exported_anywhere
                    || !self.method_visibility_flags(obj, b"destroy").1)
            {
                let code = self.oo_destroy(obj);
                if code == Code::Ok {
                    self.set_result_bytes(b"");
                }
                return code;
            }
            // The class instantiation built-ins (`create`/`new`/
            // `createWithNamespace`) are reachable here when `obj` is a class
            // — notably via an internal `my create …` from a metaclass method
            // (oo-7.4/7.5). External calls honour the class's export state.
            if let Some(code) =
                self.oo_class_factory(obj, obj, method, args, enforce && !exported_anywhere)
            {
                return code;
            }
            // The object built-ins (`variable`/`varname`/`eval`) are not in any
            // method table; they are unexported by default (reachable internally
            // via `my`), but a public call reaches them too once explicitly
            // `export`ed.
            if !external || exported_anywhere {
                if let Some(code) = self.oo_builtin_method(obj, method, args, external) {
                    return code;
                }
            }
            // Record the originating scope for the unknown-method error so the
            // `unknown` handler's own frame does not mask it (restored after).
            let saved_scope = self
                .oo
                .borrow_mut()
                .unknown_scope
                .replace(caller_scope.clone().unwrap_or_default());
            let saved_external = self.oo.borrow().unknown_external;
            self.oo.borrow_mut().unknown_external = external;
            // A user-defined `unknown` method handles the miss (the method name
            // is prepended to the args), e.g. `oo::Slot`'s default-operation.
            // An *external* call of a hidden `unknown` (e.g. `$slot unknown`)
            // also routes here; only the internal fallback invocation (where the
            // handler itself missed) bypasses it, so we never recurse.
            let code = if (method != b"unknown" || external)
                && providers
                    .iter()
                    .any(|(p, is_obj)| self.oo_has_method(p, b"unknown", *is_obj))
            {
                let head = obj::new_string_bytes(method);
                unsafe { obj::incr_ref_count(head) };
                let mut uargs: Vec<*mut TclObj> = vec![head];
                for &a in args {
                    unsafe { obj::incr_ref_count(a) };
                    uargs.push(a);
                }
                // `unknown` is itself usually unexported, so dispatch it
                // internally (it is the object's own fallback handler).
                let code = self.oo_invoke(obj, b"unknown", &uargs, false);
                for a in uargs {
                    unsafe { obj::decr_ref_count(a) };
                }
                code
            } else {
                self.oo_unknown_method(obj, method)
            };
            self.oo.borrow_mut().unknown_scope = saved_scope;
            self.oo.borrow_mut().unknown_external = saved_external;
            return code;
        }
        // Filters wrap a method call (public or `my`) — prepend each active
        // filter as its own step — unless we're already handling a filter (C's
        // `FILTER_HANDLING`): a filter's own synchronous calls are not
        // re-wrapped, which avoids infinite recursion and matches the call-chain
        // model where the wrapped method (reached via `next`) re-enables filters
        // for *its* calls only (oo-12.x).
        if !self.oo.borrow().filter_handling {
            let filters = self.active_filters(obj, &providers);
            if !filters.is_empty() {
                let mut chain: Vec<CallStep> = filters;
                chain.append(&mut steps);
                return self.oo_run(obj, chain, 0, method, args, external);
            }
        }
        self.oo_run(obj, steps, 0, method, args, external)
    }

    /// The unexported object built-in methods (`variable`/`varname`/`eval`),
    /// defined on `oo::object` in C (`tclOOBasic.c`) and callable only via
    /// `my`. Returns `None` when `method` is not one of them (so the caller
    /// falls through to the unknown-method error).
    fn oo_builtin_method(
        &mut self,
        obj: &[u8],
        method: &[u8],
        args: &[*mut TclObj],
        external: bool,
    ) -> Option<Code> {
        let var_ns = self.oo.borrow().objects.get(obj).map(|o| o.var_ns)?;
        match method {
            // Link each named instance variable into the calling method frame.
            b"variable" => {
                for &a in args {
                    let name = obj_bytes(a);
                    if name.windows(2).any(|w| w == b"::") {
                        let mut m = b"variable name \"".to_vec();
                        m.extend_from_slice(&name);
                        m.extend_from_slice(b"\" illegal: must not contain namespace separator");
                        return Some(self.error(&m));
                    }
                    // An array-element name is rejected (C: `can't define "X":
                    // name refers to an element in an array`).
                    if name.last() == Some(&b')') && name.contains(&b'(') {
                        let mut m = b"can't define \"".to_vec();
                        m.extend_from_slice(&name);
                        m.extend_from_slice(b"\": name refers to an element in an array");
                        return Some(self.error(&m));
                    }
                    // A private variable of the caller's scope links to its
                    // mangled storage (TIP 500; oo-38.5).
                    match self.private_storage_name(&name) {
                        Some(storage) => self.make_variable_mapped(var_ns, &name, &storage),
                        None => self.make_variable(var_ns, &name),
                    }
                }
                self.set_result_bytes(b"");
                Some(Code::Ok)
            }
            // The fully-qualified name of one of the object's variables.
            b"varname" => {
                if args.len() != 1 {
                    return Some(wrong_args(self, b"my varname varName"));
                }
                // A private variable of the *calling* method's declaring scope
                // maps to its mangled storage name (TIP 500; oo-38.3).
                let want = obj_bytes(args[0]);
                let storage = self.private_storage_name(&want).unwrap_or(want);
                // Follow links to the real variable the name points at, so a
                // `namespace upvar`'d / linked name reports its target (oo-19.5).
                let full = self
                    .resolved_var_full_name(var_ns, &storage)
                    .unwrap_or_else(|| {
                        let mut f = self.namespaces().qualified_name(var_ns);
                        f.extend_from_slice(b"::");
                        f.extend_from_slice(&storage);
                        f
                    });
                self.set_result(obj::new_string_bytes(&full));
                Some(Code::Ok)
            }
            // Evaluate a script in the object's namespace (concatenating multiple
            // arguments with spaces, as `Tcl_ConcatObj` does).
            b"eval" => {
                if args.is_empty() {
                    return Some(wrong_args(self, b"my eval arg ?arg ...?"));
                }
                let script = if args.len() == 1 {
                    obj_bytes(args[0])
                } else {
                    let mut out: Vec<u8> = Vec::new();
                    for &a in args {
                        let b = obj_bytes(a);
                        let Some(start) = b.iter().position(|&c| !c.is_ascii_whitespace()) else {
                            continue;
                        };
                        let end = b.iter().rposition(|&c| !c.is_ascii_whitespace()).unwrap() + 1;
                        if !out.is_empty() {
                            out.push(b' ');
                        }
                        out.extend_from_slice(&b[start..end]);
                    }
                    out
                };
                let ns_name = self.namespaces().qualified_name(var_ns);
                // Run the script with an OO context frame so `self`/`my` work
                // inside it (`$obj eval {self}` returns the object; oo-18.12).
                self.oo.borrow_mut().call_stack.push(OoFrame {
                    object: obj.to_vec(),
                    chain: Vec::new(),
                    index: 0,
                    target: Vec::new(),
                    external,
                });
                let code = self.ns_eval_no_frame(&ns_name, &script);
                self.oo.borrow_mut().call_stack.pop();
                // C's FinalizeEval: on error append `(in "<name> eval" script
                // line N)`, where name is the object's name for a public call
                // and the literal `my` for an internal (non-public) one.
                if code == Code::Error {
                    let mut inner = b"in \"".to_vec();
                    if external {
                        inner.extend_from_slice(obj);
                    } else {
                        inner.extend_from_slice(b"my");
                    }
                    inner.extend_from_slice(b" eval\" script");
                    self.append_frame_line(&inner);
                    self.clear_error_logged();
                }
                Some(code)
            }
            _ => None,
        }
    }

    /// The active filter steps for `obj` (object filters, then class filters
    /// along the MRO), each resolved to the chain provider defining it.
    fn active_filters(&self, obj: &[u8], providers: &[(Vec<u8>, bool)]) -> Vec<CallStep> {
        let mut names: Vec<Vec<u8>> = Vec::new();
        let add = |n: &Vec<u8>, names: &mut Vec<Vec<u8>>| {
            if !names.contains(n) {
                names.push(n.clone());
            }
        };
        if let Some(o) = self.oo.borrow().objects.get(obj) {
            for f in &o.filters {
                add(f, &mut names);
            }
        }
        for (p, _) in providers {
            if let Some(c) = self.oo.borrow().classes.get(p) {
                for f in &c.filters {
                    add(f, &mut names);
                }
            }
        }
        // Each filter name contributes a step for *every* provider in the
        // precedence that implements it (in precedence order), so a filter
        // method overridden along the chain wraps once per implementation —
        // e.g. `Emix.f` then `B.f` (oo-21.3).
        names
            .iter()
            .flat_map(|fname| {
                providers
                    .iter()
                    .filter(|(p, is_obj)| self.oo_has_method(p, fname, *is_obj))
                    .map(|(p, is_obj)| CallStep {
                        provider: p.clone(),
                        method: fname.clone(),
                        is_object: *is_obj,
                    })
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    /// The method-resolution (precedence) chain for `obj`: a depth-first walk —
    /// the object's mixins, then the object itself, then its class's
    /// linearization (each class: its mixins, then the class, then its
    /// superclasses) — with each provider kept at its **last** occurrence. This
    /// "keep-last" dedup defers shared bases (a diamond's apex, a mixin's
    /// superclass shared with the class chain) to after everything that derives
    /// from them, matching C's `TclOOComputePrecedenceList` / call-chain order
    /// (oo-21.x C3 ordering, oo-14.x mixins).
    fn method_chain(&self, obj: &[u8]) -> Vec<Vec<u8>> {
        self.method_chain_faceted(obj)
            .into_iter()
            .map(|(p, _)| p)
            .collect()
    }

    /// Like [`method_chain`] but each provider is tagged with its facet
    /// (`is_object`): the object's own methods vs a class's instance methods.
    /// The object facet and a self-mixed class facet share the FQN but are
    /// distinct steps, so the dedup keeps each `(provider, facet)` pair (not
    /// just each name) — needed to dispatch a `self mixin <self>` method.
    fn method_chain_faceted(&self, obj: &[u8]) -> Vec<(Vec<u8>, bool)> {
        let (obj_mixins, cls) = {
            let oo = self.oo.borrow();
            match oo.objects.get(obj) {
                Some(o) => (o.mixins.clone(), Some(o.class.clone())),
                None => return vec![(obj.to_vec(), true)],
            }
        };
        let mut seq: Vec<(Vec<u8>, bool)> = Vec::new();
        let mut path: Vec<Vec<u8>> = Vec::new();
        // One visit budget shared across every mixin and the class chain below
        // (issue #996 — see `MAX_MRO_VISITS`): the cap is on *this whole
        // dispatch's* total linearisation work, not on each sub-walk
        // independently.
        let mut budget = MAX_MRO_VISITS;
        // Mixins and the class hierarchy contribute *class*-facet steps; the
        // object itself sits between the object mixins and its class chain.
        for mx in &obj_mixins {
            let mut tmp: Vec<Vec<u8>> = Vec::new();
            self.linearize_class(mx, &mut tmp, &mut path, 0, &mut budget);
            seq.extend(tmp.into_iter().map(|c| (c, false)));
        }
        seq.push((obj.to_vec(), true));
        if let Some(cls) = cls {
            let mut tmp: Vec<Vec<u8>> = Vec::new();
            self.linearize_class(&cls, &mut tmp, &mut path, 0, &mut budget);
            seq.extend(tmp.into_iter().map(|c| (c, false)));
        }
        // Keep each (provider, facet) at its last occurrence (drop earlier dups).
        let mut chain: Vec<(Vec<u8>, bool)> = Vec::with_capacity(seq.len());
        for (i, c) in seq.iter().enumerate() {
            if !seq[i + 1..].iter().any(|x| x == c) {
                chain.push(c.clone());
            }
        }
        chain
    }

    /// If `name` is a TIP 500 private instance variable of the currently-running
    /// method's declaring scope, its mangled storage name (`"<creationEpoch> :
    /// name"`); otherwise `None`. The scope is the provider of the call-stack's
    /// top frame (the method that invoked the built-in).
    fn private_storage_name(&self, name: &[u8]) -> Option<Vec<u8>> {
        let oo = self.oo.borrow();
        let frame = oo.call_stack.last()?;
        let prov = frame.chain.get(frame.index)?.provider.clone();
        let is_object = prov == frame.object;
        let is_private = if is_object {
            oo.objects
                .get(&prov)
                .is_some_and(|o| o.private_variables.iter().any(|v| v == name))
        } else {
            oo.classes
                .get(&prov)
                .is_some_and(|c| c.private_variables.iter().any(|v| v == name))
        };
        if !is_private {
            return None;
        }
        let epoch = oo.objects.get(&prov).map(|o| o.creation_id).unwrap_or(0);
        let mut s = format!("{epoch} : ").into_bytes();
        s.extend_from_slice(name);
        Some(s)
    }

    /// Copy the procedures and variables of namespace `src_ns` into `dst_ns`
    /// (C's `TclCopyNamespaceProcedures`/`Variables`, used by `<cloned>`).
    fn oo_clone_namespace(&mut self, src_ns: NsId, dst_ns: NsId) {
        // Procedures (skip built-ins like the per-object `my`/`myclass`).
        let names: Vec<Vec<u8>> = self
            .namespaces()
            .command_names(src_ns)
            .iter()
            .map(|n| n.to_vec())
            .collect();
        // The destination namespace's FQN, for re-pointing copied procs.
        let dst_qual = {
            let q = self.namespaces().qualified_name(dst_ns);
            if q == b"::" {
                Vec::new()
            } else {
                q
            }
        };
        for n in &names {
            // Bind the resolved command before borrowing mutably (the `Ref` from
            // `namespaces()` must be dropped first).
            let cmd = self.namespaces().resolve(src_ns, n);
            if let Some(Command::Proc(def)) = cmd {
                // Re-point the copy to the destination namespace, so its body's
                // `variable`/unqualified names resolve there, not in the source.
                let mut new_def = (*def).clone();
                new_def.ns = dst_ns;
                new_def.fqn = {
                    let mut f = dst_qual.clone();
                    f.extend_from_slice(b"::");
                    f.extend_from_slice(n);
                    f
                };
                self.namespaces_mut()
                    .bind(dst_ns, n, Command::Proc(std::rc::Rc::new(new_def)));
            }
        }
        // Variables (scalars and array elements; `store_*` retains the values).
        type ArrayCopy = (Vec<u8>, Vec<(Vec<u8>, *mut TclObj)>);
        let mut scalars: Vec<(Vec<u8>, *mut TclObj)> = Vec::new();
        let mut arrays: Vec<ArrayCopy> = Vec::new();
        {
            let ns = self.namespaces();
            let table = ns.var_table(src_ns);
            for name in ns.var_names(src_ns) {
                match table.cell(&name) {
                    Some(crate::frame::Var::Scalar(p)) => scalars.push((name, *p)),
                    Some(crate::frame::Var::Array(map)) => {
                        arrays.push((name, map.iter().map(|(k, v)| (k.clone(), *v)).collect()));
                    }
                    _ => {}
                }
            }
        }
        for (name, p) in scalars {
            let _ = self
                .namespaces_mut()
                .var_table_mut(dst_ns)
                .store_scalar(&name, p);
        }
        for (name, elems) in arrays {
            for (k, p) in elems {
                let _ = self
                    .namespaces_mut()
                    .var_table_mut(dst_ns)
                    .store_elem(&name, &k, p);
            }
        }
    }

    /// Resolve an object name to its FQN, following namespace-import aliases to
    /// the origin command if the qualified name is not itself an object (so an
    /// imported object command resolves to the real object; oo-1.10).
    fn oo_resolve_object(&self, name: &[u8]) -> Vec<u8> {
        let fqn = self.fqn_for(name);
        if self.oo.borrow().objects.contains_key(&fqn) {
            return fqn;
        }
        if let Some(o) = tcl_cmd_core::namespace::origin(self, &String::from_utf8_lossy(name))
            .map(String::into_bytes)
            .filter(|o| self.oo.borrow().objects.contains_key(o))
        {
            return o;
        }
        // Object/class names resolve like commands: an unqualified name not
        // found in the current namespace falls back to the global namespace
        // (e.g. `oo::objdefine [self] class Sub` from inside a method, where the
        // current namespace is the object's instance namespace; oo-3.11).
        if !name.starts_with(b"::") {
            let mut global = b"::".to_vec();
            global.extend_from_slice(name);
            if self.oo.borrow().objects.contains_key(&global) {
                return global;
            }
        }
        fqn
    }

    /// The precedence (linearization) of a *class* itself — its mixins, the
    /// class, then its superclasses — keep-last deduped, for `info class call`.
    fn class_precedence(&self, class: &[u8]) -> Vec<Vec<u8>> {
        let mut seq: Vec<Vec<u8>> = Vec::new();
        let mut path: Vec<Vec<u8>> = Vec::new();
        let mut budget = MAX_MRO_VISITS;
        self.linearize_class(class, &mut seq, &mut path, 0, &mut budget);
        let mut out: Vec<Vec<u8>> = Vec::with_capacity(seq.len());
        for (i, c) in seq.iter().enumerate() {
            if !seq[i + 1..].iter().any(|x| x == c) {
                out.push(c.clone());
            }
        }
        out
    }

    /// Depth-first precedence of a class into `seq` (with duplicates, resolved by
    /// the caller's keep-last dedup): the class's mixins (recursively), then the
    /// class itself, then its superclasses. `path` breaks cycles in a malformed
    /// hierarchy without suppressing the legitimate re-visits keep-last relies on.
    ///
    /// `depth` is this call's nesting level (0 at the root) and `budget` is the
    /// remaining total-visit allowance shared across the whole linearisation
    /// (both callers') — see [`MAX_MRO_DEPTH`]/[`MAX_MRO_VISITS`] (issue #996).
    /// Past either cap, this call stops descending without recursing further:
    /// the already-linearised prefix in `seq` is kept as-is, the same graceful
    /// "just stop" degradation the `path` cycle guard above already applies to
    /// a malformed hierarchy, rather than overflowing the native stack or
    /// hanging on an adversarial diamond-mixin shape.
    fn linearize_class(
        &self,
        class: &[u8],
        seq: &mut Vec<Vec<u8>>,
        path: &mut Vec<Vec<u8>>,
        depth: u32,
        budget: &mut u32,
    ) {
        if MAX_MRO_DEPTH.exceeded(depth) || *budget == 0 {
            return;
        }
        *budget -= 1;
        if path.iter().any(|c| c == class) {
            return;
        }
        path.push(class.to_vec());
        let (mixins, supers) = {
            let oo = self.oo.borrow();
            match oo.classes.get(class) {
                Some(c) => (c.mixins.clone(), c.supers.clone()),
                None => (Vec::new(), Vec::new()),
            }
        };
        for mx in &mixins {
            self.linearize_class(mx, seq, path, depth + 1, budget);
        }
        seq.push(class.to_vec());
        for s in &supers {
            self.linearize_class(s, seq, path, depth + 1, budget);
        }
        path.pop();
    }

    /// Sorted (ASCII) unique property names for a class. With `all`, unions the
    /// class's readable/writable set over its mixins and superclasses (C's
    /// `FindClassProps`); otherwise just the class's own set (oo-1.1–1.6).
    fn class_property_list(&self, class: &[u8], writable: bool, all: bool) -> Vec<Vec<u8>> {
        let mut acc: Vec<Vec<u8>> = Vec::new();
        if all {
            let mut seen: Vec<Vec<u8>> = Vec::new();
            let mut budget = MAX_MRO_VISITS;
            self.gather_class_props(class, writable, &mut acc, &mut seen, 0, &mut budget);
        } else if let Some(c) = self.oo.borrow().classes.get(class) {
            let src = if writable {
                &c.writable_properties
            } else {
                &c.readable_properties
            };
            acc.extend(src.iter().cloned());
        }
        acc.sort();
        acc.dedup();
        acc
    }

    /// `depth`/`budget` — see [`linearize_class`](Self::linearize_class), which
    /// this mirrors (issue #996). Unlike `linearize_class`'s `path`,
    /// `gather_class_props`'s `seen` is a *global* (never-popped) visited set,
    /// so a diamond is naturally visited once, not once per reaching path —
    /// `budget` is therefore more a defensive second layer than a load-bearing
    /// one here, added for consistency with the sibling walk rather than
    /// because this shape is independently known to blow up; `depth` is the
    /// one that matters, guarding a long mixin/superclass *chain* the same way
    /// `linearize_class`'s does.
    fn gather_class_props(
        &self,
        class: &[u8],
        writable: bool,
        acc: &mut Vec<Vec<u8>>,
        seen: &mut Vec<Vec<u8>>,
        depth: u32,
        budget: &mut u32,
    ) {
        if MAX_MRO_DEPTH.exceeded(depth) || *budget == 0 {
            return;
        }
        *budget -= 1;
        if seen.iter().any(|c| c == class) {
            return;
        }
        seen.push(class.to_vec());
        let (props, mixins, supers) = {
            let oo = self.oo.borrow();
            match oo.classes.get(class) {
                Some(c) => {
                    let p = if writable {
                        c.writable_properties.clone()
                    } else {
                        c.readable_properties.clone()
                    };
                    (p, c.mixins.clone(), c.supers.clone())
                }
                None => return,
            }
        };
        for p in props {
            if !acc.contains(&p) {
                acc.push(p);
            }
        }
        for m in &mixins {
            self.gather_class_props(m, writable, acc, seen, depth + 1, budget);
        }
        for s in &supers {
            self.gather_class_props(s, writable, acc, seen, depth + 1, budget);
        }
    }

    /// Sorted unique property names for an object. With `all`, unions the
    /// object's own set with its full precedence (mixins + class chain);
    /// otherwise just the object's own set (oo-1.7–1.11).
    fn object_property_list(&self, obj: &[u8], writable: bool, all: bool) -> Vec<Vec<u8>> {
        let mut acc: Vec<Vec<u8>> = Vec::new();
        let push = |src: &[Vec<u8>], acc: &mut Vec<Vec<u8>>| {
            for p in src {
                if !acc.contains(p) {
                    acc.push(p.clone());
                }
            }
        };
        if let Some(o) = self.oo.borrow().objects.get(obj) {
            let src = if writable {
                &o.writable_properties
            } else {
                &o.readable_properties
            };
            push(src, &mut acc);
        }
        if all {
            for c in self.method_chain(obj) {
                if c == obj {
                    continue;
                }
                if let Some(cl) = self.oo.borrow().classes.get(&c) {
                    let src = if writable {
                        &cl.writable_properties
                    } else {
                        &cl.readable_properties
                    };
                    push(src, &mut acc);
                }
            }
        }
        acc.sort();
        acc.dedup();
        acc
    }

    /// If `word` (a forward's command name) names a command in `obj`'s instance
    /// namespace (e.g. the per-object `my`/`myclass`), return its fully-qualified
    /// form so it resolves while the command still runs in the caller's context;
    /// otherwise return `word` unchanged (resolves globally).
    fn qualify_in_object_ns(&self, obj: &[u8], word: &[u8]) -> Vec<u8> {
        if word.starts_with(b"::") {
            return word.to_vec();
        }
        let var_ns = self.oo.borrow().objects.get(obj).map(|o| o.var_ns);
        if let Some(ns) = var_ns {
            // Resolve the word as a command in the object's namespace. Only a
            // command whose home lies *within* the object's namespace subtree
            // (the per-object `my`/`myclass`, or a proc the object put in a child
            // namespace — oo-6.6) gets qualified; one merely visible via the
            // global fallback is left bare so it keeps its caller-context vars.
            let mut prefix = self.namespaces().qualified_name(ns);
            if prefix != b"::" {
                prefix.extend_from_slice(b"::");
            }
            if let Some(fqn) = self.namespaces().resolve_fqn(ns, word) {
                if fqn.starts_with(&prefix) {
                    return fqn;
                }
            }
        }
        word.to_vec()
    }

    /// Whether the provider `prov` (object FQN or class FQN) defines `method`
    /// (or a constructor, when `ctor`).
    /// Whether provider `prov` defines `method`. Resolution is positional: the
    /// object itself (`is_object`, the chain head) carries *per-object* methods
    /// (`objects[prov].methods`); the rest of the chain are classes carrying
    /// *instance* methods (`classes[prov].methods`). A class is registered in
    /// both maps (classes-as-objects), so this distinction must be by position,
    /// not by which map `prov` is in.
    fn oo_has_method(&self, prov: &[u8], method: &[u8], is_object: bool) -> bool {
        if is_object {
            self.oo
                .borrow()
                .objects
                .get(prov)
                .is_some_and(|o| o.methods.contains_key(method))
        } else {
            self.oo
                .borrow()
                .classes
                .get(prov)
                .is_some_and(|c| c.methods.contains_key(method))
        }
    }

    /// Whether `obj` has a *user-defined* `unknown` method (one declared on the
    /// object or a class other than the `oo::object` root, whose built-in
    /// `unknown` is the default error). A no-method invocation (`$obj`) routes
    /// to a user handler but otherwise reports the `wrong # args` usage.
    fn has_user_unknown(&self, obj: &[u8]) -> bool {
        self.method_chain(obj).iter().any(|p| {
            p.as_slice() != b"::oo::object"
                && self.oo_has_method(p, b"unknown", p.as_slice() == obj)
        })
    }

    /// Whether the class `prov` defines a constructor.
    fn class_has_ctor(&self, prov: &[u8]) -> bool {
        self.oo
            .borrow()
            .classes
            .get(prov)
            .is_some_and(|c| c.constructor.is_some())
    }

    /// Whether `method` was explicitly `export`ed anywhere in `obj`'s method
    /// chain (the object itself or any class in its MRO) — promotes a default-
    /// unexported built-in (`eval`/`variable`/`varname`) to public.
    fn method_exported(&self, obj: &[u8], method: &[u8]) -> bool {
        self.method_chain(obj).iter().any(|p| {
            let oo = self.oo.borrow();
            if p.as_slice() == obj {
                oo.objects
                    .get(p)
                    .is_some_and(|o| o.exported.contains(method))
            } else {
                oo.classes
                    .get(p)
                    .is_some_and(|c| c.exported.contains(method))
            }
        })
    }

    /// `(exported, unexported, private)` membership for `method` aggregated over
    /// `obj`'s whole method chain — used to resolve a built-in method's effective
    /// visibility for `info object methods -all`.
    fn method_visibility_flags(&self, obj: &[u8], method: &[u8]) -> (bool, bool, bool) {
        let (mut exp, mut unexp, mut priv_) = (false, false, false);
        for p in self.method_chain(obj) {
            let oo = self.oo.borrow();
            let sets = if p.as_slice() == obj {
                oo.objects
                    .get(&p)
                    .map(|o| (&o.exported, &o.unexported, &o.private))
            } else {
                oo.classes
                    .get(&p)
                    .map(|c| (&c.exported, &c.unexported, &c.private))
            };
            if let Some((e, u, pr)) = sets {
                exp |= e.contains(method);
                unexp |= u.contains(method);
                priv_ |= pr.contains(method);
            }
        }
        (exp, unexp, priv_)
    }

    /// Whether `method` is a TIP 500 *private* method of `prov` (a subset of its
    /// unexported methods — scoped to `prov`'s own methods).
    fn method_is_private(&self, prov: &[u8], method: &[u8], is_object: bool) -> bool {
        if is_object {
            self.oo
                .borrow()
                .objects
                .get(prov)
                .is_some_and(|o| o.private.contains(method))
        } else {
            self.oo
                .borrow()
                .classes
                .get(prov)
                .is_some_and(|c| c.private.contains(method))
        }
    }

    fn method_unexported(&self, prov: &[u8], method: &[u8], is_object: bool) -> bool {
        if is_object {
            self.oo
                .borrow()
                .objects
                .get(prov)
                .is_some_and(|o| o.unexported.contains(method))
        } else {
            self.oo
                .borrow()
                .classes
                .get(prov)
                .is_some_and(|c| c.unexported.contains(method))
        }
    }

    /// Run the call-chain step at `index` on `obj`. `target` is the originally
    /// invoked method (empty for a constructor), recorded for `self target`.
    fn oo_run(
        &mut self,
        obj: &[u8],
        chain: Vec<CallStep>,
        index: usize,
        target: &[u8],
        args: &[*mut TclObj],
        external: bool,
    ) -> Code {
        let prov = chain[index].provider.clone();
        let method = chain[index].method.clone();
        // Object-vs-class facet of this step (a class mixed into its own instance
        // contributes both, so use the step's recorded facet, not `prov == obj`).
        let is_object = chain[index].is_object;
        // The `::oo::class` synthetic constructor: instantiating a metaclass
        // runs the (optional) definition-script argument on the new class. C's
        // `oo::class` constructor is `{{definitionScript ""}}`.
        if method.is_empty() && prov == b"::oo::class" {
            let body = args.first().map(|&a| obj_bytes(a)).unwrap_or_default();
            if body.is_empty() {
                self.set_result_bytes(b"");
                return Code::Ok;
            }
            return self.oo_define_body(DefTarget::Class(obj.to_vec()), &body, None);
        }
        let m = if method.is_empty() {
            self.oo
                .borrow()
                .classes
                .get(&prov)
                .and_then(|c| c.constructor.clone())
        } else if method == b"<destructor>" {
            // A destructor chain step: the body lives in `classes[prov]`.
            self.oo
                .borrow()
                .classes
                .get(&prov)
                .and_then(|c| c.destructor.clone())
                .map(|body| Method::Body {
                    params: Vec::new(),
                    body,
                    src: None,
                })
        } else if is_object {
            self.oo
                .borrow()
                .objects
                .get(&prov)
                .and_then(|o| o.methods.get(&method).cloned())
        } else {
            self.oo
                .borrow()
                .classes
                .get(&prov)
                .and_then(|c| c.methods.get(&method).cloned())
        };
        let Some(m) = m else {
            return self.error(b"no such method");
        };

        // A native method runs without a Tcl call frame (the OO context frame is
        // already pushed by the caller for `self`/`my`).
        if let Method::Builtin(f) = m {
            self.oo.borrow_mut().call_stack.push(OoFrame {
                object: obj.to_vec(),
                chain,
                index,
                target: target.to_vec(),
                external,
            });
            let code = f(self, obj, args);
            self.oo.borrow_mut().call_stack.pop();
            return code;
        }

        // A forward: build `prefix + args` and dispatch (with the OO context so a
        // forwarded `my`/`self` still works).
        if let Method::Forward { prefix } = &m {
            let mut new_argv: Vec<*mut TclObj> = Vec::with_capacity(prefix.len() + args.len());
            // The forwarded command resolves in the object's namespace, so a bare
            // `my`/`myclass` works; but the command then *runs* in the caller's
            // context (variables resolve there). Achieve both by qualifying the
            // first word with the object namespace only when it resolves there.
            for (i, p) in prefix.iter().enumerate() {
                let word = if i == 0 {
                    self.qualify_in_object_ns(obj, p)
                } else {
                    p.clone()
                };
                let o = obj::new_string_bytes(&word);
                unsafe { obj::incr_ref_count(o) };
                new_argv.push(o);
            }
            for &a in args {
                unsafe { obj::incr_ref_count(a) };
                new_argv.push(a);
            }
            // The method this forward (possibly via a chain of forwards /
            // ensembles) leads to reports `wrong # args` against the *original*
            // invocation, not the forwarded command (C's ensemble rewrite). The
            // first forward in the chain is the root: record the invocation words
            // (`obj method ?args...?`); a downstream `wrong # args` rewrites
            // against them. `fwd_usage` keeps the simple single-step prefix for
            // the immediate next body's `info`-level naming.
            let head = if external {
                object_display(obj)
            } else {
                b"my".to_vec()
            };
            let mut fwd = head.clone();
            fwd.push(b' ');
            fwd.extend_from_slice(target);
            self.oo.borrow_mut().fwd_usage = Some(fwd);
            let mut source = vec![head, target.to_vec()];
            source.extend(args.iter().map(|&a| obj_bytes(a)));
            // `obj method` (2 words) is replaced by the forward `prefix`.
            let is_root = self.begin_ensemble_rewrite(source, 2, prefix.len());
            self.oo.borrow_mut().call_stack.push(OoFrame {
                object: obj.to_vec(),
                chain,
                index,
                target: target.to_vec(),
                external,
            });
            let code = self.dispatch(&new_argv);
            self.oo.borrow_mut().call_stack.pop();
            if is_root {
                self.clear_ensemble_rewrite();
            }
            // Clear the rewrite if the forwarded command was not a method body
            // (which would have consumed it).
            self.oo.borrow_mut().fwd_usage = None;
            for a in new_argv {
                unsafe { obj::decr_ref_count(a) };
            }
            return code;
        }

        let Method::Body { params, body, src } = m else {
            unreachable!("forward handled above");
        };
        let Some(var_ns) = self.oo.borrow().objects.get(obj).map(|o| o.var_ns) else {
            let mut m = b"object \"".to_vec();
            m.extend_from_slice(obj);
            m.extend_from_slice(b"\" has been deleted");
            return self.error(&m);
        };
        // The declared instance variables visible to the method, as `(local,
        // storage)` links: public variables (the object's own plus the union
        // over its full class hierarchy) link name→name; TIP 500 private
        // variables of the *declaring* provider link name→a per-class mangled
        // storage name (`"<creationEpoch> : name"`, C's PRIVATE_VARIABLE_PATTERN),
        // so identically-named private vars in different classes don't collide.
        let mut vars: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
        let push_public = |v: &Vec<u8>, vars: &mut Vec<(Vec<u8>, Vec<u8>)>| {
            if !vars.iter().any(|(l, _)| l == v) {
                vars.push((v.clone(), v.clone()));
            }
        };
        // Private vars first, so a same-named public var down the chain does not
        // shadow the declaring provider's private mapping.
        {
            let oo = self.oo.borrow();
            let privates = if is_object {
                oo.objects.get(&prov).map(|o| o.private_variables.clone())
            } else {
                oo.classes.get(&prov).map(|c| c.private_variables.clone())
            };
            // A class is also an object, so its creation id lives in `objects`.
            let epoch = oo.objects.get(&prov).map(|o| o.creation_id).unwrap_or(0);
            if let Some(pv) = privates {
                for v in pv {
                    if !vars.iter().any(|(l, _)| *l == v) {
                        let storage = format!("{epoch} : ").into_bytes();
                        let mut storage = storage;
                        storage.extend_from_slice(&v);
                        vars.push((v, storage));
                    }
                }
            }
        }
        // Public `variable` declarations are scoped to the *declaring* provider
        // (C links `clsPtr->variables` / `oPtr->variables` of the method's
        // declarer, not the whole hierarchy): a class method sees its class's
        // declared vars, a per-object method sees the object's (oo-38.4).
        {
            let oo = self.oo.borrow();
            let declared = if is_object {
                oo.objects.get(&prov).map(|o| o.variables.clone())
            } else {
                oo.classes.get(&prov).map(|c| c.variables.clone())
            };
            if let Some(vs) = declared {
                for v in &vs {
                    push_public(v, &mut vars);
                }
            }
        }
        let name = if method.is_empty() {
            b"<constructor>".to_vec()
        } else {
            method.clone()
        };
        // The method-body errorInfo frame (`CommonMethErrorHandler`): named by
        // the *declaring* entity — `object`/`class` per whether the provider is
        // the object itself or one of its classes.
        let kind: &[u8] = if is_object { b"object" } else { b"class" };
        let what = if method.is_empty() {
            MethodFrameWhat::Constructor
        } else if method == b"<destructor>" {
            MethodFrameWhat::Destructor
        } else {
            MethodFrameWhat::Named(&method)
        };
        self.oo.borrow_mut().call_stack.push(OoFrame {
            object: obj.to_vec(),
            chain,
            index,
            target: target.to_vec(),
            external,
        });
        // A method defined while sourcing a file carries that file + the body's
        // line base, so `info frame` reports file-absolute lines (TIP 280).
        let (source, body_line_base) = match &src {
            Some((file, base)) => (Some(file.clone()), *base),
            None => (None, 0),
        };
        // `wrong # args` prefix: a forward's rewritten original invocation, else
        // the invoking `obj method` (external) / `my method` (internal). Only
        // regular methods (constructors/destructors keep their synthetic name).
        let usage_prefix: Option<Vec<u8>> = if method.is_empty() || method == b"<destructor>" {
            None
        } else if let Some(u) = self.oo.borrow_mut().fwd_usage.take() {
            Some(u)
        } else {
            let mut p = if external {
                object_display(obj)
            } else {
                b"my".to_vec()
            };
            p.push(b' ');
            p.extend_from_slice(&method);
            Some(p)
        };
        // A constructor's `info level 0` reports the originating `create`/`new`
        // invocation (e.g. `oo::object create foo`), captured by the dispatcher,
        // rather than the synthetic `<constructor>` name (oo-2.1).
        let level_words = if method.is_empty() {
            self.oo.borrow_mut().ctor_words.take()
        } else {
            None
        };
        // A filter step (its method differs from the invoked target) runs with
        // `filter_handling` set, so its own `my` calls — and everything they
        // call — are not re-wrapped by the same filters. A `next` from the filter
        // clears it again for the wrapped method (handled in `next_cmd`). Saved
        // and restored so nested filter chains compose.
        let is_filter_step = !method.is_empty() && method != target;
        let saved_fh = self.oo.borrow().filter_handling;
        if is_filter_step {
            self.oo.borrow_mut().filter_handling = true;
        }
        let code = self.run_proc(
            &params,
            &body,
            var_ns,
            args,
            &name,
            CallMeta {
                err: ProcFrame::Method {
                    kind,
                    owner: &prov,
                    what,
                },
                fqn: None,
                source,
                body_line_base,
                link_vars: &vars,
                // Property accessors propagate break/continue to `configure`.
                keep_loop_codes: method.starts_with(b"<ReadProp-")
                    || method.starts_with(b"<WriteProp-"),
                // A non-first chain step is reached via `next`: it shares the
                // level of the original method invocation (step 0).
                same_level: index > 0,
                usage_prefix,
                level_words,
                quote_name: false,
            },
        );
        self.oo.borrow_mut().filter_handling = saved_fh;
        self.oo.borrow_mut().call_stack.pop();
        code
    }

    /// Destroy an object during implicit teardown, routing a destructor error to
    /// the `interp bgerror` handler (C's `AfterNRDestructor` → background error).
    fn oo_destroy_bg(&mut self, obj: &[u8]) {
        if self.oo_destroy(obj) == Code::Error {
            let msg = self.result_bytes();
            let ec = self.error_code();
            let options = build_list(&[
                b"-code".to_vec(),
                b"1".to_vec(),
                b"-level".to_vec(),
                b"0".to_vec(),
                b"-errorcode".to_vec(),
                ec,
                b"-errorinfo".to_vec(),
                msg.clone(),
            ]);
            self.report_bg_error(&msg, &options);
        }
    }

    /// Destroy an object. Returns the destructor's result `Code`: explicit
    /// `obj destroy` propagates a destructor error to its caller (C's
    /// `AfterNRDestructor` returns the destructor result), whereas implicit
    /// teardown (`rename {}`/`namespace delete`/class cascade) reports it to
    /// `bgerror`. Cleanup of the command/namespace happens regardless of the
    /// destructor outcome.
    fn oo_destroy(&mut self, obj: &[u8]) -> Code {
        // Re-entrancy guard (C's DESTRUCTOR_CALLED): run the destructor at most
        // once even if `destroy`/`rename {}`/`namespace delete` all reach here.
        let state = self.oo.borrow().objects.get(obj).map(|o| o.destroyed);
        let mut dtor_code = Code::Ok;
        match state {
            None => return Code::Ok, // not an object (or already fully removed)
            Some(false) => {
                if let Some(o) = self.oo.borrow_mut().objects.get_mut(obj) {
                    o.destroyed = true;
                }
                dtor_code = self.oo_run_destructors(obj);
            }
            Some(true) => {} // destructor already running/ran — fall through to cleanup
        }
        // The destructor has run; the object is now being dismantled. Mark it
        // torn-down *before* the descendant cascade so a nested-owned child whose
        // destructor calls back into this object sees "impossible to invoke
        // method" (C guts the object in `ObjectNamespaceDeleted` before its
        // child namespaces are deleted). The object stays in the registry — and
        // its command resolvable — until cleanup below.
        if let Some(o) = self.oo.borrow_mut().objects.get_mut(obj) {
            o.torn_down = true;
        }
        // If the object is also a class, its subclasses and instances are
        // descendants too (C's `TclOODeleteDescendants`): destroy them, so e.g.
        // tearing down a class reached as another metaclass's instance also tears
        // down that class's own instances (oo-16.14 pollution).
        if self.oo.borrow().classes.contains_key(obj) {
            self.oo_destroy_class_descendants(obj);
        }
        let var_ns = self.oo.borrow().objects.get(obj).map(|o| o.var_ns);
        if let Some(ns) = var_ns {
            // Nested ownership (C's `ObjectNamespaceDeleted` →
            // `TclOODeleteDescendants`): an object created inside this object's
            // instance namespace is owned by it, so destroying this object tears
            // those children down too — in nesting order, while their namespaces
            // are still intact. `oo_namespace_deleted` skips already-destroyed
            // objects, so this only reaches the (not-yet-destroyed) descendants.
            self.oo_namespace_deleted(ns);
        }
        let aliases = self
            .oo
            .borrow()
            .objects
            .get(obj)
            .map(|o| o.my_aliases.clone())
            .unwrap_or_default();
        // Delete the instance namespace first — this unsets its variables and
        // fires their unset traces while the object is still registered and
        // torn-down, so a trace callback sees `info object isa object` true, the
        // namespace already gone, and a method call as "impossible to invoke
        // method" (C's ObjectNamespaceDeleted order; oo-11.8).
        if let Some(ns) = var_ns {
            self.delete_namespace_by_id(ns);
        }
        // Then drop the registry entries (a metaclass instance is in both maps),
        // the command, and the per-object `my`/`myclass` (a `my` renamed out of
        // the namespace is tied to the object's lifetime in C, so delete it too).
        self.oo.borrow_mut().objects.remove(obj);
        self.oo.borrow_mut().classes.remove(obj);
        self.delete_command(obj);
        for a in aliases {
            self.delete_command(&a);
        }
        dtor_code
    }

    /// Run the object's destructor chain (every class in the MRO that declares a
    /// `destructor`, most-derived first, with `next` chaining), with the object
    /// still intact so the body's `my`/variables resolve.
    fn oo_run_destructors(&mut self, obj: &[u8]) -> Code {
        let Some(class) = self.oo.borrow().objects.get(obj).map(|o| o.class.clone()) else {
            return Code::Ok;
        };
        let chain: Vec<CallStep> = self
            .mro(&class)
            .into_iter()
            .filter(|c| {
                self.oo
                    .borrow()
                    .classes
                    .get(c)
                    .is_some_and(|cl| cl.destructor.is_some())
            })
            .map(|c| CallStep {
                is_object: c.as_slice() == obj,
                provider: c,
                method: b"<destructor>".to_vec(),
            })
            .collect();
        if chain.is_empty() {
            return Code::Ok;
        }
        // The destructor result is returned to the caller: explicit `obj
        // destroy` propagates it (C's `AfterNRDestructor`); implicit teardown
        // ignores it (routing to `bgerror`).
        self.oo_run(obj, chain, 0, b"<destructor>", &[], false)
    }

    fn oo_destroy_class(&mut self, class: &[u8]) {
        // Already being torn down (a cycle of mixins/superclasses, or reached
        // again via an instance cascade) — nothing more to do.
        if self
            .oo
            .borrow()
            .objects
            .get(class)
            .is_none_or(|o| o.destroyed)
        {
            return;
        }
        // Destroying a class cascades to its subclasses and instances (TclOO),
        // then removes the class object itself. Mark it destroyed first so the
        // cascade does not re-enter it.
        if let Some(o) = self.oo.borrow_mut().objects.get_mut(class) {
            o.destroyed = true;
        }
        self.oo_destroy_class_descendants(class);
        // Drop the registry entries, the command, the per-object `my`/`myclass`,
        // and the class's own instance namespace — a class created with an
        // explicit target namespace (`oo::copy … <ns>`) owns it, so destroying
        // the class must delete it too (oo-15.13.x), as `oo_destroy` does.
        let (var_ns, aliases) = {
            let oo = self.oo.borrow();
            let o = oo.objects.get(class);
            (
                o.map(|o| o.var_ns),
                o.map(|o| o.my_aliases.clone()).unwrap_or_default(),
            )
        };
        self.oo.borrow_mut().classes.remove(class);
        self.oo.borrow_mut().objects.remove(class);
        self.delete_command(class);
        for a in aliases {
            self.delete_command(&a);
        }
        if let Some(ns) = var_ns {
            self.oo_namespace_deleted(ns);
            self.delete_namespace_by_id(ns);
        }
    }

    /// Destroy a class's subclasses and instances (C's `TclOODeleteDescendants`),
    /// leaving the class object itself intact. Shared by full class destruction
    /// and by demoting a class to a plain object (oo-13.6). Already-destroyed
    /// descendants are skipped so cyclic hierarchies don't recurse forever.
    fn oo_destroy_class_descendants(&mut self, class: &[u8]) {
        // Subclasses first: any class that lists this one as a superclass/mixin.
        let subs: Vec<Vec<u8>> = self
            .oo
            .borrow()
            .classes
            .iter()
            .filter(|(k, c)| {
                k.as_slice() != class
                    && (c.supers.iter().any(|s| s.as_slice() == class)
                        || c.mixins.iter().any(|m| m.as_slice() == class))
            })
            .map(|(k, _)| k.clone())
            .filter(|k| {
                self.oo
                    .borrow()
                    .objects
                    .get(k)
                    .is_some_and(|o| !o.destroyed)
            })
            .collect();
        for s in subs {
            self.oo_destroy_class(&s);
        }
        // Then this class's direct instances *and* any object that mixes it in
        // (C's instances list includes the objects we're mixed into).
        let insts: Vec<Vec<u8>> = self
            .oo
            .borrow()
            .objects
            .iter()
            .filter(|(k, o)| {
                k.as_slice() != class
                    && !o.destroyed
                    && (o.class == class || o.mixins.iter().any(|m| m.as_slice() == class))
            })
            .map(|(k, _)| k.clone())
            .collect();
        for o in insts {
            self.oo_destroy_bg(&o);
        }
    }

    /// The method-resolution order for `class`: a preorder walk of the class and
    /// its superclasses, each class appearing once.
    fn mro(&self, class: &[u8]) -> Vec<Vec<u8>> {
        let mut guard: Vec<Vec<u8>> = Vec::new();
        self.c3_linearize(class, &mut guard)
    }

    /// C3 linearization of `class` (the class then its superclasses, each once,
    /// in C3 order — so a diamond `D(B C)`/`B(A)`/`C(A)` yields `[D B C A]`,
    /// deferring `A` until after both `B` and `C`). `guard` breaks cycles in a
    /// malformed hierarchy (falling back to a preorder remainder).
    fn c3_linearize(&self, class: &[u8], guard: &mut Vec<Vec<u8>>) -> Vec<Vec<u8>> {
        if guard.iter().any(|c| c == class) {
            return vec![class.to_vec()];
        }
        guard.push(class.to_vec());
        let supers = self
            .oo
            .borrow()
            .classes
            .get(class)
            .map(|c| c.supers.clone())
            .unwrap_or_default();
        let result = if supers.is_empty() {
            vec![class.to_vec()]
        } else {
            let mut seqs: Vec<Vec<Vec<u8>>> =
                supers.iter().map(|s| self.c3_linearize(s, guard)).collect();
            seqs.push(supers.clone());
            let mut out = vec![class.to_vec()];
            out.extend(c3_merge(seqs));
            out
        };
        guard.pop();
        result
    }
}

/// The C3 merge: repeatedly take the head of the first sequence that does not
/// appear in the *tail* of any sequence, removing it from all. On an
/// inconsistent hierarchy (no valid head), append the remaining heads in order
/// (degraded, rather than looping).
fn c3_merge(mut seqs: Vec<Vec<Vec<u8>>>) -> Vec<Vec<u8>> {
    let mut out: Vec<Vec<u8>> = Vec::new();
    loop {
        seqs.retain(|s| !s.is_empty());
        if seqs.is_empty() {
            return out;
        }
        // A good head is the first sequence's head that is not in any tail.
        let head = seqs.iter().find_map(|s| {
            let h = &s[0];
            let in_tail = seqs
                .iter()
                .any(|t| t.len() > 1 && t[1..].iter().any(|x| x == h));
            if in_tail {
                None
            } else {
                Some(h.clone())
            }
        });
        let head = match head {
            Some(h) => h,
            // Inconsistent: break the deadlock with the first available head.
            None => seqs[0][0].clone(),
        };
        out.push(head.clone());
        for s in &mut seqs {
            if s.first() == Some(&head) {
                s.remove(0);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::counters;
    use crate::interp::{Code, Interp};

    fn leak_free(body: impl FnOnce(&mut Interp)) {
        counters::reset();
        {
            let mut interp = Interp::new();
            body(&mut interp);
        }
        assert_eq!(counters::finalize(), 0, "residual objs/bufs");
        assert_eq!(counters::double_free_count(), 0);
    }

    fn ok(i: &mut Interp, src: &[u8]) -> Vec<u8> {
        assert_eq!(
            i.eval_str(src),
            Code::Ok,
            "eval {:?} -> {:?}",
            String::from_utf8_lossy(src),
            String::from_utf8_lossy(&i.result_bytes())
        );
        i.result_bytes()
    }

    /// Whether `haystack` contains the byte subsequence `needle`.
    fn contains(haystack: &[u8], needle: &[u8]) -> bool {
        haystack.windows(needle.len()).any(|w| w == needle)
    }

    #[test]
    fn filters_wrap_method_calls() {
        leak_free(|i| {
            ok(i, b"set ::r {}");
            ok(i, b"oo::class create C");
            ok(
                i,
                b"oo::define C method foo args {global r; lappend r foo; return done}",
            );
            ok(i, b"oo::define C method log args {global r; lappend r [self method]; return [next {*}$args]}");
            ok(i, b"oo::define C filter log");
            ok(i, b"C create o");
            // The filter runs first (self method = the filter name), then `next`
            // reaches the real method.
            assert_eq!(ok(i, b"o foo"), b"done");
            assert_eq!(ok(i, b"set ::r"), b"log foo");
            i.eval_str(b"C destroy; unset -nocomplain ::r");
        });
    }

    #[test]
    fn class_methods_via_self() {
        leak_free(|i| {
            // `oo::define … self method` defines a method on the class object.
            ok(i, b"oo::class create C");
            ok(
                i,
                b"oo::define C self method greet {} {return \"hi from [self]\"}",
            );
            assert_eq!(ok(i, b"C greet"), b"hi from ::C");
            // `self method` inside the class body, and `self` alone returns it.
            assert_eq!(
                ok(
                    i,
                    b"oo::class create D {self method who {} {return [self]}}; D who"
                ),
                b"::D"
            );
            // Instance methods still resolve (the class-as-object regression guard).
            ok(i, b"oo::class create E {method m {} {return inst}}");
            assert_eq!(ok(i, b"E create e; e m"), b"inst");
            i.eval_str(b"C destroy; D destroy; E destroy");
        });
    }

    #[test]
    fn rename_to_empty_frees_object_and_class_names() {
        leak_free(|i| {
            // Deleting an object's command (rename to {}) frees the name so it
            // can be recreated — the OO registry tracks the command's lifetime.
            ok(i, b"oo::object create foo");
            ok(i, b"rename foo {}");
            assert_eq!(ok(i, b"oo::object create foo"), b"::foo");
            // Same for a class (registered in both the object and class maps).
            ok(i, b"oo::class create C");
            ok(i, b"rename C {}");
            assert_eq!(ok(i, b"oo::class create C"), b"::C");
            // A renamed object follows its command to the new name.
            ok(i, b"oo::object create a; rename a b");
            assert_eq!(ok(i, b"info object isa object b"), b"1");
            i.eval_str(b"rename foo {}; rename C {}; rename b {}");
        });
    }

    #[test]
    fn precedence_ordering_c3_and_mixins() {
        leak_free(|i| {
            // oo-21.2: diamond + mixin precedence. Expected call order
            // `Fmix Emix o D B C A` (shared bases deferred via keep-last).
            ok(
                i,
                b"oo::class create A { method m {} {lappend ::result A} }",
            );
            ok(
                i,
                b"oo::class create B { superclass A; method m {} {lappend ::result B;next} }",
            );
            ok(
                i,
                b"oo::class create C { superclass A; method m {} {lappend ::result C;next} }",
            );
            ok(
                i,
                b"oo::class create D { superclass B C; method m {} {lappend ::result D;next} }",
            );
            ok(
                i,
                b"oo::class create Emix { superclass C; method m {} {lappend ::result Emix;next} }",
            );
            ok(i, b"oo::class create Fmix { superclass Emix; method m {} {lappend ::result Fmix;next} }");
            ok(i, b"D create o");
            ok(
                i,
                b"oo::objdefine o { method m {} {lappend ::result o;next}; mixin Fmix }",
            );
            ok(i, b"set ::result {}; o m");
            assert_eq!(ok(i, b"set ::result"), b"Fmix Emix o D B C A");
            // A class mixin runs before the class itself (oo-14.8 ordering):
            // the mixin's method runs first and chains via `next` to the class.
            ok(
                i,
                b"oo::class create mix { method t {} {lappend ::r mix; next} }",
            );
            ok(
                i,
                b"oo::class create cls { mixin mix; method t {} {lappend ::r cls; return $::r} }",
            );
            ok(i, b"set ::r {}");
            assert_eq!(ok(i, b"[cls new] t"), b"mix cls");
            // A filter method defined by several providers wraps once per
            // implementation, in precedence order (oo-21.3): F.flt then G.flt.
            ok(
                i,
                b"oo::class create G { method flt {} {lappend ::r G-flt; next} }",
            );
            ok(i, b"oo::class create F { superclass G; method flt {} {lappend ::r F-flt; next}; method go {} {lappend ::r go} }");
            ok(i, b"F create gf");
            ok(i, b"oo::objdefine gf filter flt");
            ok(i, b"set ::r {}; gf go");
            assert_eq!(ok(i, b"set ::r"), b"F-flt G-flt go");
        });
    }

    #[test]
    fn tip558_configurable_configure() {
        leak_free(|i| {
            ok(i, b"oo::class create parent");
            ok(
                i,
                b"oo::configurable create Point { superclass parent; property x y; \
                  constructor args { my configure -x 0 -y 0 {*}$args } }",
            );
            ok(i, b"set pt [Point new -x 3]");
            assert_eq!(ok(i, b"$pt configure -x"), b"3");
            assert_eq!(ok(i, b"$pt configure -y"), b"0");
            ok(i, b"$pt configure -y 4");
            assert_eq!(ok(i, b"$pt configure"), b"-x 3 -y 4");
            assert_eq!(i.eval_str(b"$pt configure gorp"), Code::Error);
            assert_eq!(i.result_bytes(), b"bad property \"gorp\": must be -x or -y");
            assert_eq!(i.eval_str(b"$pt configure -x 1 -y"), Code::Error);
            // per-object property declaration (oo-2.3).
            ok(i, b"oo::objdefine $pt property z");
            ok(i, b"$pt configure -z 5");
            assert_eq!(ok(i, b"$pt configure -z"), b"5");
            // -kind controls direction.
            ok(
                i,
                b"oo::configurable create RO { superclass parent; property r -kind readable; \
                  constructor {} { my variable r; set r 7 } }",
            );
            ok(i, b"RO create ro");
            assert_eq!(ok(i, b"ro configure -r"), b"7");
            assert_eq!(i.eval_str(b"ro configure -r 9"), Code::Error);
            assert_eq!(i.result_bytes(), b"property \"-r\" is read only");
            // Error codes (ooProp-4.x).
            ok(i, b"oo::configurable create E { superclass parent }");
            assert_eq!(i.eval_str(b"oo::define E {property -x}"), Code::Error);
            assert_eq!(ok(i, b"set ::errorCode"), b"TCL OO PROPERTY_FORMAT");
            assert_eq!(
                i.eval_str(b"oo::define E {property q -kind gorp}"),
                Code::Error
            );
            assert_eq!(ok(i, b"set ::errorCode"), b"TCL LOOKUP INDEX kind gorp");
        });
    }

    #[test]
    fn tip558_property_custom_accessors_and_kinds() {
        leak_free(|i| {
            ok(i, b"oo::class create parent");
            // Custom -get/-set bodies; option/kind prefix abbreviation.
            ok(
                i,
                b"oo::configurable create Point { superclass parent; variable xyz; \
                  property x -g {return [list $xyz $xyz]} -s {set xyz $value} }",
            );
            ok(i, b"Point create p");
            ok(i, b"p configure -x 5");
            assert_eq!(ok(i, b"p configure -x"), b"5 5");
            // A getter doing break/continue is reported, not leaked as a loop error.
            ok(
                i,
                b"oo::configurable create B { superclass parent; property y -get {return -code break} }",
            );
            ok(i, b"B create b");
            assert_eq!(i.eval_str(b"b configure -y"), Code::Error);
            assert_eq!(i.result_bytes(), b"property getter for -y did a break");
            // A second declaration with a different -kind replaces membership.
            ok(
                i,
                b"oo::configurable create R { superclass parent; variable z; \
                  property z -kind readable -get {return $z}; property z -kind writable -set {set z $value} }",
            );
            ok(i, b"R create r");
            ok(i, b"r configure -z ok");
            assert_eq!(i.eval_str(b"r configure -z"), Code::Error);
            assert_eq!(i.result_bytes(), b"property \"-z\" is write only");
        });
    }

    #[test]
    fn tip558_property_storage_and_introspection() {
        leak_free(|i| {
            ok(i, b"oo::class create parent");
            ok(i, b"oo::class create c {superclass parent}");
            // Empty to start; the slot stores uniqued, info sorts (oo-1.1).
            assert_eq!(ok(i, b"info class properties c"), b"");
            ok(
                i,
                b"oo::define c ::oo::configuresupport::readableproperties -set f e d",
            );
            assert_eq!(ok(i, b"info class properties c"), b"d e f");
            assert_eq!(ok(i, b"info class properties c -writable"), b"");
            ok(
                i,
                b"oo::define c ::oo::configuresupport::readableproperties -set a a a",
            );
            assert_eq!(ok(i, b"info class properties c"), b"a");
            // Writable slot is independent.
            ok(
                i,
                b"oo::define c ::oo::configuresupport::writableproperties -set w2 w1",
            );
            assert_eq!(ok(i, b"info class properties c -writable"), b"w1 w2");
            // -all merges the superclass chain (oo-1.5).
            ok(i, b"oo::class create d {superclass c}");
            ok(
                i,
                b"oo::define d ::oo::configuresupport::readableproperties -set x y z",
            );
            assert_eq!(ok(i, b"info class properties d -all"), b"a x y z");
            // Object properties + -all merges object, mixins, class chain (oo-1.9).
            ok(i, b"d create o");
            ok(
                i,
                b"oo::objdefine o ::oo::configuresupport::objreadableproperties -set m n",
            );
            assert_eq!(ok(i, b"info object properties o"), b"m n");
            assert_eq!(ok(i, b"info object properties o -all"), b"a m n x y z");
        });
    }

    #[test]
    fn define_context_is_frame_scoped() {
        leak_free(|i| {
            // A user proc in ::oo::define is reachable as a definition command,
            // and runs with the definition context suspended (frame-scoped): its
            // `variable` is the ordinary command, and `uplevel 1` re-enters the
            // definition body's context (oo-36.9, oo-43.x).
            ok(i, b"oo::class create C");
            // A non-command in a define body routes through the global `unknown`
            // proc, whose `variable ::tcl::UnknownPending` must resolve to the
            // ordinary command (the define context is suspended in that nested
            // frame) — yielding the proper "invalid command name", not a spurious
            // declared-variable error.
            assert_eq!(i.eval_str(b"oo::define C nonesuch"), Code::Error);
            assert!(
                contains(&i.result_bytes(), b"invalid command name \"nonesuch\""),
                "got {:?}",
                i.result_bytes()
            );
            // A user proc in ::oo::define is reachable as a definition command;
            // direct `self` errors at the proc level, `uplevel 1 self` re-enters
            // the definition body's context and yields the class.
            ok(
                i,
                b"proc ::oo::define::probe {} { list [catch {self} m] [catch {uplevel 1 self} m2] $m2 }",
            );
            assert_eq!(ok(i, b"oo::define C probe"), b"1 0 ::C");
        });
    }

    #[test]
    fn namespaced_define_command_context_error() {
        leak_free(|i| {
            // Outside a definition the namespaced define subcommands error.
            assert_eq!(i.eval_str(b"oo::define::private error xyz"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"this command may only be called from within the context of an \
                  ::oo::define or ::oo::objdefine command"
            );
            assert_eq!(i.eval_str(b"oo::objdefine::method m {} {}"), Code::Error);
            // Inside a definition they dispatch normally.
            ok(i, b"oo::class create C");
            ok(i, b"oo::define C { ::oo::define::method m {} {return ok} }");
            ok(i, b"C create o");
            assert_eq!(ok(i, b"o m"), b"ok");
        });
    }

    #[test]
    fn private_variable_cross_class_mangling() {
        leak_free(|i| {
            ok(i, b"oo::class create parent");
            // clsA/clsB private x, clsC public x — three independent storages
            // (oo-38.1).
            ok(
                i,
                b"oo::class create clsA { superclass parent; private variable x; \
                  constructor {} {set x 1}; method getA {} {return $x} }",
            );
            ok(
                i,
                b"oo::class create clsB { superclass clsA; private variable x; \
                  constructor {} {set x 2; next}; method getB {} {return $x} }",
            );
            ok(
                i,
                b"oo::class create clsC { superclass clsB; variable x; \
                  constructor {} {set x 3; next}; method getC {} {return $x} }",
            );
            ok(i, b"clsC create o");
            assert_eq!(ok(i, b"o getA"), b"1");
            assert_eq!(ok(i, b"o getB"), b"2");
            assert_eq!(ok(i, b"o getC"), b"3");
            // `my varname` of a private var returns its mangled storage (oo-38.3).
            ok(
                i,
                b"oo::class create V { superclass parent; private variable p; \
                  method name {} {my varname p} }",
            );
            ok(i, b"V create v");
            let n = ok(i, b"v name");
            assert!(n.ends_with(b" : p"), "mangled storage, got {n:?}");
        });
    }

    #[test]
    fn private_variable_introspection() {
        leak_free(|i| {
            // TIP 500 (oo-38.2): `private variable` is tracked separately from
            // the public declared variables; `-private` lists it.
            ok(i, b"oo::class create parent");
            ok(
                i,
                b"oo::class create cls { superclass parent; private { variable x1; variable x2 }; variable y1 y2 }",
            );
            ok(i, b"cls create obj");
            ok(
                i,
                b"oo::objdefine obj { private variable a1 a2; variable b1 b2 }",
            );
            assert_eq!(ok(i, b"lsort [info class variables cls]"), b"y1 y2");
            assert_eq!(
                ok(i, b"lsort [info class variables cls -private]"),
                b"x1 x2"
            );
            assert_eq!(ok(i, b"lsort [info object variables obj]"), b"b1 b2");
            assert_eq!(
                ok(i, b"lsort [info object variables obj -private]"),
                b"a1 a2"
            );
        });
    }

    #[test]
    fn object_deleted_in_constructor() {
        leak_free(|i| {
            // Destroying the object in its constructor makes `new`/`create` fail
            // (oo-30.1/30.2, Bug 2903011), via `[self] destroy` or `my destroy`.
            ok(
                i,
                b"oo::class create cls { constructor {} {[self] destroy} }",
            );
            assert_eq!(i.eval_str(b"cls new"), Code::Error);
            assert_eq!(i.result_bytes(), b"object deleted in constructor");
            ok(i, b"oo::class create cls2 { constructor {} {my destroy} }");
            assert_eq!(i.eval_str(b"cls2 create foo"), Code::Error);
            assert_eq!(i.result_bytes(), b"object deleted in constructor");
        });
    }

    #[test]
    fn info_methods_all_and_object_vars_pattern() {
        leak_free(|i| {
            // `info object vars obj ?pattern?` filters by the glob (oo-16.10).
            ok(i, b"oo::class create K");
            ok(i, b"K create foo");
            ok(i, b"oo::objdefine foo export eval");
            ok(i, b"foo eval {variable c 3 a 1 b 2 ddd 4}");
            assert_eq!(ok(i, b"lsort [info object vars foo ?]"), b"a b c");
            // `info class methods -all` includes destroy; `-all -private` adds
            // the oo::object built-ins (oo-17.9).
            ok(i, b"oo::class create C { method bar {} {} }");
            assert_eq!(ok(i, b"lsort [info class methods C -all]"), b"bar destroy");
            assert_eq!(
                ok(i, b"lsort [info class methods C -all -private]"),
                b"<cloned> bar destroy eval unknown variable varname"
            );
            // Unexporting destroy hides it from `-all` (oo-17.10).
            ok(i, b"oo::define C unexport {*}[info class methods C -all]");
            assert_eq!(ok(i, b"info class methods C -all"), b"");
        });
    }

    #[test]
    fn info_methods_scope_and_export_reclassifies() {
        leak_free(|i| {
            ok(i, b"oo::class create cls { private method foo {} {} }");
            assert_eq!(ok(i, b"info class methods cls -scope public"), b"");
            assert_eq!(ok(i, b"info class methods cls -scope unexported"), b"");
            assert_eq!(ok(i, b"info class methods cls -scope private"), b"foo");
            // export reclassifies private -> public.
            ok(i, b"oo::define cls { export foo }");
            assert_eq!(ok(i, b"info class methods cls -scope public"), b"foo");
            assert_eq!(ok(i, b"info class methods cls -scope private"), b"");
            // unexport reclassifies private -> unexported.
            ok(i, b"oo::class create cls2 { private method bar {} {} }");
            ok(i, b"oo::define cls2 { unexport bar }");
            assert_eq!(ok(i, b"info class methods cls2 -scope unexported"), b"bar");
            assert_eq!(ok(i, b"info class methods cls2 -scope private"), b"");
        });
    }

    #[test]
    fn destructor_error_goes_to_bgerror() {
        leak_free(|i| {
            // A destructor erroring during implicit teardown (`rename {}`) is
            // queued as a background error and reported to the `interp bgerror`
            // handler at `update`, not propagated (oo-3.7/3.8).
            ok(
                i,
                b"interp bgerror {} [list apply {{var msg args} {upvar #0 $var v; lappend v $msg}} ::caught]",
            );
            ok(i, b"oo::class create cls { destructor {error boom} }");
            ok(i, b"set ::caught {}; cls create obj");
            ok(i, b"rename obj {}");
            ok(i, b"update idletasks");
            assert_eq!(ok(i, b"set ::caught"), b"boom");
            // An empty cmdPrefix is rejected (C's `ChildBgerror`: length >= 1),
            // so the handler is reset by overwriting it with a one-element list.
            ok(i, b"interp bgerror {} noop");
        });
    }

    #[test]
    fn oo_copy_clones_namespace_contents() {
        leak_free(|i| {
            // oo::copy copies the instance namespace's procs (re-pointed to the
            // copy's namespace) and variable values (oo-15.6/15.8).
            ok(i, b"oo::class create C { export eval }");
            ok(i, b"C create a");
            ok(
                i,
                b"a eval {variable y 0; proc foo {} {variable y; incr y}}",
            );
            assert_eq!(ok(i, b"a eval foo"), b"1"); // a.y = 1
            ok(i, b"oo::copy a b");
            // b inherits y=1 and its own foo (operating on b's y).
            assert_eq!(ok(i, b"b eval foo"), b"2"); // b.y 1->2
            assert_eq!(ok(i, b"b eval foo"), b"3"); // b.y 2->3
                                                    // a's y is untouched by b's foo.
            assert_eq!(ok(i, b"a eval {set y}"), b"1");
            assert_eq!(ok(i, b"a eval foo"), b"2"); // a.y 1->2
        });
    }

    #[test]
    fn my_command_invoked_directly() {
        leak_free(|i| {
            // The per-object `my`, invoked directly (not from a method), still
            // dispatches on its own object (oo-16.13).
            ok(i, b"oo::object create foo");
            ok(i, b"oo::objdefine foo method Bar {} {return {ok in foo}}");
            assert_eq!(ok(i, b"[info object namespace foo]::my Bar"), b"ok in foo");
        });
    }

    #[test]
    fn isa_follows_import_and_unknown_list_excludes_bare_export() {
        leak_free(|i| {
            // info object isa follows a namespace-import alias to the object.
            ok(
                i,
                b"namespace eval foo { namespace eval bar { oo::object create o; namespace export o }; namespace import bar::o }",
            );
            assert_eq!(ok(i, b"info object isa object foo::o"), b"1");
            assert_eq!(ok(i, b"info object isa object foo::bar::o"), b"1");
            // An `export`ed name with no implementation is not in the unknown list.
            ok(i, b"oo::class create tc");
            ok(i, b"oo::objdefine tc export Bad");
            assert_eq!(i.eval_str(b"tc Bad"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"unknown method \"Bad\": must be create, destroy or new"
            );
        });
    }

    #[test]
    fn nextto_resumes_chain_at_class() {
        leak_free(|i| {
            ok(i, b"oo::class create root");
            ok(
                i,
                b"oo::class create A { superclass root; method x args {lappend ::r ==A==} }",
            );
            ok(
                i,
                b"oo::class create B { superclass A; method x args {lappend ::r ==B==; nextto A} }",
            );
            ok(
                i,
                b"oo::class create C { superclass A; method x args {lappend ::r ==C==} }",
            );
            ok(
                i,
                b"oo::class create D { superclass B C; method x args {lappend ::r ==D==; next} }",
            );
            ok(i, b"set ::r {}; [D new] x");
            // D -> B (next) -> B does `nextto A`, skipping C, to A.
            assert_eq!(ok(i, b"set ::r"), b"==D== ==B== ==A==");
            // nextto to an unrelated/behind class errors.
            ok(i, b"oo::class create Z { superclass root }");
            ok(
                i,
                b"oo::class create E { superclass root; method x {} {nextto Z} }",
            );
            ok(i, b"E create e");
            assert_eq!(i.eval_str(b"e x"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"method has no non-filter implementation by \"Z\""
            );
        });
    }

    #[test]
    fn info_call_chain_rendering() {
        leak_free(|i| {
            ok(i, b"oo::class create root");
            // declarer is `object` for a per-object method, the class FQN else.
            ok(
                i,
                b"oo::class create ::A { superclass root; method x {} {} }",
            );
            ok(i, b"A create y");
            ok(i, b"oo::objdefine y method x {} {}");
            assert_eq!(
                ok(i, b"info object call y x"),
                b"{method x object method} {method x ::A method}"
            );
            // info class call follows the full precedence, mixins first.
            ok(i, b"oo::class create ::B { superclass A; method x {} {} }");
            ok(i, b"oo::class create ::C { superclass A; method x {} {} }");
            ok(
                i,
                b"oo::class create ::D { superclass C; mixin B; method x {} {} }",
            );
            assert_eq!(
                ok(i, b"info class call D x"),
                b"{method x ::B method} {method x ::D method} {method x ::C method} {method x ::A method}"
            );
            // A missing method renders the unknown chain, ending at the core
            // oo::object handler with a `core method:` type.
            ok(
                i,
                b"oo::class create ::U { superclass root; method unknown args {} }",
            );
            ok(i, b"U create u");
            assert_eq!(
                ok(i, b"info object call u nosuch"),
                b"{unknown unknown ::U method} {unknown unknown ::oo::object {core method: \"unknown\"}}"
            );
        });
    }

    #[test]
    fn mixin_of_mixin_methods_are_reachable() {
        leak_free(|i| {
            // Bug 1960703 (oo-14.6): a method from a mixin's own mixin is
            // reachable via the call chain.
            ok(i, b"oo::class create parent");
            ok(
                i,
                b"oo::class create A { superclass parent; method egg {} {return chicken} }",
            );
            ok(
                i,
                b"oo::class create B { superclass parent; mixin A; method bar {} {my egg} }",
            );
            ok(
                i,
                b"oo::class create C { superclass parent; mixin B; method foo {} {my bar} }",
            );
            assert_eq!(ok(i, b"[C new] foo"), b"chicken");
        });
    }

    #[test]
    fn my_variable_rejects_array_element() {
        leak_free(|i| {
            ok(
                i,
                b"oo::class create C { method bar {} { my variable a(b) } }",
            );
            ok(i, b"C create o");
            assert_eq!(i.eval_str(b"o bar"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"can't define \"a(b)\": name refers to an element in an array"
            );
        });
    }

    #[test]
    fn variable_slot_operations() {
        leak_free(|i| {
            // `variable` accumulates by default (-append), uniquely.
            ok(i, b"oo::class create A { variable a; variable b }");
            assert_eq!(ok(i, b"lsort [info class variables A]"), b"a b");
            // `-clear` empties; a following declaration starts fresh (oo-27.16).
            ok(
                i,
                b"oo::class create B { variable x; variable -clear; variable y }",
            );
            assert_eq!(ok(i, b"info class variables B"), b"y");
            // `-set` replaces (oo-27.17).
            ok(i, b"oo::class create C { variable x; variable -set y }");
            assert_eq!(ok(i, b"info class variables C"), b"y");
            // An unknown slot op errors with the C unknown-method message
            // (oo-27.18).
            assert_eq!(
                i.eval_str(b"oo::class create D { variable -? y }"),
                Code::Error
            );
            assert_eq!(
                i.result_bytes(),
                b"unknown method \"-?\": must be -append, -appendifnew, -clear, -prepend, -remove or -set"
            );
        });
    }

    #[test]
    fn define_context_self_arity() {
        leak_free(|i| {
            ok(i, b"oo::class create Cls");
            ok(i, b"Cls create obj");
            // `oo::define`'s `self` accepts class-side directives; `self` alone
            // returns the class.
            assert_eq!(ok(i, b"oo::define Cls {set ::r [self]}; set ::r"), b"::Cls");
            ok(
                i,
                b"oo::define Cls { self { method cm {} {return classside} } }",
            );
            assert_eq!(ok(i, b"Cls cm"), b"classside");
            // `oo::objdefine`'s `self` takes no arguments (oo-36.8).
            assert_eq!(ok(i, b"oo::objdefine obj {self}"), b"::obj");
            assert_eq!(
                i.eval_str(b"oo::objdefine obj {self anything}"),
                Code::Error
            );
            assert_eq!(i.result_bytes(), b"wrong # args: should be \"self\"");
        });
    }

    #[test]
    fn isa_typeof_traverses_mixins_and_subclass_pattern() {
        leak_free(|i| {
            // `isa typeof` follows mixin links, not just the class MRO (oo-16.9).
            ok(i, b"oo::class create Ac");
            ok(i, b"oo::class create Bc { superclass Ac }");
            ok(i, b"oo::class create Cc { superclass Bc }");
            ok(i, b"oo::class create Dc { mixin Cc }");
            ok(i, b"Dc create F");
            assert_eq!(ok(i, b"info object isa typeof F Bc"), b"1");
            assert_eq!(ok(i, b"info object isa typeof F Cc"), b"1");
            assert_eq!(ok(i, b"info object isa typeof F oo::class"), b"0");
            // `info class subclasses class ?pattern?` filters by the FQN glob.
            assert_eq!(ok(i, b"lsort [info class subclasses Ac]"), b"::Bc");
            assert_eq!(ok(i, b"info class subclasses Ac ::C*"), b"");
        });
    }

    #[test]
    fn class_method_constructor_instance_var() {
        leak_free(|i| {
            ok(
                i,
                b"oo::class create Animal {
                    variable sound
                    constructor {s} { set sound $s }
                    method speak {} { return \"I say $sound\" }
                    method describe {} { return \"[self] speaks\" }
                }",
            );
            ok(i, b"set a [Animal new woof]");
            assert_eq!(ok(i, b"$a speak"), b"I say woof");
            ok(i, b"Animal create ::bessie moo");
            assert_eq!(ok(i, b"bessie speak"), b"I say moo");
            ok(i, b"bessie destroy");
            assert_eq!(i.eval_str(b"bessie speak"), Code::Error);
        });
    }

    // Needs the numeric tower: `$obj eval` runs `expr`.
    #[cfg(have_tommath)]
    #[test]
    fn object_builtin_variable_varname_eval() {
        leak_free(|i| {
            ok(
                i,
                b"oo::class create C {
                    method s {v} { my variable x; set x $v }
                    method g {} { my variable x; return $x }
                    method vn {} { my varname x }
                    method e {} { my eval {set y 9; return $y} }
                }",
            );
            ok(i, b"set o [C new]");
            ok(i, b"$o s hi");
            assert_eq!(ok(i, b"$o g"), b"hi");
            assert_eq!(ok(i, b"$o vn"), b"::oo::Obj0::x");
            assert_eq!(ok(i, b"$o e"), b"9");
            // Built-ins are unexported: an external call is an unknown method.
            assert_eq!(i.eval_str(b"$o variable x"), Code::Error);
            // ... but `export` promotes a built-in to a public method.
            ok(i, b"oo::class create E { export eval }; set e [E new]");
            assert_eq!(ok(i, b"$e eval {expr 3+4}"), b"7");
            // `variable` rejects a namespace-qualified name.
            assert_eq!(
                i.eval_str(b"oo::class create D {method z {} {my variable a::b}}; [D new] z"),
                Code::Error,
            );
        });
    }

    #[test]
    fn classvariable_shares_state_per_declaring_class() {
        leak_free(|i| {
            // A class variable is shared across all instances of the class, and
            // lives in the class object's namespace.
            ok(
                i,
                b"oo::class create Counter {
                    constructor {} { classvariable count; incr count }
                    method count {} { classvariable count; return $count }
                }",
            );
            ok(i, b"Counter create a; Counter create b; Counter create c");
            assert_eq!(ok(i, b"a count"), b"3");
            assert_eq!(ok(i, b"b count"), b"3");
            // It really lives in the class's own namespace.
            assert_eq!(ok(i, b"set [info object namespace Counter]::count"), b"3");
            // A method reached via `next` links to *its own* declaring class, not
            // the leaf object's — so Base and Derived keep separate stores.
            ok(
                i,
                b"oo::class create Base {
                    method tag {v} { classvariable t; set t $v }
                    method get {} { classvariable t; return $t }
                }",
            );
            ok(
                i,
                b"oo::class create Sub {
                    superclass Base
                    method tag {v} { classvariable t; set t sub-$v; next $v }
                }",
            );
            ok(i, b"Sub create s; s tag hi");
            assert_eq!(ok(i, b"set [info object namespace Base]::t"), b"hi");
            assert_eq!(ok(i, b"set [info object namespace Sub]::t"), b"sub-hi");
            assert_eq!(ok(i, b"s get"), b"hi");
        });
    }

    #[test]
    fn classvariable_context_and_name_errors() {
        leak_free(|i| {
            // Outside a method it is unusable (helpers are call-stack-gated).
            assert_eq!(i.eval_str(b"classvariable foo"), Code::Error);
            assert!(contains(
                &i.result_bytes(),
                b"classvariable may only be called from inside a method"
            ));
            // A per-object / class-side method has no class namespace to share.
            ok(
                i,
                b"oo::object create solo; oo::objdefine solo {method m {} {classvariable x}}",
            );
            assert_eq!(i.eval_str(b"solo m"), Code::Error);
            assert_eq!(i.result_bytes(), b"method not defined by a class");
            // Name validation mirrors C: array look-alike, then namespace sep.
            ok(
                i,
                b"oo::class create C {
                    method arr {} { classvariable a(b) }
                    method ns {} { classvariable ::foo }
                    method none {} { classvariable }
                }",
            );
            ok(i, b"C create o");
            assert_eq!(i.eval_str(b"o arr"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"bad variable name \"a(b)\": can't create a scalar variable that looks like an array element"
            );
            assert_eq!(i.eval_str(b"o ns"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"bad variable name \"::foo\": can't create a local variable with a namespace separator in it"
            );
            assert_eq!(i.eval_str(b"o none"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"wrong # args: should be \"classvariable name ...\""
            );
        });
    }

    #[test]
    fn object_creationid_and_definitionnamespace() {
        leak_free(|i| {
            ok(i, b"oo::class create cls");
            // Distinct objects get distinct creation IDs.
            let a = ok(i, b"info object creationid [cls new]");
            let b = ok(i, b"info object creationid [cls new]");
            assert_ne!(a, b);
            // The ID is stable across rename.
            ok(
                i,
                b"set o [cls new]; set id [info object creationid $o]; rename $o gorp",
            );
            assert_eq!(ok(i, b"info object creationid gorp"), ok(i, b"set id"));
            // TIP 524 definition-namespace introspection (built-in defaults).
            assert_eq!(
                ok(i, b"info class definitionnamespace oo::object -instance"),
                b"::oo::objdefine"
            );
            assert_eq!(ok(i, b"info class definitionnamespace oo::object"), b"");
            assert_eq!(
                ok(i, b"info class definitionnamespace oo::class -class"),
                b"::oo::define"
            );
            assert_eq!(
                ok(i, b"info class definitionnamespace oo::class -instance"),
                b""
            );
            // A non-object reports an error (without surrounding quotes).
            assert_eq!(i.eval_str(b"info object creationid nosuch"), Code::Error);
        });
    }

    #[test]
    fn define_subcommand_abbrev_and_extras() {
        leak_free(|i| {
            // Abbreviated definition subcommands (unique prefixes).
            ok(i, b"oo::class create A");
            ok(i, b"oo::class create C { meth foo {} {return X} }");
            assert_eq!(ok(i, b"[C new] foo"), b"X");
            ok(i, b"oo::define C { super A }");
            assert_eq!(ok(i, b"info class superclasses C"), b"::A");
            // An ambiguous prefix is not a define subcommand.
            assert_eq!(i.eval_str(b"oo::define C { m foo {} {} }"), Code::Error);
            // deletemethod / renamemethod.
            ok(
                i,
                b"oo::class create D { method foo {} {return F}; method bar {} {return B} }",
            );
            ok(i, b"oo::define D deletemethod foo");
            assert_eq!(i.eval_str(b"[D new] foo"), Code::Error);
            ok(i, b"oo::define D renamemethod bar baz");
            assert_eq!(ok(i, b"[D new] baz"), b"B");
            assert_eq!(i.eval_str(b"oo::define D deletemethod nope"), Code::Error);
            // objdefine class — change an object's class.
            ok(i, b"oo::class create P { method m {} {return P} }");
            ok(i, b"oo::class create Q { method m {} {return Q} }");
            ok(i, b"set o [P new]");
            ok(i, b"oo::objdefine $o class Q");
            assert_eq!(ok(i, b"$o m"), b"Q");
        });
    }

    #[test]
    fn info_class_introspection_and_abbrev() {
        leak_free(|i| {
            ok(
                i,
                b"oo::class create C { method foo {a b} {return x}; forward fwd ::tcl::mathop::+ 1; filter foo }",
            );
            assert_eq!(ok(i, b"info class methodtype C foo"), b"method");
            assert_eq!(ok(i, b"info class methodtype C fwd"), b"forward");
            assert_eq!(ok(i, b"info class definition C foo"), b"{a b} {return x}");
            assert_eq!(ok(i, b"info class forward C fwd"), b"::tcl::mathop::+ 1");
            assert_eq!(ok(i, b"info class filters C"), b"foo");
            // Abbreviated subcommand (unique prefix).
            assert_eq!(ok(i, b"info class methodt C foo"), b"method");
            // Ambiguous / unknown subcommand errors.
            assert_eq!(i.eval_str(b"info class gorp C"), Code::Error);
            assert_eq!(i.eval_str(b"info class methodtype C nope"), Code::Error);
            // `forward` on a non-forward method.
            assert_eq!(i.eval_str(b"info class forward C foo"), Code::Error);
        });
    }

    #[test]
    fn info_call_and_self_call() {
        leak_free(|i| {
            ok(i, b"oo::class create A { method foo {} {} }");
            ok(
                i,
                b"oo::class create B { superclass A; method foo {} { next } }",
            );
            assert_eq!(
                ok(i, b"info class call B foo"),
                b"{method foo ::B method} {method foo ::A method}"
            );
            // `self call` reports the live chain + index; a private method shows
            // as `private` and is visible to a same-object `[self]` dispatch.
            ok(
                i,
                b"oo::class create P { method chain {} { return [self call] } }",
            );
            ok(
                i,
                b"oo::class create Q { superclass P; private method chain {} { next }; method viapub {} { [self] chain } }",
            );
            ok(i, b"set q [Q new]");
            assert_eq!(
                ok(i, b"$q viapub"),
                b"{{private chain ::Q method} {method chain ::P method}} 1"
            );
            // An external call from outside the object skips Q's private `chain`
            // and runs P's public one.
            assert_eq!(ok(i, b"$q chain"), b"{{method chain ::P method}} 0");
        });
    }

    #[test]
    fn metaclass_instantiation() {
        leak_free(|i| {
            // A metaclass's instances are themselves classes.
            ok(i, b"oo::class create meta { superclass oo::class }");
            ok(i, b"meta create instance1");
            ok(i, b"instance1 create instance2");
            assert_eq!(ok(i, b"info object class instance1"), b"::meta");
            assert_eq!(ok(i, b"info object class instance2"), b"::instance1");
            assert_eq!(ok(i, b"info object isa class instance1"), b"1");
            assert_eq!(ok(i, b"info object isa metaclass meta"), b"1");
            assert_eq!(ok(i, b"info object isa metaclass instance1"), b"0");
            assert_eq!(ok(i, b"info object isa object instance2"), b"1");
            // A metaclass instance can carry a definition-script body.
            ok(i, b"set c [meta create C { method foo {} { return MF } }]");
            assert_eq!(ok(i, b"[C new] foo"), b"MF");
            // Destroying the metaclass cascades to its class-instances (both maps
            // freed), so the names can be recreated.
            ok(i, b"meta destroy");
            assert_eq!(i.eval_str(b"instance1 create x"), Code::Error);
            ok(i, b"oo::class create instance1");
        });
    }

    #[test]
    fn oo_slot_base_class() {
        leak_free(|i| {
            ok(
                i,
                b"oo::class create SampleSlot {\n\
                    superclass oo::Slot\n\
                    constructor {} { variable contents {a b c} }\n\
                    method contents {} { variable contents; return $contents }\n\
                    method Get {} { variable contents; return $contents }\n\
                    method Set {lst} { variable contents $lst; return }\n\
                    method Resolve {x} { return $x }\n\
                }",
            );
            ok(i, b"SampleSlot create s");
            ok(i, b"s -append g h");
            assert_eq!(ok(i, b"s contents"), b"a b c g h");
            ok(i, b"s -set d e");
            assert_eq!(ok(i, b"s contents"), b"d e");
            ok(i, b"s -prepend x");
            assert_eq!(ok(i, b"s contents"), b"x d e");
            ok(i, b"s -remove d");
            assert_eq!(ok(i, b"s contents"), b"x e");
            ok(i, b"s -clear");
            assert_eq!(ok(i, b"s contents"), b"");
            // Defaulting: a bare value goes through `unknown` → -append.
            ok(i, b"s p q");
            assert_eq!(ok(i, b"s contents"), b"p q");
        });
    }

    #[test]
    fn definitionnamespace_semantics() {
        leak_free(|i| {
            // TIP 524: a metaclass's -class definition namespace becomes the
            // resolution scope when defining its instances (classes).
            ok(i, b"oo::class create parent");
            ok(
                i,
                b"namespace eval foodef { proc sparkle {} { return ok } }",
            );
            ok(
                i,
                b"oo::class create foocls { superclass oo::class parent; definitionnamespace foodef }",
            );
            ok(
                i,
                b"oo::class create foo { superclass parent; self class foocls }",
            );
            // `sparkle` (a proc in foodef) is now reachable in foo's definition.
            assert_eq!(ok(i, b"oo::define foo { sparkle }"), b"ok");
            // Round-trip introspection of an explicitly-set namespace.
            ok(i, b"namespace eval ::nd {}");
            ok(
                i,
                b"oo::class create D; oo::define D { definitionnamespace ::nd }",
            );
            assert_eq!(ok(i, b"info class definitionnamespace D"), b"::nd");
            assert_eq!(ok(i, b"info class definitionnamespace D -instance"), b"");
            // The root classes reject definition-namespace changes.
            assert_eq!(
                i.eval_str(b"oo::define oo::object { definitionnamespace ::nd }"),
                Code::Error,
            );
        });
    }

    #[test]
    fn destructors_fire_on_rename_destroy_and_namespace_delete() {
        leak_free(|i| {
            ok(
                i,
                b"oo::class create cls { destructor { lappend ::result died } }",
            );
            // `obj destroy`
            ok(i, b"set ::result {}; cls create a; a destroy");
            assert_eq!(ok(i, b"set ::result"), b"died");
            // `rename obj {}`
            ok(i, b"set ::result {}; cls create b; rename b {}");
            assert_eq!(ok(i, b"set ::result"), b"died");
            // `namespace delete <object-namespace>`
            ok(
                i,
                b"set ::result {}; cls create c; namespace delete [info object namespace c]",
            );
            assert_eq!(ok(i, b"set ::result"), b"died");
            // Re-entrancy guard: a destructor that re-destroys the object runs once.
            ok(
                i,
                b"oo::class create g { destructor { lappend ::result x; catch {[self] destroy} } }",
            );
            ok(i, b"set ::result {}; g create o; o destroy");
            assert_eq!(ok(i, b"set ::result"), b"x");
            // A2: the dup-name error uses the as-written name and `object`.
            ok(i, b"oo::object create foo");
            assert_eq!(i.eval_str(b"oo::object create foo"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"can't create object \"foo\": command already exists with that name"
            );
            ok(i, b"oo::class create K");
            assert_eq!(i.eval_str(b"oo::class create K"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"can't create object \"K\": command already exists with that name"
            );
        });
    }

    // Needs the numeric tower: method bodies compute via `expr`.
    #[cfg(have_tommath)]
    #[test]
    fn private_method_scope_and_unknown_listing() {
        leak_free(|i| {
            ok(i, b"oo::class create parent");
            // A class's private method is invisible to a different scope's `my`
            // call: the object's private `step` does not shadow the class's
            // public `step` when the caller is a class method (oo-39.4/39.5).
            ok(
                i,
                b"oo::class create clsA { superclass parent; variable x; \
                  constructor {} {set x 1}; method act {} {my step; return}; \
                  method step {} {incr x}; method x {} {return $x} }",
            );
            ok(i, b"clsA create obj; obj act");
            assert_eq!(ok(i, b"obj x"), b"2");
            ok(
                i,
                b"oo::objdefine obj { variable x; private { method step {} {incr x 2} } }",
            );
            ok(i, b"obj act"); // class act -> class step (public), not obj private
            assert_eq!(ok(i, b"obj x"), b"3");
            // The unknown-method error lists in-scope private methods (oo-39.6).
            ok(
                i,
                b"oo::class create cls { superclass parent; variable x; \
                  constructor {val} {set x $val}; private method x {} {return $x}; \
                  method equal {other} {expr {$x == [$other y]}} }",
            );
            ok(i, b"cls create a 1; cls create b 2");
            assert_eq!(i.eval_str(b"a equal b"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"unknown method \"y\": must be destroy, equal or x"
            );
            // An internal (`my`) miss also lists the unexported oo::object
            // built-ins and unexported methods (oo-39.8).
            ok(i, b"oo::class create P { method m {} { my nonesuch } }");
            ok(i, b"P create p");
            assert_eq!(i.eval_str(b"p m"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"unknown method \"nonesuch\": must be <cloned>, destroy, eval, m, unknown, variable or varname"
            );
        });
    }

    #[test]
    fn method_body_errorinfo_frames() {
        leak_free(|i| {
            ok(i, b"oo::class create cls { constructor {} {} }");
            ok(i, b"cls create obj");
            // A per-object method: `(object "::obj" method "bar" line N)`.
            ok(i, b"oo::objdefine obj method bar {} {error foo}");
            assert_eq!(i.eval_str(b"obj bar"), Code::Error);
            let ei = ok(i, b"set errorInfo");
            assert!(
                contains(&ei, b"(object \"::obj\" method \"bar\" line 1)"),
                "got {ei:?}"
            );
            // A class method: `(class "::cls" method "cm" line N)`.
            ok(i, b"oo::define cls method cm {} {error boom}");
            assert_eq!(i.eval_str(b"obj cm"), Code::Error);
            let ei = ok(i, b"set errorInfo");
            assert!(
                contains(&ei, b"(class \"::cls\" method \"cm\" line 1)"),
                "got {ei:?}"
            );
            // `my eval {…}` adds `(in "my eval" script line N)`.
            ok(i, b"oo::objdefine obj method ev {} {my eval {error e}}");
            assert_eq!(i.eval_str(b"obj ev"), Code::Error);
            let ei = ok(i, b"set errorInfo");
            assert!(
                contains(&ei, b"(in \"my eval\" script line 1)"),
                "got {ei:?}"
            );
        });
    }

    #[test]
    fn define_script_errorinfo_and_super_mixin_messages() {
        leak_free(|i| {
            // superclass/mixin: not-an-object vs not-a-class messages (as-written
            // name; matches C's Tcl_GetObjectFromObj + ClassSuperSet/MixinSet).
            ok(i, b"oo::object create anobj");
            assert_eq!(
                i.eval_str(b"oo::class create c1 { superclass nosuch }"),
                Code::Error
            );
            assert_eq!(i.result_bytes(), b"nosuch does not refer to an object");
            assert_eq!(
                i.eval_str(b"oo::class create c2 { superclass anobj }"),
                Code::Error
            );
            assert_eq!(i.result_bytes(), b"only a class can be a superclass");
            assert_eq!(
                i.eval_str(b"oo::class create c3 { mixin nosuch }"),
                Code::Error
            );
            assert_eq!(i.result_bytes(), b"nosuch does not refer to an object");
            assert_eq!(
                i.eval_str(b"oo::class create c4 { mixin anobj }"),
                Code::Error
            );
            assert_eq!(i.result_bytes(), b"may only mix in classes");
            // The define-script errorInfo frame uses the *current* (renamed) name.
            ok(i, b"oo::class create base");
            assert_eq!(
                i.eval_str(b"oo::class create abc { superclass base; rename abc def; error foo }"),
                Code::Error
            );
            let ei = ok(i, b"set errorInfo");
            assert!(
                ei.windows(b"class \"::def\"".len())
                    .any(|w| w == b"class \"::def\""),
                "errorInfo should name the renamed class ::def, got {ei:?}"
            );
        });
    }

    #[test]
    fn define_single_command_wrong_args_names_whole_command() {
        leak_free(|i| {
            ok(i, b"oo::class create Foo");
            // Single-command form: the message names the whole original command.
            assert_eq!(
                i.eval_str(b"oo::define Foo method missingArgs"),
                Code::Error
            );
            assert_eq!(
                i.result_bytes(),
                b"wrong # args: should be \"oo::define Foo method name ?option? args body\""
            );
            assert_eq!(
                i.eval_str(b"oo::objdefine Foo method missingArgs"),
                Code::Error
            );
            assert_eq!(
                i.result_bytes(),
                b"wrong # args: should be \"oo::objdefine Foo method name ?option? args body\""
            );
            // Body form keeps the bare usage (no rewrite prefix).
            assert_eq!(
                i.eval_str(b"oo::define Foo { method missingArgs }"),
                Code::Error
            );
            assert_eq!(
                i.result_bytes(),
                b"wrong # args: should be \"method name ?option? args body\""
            );
        });
    }

    #[test]
    fn destructor_error_propagation_and_renamed_my_teardown() {
        leak_free(|i| {
            // Explicit `obj destroy` propagates a destructor error to its caller
            // (oo-3.6); the object command is still removed.
            ok(i, b"oo::class create cls { destructor { error foo } }");
            assert_eq!(
                ok(
                    i,
                    b"list [catch {[cls create obj] destroy} m] $m [info commands obj]"
                ),
                b"1 foo {}"
            );
            // Implicit teardown (rename {}) swallows the destructor error.
            ok(i, b"cls create obj2");
            ok(i, b"rename obj2 {}");
            // A `my` renamed out of the instance namespace is deleted when the
            // object is destroyed (oo-3.4): tie its lifetime to the object.
            ok(i, b"oo::class create plain { constructor {} {} }");
            ok(i, b"plain create p");
            ok(i, b"rename [info object namespace p]::my ::pmy");
            assert_eq!(ok(i, b"info commands ::pmy"), b"::pmy");
            ok(i, b"p destroy");
            assert_eq!(ok(i, b"info commands ::pmy"), b"");
        });
    }

    #[test]
    fn create_with_namespace_and_copy_targetns() {
        leak_free(|i| {
            // `createWithNamespace` is unexported by default; once exported it
            // creates the object in a fresh (must-not-exist) namespace.
            ok(i, b"oo::class create AC");
            assert_eq!(
                i.eval_str(b"AC createWithNamespace inst ::ns1"),
                Code::Error
            );
            ok(i, b"oo::objdefine AC export createWithNamespace");
            ok(i, b"AC createWithNamespace inst ::ns1");
            assert_eq!(ok(i, b"info object namespace inst"), b"::ns1");
            // An already-existing namespace is rejected.
            ok(i, b"namespace eval ::taken {}");
            assert_eq!(
                i.eval_str(b"AC createWithNamespace inst2 ::taken"),
                Code::Error
            );
            // `oo::copy src tgt targetNamespace` places the copy's vars there.
            ok(i, b"oo::class create C { method m {} {} }");
            ok(i, b"C create src");
            ok(i, b"oo::copy src dst ::dstns");
            assert_eq!(ok(i, b"info object namespace dst"), b"::dstns");
        });
    }

    #[test]
    fn oo_copy_clones_classes() {
        leak_free(|i| {
            // `oo::copy` of a class clones the class facet, so the copy is a
            // working class whose instances run the cloned methods.
            ok(
                i,
                b"oo::class create foo { method testme {} { return [self class] } }",
            );
            ok(i, b"oo::copy foo bar");
            assert_eq!(ok(i, b"[bar new] testme"), b"::bar");
            // Cloned superclasses + declared variables come across too.
            ok(i, b"oo::class create AC");
            ok(i, b"oo::class create Foo { superclass AC; variable a b c }");
            ok(i, b"oo::copy Foo Bar");
            assert_eq!(ok(i, b"info class variable Bar"), b"a b c");
            assert_eq!(ok(i, b"info class superclasses Bar"), b"::AC");
        });
    }

    #[test]
    fn inheritance_and_next() {
        leak_free(|i| {
            ok(i, b"oo::class create Animal { variable s; constructor {x} {set s $x}; method speak {} {return \"I say $s\"} }");
            ok(i, b"oo::class create Dog { superclass Animal; constructor {} {next bark}; method speak {} {return \"Dog: [next]\"} }");
            ok(i, b"set d [Dog new]");
            assert_eq!(ok(i, b"$d speak"), b"Dog: I say bark");
            ok(i, b"oo::define Dog method legs {} { return 4 }");
            assert_eq!(ok(i, b"$d legs"), b"4");
        });
    }

    // Needs the numeric tower: `forward` targets `::tcl::mathop::+`.
    #[cfg(have_tommath)]
    #[test]
    fn objdefine_forward_mixin_unexport_copy() {
        leak_free(|i| {
            ok(i, b"oo::class create Base { method m {} {return base}; method hide {} {return H}; unexport hide }");
            ok(i, b"set o [Base new]");
            // unexported method: external call fails, `my` works.
            assert_eq!(i.eval_str(b"$o hide"), Code::Error);
            // per-object method via objdefine
            ok(i, b"oo::objdefine $o { method extra {} {return per-obj} }");
            assert_eq!(ok(i, b"$o extra"), b"per-obj");
            // forward
            ok(i, b"oo::class create F { forward add ::tcl::mathop::+ 10 }");
            assert_eq!(ok(i, b"[F new] add 5"), b"15");
            // mixin
            ok(
                i,
                b"oo::class create Mix { method mixed {} {return MIXED} }",
            );
            ok(i, b"oo::objdefine $o { mixin Mix }");
            assert_eq!(ok(i, b"$o mixed"), b"MIXED");
            // info introspection
            assert_eq!(ok(i, b"info object class $o"), b"::Base");
            assert_eq!(ok(i, b"info object isa object $o"), b"1");
            assert_eq!(ok(i, b"info class instances Base"), b"::oo::Obj0");
        });
    }

    #[test]
    fn oo_package_is_provided() {
        leak_free(|i| {
            assert_eq!(ok(i, b"package require tcl::oo"), b"1.3.1");
        });
    }

    #[test]
    fn info_frame_method_body_lines_are_relative() {
        leak_free(|i| {
            // A method body defined while sourcing reports file-absolute lines,
            // so a body command's `info frame` line tracks the source — even
            // when the body opens on a later line than the `method` command.
            let script = b"oo::class create C {}\n\
                oo::define C {\n\
                  method a {} {info frame 0}\n\
                  method b {\n\
                  } {info frame 0}\n\
                }\n\
                oo::define C method c {} {info frame 0}\n\
                C create o\n\
                list [dict get [o a] line] [dict get [o b] line] \
                     [dict get [o c] line] [dict get [o a] type] \
                     [dict get [o a] file]";
            let r = i.eval_sourced(script, b"/tmp/x.tcl");
            assert_eq!(r, Code::Ok);
            // `method a` body line 3; `method b` body line 5; `method c` line 7.
            assert_eq!(i.result_bytes(), b"3 5 7 source /tmp/x.tcl");
        });
    }

    #[test]
    fn info_frame_reports_method_context() {
        leak_free(|i| {
            // `info frame 0` in a class-defined method reports method + class;
            // an object-defined method reports method + object (not `proc`).
            ok(i, b"oo::class create c");
            ok(i, b"c create i");
            ok(i, b"oo::define c method m {} { info frame 0 }");
            ok(i, b"oo::objdefine i method n {} { info frame 0 }");
            let cm = ok(i, b"c create o; o m");
            assert!(cm.windows(13).any(|w| w == b"method m clas"));
            assert!(cm.ends_with(b"class ::c level 0"));
            let inf = ok(i, b"i n");
            assert!(inf.ends_with(b"object ::i level 0"));
            // A plain proc still reports `proc`, not method/class.
            ok(i, b"proc p {} { info frame 0 }");
            let pf = ok(i, b"p");
            assert!(pf.windows(7).any(|w| w == b"proc ::"));
            assert!(!pf.windows(6).any(|w| w == b"method"));
        });
    }

    #[test]
    fn class_destroy_cascades_to_mixin_users() {
        leak_free(|i| {
            // Destroying a class destroys objects that mix it in; an object
            // whose mixin was cleared survives.
            ok(i, b"oo::class create A");
            ok(i, b"oo::class create B");
            ok(i, b"oo::objdefine [A create keep] mixin B");
            ok(i, b"oo::objdefine [A create gone] mixin B");
            ok(i, b"oo::objdefine keep mixin");
            ok(i, b"B destroy");
            assert_eq!(ok(i, b"info object isa object gone"), b"0");
            assert_eq!(ok(i, b"info object isa object keep"), b"1");
        });
    }

    #[test]
    fn self_class_in_object_method_errors() {
        leak_free(|i| {
            // `self class` in a method declared directly on the object has no
            // declaring class.
            ok(i, b"oo::object create obj");
            ok(i, b"oo::objdefine obj method demo {} { self class }");
            assert_eq!(i.eval_str(b"obj demo"), Code::Error);
            assert_eq!(i.result_bytes(), b"method not defined by a class");
            // A class instance method still reports its declaring class.
            ok(i, b"oo::class create C { method d {} { self class } }");
            assert_eq!(ok(i, b"[C new] d"), b"::C");
        });
    }

    #[test]
    fn no_method_forces_user_unknown() {
        leak_free(|i| {
            // A no-method invocation `$obj` with a user `unknown` runs it with
            // empty args (C's FORCE_UNKNOWN); a plain object reports usage.
            ok(i, b"set o [oo::object new]");
            assert_eq!(i.eval_str(b"$o"), Code::Error);
            assert!(i.result_bytes().starts_with(b"wrong # args: should be"));
            ok(
                i,
                b"oo::objdefine $o method unknown args { return \"u:>>$args<<\" }",
            );
            assert_eq!(ok(i, b"$o"), b"u:>><<");
            assert_eq!(ok(i, b"$o foo bar"), b"u:>>foo bar<<");
        });
    }

    #[test]
    fn my_create_reaches_class_factory() {
        leak_free(|i| {
            // A metaclass method can instantiate via `my create`, even when the
            // class unexports `create` for external callers (TclOO factory).
            ok(
                i,
                b"oo::class create meta { superclass oo::class; self { unexport create new; \
                   method make {x} { my create $x } } }",
            );
            // External create is hidden (unknown method, not listing create/new).
            assert_eq!(i.eval_str(b"meta create foo"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"unknown method \"create\": must be destroy or make"
            );
            // The internal `my create` still works.
            assert_eq!(ok(i, b"meta make ::bar"), b"::bar");
            assert_eq!(ok(i, b"info object class ::bar"), b"::meta");
        });
    }

    #[test]
    fn next_shares_caller_frame_level() {
        leak_free(|i| {
            // Every method in a `next` chain runs at the level of the original
            // invocation, so `info level` is the same and `upvar 1` from a
            // next-invoked method reaches the original caller's frame.
            ok(
                i,
                b"oo::class create A { method incr {var step} { upvar 1 $var v; ::incr v $step; return [info level] } }",
            );
            ok(
                i,
                b"oo::class create B { superclass A; method incr {var {step 1}} { list [info level] [next $var $step] } }",
            );
            ok(i, b"B create b");
            ok(i, b"set x 10");
            // Both methods report level 1; A's upvar bumps the global x to 11.
            assert_eq!(ok(i, b"b incr x"), b"1 1");
            assert_eq!(ok(i, b"set x"), b"11");
        });
    }

    #[test]
    fn builtin_slot_objects_present() {
        leak_free(|i| {
            // The eleven built-in slots are ::oo::Slot instances.
            assert_eq!(
                ok(i, b"lsort [info class instances ::oo::Slot]"),
                b"::oo::configuresupport::objreadableproperties \
                  ::oo::configuresupport::objwritableproperties \
                  ::oo::configuresupport::readableproperties \
                  ::oo::configuresupport::writableproperties ::oo::define::filter \
                  ::oo::define::mixin ::oo::define::superclass ::oo::define::variable \
                  ::oo::objdefine::filter ::oo::objdefine::mixin ::oo::objdefine::variable"
            );
            // filter slot: public ops from the class, Get/Set private.
            assert_eq!(
                ok(i, b"lsort [info object methods ::oo::define::filter -all]"),
                b"-append -appendifnew -clear -prepend -remove -set"
            );
            assert_eq!(
                ok(
                    i,
                    b"lsort [info object methods ::oo::define::filter -private]"
                ),
                b"Get Set"
            );
            // mixin slot also has a Resolve + a --default-operation forward.
            assert_eq!(
                ok(
                    i,
                    b"lsort [info object methods ::oo::define::mixin -private]"
                ),
                b"--default-operation Get Resolve Set"
            );
        });
    }

    #[test]
    fn foundation_abstract_singleton_classes() {
        leak_free(|i| {
            // The three foundation metaclasses exist in a fresh interp.
            assert_eq!(ok(i, b"info object class ::oo::singleton"), b"::oo::class");
            assert_eq!(ok(i, b"info object class ::oo::abstract"), b"::oo::class");
            assert_eq!(
                ok(i, b"info object class ::oo::SingletonInstance"),
                b"::oo::class"
            );
            // singleton/abstract are metaclasses (superclass oo::class).
            assert_eq!(
                ok(i, b"info class superclasses ::oo::singleton"),
                b"::oo::class"
            );
            assert_eq!(
                ok(i, b"info class superclasses ::oo::abstract"),
                b"::oo::class"
            );
            // SingletonInstance is a plain subclass of oo::object.
            assert_eq!(
                ok(i, b"info class superclasses ::oo::SingletonInstance"),
                b"::oo::object"
            );
            assert_eq!(
                ok(i, b"lsort [info class subclasses ::oo::class]"),
                b"::oo::abstract ::oo::configurable ::oo::singleton"
            );
        });
    }

    /// Regression coverage for issue #996: `linearize_class` (backing
    /// `method_chain_faceted` — the hot path for every ordinary method
    /// dispatch — and `class_precedence`, `info class call`) and
    /// `gather_class_props` (`info class properties -all`) recursed once
    /// per mixin/superclass level, with only a same-branch cycle guard and
    /// no depth cap before this fix. Confirmed crash reproduction (this
    /// sweep): a deep `mixin` chain (`oo::class create C$i { mixin C[i-1]
    /// }`, no `{*}` needed) SIGABRTs between depth 100-150 on a 256 KiB
    /// stack, and still crashes at depth 2000 on a 1 MiB stack (a plain
    /// `superclass` chain hits the same recursion but is masked by
    /// `self_reachable`'s separate O(n²)-ish cycle-check cost, which makes
    /// naive *construction* slow before reaching crash depth — mixins avoid
    /// that and reproduce cleanly). This builds a 2000-deep mixin chain
    /// (matching that confirmed-still-crashing depth) and drives both fixed
    /// functions over the whole thing via `info class call` and `info class
    /// properties -all`; the assertion is that both complete at all, not
    /// what they return — `MAX_MRO_DEPTH` (64) means the reported
    /// precedence/property set is legitimately truncated for a hierarchy
    /// this deep, the same graceful "just stop descending" degradation the
    /// pre-existing `path`/`seen` cycle guards already apply to a malformed
    /// hierarchy, rather than erroring the whole dispatch out or crashing.
    #[test]
    fn deeply_nested_mixin_chain_survives_linearisation_and_property_gathering() {
        leak_free(|i| {
            const DEPTH: usize = 2000;
            ok(i, b"oo::class create C0");
            for n in 1..DEPTH {
                ok(
                    i,
                    format!("oo::class create C{n} {{ mixin C{} }}", n - 1).as_bytes(),
                );
            }
            let last = DEPTH - 1;
            let _ = i.eval_str(format!("info class call C{last} foo").as_bytes());
            let _ = i.eval_str(format!("info class properties C{last} -all").as_bytes());
            for n in (0..DEPTH).rev() {
                let _ = i.eval_str(format!("C{n} destroy").as_bytes());
            }
        });
    }

    /// A moderately nested mixin chain (well under `MAX_MRO_DEPTH`) still
    /// linearises completely and correctly — the safety net must not fire,
    /// let alone truncate anything, on realistic nesting depths. Checks both
    /// fixed functions: method dispatch still resolves through the whole
    /// mixin chain to the base class's implementation (`linearize_class` via
    /// `method_chain_faceted`), and `-all` property gathering still unions
    /// every level's declared properties (`gather_class_props`) — exactly
    /// the behaviour before this fix, not merely "does not crash".
    #[test]
    fn moderately_nested_mixin_chain_still_behaves_identically() {
        leak_free(|i| {
            ok(
                i,
                b"oo::class create C0 { method foo {} { return from-c0 } }",
            );
            ok(i, b"oo::class create C1 { mixin C0 }");
            ok(i, b"oo::class create C2 { mixin C1 }");
            ok(i, b"oo::class create C3 { mixin C2 }");
            ok(i, b"C3 create obj");
            assert_eq!(ok(i, b"obj foo"), b"from-c0");
            ok(i, b"obj destroy");

            ok(
                i,
                b"oo::configurable create Base { property p0 -kind readable }",
            );
            ok(
                i,
                b"oo::configurable create P1 { mixin Base; property p1 -kind readable }",
            );
            ok(
                i,
                b"oo::configurable create P2 { mixin P1; property p2 -kind readable }",
            );
            assert_eq!(
                ok(i, b"lsort [info class properties P2 -all]"),
                b"-p0 -p1 -p2"
            );

            ok(i, b"C3 destroy; C2 destroy; C1 destroy; C0 destroy");
            ok(i, b"P2 destroy; P1 destroy; Base destroy");
        });
    }
}
