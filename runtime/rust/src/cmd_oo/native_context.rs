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

//! TclOO's original object-name header and selected variable-frame ownership.

use super::*;

pub(super) fn install_object_helpers(interp: &mut Interp, namespace: NsId) {
    let helpers = interp.ensure_namespace(b"::oo::Helpers");
    interp.namespaces_mut().set_path(namespace, vec![helpers]);
}

pub(super) fn helper_self(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let live = interp
        .oo
        .borrow()
        .call_stack
        .iter()
        .any(|frame| frame.activation == Some(interp.frames.borrow().current_activation()));
    if !live {
        let Some(&original) = argv.first() else {
            return interp.report_cmd_error(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "original self invocation head",
                )
                .into(),
            );
        };
        let head = match interp.native_string_bytes(&original) {
            Ok(head) => head,
            Err(error) => return interp.report_cmd_error(error.into()),
        };
        return interp.native_oo_context_required(&head);
    }
    self_cmd(interp, argv)
}

pub(super) fn install_unknown(interp: &mut Interp) {
    interp.ns_register(
        b"::oo::UnknownDefinition",
        Command::Builtin(unknown_definition),
    );
    let prefix = obj::Owned::fresh(obj::new_string_bytes(b"::oo::UnknownDefinition"));
    for name in [b"::oo::define".as_slice(), b"::oo::objdefine"] {
        if let Some(namespace) = interp.find_namespace_id(name) {
            let retired = interp
                .namespaces_mut()
                .set_unknown_handler(namespace, Some(prefix.clone()));
            drop(retired);
        }
    }
}

fn unknown_definition(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if interp.active_def_target().is_none() {
        return monkey_business(interp);
    }
    let Some((&requested, operands)) = argv.get(1..).and_then(|args| args.split_first()) else {
        return interp.set_error(b"bad call of unknown handler");
    };
    let requested_bytes = obj_bytes(requested);
    let Some(head) = interp.oo_definition_head(&requested_bytes) else {
        return interp.invalid_command(&requested_bytes);
    };
    let mut arguments = vec![head.as_ptr()];
    arguments.extend_from_slice(operands);
    interp.dispatch(&arguments)
}

pub(super) fn monkey_business(interp: &mut Interp) -> Code {
    let code = crate::interp::error_code_list(&[b"TCL", b"OO", b"MONKEY_BUSINESS"]);
    interp.error_with_code(b"this command may only be called from within the context of an ::oo::define or ::oo::objdefine command", &code)
}

pub(super) fn definition_worker(
    interp: &mut Interp,
    argv: &[*mut TclObj],
    class: bool,
    handler: crate::interp::BuiltinFn,
) -> Code {
    let Some(target) = interp.active_def_target() else {
        return handler(interp, argv);
    };
    let object = match target {
        DefTarget::Class(object) | DefTarget::Object(object) => object,
    };
    if class && !interp.oo.borrow().classes.contains_key(&object) {
        let code = crate::interp::error_code_list(&[b"TCL", b"OO", b"MONKEY_BUSINESS"]);
        return interp.error_with_code(b"attempt to misuse API", &code);
    }
    let index = interp
        .oo_definition_index()
        .expect("selected definition activation");
    interp.oo.borrow_mut().def_stack[index].0 = if class {
        DefTarget::Class(object)
    } else {
        DefTarget::Object(object)
    };
    let code = handler(interp, argv);
    interp.oo.borrow_mut().def_stack[index].0 = target;
    code
}

impl Interp {
    pub(super) fn oo_outer_namespace(&self) -> NsId {
        let mut activation = self.frames.borrow().current_activation();
        let mut namespace = self.current_ns();
        let state = self.oo.borrow();
        while let Some((_, _, _, caller, caller_activation)) = state
            .def_stack
            .iter()
            .rev()
            .find(|(_, active, _, _, _)| *active == activation)
        {
            namespace = *caller;
            activation = *caller_activation;
        }
        namespace
    }

