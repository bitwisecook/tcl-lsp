//! TclOO — the object system (`oo::class`, `oo::object`, `oo::define`, …).
//!
//! A core subset (C ref `tclOO.c`): classes with single/multiple superclasses,
//! methods, constructor/destructor, instance variables, object creation
//! (`new`/`create`), method dispatch with a linearised MRO, and the
//! method-context commands `self` / `my` / `next`. Each object/class is a
//! command (`Command::OoObject`); each object's instance variables live in a
//! private namespace, auto-linked into a method frame from the class's
//! `variable` declarations (reusing the proc-call machinery via
//! [`Interp::run_proc`] with `CallMeta::link_vars`).
//!
//! Deferred: mixins, filters, forwards, `oo::copy`, per-object methods
//! (`oo::objdefine`), private methods/variables, and `info object`/`info class`
//! introspection.
//!
//! See `list.rs` for the module-level `not_unsafe_ptr_arg_deref` rationale.
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use std::collections::BTreeMap;

use crate::interp::{obj_bytes, CallMeta, Code, Command, Interp, Param, ProcFrame};
use crate::namespace::NsId;
use crate::obj::{self, TclObj};

/// A method (or constructor): parsed parameters + the body script.
#[derive(Clone)]
struct Method {
    params: Vec<Param>,
    body: Vec<u8>,
}

/// A class definition.
#[derive(Default)]
struct Class {
    /// Superclass FQNs (defaults to `::oo::object` when none are declared).
    supers: Vec<Vec<u8>>,
    methods: BTreeMap<Vec<u8>, Method>,
    constructor: Option<Method>,
    destructor: Option<Vec<u8>>,
    /// Declared instance-variable names (auto-linked into every method frame).
    variables: Vec<Vec<u8>>,
}

/// An object instance.
struct Object {
    /// FQN of the object's class.
    class: Vec<u8>,
    /// The namespace holding this object's instance variables.
    var_ns: NsId,
}

/// One active method invocation (for `self` / `my` / `next`).
struct OoFrame {
    object: Vec<u8>,
    /// The class linearisation (MRO) this dispatch walks.
    mro: Vec<Vec<u8>>,
    /// Index into `mro` of the class whose method is currently running.
    index: usize,
    /// The method name (or empty for a constructor).
    method: Vec<u8>,
}

/// The interpreter's TclOO state.
#[derive(Default)]
pub struct OoState {
    classes: BTreeMap<Vec<u8>, Class>,
    objects: BTreeMap<Vec<u8>, Object>,
    counter: usize,
    /// Classes currently being defined (the `oo::define`/`oo::class create`
    /// script context the definition commands mutate).
    def_stack: Vec<Vec<u8>>,
    /// Active method-call contexts.
    call_stack: Vec<OoFrame>,
}

