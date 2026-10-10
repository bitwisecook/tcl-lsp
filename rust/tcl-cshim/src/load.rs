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

//! The host's opt-in `load`: Tcl's `load` command over the extensions the host
//! has linked in.
//!
//! The shim is linked, not loaded against a stub table, so there is no shared
//! library for `load` to open. What a host can offer a script is the Tcl
//! 9 model of a *static library*: an entry point the program carries, named by
//! its prefix (`Pkga`, whose entry point is `Pkga_Init`), which `load {} Pkga`
//! initialises into the interpreter. [`StaticExtensions`] is that table as a
//! host command a host gives an engine, and it answers what `load` answers: a
//! file name stands for the prefix Tcl would guess from it, so an unchanged
//! `package ifneeded pkga 1.0 [list load [file join $dir libpkga.so] Pkga]`
//! reaches the table; the file itself is never opened.
//!
//! **Trust.** Putting an entry point in a table is the act of trusting native
//! code, so building a table is `unsafe` ([`StaticExtensions::new`]) exactly as
//! [`crate::Interp::load_static`] is. The command exists on an engine only
//! because the host registered it: no script, pack or hook body can say so.
//!
//! A library loads once into the interpreter holding this command; another
//! load of its prefix is a no-op. Provided packages and loaded-library records
//! reach the engine through its explicit doors when those doors are available.
//! The shim also retains its own provided-package records.

use std::cell::RefCell;
use std::collections::BTreeSet;
use std::rc::Rc;

use tcl_cmd_core::prefix::OptionTable;
use tcl_engine_api::{CommandRegistrar, EngineError, HostCommand, HostOutcome, Value};

use crate::obj::Obj;
use crate::state::{InitProc, InterpState};
use crate::{LoadError, run_init};

/// The name the command is registered under.
pub(crate) const COMMAND: &str = "load";

const OPTIONS: &[&str] = &["-global", "-lazy", "--"];

const USAGE: &str =
    "wrong # args: should be \"load ?-global? ?-lazy? ?--? fileName ?prefix? ?interp?\"";

/// The extensions a host has linked in and lets a script `load`, as the
/// `load` command: register it with [`crate::Interp::enable_static_extensions`],
/// or with `define_command("load", Rc::new(extensions))` on an engine the host
/// drives itself.
///
/// Cloning is not offered: the command remembers which prefixes it has
/// loaded, and two copies would load one twice.
pub struct StaticExtensions {
    table: &'static [(&'static str, InitProc)],
    state: Rc<InterpState>,
    loaded: RefCell<BTreeSet<&'static str>>,
}

impl StaticExtensions {
    /// A `load` over `table`: each entry is a prefix and the entry point
    /// ([`InitProc`]) `load` calls for it. The first entry of a prefix wins.
    ///
    /// # Safety
    ///
    /// Every entry point in `table` must be a package entry point written
    /// against `runtime/rust/include/tcl.h`: the shim contains Rust panics, not C
    /// undefined behaviour. Building the table is the act of trusting native
    /// code, as calling [`crate::Interp::load_static`] is.
    #[must_use]
    pub unsafe fn new(table: &'static [(&'static str, InitProc)]) -> Self {
        Self {
            table,
            state: InterpState::new_shared(),
            loaded: RefCell::new(BTreeSet::new()),
        }
    }

    /// The extension this crate's build script compiled in: the test extension
    /// `Pkga` (`tests/c/pkga.c`), which `tclsh` links the same way in a test
    /// build. The table is empty where no C compiler built it (Windows).
    ///
    /// Referencing this is what makes a linker carry the extension: a program
    /// that does not call it carries nothing.
    #[must_use]
    pub fn bundled() -> Self {
        #[cfg(cshim_c_tests)]
        static TABLE: &[(&str, InitProc)] = &[("Pkga", bundled::pkga_init)];
        #[cfg(not(cshim_c_tests))]
        static TABLE: &[(&str, InitProc)] = &[];
        // SAFETY: the table holds the shim's own test extension, written against
        // the shim's header and built by this crate's build script.
        unsafe { Self::new(TABLE) }
    }

    /// The prefixes a script can load, in table order.
    pub fn prefixes(&self) -> impl Iterator<Item = &'static str> {
        self.table.iter().map(|(prefix, _)| *prefix)
    }

    /// The same `load`, registering into `state` and not a state of its own.
    pub(crate) fn sharing(mut self, state: Rc<InterpState>) -> Self {
        self.state = state;
        self
    }

    fn error(&self, error: LoadError) -> EngineError {
        match error {
            // What Tcl does with a failed entry point: the interpreter's result
            // and error code are the `load`'s own.
            LoadError::InitFailed { message, .. } => EngineError::ScriptBytes {
                message,
                code: self.state.error_code_bytes(),
                options: None,
            },
            LoadError::Crashed(payload) => EngineError::Crashed(payload),
            LoadError::Engine(error) => error,
        }
    }
}