    fn oo_definition_index(&self) -> Option<usize> {
        let activation = self.frames.borrow().current_activation();
        self.oo
            .borrow()
            .def_stack
            .iter()
            .rposition(|(_, active, _, _, _)| *active == activation)
    }

    fn oo_definition_head(&self, requested: &[u8]) -> Option<obj::Owned> {
        if requested.is_empty() || requested.windows(2).any(|pair| pair == b"::") {
            return None;
        }
        let names = self.visible_command_names_in(self.current_ns());
        let selected = if let Some(exact) = names.iter().find(|name| name.as_slice() == requested) {
            exact
        } else {
            let mut candidates = names.iter().filter(|name| name.starts_with(requested));
            match (candidates.next(), candidates.next()) {
                (Some(name), None) => name,
                _ => return None,
            }
        };
        let mut full = self.namespaces().qualified_name(self.current_ns());
        if full != b"::" {
            full.extend_from_slice(b"::");
        }
        full.extend_from_slice(selected);
        Some(obj::Owned::fresh(obj::new_string_bytes(&full)))
    }

    pub(super) fn oo_magic_definition_invoke(
        &mut self,
        requested: *mut TclObj,
        operands: &[*mut TclObj],
    ) -> Code {
        let bytes = obj_bytes(requested);
        let head = self
            .oo_definition_head(&bytes)
            .unwrap_or_else(|| obj::Owned::retain(requested));
        let mut arguments = vec![head.as_ptr()];
        arguments.extend_from_slice(operands);
        self.dispatch(&arguments)
    }

    pub(super) fn oo_enter_definition(
        &mut self,
        target: &DefTarget,
        argv: &[*mut TclObj],
    ) -> Result<NsId, Code> {
        let default = match target {
            DefTarget::Class(_) => b"::oo::define".as_slice(),
            DefTarget::Object(_) => b"::oo::objdefine",
        };
        let name = self
            .definition_namespace_for(target)
            .unwrap_or_else(|| default.to_vec());
        let Some(namespace) = self.find_namespace_id(&name) else {
            return Err(self.set_error(b"cannot process definitions; support namespace deleted"));
        };
        let caller = self.current_ns();
        let caller_activation = self.frames.borrow().current_activation();
        self.set_current_ns(namespace);
        self.enter_namespace_activation(namespace);
        self.frames.borrow_mut().push_namespace(namespace);
        self.frames
            .borrow_mut()
            .set_words(argv.iter().map(|&word| obj_bytes(word)).collect());
        self.frames
            .borrow_mut()
            .install_original_error_stack_argv(argv);
        let object = match target {
            DefTarget::Class(object) | DefTarget::Object(object) => *object,
        };
        let creation = self
            .oo
            .borrow()
            .objects
            .get(&object)
            .map(|owner| owner.creation_id);
        let activation = self.frames.borrow().current_activation();
        self.oo.borrow_mut().def_stack.push((
            target.clone(),
            activation,
            creation,
            caller,
            caller_activation,
        ));
        Ok(caller)
    }

    pub(super) fn oo_leave_definition(&mut self, caller: NsId) {
        self.oo.borrow_mut().def_stack.pop();
        let popped = self.pop_native_call_frame();
        self.set_current_ns(caller);
        self.leave_namespace_activation(popped);
    }

    pub(crate) fn oo_bind_method_activation(&self) {
        let activation = self.frames.borrow().current_activation();
        if let Some(frame) = self.oo.borrow_mut().call_stack.last_mut() {
            frame.activation = Some(activation);
        }
    }

    pub(super) fn oo_name_owner(&self, object: OoId) -> Rc<RefCell<Option<obj::Owned>>> {
        Rc::clone(&self.oo.borrow().objects[&object].cached_name)
    }

