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

//! `tcl-cshim` — the **C Tcl extension shim**: the third leg of the Tcl
//! extension interface in `docs/design/registry/spec-packs.md`, designed in
//! `docs/design/runtime/c-extension-shim.md`.
//!
//! A command written against the C Tcl API — registered with
//! `Tcl_CreateObjCommand`, reading `objv`, answering with `Tcl_SetObjResult`
//! — is hosted behind [`tcl_engine_api::Engine`] without knowing which engine
//! sits underneath:
//!
//! ```text
//!   C extension  (compiled against runtime/rust/include/tcl.h)
//!  ---------------- this crate ----------------
//!   ffi.rs   the exported Tcl_* symbols, panic-guarded
//!   obj.rs   Tcl_Obj: refcounted, dual-rep, typed across the boundary
//!   state.rs Tcl_Interp: result slot, error code, command table
//!   Interp   owns an engine; publishes C commands as HostCommands
//!  ---------------- tcl-engine-api -------------
//!   tcl-vm engine | Tcl->WASM codegen engine (later)
//! ```
//!
//! **Trust posture.** A shimmed extension is *trusted native code*: it is
//! loaded only by the host process's own configuration, through
//! [`Interp::load_static`], which is `unsafe` for exactly that reason. No
//! `.tclspec` can name one, a pack program cannot `load` one, and a hook body
//! cannot call one — the sandbox those run in has no door to this crate (see
//! `tests/sandbox_isolation.rs`). C code cannot be fuel-limited, so
//! containment stops at `catch_unwind` around every crossing; undefined
//! behaviour in the extension is outside any boundary.
//!
//! **No string round-trips.** `Tcl_NewIntObj(5)` crosses as
//! [`Value::Int`], a `Tcl_NewListObj` as [`Value::List`]; text is only used
//! where the C code made text authoritative. See [`obj`].

mod doors;
pub mod ffi;
pub mod load;
pub mod obj;
pub mod state;

use std::ffi::c_int;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Rc;

use tcl_dialect::model::Provenance;
use tcl_engine_api::{
    CommandRegistrar, CompileUnit, CompletionCode, Engine, EngineError, HostCommand, HostOutcome,
    Value,
};
use tcl_registry::model::DeclaredCommand;

pub use load::StaticExtensions;
pub use obj::{Obj, ObjRef, TclError};
use state::DoorRef;
pub use state::{CommandChange, InitProc, InterpState};

/// What a `<Pkg>_Init` call left behind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Loaded {
    /// The commands the entry point registered, sorted by name.
    pub commands: Vec<String>,
    /// The packages it provided, `(name, version)` in provision order.
    pub packages: Vec<(String, String)>,
}

impl Loaded {
    /// The commands the entry point registered, as the analyser would have them
    /// declared: each at the conservative default for a command native code
    /// registers (`tcl_registry::extension_default`), because nothing the shim
    /// is given says what a C command does to state.
    ///
    /// This is the third source an extension is described from, beside a scan of
    /// its C source and a probe of a shell that requires it: the one for a host
    /// that loads the extension in-process and so knows exactly which commands
    /// it registered. The provenance is [`Provenance::User`], the host's own
    /// configuration, since [`Interp::load_static`] is the host's act.
    #[must_use]
    pub fn declared_surface(&self) -> Vec<DeclaredCommand> {
        self.commands
            .iter()
            .map(|name| DeclaredCommand::extension(name.clone(), Vec::new(), Provenance::User))
            .collect()
    }
}

/// Why [`Interp::load_static`] failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadError {
    /// The entry point returned something other than `TCL_OK`; `message` is
    /// the interpreter result it left.
    InitFailed {
        /// The code returned.
        code: c_int,
        /// The result text.
        message: String,
    },
    /// The shim panicked during the call (a Rust-side defect, contained).
    Crashed(String),
    /// The engine refused a command registration.
    Engine(EngineError),
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InitFailed { code, message } => {
                write!(f, "extension init returned {code}: {message}")
            }
            Self::Crashed(payload) => write!(f, "shim crashed during init: {payload}"),
            Self::Engine(error) => write!(f, "engine refused registration: {error}"),
        }
    }
}

impl std::error::Error for LoadError {}

impl From<EngineError> for LoadError {
    fn from(error: EngineError) -> Self {
        Self::Engine(error)
    }
}

/// A C command published to the engine: on invocation, marshals the
/// engine's words into `objv`, runs the C procedure, and marshals the result
/// (or the error, with its code) back.
struct ShimCommand {
    state: Rc<InterpState>,
    name: String,
}

impl ShimCommand {
    /// Publish the command-table changes the C code queued (a factory's
    /// `Tcl_CreateObjCommand`, a `Tcl_DeleteCommand`) through `registrar`.
    fn publish(
        state: &Rc<InterpState>,
        registrar: &mut dyn CommandRegistrar,
    ) -> Result<Vec<CommandChange>, EngineError> {
        let changes = state.take_pending();
        for change in &changes {
            match change {
                CommandChange::Created(name) => {
                    let command = Rc::new(ShimCommand {
                        state: Rc::clone(state),
                        name: name.clone(),
                    });
                    registrar.define_command(name, command)?;
                }
                CommandChange::Deleted(name) => {
                    registrar.remove_command(name)?;
                }
            }
        }
        Ok(changes)
    }

