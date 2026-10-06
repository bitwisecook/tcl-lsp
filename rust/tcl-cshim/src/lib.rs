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
//!   C extension  (compiled against include/tclshim.h)
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

pub mod ffi;
mod index_table;
pub mod obj;
pub mod state;

use std::ffi::c_int;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Rc;

use tcl_core_types::NameBytes;
use tcl_engine_api::{
    CommandPublicationKey, CommandPublicationService, CommandRegistrar, CompileUnit, Engine,
    EngineError, HostCommand, PreparedCommandPublication, Value,
};

pub use obj::{Obj, ObjRef, TclError};
pub use state::{CommandChange, InitProc, InterpState};

/// What a `<Pkg>_Init` call left behind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Loaded {
    /// The commands the entry point registered, sorted by name.
    pub commands: Vec<NameBytes>,
    /// The packages it provided, `(name, version)` in provision order.
    pub packages: Vec<(NameBytes, Vec<u8>)>,
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
        message: Vec<u8>,
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
                write!(
                    f,
                    "extension init returned {code}: {:?}",
                    message.escape_ascii().to_string()
                )
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
    name: NameBytes,
    key: CommandPublicationKey,
    entry: Rc<state::CommandEntry>,
}

impl ShimCommand {
    /// Publish the command-table changes the C code queued (a factory's
    /// `Tcl_CreateObjCommand`, a `Tcl_DeleteCommand`) through `registrar`.
    fn publish(
        state: &Rc<InterpState>,
        registrar: &mut dyn CommandRegistrar,
    ) -> Result<Vec<CommandChange>, EngineError> {
        let changes = state.take_pending();
        // A native callback can create, replace and delete the same slot before
        // returning. Only its last prepared disposition reaches the engine.
        let mut seen = std::collections::BTreeSet::new();
        let mut changes: Vec<_> = changes
            .into_iter()
            .rev()
            .filter(|change| {
                let publication = match change {
                    CommandChange::Created(publication) | CommandChange::Deleted(publication) => {
                        publication
                    }
                };
                seen.insert(publication.key.clone())
            })
            .collect();
        changes.reverse();
        for change in &changes {
            match change {
                CommandChange::Created(publication) => {
                    // Intermediate creations may already have been replaced/deleted by a callback.
                    let Some(entry) = state.command(&publication.key) else {
                        continue;
                    };
                    let publication = &entry.publication;
                    let command = Rc::new(ShimCommand {
                        state: Rc::clone(state),
                        name: NameBytes::from(publication.original.as_ref()),
                        key: publication.key.clone(),
                        entry: Rc::clone(&entry),
                    });
                    registrar.define_prepared_command(publication.clone(), command)?;
                }
                CommandChange::Deleted(publication) => {
                    if state.command(&publication.key).is_some() {
                        continue;
                    }
                    registrar.remove_prepared_command(publication.clone())?;
                }
            }
        }
        Ok(changes)
    }

    fn lookup_error(&self) -> EngineError {
        let mut message = b"invalid command name \"".to_vec();
        message.extend_from_slice(self.name.as_bytes());
        message.push(b'"');
        let code = tcl_syntax::list_result::NativeListResultSerialization::Tcl85Plus.render(&[
            b"TCL".as_slice(),
            b"LOOKUP",
            b"COMMAND",
            self.name.as_bytes(),
        ]);
        EngineError::script_bytes(message, Some(code))
    }
}

impl HostCommand for ShimCommand {
    fn argument_view(&self) -> tcl_engine_api::HostArgumentView {
        tcl_engine_api::HostArgumentView::OriginalObjects
    }

    fn retire_with_registrar(
        &self,
        registrar: &mut dyn CommandRegistrar,
    ) -> Result<(), EngineError> {
        let service = registrar.command_publication_service()?;
        let previous = self.state.replace_publication_service(Some(service));
        self.state.retire_entry(&self.key, &self.entry);
        self.state.replace_publication_service(previous);
        if let Some(error) = self.state.take_host_refusal() {
            return Err(error);
        }
        Self::publish(&self.state, registrar).map(|_| ())
    }