    pub(super) fn oo_original_name(&self, object: OoId) -> *mut TclObj {
        let state = self.oo.borrow();
        let alive = state.objects.contains_key(&object);
        let owner = state
            .objects
            .get(&object)
            .map(|owner| &owner.cached_name)
            .or_else(|| {
                state
                    .call_stack
                    .iter()
                    .rev()
                    .find(|frame| frame.object == object)
                    .map(|frame| &frame.name_owner)
            })
            .expect("object-name getter retains the actual OO owner");
        let mut cached = owner.borrow_mut();
        cached
            .get_or_insert_with(|| {
                let bytes = if alive { state.name(object) } else { b"" };
                let value = obj::Owned::fresh(obj::new_string_bytes(bytes));
                if alive {
                    if let Some(protocol) = self
                        .native_invocation_dialect()
                        .native_string_protocol()
                        .filter(|protocol| protocol.tcl_version().is_some())
                    {
                        obj::set_native_append_string(
                            value.as_ptr(),
                            protocol,
                            Some((
                                Rc::<[u8]>::from(bytes),
                                tcl_syntax::native_string::NativeStringStorageIdentity::Allocated,
                            )),
                            None,
                            None,
                        )
                        .expect("fresh original command-full-name append String");
                    }
                }
                value
            })
            .as_ptr()
    }
}

impl Interp {
    pub(crate) fn execute_native_oo_self(&mut self, namespace: bool) -> Code {
        let owner = self
            .oo
            .borrow()
            .call_stack
            .iter()
            .rev()
            .find(|frame| frame.activation == Some(self.frames.borrow().current_activation()))
            .map(|frame| frame.object);
        let Some(object) = owner else {
            return self.native_oo_context_required(b"self");
        };
        let original = obj::Owned::retain(self.oo_original_name(object));
        if namespace {
            // Native SELF first checks the frame and obtains its original name.
            drop(original);
            return match tcl_cmd_core::namespace::current_original(self) {
                Ok(value) => {
                    self.set_result(value);
                    Code::Ok
                }
                Err(error) => self.report_cmd_error(error),
            };
        } else {
            self.set_result(original.as_ptr());
        }
        Code::Ok
    }

    pub(crate) fn execute_native_oo_next(
        &mut self,
        words: &[*mut TclObj],
        class: bool,
        version: tcl_dialect::TclVersion,
    ) -> Code {
        let live = self
            .oo
            .borrow()
            .call_stack
            .iter()
            .any(|frame| frame.activation == Some(self.frames.borrow().current_activation()));
        let head = if version == tcl_dialect::TclVersion::V9_1 {
            let Some(original) = words.first() else {
                return self.set_error(b"missing native helper head");
            };
            let bytes = match self.native_string_bytes(original) {
                Ok(bytes) => bytes.to_vec(),
                Err(error) => return self.report_cmd_error(error.into()),
            };
            tcl_core_types::c_string_extent(&bytes).to_vec()
        } else if class {
            b"nextto".to_vec()
        } else {
            b"next".to_vec()
        };
        if !live {
            return self.native_oo_context_required(&head);
        }
        if class {
            nextto_cmd(self, words)
        } else {
            next_cmd(self, words)
        }
    }

