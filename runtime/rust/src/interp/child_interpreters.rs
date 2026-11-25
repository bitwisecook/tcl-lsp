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

//! Child interpreters: the child command, creation and deletion, addressing by path, and safe interpreters.

use super::commands::{Command, CommandVisibilityOp};
use super::native_alias_roles::NativeAliasHeader;
use super::native_children::ChildInterpreterCommand;
use super::{native_arguments, new_string, Code, Interp};
use crate::namespace::GLOBAL;
use crate::obj;
use crate::obj::TclObj;
use std::rc::Rc;

impl Interp {
    /// The child-as-command subcommand words, in C table order (`options[]` in
    /// `NRChildCmd`, `tclInterp.c`), resolved with
    /// `Tcl_GetIndexFromObj(…, "option", 0)` — so `ev` abbreviates `eval` and
    /// the empty word is `ambiguous option ""`. The child command object
    /// advertises a *shorter* list than `interp` does (no `children`, `create`,
    /// `delete`, or `exists`: those are only ever spelled `interp <op> path`),
    /// and this runtime dispatches all thirteen.
    /// The `wrong # args: should be "<child><tail>"` message for the `$child
    /// <sub>` shorthand. C's `NRChildCmd` builds every one of its arity errors
    /// with `Tcl_WrongNumArgs(interp, 1, objv, …)`, so the noun is **`objv[0]`
    /// — the word this call was written with**, never the `interp` ensemble and
    /// never the child's table key.
    ///
    /// The two spellings diverge whenever the command is not reached under the
    /// name it was created with: after `interp create kid; rename kid foo`,
    /// `foo hidden extra` reports `"foo hidden"`, and the qualified `::foo
    /// hidden extra` reports `"::foo hidden"` (tclsh 8.6.16 / 9.0.4-pinned).
    /// Only the child *lookup* keeps using the table key.
    ///
    /// One spelling this does not yet recover: reached through an `interp
    /// alias`, C reports the *alias*' name, because `AliasObjCmd` installs an
    /// ensemble rewrite (`TclInitRewriteEnsemble`, tclInterp.c) that
    /// `Tcl_WrongNumArgs` reads back. This runtime's alias trampoline records no
    /// such rewrite, so `bar hidden extra` reports the target's name rather than
    /// `bar` — a separate gap in alias dispatch, not in this seam.
    ///
    /// The *subcommand* half of the noun goes the other way: every `tail` here
    /// spells the table word in full, because `Tcl_WrongNumArgs` expands an
    /// index-typed argument back to its table entry (`tclIndexObj.c`) and
    /// `Tcl_GetIndexFromObj` has already retyped `objv[1]` by the time the
    /// arity check runs. So an abbreviated call reports the canonical word:
    /// `kid hidd extra` is `"kid hidden"`, not `"kid hidd"` (tclsh
    /// 8.6.16 / 9.0.4-pinned). Written word for the command, canonical word
    /// for the subcommand.
    fn child_wrong_args(
        &mut self,
        command: &ChildInterpreterCommand,
        argv: &[*mut TclObj],
        arguments: &native_arguments::OriginalArguments,
        tail: &[u8],
    ) -> Code {
        let head = match self.invoked_word_checked(argv, arguments) {
            Ok(head) => head,
            Err(error) => return self.report_cmd_error(error),
        };
        if let Err(error) = self.check_original_child_command(command, arguments) {
            return self.report_cmd_error(error);
        }
        let mut message = b"wrong # args: should be \"".to_vec();
        message.extend_from_slice(&head);
        message.extend_from_slice(tail);
        message.push(b'"');
        self.wrong_arguments_message(&message)
    }

    fn invoked_word_checked(
        &self,
        argv: &[*mut TclObj],
        arguments: &native_arguments::OriginalArguments,
    ) -> Result<Vec<u8>, tcl_cmd_core::CmdError> {
        arguments.check_for(self)?;
        let Some(&word) = argv.first() else {
            return Ok(Vec::new());
        };
        arguments.check_original_for(self, 0, word)?;
        let bytes = arguments.bytes(0)?;
        arguments.check_original_for(self, 0, word)?;
        Ok(bytes.to_vec())
    }