impl HostCommand for StaticExtensions {
    /// A load publishes the commands the entry point registers, which only the
    /// engine's registration door can do, so it is refused without one.
    fn invoke(&self, _arguments: &[Value]) -> Result<HostOutcome, EngineError> {
        Err(EngineError::Unsupported(
            "loading without the engine's registration door",
        ))
    }

    fn invoke_with_registrar(
        &self,
        registrar: &mut dyn CommandRegistrar,
        arguments: &[Value],
    ) -> Result<HostOutcome, EngineError> {
        let words: Vec<String> = arguments
            .iter()
            .map(|argument| {
                Obj::from_value(argument)?
                    .text()
                    .map_err(|error| EngineError::ExecutionRefusal(error.to_string()))
            })
            .collect::<Result<_, _>>()?;
        let request = Request::parse(&words)?;
        let prefix = request.prefix()?;
        let Some(&(name, init)) = self.table.iter().find(|(known, _)| *known == prefix) else {
            return Err(if request.file.is_empty() {
                failure(
                    format!("no library with prefix \"{prefix}\" is loaded statically"),
                    Some("TCL OPERATION LOAD NOTSTATIC"),
                )
            } else {
                failure(
                    format!(
                        "couldn't load file \"{}\": no extension with the prefix \"{prefix}\" is \
                         linked into this program",
                        request.file
                    ),
                    None,
                )
            });
        };
        if self.loaded.borrow().contains(name) {
            return Ok(Value::Empty.into());
        }
        // SAFETY: the table was vouched for when it was built.
        match unsafe { run_init(&self.state, registrar, init) } {
            Ok(_) => {
                self.loaded.borrow_mut().insert(name);
                // `info loaded` lists the library under the file it was loaded
                // from; an engine with no such list has nothing to say it to.
                match registrar.library_loaded(request.file, name) {
                    Ok(()) | Err(EngineError::Unsupported(_)) => Ok(Value::Empty.into()),
                    Err(error) => Err(error),
                }
            }
            Err(error) => Err(self.error(error)),
        }
    }
}

/// What a call to `load` asked for.
struct Request<'a> {
    file: &'a str,
    prefix: Option<&'a str>,
}

impl<'a> Request<'a> {
    /// `load ?-global? ?-lazy? ?--? fileName ?prefix? ?interp?`, as Tcl 9 reads
    /// it. The two flags mean something to a loader of shared libraries and
    /// nothing to one of linked ones, so they are accepted and ignored.
    fn parse(mut words: &'a [String]) -> Result<Self, EngineError> {
        let options = OptionTable::abbreviating("option", OPTIONS);
        while words.len() >= 2 && words[0].starts_with('-') {
            let index = options.index_of(words[0].as_bytes()).map_err(|message| {
                failure(
                    String::from_utf8_lossy(&message).into_owned(),
                    Some(&tcl_syntax::list::join_list([
                        "TCL", "LOOKUP", "INDEX", "option", &words[0],
                    ])),
                )
            })?;
            let option = OPTIONS[index];
            words = &words[1..];
            if option == "--" {
                break;
            }
        }
        if words.is_empty() || words.len() > 3 {
            return Err(failure(USAGE.to_owned(), Some("TCL WRONGARGS")));
        }
        let file = words[0].as_str();
        let prefix = words
            .get(1)
            .map(String::as_str)
            .filter(|prefix| !prefix.is_empty());
        if file.is_empty() && prefix.is_none() {
            return Err(failure(
                "must specify either file name or prefix".to_owned(),
                Some("TCL OPERATION LOAD NOLIBRARY"),
            ));
        }
        if let Some(interp) = words.get(2).filter(|interp| !interp.is_empty()) {
            return Err(failure(
                format!(
                    "couldn't load into interpreter \"{interp}\": this load reaches only the \
                     interpreter that holds it"
                ),
                None,
            ));
        }
        Ok(Self { file, prefix })
    }