    fn native_oo_context_required(&mut self, head: &[u8]) -> Code {
        let mut message = head.to_vec();
        message.extend_from_slice(b" may only be called from inside a method");
        let code = crate::interp::error_code_list(&[b"TCL", b"OO", b"CONTEXT_REQUIRED"]);
        self.error_with_code(&message, &code)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn self_call_uses_the_selected_uplevel_activation_and_restores_its_caller() {
        for version in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut interp = Interp::with_native_core(
                crate::interp::default_host(),
                crate::environment::profile_for_dialect(version),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            assert_eq!(
                interp.eval_str(
                    br"oo::class create Outer {method run {peer} {$peer inspect}}
oo::class create Inner {method inspect {} {
    set direct [self call]
    set caller [uplevel 1 {::oo::Helpers::self call}]
    list $direct $caller [self call]
}}
Outer create outer
Inner create inner
outer run inner"
                ),
                Code::Ok,
                "{version}: {:?}",
                interp.result_bytes()
            );
            assert_eq!(
                interp.result_bytes(),
                b"{{{method inspect ::Inner method}} 0} {{{method run ::Outer method}} 0} {{{method inspect ::Inner method}} 0}",
                "{version}"
            );
            assert_eq!(
                interp.eval_str(b"proc ordinary {} {::oo::Helpers::self call}; ordinary"),
                Code::Error,
                "{version}"
            );
            assert_eq!(interp.error_code(), b"TCL OO CONTEXT_REQUIRED", "{version}");
            assert_eq!(
                interp.result_bytes(),
                b"::oo::Helpers::self may only be called from inside a method",
                "{version}"
            );
            assert_eq!(interp.eval_str(b"namespace current"), Code::Ok);
            assert_eq!(interp.error_code(), b"NONE", "{version}");
        }
    }

    #[test]
    fn dynamic_self_subcommands_use_the_original_native_index_table() {
        for version in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut interp = Interp::with_native_core(
                crate::interp::default_host(),
                crate::environment::profile_for_dialect(version),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            assert_eq!(
                interp.eval_str(b"oo::class create C {method query {which} {self $which}}; C create obj; obj query o"),
                Code::Ok
            );
            assert_eq!(interp.result_bytes(), b"::obj");
            assert_eq!(
                interp.eval_str(b"string match ::oo::Obj* [obj query nam]"),
                Code::Ok
            );
            assert_eq!(interp.result_bytes(), b"1");
            for (script, error_code) in [
                (
                    b"obj query c".as_slice(),
                    b"TCL LOOKUP INDEX subcommand c".as_slice(),
                ),
                (b"obj query n", b"TCL LOOKUP INDEX subcommand n"),
                (b"obj query {}", b"TCL LOOKUP INDEX subcommand {}"),
                (b"obj query missing", b"TCL LOOKUP INDEX subcommand missing"),
            ] {
                assert_eq!(
                    interp.eval_str(script),
                    Code::Error,
                    "{version}: {script:?}"
                );
                assert_eq!(interp.error_code(), error_code, "{version}: {script:?}");
            }
        }
    }

    #[test]
    fn helper_artifact_executes_only_the_actual_method_frame_and_original_words() {
        for version in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut interp = Interp::with_native_core(
                crate::interp::default_host(),
                crate::environment::profile_for_dialect(version),
                tcl_registry::special_vars::NativeBootstrapInputs {
                    package_path: Vec::new(),
                    default_library: None,
                },
            )
            .unwrap();
            assert_eq!(interp.eval_str(br"oo::class create B {method m args {list BASE {*}$args}; method by args {list BASE {*}$args}; method static args {list STATIC {*}$args}}
oo::class create C {
 superclass B
 method m args {next {*}$args}
 method by args {nextto B {*}$args}
 method static {} {nextto {*}{B ONE}}
 method object {} {self o}
 method space {} {self n}
 method nested {} {ordinary}
}
proc ordinary {} {::oo::Helpers::self}
C create obj
list [obj m A B] [obj by C D] [obj static] [obj object] [string match ::oo::Obj* [obj space]] [catch {obj nested}] [llength [info commands ::self]]
"),Code::Ok,"{version}: {:?}",interp.result_bytes());
            assert_eq!(
                interp.result_bytes(),
                b"{BASE A B} {BASE C D} {STATIC ONE} ::obj 1 1 0",
                "{version}"
            );
            assert_eq!(interp.eval_str(b"obj nested"), Code::Error);
            assert_eq!(interp.error_code(), b"TCL OO CONTEXT_REQUIRED");
            assert_eq!(
                interp.eval_str(
                    b"oo::define C method evalContext {} {my eval {self object}}; obj evalContext"
                ),
                Code::Ok
            );
            assert_eq!(interp.result_bytes(), b"::obj");
        }
    }

    #[test]
    fn definition_namespace_owns_its_scope_and_original_workers() {
        let mut interp = Interp::new();
        let script = b"proc ::method args {error GLOBAL_WORKER}; namespace eval caller {proc build {} {set private CALLER; oo::class create ::C {set scope [namespace current]; set inherited [info exists private]; set local [uplevel 1 {set private}]; proc born {} {return HERE}; meth m {} {return OK}}}}; caller::build; C create obj; list [obj m] $::oo::define::scope $::oo::define::inherited $::oo::define::local [::oo::define::born]";
        assert_eq!(
            interp.eval_str(script),
            Code::Ok,
            "{:?}",
            interp.result_bytes()
        );
        assert_eq!(interp.result_bytes(), b"OK ::oo::define 0 CALLER HERE");
        assert_eq!(
            interp.eval_str(b"::oo::define::method m {} {}"),
            Code::Error
        );
        assert_eq!(interp.error_code(), b"TCL OO MONKEY_BUSINESS");
        assert_eq!(interp.current_ns(), GLOBAL);
    }

    #[test]
    fn nested_definitions_retain_actual_caller_frames_and_qualified_workers() {
        for version in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut interp = Interp::with_native_core(
                crate::interp::default_host(),
                crate::environment::profile_for_dialect(version),
                tcl_registry::special_vars::NativeBootstrapInputs {
                    package_path: Vec::new(),
                    default_library: None,
                },
            )
            .unwrap();
            assert_eq!(interp.eval_str(br"namespace eval ::outer {
 oo::class create B {}
 oo::class create C {}
 oo::class create D {}
 proc build {} {
  oo::define C {oo::define ::outer::D {superclass B; uplevel 1 {::oo::define::method resumed {} {return OUTER}}}}
 }
 build
}
::outer::C create obj
set chained [list [info class superclasses ::outer::D] [obj resumed]]
oo::class create ::oo::define::B {}
proc ::oo::define::bridge {} {oo::define ::outer::D {superclass B}}
oo::define ::outer::C {bridge}
set ordinary [info class superclasses ::outer::D]
proc ::inspect args {error GLOBAL}
proc ::oo::define::inspect args {list [lindex [info level 0] 0] {*}$args}
set magic [list [oo::define ::outer::C insp FIRST SECOND] [oo::define ::outer::C inspect THIRD]]
set outside [catch {::oo::define::method outside {} {}} result options]
list $chained $ordinary $magic [namespace current] $outside [dict get $options -errorcode]"), Code::Ok, "{version}: {:?}", interp.result_bytes());
            assert_eq!(interp.result_bytes(), b"{::outer::B OUTER} ::oo::define::B {{::oo::define::inspect FIRST SECOND} {::oo::define::inspect THIRD}} :: 1 {TCL OO MONKEY_BUSINESS}");
            assert!(interp.oo.borrow().def_stack.is_empty());
        }
    }

    #[test]
    fn cached_self_name_is_the_same_header_and_nested_procs_have_no_context() {
        let mut interp = Interp::new();
        assert_eq!(interp.eval_str(b"oo::class create C {method object {} {self object}; method nested {} {p}; method shifted {} {uplevel #0 {::oo::Helpers::self}}}; proc p {} {::oo::Helpers::self}; C create obj; obj object"), Code::Ok);
        let original = obj::Owned::retain(interp.get_obj_result());
        list::list_elements(original.as_ptr()).unwrap();
        assert_eq!(interp.eval_str(b"obj object"), Code::Ok);
        assert_eq!(original.as_ptr(), interp.get_obj_result());
        assert!(core::ptr::eq(
            obj::obj_type_ptr(interp.get_obj_result()),
            &list::TCL_LIST_TYPE
        ));
        assert_eq!(interp.eval_str(b"obj nested"), Code::Error);
        assert_eq!(interp.eval_str(b"obj shifted"), Code::Error);
        assert_eq!(
            interp.eval_str(b"rename obj renamed; renamed object"),
            Code::Ok
        );
        assert_ne!(original.as_ptr(), interp.get_obj_result());
        let renamed = obj::Owned::retain(interp.get_obj_result());
        assert_eq!(
            interp.eval_str(
                b"oo::define C method deleted {} {rename [self] {}; self}; renamed deleted"
            ),
            Code::Ok
        );
        assert_eq!(interp.result_bytes(), b"");
        assert_eq!(obj_bytes(original.as_ptr()), b"::obj");
        assert_eq!(obj_bytes(renamed.as_ptr()), b"::renamed");
    }
}
