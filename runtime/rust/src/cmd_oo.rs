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

use crate::interp::{obj_bytes, CallMeta, Code, Command, Interp, Param, ProcFrame};
use crate::list;
use crate::namespace::NsId;
use crate::obj::{self, TclObj};

/// A method: a normal body, or a `forward` to a command prefix.
#[derive(Clone)]
enum Method {
    Body { params: Vec<Param>, body: Vec<u8> },
    Forward { prefix: Vec<Vec<u8>> },
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
    /// Mixed-in classes (searched before the superclass MRO).
    mixins: Vec<Vec<u8>>,
    /// Methods marked non-exported (callable only via `my`).
    unexported: BTreeSet<Vec<u8>>,
    /// Filter method names applied to instances' method calls (`filter`).
    filters: Vec<Vec<u8>>,
}

/// An object instance.
#[derive(Default, Clone)]
struct Object {
    /// FQN of the object's class.
    class: Vec<u8>,
    /// The namespace holding this object's instance variables.
    var_ns: NsId,
    /// Per-object methods (`oo::objdefine method`).
    methods: BTreeMap<Vec<u8>, Method>,
    /// Per-object mixins.
    mixins: Vec<Vec<u8>>,
    unexported: BTreeSet<Vec<u8>>,
    /// Per-object filter method names (`oo::objdefine filter`).
    filters: Vec<Vec<u8>>,
}

/// One step of a method-call chain: the provider (object or class FQN) and the
/// method name to run there. Filters appear as steps whose `method` is the
/// filter's name, ahead of the steps for the actually-invoked method.
#[derive(Clone)]
struct CallStep {
    provider: Vec<u8>,
    method: Vec<u8>,
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
    def_stack: Vec<DefTarget>,
    call_stack: Vec<OoFrame>,
}

/// Register the `oo::*` commands and the definition / context commands.
pub fn install(interp: &mut Interp) {
    interp.register_builtin(b"oo::class", oo_class_cmd);
    interp.register_builtin(b"oo::object", oo_object_cmd);
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
    // Method-context commands.
    interp.register_builtin(b"self", self_cmd);
    interp.register_builtin(b"my", my_cmd);
    interp.register_builtin(b"next", next_cmd);
    // Root classes (so `superclass`-less classes inherit `object` and
    // `superclass oo::class`/`oo::object` validate; `oo::class` keeps its
    // builtin command — only the registry entry is added).
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
    let _ =
        interp.eval_str(b"namespace eval ::oo {variable version 1.3.1; variable patchlevel 1.3.1}");
}

fn err(interp: &mut Interp, msg: &[u8]) -> Code {
    interp.set_error(msg)
}

fn wrong_args(interp: &mut Interp, usage: &[u8]) -> Code {
    let mut m = b"wrong # args: should be \"".to_vec();
    m.extend_from_slice(usage);
    m.push(b'"');
    interp.set_error(&m)
}

fn unknown_method(interp: &mut Interp, m: &[u8]) -> Code {
    let mut msg = b"unknown method \"".to_vec();
    msg.extend_from_slice(m);
    msg.extend_from_slice(b"\": must be a supported method");
    interp.set_error(&msg)
}

// -- oo::class / oo::object / oo::define / oo::objdefine ---------------------

/// `oo::class create name ?definitionScript?` — define a class.
fn oo_class_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    match argv.get(1).map(|&a| obj_bytes(a)).as_deref() {
        Some(b"create") if argv.len() >= 3 => {
            let fqn = interp.fqn_for(&obj_bytes(argv[2]));
            interp.oo_make_class(&fqn, argv.get(3).map(|&a| obj_bytes(a)).as_deref())
        }
        Some(b"new") => {
            // Anonymous class.
            let n = interp.oo.borrow().counter;
            interp.oo.borrow_mut().counter += 1;
            let fqn = format!("::oo::Obj{n}").into_bytes();
            interp.oo_make_class(&fqn, argv.get(2).map(|&a| obj_bytes(a)).as_deref())
        }
        _ => wrong_args(interp, b"oo::class create name ?definitionScript?"),
    }
}