    /// The prefix named, or the one Tcl guesses from the file name.
    fn prefix(&self) -> Result<String, EngineError> {
        if let Some(prefix) = self.prefix {
            return Ok(prefix.to_owned());
        }
        guess_prefix(self.file).ok_or_else(|| {
            failure(
                format!("cannot figure out prefix for {}", self.file),
                Some("TCL OPERATION LOAD WHATLIBRARY"),
            )
        })
    }
}

fn failure(message: String, code: Option<&str>) -> EngineError {
    EngineError::Script {
        message,
        code: code.map(str::to_owned),
    }
}

fn is_separator(c: char) -> bool {
    c == '/' || (cfg!(windows) && c == '\\')
}

/// The prefix Tcl 9 guesses from a file name when none is given
/// (`tclLoad.c`): the last element of the path, without a leading `lib` and
/// then a leading `tcl9`, up to the first character that is not a letter or an
/// underscore, with its first letter in capitals and the rest in lower case.
/// `None` when nothing is left.
fn guess_prefix(file: &str) -> Option<String> {
    let tail = file
        .trim_end_matches(is_separator)
        .rsplit(is_separator)
        .next()?;
    let tail = tail.strip_prefix("lib").unwrap_or(tail);
    let tail = tail.strip_prefix("tcl9").unwrap_or(tail);
    let end = tail
        .find(|c: char| !(c.is_alphabetic() || c == '_'))
        .unwrap_or(tail.len());
    let mut letters = tail[..end].chars();
    let first = letters.next()?;
    Some(
        first
            .to_uppercase()
            .chain(letters.flat_map(char::to_lowercase))
            .collect(),
    )
}

#[cfg(cshim_c_tests)]
mod bundled {
    use std::ffi::{c_int, c_void};

    use crate::state::InterpState;

    unsafe extern "C" {
        // Declared over `void *`: the interpreter is opaque to C, and an `extern`
        // block naming a Rust type would trip the FFI-safety lint.
        fn Pkga_Init(interp: *mut c_void) -> c_int;
    }

    pub(super) unsafe extern "C" fn pkga_init(interp: *mut InterpState) -> c_int {
        // SAFETY: the shim passes its live interpreter pointer through.
        unsafe { Pkga_Init(interp.cast::<c_void>()) }
    }
}

#[cfg(test)]
mod tests {
    //! The command over Rust-defined extensions through the shim's own exports,
    //! so every platform runs them, and a door that records what it is given.

    use std::cell::Cell;
    use std::ffi::{c_int, c_void};
    use std::rc::Rc;

    use tcl_engine_api::{CommandRegistrar, EngineError, HostCommand, Value};

    use super::{StaticExtensions, guess_prefix};
    use crate::state::{InitProc, InterpState};
    use crate::{Obj, ffi};

    thread_local! {
        static INITS: Cell<u32> = const { Cell::new(0) };
        static FAIL_NEXT: Cell<bool> = const { Cell::new(false) };
    }

    unsafe extern "C" fn answer(
        _client_data: *mut c_void,
        interp: *mut InterpState,
        _word_count: c_int,
        _words: *const *mut Obj,
    ) -> c_int {
        // SAFETY: the shim passes a live interpreter.
        unsafe { ffi::tclshim_set_result_string(interp, c"answered".as_ptr()) };
        ffi::TCL_OK
    }

    unsafe extern "C" fn demo_init(interp: *mut InterpState) -> c_int {
        INITS.with(|count| count.set(count.get() + 1));
        if FAIL_NEXT.with(Cell::take) {
            // SAFETY: as above.
            unsafe {
                ffi::tclshim_set_result_string(interp, c"no licence".as_ptr());
                ffi::tcl_set_obj_error_code(
                    interp,
                    ffi::tcl_new_string_obj(c"DEMO LICENCE".as_ptr(), -1),
                );
            }
            return ffi::TCL_ERROR;
        }
        // SAFETY: as above.
        unsafe {
            ffi::tcl_create_obj_command(
                interp,
                c"demo_one".as_ptr(),
                answer,
                std::ptr::null_mut(),
                None,
            );
            ffi::tcl_create_obj_command(
                interp,
                c"demo_two".as_ptr(),
                answer,
                std::ptr::null_mut(),
                None,
            );
            ffi::tcl_pkg_provide_ex(interp, c"demo".as_ptr(), c"1.0".as_ptr(), std::ptr::null())
        }
    }