    fn lookup_error(&self) -> EngineError {
        EngineError::Script {
            message: format!("invalid command name \"{}\"", self.name),
            code: Some(tcl_syntax::list::join_list([
                "TCL", "LOOKUP", "COMMAND", &self.name,
            ])),
        }
    }
}

impl HostCommand for ShimCommand {
    /// With the engine's door open, changes the C code made to the command
    /// table are published before the calling script's next statement — so
    /// `factory x; x` works — and the C code can read, write and unset the
    /// variables of the frame that called it and evaluate a script there
    /// (`Tcl_GetVar2Ex`, `Tcl_ObjSetVar2`, `Tcl_UnsetVar2`, `Tcl_EvalObjEx`). An
    /// engine that does not open the door leaves the first to [`Interp::sync`]
    /// and gives the second nothing to reach.
    fn invoke_with_registrar(
        &self,
        registrar: &mut dyn CommandRegistrar,
        arguments: &[Value],
    ) -> Result<HostOutcome, EngineError> {
        let answer = {
            let mut door = DoorRef::new(&mut *registrar);
            let _open = self.state.open_door(&mut door);
            self.invoke(arguments)
        };
        Self::publish(&self.state, registrar)?;
        answer
    }

    fn invoke(&self, arguments: &[Value]) -> Result<HostOutcome, EngineError> {
        let Some(entry) = self.state.command(&self.name) else {
            return Err(self.lookup_error());
        };
        let mut objv: Vec<ObjRef> = Vec::with_capacity(arguments.len() + 1);
        objv.push(ObjRef::new(Obj::from_text(&self.name)));
        objv.extend(
            arguments
                .iter()
                .map(|argument| ObjRef::new(Obj::from_value(argument))),
        );
        let raw: Vec<*mut Obj> = objv.iter().map(ObjRef::as_ptr).collect();
        let word_count = c_int::try_from(raw.len()).map_err(|_| EngineError::Script {
            message: "too many arguments for a C command".to_owned(),
            code: None,
        })?;

        self.state.reset_result();
        let state_ptr = Rc::as_ptr(&self.state).cast_mut();
        let code = catch_unwind(AssertUnwindSafe(|| {
            // SAFETY: `entry` came from `Tcl_CreateObjCommand`, so `proc` is
            // the extension's own command procedure and `client_data` what it
            // registered; `state_ptr` is live for the whole call because
            // `self.state` holds it; `raw` holds `word_count` live objects that
            // `objv` keeps alive until after the call returns.
            unsafe { (entry.proc)(entry.client_data, state_ptr, word_count, raw.as_ptr()) }
        }));
        let code = match code {
            Ok(code) => code,
            Err(payload) => return Err(EngineError::Crashed(panic_text(payload.as_ref()))),
        };
        if let Some(panic) = ffi::take_panic() {
            return Err(EngineError::Crashed(panic));
        }
        if let Some(fatal) = self.state.take_fatal() {
            return Err(fatal);
        }
        drop(objv);

        let result = self.state.result();
        // Every code but `TCL_ERROR` crosses as the code it is, and the engine
        // does with each what Tcl does: the calling procedure returns, the
        // enclosing loop ends or goes on, a code of the command's own reaches
        // the `catch` that reports it.
        match CompletionCode::from_int(code) {
            // A `TCL_RETURN` keeps the options of the `return` the last
            // evaluation ended in, as C Tcl's interpreter does.
            Some(CompletionCode::Return) => Ok(HostOutcome::returning(
                result.get().to_value(),
                self.state.take_return_options().unwrap_or(Value::Empty),
            )),
            Some(code) => Ok(HostOutcome::completing(code, result.get().to_value())),
            None => Err(EngineError::Script {
                message: result.get().text(),
                code: self.state.error_code_text(),
            }),
        }
    }
}

fn panic_text(payload: &(dyn std::any::Any + Send)) -> String {
    payload.downcast_ref::<&str>().map_or_else(
        || {
            payload
                .downcast_ref::<String>()
                .cloned()
                .unwrap_or_else(|| "panic with an unreadable payload".to_owned())
        },
        |message| (*message).to_owned(),
    )
}

/// An engine's own registration methods as the door a running command sees,
/// so [`Interp::sync`] and an in-invocation publish share one code path.
struct EngineDoor<'a, E: Engine>(&'a mut E);

impl<E: Engine> CommandRegistrar for EngineDoor<'_, E> {
    fn define_command(
        &mut self,
        name: &str,
        command: Rc<dyn HostCommand>,
    ) -> Result<(), EngineError> {
        self.0.define_command(name, command)
    }

    fn remove_command(&mut self, name: &str) -> Result<bool, EngineError> {
        self.0.remove_command(name)
    }

    fn provide_package(&mut self, name: &str, version: &str) -> Result<(), EngineError> {
        self.0.provide_package(name, version)
    }

    fn library_loaded(&mut self, file_name: &str, prefix: &str) -> Result<(), EngineError> {
        self.0.library_loaded(file_name, prefix)
    }

    fn variable(&mut self, name: &str) -> Result<Value, EngineError> {
        self.0.variable(name)
    }

    fn set_variable(&mut self, name: &str, value: Value) -> Result<(), EngineError> {
        self.0.set_variable(name, value)
    }

    fn unset_variable(&mut self, name: &str) -> Result<(), EngineError> {
        self.0.unset_variable(name)
    }

    fn eval_in_invocation(&mut self, script: &str) -> Result<HostOutcome, EngineError> {
        self.0.eval_in_invocation(script)
    }
}