    /// Dispatch a child-interpreter command (`$child subcommand ?arg ...?`): the
    /// child is addressable like the `interp` ensemble restricted to it.
    ///
    /// The `hide` / `expose` / `invokehidden` arms hand off to
    /// [`crate::cmd_alias::hidectl_in`] / [`crate::cmd_alias::invokehidden_in`],
    /// the same owners `interp hide|expose|invokehidden path …` calls — C's
    /// `NRChildCmd` and `NRInterpCmd` share `ChildHide` / `ChildExpose` /
    /// `ChildInvokeHidden` the same way. Only the arity check and its noun
    /// differ between the two entry points.
    pub(super) fn dispatch_child(
        &mut self,
        command: &ChildInterpreterCommand,
        argv: &[*mut TclObj],
    ) -> Code {
        let arguments = match native_arguments::OriginalArguments::enter(self, argv) {
            Ok(arguments) => arguments,
            Err(error) => return self.report_cmd_error(error),
        };
        if let Err(error) = arguments.check_for(self) {
            return self.report_cmd_error(error);
        }
        if !command.is_current_for(self) {
            return self.error(b"interpreter has been deleted");
        }
        let code = self.dispatch_child_original(command, argv, &arguments);
        arguments.finish(code)
    }

    fn dispatch_child_original(
        &mut self,
        command: &ChildInterpreterCommand,
        argv: &[*mut TclObj],
        arguments: &native_arguments::OriginalArguments,
    ) -> Code {
        let name = command.name();
        if argv.len() < 2 {
            return self.child_wrong_args(command, argv, arguments, b" cmd ?arg ...?");
        }
        if self
            .native_invocation_dialect()
            .native_jim_lookup_protocol()
            .is_some()
        {
            const CHOICES: &[&[u8]] = &[b"eval", b"delete", b"alias"];
            let word = match arguments.bytes(1) {
                Ok(word) => word,
                Err(error) => return self.report_cmd_error(error),
            };
            if let Err(error) = self.check_original_child_command(command, arguments) {
                return self.report_cmd_error(error);
            }
            let sub = match crate::cmd_alias::resolve_interp_option(CHOICES, CHOICES, &word) {
                Ok(sub) => sub,
                Err(message) => return self.error(&message),
            };
            return match sub {
                b"eval" if argv.len() >= 3 => self.child_eval_original(name, &argv[2..]),
                b"delete" if argv.len() == 2 => {
                    self.delete_child(name);
                    if let Err(error) = arguments.check_for(self) {
                        return self.report_cmd_error(error);
                    }
                    self.set_result_bytes(b"");
                    Code::Ok
                }
                b"alias" if argv.len() >= 4 => {
                    crate::cmd_alias::child_alias_handle_original(self, name, argv, arguments)
                }
                _ => self.child_wrong_args(command, argv, arguments, b" subcommand ?arg ...?"),
            };
        }
        if let Err(error) = arguments.check_original_for(self, 1, argv[1]) {
            return self.report_cmd_error(error);
        }
        let selected = self.native_interpreter_option_from_original(argv[1], true);
        if let Err(error) = arguments.check_original_for(self, 1, argv[1]) {
            return self.report_cmd_error(error);
        }
        if let Err(error) = self.check_original_child_command(command, arguments) {
            return self.report_cmd_error(error);
        }
        let sub = match selected {
            Ok(name) => name.as_bytes(),
            Err(error) => return self.report_cmd_error(error),
        };
        match sub {
            b"eval" => {
                if argv.len() < 3 {
                    return self.child_wrong_args(command, argv, arguments, b" eval arg ?arg ...?");
                }
                self.child_eval_original(name, &argv[2..])
            }
            b"issafe" => {
                if argv.len() != 2 {
                    return self.child_wrong_args(command, argv, arguments, b" issafe");
                }
                let safe = self.with_child(name, |c| c.is_safe()).unwrap_or(false);
                self.set_result_bytes(if safe { b"1" } else { b"0" });
                Code::Ok
            }
            b"delete" => {
                self.delete_child(name);
                if let Err(error) = arguments.check_for(self) {
                    return self.report_cmd_error(error);
                }
                self.set_result_bytes(b"");
                Code::Ok
            }
            // Both forms are `$child hide|expose name ?other?`. The one-word
            // form spells source and destination the same, which is what makes
            // C's asymmetric checks visible here: `kid hide ::foo::bar` is a
            // qualified *token* and `kid expose ::foo::bar` a qualified
            // *destination*, so the two report different errors.
            b"hide" | b"expose" => {
                // The *resolved* word, so `kid ex foo` is an expose too.
                let hide = sub == b"hide";
                let op = if hide {
                    CommandVisibilityOp::Hide
                } else {
                    CommandVisibilityOp::Expose
                };
                if argv.len() != 3 && argv.len() != 4 {
                    return self.child_wrong_args(
                        command,
                        argv,
                        arguments,
                        if hide {
                            b" hide cmdName ?hiddenCmdName?"
                        } else {
                            b" expose hiddenCmdName ?cmdName?"
                        },
                    );
                }
                // A safe interpreter may not touch any hidden-command table
                // (checked on the executing interp).
                if self.is_safe() {
                    return self.error(if hide {
                        b"permission denied: safe interpreter cannot hide commands"
                    } else {
                        b"permission denied: safe interpreter cannot expose commands"
                    });
                }
                crate::cmd_alias::hidectl_in(self, &[name.to_vec()], op, &argv[2..])
            }
            b"invokehidden" => {
                let mut usage = match self.invoked_word_checked(argv, arguments) {
                    Ok(usage) => usage,
                    Err(error) => return self.report_cmd_error(error),
                };
                if let Err(error) = self.check_original_child_command(command, arguments) {
                    return self.report_cmd_error(error);
                }
                usage.extend_from_slice(
                    b" invokehidden ?-namespace ns? ?-global? ?--? cmd ?arg ..?",
                );
                crate::cmd_alias::invokehidden_in(self, &[name.to_vec()], &argv[2..], &usage)
            }
            b"hidden" => {
                if argv.len() != 2 {
                    return self.child_wrong_args(command, argv, arguments, b" hidden");
                }
                let names = self
                    .with_child(name, |c| c.hidden_names())
                    .unwrap_or_default();
                let elems: Vec<*mut TclObj> =
                    names.iter().map(|n| obj::new_string_bytes(n)).collect();
                self.set_result(self.new_list_object(&elems));
                Code::Ok
            }
            b"aliases" => {
                if argv.len() != 2 {
                    return self.child_wrong_args(command, argv, arguments, b" aliases");
                }
                let names = match self.with_child(name, |c| c.alias_names()) {
                    Some(Ok(names)) => names,
                    Some(Err(error)) => return self.report_cmd_error(error.into()),
                    None => return self.error(b"could not find interpreter"),
                };
                let elems = names.iter().map(obj::Owned::as_ptr).collect::<Vec<_>>();
                self.set_result(self.new_list_object(&elems));
                Code::Ok
            }
            // `$child alias srcCmd targetCmd ?arg ...?` — a cross-interp alias in
            // the child delegating to `targetCmd` in this (parent) interp. (The
            // target is implicitly the parent, so there is no target-path arg.)
            b"alias" => crate::cmd_alias::child_alias_handle_original(self, name, argv, arguments),
            // `$child recursionlimit ?newlimit?` — the path-less child form.
            b"recursionlimit" => {
                if argv.len() > 3 {
                    return self.child_wrong_args(
                        command,
                        argv,
                        arguments,
                        b" recursionlimit ?newlimit?",
                    );
                }
                let newlimit = if argv.len() == 3 {
                    if let Err(error) = arguments.bytes(2) {
                        return self.report_cmd_error(error);
                    }
                    if let Err(error) = self.check_original_child_command(command, arguments) {
                        return self.report_cmd_error(error);
                    }
                    argv.get(2).copied()
                } else {
                    None
                };
                match self.with_child(name, |c| c.recursion_limit_apply(newlimit)) {
                    // C answers a new limit with the word it was given.
                    Some(Ok(n)) => {
                        match newlimit {
                            Some(word) => self.set_result(word),
                            None => self.set_result_bytes(n.to_string().as_bytes()),
                        }
                        Code::Ok
                    }
                    Some(Err(e)) => self.report_cmd_error(e),
                    None => self.error(b"could not find interpreter"),
                }
            }
            // `$child bgerror ?cmdPrefix?` — get/set the child's background-error
            // handler.
            b"bgerror" => {
                if argv.len() > 3 {
                    return self.child_wrong_args(
                        command,
                        argv,
                        arguments,
                        b" bgerror ?cmdPrefix?",
                    );
                }
                let prefix = argv.get(2).copied();
                match self.with_child(name, |c| c.bgerror_apply(prefix)) {
                    Some(Ok(h)) => {
                        unsafe {
                            self.set_obj_result(h.as_ptr());
                        }
                        Code::Ok
                    }
                    Some(Err(error)) => self.report_cmd_error(error),
                    None => self.error(b"could not find interpreter"),
                }
            }
            // `$child marktrusted` — clear the child's safe flag (denied from a
            // safe interpreter).
            b"marktrusted" => {
                if self.is_safe() {
                    return self.error(b"permission denied: safe interpreter cannot mark trusted");
                }
                self.with_child(name, |c| c.mark_trusted());
                self.set_result_bytes(b"");
                Code::Ok
            }
            // `$child debug ?-frame ?bool??` — the per-interp frame-debug switch.
            b"debug" => {
                if argv.len() > 4 {
                    return self.child_wrong_args(
                        command,
                        argv,
                        arguments,
                        b" debug ?-frame ?bool??",
                    );
                }
                let opts: Vec<*mut TclObj> = argv[2..].to_vec();
                match self.with_child(name, |c| c.debug_apply(&opts)) {
                    Some(Ok(o)) => {
                        self.set_result(o);
                        Code::Ok
                    }
                    Some(Err(error)) => self.report_cmd_error(error),
                    None => self.error(b"could not find interpreter"),
                }
            }
            // `$child limit limitType ?-option value …?` — query/configure the
            // child's commands/time limit.
            b"limit" => {
                if argv.len() < 3 {
                    return self.child_wrong_args(
                        command,
                        argv,
                        arguments,
                        b" limit limitType ?-option value ...?",
                    );
                }
                let ltype = match arguments.bytes(2) {
                    Ok(value) => value,
                    Err(error) => return self.report_cmd_error(error),
                };
                if let Err(error) = self.check_original_child_command(command, arguments) {
                    return self.report_cmd_error(error);
                }
                let opts: Vec<*mut TclObj> = argv[3..].to_vec();
                match self.with_child(name, |c| c.limit_apply(&ltype, &opts)) {
                    Some(Ok(o)) => {
                        self.set_result(o);
                        Code::Ok
                    }
                    Some(Err(error)) => self.report_cmd_error(error),
                    None => self.error(b"could not find interpreter"),
                }
            }
            _ => self.report_cmd_error(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "native child interpreter worker",
                )
                .into(),
            ),
        }
    }

    /// Jim's zero-argument factory creates a global interpreter handle.
    pub(crate) fn create_jim_child(&mut self) -> Vec<u8> {
        let name = format!("::interp.handle{}", self.interp_counter.get()).into_bytes();
        self.interp_counter.set(self.interp_counter.get() + 1);
        self.create_child(Some(name))
    }

    /// Create a C Tcl child, then initialise its trusted library from the
    /// same installation as its parent.
    pub(crate) fn create_tcl_child(
        &mut self,
        name: Option<Vec<u8>>,
        safe: bool,
    ) -> Result<Vec<u8>, Code> {
        let safe = safe || self.is_safe();
        let library = self.initialised_library.borrow().clone();
        let name = self.create_child(name);
        if self.host_refusal_pending() {
            return Err(Code::Error);
        }
        if safe {
            self.with_child(&name, |child| child.make_safe());
            return Ok(name);
        }
        let Some(library) = library else {
            return Ok(name);
        };
        let failed = self.with_child(&name, |child| {
            (child.init_library_from(Some(library)) != Code::Ok).then(|| child.clone())
        });
        let Some(Some(child)) = failed else {
            return Ok(name);
        };
        self.delete_child(&name);
        if child.host_refusal_pending() {
            return Err(self.transport_host_refusal_from(&child));
        }
        self.set_result(child.get_obj_result());
        Err(Code::Error)
    }

    /// Create a child interpreter named `name` (auto-generated when empty),
    /// registering it as a command in this interp. Returns the name.
    pub(crate) fn create_child(&mut self, name: Option<Vec<u8>>) -> Vec<u8> {
        self.invalidate_interpreter_policy();
        self.invalidate_command_environment();
        let name = name.unwrap_or_else(|| {
            let n = format!("interp{}", self.interp_counter.get());
            self.interp_counter.set(self.interp_counter.get() + 1);
            n.into_bytes()
        });
        // Physical constructor inventory requires the same explicit profile
        // recipe as with_native_core; a logical runtime version is not one.
        let bootstrap = tcl_registry::InvocationDialect::of_profile(self.dialect_profile())
            .native_bootstrap_protocol();
        let mut child = if bootstrap.is_some() {
            Interp::with_native_core(
                self.host(),
                self.dialect_profile(),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .expect("selected native child core bootstrap")
        } else {
            Interp::with_host(self.host())
        };
        child.native_child_interpreter.set(true);
        // A child interpreter is another interpreter of the *same* Tcl build,
        // not a different release — C compiles one library in, so every child
        // reports and behaves as its parent's release. Inherited before the
        // globals are written, so the child's `tcl_version`/`tcl_patchLevel`
        // and its namespace-scope variable resolution both agree with the
        // parent. Resolution still runs against the child's *own* global
        // namespace: the rule is shared, the variables are not. The whole
        // profile is inherited, not just the release, so a child's
        // command-surface availability gate agrees too.
        child.set_dialect_profile(self.dialect_profile());
        let initialisation = bootstrap
            .map(|bootstrap| bootstrap.child_initialisation())
            .filter(|initialisation| initialisation.initialises_jim_static_extensions());
        let initialisation = initialisation.map_or(Ok(()), |initialisation| {
            self.initialise_original_jim_child(&mut child, initialisation)
        });
        if self.host_refusal_pending() {
            return name;
        }
        if child.host_refusal_pending() {
            self.transport_host_refusal_from(&child);
            return name;
        }
        if let Err(error) = initialisation {
            self.report_cmd_error(error.into());
            return name;
        }
        // Inherited logical ingress can differ from the fresh child constructor.
        // Invalidate that child's actual context before changing any provider.
        if child.logical_expression_parse_provider.get()
            != self.logical_expression_parse_provider.get()
            || child.logical_source_word_provider.get() != self.logical_source_word_provider.get()
            || child.logical_eval_object_provider.get() != self.logical_eval_object_provider.get()
        {
            child.invalidate_native_operation_context();
            child.invalidate_interpreter_policy();
        }
        child
            .logical_expression_parse_provider
            .set(self.logical_expression_parse_provider.get());
        child
            .logical_source_word_provider
            .set(self.logical_source_word_provider.get());
        child
            .logical_eval_object_provider
            .set(self.logical_eval_object_provider.get());
        let pin = self.0.pin.borrow().clone();
        child.install_pin(pin);
        child
            .channels
            .borrow_mut()
            .share_process_state_from(&self.channels.borrow());
        // The child owns its selected core root allocations. Jim's child API
        // additionally initializes its supported static distribution and copies
        // its designated parent globals; C library initialisation is separate.
        // `interp debug -frame` is seeded from the creating interp's
        // `env(TCL_INTERP_DEBUG_FRAME)` (C's `Tcl_CreateChild`).
        if self
            .var_get_elem(b"env", b"TCL_INTERP_DEBUG_FRAME")
            .map(|value| {
                match crate::typed_value::native_boolean(value, self.native_invocation_dialect()) {
                    Ok(boolean) => boolean,
                    Err(error) => {
                        if let Some(refusal) = error.native_access_refusal() {
                            self.refuse_native_access(refusal);
                        }
                        false
                    }
                }
            })
            .unwrap_or(false)
        {
            child.0.debug_frame.set(true);
            child.invalidate_interpreter_policy();
        }
        if self.host_refusal_pending() {
            return name;
        }
        let command = ChildInterpreterCommand::new(name.clone(), child.clone());
        self.children
            .borrow_mut()
            .insert(name.clone(), child.clone());
        let (namespace, tail) = if self.invocation_string_protocol().is_some_and(|protocol| {
            !matches!(
                protocol,
                tcl_syntax::native_string::NativeStringProtocol::Jim084
            )
        }) {
            self.namespaces
                .borrow_mut()
                .command_c_api_publication_at(self.current_ns.get(), &name)
                .expect("selected native child C command registration")
        } else {
            self.namespaces
                .borrow_mut()
                .support_command_publication_at(GLOBAL, &name)
        };
        self.bind_command_replacement(namespace, &tail, Command::ChildInterp(command));
        child.native_child_command_generation.set(
            self.namespaces
                .borrow()
                .command_generation(namespace, &tail),
        );
        name
    }

    /// Whether a child interpreter `name` exists.
    pub(crate) fn child_exists(&self, name: &[u8]) -> bool {
        self.children
            .borrow()
            .get(name)
            .is_some_and(|child| !child.pending_delete.get())
    }

    /// Run `f` on the child interpreter `name` (or `None` if it doesn't exist) —
    /// the mutable-access path for `interp <sub> childPath …`.
    ///
    /// The child's handle is **cloned out** of the table (an `Rc` bump) so the
    /// `children` borrow is released before `f` runs. `f` may therefore re-enter
    /// `self` — a child's aliased `source`/`invokehidden` calling back to the
    /// parent — and even re-enter the same child through a fresh handle: the
    /// shared `InterpState` is reached via the `Rc` plus per-field interior
    /// mutability, never an aliased `&mut`. The child's `parent` `Weak` is set for
    /// the call so re-entrancy can reach up, and restored after.
    ///
    /// The child is marked active for the call ([`eval_active`](InterpState)), so
    /// a deleted child remains alive through this retained activation while its
    /// former parent-table entry and command token are retired independently.
    pub(crate) fn with_child<R>(
        &mut self,
        name: &[u8],
        f: impl FnOnce(&mut Interp) -> R,
    ) -> Option<R> {
        let mut child = self
            .children
            .borrow()
            .get(name)
            .filter(|child| !child.pending_delete.get())?
            .clone();
        // Parent association and active/deferred-delete lifecycle are part of
        // child-interpreter policy. A token minted before this entry must not
        // survive into a differently-associated child evaluation.
        child.invalidate_interpreter_policy();
        let saved_parent = child.parent.replace(Rc::downgrade(&self.0));
        child.eval_active.set(child.eval_active.get() + 1);
        let r = f(&mut child);
        child.eval_active.set(child.eval_active.get() - 1);
        *child.parent.borrow_mut() = saved_parent;
        child.invalidate_interpreter_policy();
        self.retire_pending_children();
        Some(r)
    }

    /// Run `f` against the interpreter addressed by a (possibly multi-level)
    /// path — a list of child names descending from this interp. An empty path
    /// is this interp itself; otherwise each name is resolved through
    /// [`with_child`] in turn. Returns `None` if any name in the chain is not a
    /// child of its predecessor (`interp create {a b}`, `interp eval {a b} …`).
    pub(crate) fn with_child_path<R>(
        &mut self,
        path: &[Vec<u8>],
        f: impl FnOnce(&mut Interp) -> R,
    ) -> Option<R> {
        match path {
            [] => Some(f(self)),
            [name] => self.with_child(name, f),
            [name, rest @ ..] => self
                .with_child(name, |c| c.with_child_path(rest, f))
                .flatten(),
        }
    }

    /// Make this interp "safe": hide the commands that touch the host
    /// (filesystem, processes, sockets, the interpreter itself) — the core of
    /// `interp create -safe`. This does not re-alias `source`/`load`/`file`
    /// through the Safe Base, which would need cross-interp aliases.
    pub(crate) fn make_safe(&mut self) {
        // Variable unsets below can fire callbacks. Stale existing tokens before
        // the first visibility/policy write, not after re-entrant code can run.
        self.invalidate_interpreter_policy();
        // The hide list is the registry's `Traits::SAFE_INTERP_HIDDEN` query,
        // not a name list this engine keeps: C's own set is the `CmdInfo`
        // rows lacking `CMD_IS_SAFE` plus the whole-command rows of
        // `unsafeEnsembleCommands`, and that is what the trait records.
        // `hide_command` returns `false` for a name this interpreter does not
        // carry, which is the per-release narrowing: `unload` (8.5+) and
        // `zipfs` (9.0+) are release-gated and simply are not there under an
        // older pin, so no second availability rule is needed.
        //
        // `after` / `vwait` are correctly absent from the trait — confirmed
        // present and callable inside a real safe child on tclsh 8.6.14
        // (`s eval {info commands after}` returns `after`). A hand-typed hide
        // list here would risk hiding them by mistake, breaking legitimate
        // safe-interp code that uses `after idle` / `after cancel`.
        for name in tcl_registry::safe_interp_hidden_commands() {
            self.hide_command(name.as_bytes(), name.as_bytes());
        }
        self.scrub_host_globals_for_safe();
        // A safe interp's `clock` is aliased to the parent's, so date/time
        // formatting works without the child reaching the timezone files.
        self.ns_register(
            b"clock",
            Command::ParentAlias {
                original_prefix: Rc::new(NativeAliasHeader::new(
                    self.generated_alias_prefix(b"clock", &[]),
                )),
                registration_name: Some(Rc::new(NativeAliasHeader::new(obj::Owned::fresh(
                    new_string(b"clock"),
                )))),
                publication_name: b"clock".to_vec(),
                identity: Rc::new(()),
            },
        );
        self.is_safe.set(true);
    }

    pub(super) fn scrub_host_globals_for_safe(&mut self) {
        // The shared schema owns the portable/host-revealing distinction too,
        // so installation and safe scrubbing cannot drift independently.
        for key in tcl_platform::bootstrap::safe_scrub_keys() {
            self.var_unset_elem(b"::tcl_platform", key.as_bytes());
        }
        // `Tcl_MakeSafe` unsets `env`: the host environment is not touched.
        self.disarm_env_trace();
        self.var_unset(b"::env");
        // A safe interp has no real library/package paths. The Safe Base may
        // re-virtualise `auto_path` after this scrub.
        for name in tcl_platform::bootstrap::HOST_PATH_GLOBALS {
            self.var_unset(tcl_syntax::naming::root_unrooted_key(name).as_bytes());
        }
    }

    /// Whether this interp is safe (`interp issafe`).
    pub(crate) fn is_safe(&self) -> bool {
        self.is_safe.get()
    }

    /// `interp marktrusted` — clear this interp's safe flag (a parent demoting a
    /// child from safe to trusted). Future children it creates are trusted too.
    pub(crate) fn mark_trusted(&self) {
        self.invalidate_interpreter_policy();
        self.is_safe.set(false);
    }

    /// The names of this interp's direct child interpreters (sorted).
    pub(crate) fn child_names(&self) -> Vec<Vec<u8>> {
        self.children.borrow().keys().cloned().collect()
    }

    /// Delete a child interpreter (and its command). Returns whether it existed.
    ///
    /// Its exact command token is removed and its parent-table entry retired now.
    /// An executing child is retained by the actual `with_child` activation,
    /// so the interpreter allocation survives until that eval unwinds
    /// ([`with_child`]/[`eval_in_child`]) — never while a re-entrant eval of it is
    /// still on the stack. Mirrors C's deferred `Tcl_DeleteInterp`.
    ///
    /// [`with_child`]: Interp::with_child
    /// [`eval_in_child`]: Interp::eval_in_child
    pub(crate) fn delete_child(&mut self, name: &[u8]) -> bool {
        let child = self.children.borrow().get(name).cloned();
        let Some(child) = child else {
            return false;
        };
        self.invalidate_interpreter_policy();
        let generation = child.native_child_command_generation.get();
        let placed = generation.and_then(|generation| {
            self.namespaces
                .borrow()
                .native_command_at_node(generation)
                .map(|(_, full_name)| full_name)
        });
        if let Some(full_name) = placed {
            self.delete_bound_command(&full_name);
        } else if let Some(generation) = generation {
            if self.uses_original_c_command_retirement() {
                self.finish_original_command_deletion(generation, true);
            } else {
                self.hidden
                    .borrow_mut()
                    .retain(|_, binding| binding.generation != generation);
            }
        }
        child.retire_ordinary_entry_observation();
        child.pending_delete.set(true);
        child.invalidate_interpreter_policy();
        self.retire_pending_children();
        self.invalidate_command_environment();
        true
    }
}