    unsafe extern "C" fn other_init(interp: *mut InterpState) -> c_int {
        // SAFETY: as above.
        unsafe {
            ffi::tcl_create_obj_command(
                interp,
                c"other_one".as_ptr(),
                answer,
                std::ptr::null_mut(),
                None,
            );
        }
        ffi::TCL_OK
    }

    unsafe extern "C" fn panicking_init(_interp: *mut InterpState) -> c_int {
        // SAFETY: a NULL object is the defect being injected.
        let _null = unsafe { ffi::tcl_get_string(std::ptr::null_mut()) };
        ffi::TCL_OK
    }

    static TABLE: &[(&str, InitProc)] = &[
        ("Demo", demo_init),
        ("Other", other_init),
        ("Boom", panicking_init),
    ];

    /// An engine's registration door that remembers what it was asked, and
    /// refuses a name it has been told to.
    #[derive(Default)]
    struct Door {
        defined: Vec<String>,
        refuses: Option<&'static str>,
        /// Whether the engine has the package and library doors, and so what
        /// it is told.
        records: bool,
        provided: Vec<(String, String)>,
        libraries: Vec<(String, String)>,
        refuses_package: Option<&'static str>,
        refuses_library: bool,
    }

    impl CommandRegistrar for Door {
        fn command_publication_service(
            &mut self,
        ) -> Result<Rc<dyn tcl_engine_api::CommandPublicationService>, EngineError> {
            Ok(Rc::new(crate::tests::RecordingPublicationService))
        }

        fn define_prepared_command(
            &mut self,
            publication: tcl_engine_api::PreparedCommandPublication,
            command: Rc<dyn HostCommand>,
        ) -> Result<(), EngineError> {
            let name = std::str::from_utf8(&publication.key.simple)
                .map_err(|_| EngineError::ExecutionRefusal("test door requires Unicode".into()))?;
            self.define_command(name, command)
        }

        fn remove_prepared_command(
            &mut self,
            publication: tcl_engine_api::PreparedCommandPublication,
        ) -> Result<bool, EngineError> {
            let name = std::str::from_utf8(&publication.key.simple)
                .map_err(|_| EngineError::ExecutionRefusal("test door requires Unicode".into()))?;
            self.remove_command(name)
        }

        fn define_command(
            &mut self,
            name: &str,
            _command: Rc<dyn HostCommand>,
        ) -> Result<(), EngineError> {
            if self.refuses == Some(name) {
                return Err(EngineError::Unsupported("that command"));
            }
            self.defined.push(name.to_owned());
            Ok(())
        }

        fn remove_command(&mut self, _name: &str) -> Result<bool, EngineError> {
            Ok(false)
        }

        fn provide_package(&mut self, name: &str, version: &str) -> Result<(), EngineError> {
            if !self.records {
                return Err(EngineError::Unsupported("providing a package"));
            }
            if self.refuses_package == Some(name) {
                return Err(EngineError::Script {
                    message: format!("conflicting versions provided for package \"{name}\""),
                    code: Some("TCL PACKAGE VERSIONCONFLICT".to_owned()),
                });
            }
            self.provided.push((name.to_owned(), version.to_owned()));
            Ok(())
        }

        fn library_loaded(&mut self, file_name: &str, prefix: &str) -> Result<(), EngineError> {
            if !self.records {
                return Err(EngineError::Unsupported("recording a loaded library"));
            }
            if self.refuses_library {
                return Err(EngineError::Script {
                    message: "no room for another library".to_owned(),
                    code: None,
                });
            }
            self.libraries
                .push((file_name.to_owned(), prefix.to_owned()));
            Ok(())
        }
    }

    fn recording() -> Door {
        Door {
            records: true,
            ..Door::default()
        }
    }

    fn fresh() -> StaticExtensions {
        INITS.with(|count| count.set(0));
        FAIL_NEXT.with(|flag| flag.set(false));
        // SAFETY: every entry point is written against the shim's own exports.
        unsafe { StaticExtensions::new(TABLE) }
    }