/// Register the `oo::*` commands and the definition / context commands.
pub fn install(interp: &mut Interp) {
    interp.register_builtin(b"oo::class", oo_class_cmd);
    interp.register_builtin(b"oo::object", oo_object_cmd);
    interp.register_builtin(b"oo::define", oo_define_cmd);
    // Definition-script commands (valid only inside an `oo::define` body).
    interp.register_builtin(b"method", def_method);
    interp.register_builtin(b"constructor", def_constructor);
    interp.register_builtin(b"destructor", def_destructor);
    interp.register_builtin(b"superclass", def_superclass);
    interp.register_builtin(b"variable", def_variable);
    interp.register_builtin(b"export", def_export);
    // Method-context commands.
    interp.register_builtin(b"self", self_cmd);
    interp.register_builtin(b"my", my_cmd);
    interp.register_builtin(b"next", next_cmd);
    // The root classes exist so superclass-less classes inherit from `object`
    // and `superclass oo::class`/`oo::object` validate. (`oo::class` keeps its
    // builtin command — only the class-registry entry is added.)
    interp
        .oo
        .classes
        .insert(b"::oo::object".to_vec(), Class::default());
    interp.oo.classes.insert(
        b"::oo::class".to_vec(),
        Class {
            supers: vec![b"::oo::object".to_vec()],
            ..Class::default()
        },
    );
    // `::oo::version` / `::oo::patchlevel` (read by the test harness).
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

// -- oo::class / oo::object / oo::define ------------------------------------

/// `oo::class create name ?definitionScript?` — define a class.
fn oo_class_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 3 || obj_bytes(argv[1]) != b"create" {
        return wrong_args(interp, b"oo::class create name ?definitionScript?");
    }
    let fqn = interp.fqn_for(&obj_bytes(argv[2]));
    if interp.oo.classes.contains_key(&fqn) || interp.oo.objects.contains_key(&fqn) {
        let mut m = b"can't create class \"".to_vec();
        m.extend_from_slice(&obj_bytes(argv[2]));
        m.extend_from_slice(b"\": command already exists with that name");
        return err(interp, &m);
    }
    interp.oo.classes.insert(
        fqn.clone(),
        Class {
            supers: vec![b"::oo::object".to_vec()],
            ..Class::default()
        },
    );
    interp.ns_register(&fqn, Command::OoObject(fqn.clone()));
    // Run the definition script (if any) with `fqn` as the definition target.
    if argv.len() >= 4 {
        let script = obj_bytes(argv[3]);
        let code = interp.oo_define(&fqn, &script);
        if code != Code::Ok {
            return code;
        }
    }
    interp.set_result(obj::new_string_bytes(&fqn));
    Code::Ok
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
    if !interp.oo.classes.contains_key(&fqn) {
        let mut m = b"\"".to_vec();
        m.extend_from_slice(&obj_bytes(argv[1]));
        m.extend_from_slice(b"\" does not refer to a class");
        return err(interp, &m);
    }
    if argv.len() == 3 {
        // Script form.
        let script = obj_bytes(argv[2]);
        interp.oo_define(&fqn, &script)
    } else {
        // Subcommand form: push the context and dispatch the one definition cmd.
        interp.oo.def_stack.push(fqn);
        let code = interp.dispatch(&argv[2..]);
        interp.oo.def_stack.pop();
        code
    }
}

// -- definition-script commands ---------------------------------------------

/// The class currently being defined, or an error if outside `oo::define`.
fn def_class(interp: &mut Interp) -> Result<Vec<u8>, Code> {
    match interp.oo.def_stack.last() {
        Some(c) => Ok(c.clone()),
        None => Err(interp
            .set_error(b"this command can only be called from within the body of a definition")),
    }
}

/// `method name params body` — add an (instance) method to the class.
fn def_method(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 4 {
        return wrong_args(interp, b"method name args body");
    }
    let class = match def_class(interp) {
        Ok(c) => c,
        Err(code) => return code,
    };
    let params = match crate::cmd_proc::parse_params(&obj_bytes(argv[2])) {
        Ok(p) => p,
        Err(e) => return err(interp, &e),
    };
    let m = Method {
        params,
        body: obj_bytes(argv[3]),
    };
    interp
        .oo
        .classes
        .get_mut(&class)
        .unwrap()
        .methods
        .insert(obj_bytes(argv[1]), m);
    interp.set_result_bytes(b"");
    Code::Ok
}

/// `constructor params body`.
fn def_constructor(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 3 {
        return wrong_args(interp, b"constructor arguments body");
    }
    let class = match def_class(interp) {
        Ok(c) => c,
        Err(code) => return code,
    };
    let params = match crate::cmd_proc::parse_params(&obj_bytes(argv[1])) {
        Ok(p) => p,
        Err(e) => return err(interp, &e),
    };
    interp.oo.classes.get_mut(&class).unwrap().constructor = Some(Method {
        params,
        body: obj_bytes(argv[2]),
    });
    interp.set_result_bytes(b"");
    Code::Ok
}

/// `destructor body`.
fn def_destructor(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 2 {
        return wrong_args(interp, b"destructor body");
    }
    let class = match def_class(interp) {
        Ok(c) => c,
        Err(code) => return code,
    };
    interp.oo.classes.get_mut(&class).unwrap().destructor = Some(obj_bytes(argv[1]));
    interp.set_result_bytes(b"");
    Code::Ok
}