    /// With the engine's door open, changes the C code made to the command
    /// table are published before the calling script's next statement — so
    /// `factory x; x` works. An engine that does not open the door leaves
    /// them to [`Interp::sync`].
    fn invoke_with_registrar(
        &self,
        registrar: &mut dyn CommandRegistrar,
        arguments: &[Value],
    ) -> Result<Value, EngineError> {
        let service = registrar.command_publication_service()?;
        let previous = self.state.replace_publication_service(Some(service));
        let answer = self.invoke(arguments);
        self.state.replace_publication_service(previous);
        Self::publish(&self.state, registrar)?;
        answer
    }

    fn invoke(&self, arguments: &[Value]) -> Result<Value, EngineError> {
        let completion = self.invoke_objects(arguments, None)?;
        if completion.code == tcl_core_types::Code::Error {
            return Err(EngineError::ScriptBytes {
                message: self.state.result().get().bytes(),
                code: self.state.error_code_bytes(),
                options: Some(self.state.return_options(ffi::TCL_ERROR).bytes()),
            });
        }
        normal_callback_result(completion).and_then(|result| result.snapshot())
    }

    fn invoke_original_objects_with_registrar(
        &self,
        registrar: &mut dyn CommandRegistrar,
        arguments: &[Rc<dyn tcl_engine_api::OriginalObject>],
    ) -> Result<tcl_engine_api::OriginalObjectResult, EngineError> {
        if registrar.native_c_version() != Some(tcl_engine_api::NativeCVersion::V9_0) {
            return Err(EngineError::ExecutionRefusal(
                "callback engine differs from the C9.0 shim ABI".into(),
            ));
        }
        let service = registrar.command_publication_service()?;
        let previous = self.state.replace_publication_service(Some(service));
        let answer = self
            .invoke_objects(&[], Some(arguments))
            .and_then(normal_callback_result);
        self.state.replace_publication_service(previous);
        Self::publish(&self.state, registrar)?;
        answer
    }
    fn invoke_original_completion_with_registrar(
        &self,
        registrar: &mut dyn CommandRegistrar,
        arguments: &[Rc<dyn tcl_engine_api::OriginalObject>],
    ) -> Result<tcl_engine_api::OriginalObjectCompletion, EngineError> {
        if registrar.native_c_version() != Some(tcl_engine_api::NativeCVersion::V9_0) {
            return Err(EngineError::ExecutionRefusal(
                "callback engine differs from the C9.0 shim ABI".into(),
            ));
        }
        let service = registrar.command_publication_service()?;
        let previous = self.state.replace_publication_service(Some(service));
        let answer = self.invoke_objects(&[], Some(arguments));
        self.state.replace_publication_service(previous);
        Self::publish(&self.state, registrar)?;
        answer
    }
}

fn normal_callback_result(
    completion: tcl_engine_api::OriginalObjectCompletion,
) -> Result<tcl_engine_api::OriginalObjectResult, EngineError> {
    if completion.code.is_ok() {
        Ok(completion.result)
    } else {
        Err(EngineError::ExecutionRefusal(
            "callback requires the native completion interface".into(),
        ))
    }
}