/// Call `init` against `state` and publish what it registered through
/// `registrar`: the part of a load that does not depend on who owns the engine,
/// so [`Interp::load_static`] (the engine in hand) and the `load` command
/// ([`StaticExtensions`], the engine's registration door in hand) share it.
///
/// # Safety
///
/// As [`Interp::load_static`].
pub(crate) unsafe fn run_init(
    state: &Rc<InterpState>,
    registrar: &mut dyn CommandRegistrar,
    init: InitProc,
) -> Result<Loaded, LoadError> {
    let state_ptr = Rc::as_ptr(state).cast_mut();
    state.reset_result();
    let code = {
        let mut door = DoorRef::new(&mut *registrar);
        let _open = state.open_door(&mut door);
        // SAFETY: the caller vouches for `init`; `state_ptr` is live.
        catch_unwind(AssertUnwindSafe(|| unsafe { init(state_ptr) }))
    };
    let code = match code {
        Ok(code) => code,
        Err(payload) => return Err(LoadError::Crashed(panic_text(payload.as_ref()))),
    };
    if let Some(panic) = ffi::take_panic() {
        return Err(LoadError::Crashed(panic));
    }
    if let Some(fatal) = state.take_fatal() {
        return Err(LoadError::Engine(fatal));
    }
    if code != ffi::TCL_OK {
        return Err(LoadError::InitFailed {
            code,
            message: state.result().get().text(),
        });
    }
    let mut commands: Vec<String> = ShimCommand::publish(state, registrar)?
        .into_iter()
        .filter_map(|change| match change {
            CommandChange::Created(name) => Some(name),
            CommandChange::Deleted(_) => None,
        })
        .collect();
    commands.sort();
    Ok(Loaded {
        commands,
        packages: state.provided_packages(),
    })
}

/// A shim interpreter: an engine plus the `Tcl_Interp` state C code sees.
///
/// Generic over the engine rather than boxing a trait object because
/// [`Engine::Handle`] is an associated type; the C side never sees the
/// engine, only the [`InterpState`] this owns.
pub struct Interp<E: Engine> {
    engine: E,
    state: Rc<InterpState>,
}

impl<E: Engine> Interp<E> {
    /// A shim interpreter over `engine`, with no commands yet.
    pub fn new(engine: E) -> Self {
        Self {
            engine,
            state: InterpState::new_shared(),
        }
    }

    /// The `Tcl_Interp *` C code is given. Valid for the life of this value.
    #[must_use]
    pub fn raw(&self) -> *mut InterpState {
        Rc::as_ptr(&self.state).cast_mut()
    }

    /// Call an extension's `<Pkg>_Init` and publish what it registered.
    ///
    /// # Safety
    ///
    /// `init` must be a package entry point written against
    /// `runtime/rust/include/tcl.h`: the shim contains Rust panics, not C undefined
    /// behaviour. Calling this is the act of trusting native code.
    pub unsafe fn load_static(&mut self, init: InitProc) -> Result<Loaded, LoadError> {
        // SAFETY: the caller vouches for `init`.
        unsafe { run_init(&self.state, &mut EngineDoor(&mut self.engine), init) }
    }

    /// Give the engine a `load` command over `extensions`, the extensions the
    /// host has linked in and vouched for ([`StaticExtensions`]), sharing this
    /// interpreter's state: the commands a script loads appear in
    /// [`Self::commands`] and the packages they provide in
    /// [`Self::provided_packages`].
    ///
    /// The opt-in is the host's: nothing a script, a pack or a hook body can
    /// say registers it. An engine that also runs untrusted bodies must not
    /// be given one, since [`Engine::restrict_commands`] keeps what
    /// [`Engine::define_command`] registered.
    pub fn enable_static_extensions(
        &mut self,
        extensions: StaticExtensions,
    ) -> Result<(), EngineError> {
        self.engine.define_command(
            load::COMMAND,
            Rc::new(extensions.sharing(Rc::clone(&self.state))),
        )
    }

    /// Apply the command-table changes C code has made since the last sync
    /// to the engine, returning them.
    ///
    /// [`Self::load_static`] publishes what the entry point registered the
    /// same way, and [`Self::eval`] calls this afterwards as a backstop.
    /// During an invocation the engine's registration door
    /// ([`CommandRegistrar`]) publishes changes as they happen, so a host
    /// driving the engine directly only needs this after a change made
    /// outside any invocation.
    pub fn sync(&mut self) -> Result<Vec<CommandChange>, EngineError> {
        ShimCommand::publish(&self.state, &mut EngineDoor(&mut self.engine))
    }