/// `superclass name ?name ...?`.
fn def_superclass(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let class = match def_class(interp) {
        Ok(c) => c,
        Err(code) => return code,
    };
    let supers: Vec<Vec<u8>> = argv[1..]
        .iter()
        .map(|&a| interp.fqn_for(&obj_bytes(a)))
        .collect();
    // Validate each names a class.
    for s in &supers {
        if !interp.oo.classes.contains_key(s) {
            let mut m = b"\"".to_vec();
            m.extend_from_slice(s);
            m.extend_from_slice(b"\" does not refer to a class");
            return err(interp, &m);
        }
    }
    interp.oo.classes.get_mut(&class).unwrap().supers = if supers.is_empty() {
        vec![b"::oo::object".to_vec()]
    } else {
        supers
    };
    interp.set_result_bytes(b"");
    Code::Ok
}

/// `variable ?name ...?` — declare the class's instance variables.
///
/// Note: this shadows the global `variable` command. Inside an `oo::define`
/// body it records instance-variable names; otherwise it forwards to the
/// ordinary [`crate::cmd_var`] `variable`.
fn def_variable(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if interp.oo.def_stack.is_empty() {
        return crate::cmd_var::variable(interp, argv);
    }
    let class = interp.oo.def_stack.last().unwrap().clone();
    let names: Vec<Vec<u8>> = argv[1..].iter().map(|&a| obj_bytes(a)).collect();
    let c = interp.oo.classes.get_mut(&class).unwrap();
    for n in names {
        if !c.variables.contains(&n) {
            c.variables.push(n);
        }
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

/// `export name ?name ...?` — accepted; method visibility is not yet enforced.
fn def_export(interp: &mut Interp, _argv: &[*mut TclObj]) -> Code {
    interp.set_result_bytes(b"");
    Code::Ok
}

// -- method context: self / my / next ---------------------------------------

fn self_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let Some(frame) = interp.oo.call_stack.last() else {
        return err(interp, b"self may only be called from inside a method");
    };
    let object = frame.object.clone();
    let class = frame.mro.get(frame.index).cloned().unwrap_or_default();
    let method = frame.method.clone();
    match argv.get(1).map(|&a| obj_bytes(a)).as_deref() {
        None | Some(b"object") => interp.set_result(obj::new_string_bytes(&object)),
        Some(b"class") => interp.set_result(obj::new_string_bytes(&class)),
        Some(b"method") => interp.set_result(obj::new_string_bytes(&method)),
        Some(other) => {
            let mut m = b"unsupported self subcommand \"".to_vec();
            m.extend_from_slice(other);
            m.push(b'"');
            return err(interp, &m);
        }
    }
    Code::Ok
}

/// `my methodName ?arg ...?` — invoke a method on the current object (including
/// non-exported ones).
fn my_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 {
        return wrong_args(interp, b"my methodName ?arg ...?");
    }
    let Some(object) = interp.oo.call_stack.last().map(|f| f.object.clone()) else {
        return err(interp, b"my may only be called from inside a method");
    };
    let method = obj_bytes(argv[1]);
    interp.oo_invoke(&object, &method, &argv[2..])
}

/// `next ?arg ...?` — call the next implementation of the current method along
/// the MRO (e.g. a superclass method or constructor).
fn next_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let Some(frame) = interp.oo.call_stack.last() else {
        return err(interp, b"next may only be called from inside a method");
    };
    let (object, mro, index, method) = (
        frame.object.clone(),
        frame.mro.clone(),
        frame.index,
        frame.method.clone(),
    );
    let is_ctor = method.is_empty();
    // Find the next class along the MRO that defines this method/constructor.
    let next = (index + 1..mro.len()).find(|&j| {
        interp.oo.classes.get(&mro[j]).is_some_and(|c| {
            if is_ctor {
                c.constructor.is_some()
            } else {
                c.methods.contains_key(&method)
            }
        })
    });
    match next {
        Some(j) => interp.oo_call(&object, mro, j, &method, &argv[1..]),
        None if is_ctor => {
            // The implicit root constructor is a no-op.
            interp.set_result_bytes(b"");
            Code::Ok
        }
        None => err(interp, b"no next method implementation"),
    }
}