    fn load(
        extensions: &StaticExtensions,
        door: &mut Door,
        words: &[&str],
    ) -> Result<Value, EngineError> {
        let arguments: Vec<Value> = words.iter().copied().map(Value::string).collect();
        extensions
            .invoke_with_registrar(door, &arguments)
            .map(|outcome| outcome.value)
    }

    fn refused(extensions: &StaticExtensions, words: &[&str]) -> (String, Option<String>) {
        let mut door = Door::default();
        let error = load(extensions, &mut door, words).expect_err("refused");
        assert!(
            door.defined.is_empty(),
            "{words:?} registered {:?}",
            door.defined
        );
        match error {
            EngineError::Script { message, code } => (message, code),
            other => panic!("{words:?}: {other:?}"),
        }
    }

    #[test]
    fn a_prefix_loads_its_extension_through_the_door() {
        let extensions = fresh();
        let mut door = Door::default();
        let answer = load(&extensions, &mut door, &["", "Demo"]).expect("loads");
        assert!(answer.is_empty(), "load answers the empty string");
        assert_eq!(door.defined, ["demo_one", "demo_two"]);
        assert_eq!(INITS.with(Cell::get), 1);
        assert_eq!(
            extensions.prefixes().collect::<Vec<_>>(),
            ["Demo", "Other", "Boom"]
        );
    }

    #[test]
    fn a_load_tells_the_engine_the_library_under_the_file_it_was_loaded_from() {
        let extensions = fresh();
        let mut door = recording();
        load(&extensions, &mut door, &["/opt/demo/libdemo.so", "Demo"]).expect("loads");
        assert_eq!(
            door.libraries,
            [("/opt/demo/libdemo.so".to_owned(), "Demo".to_owned())]
        );
        load(&extensions, &mut door, &["", "Other"]).expect("a static library loads");
        assert_eq!(
            door.libraries.last(),
            Some(&(String::new(), "Other".to_owned())),
            "its file is empty"
        );
        load(&extensions, &mut door, &["/elsewhere/libdemo.so", "Demo"]).expect("again");
        assert_eq!(
            door.libraries.len(),
            2,
            "a prefix is told once, as it loads once"
        );
    }

    #[test]
    fn a_failed_load_tells_the_engine_no_library() {
        let extensions = fresh();
        let mut door = recording();
        FAIL_NEXT.with(|flag| flag.set(true));
        load(&extensions, &mut door, &["", "Demo"]).expect_err("fails");
        assert!(door.libraries.is_empty(), "{:?}", door.libraries);
        refused_by(&extensions, &mut door, &["", "Nosuch"]);
        assert!(door.libraries.is_empty(), "{:?}", door.libraries);
    }

    #[test]
    fn an_engine_that_cannot_record_the_library_fails_the_load_with_its_error() {
        let extensions = fresh();
        let mut door = recording();
        door.refuses_library = true;
        let error = load(&extensions, &mut door, &["", "Demo"]).expect_err("fails");
        assert_eq!(
            error,
            EngineError::Script {
                message: "no room for another library".to_owned(),
                code: None,
            }
        );
    }

    fn refused_by(extensions: &StaticExtensions, door: &mut Door, words: &[&str]) {
        load(extensions, door, words).expect_err("refused");
    }

    #[test]
    fn the_packages_an_entry_point_provides_reach_the_engine_as_it_provides_them() {
        let extensions = fresh();
        let mut door = recording();
        load(&extensions, &mut door, &["", "Demo"]).expect("loads");
        assert_eq!(door.provided, [("demo".to_owned(), "1.0".to_owned())]);
    }

    #[test]
    fn a_package_the_engine_refuses_fails_the_entry_point_and_the_load() {
        let extensions = fresh();
        let mut door = recording();
        door.refuses_package = Some("demo");
        let error = load(&extensions, &mut door, &["", "Demo"]).expect_err("fails");
        assert_eq!(
            error.script_message_bytes(),
            Some(b"conflicting versions provided for package \"demo\"".as_slice())
        );
        assert_eq!(
            error.script_code_bytes(),
            Some(b"TCL PACKAGE VERSIONCONFLICT".as_slice())
        );
        assert!(door.libraries.is_empty());
        door.refuses_package = None;
        load(&extensions, &mut door, &["", "Demo"])
            .expect("a later load runs the entry point again");
        assert_eq!(INITS.with(Cell::get), 2);
    }