    /// Run `script` on the engine as a parameterless unit and sync afterwards.
    pub fn eval(&mut self, script: &str) -> Result<Value, EngineError> {
        let handle = self.engine.compile(CompileUnit {
            name: "cshim script",
            parameters: &[],
            body: script,
        })?;
        let result = self.engine.invoke(&handle, &[]);
        self.sync()?;
        result
    }

    /// The engine.
    pub fn engine(&self) -> &E {
        &self.engine
    }

    /// The engine, mutably — for budgets or engine-specific facilities.
    pub fn engine_mut(&mut self) -> &mut E {
        &mut self.engine
    }

    /// The commands currently registered by C code, sorted.
    #[must_use]
    pub fn commands(&self) -> Vec<String> {
        self.state.command_names()
    }

    /// The packages C code has provided.
    #[must_use]
    pub fn provided_packages(&self) -> Vec<(String, String)> {
        self.state.provided_packages()
    }
}

#[cfg(test)]
mod tests {
    //! A Rust-defined "extension" through the same exports the C header
    //! declares, so the registration and marshalling story is tested on
    //! every platform, C compiler or not.

    use std::cell::Cell;
    use std::ffi::{c_int, c_void};
    use std::rc::Rc;

    use tcl_dialect::model::Provenance;
    use tcl_engine_api::{
        Budget, CommandRegistrar, CompletionCode, Engine, EngineError, HostCommand, HostOutcome,
        Value,
    };

    use super::{
        CommandChange, EngineDoor, InitProc, Interp, InterpState, LoadError, Loaded, Obj,
        StaticExtensions, ffi,
    };

    /// `echo ?arg …?` — answers with its arguments as a list.
    unsafe extern "C" fn echo(
        _client_data: *mut c_void,
        interp: *mut InterpState,
        word_count: c_int,
        words: *const *mut Obj,
    ) -> c_int {
        // SAFETY: the shim passes a live interpreter and `word_count` live
        // objects.
        unsafe {
            let list = ffi::tcl_new_list_obj(
                isize::try_from(word_count).expect("small") - 1,
                words.add(1),
            );
            ffi::tcl_set_obj_result(interp, list);
        }
        ffi::TCL_OK
    }

    /// `twice n` — doubles an integer, or reports the conversion error.
    unsafe extern "C" fn twice(
        _client_data: *mut c_void,
        interp: *mut InterpState,
        word_count: c_int,
        words: *const *mut Obj,
    ) -> c_int {
        // SAFETY: as in `echo`.
        unsafe {
            if word_count != 2 {
                ffi::tcl_wrong_num_args(interp, 1, words, c"n".as_ptr());
                return ffi::TCL_ERROR;
            }
            let mut value: c_int = 0;
            if ffi::tcl_get_int_from_obj(interp, *words.add(1), &raw mut value) != ffi::TCL_OK {
                return ffi::TCL_ERROR;
            }
            ffi::tcl_set_obj_result(interp, ffi::tcl_new_int_obj(value * 2));
        }
        ffi::TCL_OK
    }

    /// `completes code` — answers "done" and returns the integer `code`, as a C
    /// command returns the completion code it means.
    unsafe extern "C" fn completes(
        _client_data: *mut c_void,
        interp: *mut InterpState,
        word_count: c_int,
        words: *const *mut Obj,
    ) -> c_int {
        // SAFETY: as in `echo`.
        unsafe {
            let mut code: c_int = 0;
            if word_count != 2
                || ffi::tcl_get_int_from_obj(interp, *words.add(1), &raw mut code) != ffi::TCL_OK
            {
                ffi::tcl_wrong_num_args(interp, 1, words, c"code".as_ptr());
                return ffi::TCL_ERROR;
            }
            ffi::tcl_set_obj_result(interp, ffi::tcl_new_string_obj(c"done".as_ptr(), 4));
            code
        }
    }

    unsafe extern "C" fn panicking(
        _client_data: *mut c_void,
        _interp: *mut InterpState,
        _objc: c_int,
        _objv: *const *mut Obj,
    ) -> c_int {
        // SAFETY: a NULL object is the defect being injected.
        let _null = unsafe { ffi::tcl_get_string(std::ptr::null_mut()) };
        ffi::TCL_OK
    }

    unsafe extern "C" fn init(interp: *mut InterpState) -> c_int {
        // SAFETY: the shim passes a live interpreter.
        unsafe {
            ffi::tcl_create_obj_command(interp, c"echo".as_ptr(), echo, std::ptr::null_mut(), None);
            ffi::tcl_create_obj_command(
                interp,
                c"twice".as_ptr(),
                twice,
                std::ptr::null_mut(),
                None,
            );
            ffi::tcl_create_obj_command(
                interp,
                c"boom".as_ptr(),
                panicking,
                std::ptr::null_mut(),
                None,
            );
            ffi::tcl_create_obj_command(
                interp,
                c"completes".as_ptr(),
                completes,
                std::ptr::null_mut(),
                None,
            );
            ffi::tcl_pkg_provide_ex(interp, c"demo".as_ptr(), c"1.0".as_ptr(), std::ptr::null())
        }
    }