// -- the object-system engine (impl Interp) ----------------------------------

impl Interp {
    /// Run a class definition `script` with `class` as the target. Definition
    /// commands (`method`/`constructor`/…) mutate that class.
    fn oo_define(&mut self, class: &[u8], script: &[u8]) -> Code {
        self.oo.def_stack.push(class.to_vec());
        let code = self.eval_str(script);
        self.oo.def_stack.pop();
        code
    }

    /// Dispatch a command bound to the OO object/class FQN `fqn`.
    pub(crate) fn oo_dispatch(&mut self, fqn: &[u8], argv: &[*mut TclObj]) -> Code {
        if self.oo.classes.contains_key(fqn) {
            // Class command: `Class new|create|destroy ...`.
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
                Some(other) => {
                    let mut m = b"unknown method \"".to_vec();
                    m.extend_from_slice(other);
                    m.extend_from_slice(b"\"");
                    self.error(&m)
                }
                None => self.error(b"wrong # args: should be \"class method ?arg ...?\""),
            }
        } else if self.oo.objects.contains_key(fqn) {
            // Object command: `$obj destroy` or `$obj method ...`.
            match argv.get(1).map(|&a| obj_bytes(a)) {
                Some(m) if m == b"destroy" => {
                    self.oo_destroy(fqn);
                    self.set_result_bytes(b"");
                    Code::Ok
                }
                Some(method) => self.oo_invoke(fqn, &method, &argv[2..]),
                None => self.error(b"wrong # args: should be \"object method ?arg ...?\""),
            }
        } else {
            self.invalid_command(fqn)
        }
    }

    /// Create an instance of `class` (optionally named `name`, else auto-named),
    /// running its constructor with `args`. Result is the object FQN.
    fn oo_new(&mut self, class: &[u8], name: Option<Vec<u8>>, args: &[*mut TclObj]) -> Code {
        let fqn = name.unwrap_or_else(|| {
            let n = format!("::oo::Obj{}", self.oo.counter);
            self.oo.counter += 1;
            n.into_bytes()
        });
        if self.oo.objects.contains_key(&fqn) || self.oo.classes.contains_key(&fqn) {
            let mut m = b"can't create object \"".to_vec();
            m.extend_from_slice(&fqn);
            m.extend_from_slice(b"\": command already exists with that name");
            return self.error(&m);
        }
        // A private namespace for the object's instance variables.
        let var_ns = self.ensure_namespace(&fqn);
        self.oo.objects.insert(
            fqn.clone(),
            Object {
                class: class.to_vec(),
                var_ns,
            },
        );
        self.ns_register(&fqn, Command::OoObject(fqn.clone()));

        // Run the constructor along the MRO (if any class defines one).
        let mro = self.mro(class);
        if let Some(j) = mro.iter().position(|c| {
            self.oo
                .classes
                .get(c)
                .is_some_and(|c| c.constructor.is_some())
        }) {
            let code = self.oo_call(&fqn, mro, j, b"", args);
            if code == Code::Error {
                // Construction failed: roll back the half-built object.
                self.oo.objects.remove(&fqn);
                self.delete_command(&fqn);
                return Code::Error;
            }
        } else if !args.is_empty() {
            return self.error(b"object creation takes no arguments (no constructor)");
        }
        self.set_result(obj::new_string_bytes(&fqn));
        Code::Ok
    }

    /// Invoke instance method `method` on object `obj` with `args`.
    fn oo_invoke(&mut self, obj: &[u8], method: &[u8], args: &[*mut TclObj]) -> Code {
        let Some(class) = self.oo.objects.get(obj).map(|o| o.class.clone()) else {
            return self.invalid_command(obj);
        };
        let mro = self.mro(&class);
        match mro.iter().position(|c| {
            self.oo
                .classes
                .get(c)
                .is_some_and(|c| c.methods.contains_key(method))
        }) {
            Some(j) => self.oo_call(obj, mro, j, method, args),
            None => {
                let mut m = b"unknown method \"".to_vec();
                m.extend_from_slice(method);
                m.extend_from_slice(b"\"");
                self.error(&m)
            }
        }
    }

    /// Run the method (or constructor, when `method` is empty) defined by
    /// `mro[index]` on `obj`, pushing the OO call frame so `self`/`my`/`next`
    /// work and auto-linking the declared instance variables.
    fn oo_call(
        &mut self,
        obj: &[u8],
        mro: Vec<Vec<u8>>,
        index: usize,
        method: &[u8],
        args: &[*mut TclObj],
    ) -> Code {
        // Clone what we need before borrowing `self` mutably for `run_proc`.
        let cls = &self.oo.classes[&mro[index]];
        let m = if method.is_empty() {
            cls.constructor.clone()
        } else {
            cls.methods.get(method).cloned()
        };
        let Some(m) = m else {
            return self.error(b"no such method");
        };
        let var_ns = self.oo.objects[obj].var_ns;
        // The declared instance variables visible to the method: the union over
        // the whole MRO.
        let mut vars: Vec<Vec<u8>> = Vec::new();
        for c in &mro {
            if let Some(cl) = self.oo.classes.get(c) {
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
        self.oo.call_stack.push(OoFrame {
            object: obj.to_vec(),
            mro,
            index,
            method: method.to_vec(),
        });
        let code = self.run_proc(
            &m.params,
            &m.body,
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
        self.oo.call_stack.pop();
        code
    }

    /// Destroy an object: run its destructor (best-effort), then remove it.
    fn oo_destroy(&mut self, obj: &[u8]) {
        let class = self.oo.objects.get(obj).map(|o| o.class.clone());
        if let Some(class) = class {
            let mro = self.mro(&class);
            for c in &mro {
                if let Some(body) = self.oo.classes.get(c).and_then(|c| c.destructor.clone()) {
                    let var_ns = self.oo.objects[obj].var_ns;
                    self.oo.call_stack.push(OoFrame {
                        object: obj.to_vec(),
                        mro: mro.clone(),
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
                    self.oo.call_stack.pop();
                    break;
                }
            }
        }
        self.oo.objects.remove(obj);
        self.delete_command(obj);
    }

    /// Destroy a class (and its command). Instances are left as-is (a follow-up
    /// would cascade); this is enough for test teardown.
    fn oo_destroy_class(&mut self, class: &[u8]) {
        self.oo.classes.remove(class);
        self.delete_command(class);
    }

    /// The method-resolution order for `class`: a preorder walk of the class and
    /// its superclasses, each class appearing once (first occurrence wins).
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
        if let Some(c) = self.oo.classes.get(class) {
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
            // `self` inside a method is the object.
            assert_eq!(
                ok(i, b"Animal create ::bessie moo; bessie speak"),
                b"I say moo"
            );
            // destroy removes the command.
            ok(i, b"bessie destroy");
            assert_eq!(i.eval_str(b"bessie speak"), Code::Error);
        });
    }

    #[test]
    fn inheritance_and_next() {
        leak_free(|i| {
            ok(
                i,
                b"oo::class create Animal {
                    variable sound
                    constructor {s} { set sound $s }
                    method speak {} { return \"I say $sound\" }
                }",
            );
            ok(
                i,
                b"oo::class create Dog {
                    superclass Animal
                    constructor {} { next bark }
                    method speak {} { return \"Dog: [next]\" }
                }",
            );
            ok(i, b"set d [Dog new]");
            // `next bark` chained to Animal's constructor; `next` chained speak.
            assert_eq!(ok(i, b"$d speak"), b"Dog: I say bark");
            // `oo::define` adds a method after the fact.
            ok(i, b"oo::define Dog method legs {} { return 4 }");
            assert_eq!(ok(i, b"$d legs"), b"4");
        });
    }

    #[test]
    fn oo_package_is_provided() {
        leak_free(|i| {
            assert_eq!(ok(i, b"package require tcl::oo"), b"1.3.1");
            assert_eq!(ok(i, b"package require TclOO"), b"1.3.1");
        });
    }
}