    #[test]
    fn a_file_name_stands_for_the_prefix_tcl_guesses_from_it() {
        for file in [
            "libdemo.so",
            "./lib/libdemo9.so",
            "/usr/lib/tcl9demo.so",
            "libtcl9demo.so",
            "DEMO.dll",
            "demo",
            "lib/demo/",
        ] {
            let extensions = fresh();
            let mut door = Door::default();
            load(&extensions, &mut door, &[file]).unwrap_or_else(|error| panic!("{file}: {error}"));
            assert_eq!(door.defined, ["demo_one", "demo_two"], "{file}");
        }
    }

    #[test]
    fn the_guess_is_the_one_tcl_makes() {
        for (file, prefix) in [
            ("libpkga.so", Some("Pkga")),
            ("pkga1.2.so", Some("Pkga")),
            ("a/b/libtcl9thread.so", Some("Thread")),
            ("libtcl8thread.so", Some("Tcl")),
            ("my_ext.so", Some("My_ext")),
            ("libÉcole.so", Some("École")),
            ("lib.so", None),
            ("9lives.so", None),
            ("libtcl9.so", None),
            ("", None),
        ] {
            assert_eq!(guess_prefix(file).as_deref(), prefix, "{file:?}");
        }
    }

    #[test]
    fn an_explicit_prefix_wins_over_the_file_name() {
        let extensions = fresh();
        let mut door = Door::default();
        load(&extensions, &mut door, &["libdemo.so", "Other"]).expect("loads");
        assert_eq!(door.defined, ["other_one"], "the prefix, not the file");
    }

    #[test]
    fn a_library_the_host_did_not_link_is_not_loaded() {
        let extensions = fresh();
        assert_eq!(
            refused(&extensions, &["nosuch"]),
            (
                "couldn't load file \"nosuch\": no extension with the prefix \"Nosuch\" is \
                 linked into this program"
                    .to_owned(),
                None
            )
        );
        assert_eq!(
            refused(&extensions, &["./libdemo.so", "Absent"]),
            (
                "couldn't load file \"./libdemo.so\": no extension with the prefix \"Absent\" \
                 is linked into this program"
                    .to_owned(),
                None
            )
        );
        assert_eq!(
            refused(&extensions, &["", "Nosuch"]),
            (
                "no library with prefix \"Nosuch\" is loaded statically".to_owned(),
                Some("TCL OPERATION LOAD NOTSTATIC".to_owned())
            )
        );
        assert_eq!(INITS.with(Cell::get), 0, "nothing was called");
    }

    #[test]
    fn a_second_load_of_a_prefix_runs_nothing() {
        let extensions = fresh();
        let mut door = Door::default();
        load(&extensions, &mut door, &["", "Demo"]).expect("loads");
        load(&extensions, &mut door, &["libdemo.so"]).expect("loads again");
        load(&extensions, &mut door, &["x", "Demo"]).expect("and again");
        assert_eq!(INITS.with(Cell::get), 1);
        assert_eq!(door.defined, ["demo_one", "demo_two"]);
        load(&extensions, &mut door, &["", "Other"]).expect("another prefix loads");
        assert_eq!(door.defined, ["demo_one", "demo_two", "other_one"]);
    }

