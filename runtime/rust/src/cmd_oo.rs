//! TclOO — the object system (`oo::class`, `oo::object`, `oo::define`, …).
//!
//! Covers (C ref `tclOO.c`): classes with single/multiple superclasses,
//! methods (incl. `forward`), constructor/destructor, instance variables,
//! object creation (`new`/`create`), per-object definitions (`oo::objdefine`,
//! per-object methods/mixins), class/object `mixin`s, `export`/`unexport`
//! visibility, `oo::copy`, method dispatch over a linearised chain (object →
//! object mixins → class mixins → class MRO), the method-context commands
//! `self`/`my`/`next`, and `info object`/`info class` introspection.
//!
//! Each object/class is a command (`Command::OoObject`); each object's instance
//! variables live in a private namespace, auto-linked into a method frame from
//! the class's `variable` declarations (reusing the proc machinery via
//! [`Interp::run_proc`] with `CallMeta::link_vars`).
//!
//! Deferred: filters, private methods/variables (8.7+), the full C3 mixin
//! linearisation, and `oo::define`'s rarer subcommands.
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
}

/// One active method invocation (for `self` / `my` / `next`).
struct OoFrame {
    object: Vec<u8>,
    /// The method-resolution chain (object FQN, then mixin/class FQNs).
    chain: Vec<Vec<u8>>,
    /// Index into `chain` of the provider whose method is currently running.
    index: usize,
    /// The method name (or empty for a constructor).
    method: Vec<u8>,
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
    match def_target(interp) {
        Ok(DefTarget::Class(c)) => {
            interp
                .oo
                .borrow_mut()
                .classes
                .get_mut(&c)
                .unwrap()
                .methods
                .insert(name, m);
        }
        Ok(DefTarget::Object(o)) => {
            interp
                .oo
                .borrow_mut()
                .objects
                .get_mut(&o)
                .unwrap()
                .methods
                .insert(name, m);
        }
        Err(code) => return code,
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
        (
            frame.object.clone(),
            frame.chain.get(frame.index).cloned().unwrap_or_default(),
            frame.method.clone(),
        )
    });
    let Some((object, class, method)) = ctx else {
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
            frame.method.clone(),
        )
    });
    let Some((object, chain, index, method)) = ctx else {
        return err(interp, b"next may only be called from inside a method");
    };
    let is_ctor = method.is_empty();
    let next =
        (index + 1..chain.len()).find(|&j| interp.oo_has_method(&chain[j], &method, is_ctor));
    match next {
        Some(j) => interp.oo_call(&object, chain, j, &method, &argv[1..]),
        None if is_ctor => {
            interp.set_result_bytes(b"");
            Code::Ok
        }
        None => err(interp, b"no next method implementation"),
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
        self.ns_register(fqn, Command::OoObject(fqn.to_vec()));
        if let Some(script) = script {
            self.oo
                .borrow_mut()
                .def_stack
                .push(DefTarget::Class(fqn.to_vec()));
            let code = self.eval_str(script);
            self.oo.borrow_mut().def_stack.pop();
            if code != Code::Ok {
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
                Some(other) => unknown_method(self, other),
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
        if let Some(j) = mro.iter().position(|c| {
            self.oo
                .borrow()
                .classes
                .get(c)
                .is_some_and(|c| c.constructor.is_some())
        }) {
            // Constructor dispatch runs along the *class* MRO (objects can't
            // define constructors), with the object as `self`.
            let chain = mro;
            let code = self.oo_call(&fqn, chain, j, b"", args);
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
        let chain = self.method_chain(obj);
        let found = chain.iter().position(|c| {
            self.oo_has_method(c, method, false) && !(external && self.method_unexported(c, method))
        });
        match found {
            Some(j) => self.oo_call(obj, chain, j, method, args),
            None => unknown_method(self, method),
        }
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
    fn oo_has_method(&self, prov: &[u8], method: &[u8], ctor: bool) -> bool {
        if ctor {
            return self
                .oo
                .borrow()
                .classes
                .get(prov)
                .is_some_and(|c| c.constructor.is_some());
        }
        if let Some(o) = self.oo.borrow().objects.get(prov) {
            o.methods.contains_key(method)
        } else {
            self.oo
                .borrow()
                .classes
                .get(prov)
                .is_some_and(|c| c.methods.contains_key(method))
        }
    }

    fn method_unexported(&self, prov: &[u8], method: &[u8]) -> bool {
        if let Some(o) = self.oo.borrow().objects.get(prov) {
            o.unexported.contains(method)
        } else {
            self.oo
                .borrow()
                .classes
                .get(prov)
                .is_some_and(|c| c.unexported.contains(method))
        }
    }

    /// Run the method (or constructor when `method` is empty) provided by
    /// `chain[index]` on `obj`.
    fn oo_call(
        &mut self,
        obj: &[u8],
        chain: Vec<Vec<u8>>,
        index: usize,
        method: &[u8],
        args: &[*mut TclObj],
    ) -> Code {
        let prov = chain[index].clone();
        let m = if method.is_empty() {
            self.oo
                .borrow()
                .classes
                .get(&prov)
                .and_then(|c| c.constructor.clone())
        } else if let Some(o) = self.oo.borrow().objects.get(&prov) {
            o.methods.get(method).cloned()
        } else {
            self.oo
                .borrow()
                .classes
                .get(&prov)
                .and_then(|c| c.methods.get(method).cloned())
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
                method: method.to_vec(),
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
        let var_ns = self.oo.borrow().objects[obj].var_ns;
        // The declared instance variables visible to the method: union over the
        // chain's classes.
        let mut vars: Vec<Vec<u8>> = Vec::new();
        for c in &chain {
            if let Some(cl) = self.oo.borrow().classes.get(c) {
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
            method.to_vec()
        };
        self.oo.borrow_mut().call_stack.push(OoFrame {
            object: obj.to_vec(),
            chain,
            index,
            method: method.to_vec(),
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
                if let Some(body) = dtor {
                    let var_ns = self.oo.borrow().objects[obj].var_ns;
                    self.oo.borrow_mut().call_stack.push(OoFrame {
                        object: obj.to_vec(),
                        chain: mro.clone(),
                        index: 0,
                        method: b"<destructor>".to_vec(),
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
        // Destroy the class's instances first (TclOO cascades).
        let insts: Vec<Vec<u8>> = self
            .oo
            .borrow()
            .objects
            .iter()
            .filter(|(_, o)| o.class == class)
            .map(|(k, _)| k.clone())
            .collect();
        for o in insts {
            self.oo_destroy(&o);
        }
        self.oo.borrow_mut().classes.remove(class);
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