    /// `peek name` — answers the variable `name` as `Tcl_GetVar2Ex` reads it, or
    /// the error that call left.
    unsafe extern "C" fn peek(
        _client_data: *mut c_void,
        interp: *mut InterpState,
        word_count: c_int,
        words: *const *mut Obj,
    ) -> c_int {
        // SAFETY: the shim passes a live interpreter and `word_count` live
        // objects.
        unsafe {
            if word_count != 2 {
                ffi::tcl_wrong_num_args(interp, 1, words, c"name".as_ptr());
                return ffi::TCL_ERROR;
            }
            let value = ffi::tcl_get_var2_ex(
                interp,
                ffi::tcl_get_string(*words.add(1)),
                std::ptr::null(),
                ffi::TCL_LEAVE_ERR_MSG,
            );
            if value.is_null() {
                return ffi::TCL_ERROR;
            }
            ffi::tcl_set_obj_result(interp, value);
        }
        ffi::TCL_OK
    }

    unsafe extern "C" fn peek_init(interp: *mut InterpState) -> c_int {
        // SAFETY: the shim passes a live interpreter.
        unsafe {
            ffi::tcl_create_obj_command(interp, c"peek".as_ptr(), peek, std::ptr::null_mut(), None);
        }
        ffi::TCL_OK
    }

    unsafe extern "C" fn failing_init(interp: *mut InterpState) -> c_int {
        // SAFETY: as above.
        unsafe { ffi::tclshim_set_result_string(interp, c"no licence".as_ptr()) };
        ffi::TCL_ERROR
    }

    /// A recording engine: enough of the interface to see what the shim
    /// registers and to invoke it directly.
    #[derive(Default)]
    struct RecordingEngine {
        commands: Vec<(String, Rc<dyn HostCommand>)>,
        removed: Vec<String>,
    }

    impl Engine for RecordingEngine {
        type Handle = String;

        fn name(&self) -> &'static str {
            "recording"
        }

        fn define_command(
            &mut self,
            name: &str,
            command: Rc<dyn HostCommand>,
        ) -> Result<(), EngineError> {
            self.commands.push((name.to_owned(), command));
            Ok(())
        }

        fn remove_command(&mut self, name: &str) -> Result<bool, EngineError> {
            self.removed.push(name.to_owned());
            let before = self.commands.len();
            self.commands.retain(|(registered, _)| registered != name);
            Ok(self.commands.len() != before)
        }

        fn restrict_commands(&mut self, _allowed: &[&str]) -> Result<(), EngineError> {
            Ok(())
        }

        fn compile(
            &mut self,
            unit: tcl_engine_api::CompileUnit<'_>,
        ) -> Result<Self::Handle, EngineError> {
            Ok(unit.body.to_owned())
        }

        fn invoke(&mut self, _handle: &String, _arguments: &[Value]) -> Result<Value, EngineError> {
            Err(EngineError::Unsupported("scripts"))
        }

        fn set_budget(&mut self, _budget: Budget) -> Result<(), EngineError> {
            Ok(())
        }