    #[test]
    fn the_options_and_arguments_are_the_ones_load_has() {
        for words in [
            &["-global", "", "Demo"][..],
            &["-lazy", "-global", "", "Demo"],
            &["--", "", "Demo"],
            &["-g", "-l", "--", "libdemo.so"],
            &["-lazy", "--", "-odd.so", "Demo"],
            &["-global", "Demo"],
        ] {
            let extensions = fresh();
            let mut door = Door::default();
            load(&extensions, &mut door, words)
                .unwrap_or_else(|error| panic!("{words:?}: {error}"));
            assert_eq!(door.defined, ["demo_one", "demo_two"], "{words:?}");
        }

        let extensions = fresh();
        let (message, code) = refused(&extensions, &["-bogus", "", "Demo"]);
        assert_eq!(
            message,
            "bad option \"-bogus\": must be -global, -lazy, or --"
        );
        assert_eq!(code.as_deref(), Some("TCL LOOKUP INDEX option -bogus"));
        let (message, _) = refused(&extensions, &["-", "", "Demo"]);
        assert!(message.starts_with("ambiguous option \"-\""), "{message}");

        let usage =
            "wrong # args: should be \"load ?-global? ?-lazy? ?--? fileName ?prefix? ?interp?\"";
        for words in [&[][..], &["a", "b", "c", "d"], &["--", "a", "b", "c", "d"]] {
            let (message, code) = refused(&extensions, words);
            assert_eq!(message, usage, "{words:?}");
            assert_eq!(code.as_deref(), Some("TCL WRONGARGS"), "{words:?}");
        }
        // A lone word that looks like an option is the file name.
        let (message, _) = refused(&extensions, &["-global"]);
        assert_eq!(message, "cannot figure out prefix for -global");

        let (message, code) = refused(&extensions, &["", ""]);
        assert_eq!(message, "must specify either file name or prefix");
        assert_eq!(code.as_deref(), Some("TCL OPERATION LOAD NOLIBRARY"));
        let (message, code) = refused(&extensions, &["lib.so"]);
        assert_eq!(message, "cannot figure out prefix for lib.so");
        assert_eq!(code.as_deref(), Some("TCL OPERATION LOAD WHATLIBRARY"));

        let (message, _) = refused(&extensions, &["", "Demo", "child"]);
        assert!(message.contains("interpreter \"child\""), "{message}");
        let mut door = Door::default();
        load(&extensions, &mut door, &["", "Demo", ""]).expect("an empty interp is this one");
        assert_eq!(door.defined, ["demo_one", "demo_two"]);
    }

    #[test]
    fn a_failing_entry_point_is_the_error_load_reports_and_can_be_tried_again() {
        let extensions = fresh();
        FAIL_NEXT.with(|flag| flag.set(true));
        assert_eq!(
            refused(&extensions, &["", "Demo"]),
            ("no licence".to_owned(), Some("DEMO LICENCE".to_owned()))
        );
        let mut door = Door::default();
        load(&extensions, &mut door, &["", "Demo"]).expect("a failure is not a load");
        assert_eq!(
            INITS.with(Cell::get),
            2,
            "the second call ran the entry point"
        );
        assert_eq!(door.defined, ["demo_one", "demo_two"]);
    }

    #[test]
    fn a_panic_inside_the_shim_during_a_load_is_a_contained_crash() {
        let extensions = fresh();
        let mut door = Door::default();
        let error = load(&extensions, &mut door, &["", "Boom"]).expect_err("crashes");
        assert!(
            matches!(&error, EngineError::Crashed(text) if text.contains("NULL Tcl_Obj")),
            "{error:?}"
        );
        load(&extensions, &mut door, &["", "Other"]).expect("the host is still usable");
    }

    #[test]
    fn a_prefix_the_table_holds_twice_loads_its_first_entry() {
        static TWICE: &[(&str, InitProc)] = &[("Demo", other_init), ("Demo", demo_init)];
        INITS.with(|count| count.set(0));
        // SAFETY: every entry point is written against the shim's own exports.
        let extensions = unsafe { StaticExtensions::new(TWICE) };
        let mut door = Door::default();
        load(&extensions, &mut door, &["", "Demo"]).expect("loads");
        assert_eq!(door.defined, ["other_one"]);
        assert_eq!(INITS.with(Cell::get), 0, "the second entry never ran");
    }

    #[test]
    fn an_engine_that_refuses_a_command_fails_the_load_with_its_error() {
        let extensions = fresh();
        let mut door = Door {
            refuses: Some("demo_two"),
            ..Door::default()
        };
        let error = load(&extensions, &mut door, &["", "Demo"]).expect_err("the door refuses");
        assert_eq!(error, EngineError::Unsupported("that command"));
    }

    #[test]
    fn load_needs_the_registration_door() {
        let extensions = fresh();
        let error = extensions
            .invoke(&[Value::string(""), Value::string("Demo")])
            .expect_err("no door");
        assert!(matches!(error, EngineError::Unsupported(_)), "{error:?}");
        assert_eq!(INITS.with(Cell::get), 0);
    }

    #[test]
    fn the_bundled_extension_is_the_test_extension_where_a_compiler_built_it() {
        let bundled = StaticExtensions::bundled();
        let expected: &[&str] = if cfg!(cshim_c_tests) { &["Pkga"] } else { &[] };
        assert_eq!(bundled.prefixes().collect::<Vec<_>>(), expected);
    }
}
