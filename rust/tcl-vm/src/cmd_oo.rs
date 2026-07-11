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

//! `TclOO` for the bytecode VM (`RUST_ISSUE_009`).
//!
//! The object system the tree-walking WASM runtime already backs
//! (`runtime/rust/src/cmd_oo.rs`), ported to the VM's `Rc`-value / bytecode
//! model so `oo::class create C {…}`, method dispatch, inheritance (`next`),
//! `self`/`my`, constructors/destructors, and `oo::define`/`oo::objdefine` work
//! in the VM as they do under WASM and native tclsh 9.
//!
//! Design (mirrors the runtime, simplified where the VM's common-case parity
//! target allows):
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
//! Names are stored canonically (no leading `::`, the VM's command-table
//! convention); [`display`] renders the `::`-qualified form used in results and
//! messages.

use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use tcl_bytecode::FunctionAsm;
use tcl_runtime_api::{Code, Completion};

use crate::command::{Command, Param, ProcDef, parse_params};
use crate::interp::{Vm, err, ok};
use crate::value::Value;

/// A method (or constructor/destructor) body: a proc-like param list plus the
/// pre-compiled body, and whether it is exported for public (`$obj m`) dispatch.
#[derive(Clone)]
struct Method {
    params: Vec<Param>,
    has_args: bool,
    body: Rc<FunctionAsm>,
    body_src: Value,
    /// `None` for an ordinary method; `Some(prefix)` for a `forward` (invoking
    /// the method evaluates `prefix args…`).
    forward: Option<Vec<Value>>,
    /// Default public visibility (from the name / definition flags). The
    /// per-class/object `exported`/`unexported` override sets take precedence.
    exported: bool,
}

/// The class facet of an entity: what its *instances* inherit.
#[derive(Clone, Default)]
struct Class {
    /// Superclass FQNs (canonical). Empty means `::oo::object`.
    supers: Vec<String>,
    methods: BTreeMap<String, Method>,
    constructor: Option<Method>,
    destructor: Option<Method>,
    /// Declared instance-variable names, auto-linked into method frames.
    variables: Vec<String>,
    mixins: Vec<String>,
    /// `export`/`unexport` overrides applied to instance methods.
    exported: BTreeSet<String>,
    unexported: BTreeSet<String>,
    /// TIP 558 property slots — the readable/writable property names declared
    /// on this class via `::oo::configuresupport::{readable,writable}properties`
    /// (a `BTreeSet` keeps them sorted and unique, as `info class properties`
    /// reports them).
    readable_properties: BTreeSet<String>,
    writable_properties: BTreeSet<String>,
}

/// The object facet of an entity: its own (per-instance) state.
#[derive(Clone)]
struct Object {
    /// FQN of the object's class (canonical).
    class: String,
    /// The namespace holding this object's instance variables (canonical).
    ns: String,
    /// Monotonic id (`info object creationid`), stable across rename.
    creation_id: u64,
    /// Per-object methods (`oo::objdefine … method`), incl. class-side methods
    /// (`oo::define C self method`) since a class is an object too.
    methods: BTreeMap<String, Method>,
    variables: Vec<String>,
    mixins: Vec<String>,
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
    provider: String,
    is_object: bool,
}

/// What a running method's `self`/`my`/`next` operate on: the object, the
/// resolved chain of providers for the invoked method, the current index into
/// it, the public method name, and whether it was an external (`$obj m`) call.
pub(crate) struct OoFrame {
    object: String,
    chain: Vec<Step>,
    index: usize,
    /// The invoked method name (empty for a constructor, `<destructor>` for a
    /// destructor).
    method: String,
    external: bool,
    /// The object as the caller named it (`obj`, `::oo::Obj7`, …), reused by
    /// `my`/`next` to build a `wrong # args` usage that echoes the call site.
    invoked: String,
    /// The `wrong # args` leading words for the running step (`obj m`, `C new`).
    usage_prefix: String,
}

/// The current `oo::define`/`oo::objdefine` target and the call-frame level it
/// is active at (so a `method`/`variable` in a proc called from a body is not
/// mistaken for a definition directive).
enum DefTarget {
    Class(String),
    Object(String),
}

/// The whole object system's runtime state, held by the [`Vm`].
#[derive(Default)]
pub(crate) struct OoState {
    classes: BTreeMap<String, Class>,
    objects: BTreeMap<String, Object>,
    /// Monotonic counter for anonymous `::oo::ObjN` names and creation ids.
    counter: u64,
    /// Active method invocations (innermost last) — drives `self`/`my`/`next`.
    pub(crate) call_stack: Vec<OoFrame>,
    /// Active definition targets (innermost last), each tagged with its level.
    def_stack: Vec<(DefTarget, usize)>,
}

/// The per-flow OO execution stacks — the subset of [`OoState`] that a coroutine
/// owns and that is swapped in/out on suspend/resume (the class/object
/// registries stay shared). Mirrors `runtime/rust/src/cmd_oo.rs`'s `OoExec`.
#[derive(Default)]
pub(crate) struct OoExec {
    call_stack: Vec<OoFrame>,
    def_stack: Vec<(DefTarget, usize)>,
}