        fn commands_spent(&self) -> Option<u64> {
            None
        }
    }

    /// An engine that implements the twins of the door's methods and records
    /// what each is asked, answering a budget to an evaluation.
    #[derive(Default)]
    struct TwinEngine {
        log: Vec<String>,
        /// What an evaluation answers; a budget when none is set.
        eval: Option<HostOutcome>,
    }

    impl Engine for TwinEngine {
        type Handle = ();

        fn name(&self) -> &'static str {
            "twin"
        }

        fn define_command(
            &mut self,
            name: &str,
            _command: Rc<dyn HostCommand>,
        ) -> Result<(), EngineError> {
            self.log.push(format!("define {name}"));
            Ok(())
        }

        fn remove_command(&mut self, name: &str) -> Result<bool, EngineError> {
            self.log.push(format!("remove {name}"));
            Ok(true)
        }

        fn provide_package(&mut self, name: &str, version: &str) -> Result<(), EngineError> {
            self.log.push(format!("provide {name} {version}"));
            Ok(())
        }

        fn library_loaded(&mut self, file_name: &str, prefix: &str) -> Result<(), EngineError> {
            self.log.push(format!("library {file_name} {prefix}"));
            Ok(())
        }

        fn variable(&mut self, name: &str) -> Result<Value, EngineError> {
            self.log.push(format!("read {name}"));
            Ok(Value::Int(3))
        }

        fn set_variable(&mut self, name: &str, value: Value) -> Result<(), EngineError> {
            self.log.push(format!("write {name} {value:?}"));
            Ok(())
        }

        fn unset_variable(&mut self, name: &str) -> Result<(), EngineError> {
            self.log.push(format!("unset {name}"));
            Ok(())
        }

        fn eval_in_invocation(&mut self, script: &str) -> Result<HostOutcome, EngineError> {
            self.log.push(format!("eval {script}"));
            self.eval.clone().ok_or(EngineError::BudgetExceeded(
                tcl_engine_api::BudgetKind::Commands,
            ))
        }

        fn restrict_commands(&mut self, _allowed: &[&str]) -> Result<(), EngineError> {
            Ok(())
        }

        fn compile(
            &mut self,
            _unit: tcl_engine_api::CompileUnit<'_>,
        ) -> Result<Self::Handle, EngineError> {
            Ok(())
        }

        fn invoke(&mut self, _handle: &(), _arguments: &[Value]) -> Result<Value, EngineError> {
            Ok(Value::Empty)
        }

        fn set_budget(&mut self, _budget: Budget) -> Result<(), EngineError> {
            Ok(())
        }

        fn commands_spent(&self) -> Option<u64> {
            None
        }
    }

    #[test]
    fn the_door_of_an_engine_in_hand_forwards_each_method_to_its_twin() {
        let mut engine = TwinEngine::default();
        let mut door = EngineDoor(&mut engine);
        door.provide_package("p", "1.0").expect("provides");
        door.library_loaded("/lib.so", "Lib").expect("records");
        assert!(matches!(door.variable("v"), Ok(Value::Int(3))));
        door.set_variable("w", Value::Int(4)).expect("writes");
        door.unset_variable("x").expect("unsets");
        assert_eq!(
            door.eval_in_invocation("body")
                .expect_err("the engine's answer"),
            EngineError::BudgetExceeded(tcl_engine_api::BudgetKind::Commands)
        );
        assert_eq!(
            engine.log,
            [
                "provide p 1.0",
                "library /lib.so Lib",
                "read v",
                "write w Int(4)",
                "unset x",
                "eval body"
            ]
        );
    }

    /// An entry point that evaluates a script, ignores how that went and answers
    /// `TCL_OK`.
    unsafe extern "C" fn evaluating_init(interp: *mut InterpState) -> c_int {
        // SAFETY: the shim passes a live interpreter.
        unsafe {
            let script = ffi::tcl_new_string_obj(c"loop".as_ptr(), 4);
            let _ = ffi::tcl_eval_obj_ex(interp, script, 0);
        }
        ffi::TCL_OK
    }

    /// `evalret script ?code?` — evaluates `script` and answers the code it had, or
    /// `code` when one is given.
    unsafe extern "C" fn evalret(
        _client_data: *mut c_void,
        interp: *mut InterpState,
        word_count: c_int,
        words: *const *mut Obj,
    ) -> c_int {
        // SAFETY: the shim passes a live interpreter and `word_count` live
        // objects.
        unsafe {
            let evaluated = ffi::tcl_eval_obj_ex(interp, *words.add(1), 0);
            let mut code = evaluated;
            if word_count == 3 {
                ffi::tcl_get_int_from_obj(interp, *words.add(2), &raw mut code);
            }
            code
        }
    }

    unsafe extern "C" fn evalret_init(interp: *mut InterpState) -> c_int {
        // SAFETY: the shim passes a live interpreter.
        unsafe {
            ffi::tcl_create_obj_command(
                interp,
                c"evalret".as_ptr(),
                evalret,
                std::ptr::null_mut(),
                None,
            );
        }
        ffi::TCL_OK
    }

    fn evaluating(answer: HostOutcome) -> (Interp<TwinEngine>, Rc<dyn HostCommand>) {
        let mut interp = Interp::new(TwinEngine {
            eval: Some(answer),
            ..TwinEngine::default()
        });
        // SAFETY: `evalret_init` is written against the shim's own exports.
        unsafe { interp.load_static(evalret_init) }.expect("loads");
        // The engine here has no command table, so the command is the one the
        // state holds, run through the door of the engine in hand.
        let command: Rc<dyn HostCommand> = Rc::new(super::ShimCommand {
            state: Rc::clone(&interp.state),
            name: "evalret".to_owned(),
        });
        (interp, command)
    }

    #[test]
    fn a_tcl_return_carries_the_options_of_the_return_the_last_evaluation_ended_in() {
        let options = Value::string("-code 1 -level 1 -errorcode {X Y}");
        let (mut interp, command) = evaluating(HostOutcome::returning(
            Value::string("msg"),
            options.clone(),
        ));
        let outcome = command
            .invoke_with_registrar(
                &mut EngineDoor(interp.engine_mut()),
                &[Value::string("return -code error -errorcode {X Y} msg")],
            )
            .expect("completes");
        assert_eq!(outcome.code, CompletionCode::Return);
        assert_eq!(outcome.value.as_str(), Some("msg"));
        assert_eq!(outcome.options.as_str(), options.as_str());

        let next = command
            .invoke_with_registrar(
                &mut EngineDoor(interp.engine_mut()),
                &[Value::string("again")],
            )
            .expect("completes");
        assert_eq!(
            next.options.as_str(),
            options.as_str(),
            "each call evaluates and so has its own"
        );
    }

    #[test]
    fn a_code_that_is_not_a_return_carries_no_options_whatever_the_evaluation_left() {
        let (mut interp, command) = evaluating(HostOutcome::returning(
            Value::string("msg"),
            Value::string("-code 1 -level 1"),
        ));
        for (code, expected) in [
            (ffi::TCL_OK, CompletionCode::Ok),
            (ffi::TCL_BREAK, CompletionCode::Break),
            (7, CompletionCode::Other(7)),
        ] {
            let outcome = command
                .invoke_with_registrar(
                    &mut EngineDoor(interp.engine_mut()),
                    &[Value::string("script"), Value::Int(i64::from(code))],
                )
                .expect("completes");
            assert_eq!(outcome.code, expected);
            assert!(outcome.options.is_empty(), "{:?}", outcome.options);
        }
    }

    #[test]
    fn an_evaluation_that_did_not_end_in_a_return_leaves_no_options_for_a_later_one() {
        let (mut interp, command) = evaluating(HostOutcome::ok(Value::string("v")));
        let outcome = command
            .invoke_with_registrar(
                &mut EngineDoor(interp.engine_mut()),
                &[
                    Value::string("script"),
                    Value::Int(i64::from(ffi::TCL_RETURN)),
                ],
            )
            .expect("completes");
        assert_eq!(outcome.code, CompletionCode::Return);
        assert!(
            outcome.options.is_empty(),
            "a return of the command's own is a plain one"
        );
    }

    #[test]
    fn a_budget_an_entry_point_swallows_fails_the_load() {
        let mut interp = Interp::new(TwinEngine::default());
        // SAFETY: `evaluating_init` is written against the shim's own exports.
        let error = unsafe { interp.load_static(evaluating_init) }.expect_err("fails");
        assert_eq!(
            error,
            LoadError::Engine(EngineError::BudgetExceeded(
                tcl_engine_api::BudgetKind::Commands
            ))
        );
        assert_eq!(interp.engine().log, ["eval loop"]);
    }

    fn loaded() -> Interp<RecordingEngine> {
        let mut interp = Interp::new(RecordingEngine::default());
        // SAFETY: `init` is written against the shim's own exports.
        let loaded = unsafe { interp.load_static(init) }.expect("loads");
        assert_eq!(loaded.commands, ["boom", "completes", "echo", "twice"]);
        assert_eq!(loaded.packages, [("demo".to_owned(), "1.0".to_owned())]);
        interp
    }

    fn command(interp: &Interp<RecordingEngine>, name: &str) -> Rc<dyn HostCommand> {
        interp
            .engine()
            .commands
            .iter()
            .find(|(registered, _)| registered == name)
            .map(|(_, command)| Rc::clone(command))
            .expect("registered")
    }

    #[test]
    fn smoke_registration_marshals_words_and_results() {
        let interp = loaded();
        let echo = command(&interp, "echo");
        let answer = echo
            .invoke(&[Value::string("a b"), Value::Int(3)])
            .expect("ok");
        let items = answer.value.as_list().expect("a typed list, not text");
        assert_eq!(items[0].as_str(), Some("a b"));
        assert!(
            matches!(items[1], Value::Int(3)),
            "ints stay ints: {items:?}"
        );

        let twice = command(&interp, "twice");
        let doubled = twice.invoke(&[Value::Int(21)]).expect("ok");
        assert!(matches!(doubled.value, Value::Int(42)));
        assert_eq!(doubled.code, CompletionCode::Ok);
        let parsed = twice.invoke(&[Value::string("0x10")]).expect("ok");
        assert!(matches!(parsed.value, Value::Int(32)));
    }

    #[test]
    fn a_c_command_returning_break_is_a_break_completion() {
        let interp = loaded();
        let completes = command(&interp, "completes");
        for (code, expected) in [
            (0, CompletionCode::Ok),
            (ffi::TCL_RETURN, CompletionCode::Return),
            (ffi::TCL_BREAK, CompletionCode::Break),
            (ffi::TCL_CONTINUE, CompletionCode::Continue),
        ] {
            let outcome = completes
                .invoke(&[Value::Int(i64::from(code))])
                .expect("ok");
            assert_eq!(outcome.code, expected, "TCL code {code}");
            assert_eq!(
                outcome.value.as_str(),
                Some("done"),
                "the result is carried with the code"
            );
        }
        assert_eq!(
            completes
                .invoke(&[Value::Int(i64::from(ffi::TCL_ERROR))])
                .expect_err("an error"),
            EngineError::Script {
                message: "done".to_owned(),
                code: None,
            },
            "an error is an error, its message the result"
        );
        let own = completes
            .invoke(&[Value::Int(9)])
            .expect("a code of its own");
        assert_eq!(own.code, CompletionCode::Other(9));
        assert_eq!(own.value.as_str(), Some("done"));
    }

    #[test]
    fn a_command_an_engine_runs_without_a_door_reaches_no_variable() {
        let mut interp = Interp::new(RecordingEngine::default());
        // SAFETY: `peek_init` is written against the shim's own exports.
        unsafe { interp.load_static(peek_init) }.expect("loads");
        let peek = command(&interp, "peek");
        let error = peek.invoke(&[Value::string("x")]).expect_err("no door");
        assert!(
            matches!(&error, EngineError::Script { message, .. }
                if message.starts_with("no engine door is open")),
            "{error:?}"
        );

        let error = peek
            .invoke_with_registrar(&mut EngineDoor(interp.engine_mut()), &[Value::string("x")])
            .expect_err("a door with no variable door");
        assert_eq!(
            error,
            EngineError::Script {
                message: "unsupported by this engine: reading a variable".to_owned(),
                code: None,
            },
            "the engine's refusal is the command's error, not an empty value"
        );
    }

    #[test]
    fn errors_cross_with_their_message_and_code() {
        let interp = loaded();
        let twice = command(&interp, "twice");
        let error = twice.invoke(&[]).expect_err("arity");
        assert_eq!(
            error,
            EngineError::Script {
                message: "wrong # args: should be \"twice n\"".to_owned(),
                code: Some("TCL WRONGARGS".to_owned()),
            }
        );
        let error = twice.invoke(&[Value::string("x")]).expect_err("conversion");
        assert_eq!(
            error,
            EngineError::Script {
                message: "expected integer but got \"x\"".to_owned(),
                code: Some("TCL VALUE NUMBER".to_owned()),
            }
        );
    }

    #[test]
    fn a_panic_inside_the_shim_is_a_contained_crash() {
        let interp = loaded();
        let boom = command(&interp, "boom");
        let error = boom.invoke(&[]).expect_err("crashes");
        assert!(matches!(&error, EngineError::Crashed(text) if text.contains("NULL Tcl_Obj")));
        assert!(
            matches!(
                command(&interp, "echo").invoke(&[]),
                Ok(HostOutcome {
                    value: Value::List(_),
                    ..
                })
            ),
            "the interpreter is still usable"
        );
    }

    #[test]
    fn a_loaded_report_declares_every_command_at_the_default_fact() {
        use tcl_registry::CommandSpec;

        let mut interp = Interp::new(RecordingEngine::default());
        // SAFETY: `init` is written against the shim's own exports.
        let loaded = unsafe { interp.load_static(init) }.expect("loads");
        let declared = loaded.declared_surface();
        let names: Vec<&str> = declared.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["boom", "completes", "echo", "twice"]);
        let default = CommandSpec::extension_default("");
        for command in &declared {
            assert_eq!(command.traits, default.traits, "{}", command.name);
            assert_eq!(
                command.side_effects, default.side_effects,
                "{}",
                command.name
            );
            assert_eq!(command.provenance(), Provenance::User);
            assert!(command.arguments.is_empty(), "no argument is known");
        }
    }

    #[test]
    fn a_load_that_registered_nothing_declares_nothing() {
        let mut interp = Interp::new(RecordingEngine::default());
        // SAFETY: as above.
        assert!(unsafe { interp.load_static(failing_init) }.is_err());
        let empty = Loaded {
            commands: Vec::new(),
            packages: Vec::new(),
        };
        assert!(empty.declared_surface().is_empty());
    }

    #[test]
    fn the_host_load_registers_with_the_engine_and_shares_the_interpreters_state() {
        static TABLE: &[(&str, InitProc)] = &[("Demo", init)];
        let mut interp = Interp::new(RecordingEngine::default());
        // SAFETY: `init` is written against the shim's own exports.
        let extensions = unsafe { StaticExtensions::new(TABLE) };
        interp
            .enable_static_extensions(extensions)
            .expect("registers");
        assert!(interp.commands().is_empty(), "nothing is loaded yet");

        let load = command(&interp, "load");
        load.invoke_with_registrar(
            &mut EngineDoor(interp.engine_mut()),
            &[Value::string(""), Value::string("Demo")],
        )
        .expect("loads");
        assert_eq!(interp.commands(), ["boom", "completes", "echo", "twice"]);
        assert_eq!(
            interp.provided_packages(),
            [("demo".to_owned(), "1.0".to_owned())]
        );
        for name in ["boom", "completes", "echo", "twice"] {
            command(&interp, name);
        }
    }

    #[test]
    fn a_failing_init_reports_its_result() {
        let mut interp = Interp::new(RecordingEngine::default());
        // SAFETY: as above.
        let error = unsafe { interp.load_static(failing_init) }.expect_err("fails");
        assert_eq!(
            error,
            LoadError::InitFailed {
                code: ffi::TCL_ERROR,
                message: "no licence".to_owned(),
            }
        );
        assert!(interp.commands().is_empty());
    }

    #[test]
    fn deleting_a_command_reaches_the_engine_and_runs_the_delete_proc() {
        thread_local! {
            static DELETED: Cell<bool> = const { Cell::new(false) };
        }
        unsafe extern "C" fn on_delete(_client_data: *mut c_void) {
            DELETED.with(|flag| flag.set(true));
        }
        let mut interp = loaded();
        // SAFETY: the interpreter pointer is live.
        unsafe {
            ffi::tcl_create_obj_command(
                interp.raw(),
                c"temp".as_ptr(),
                echo,
                std::ptr::null_mut(),
                Some(on_delete),
            );
        }
        assert_eq!(
            interp.sync().expect("syncs"),
            [CommandChange::Created("temp".to_owned())]
        );
        // SAFETY: as above.
        assert_eq!(
            unsafe { ffi::tcl_delete_command(interp.raw(), c"temp".as_ptr()) },
            0
        );
        assert!(DELETED.with(Cell::get));
        assert_eq!(
            interp.sync().expect("syncs"),
            [CommandChange::Deleted("temp".to_owned())]
        );
        assert_eq!(interp.engine().removed, ["temp"]);
        // SAFETY: as above.
        assert_eq!(
            unsafe { ffi::tcl_delete_command(interp.raw(), c"temp".as_ptr()) },
            -1
        );
    }
}