/// `oo::object create name` / `oo::object new` — create a bare object.
fn oo_object_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    match argv.get(1).map(|&a| obj_bytes(a)).as_deref() {
        Some(b"create") if argv.len() >= 3 => {
            let name = interp.fqn_for(&obj_bytes(argv[2]));
            interp.oo_new(b"::oo::object", Some(name), &argv[3..])
        }
        Some(b"new") => interp.oo_new(b"::oo::object", None, &argv[2..]),
        _ => wrong_args(interp, b"oo::object create|new ?arg ...?"),
    }
}

/// `oo::define class script` or `oo::define class subcommand ?arg ...?`.
fn oo_define_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 3 {
        return wrong_args(interp, b"oo::define target ?arg ...?");
    }
    let fqn = interp.fqn_for(&obj_bytes(argv[1]));
    if !interp.oo.borrow().classes.contains_key(&fqn) {
        let mut m = b"\"".to_vec();
        m.extend_from_slice(&obj_bytes(argv[1]));
        m.extend_from_slice(b"\" does not refer to a class");
        return err(interp, &m);
    }
    interp.oo_run_def(DefTarget::Class(fqn), argv)
}

/// `oo::objdefine object script` / `oo::objdefine object subcommand ?arg ...?`.
fn oo_objdefine_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 3 {
        return wrong_args(interp, b"oo::objdefine target ?arg ...?");
    }
    let fqn = interp.fqn_for(&obj_bytes(argv[1]));
    if !interp.oo.borrow().objects.contains_key(&fqn) {
        let mut m = b"\"".to_vec();
        m.extend_from_slice(&obj_bytes(argv[1]));
        m.extend_from_slice(b"\" does not refer to an object");
        return err(interp, &m);
    }
    interp.oo_run_def(DefTarget::Object(fqn), argv)
}

/// `oo::copy srcObject ?targetObject?` — clone an object (its class + per-object
/// methods/mixins).
fn oo_copy_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 || argv.len() > 3 {
        return wrong_args(interp, b"oo::copy sourceName ?targetName?");
    }
    let src = interp.fqn_for(&obj_bytes(argv[1]));
    let Some(src_obj) = interp.oo.borrow().objects.get(&src).cloned() else {
        let mut m = b"\"".to_vec();
        m.extend_from_slice(&obj_bytes(argv[1]));
        m.extend_from_slice(b"\" does not refer to an object");
        return err(interp, &m);
    };
    let dst = match argv.get(2) {
        Some(&a) => interp.fqn_for(&obj_bytes(a)),
        None => {
            let n = interp.oo.borrow().counter;
            interp.oo.borrow_mut().counter += 1;
            format!("::oo::Obj{n}").into_bytes()
        }
    };
    let var_ns = interp.ensure_namespace(&dst);
    interp.oo.borrow_mut().objects.insert(
        dst.clone(),
        Object {
            class: src_obj.class,
            var_ns,
            methods: src_obj.methods,
            mixins: src_obj.mixins,
            unexported: src_obj.unexported,
            filters: src_obj.filters,
        },
    );
    interp.ns_register(&dst, Command::OoObject(dst.clone()));
    interp.set_result(obj::new_string_bytes(&dst));
    Code::Ok
}

// -- definition-script commands ---------------------------------------------

/// The current definition target, or an error if outside a definition body.
fn def_target(interp: &mut Interp) -> Result<DefTarget, Code> {
    let target = interp.oo.borrow().def_stack.last().cloned();
    match target {
        Some(t) => Ok(t),
        None => Err(interp
            .set_error(b"this command can only be called from within the body of a definition")),
    }
}