impl ShimCommand {
    fn invoke_objects(
        &self,
        arguments: &[Value],
        originals: Option<&[Rc<dyn tcl_engine_api::OriginalObject>]>,
    ) -> Result<tcl_engine_api::OriginalObjectCompletion, EngineError> {
        let Some(entry) = self.state.command(&self.key) else {
            return Err(self.lookup_error());
        };
        if !Rc::ptr_eq(&entry, &self.entry) {
            return Err(self.lookup_error());
        }
        let mut objv: Vec<ObjRef> = Vec::with_capacity(arguments.len() + 1);
        objv.push(ObjRef::new(Obj::from_bytes(self.name.as_bytes())));
        if let Some(originals) = originals {
            self.state.prune_original_objects();
            let mut identities: Vec<((u64, u64, usize), usize)> = Vec::new();
            for original in originals {
                let scope = original.scope_identity();
                let key = (scope.0, scope.1, original.identity());
                if let Some((_, index)) = identities.iter().find(|(identity, _)| *identity == key) {
                    objv.push(objv[*index].clone());
                } else {
                    let index = objv.len();
                    objv.push(self.state.import_original(Rc::clone(original))?);
                    identities.push((key, index));
                }
            }
        } else {
            objv.extend(
                arguments
                    .iter()
                    .map(|argument| Obj::from_value(argument).map(ObjRef::new))
                    .collect::<Result<Vec<_>, _>>()?,
            );
        }
        let raw: Vec<*mut Obj> = objv.iter().map(ObjRef::as_ptr).collect();
        let word_count = c_int::try_from(raw.len()).map_err(|_| EngineError::Script {
            message: "too many arguments for a C command".to_owned(),
            code: None,
        })?;

        self.state.reset_result();
        let state_ptr = Rc::as_ptr(&self.state).cast_mut();
        let code = self
            .state
            .with_static_extension(entry.extension.clone(), || {
                catch_unwind(AssertUnwindSafe(|| {
                    // SAFETY: `entry` came from `Tcl_CreateObjCommand`, so `proc` is
                    // the extension's own command procedure and `client_data` what it
                    // registered; `state_ptr` is live for the whole call because
                    // `self.state` holds it; `raw` holds `word_count` live objects that
                    // `objv` keeps alive until after the call returns.
                    unsafe { (entry.proc)(entry.client_data, state_ptr, word_count, raw.as_ptr()) }
                }))
            });
        let code = match code {
            Ok(code) => code,
            Err(payload) => return Err(EngineError::Crashed(panic_text(payload.as_ref()))),
        };
        if let Some(panic) = ffi::take_panic() {
            return Err(EngineError::Crashed(panic));
        }
        drop(objv);
        if let Some(refusal) = self.state.take_host_refusal() {
            return Err(refusal);
        }

        let result = self.state.result();
        let options = ObjRef::new(self.state.return_options(code));
        let mut graph = obj::OriginalObjectGraph::default();
        let result = graph.export(result.get(), false)?;
        let options = graph.export(options.get(), false)?;
        Ok(tcl_core_types::Completion::new(
            tcl_core_types::Code::from_int(code),
            result,
            options,
        ))
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
    fn command_publication_service(
        &mut self,
    ) -> Result<Rc<dyn CommandPublicationService>, EngineError> {
        self.0.command_publication_service()
    }
    fn define_prepared_command(
        &mut self,
        publication: PreparedCommandPublication,
        command: Rc<dyn HostCommand>,
    ) -> Result<(), EngineError> {
        self.0.define_prepared_command(publication, command)
    }
    fn remove_prepared_command(
        &mut self,
        publication: PreparedCommandPublication,
    ) -> Result<bool, EngineError> {
        self.0.remove_prepared_command(publication)
    }

    fn define_command_bytes(
        &mut self,
        name: &[u8],
        command: Rc<dyn HostCommand>,
    ) -> Result<(), EngineError> {
        self.0.define_command_bytes(name, command)
    }

    fn remove_command_bytes(&mut self, name: &[u8]) -> Result<bool, EngineError> {
        self.0.remove_command_bytes(name)
    }

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
            state: Rc::new(InterpState::new()),
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
    /// `include/tclshim.h`: the shim contains Rust panics, not C undefined
    /// behaviour. Calling this is the act of trusting native code.
    /// The extension's code and non-temporary Index tables must remain valid
    /// through every retained object/cache, even after command or interpreter
    /// retirement. This static loader does not support unloading a library.
    pub unsafe fn load_static(&mut self, init: InitProc) -> Result<Loaded, LoadError> {
        let service = self.engine.command_publication_service()?;
        self.state.replace_publication_service(Some(service));
        let state_ptr = self.raw();
        self.state.reset_result();
        // SAFETY: the caller vouches for `init`; `state_ptr` is live.
        let extension = Rc::new(state::StaticExtensionLifetime::new(init));
        let code = self.state.with_static_extension(Some(extension), || {
            catch_unwind(AssertUnwindSafe(|| unsafe { init(state_ptr) }))
        });
        let code = match code {
            Ok(code) => code,
            Err(payload) => return Err(LoadError::Crashed(panic_text(payload.as_ref()))),
        };
        if let Some(panic) = ffi::take_panic() {
            return Err(LoadError::Crashed(panic));
        }
        if let Some(refusal) = self.state.take_host_refusal() {
            return Err(LoadError::Engine(refusal));
        }
        if code != ffi::TCL_OK {
            return Err(LoadError::InitFailed {
                code,
                message: self.state.result().get().bytes(),
            });
        }
        let mut commands: Vec<NameBytes> = self
            .sync()?
            .into_iter()
            .filter_map(|change| match change {
                CommandChange::Created(publication) => {
                    Some(NameBytes::from(publication.original.as_ref()))
                }
                CommandChange::Deleted(_) => None,
            })
            .collect();
        commands.sort();
        Ok(Loaded {
            commands,
            packages: self.state.provided_packages(),
        })
    }

    /// Apply the command-table changes C code has made since the last sync
    /// to the engine, returning them.
    ///
    /// [`Self::load_static`] calls this, and [`Self::eval`] does afterwards
    /// as a backstop. During an invocation the engine's registration door
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
    pub fn commands(&self) -> Vec<NameBytes> {
        self.state.command_names()
    }

    /// The packages C code has provided.
    #[must_use]
    pub fn provided_packages(&self) -> Vec<(NameBytes, Vec<u8>)> {
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

    use tcl_engine_api::{Budget, Engine, EngineError, HostCommand, Value};

    use super::{CommandChange, Interp, InterpState, LoadError, Obj, ffi};

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
            ffi::tcl_pkg_provide_ex(interp, c"demo".as_ptr(), c"1.0".as_ptr(), std::ptr::null())
        }
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

    struct RecordingPublicationService;

    impl tcl_engine_api::CommandPublicationService for RecordingPublicationService {
        fn observed_presence(
            &self,
            _publication: &tcl_engine_api::PreparedCommandPublication,
        ) -> Result<bool, EngineError> {
            // This service has no commands outside the shim's local table.
            Ok(false)
        }

        fn note_publication(
            &self,
            _publication: &tcl_engine_api::PreparedCommandPublication,
            _present: bool,
        ) -> Result<(), EngineError> {
            Ok(())
        }

        fn prepare(
            &self,
            original: &[u8],
            _purpose: tcl_engine_api::CommandPublicationPurpose,
        ) -> Result<tcl_engine_api::PreparedCommandPublication, EngineError> {
            // This mock only provides the root names used by the boundary tests.
            if original.windows(2).any(|pair| pair == b"::") {
                return Err(EngineError::ExecutionRefusal(
                    "recording engine has only a root slot".into(),
                ));
            }
            Ok(tcl_engine_api::PreparedCommandPublication {
                original: Rc::from(original),
                key: tcl_engine_api::CommandPublicationKey {
                    owner: 1,
                    interpreter: 0,
                    namespace: 0,
                    simple: Rc::from(original),
                },
                receipt: Rc::new(()),
            })
        }
    }

    impl Engine for RecordingEngine {
        type Handle = String;

        fn command_publication_service(
            &mut self,
        ) -> Result<Rc<dyn tcl_engine_api::CommandPublicationService>, EngineError> {
            Ok(Rc::new(RecordingPublicationService))
        }

        fn define_prepared_command(
            &mut self,
            publication: tcl_engine_api::PreparedCommandPublication,
            command: Rc<dyn HostCommand>,
        ) -> Result<(), EngineError> {
            let name = std::str::from_utf8(&publication.key.simple).map_err(|_| {
                EngineError::ExecutionRefusal("recording engine requires Unicode".into())
            })?;
            self.define_command(name, command)
        }

        fn remove_prepared_command(
            &mut self,
            publication: tcl_engine_api::PreparedCommandPublication,
        ) -> Result<bool, EngineError> {
            let name = std::str::from_utf8(&publication.key.simple).map_err(|_| {
                EngineError::ExecutionRefusal("recording engine requires Unicode".into())
            })?;
            self.remove_command(name)
        }

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

    fn loaded() -> Interp<RecordingEngine> {
        let mut interp = Interp::new(RecordingEngine::default());
        // SAFETY: `init` is written against the shim's own exports.
        let loaded = unsafe { interp.load_static(init) }.expect("loads");
        assert_eq!(loaded.commands, ["boom", "echo", "twice"]);
        assert_eq!(
            loaded.packages,
            [(tcl_core_types::NameBytes::from("demo"), b"1.0".to_vec())]
        );
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
        let items = answer.as_list().expect("a typed list, not text");
        assert_eq!(items[0].as_str(), Some("a b"));
        assert!(
            matches!(items[1], Value::Int(3)),
            "ints stay ints: {items:?}"
        );

        let twice = command(&interp, "twice");
        assert!(matches!(
            twice.invoke(&[Value::Int(21)]),
            Ok(Value::Int(42))
        ));
        assert!(matches!(
            twice.invoke(&[Value::string("0x10")]),
            Ok(Value::Int(32))
        ));
    }

    #[test]
    fn errors_cross_with_their_message_and_code() {
        let interp = loaded();
        let twice = command(&interp, "twice");
        let error = twice.invoke(&[]).expect_err("arity");
        assert_eq!(
            error.script_message_bytes(),
            Some(b"wrong # args: should be \"twice n\"".as_slice())
        );
        assert_eq!(error.script_code_bytes(), Some(b"TCL WRONGARGS".as_slice()));
        assert!(error.script_options_bytes().is_some());
        let error = twice.invoke(&[Value::string("x")]).expect_err("conversion");
        assert_eq!(
            error.script_message_bytes(),
            Some(b"expected integer but got \"x\"".as_slice())
        );
        assert_eq!(
            error.script_code_bytes(),
            Some(b"TCL VALUE NUMBER".as_slice())
        );
        assert!(error.script_options_bytes().is_some());
    }

    #[test]
    fn a_panic_inside_the_shim_is_a_contained_crash() {
        let interp = loaded();
        let boom = command(&interp, "boom");
        let error = boom.invoke(&[]).expect_err("crashes");
        assert!(matches!(&error, EngineError::Crashed(text) if text.contains("NULL Tcl_Obj")));
        assert!(
            command(&interp, "echo").invoke(&[]).unwrap().as_bytes() == Some(b"".as_slice()),
            "the interpreter is still usable"
        );
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
                message: b"no licence".to_vec(),
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
        assert!(matches!(interp.sync().expect("syncs").as_slice(),
            [CommandChange::Created(publication)] if publication.original.as_ref() == b"temp"));
        // SAFETY: as above.
        assert_eq!(
            unsafe { ffi::tcl_delete_command(interp.raw(), c"temp".as_ptr()) },
            0
        );
        assert!(DELETED.with(Cell::get));
        assert!(matches!(interp.sync().expect("syncs").as_slice(),
            [CommandChange::Deleted(publication)] if publication.original.as_ref() == b"temp"));
        assert_eq!(interp.engine().removed, ["temp"]);
        // SAFETY: as above.
        assert_eq!(
            unsafe { ffi::tcl_delete_command(interp.raw(), c"temp".as_ptr()) },
            -1
        );
    }
    #[test]
    fn native_delete_callbacks_observe_original_and_nested_generation() {
        struct DeleteContext {
            state: *const InterpState,
            nested: bool,
            saw_original: Cell<bool>,
            created: std::cell::RefCell<Option<Rc<super::state::CommandEntry>>>,
        }
        unsafe extern "C" fn inspect_and_replace(data: *mut c_void) {
            // SAFETY: the test keeps context and state alive until deletion returns.
            let context = unsafe { &*data.cast::<DeleteContext>() };
            let state = unsafe { &*context.state };
            let key = tcl_engine_api::CommandPublicationKey {
                owner: 1,
                interpreter: 0,
                namespace: 0,
                simple: Rc::from(b"target".as_slice()),
            };
            context.saw_original.set(
                state
                    .command(&key)
                    .is_some_and(|entry| entry.client_data == data),
            );
            if context.nested {
                let created = state
                    .create_command(b"target", echo, std::ptr::null_mut(), None)
                    .expect("mock root publication");
                *context.created.borrow_mut() = Some(created);
            }
        }
        for replacement in [false, true] {
            for nested in [false, true] {
                let state = InterpState::new();
                state.replace_publication_service(Some(Rc::new(RecordingPublicationService)));
                let context = DeleteContext {
                    state: &raw const state,
                    nested,
                    saw_original: Cell::new(false),
                    created: std::cell::RefCell::new(None),
                };
                let old = state
                    .create_command(
                        b"target",
                        echo,
                        std::ptr::from_ref(&context).cast_mut().cast(),
                        Some(inspect_and_replace),
                    )
                    .expect("mock root publication");
                if replacement {
                    let outer = state
                        .create_command(b"target", echo, std::ptr::null_mut(), None)
                        .expect("outer publication");
                    assert!(Rc::ptr_eq(
                        &state.command(&outer.publication.key).expect("present"),
                        &outer
                    ));
                } else {
                    assert!(state.delete_command(b"target").expect("mock root deletion"));
                    let current = state.command(&old.publication.key);
                    if nested {
                        assert!(Rc::ptr_eq(
                            &current.expect("nested survives"),
                            context.created.borrow().as_ref().expect("created")
                        ));
                    } else {
                        assert!(current.is_none());
                    }
                }
                assert!(context.saw_original.get());
            }
        }
    }
}