impl OoState {
    /// Exchange this interpreter's live OO execution stacks with `e` — one half
    /// of a coroutine context switch. The registries are shared and untouched.
    pub(crate) fn swap_exec(&mut self, e: &mut OoExec) {
        std::mem::swap(&mut self.call_stack, &mut e.call_stack);
        std::mem::swap(&mut self.def_stack, &mut e.def_stack);
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
fn resolve_class(vm: &Vm, name: &str) -> String {
    if let Some(abs) = name.strip_prefix("::") {
        return abs.to_string();
    }
    let cur = vm.current_ns();
    if !cur.is_empty() {
        let q = format!("{cur}::{name}");
        if vm.oo.classes.contains_key(&q) {
            return q;
        }
    }
    name.to_string()
}

/// A method is exported by default iff its name begins with an ASCII lowercase
/// letter (C's `TclOO` visibility default: `alpha` is public, `Cap`/`_priv` are
/// not).
fn exported_by_default(name: &str) -> bool {
    name.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
}

/// Format a sorted method-name set as `must be a, b or c` (`TclOO`'s list style
/// — no Oxford comma before the final `or`).
fn oxford_or(names: &BTreeSet<String>) -> String {
    let v: Vec<&str> = names.iter().map(String::as_str).collect();
    match v.as_slice() {
        [] => String::new(),
        [a] => (*a).to_string(),
        [rest @ .., last] => format!("{} or {last}", rest.join(", ")),
    }
}

pub(crate) fn register(vm: &mut Vm) {
    // Definition-body commands: global builtins that consult the active
    // definition target. Outside a body they error (or, for `variable`, fall
    // back to the ordinary command).
    vm.register("oo::define", cmd_oo_define);
    vm.register("oo::objdefine", cmd_oo_objdefine);
    vm.register("method", |v, a| def_body_cmd(v, "method", a));
    vm.register("constructor", |v, a| def_body_cmd(v, "constructor", a));
    vm.register("destructor", |v, a| def_body_cmd(v, "destructor", a));
    vm.register("superclass", |v, a| def_body_cmd(v, "superclass", a));
    vm.register("export", |v, a| def_body_cmd(v, "export", a));
    vm.register("unexport", |v, a| def_body_cmd(v, "unexport", a));
    vm.register("mixin", |v, a| def_body_cmd(v, "mixin", a));
    vm.register("forward", |v, a| def_body_cmd(v, "forward", a));
    // The method-context commands. tclsh exposes these only inside a method
    // body; the VM registers them globally but each reports the tclsh
    // `invalid command name "…"` when invoked with no active method (the call
    // stack is empty), so top-level use matches (`oo-1.x`).
    vm.register("self", cmd_self);
    vm.register("next", cmd_next);
    vm.register("nextto", cmd_nextto);
    vm.register("my", cmd_my);
    // TIP 558 property slots — the low-level readable/writable property setters
    // used inside `oo::define` / `oo::objdefine` bodies. `(obj, writable)`:
    // `obj` targets the per-object slot (`obj*properties`), else the class slot.
    vm.register("::oo::configuresupport::readableproperties", |v, a| {
        property_slot(v, a, false, false)
    });
    vm.register("::oo::configuresupport::writableproperties", |v, a| {
        property_slot(v, a, false, true)
    });
    vm.register("::oo::configuresupport::objreadableproperties", |v, a| {
        property_slot(v, a, true, false)
    });
    vm.register("::oo::configuresupport::objwritableproperties", |v, a| {
        property_slot(v, a, true, true)
    });
    bootstrap(vm);
}

/// A TIP 558 property slot (`::oo::configuresupport::{obj,}{readable,writable}properties`).
///
/// Callable only inside an `oo::define`/`oo::objdefine` body; applies a slot
/// operation to the target's readable or writable property set. Supported ops
/// are `-set` (replace, also the default for bare names), `-append`, `-remove`,
/// and `-clear`; every op keeps the set sorted and duplicate-free.
fn property_slot(vm: &mut Vm, args: &[Value], obj: bool, writable: bool) -> Completion<Value> {
    let Some((is_class, target)) = active_target(vm) else {
        return err("this command can only be called from within the body of a definition");
    };
    let (op, names): (String, &[Value]) = match args.split_first() {
        Some((first, rest)) if first.to_str().starts_with('-') => {
            (first.to_str().to_string(), rest)
        }
        _ => ("-set".to_owned(), args),
    };
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
        return err(format!("{} does not refer to an object", display(&target)));
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

    for root in ["oo::object", "oo::class"] {
        vm.oo.classes.insert(root.to_string(), Class::default());
        let id = vm.oo.counter;
        vm.oo.counter += 1;
        vm.oo.objects.insert(
            root.to_string(),
            Object {
                class: "oo::class".to_string(),
                ns: root.to_string(),
                creation_id: id,
                methods: BTreeMap::new(),
                variables: Vec::new(),
                mixins: Vec::new(),
                exported: BTreeSet::new(),
                unexported: BTreeSet::new(),
                readable_properties: BTreeSet::new(),
                writable_properties: BTreeSet::new(),
                destroyed: false,
            },
        );
        vm.register_command(root, Command::Object(root.to_string()));
    }
    // `oo::object`'s only super is nothing (it is the root); `oo::class` extends
    // `oo::object`.
    if let Some(c) = vm.oo.classes.get_mut("oo::class") {
        c.supers = vec!["oo::object".to_string()];
    }
}

// ---------------------------------------------------------------------------
// Dispatch entry (called from the engine for a `Command::Object`).
// ---------------------------------------------------------------------------

/// Dispatch `argv` against the object/class `key` (canonical). `invoked` is the
/// word the command was called under (for the `wrong # args` usage).
pub(crate) fn oo_dispatch(
    vm: &mut Vm,
    key: &str,
    invoked: &str,
    argv: &[Value],
) -> Completion<Value> {
    if argv.is_empty() {
        return err(format!(
            "wrong # args: should be \"{invoked} method ?arg ...?\""
        ));
    }
    let method = argv[0].to_str().to_string();
    let rest = &argv[1..];
    // A class responds to the factory built-ins first (create/new); anything
    // else is a class-side method (or `destroy`).
    if vm.oo.classes.contains_key(key) {
        match method.as_str() {
            "create" => return factory(vm, key, invoked, false, rest),
            "new" if key != "oo::class" => return factory(vm, key, invoked, true, rest),
            _ => {}
        }
    }
    oo_invoke(vm, key, &method, rest, true, invoked)
}

// ---------------------------------------------------------------------------
// Object / class creation.
// ---------------------------------------------------------------------------

/// The `create objectName ?arg…?` / `new ?arg…?` factory built-ins of a class.
/// `class_invoked` is the class as the caller named it (for the constructor's
/// `wrong # args` usage).
fn factory(
    vm: &mut Vm,
    class_key: &str,
    class_invoked: &str,
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
        .any(|s| s.provider == "oo::class");
    if is_meta {
        let (name, body) = if anon {
            (fresh_obj_name(vm), args.first())
        } else {
            let Some((n, rest)) = args.split_first() else {
                return err(format!(
                    "wrong # args: should be \"{} create className ?definitionScript?\"",
                    display(class_key)
                ));
            };
            let n = n.to_str();
            if n.is_empty() {
                return err("object name must not be empty");
            }
            (vm.qualify_name(&n), rest.first())
        };
        return make_class(vm, &name, body);
    }

    let (obj_key, obj_ns, ctor_args, ctor_usage) = if anon {
        // An anonymous object's command *is* its instance namespace (`oo::ObjN`).
        let n = fresh_obj_name(vm);
        (n.clone(), n, args, format!("{class_invoked} new"))
    } else {
        let Some((name, rest)) = args.split_first() else {
            return err(format!(
                "wrong # args: should be \"{} create objectName ?arg ...?\"",
                display(class_key)
            ));
        };
        let name = name.to_str();
        if name.is_empty() {
            return err("object name must not be empty");
        }
        // A named object's instance namespace is still a fresh `oo::ObjN`,
        // decoupled from its command name (C's `oo::Obj` counter).
        (
            vm.qualify_name(&name),
            fresh_obj_name(vm),
            rest,
            format!("{class_invoked} create {name}"),
        )
    };
    if vm.lookup_command(&obj_key).is_some() {
        return err(format!(
            "can't create object \"{}\": command already exists with that name",
            display(&obj_key)
        ));
    }
    oo_new(vm, class_key, &obj_key, &obj_ns, ctor_args, &ctor_usage)
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
    class_key: &str,
    obj_key: &str,
    obj_ns: &str,
    ctor_args: &[Value],
    ctor_usage: &str,
) -> Completion<Value> {
    let id = vm.oo.counter;
    vm.oo.counter += 1;
    let ns = obj_ns.to_string();
    vm.declare_namespace(&ns);
    vm.oo.objects.insert(
        obj_key.to_string(),
        Object {
            class: class_key.to_string(),
            ns: ns.clone(),
            creation_id: id,
            methods: BTreeMap::new(),
            variables: Vec::new(),
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
    vm.register_command(obj_key, Command::Object(obj_key.to_string()));

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
    if !chain.is_empty() {
        let res = run_step(
            vm,
            obj_key,
            &chain,
            0,
            String::new(),
            ctor_args,
            false,
            display(obj_key),
            ctor_usage.to_string(),
        );
        if !res.code.is_ok() && res.code != Code::Return {
            // Tear the half-built object down and re-raise.
            teardown(vm, obj_key);
            return res;
        }
    }
    ok(Value::string(display(obj_key)))
}

// ---------------------------------------------------------------------------
// Method resolution + execution.
// ---------------------------------------------------------------------------

impl OoState {
    /// Depth-first linearisation of a class (mixins, self, supers), with a
    /// path-based cycle guard so a diamond revisits a shared base (kept once by
    /// the caller's keep-last dedup) but a true cycle terminates.
    fn class_linear(&self, class_key: &str, out: &mut Vec<Step>, path: &mut Vec<String>) {
        if path.iter().any(|p| p == class_key) {
            return;
        }
        let Some(cls) = self.classes.get(class_key) else {
            return;
        };
        path.push(class_key.to_string());
        for mx in &cls.mixins {
            self.class_linear(mx, out, path);
        }
        out.push(Step {
            provider: class_key.to_string(),
            is_object: false,
        });
        for s in &cls.supers {
            self.class_linear(s, out, path);
        }
        // Every class ultimately derives from `oo::object`.
        if cls.supers.is_empty() && class_key != "oo::object" {
            self.class_linear("oo::object", out, path);
        }
        path.pop();
    }

    /// The class-side linearisation (keep-last deduped), used for constructor /
    /// destructor chains.
    fn class_linear_of(&self, class_key: &str) -> Vec<Step> {
        let mut out = Vec::new();
        self.class_linear(class_key, &mut out, &mut Vec::new());
        keep_last(&out)
    }

    /// The full method-search precedence for an object: object mixins → the
    /// object itself → the class MRO, keep-last deduped.
    fn object_precedence(&self, obj_key: &str) -> Vec<Step> {
        let mut out = Vec::new();
        let mut path = Vec::new();
        if let Some(obj) = self.objects.get(obj_key) {
            for mx in &obj.mixins {
                self.class_linear(mx, &mut out, &mut path);
            }
            out.push(Step {
                provider: obj_key.to_string(),
                is_object: true,
            });
            self.class_linear(&obj.class, &mut out, &mut path);
        }
        keep_last(&out)
    }

    /// The `Method` a step provides for `name`, if any.
    fn step_method(&self, step: &Step, name: &str) -> Option<Method> {
        if step.is_object {
            self.objects.get(&step.provider)?.methods.get(name).cloned()
        } else {
            self.classes.get(&step.provider)?.methods.get(name).cloned()
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
    fn visible_methods(&self, obj_key: &str, internal: bool) -> BTreeSet<String> {
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
        if self.classes.contains_key(obj_key) {
            names.insert("create".to_string());
            if obj_key != "oo::class" {
                names.insert("new".to_string());
            }
        }
        names
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
    obj_key: &str,
    method: &str,
    args: &[Value],
    external: bool,
    invoked: &str,
) -> Completion<Value> {
    if !vm.oo.objects.contains_key(obj_key) {
        return err(format!("invalid command name \"{}\"", display(obj_key)));
    }
    // Object built-in methods (available internally, or when exported).
    if matches!(method, "variable" | "varname" | "eval")
        && (!external || is_exported_builtin(vm, obj_key, method))
    {
        return builtin_method(vm, obj_key, method, args);
    }

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
        return unknown_method(vm, obj_key, method, external);
    }
    // External calls require the most-derived definer to be exported.
    if external && !vm.oo.step_exported(&chain[0], method) {
        return unknown_method(vm, obj_key, method, external);
    }
    let usage = format!("{invoked} {method}");
    run_step(
        vm,
        obj_key,
        &chain,
        0,
        method.to_string(),
        args,
        external,
        invoked.to_string(),
        usage,
    )
}

/// Whether an object built-in (`variable`/`varname`/`eval`) is exported — they
/// are unexported by default, so only reachable externally after `self export`.
fn is_exported_builtin(vm: &Vm, obj_key: &str, method: &str) -> bool {
    vm.oo
        .objects
        .get(obj_key)
        .is_some_and(|o| o.exported.contains(method))
}

/// The `unknown method "X": must be …` error (or "no visible methods").
fn unknown_method(vm: &Vm, obj_key: &str, method: &str, external: bool) -> Completion<Value> {
    let names = vm.oo.visible_methods(obj_key, !external);
    if names.is_empty() {
        return err(format!(
            "object \"{}\" has no visible methods",
            display(obj_key)
        ));
    }
    err(format!(
        "unknown method \"{method}\": must be {}",
        oxford_or(&names)
    ))
}

/// Execute step `index` of `chain` on `obj_key`, for the invoked `method`
/// (empty = constructor, `<destructor>` = destructor). Pushes an [`OoFrame`] so
/// `self`/`next` see the chain.
#[allow(clippy::too_many_arguments)] // A method activation genuinely carries this context.
fn run_step(
    vm: &mut Vm,
    obj_key: &str,
    chain: &[Step],
    index: usize,
    method: String,
    args: &[Value],
    external: bool,
    invoked: String,
    usage_prefix: String,
) -> Completion<Value> {
    let step = chain[index].clone();
    // Resolve the method body for this step.
    let m = if method.is_empty() {
        vm.oo
            .classes
            .get(&step.provider)
            .and_then(|c| c.constructor.clone())
    } else if method == "<destructor>" {
        vm.oo
            .classes
            .get(&step.provider)
            .and_then(|c| c.destructor.clone())
    } else {
        vm.oo.step_method(&step, &method)
    };
    let Some(m) = m else {
        return err("no such method");
    };

    // A forward: evaluate `prefix args…` (qualified in the object namespace).
    if let Some(prefix) = &m.forward {
        let mut words: Vec<Value> = prefix.clone();
        words.extend_from_slice(args);
        let ns = vm.oo.objects.get(obj_key).map(|o| o.ns.clone());
        let frame = OoFrame {
            object: obj_key.to_string(),
            chain: chain.to_vec(),
            index,
            method: method.clone(),
            external,
            invoked: invoked.clone(),
            usage_prefix: usage_prefix.clone(),
        };
        vm.oo.call_stack.push(frame);
        let _ = ns;
        let name = words[0].to_str().to_string();
        let res = vm.invoke_command(&name, &words[1..]);
        vm.oo.call_stack.pop();
        return res;
    }

    let obj_ns = vm
        .oo
        .objects
        .get(obj_key)
        .map(|o| o.ns.clone())
        .unwrap_or_default();
    // Instance variables to auto-link: the providing entity's declared vars,
    // plus the object's own declared vars.
    let mut decl: Vec<String> = Vec::new();
    if step.is_object {
        if let Some(o) = vm.oo.objects.get(&step.provider) {
            decl.extend(o.variables.iter().cloned());
        }
    } else if let Some(c) = vm.oo.classes.get(&step.provider) {
        decl.extend(c.variables.iter().cloned());
    }
    if let Some(o) = vm.oo.objects.get(obj_key) {
        for v in &o.variables {
            if !decl.contains(v) {
                decl.push(v.clone());
            }
        }
    }
    let link_vars: Vec<(String, String)> = decl
        .into_iter()
        .map(|v| {
            let storage = format!("{obj_ns}::{v}");
            (v, storage)
        })
        .collect();

    let proc = ProcDef {
        name: format!("{obj_ns}::{method}"),
        namespace: obj_ns,
        params: m.params.clone(),
        has_args: m.has_args,
        body: Rc::clone(&m.body),
        body_src: m.body_src.clone(),
        usage_name: Some(usage_prefix.clone()),
    };
    let frame = OoFrame {
        object: obj_key.to_string(),
        chain: chain.to_vec(),
        index,
        method,
        external,
        invoked,
        usage_prefix,
    };
    vm.oo_run_method(&proc, args, &link_vars, frame)
}

/// The object built-in methods `variable name…`, `varname name`, `eval script`.
fn builtin_method(vm: &mut Vm, obj_key: &str, method: &str, args: &[Value]) -> Completion<Value> {
    let ns = match vm.oo.objects.get(obj_key) {
        Some(o) => o.ns.clone(),
        None => return err(format!("invalid command name \"{}\"", display(obj_key))),
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
                vm.add_link(&name, 0, &format!("{ns}::{name}"));
            }
            ok(Value::empty())
        }
        "varname" => {
            let Some(a) = args.first() else {
                return err("wrong # args: should be \"my varname varName\"");
            };
            ok(Value::string(display(&format!("{ns}::{}", a.to_str()))))
        }
        "eval" => {
            let script = args
                .iter()
                .map(|v| v.to_str().to_string())
                .collect::<Vec<_>>()
                .join(" ");
            vm.push_ns(ns);
            let res = match vm.eval_source(&script) {
                Ok(c) => c,
                Err(e) => err(e.message),
            };
            vm.pop_ns();
            res
        }
        _ => unreachable!("builtin_method called with {method}"),
    }
}

// ---------------------------------------------------------------------------
// self / my / next / nextto
// ---------------------------------------------------------------------------

fn cmd_my(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some(fr) = vm.oo.call_stack.last() else {
        return err("invalid command name \"my\"");
    };
    let obj = fr.object.clone();
    let invoked = fr.invoked.clone();
    let Some((method, rest)) = args.split_first() else {
        return err("wrong # args: should be \"my methodName ?arg ...?\"");
    };
    let method = method.to_str().to_string();
    oo_invoke(vm, &obj, &method, rest, false, &invoked)
}

fn cmd_self(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some(fr) = vm.oo.call_stack.last() else {
        return err("invalid command name \"self\"");
    };
    let obj = fr.object.clone();
    let sub = args.first().map(|v| v.to_str().to_string());
    match sub.as_deref() {
        None | Some("object") => ok(Value::string(display(&obj))),
        Some("namespace") => {
            let ns = vm.oo.objects.get(&obj).map(|o| o.ns.clone()).unwrap_or(obj);
            ok(Value::string(display(&ns)))
        }
        Some("class") => {
            // The class that declares the running method (its step provider).
            let step = &fr.chain[fr.index];
            if step.is_object {
                return err("method not defined by a class");
            }
            ok(Value::string(display(&step.provider)))
        }
        Some("method") => ok(Value::string(fr.method.clone())),
        Some("call") => {
            let steps: Vec<Value> = fr
                .chain
                .iter()
                .map(|s| Value::string(display(&s.provider)))
                .collect();
            ok(Value::list(vec![
                Value::list(steps),
                Value::int(i64::try_from(fr.index).unwrap_or(0)),
            ]))
        }
        Some("target") => {
            let step = &fr.chain[fr.index];
            ok(Value::list(vec![
                Value::string(display(&step.provider)),
                Value::string(fr.method.clone()),
            ]))
        }
        Some(other) => err(format!("unsupported self subcommand \"{other}\"")),
    }
}

fn cmd_next(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some(fr) = vm.oo.call_stack.last() else {
        return err("invalid command name \"next\"");
    };
    let obj = fr.object.clone();
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
    run_step(
        vm,
        &obj,
        &chain,
        index + 1,
        method,
        args,
        external,
        invoked,
        usage,
    )
}

fn cmd_nextto(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some(fr) = vm.oo.call_stack.last() else {
        return err("invalid command name \"nextto\"");
    };
    let Some((cls, rest)) = args.split_first() else {
        return err("wrong # args: should be \"nextto class ?arg...?\"");
    };
    let obj = fr.object.clone();
    let chain = fr.chain.clone();
    let index = fr.index;
    let method = fr.method.clone();
    let external = fr.external;
    let invoked = fr.invoked.clone();
    let usage = fr.usage_prefix.clone();
    let target = resolve_class(vm, &cls.to_str());
    // Find the target class ahead of the current step.
    if let Some(pos) = chain
        .iter()
        .enumerate()
        .position(|(i, s)| i > index && !s.is_object && s.provider == target)
    {
        return run_step(
            vm, &obj, &chain, pos, method, rest, external, invoked, usage,
        );
    }
    err(format!(
        "method has no non-filter implementation by \"{}\"",
        display(&target)
    ))
}

// ---------------------------------------------------------------------------
// destroy / destructor
// ---------------------------------------------------------------------------

/// Run the destructor chain, then tear the object (and, if a class, its
/// descendants) down.
fn oo_destroy(vm: &mut Vm, obj_key: &str) -> Completion<Value> {
    match vm.oo.objects.get_mut(obj_key) {
        Some(o) if o.destroyed => return ok(Value::empty()),
        Some(o) => o.destroyed = true,
        None => return ok(Value::empty()),
    }
    // Destroying a class cascades to everything derived from it: its direct
    // instances and its direct subclasses (each recursion in turn reaches their
    // instances / subclasses), matching TclOO. The `destroyed` guard above makes
    // this safe against cycles.
    if vm.oo.classes.contains_key(obj_key) {
        let derived: Vec<String> = vm
            .oo
            .objects
            .iter()
            .filter(|(k, o)| {
                k.as_str() != obj_key
                    && (o.class == obj_key
                        || vm
                            .oo
                            .classes
                            .get(*k)
                            .is_some_and(|c| c.supers.iter().any(|s| s == obj_key)))
            })
            .map(|(k, _)| k.clone())
            .collect();
        for d in derived {
            let _ = oo_destroy(vm, &d);
        }
    }
    let class = vm.oo.objects.get(obj_key).map(|o| o.class.clone());
    let mut result = ok(Value::empty());
    if let Some(class) = class {
        let chain: Vec<Step> = vm
            .oo
            .class_linear_of(&class)
            .into_iter()
            .filter(|s| {
                vm.oo
                    .classes
                    .get(&s.provider)
                    .is_some_and(|c| c.destructor.is_some())
            })
            .collect();
        if !chain.is_empty() {
            let d = display(obj_key);
            let res = run_step(
                vm,
                obj_key,
                &chain,
                0,
                "<destructor>".to_string(),
                &[],
                false,
                d.clone(),
                d,
            );
            if !res.code.is_ok() && res.code != Code::Return {
                result = res;
            }
        }
    }
    teardown(vm, obj_key);
    result
}

/// Remove an object's command and records (no destructor run).
fn teardown(vm: &mut Vm, obj_key: &str) {
    vm.take_command(obj_key);
    vm.oo.objects.remove(obj_key);
    vm.oo.classes.remove(obj_key);
}

// ---------------------------------------------------------------------------
// oo::define / oo::objdefine and the definition-body commands.
// ---------------------------------------------------------------------------

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
    let target = vm.qualify_name(&args[0].to_str());
    if !vm.oo.objects.contains_key(&target) {
        return err(format!("{} does not refer to an object", display(&target)));
    }
    if is_class && !vm.oo.classes.contains_key(&target) {
        return err(format!("{} does not refer to a class", display(&target)));
    }
    let dt = if is_class {
        DefTarget::Class(target.clone())
    } else {
        DefTarget::Object(target.clone())
    };
    let level = vm.current_level();
    vm.oo.def_stack.push((dt, level));
    let result = if args.len() == 2 {
        // Script form: evaluate the body with the def target active.
        match vm.eval_source(&args[1].to_str()) {
            Ok(c) if c.code == Code::Return => ok(c.result),
            Ok(c) => c,
            Err(e) => err(e.message),
        }
    } else {
        // Single-command form: dispatch `sub args…` as one definition directive.
        let name = args[1].to_str().to_string();
        vm.invoke_command(&name, &args[2..])
    };
    vm.oo.def_stack.pop();
    result
}

/// The active definition target, iff evaluation is directly at its level.
fn active_target(vm: &Vm) -> Option<(bool, String)> {
    let (dt, level) = vm.oo.def_stack.last()?;
    if *level != vm.current_level() {
        return None;
    }
    Some(match dt {
        DefTarget::Class(c) => (true, c.clone()),
        DefTarget::Object(o) => (false, o.clone()),
    })
}

/// If evaluation is directly inside an `oo::define`/`oo::objdefine` body,
/// declare `args` as instance variables of the target and return `Some`;
/// otherwise `None` so the ordinary `variable` command runs. The `variable`
/// builtin calls this first (C's `TclOODefineVariablesObjCmd` vs the core
/// `variable`).
pub(crate) fn maybe_declare_variable(vm: &mut Vm, args: &[Value]) -> Option<Completion<Value>> {
    let (is_class, target) = active_target(vm)?;
    let mut names: Vec<String> = Vec::new();
    for a in args {
        let name = a.to_str().to_string();
        if name.contains("::") {
            return Some(err(format!(
                "invalid declared variable name \"{name}\": must not contain namespace separators"
            )));
        }
        names.push(name);
    }
    if is_class {
        if let Some(c) = vm.oo.classes.get_mut(&target) {
            for n in names {
                if !c.variables.contains(&n) {
                    c.variables.push(n);
                }
            }
        }
    } else if let Some(o) = vm.oo.objects.get_mut(&target) {
        for n in names {
            if !o.variables.contains(&n) {
                o.variables.push(n);
            }
        }
    }
    Some(ok(Value::empty()))
}

/// A definition-body command (`method`, `superclass`, …). Resolves the active
/// target and applies the directive.
fn def_body_cmd(vm: &mut Vm, verb: &str, args: &[Value]) -> Completion<Value> {
    let Some((is_class, target)) = active_target(vm) else {
        return err("this command can only be called from within the body of a definition");
    };
    match verb {
        "method" => def_method(vm, is_class, &target, args),
        "constructor" => def_constructor(vm, is_class, &target, args),
        "destructor" => def_destructor(vm, is_class, &target, args),
        "superclass" => def_superclass(vm, is_class, &target, args),
        "export" => def_export(vm, is_class, &target, args, true),
        "unexport" => def_export(vm, is_class, &target, args, false),
        "mixin" => def_mixin(vm, is_class, &target, args),
        "forward" => def_forward(vm, is_class, &target, args),
        _ => err(format!("unknown definition command \"{verb}\"")),
    }
}

/// Compile a method/constructor body and build a [`Method`].
fn build_method(
    vm: &mut Vm,
    params_spec: &str,
    body: &Value,
    exported: bool,
) -> Result<Method, Completion<Value>> {
    let (params, has_args) = parse_params(params_spec).map_err(err)?;
    let compiled = vm
        .compile_dynamic_body(&body.to_str())
        .ok_or_else(|| err("could not compile method body"))?;
    Ok(Method {
        params,
        has_args,
        body: compiled,
        body_src: body.clone(),
        forward: None,
        exported,
    })
}

fn def_method(vm: &mut Vm, is_class: bool, target: &str, args: &[Value]) -> Completion<Value> {
    // `method name ?-export|-unexport? args body` (a subset of the flag set).
    let mut idx = 0;
    let mut vis: Option<bool> = None;
    while let Some(a) = args.get(idx) {
        match a.to_str().as_ref() {
            "-export" => vis = Some(true),
            "-unexport" | "-private" => vis = Some(false),
            _ => break,
        }
        idx += 1;
    }
    let (Some(name), Some(params), Some(body)) =
        (args.get(idx), args.get(idx + 1), args.get(idx + 2))
    else {
        return err("wrong # args: should be \"method name ?option? args body\"");
    };
    let name = name.to_str().to_string();
    let exported = vis.unwrap_or_else(|| exported_by_default(&name));
    let method = match build_method(vm, &params.to_str(), body, exported) {
        Ok(m) => m,
        Err(e) => return e,
    };
    if is_class {
        if let Some(c) = vm.oo.classes.get_mut(target) {
            c.methods.insert(name, method);
        }
    } else if let Some(o) = vm.oo.objects.get_mut(target) {
        o.methods.insert(name, method);
    }
    ok(Value::empty())
}

fn def_constructor(vm: &mut Vm, is_class: bool, target: &str, args: &[Value]) -> Completion<Value> {
    if !is_class {
        return err("constructors are only for classes");
    }
    let [params, body] = args else {
        return err("wrong # args: should be \"constructor arguments body\"");
    };
    // An empty (whitespace-only) body installs *no* constructor — C TclOO's
    // optimisation, so `C new <extra args>` is then silently accepted.
    if body.to_str().trim().is_empty() {
        if let Some(c) = vm.oo.classes.get_mut(target) {
            c.constructor = None;
        }
        return ok(Value::empty());
    }
    let method = match build_method(vm, &params.to_str(), body, true) {
        Ok(m) => m,
        Err(e) => return e,
    };
    if let Some(c) = vm.oo.classes.get_mut(target) {
        c.constructor = Some(method);
    }
    ok(Value::empty())
}

fn def_destructor(vm: &mut Vm, is_class: bool, target: &str, args: &[Value]) -> Completion<Value> {
    if !is_class {
        return err("destructors are only for classes");
    }
    let [body] = args else {
        return err("wrong # args: should be \"destructor body\"");
    };
    if body.to_str().trim().is_empty() {
        if let Some(c) = vm.oo.classes.get_mut(target) {
            c.destructor = None;
        }
        return ok(Value::empty());
    }
    let method = match build_method(vm, "", body, true) {
        Ok(m) => m,
        Err(e) => return e,
    };
    if let Some(c) = vm.oo.classes.get_mut(target) {
        c.destructor = Some(method);
    }
    ok(Value::empty())
}

fn def_superclass(vm: &mut Vm, is_class: bool, target: &str, args: &[Value]) -> Completion<Value> {
    if !is_class {
        return err("superclass is only for classes");
    }
    let mut supers = Vec::new();
    for a in args {
        let s = resolve_class(vm, &a.to_str());
        if !vm.oo.classes.contains_key(&s) {
            return err("only a class can be a superclass");
        }
        if s == target {
            return err("attempt to form circular dependency graph");
        }
        supers.push(s);
    }
    if let Some(c) = vm.oo.classes.get_mut(target) {
        c.supers = supers;
    }
    ok(Value::empty())
}

fn def_export(
    vm: &mut Vm,
    is_class: bool,
    target: &str,
    args: &[Value],
    export: bool,
) -> Completion<Value> {
    for a in args {
        let name = a.to_str().to_string();
        if is_class {
            if let Some(c) = vm.oo.classes.get_mut(target) {
                if export {
                    c.unexported.remove(&name);
                    c.exported.insert(name);
                } else {
                    c.exported.remove(&name);
                    c.unexported.insert(name);
                }
            }
        } else if let Some(o) = vm.oo.objects.get_mut(target) {
            if export {
                o.unexported.remove(&name);
                o.exported.insert(name);
            } else {
                o.exported.remove(&name);
                o.unexported.insert(name);
            }
        }
    }
    ok(Value::empty())
}

fn def_mixin(vm: &mut Vm, is_class: bool, target: &str, args: &[Value]) -> Completion<Value> {
    let mut mixins = Vec::new();
    for a in args {
        let s = resolve_class(vm, &a.to_str());
        if !vm.oo.classes.contains_key(&s) {
            return err("may only mix in classes");
        }
        mixins.push(s);
    }
    if is_class {
        if let Some(c) = vm.oo.classes.get_mut(target) {
            c.mixins = mixins;
        }
    } else if let Some(o) = vm.oo.objects.get_mut(target) {
        o.mixins = mixins;
    }
    ok(Value::empty())
}

fn def_forward(vm: &mut Vm, is_class: bool, target: &str, args: &[Value]) -> Completion<Value> {
    let Some((name, prefix)) = args.split_first() else {
        return err("wrong # args: should be \"forward name cmdName ?arg ...?\"");
    };
    let name = name.to_str().to_string();
    let exported = exported_by_default(&name);
    let method = Method {
        params: Vec::new(),
        has_args: true,
        body: empty_body(vm),
        body_src: Value::empty(),
        forward: Some(prefix.to_vec()),
        exported,
    };
    if is_class {
        if let Some(c) = vm.oo.classes.get_mut(target) {
            c.methods.insert(name, method);
        }
    } else if let Some(o) = vm.oo.objects.get_mut(target) {
        o.methods.insert(name, method);
    }
    ok(Value::empty())
}

/// A compiled empty body (placeholder for a forward's unused `body`).
fn empty_body(vm: &mut Vm) -> Rc<FunctionAsm> {
    vm.compile_dynamic_body("").expect("empty body compiles")
}

// ---------------------------------------------------------------------------
// oo::class create — the metaclass factory, handled specially so the new
// class's definition body runs with a Class def target.
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// info object / info class introspection (a common subset).
// ---------------------------------------------------------------------------

impl OoState {
    /// Whether `class_key` is reachable in the linearisation of object `obj`'s
    /// class (i.e. `obj` is an instance of `class_key`, honouring inheritance).
    fn is_a(&self, obj_key: &str, class_key: &str) -> bool {
        self.objects.get(obj_key).is_some_and(|o| {
            self.class_linear_of(&o.class)
                .iter()
                .any(|s| s.provider == class_key)
        })
    }

    /// Whether `class_key` is a metaclass (its instances are classes — its
    /// linearisation includes `::oo::class`).
    fn is_metaclass(&self, class_key: &str) -> bool {
        self.classes.contains_key(class_key)
            && self
                .class_linear_of(class_key)
                .iter()
                .any(|s| s.provider == "oo::class")
    }
}

/// `info object subcommand object ?arg…?`.
pub(crate) fn info_object(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some((sub, rest)) = args.split_first() else {
        return err("wrong # args: should be \"info object subcommand object ?arg ...?\"");
    };
    let sub = sub.to_str();
    // `isa` is the odd one out: `info object isa category object ?arg?` — the
    // object comes *after* the category.
    if sub.as_ref() == "isa" {
        return info_object_isa(vm, rest);
    }
    let Some(obj_arg) = rest.first() else {
        return err(format!(
            "wrong # args: should be \"info object {sub} object ?arg ...?\""
        ));
    };
    let obj = vm.qualify_name(&obj_arg.to_str());
    if !vm.oo.objects.contains_key(&obj) {
        return err(format!("{} does not refer to an object", display(&obj)));
    }
    let extra = &rest[1..];
    match sub.as_ref() {
        "class" => {
            if let Some(cls) = extra.first() {
                let c = vm.qualify_name(&cls.to_str());
                ok(Value::bool(vm.oo.is_a(&obj, &c)))
            } else {
                ok(Value::string(display(&vm.oo.objects[&obj].class)))
            }
        }
        "namespace" => ok(Value::string(display(&vm.oo.objects[&obj].ns))),
        "creationid" => ok(Value::int(
            i64::try_from(vm.oo.objects[&obj].creation_id).unwrap_or(0),
        )),
        "mixins" => ok(Value::list(
            vm.oo.objects[&obj]
                .mixins
                .iter()
                .map(|m| Value::string(display(m)))
                .collect(),
        )),
        "variables" => ok(Value::list(
            vm.oo.objects[&obj]
                .variables
                .iter()
                .cloned()
                .map(Value::string)
                .collect(),
        )),
        "vars" => {
            // The instance variables, glob-filtered. v1 reports the declared set
            // (`variable`-listed); enumerating ad-hoc `set` vars in the object
            // namespace is a follow-up.
            let pat = extra.first().map(|v| v.to_str().to_string());
            let mut names: Vec<String> = vm.oo.objects[&obj]
                .variables
                .iter()
                .filter(|n| {
                    pat.as_ref()
                        .is_none_or(|p| tcl_syntax::glob::string_match(p, n))
                })
                .cloned()
                .collect();
            names.sort();
            ok(Value::list(names.into_iter().map(Value::string).collect()))
        }
        "methods" => {
            let all = extra.iter().any(|v| v.to_str().as_ref() == "-all");
            let mut names: BTreeSet<String> = vm.oo.objects[&obj].methods.keys().cloned().collect();
            if all {
                names.extend(vm.oo.visible_methods(&obj, false));
                names.remove("create");
                names.remove("new");
                names.remove("destroy");
            }
            ok(Value::list(names.into_iter().map(Value::string).collect()))
        }
        "properties" => info_object_properties(vm, &obj, extra),
        _ => err(format!(
            "unknown or ambiguous subcommand \"{sub}\": must be class, creationid, isa, methods, mixins, namespace, properties, variables or vars"
        )),
    }
}

/// `info object properties object ?-all? ?-readable|-writable?` (TIP 558).
///
/// Reports the readable (default) or writable properties declared directly on
/// the object; `-all` unions in the object's full precedence (its mixins, then
/// its class MRO). Sorted and duplicate-free.
fn info_object_properties(vm: &mut Vm, obj: &str, extra: &[Value]) -> Completion<Value> {
    let all = extra.iter().any(|v| v.to_str().as_ref() == "-all");
    let writable = extra.iter().any(|v| v.to_str().as_ref() == "-writable");
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
    } else if let Some(o) = vm.oo.objects.get(obj) {
        names.extend(slot_ref_obj(o, writable).iter().cloned());
    }
    ok(Value::list(names.into_iter().map(Value::string).collect()))
}

/// `info object isa category object ?arg?`.
fn info_object_isa(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    let (Some(kind), Some(obj_arg)) = (rest.first(), rest.get(1)) else {
        return err("wrong # args: should be \"info object isa category object ?arg ...?\"");
    };
    let kind = kind.to_str().to_string();
    let obj = vm.qualify_name(&obj_arg.to_str());
    // `isa object` on a non-object is a plain false, not an error.
    if kind == "object" {
        return ok(Value::bool(vm.oo.objects.contains_key(&obj)));
    }
    if !vm.oo.objects.contains_key(&obj) {
        return ok(Value::bool(false));
    }
    let res = match kind.as_str() {
        "class" => vm.oo.classes.contains_key(&obj),
        "metaclass" => vm.oo.is_metaclass(&obj),
        "typeof" => rest
            .get(2)
            .is_some_and(|c| vm.oo.is_a(&obj, &vm.qualify_name(&c.to_str()))),
        "mixin" => rest.get(2).is_some_and(|c| {
            let ck = vm.qualify_name(&c.to_str());
            vm.oo.objects[&obj].mixins.contains(&ck)
        }),
        _ => {
            return err(format!(
                "bad category \"{kind}\": must be class, metaclass, mixin, object or typeof"
            ));
        }
    };
    ok(Value::bool(res))
}

/// `info class subcommand class ?arg…?`.
pub(crate) fn info_class(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some((sub, rest)) = args.split_first() else {
        return err("wrong # args: should be \"info class subcommand class ?arg ...?\"");
    };
    let sub = sub.to_str();
    let Some(cls_arg) = rest.first() else {
        return err(format!(
            "wrong # args: should be \"info class {sub} class ?arg ...?\""
        ));
    };
    let cls = vm.qualify_name(&cls_arg.to_str());
    if !vm.oo.objects.contains_key(&cls) {
        return err(format!("{} does not refer to an object", display(&cls)));
    }
    if !vm.oo.classes.contains_key(&cls) {
        return err(format!("\"{}\" is not a class", display(&cls)));
    }
    let extra = &rest[1..];
    match sub.as_ref() {
        "superclasses" => ok(Value::list(
            vm.oo.classes[&cls]
                .supers
                .iter()
                .map(|s| Value::string(display(s)))
                .collect(),
        )),
        "mixins" => ok(Value::list(
            vm.oo.classes[&cls]
                .mixins
                .iter()
                .map(|m| Value::string(display(m)))
                .collect(),
        )),
        "subclasses" => {
            let pat = extra.first().map(|v| v.to_str().to_string());
            let mut subs: Vec<String> = vm
                .oo
                .classes
                .iter()
                .filter(|(k, c)| *k != &cls && c.supers.contains(&cls))
                .map(|(k, _)| display(k))
                .filter(|d| {
                    pat.as_ref()
                        .is_none_or(|p| tcl_syntax::glob::string_match(p, d))
                })
                .collect();
            subs.sort();
            ok(Value::list(subs.into_iter().map(Value::string).collect()))
        }
        "instances" => {
            let pat = extra.first().map(|v| v.to_str().to_string());
            let mut inst: Vec<String> = vm
                .oo
                .objects
                .iter()
                .filter(|(k, o)| o.class == cls && !vm.oo.classes.contains_key(*k))
                .map(|(k, _)| display(k))
                .filter(|d| {
                    pat.as_ref()
                        .is_none_or(|p| tcl_syntax::glob::string_match(p, d))
                })
                .collect();
            inst.sort();
            ok(Value::list(inst.into_iter().map(Value::string).collect()))
        }
        "methods" => info_class_methods(vm, &cls, extra),
        "constructor" => match &vm.oo.classes[&cls].constructor {
            Some(m) => ok(Value::list(vec![
                params_value(&m.params),
                m.body_src.clone(),
            ])),
            None => ok(Value::empty()),
        },
        "destructor" => match &vm.oo.classes[&cls].destructor {
            Some(m) => ok(m.body_src.clone()),
            None => ok(Value::empty()),
        },
        "variables" => ok(Value::list(
            vm.oo.classes[&cls]
                .variables
                .iter()
                .cloned()
                .map(Value::string)
                .collect(),
        )),
        "properties" => info_class_properties(vm, &cls, extra),
        _ => err(format!(
            "unknown or ambiguous subcommand \"{sub}\": must be constructor, destructor, instances, methods, mixins, properties, subclasses, superclasses or variables"
        )),
    }
}

/// `info class properties class ?-all? ?-readable|-writable?` (TIP 558).
///
/// Reports the readable (default) or writable property names declared on the
/// class; `-all` unions in every class up the MRO (mixins + superclasses). The
/// result is sorted and duplicate-free.
fn info_class_properties(vm: &mut Vm, cls: &str, extra: &[Value]) -> Completion<Value> {
    let all = extra.iter().any(|v| v.to_str().as_ref() == "-all");
    let writable = extra.iter().any(|v| v.to_str().as_ref() == "-writable");
    let mut names: BTreeSet<String> = BTreeSet::new();
    if all {
        for step in vm.oo.class_linear_of(cls) {
            if let Some(c) = vm.oo.classes.get(&step.provider) {
                names.extend(slot_ref_class(c, writable).iter().cloned());
            }
        }
    } else if let Some(c) = vm.oo.classes.get(cls) {
        names.extend(slot_ref_class(c, writable).iter().cloned());
    }
    ok(Value::list(names.into_iter().map(Value::string).collect()))
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
fn info_class_methods(vm: &mut Vm, cls: &str, extra: &[Value]) -> Completion<Value> {
    let all = extra.iter().any(|v| v.to_str().as_ref() == "-all");
    let private = extra.iter().any(|v| v.to_str().as_ref() == "-private");
    let mut names: Vec<String> = if all {
        let mut s = vm.oo.visible_methods(cls, private);
        s.remove("create");
        s.remove("new");
        s.remove("destroy");
        s.into_iter().collect()
    } else {
        let c = &vm.oo.classes[cls];
        c.methods
            .iter()
            .filter(|(n, m)| private || m.exported || c.exported.contains(*n))
            .filter(|(n, _)| private || !c.unexported.contains(*n))
            .map(|(n, _)| n.clone())
            .collect()
    };
    names.sort();
    ok(Value::list(names.into_iter().map(Value::string).collect()))
}

/// Render a param list as an `info class constructor`-style arg spec (`args` is
/// already the last element when present).
fn params_value(params: &[Param]) -> Value {
    Value::list(
        params
            .iter()
            .map(|p| match &p.default {
                Some(d) => Value::list(vec![Value::string(p.name.clone()), d.clone()]),
                None => Value::string(p.name.clone()),
            })
            .collect(),
    )
}

/// Create a class named `class_key` (canonical), optionally running `body` as
/// its definition script.
pub(crate) fn make_class(vm: &mut Vm, class_key: &str, body: Option<&Value>) -> Completion<Value> {
    if vm.lookup_command(class_key).is_some() {
        return err(format!(
            "can't create object \"{}\": command already exists with that name",
            display(class_key)
        ));
    }
    let id = vm.oo.counter;
    vm.oo.counter += 1;
    vm.oo
        .classes
        .insert(class_key.to_string(), Class::default());
    vm.oo.objects.insert(
        class_key.to_string(),
        Object {
            class: "oo::class".to_string(),
            ns: class_key.to_string(),
            creation_id: id,
            methods: BTreeMap::new(),
            variables: Vec::new(),
            mixins: Vec::new(),
            exported: BTreeSet::new(),
            unexported: BTreeSet::new(),
            readable_properties: BTreeSet::new(),
            writable_properties: BTreeSet::new(),
            destroyed: false,
        },
    );
    vm.declare_namespace(class_key);
    vm.register_command(class_key, Command::Object(class_key.to_string()));
    if let Some(body) = body {
        let level = vm.current_level();
        vm.oo
            .def_stack
            .push((DefTarget::Class(class_key.to_string()), level));
        let res = match vm.eval_source(&body.to_str()) {
            Ok(c) if c.code == Code::Return => ok(c.result),
            Ok(c) => c,
            Err(e) => err(e.message),
        };
        vm.oo.def_stack.pop();
        if !res.code.is_ok() && res.code != Code::Return {
            teardown(vm, class_key);
            return res;
        }
    }
    ok(Value::string(display(class_key)))
}