/// Install `method` into the current definition target (class or object).
fn install_method(interp: &mut Interp, name: Vec<u8>, m: Method) -> Code {
    let ok = match def_target(interp) {
        Ok(DefTarget::Class(c)) => {
            let mut oo = interp.oo.borrow_mut();
            oo.classes
                .get_mut(&c)
                .map(|cl| cl.methods.insert(name, m))
                .is_some()
        }
        Ok(DefTarget::Object(o)) => {
            let mut oo = interp.oo.borrow_mut();
            oo.objects
                .get_mut(&o)
                .map(|ob| ob.methods.insert(name, m))
                .is_some()
        }
        Err(code) => return code,
    };
    if !ok {
        return interp.set_error(b"no current class/object to define on");
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

fn def_method(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 4 {
        return wrong_args(interp, b"method name args body");
    }
    let params = match crate::cmd_proc::parse_params(&obj_bytes(argv[2])) {
        Ok(p) => p,
        Err(e) => return err(interp, &e),
    };
    install_method(
        interp,
        obj_bytes(argv[1]),
        Method::Body {
            params,
            body: obj_bytes(argv[3]),
        },
    )
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
    interp
        .oo
        .borrow_mut()
        .classes
        .get_mut(&class)
        .unwrap()
        .constructor = Some(Method::Body {
        params,
        body: obj_bytes(argv[2]),
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
    let supers: Vec<Vec<u8>> = argv[1..]
        .iter()
        .map(|&a| interp.fqn_for(&obj_bytes(a)))
        .collect();
    for s in &supers {
        if !interp.oo.borrow().classes.contains_key(s) {
            let mut m = b"\"".to_vec();
            m.extend_from_slice(s);
            m.extend_from_slice(b"\" does not refer to a class");
            return err(interp, &m);
        }
    }
    interp
        .oo
        .borrow_mut()
        .classes
        .get_mut(&class)
        .unwrap()
        .supers = if supers.is_empty() {
        vec![b"::oo::object".to_vec()]
    } else {
        supers
    };
    interp.set_result_bytes(b"");
    Code::Ok
}

/// `mixin ?class ...?` — set the mixins of the current class/object.
/// `filter ?methodName ...?` — set the filter methods on the def target (class
/// or object). Filters wrap every public method call on instances.
fn def_filter(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let filters: Vec<Vec<u8>> = argv[1..].iter().map(|&a| obj_bytes(a)).collect();
    match def_target(interp) {
        Ok(DefTarget::Class(c)) => {
            if let Some(cl) = interp.oo.borrow_mut().classes.get_mut(&c) {
                cl.filters = filters;
            }
        }
        Ok(DefTarget::Object(o)) => {
            if let Some(ob) = interp.oo.borrow_mut().objects.get_mut(&o) {
                ob.filters = filters;
            }
        }
        Err(code) => return code,
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

fn def_mixin(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let mixins: Vec<Vec<u8>> = argv[1..]
        .iter()
        .map(|&a| interp.fqn_for(&obj_bytes(a)))
        .collect();
    for mx in &mixins {
        if !interp.oo.borrow().classes.contains_key(mx) {
            let mut m = b"\"".to_vec();
            m.extend_from_slice(mx);
            m.extend_from_slice(b"\" does not refer to a class");
            return err(interp, &m);
        }
    }
    match def_target(interp) {
        Ok(DefTarget::Class(c)) => {
            interp.oo.borrow_mut().classes.get_mut(&c).unwrap().mixins = mixins
        }
        Ok(DefTarget::Object(o)) => {
            interp.oo.borrow_mut().objects.get_mut(&o).unwrap().mixins = mixins
        }
        Err(code) => return code,
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

fn def_variable(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    // Outside a definition body, this is the ordinary `variable` command.
    if interp.oo.borrow().def_stack.is_empty() {
        return crate::cmd_var::variable(interp, argv);
    }
    let names: Vec<Vec<u8>> = argv[1..].iter().map(|&a| obj_bytes(a)).collect();
    match def_target(interp) {
        Ok(DefTarget::Class(c)) => {
            let mut oo = interp.oo.borrow_mut();
            let cl = oo.classes.get_mut(&c).unwrap();
            for n in names {
                if !cl.variables.contains(&n) {
                    cl.variables.push(n);
                }
            }
        }
        // Per-object `variable` declarations are not tracked separately yet;
        // accept them (they behave like locals).
        Ok(DefTarget::Object(_)) => {}
        Err(code) => return code,
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

/// `export`/`unexport name ...` — set method visibility on the current target.
fn def_export(interp: &mut Interp, argv: &[*mut TclObj], export: bool) -> Code {
    let names: Vec<Vec<u8>> = argv[1..].iter().map(|&a| obj_bytes(a)).collect();
    let set = |s: &mut BTreeSet<Vec<u8>>| {
        for n in &names {
            if export {
                s.remove(n);
            } else {
                s.insert(n.clone());
            }
        }
    };
    match def_target(interp) {
        Ok(DefTarget::Class(c)) => set(&mut interp
            .oo
            .borrow_mut()
            .classes
            .get_mut(&c)
            .unwrap()
            .unexported),
        Ok(DefTarget::Object(o)) => set(&mut interp
            .oo
            .borrow_mut()
            .objects
            .get_mut(&o)
            .unwrap()
            .unexported),
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
        let def_target = interp.oo.borrow().def_stack.last().cloned();
        if let Some(target) = def_target {
            let tfqn = match target {
                DefTarget::Class(c) | DefTarget::Object(c) => c,
            };
            if argv.len() == 1 {
                interp.set_result(obj::new_string_bytes(&tfqn));
                return Code::Ok;
            }
            interp
                .oo
                .borrow_mut()
                .def_stack
                .push(DefTarget::Object(tfqn));
            let code = interp.dispatch(&argv[1..]);
            interp.oo.borrow_mut().def_stack.pop();
            return code;
        }
        return err(interp, b"self may only be called from inside a method");
    };
    match argv.get(1).map(|&a| obj_bytes(a)).as_deref() {
        None | Some(b"object") => interp.set_result(obj::new_string_bytes(&object)),
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
            match pair {
                Some((p, m)) => {
                    let objs = [obj::new_string_bytes(&p), obj::new_string_bytes(&m)];
                    interp.set_result(crate::list::new_list_obj(&objs));
                }
                None => interp.set_result_bytes(b""),
            }
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

fn my_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 {
        return wrong_args(interp, b"my methodName ?arg ...?");
    }
    let Some(object) = interp
        .oo
        .borrow()
        .call_stack
        .last()
        .map(|f| f.object.clone())
    else {
        return err(interp, b"my may only be called from inside a method");
    };
    let method = obj_bytes(argv[1]);
    interp.oo_invoke(&object, &method, &argv[2..], false)
}

fn next_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let ctx = interp.oo.borrow().call_stack.last().map(|frame| {
        (
            frame.object.clone(),
            frame.chain.clone(),
            frame.index,
            frame.target.clone(),
        )
    });
    let Some((object, chain, index, target)) = ctx else {
        return err(interp, b"next may only be called from inside a method");
    };
    // The chain is pre-built (filters then the method steps), so `next` simply
    // advances to the following step.
    if index + 1 < chain.len() {
        interp.oo_run(&object, chain, index + 1, &target, &argv[1..])
    } else if target.is_empty() {
        // Past the last constructor in the chain — a no-op (C's default).
        interp.set_result_bytes(b"");
        Code::Ok
    } else {
        err(interp, b"no next method implementation")
    }
}

// -- info object / info class (called from cmd_info) -------------------------

/// `info object subcommand object ?arg?`.
pub(crate) fn info_object(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 4 {
        return wrong_args(interp, b"info object subcommand objName ?arg ...?");
    }
    let sub = obj_bytes(argv[2]);
    let obj = interp.fqn_for(&obj_bytes(argv[3]));
    match sub.as_slice() {
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
            interp.set_result(obj::new_string_bytes(&class));
            Code::Ok
        }
        b"isa" => {
            // info object isa category objName ?arg?
            let cat = obj_bytes(argv[3]);
            let target = interp.fqn_for(&obj_bytes(argv[4]));
            let yes = match cat.as_slice() {
                b"object" => interp.oo.borrow().objects.contains_key(&target),
                b"class" => interp.oo.borrow().classes.contains_key(&target),
                b"typeof" => {
                    let want = interp.fqn_for(&obj_bytes(argv[5]));
                    interp
                        .oo
                        .borrow()
                        .objects
                        .get(&target)
                        .map(|o| o.class.clone())
                        .is_some_and(|c| interp.mro(&c).contains(&want))
                }
                _ => false,
            };
            interp.set_result_bytes(if yes { b"1" } else { b"0" });
            Code::Ok
        }
        b"vars" | b"variables" => {
            let ns = interp.oo.borrow().objects.get(&obj).map(|o| o.var_ns);
            let Some(ns) = ns else {
                return not_object(interp, &obj_bytes(argv[3]));
            };
            let mut names = interp.namespaces().var_names(ns);
            names.sort();
            set_list(interp, &names);
            Code::Ok
        }
        b"methods" => {
            let names: Option<Vec<Vec<u8>>> = interp
                .oo
                .borrow()
                .objects
                .get(&obj)
                .map(|o| o.methods.keys().cloned().collect());
            let Some(mut names) = names else {
                return not_object(interp, &obj_bytes(argv[3]));
            };
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
        other => {
            let mut m = b"unknown info object subcommand \"".to_vec();
            m.extend_from_slice(other);
            m.push(b'"');
            err(interp, &m)
        }
    }
}

/// `info class subcommand class ?arg?`.
pub(crate) fn info_class(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 4 {
        return wrong_args(interp, b"info class subcommand className ?arg ...?");
    }
    let sub = obj_bytes(argv[2]);
    let cls = interp.fqn_for(&obj_bytes(argv[3]));
    if !interp.oo.borrow().classes.contains_key(&cls) {
        let mut m = b"\"".to_vec();
        m.extend_from_slice(&obj_bytes(argv[3]));
        m.extend_from_slice(b"\" is not a class");
        return err(interp, &m);
    }
    match sub.as_slice() {
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
            let v = interp.oo.borrow().classes[&cls].variables.clone();
            set_list(interp, &v);
            Code::Ok
        }
        b"instances" => {
            let mut insts: Vec<Vec<u8>> = interp
                .oo
                .borrow()
                .objects
                .iter()
                .filter(|(_, o)| o.class == cls)
                .map(|(k, _)| k.clone())
                .collect();
            insts.sort();
            set_list(interp, &insts);
            Code::Ok
        }
        b"subclasses" => {
            let mut subs: Vec<Vec<u8>> = interp
                .oo
                .borrow()
                .classes
                .iter()
                .filter(|(k, c)| **k != cls && c.supers.contains(&cls))
                .map(|(k, _)| k.clone())
                .collect();
            subs.sort();
            set_list(interp, &subs);
            Code::Ok
        }
        b"methods" => {
            let all = argv[4..].iter().any(|&a| obj_bytes(a) == b"-all");
            let private = argv[4..].iter().any(|&a| obj_bytes(a) == b"-private");
            let mut names: Vec<Vec<u8>> = Vec::new();
            let chain = if all {
                interp.mro(&cls)
            } else {
                vec![cls.clone()]
            };
            for c in &chain {
                if let Some(cl) = interp.oo.borrow().classes.get(c) {
                    for n in cl.methods.keys() {
                        if (private || !cl.unexported.contains(n)) && !names.contains(n) {
                            names.push(n.clone());
                        }
                    }
                }
            }
            names.sort();
            set_list(interp, &names);
            Code::Ok
        }
        b"constructor" => {
            let body = match &interp.oo.borrow().classes[&cls].constructor {
                Some(Method::Body { params, body }) => list_params_body(params, body),
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
        other => {
            let mut m = b"unknown info class subcommand \"".to_vec();
            m.extend_from_slice(other);
            m.push(b'"');
            err(interp, &m)
        }
    }
}

fn not_object(interp: &mut Interp, name: &[u8]) -> Code {
    let mut m = b"\"".to_vec();
    m.extend_from_slice(name);
    m.extend_from_slice(b"\" does not refer to an object");
    interp.set_error(&m)
}

/// `{params} body` as a 2-element list (for `info class constructor`).
fn list_params_body(params: &[Param], body: &[u8]) -> Vec<u8> {
    let mut spec = Vec::new();
    for (i, p) in params.iter().enumerate() {
        if i > 0 {
            spec.push(b' ');
        }
        spec.extend_from_slice(&p.name);
    }
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
        let mut oo = self.oo.borrow_mut();
        // A class is registered in *both* maps (a class is also an object), so
        // move/remove from both to avoid a dangling half-entry.
        let obj = oo.objects.remove(old_fqn);
        let cls = oo.classes.remove(old_fqn);
        if let Some(nf) = new_fqn {
            if let Some(o) = obj {
                oo.objects.insert(nf.to_vec(), o);
            }
            if let Some(c) = cls {
                oo.classes.insert(nf.to_vec(), c);
            }
        }
    }

    /// Whether the OO registry is empty (the rename hot-path early-out).
    pub(crate) fn oo_is_empty(&self) -> bool {
        let oo = self.oo.borrow();
        oo.objects.is_empty() && oo.classes.is_empty()
    }

    /// Create class `fqn` (running its optional definition script).
    fn oo_make_class(&mut self, fqn: &[u8], script: Option<&[u8]>) -> Code {
        if self.oo.borrow().classes.contains_key(fqn) || self.oo.borrow().objects.contains_key(fqn)
        {
            let mut m = b"can't create class \"".to_vec();
            m.extend_from_slice(fqn);
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
        self.oo.borrow_mut().objects.insert(
            fqn.to_vec(),
            Object {
                class: b"::oo::class".to_vec(),
                var_ns,
                ..Object::default()
            },
        );
        self.ns_register(fqn, Command::OoObject(fqn.to_vec()));
        if let Some(script) = script {
            self.oo
                .borrow_mut()
                .def_stack
                .push(DefTarget::Class(fqn.to_vec()));
            let code = self.eval_str(script);
            self.oo.borrow_mut().def_stack.pop();
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
        self.oo.borrow_mut().def_stack.push(target);
        let code = if argv.len() == 3 {
            self.eval_str(&obj_bytes(argv[2]))
        } else {
            self.dispatch(&argv[2..])
        };
        self.oo.borrow_mut().def_stack.pop();
        code
    }

    /// Dispatch a command bound to the OO object/class FQN `fqn`.
    pub(crate) fn oo_dispatch(&mut self, fqn: &[u8], argv: &[*mut TclObj]) -> Code {
        if self.oo.borrow().classes.contains_key(fqn) {
            match argv.get(1).map(|&a| obj_bytes(a)).as_deref() {
                Some(b"new") => self.oo_new(fqn, None, &argv[2..]),
                Some(b"create") if argv.len() >= 3 => {
                    let name = self.fqn_for(&obj_bytes(argv[2]));
                    self.oo_new(fqn, Some(name), &argv[3..])
                }
                Some(b"destroy") => {
                    self.oo_destroy_class(fqn);
                    self.set_result_bytes(b"");
                    Code::Ok
                }
                // Any other subcommand is a class-object method (defined via
                // `oo::define … self method`); dispatch it on the class object.
                Some(other) => self.oo_invoke(fqn, other, &argv[2..], true),
                None => self.error(b"wrong # args: should be \"class method ?arg ...?\""),
            }
        } else if self.oo.borrow().objects.contains_key(fqn) {
            match argv.get(1).map(|&a| obj_bytes(a)) {
                Some(m) if m == b"destroy" => {
                    self.oo_destroy(fqn);
                    self.set_result_bytes(b"");
                    Code::Ok
                }
                Some(method) => self.oo_invoke(fqn, &method, &argv[2..], true),
                None => self.error(b"wrong # args: should be \"object method ?arg ...?\""),
            }
        } else {
            self.invalid_command(fqn)
        }
    }

    fn oo_new(&mut self, class: &[u8], name: Option<Vec<u8>>, args: &[*mut TclObj]) -> Code {
        let fqn = name.unwrap_or_else(|| {
            let n = format!("::oo::Obj{}", self.oo.borrow().counter);
            self.oo.borrow_mut().counter += 1;
            n.into_bytes()
        });
        if self.oo.borrow().objects.contains_key(&fqn)
            || self.oo.borrow().classes.contains_key(&fqn)
        {
            let mut m = b"can't create object \"".to_vec();
            m.extend_from_slice(&fqn);
            m.extend_from_slice(b"\": command already exists with that name");
            return self.error(&m);
        }
        let var_ns = self.ensure_namespace(&fqn);
        self.oo.borrow_mut().objects.insert(
            fqn.clone(),
            Object {
                class: class.to_vec(),
                var_ns,
                ..Object::default()
            },
        );
        self.ns_register(&fqn, Command::OoObject(fqn.clone()));

        let mro = self.mro(class);
        if mro.iter().any(|c| self.class_has_ctor(c)) {
            // Constructor dispatch runs along the *class* MRO (objects can't
            // define constructors), with the object as `self`. The chain is the
            // constructor-providing classes in MRO order (so `next` chains).
            let chain: Vec<CallStep> = mro
                .iter()
                .filter(|c| self.class_has_ctor(c))
                .map(|c| CallStep {
                    provider: c.clone(),
                    method: Vec::new(),
                })
                .collect();
            let code = self.oo_run(&fqn, chain, 0, b"", args);
            if code == Code::Error {
                self.oo.borrow_mut().objects.remove(&fqn);
                self.delete_command(&fqn);
                return Code::Error;
            }
        } else if !args.is_empty() {
            return self.error(b"object creation takes no arguments (no constructor)");
        }
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
        let providers = self.method_chain(obj);
        // The target-method steps: every provider that defines `method`.
        let mut steps: Vec<CallStep> = providers
            .iter()
            .filter(|p| {
                self.oo_has_method(p, method, p.as_slice() == obj)
                    && !(external && self.method_unexported(p, method, p.as_slice() == obj))
            })
            .map(|p| CallStep {
                provider: p.clone(),
                method: method.to_vec(),
            })
            .collect();
        if steps.is_empty() {
            return unknown_method(self, method);
        }
        // Filters wrap an *external* (public) call: prepend each active filter
        // method as its own step (resolved to the provider defining it).
        if external {
            let filters = self.active_filters(obj, &providers);
            let mut chain: Vec<CallStep> = filters;
            chain.append(&mut steps);
            return self.oo_run(obj, chain, 0, method, args);
        }
        self.oo_run(obj, steps, 0, method, args)
    }

    /// The active filter steps for `obj` (object filters, then class filters
    /// along the MRO), each resolved to the chain provider defining it.
    fn active_filters(&self, obj: &[u8], providers: &[Vec<u8>]) -> Vec<CallStep> {
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
        for p in providers {
            if let Some(c) = self.oo.borrow().classes.get(p) {
                for f in &c.filters {
                    add(f, &mut names);
                }
            }
        }
        // Resolve each filter name to the first provider defining it as a method.
        names
            .iter()
            .filter_map(|fname| {
                providers
                    .iter()
                    .find(|p| self.oo_has_method(p, fname, p.as_slice() == obj))
                    .map(|p| CallStep {
                        provider: p.clone(),
                        method: fname.clone(),
                    })
            })
            .collect()
    }

    /// The method-resolution chain for `obj`: the object's own methods, then its
    /// mixins, then the class's mixins, then the class MRO (deduped, first wins).
    fn method_chain(&self, obj: &[u8]) -> Vec<Vec<u8>> {
        let mut chain: Vec<Vec<u8>> = vec![obj.to_vec()];
        let push = |c: &[u8], chain: &mut Vec<Vec<u8>>| {
            if !chain.iter().any(|x| x == c) {
                chain.push(c.to_vec());
            }
        };
        if let Some(o) = self.oo.borrow().objects.get(obj) {
            for mx in &o.mixins {
                for c in self.mro(mx) {
                    push(&c, &mut chain);
                }
            }
            let cls = o.class.clone();
            if let Some(cl) = self.oo.borrow().classes.get(&cls) {
                for mx in &cl.mixins {
                    for c in self.mro(mx) {
                        push(&c, &mut chain);
                    }
                }
            }
            for c in self.mro(&cls) {
                push(&c, &mut chain);
            }
        }
        chain
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

    /// Whether the class `prov` defines a constructor.
    fn class_has_ctor(&self, prov: &[u8]) -> bool {
        self.oo
            .borrow()
            .classes
            .get(prov)
            .is_some_and(|c| c.constructor.is_some())
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
    ) -> Code {
        let prov = chain[index].provider.clone();
        let method = chain[index].method.clone();
        // Object-vs-class resolution is by identity (a class is in both maps).
        let is_object = prov == obj;
        let m = if method.is_empty() {
            self.oo
                .borrow()
                .classes
                .get(&prov)
                .and_then(|c| c.constructor.clone())
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

        // A forward: build `prefix + args` and dispatch (with the OO context so a
        // forwarded `my`/`self` still works).
        if let Method::Forward { prefix } = &m {
            let mut new_argv: Vec<*mut TclObj> = Vec::with_capacity(prefix.len() + args.len());
            // Each element owns a +1 (released in the `decr` loop below).
            for p in prefix {
                let o = obj::new_string_bytes(p);
                unsafe { obj::incr_ref_count(o) };
                new_argv.push(o);
            }
            for &a in args {
                unsafe { obj::incr_ref_count(a) };
                new_argv.push(a);
            }
            self.oo.borrow_mut().call_stack.push(OoFrame {
                object: obj.to_vec(),
                chain,
                index,
                target: target.to_vec(),
            });
            let code = self.dispatch(&new_argv);
            self.oo.borrow_mut().call_stack.pop();
            for a in new_argv {
                unsafe { obj::decr_ref_count(a) };
            }
            return code;
        }

        let Method::Body { params, body } = m else {
            unreachable!("forward handled above");
        };
        let Some(var_ns) = self.oo.borrow().objects.get(obj).map(|o| o.var_ns) else {
            let mut m = b"object \"".to_vec();
            m.extend_from_slice(obj);
            m.extend_from_slice(b"\" has been deleted");
            return self.error(&m);
        };
        // The declared instance variables visible to the method: union over the
        // object's full class hierarchy (independent of the call chain, which a
        // filter may have reordered).
        let mut vars: Vec<Vec<u8>> = Vec::new();
        for c in self.method_chain(obj) {
            if let Some(cl) = self.oo.borrow().classes.get(&c) {
                for v in &cl.variables {
                    if !vars.contains(v) {
                        vars.push(v.clone());
                    }
                }
            }
        }
        let name = if method.is_empty() {
            b"<constructor>".to_vec()
        } else {
            method.clone()
        };
        self.oo.borrow_mut().call_stack.push(OoFrame {
            object: obj.to_vec(),
            chain,
            index,
            target: target.to_vec(),
        });
        let code = self.run_proc(
            &params,
            &body,
            var_ns,
            args,
            &name,
            CallMeta {
                err: ProcFrame::Proc(&name),
                fqn: None,
                source: None,
                body_line_base: 0,
                link_vars: &vars,
            },
        );
        self.oo.borrow_mut().call_stack.pop();
        code
    }

    fn oo_destroy(&mut self, obj: &[u8]) {
        let class = self.oo.borrow().objects.get(obj).map(|o| o.class.clone());
        if let Some(class) = class {
            let mro = self.mro(&class);
            for c in &mro {
                let dtor = self
                    .oo
                    .borrow()
                    .classes
                    .get(c)
                    .and_then(|c| c.destructor.clone());
                let var_ns = self.oo.borrow().objects.get(obj).map(|o| o.var_ns);
                if let (Some(body), Some(var_ns)) = (dtor, var_ns) {
                    self.oo.borrow_mut().call_stack.push(OoFrame {
                        object: obj.to_vec(),
                        chain: vec![CallStep {
                            provider: c.clone(),
                            method: b"<destructor>".to_vec(),
                        }],
                        index: 0,
                        target: b"<destructor>".to_vec(),
                    });
                    let _ = self.run_proc(
                        &[],
                        &body,
                        var_ns,
                        &[],
                        b"<destructor>",
                        CallMeta {
                            err: ProcFrame::Proc(b"<destructor>"),
                            fqn: None,
                            source: None,
                            body_line_base: 0,
                            link_vars: &[],
                        },
                    );
                    self.oo.borrow_mut().call_stack.pop();
                    break;
                }
            }
        }
        self.oo.borrow_mut().objects.remove(obj);
        self.delete_command(obj);
    }

    fn oo_destroy_class(&mut self, class: &[u8]) {
        // Destroying a class cascades to its subclasses and instances (TclOO).
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
            .collect();
        for s in subs {
            self.oo_destroy_class(&s);
        }
        // Then this class's direct instances.
        let insts: Vec<Vec<u8>> = self
            .oo
            .borrow()
            .objects
            .iter()
            .filter(|(k, o)| k.as_slice() != class && o.class == class)
            .map(|(k, _)| k.clone())
            .collect();
        for o in insts {
            self.oo_destroy(&o);
        }
        self.oo.borrow_mut().classes.remove(class);
        self.oo.borrow_mut().objects.remove(class);
        self.delete_command(class);
    }

    /// The method-resolution order for `class`: a preorder walk of the class and
    /// its superclasses, each class appearing once.
    fn mro(&self, class: &[u8]) -> Vec<Vec<u8>> {
        let mut out: Vec<Vec<u8>> = Vec::new();
        self.mro_visit(class, &mut out);
        out
    }

    fn mro_visit(&self, class: &[u8], out: &mut Vec<Vec<u8>>) {
        if out.iter().any(|c| c == class) {
            return;
        }
        out.push(class.to_vec());
        if let Some(c) = self.oo.borrow().classes.get(class) {
            let supers = c.supers.clone();
            for s in &supers {
                self.mro_visit(s, out);
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
}
