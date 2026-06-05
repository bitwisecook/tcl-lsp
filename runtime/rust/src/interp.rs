//! The interpreter: `Tcl_Interp` + the eval loop + command dispatch (T1.4).
//!
//! Builds on the value model (T1.1), parse/subst (T1.2), and the frame/var
//! store (T1.3). This is the **interpreter-fallback** path — the AOT compiler
//! is the primary route (north star); this runs what isn't (yet) AOT-compiled
//! and what genuinely needs runtime interpretation (`eval $dynamic`, etc.).
//!
//! Closes the **command** half of T1.2's subst seam: a `[cmd]` substitution
//! recursively evaluates its inner script through this loop.
//!
//! ## Why no deferred-free queue
//!
//! The Zig runtime defers frees to a drain queue (`tcl_obj_drain_pending`) to
//! survive an aliasing hazard: releasing a command's argv after dispatch could
//! free the result if it aliased an argv element. We avoid the queue (and match
//! `tclObj.c`'s immediate `TclFreeObj`) because [`set_result`] **retains** the
//! result into the interp's result slot — so the slot holds an independent +1,
//! and releasing argv can never free a still-referenced result. Immediate free
//! + retain-into-result is the whole discipline.

use core::ffi::c_char;

use crate::builtins;
use crate::frame::FrameStack;
use crate::namespace::{Namespaces, NsId, RenameOutcome, GLOBAL};
use crate::obj::{self, TclObj};
use crate::parse::{self, WordBody, WordPart};

/// Tcl completion codes (`tcl.h` `TCL_OK`..`TCL_CONTINUE`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Code {
    Ok,
    Error,
    Return,
    Break,
    Continue,
}

/// A built-in command handler. Receives the full argv (`argv[0]` is the command
/// name, like Tcl's `objv`); sets the result via [`Interp::set_result`] /
/// [`Interp::set_result_bytes`] and returns a [`Code`].
pub type BuiltinFn = fn(&mut Interp, &[*mut TclObj]) -> Code;

/// A registered command. `Builtin` and the `Alias` redirect today; `Proc {
/// params, body }` (user procs, T1.5) and `External { table_index, client_data }`
/// (extension commands via `Tcl_CreateObjCommand`, §4.6/§13.2, Track 2) are the
/// next variants.
///
/// `Clone` but not `Copy`: the dispatch lookup clones the small handle out of the
/// command table (a fn-pointer copy for `Builtin`; the target name + frozen
/// prefix for `Alias`). Cloning detaches the handle from the table so dispatch
/// can mutate the interp (and the table) without holding a borrow.
#[derive(Clone)]
pub enum Command {
    /// A native Rust handler.
    Builtin(BuiltinFn),
    /// An `interp alias`: dispatch re-resolves `target` **by name, anchored at
    /// the global namespace, on every call** (so it lazily observes the target's
    /// *deletion* but does NOT follow its *rename* — the stored name simply stops
    /// resolving), then prepends the frozen `prefix` words to the caller's args.
    /// See `docs/design/runtime/rename-alias.md` §4.
    Alias {
        target: Vec<u8>,
        prefix: Vec<Vec<u8>>,
    },
    /// A `namespace import` redirect: dispatch re-resolves `source` (the source
    /// command's FQN) anchored at global and forwards the caller's argv
    /// unchanged. The importing-ns binding is transparent to callers; `namespace
    /// forget` removes redirects by matching `source`.
    Imported { source: Vec<u8> },
}

/// `Tcl_Interp`. Owns the frame stack, the command table, and the current
/// result object (a `+1` it holds; never null after `new`).
#[repr(C)]
pub struct Interp {
    pub(crate) frames: FrameStack,
    /// The command-table-as-core-service: the namespace tree + the one
    /// `resolve(currentNs, name)` resolver (T1.5).
    namespaces: Namespaces,
    /// The current namespace for command resolution (the eval context; a proc
    /// runs in its *defining* namespace — wired with procs). Global at top level.
    current_ns: NsId,
    result: *mut TclObj,
}

impl Interp {
    /// Create an interp: global frame, the built-in command set, an empty
    /// result.
    pub fn new() -> Box<Interp> {
        let result = obj::new_obj();
        // SAFETY: `result` is freshly created; the interp takes the owning ref.
        unsafe { obj::incr_ref_count(result) };
        let mut interp = Box::new(Interp {
            frames: FrameStack::new(),
            namespaces: Namespaces::new(),
            current_ns: GLOBAL,
            result,
        });
        builtins::install(&mut interp);
        interp
    }

    // -- command registry -----------------------------------------------------

    /// Register a built-in command (a possibly-qualified `name`, creating
    /// intermediate namespaces; overwrites any existing command of `name`).
    pub fn register_builtin(&mut self, name: &[u8], f: BuiltinFn) {
        self.namespaces.register(name, Command::Builtin(f));
    }

    /// Command names in the current namespace, sorted (`info commands`).
    #[must_use]
    pub fn command_names(&self) -> Vec<&[u8]> {
        self.namespaces.command_names(self.current_ns)
    }

    /// `rename old new` (or `rename old ""` to delete), relative to the current
    /// namespace. Drives the one command table; see [`Namespaces::rename`].
    pub(crate) fn rename_command(&mut self, old: &[u8], new: &[u8]) -> RenameOutcome {
        self.namespaces.rename(self.current_ns, old, new)
    }

    /// Install an `interp alias` redirect named `name` → `target ?prefix...?`.
    pub(crate) fn install_alias(&mut self, name: &[u8], target: Vec<u8>, prefix: Vec<Vec<u8>>) {
        self.namespaces
            .register(name, Command::Alias { target, prefix });
    }

    /// The `(target, prefix)` of the alias bound to `name` (the query form), or
    /// `None` if `name` resolves to something that isn't an alias.
    pub(crate) fn alias_info(&self, name: &[u8]) -> Option<(Vec<u8>, Vec<Vec<u8>>)> {
        match self.namespaces.resolve(self.current_ns, name) {
            Some(Command::Alias { target, prefix }) => Some((target, prefix)),
            _ => None,
        }
    }

    /// Delete the command bound to `name` (the alias-clear form); returns whether
    /// it existed.
    pub(crate) fn delete_command(&mut self, name: &[u8]) -> bool {
        self.namespaces.delete(self.current_ns, name)
    }

    /// Every alias command's name across the whole tree (`interp aliases`).
    pub(crate) fn alias_names(&self) -> Vec<Vec<u8>> {
        self.namespaces.alias_names()
    }

    /// The current namespace (the eval context) — for the `namespace` builtin.
    pub(crate) fn current_ns(&self) -> NsId {
        self.current_ns
    }

    /// The namespace tree (read) — for the `namespace` builtin's queries.
    pub(crate) fn namespaces(&self) -> &Namespaces {
        &self.namespaces
    }

    /// The namespace tree (mutable) — for the `namespace` builtin's mutations
    /// (`export`/`import`/`forget`/`path`).
    pub(crate) fn namespaces_mut(&mut self) -> &mut Namespaces {
        &mut self.namespaces
    }

    /// `namespace eval name body`: switch the current namespace to `name`
    /// (creating it, relative to the current ns unless `::`-anchored), evaluate
    /// `body` there, then restore. The current-ns switch is what makes commands
    /// defined in `body` land in the right table.
    pub(crate) fn ns_eval(&mut self, name: &[u8], body: &[u8]) -> Code {
        let target = self.namespaces.ensure_namespace(self.current_ns, name);
        let saved = self.current_ns;
        self.current_ns = target;
        let code = self.eval_str(body);
        self.current_ns = saved;
        code
    }

    // -- result ---------------------------------------------------------------

    /// `Tcl_SetObjResult`: retain `obj` into the result slot, release the prior.
    ///
    /// # Safety
    /// `obj` must be a live `TclObj`.
    pub unsafe fn set_obj_result(&mut self, obj: *mut TclObj) {
        let old = self.result;
        // SAFETY: `obj` live (caller); `old` is the interp's owned result.
        unsafe {
            obj::incr_ref_count(obj);
            self.result = obj;
            obj::decr_ref_count(old);
        }
    }

    /// Set the result to `obj` (the safe wrapper builtins use on objects they
    /// already hold — argv elements, or fresh objects they just minted).
    ///
    /// Not marked `unsafe`: in the runtime every `TclObj *` in flight is a live
    /// object from our single allocator (the Tcl C model), and builtins handle
    /// these pointers ubiquitously — threading `unsafe` through every call site
    /// would add noise without adding safety. The invariant is upheld by the
    /// eval loop's retain/release discipline.
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    pub fn set_result(&mut self, obj: *mut TclObj) {
        // SAFETY: builtins only pass live objects (argv elements / fresh objs).
        unsafe { self.set_obj_result(obj) }
    }

    /// Set the result to a fresh string object with `bytes`.
    pub fn set_result_bytes(&mut self, bytes: &[u8]) {
        let obj = new_string(bytes);
        // SAFETY: fresh obj; set_obj_result retains it, then we drop our 0-ref
        // (the obj was rc 0; set_obj_result took it to rc 1, interp-owned).
        unsafe { self.set_obj_result(obj) };
    }

    /// `Tcl_GetObjResult` — borrowed (interp keeps its +1).
    pub fn get_obj_result(&self) -> *mut TclObj {
        self.result
    }

    /// The current result's string bytes (copied).
    pub fn result_bytes(&self) -> Vec<u8> {
        obj_bytes(self.result)
    }

    /// Set an error result and return [`Code::Error`].
    fn error(&mut self, msg: &[u8]) -> Code {
        self.set_result_bytes(msg);
        Code::Error
    }

    // -- eval -----------------------------------------------------------------

    /// Evaluate a whole script; the result is left in the interp result. Returns
    /// the completion code of the last command (or `Ok` for an empty script).
    pub fn eval_str(&mut self, src: &[u8]) -> Code {
        let mut last = Code::Ok;
        let commands = parse::parse_script(src);
        for cmd in &commands {
            last = self.eval_command(&cmd.words);
            if last != Code::Ok {
                break; // error/return/break/continue propagate up
            }
        }
        last
    }

    /// Evaluate one already-parsed command: substitute each word (with `{*}`
    /// expansion), then dispatch.
    fn eval_command(&mut self, words: &[parse::Word]) -> Code {
        let mut argv: Vec<*mut TclObj> = Vec::new();
        for w in words {
            let obj = match self.substitute_word(&w.body) {
                Ok(o) => o, // owned (+1)
                Err(code) => {
                    release_all(&argv);
                    return code;
                }
            };
            if w.expand {
                // Split the substituted value as a list; each element is an arg.
                let bytes = obj_bytes(obj);
                unsafe { obj::decr_ref_count(obj) }; // done with the word obj
                match parse::split_list(&bytes) {
                    Ok(elems) => {
                        for e in elems {
                            let eo = new_string(&e);
                            unsafe { obj::incr_ref_count(eo) };
                            argv.push(eo);
                        }
                    }
                    Err(_) => {
                        release_all(&argv);
                        return self.error(b"list element in braces followed by junk");
                    }
                }
            } else {
                argv.push(obj); // already owned (+1)
            }
        }

        if argv.is_empty() {
            return Code::Ok;
        }

        let code = self.dispatch(&argv);
        // Safe to release argv now: a command that made an argv element its
        // result did so via set_obj_result, which holds an independent +1.
        release_all(&argv);
        code
    }

    /// Look up `argv[0]` and invoke it.
    fn dispatch(&mut self, argv: &[*mut TclObj]) -> Code {
        let name = obj_bytes(argv[0]);
        match self.namespaces.resolve(self.current_ns, &name) {
            Some(cmd) => self.invoke(cmd, argv),
            None => self.invalid_command(&name),
        }
    }

    /// Invoke an already-resolved command handle with `argv`.
    fn invoke(&mut self, cmd: Command, argv: &[*mut TclObj]) -> Code {
        match cmd {
            Command::Builtin(f) => f(self, argv),
            Command::Alias { target, prefix } => self.dispatch_alias(&target, &prefix, argv),
            Command::Imported { source } => match self.namespaces.resolve(GLOBAL, &source) {
                // Transparent redirect: forward argv unchanged to the source.
                Some(cmd) => self.invoke(cmd, argv),
                None => self.invalid_command(&source),
            },
        }
    }

    /// The alias trampoline (`docs/design/runtime/rename-alias.md` §4.2): resolve
    /// the stored `target` by name **anchored at the global namespace** (so a
    /// target deleted after the alias was created surfaces lazily here, but a
    /// *renamed* target is not followed), synthesise
    /// `[target, *prefix, *caller_tail]`, and invoke. Alias-of-alias chains fall
    /// out naturally (the resolved target may itself be an `Alias`).
    fn dispatch_alias(&mut self, target: &[u8], prefix: &[Vec<u8>], argv: &[*mut TclObj]) -> Code {
        let Some(target_cmd) = self.namespaces.resolve(GLOBAL, target) else {
            // Lazily bound: the target was deleted (or never existed).
            return self.invalid_command(target);
        };
        // Build [target, *prefix, *argv[1..]] — each element owned (+1).
        let mut new_argv: Vec<*mut TclObj> = Vec::with_capacity(prefix.len() + argv.len());
        let push_owned = |v: &mut Vec<*mut TclObj>, o: *mut TclObj| {
            // SAFETY: `o` is a live object; take the owning +1 the new argv holds.
            unsafe { obj::incr_ref_count(o) };
            v.push(o);
        };
        push_owned(&mut new_argv, new_string(target));
        for p in prefix {
            push_owned(&mut new_argv, new_string(p));
        }
        for &a in &argv[1..] {
            push_owned(&mut new_argv, a);
        }
        let code = self.invoke(target_cmd, &new_argv);
        release_all(&new_argv);
        code
    }

    /// The `invalid command name "X"` error (the resolver miss; `unknown` later).
    fn invalid_command(&mut self, name: &[u8]) -> Code {
        let mut msg = b"invalid command name \"".to_vec();
        msg.extend_from_slice(name);
        msg.push(b'"');
        self.error(&msg)
    }

    /// Substitute one word's body into an **owned** (`+1`) object.
    /// A `Variable` reference to an unset variable, or a `[cmd]` that errors,
    /// returns `Err(code)` with the interp result already set.
    fn substitute_word(&mut self, body: &WordBody) -> Result<*mut TclObj, Code> {
        match body {
            WordBody::Literal(bytes) => {
                let obj = new_string(bytes);
                unsafe { obj::incr_ref_count(obj) };
                Ok(obj)
            }
            WordBody::Parts(parts) => {
                // Object-passthrough fast path (Zig lesson #1/#4): a word that is
                // *exactly one* substitution returns that value's **object**
                // (preserving its internal rep), not a stringified copy. This is
                // what keeps `$list`→`lindex`/`llength` etc. O(1) instead of
                // re-shimmering the string each access (the hidden-O(N²) seam).
                if parts.len() == 1 {
                    match &parts[0] {
                        WordPart::Variable(v) => {
                            let index = match &v.index {
                                Some(p) => Some(self.subst_index(p)?),
                                None => None,
                            };
                            let obj = match index.as_deref() {
                                Some(key) => self.frames.get_elem(v.name, key),
                                None => self.frames.get(v.name),
                            };
                            return match obj {
                                Some(o) => {
                                    // SAFETY: `o` is a live store-owned object;
                                    // we take an owning +1 to hand to the caller.
                                    unsafe { obj::incr_ref_count(o) };
                                    Ok(o)
                                }
                                None => Err(self.no_such_variable(v.name, index.as_deref())),
                            };
                        }
                        WordPart::Command(script) => {
                            if self.eval_str(script) == Code::Error {
                                return Err(Code::Error);
                            }
                            let r = self.result;
                            // SAFETY: the interp result is live; take an owning +1.
                            unsafe { obj::incr_ref_count(r) };
                            return Ok(r);
                        }
                        // single Text/Backslash → fall through to the buffer path
                        _ => {}
                    }
                }
                let mut buf: Vec<u8> = Vec::new();
                for part in parts {
                    match part {
                        WordPart::Text(b) => buf.extend_from_slice(b),
                        WordPart::Variable(v) => {
                            let index = match &v.index {
                                Some(parts) => Some(self.subst_index(parts)?),
                                None => None,
                            };
                            match self.read_var(v.name, index.as_deref()) {
                                Some(bytes) => buf.extend_from_slice(&bytes),
                                None => return Err(self.no_such_variable(v.name, index.as_deref())),
                            }
                        }
                        WordPart::Command(script) => {
                            let code = self.eval_str(script);
                            if code == Code::Error {
                                return Err(Code::Error);
                            }
                            buf.extend_from_slice(&self.result_bytes());
                        }
                    }
                }
                let obj = new_string(&buf);
                unsafe { obj::incr_ref_count(obj) };
                Ok(obj)
            }
        }
    }

    /// Resolve a `$arr(index)` index (itself substituted) to its bytes.
    fn subst_index(&mut self, parts: &[WordPart]) -> Result<Vec<u8>, Code> {
        let mut buf = Vec::new();
        for part in parts {
            match part {
                WordPart::Text(b) => buf.extend_from_slice(b),
                WordPart::Variable(v) => {
                    let idx = match &v.index {
                        Some(p) => Some(self.subst_index(p)?),
                        None => None,
                    };
                    match self.read_var(v.name, idx.as_deref()) {
                        Some(bytes) => buf.extend_from_slice(&bytes),
                        None => return Err(self.no_such_variable(v.name, idx.as_deref())),
                    }
                }
                WordPart::Command(script) => {
                    if self.eval_str(script) == Code::Error {
                        return Err(Code::Error);
                    }
                    buf.extend_from_slice(&self.result_bytes());
                }
            }
        }
        Ok(buf)
    }

    /// Read a variable's value bytes via the frame store.
    fn read_var(&self, name: &[u8], index: Option<&[u8]>) -> Option<Vec<u8>> {
        self.frames.resolve_var_bytes(name, index)
    }

    fn no_such_variable(&mut self, name: &[u8], index: Option<&[u8]>) -> Code {
        let mut msg = b"can't read \"".to_vec();
        msg.extend_from_slice(name);
        if let Some(i) = index {
            msg.push(b'(');
            msg.extend_from_slice(i);
            msg.push(b')');
        }
        msg.extend_from_slice(b"\": no such variable");
        self.error(&msg)
    }

    /// Set an error result and return [`Code::Error`] — for builtins.
    pub(crate) fn set_error(&mut self, msg: &[u8]) -> Code {
        self.error(msg)
    }
}

/// Discard a freshly created (`rc 0`) object that is not going to be stored
/// (the error path of a builtin that already minted its result object).
pub(crate) fn drop_fresh(obj: *mut TclObj) {
    // SAFETY: `obj` is a live rc-0 object; retain-then-release frees it without
    // tripping the double-free guard (which fires on releasing at rc 0).
    unsafe {
        obj::incr_ref_count(obj);
        obj::decr_ref_count(obj);
    }
}

impl Drop for Interp {
    fn drop(&mut self) {
        // Release the result; the FrameStack field drops afterwards, releasing
        // all variable refs. The command table holds no object refs.
        // SAFETY: `result` is the interp's owned reference, dropped once.
        unsafe { obj::decr_ref_count(self.result) };
        self.result = core::ptr::null_mut();
    }
}

// ---------------------------------------------------------------------------
// Object byte helpers.
// ---------------------------------------------------------------------------

/// A fresh (`rc 0`) string object holding `bytes`.
pub(crate) fn new_string(bytes: &[u8]) -> *mut TclObj {
    // SAFETY: `bytes` is a valid readable slice.
    unsafe { obj::new_string_obj(bytes.as_ptr() as *const c_char, bytes.len() as obj::TclSize) }
}

/// Copy an object's string rep (shimmering if needed) into owned bytes.
pub(crate) fn obj_bytes(obj: *mut TclObj) -> Vec<u8> {
    // SAFETY: `obj` is a live object; `get_string` returns a borrowed pointer
    // into its (possibly just-generated) string rep, which we copy immediately.
    unsafe {
        let mut len: obj::TclSize = 0;
        let p = obj::get_string(obj, &mut len);
        if p.is_null() {
            return Vec::new();
        }
        core::slice::from_raw_parts(p as *const u8, len as usize).to_vec()
    }
}

/// Release each object in `objs` (each holds a `+1` taken by `eval_command`).
fn release_all(objs: &[*mut TclObj]) {
    for &o in objs {
        // SAFETY: every argv element was retained when pushed; this balances it.
        unsafe { obj::decr_ref_count(o) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::counters;

    fn leak_free(body: impl FnOnce(&mut Interp)) {
        counters::reset();
        {
            let mut interp = Interp::new();
            body(&mut interp);
        }
        assert_eq!(
            counters::finalize(),
            0,
            "residual: {} objs, {} bufs",
            counters::live_objs(),
            counters::live_bufs()
        );
        assert_eq!(counters::double_free_count(), 0);
    }

    #[test]
    fn set_and_read_back() {
        leak_free(|i| {
            assert_eq!(i.eval_str(b"set x 5"), Code::Ok);
            assert_eq!(i.result_bytes(), b"5");
            assert_eq!(i.eval_str(b"set y $x"), Code::Ok);
            assert_eq!(i.result_bytes(), b"5");
        });
    }

    #[test]
    fn command_substitution_closes_the_seam() {
        // [set y 42] evaluates the inner command; x becomes its result.
        leak_free(|i| {
            assert_eq!(i.eval_str(b"set x [set y 42]"), Code::Ok);
            assert_eq!(i.result_bytes(), b"42");
            assert_eq!(i.eval_str(b"set x"), Code::Ok);
            assert_eq!(i.result_bytes(), b"42");
        });
    }

    #[test]
    fn incr_arithmetic() {
        leak_free(|i| {
            i.eval_str(b"set n 10");
            assert_eq!(i.eval_str(b"incr n"), Code::Ok);
            assert_eq!(i.result_bytes(), b"11");
            assert_eq!(i.eval_str(b"incr n 5"), Code::Ok);
            assert_eq!(i.result_bytes(), b"16");
            // incr of an unset var starts from 0
            assert_eq!(i.eval_str(b"incr fresh"), Code::Ok);
            assert_eq!(i.result_bytes(), b"1");
        });
    }

    #[test]
    fn undefined_variable_is_an_error() {
        leak_free(|i| {
            assert_eq!(i.eval_str(b"set y $nope"), Code::Error);
            assert_eq!(i.result_bytes(), b"can't read \"nope\": no such variable");
        });
    }

    #[test]
    fn unknown_command_is_an_error() {
        leak_free(|i| {
            assert_eq!(i.eval_str(b"frobnicate a b"), Code::Error);
            assert_eq!(i.result_bytes(), b"invalid command name \"frobnicate\"");
        });
    }

    #[test]
    fn absolute_qualified_command_resolves() {
        leak_free(|i| {
            // `::set` is the global `set`, reached via the namespace resolver.
            assert_eq!(i.eval_str(b"::set x 5"), Code::Ok);
            assert_eq!(i.result_bytes(), b"5");
            assert_eq!(i.eval_str(b"set y $x"), Code::Ok);
            assert_eq!(i.result_bytes(), b"5");
            // an unknown namespace qualifier is an error
            assert_eq!(i.eval_str(b"::nosuch::cmd a"), Code::Error);
            assert_eq!(i.result_bytes(), b"invalid command name \"::nosuch::cmd\"");
        });
    }

    #[cfg(have_tommath)]
    #[test]
    fn expr_command_end_to_end() {
        leak_free(|i| {
            // braced arithmetic with precedence
            assert_eq!(i.eval_str(b"expr {2 + 3 * 4}"), Code::Ok);
            assert_eq!(i.result_bytes(), b"14");
            // a variable resolved through the frame store (object-preserving)
            assert_eq!(i.eval_str(b"set x 20"), Code::Ok);
            assert_eq!(i.eval_str(b"expr {$x * 2 + 2}"), Code::Ok);
            assert_eq!(i.result_bytes(), b"42");
            // overflow promotes to a bignum, then a command substitution feeds back in
            assert_eq!(i.eval_str(b"expr {2 ** 64}"), Code::Ok);
            assert_eq!(i.result_bytes(), b"18446744073709551616");
            assert_eq!(i.eval_str(b"expr {[set x] < 100}"), Code::Ok);
            assert_eq!(i.result_bytes(), b"1");
            // divide by zero surfaces the verbatim error
            assert_eq!(i.eval_str(b"expr {1 / 0}"), Code::Error);
            assert_eq!(i.result_bytes(), b"divide by zero");
        });
    }

    #[cfg(have_tommath)]
    #[test]
    fn incr_promotes_to_bignum() {
        leak_free(|i| {
            // incr starts at 0 for an unset var
            assert_eq!(i.eval_str(b"incr n"), Code::Ok);
            assert_eq!(i.result_bytes(), b"1");
            // incr past a wide promotes to a bignum (never wraps)
            assert_eq!(i.eval_str(b"set big 9223372036854775807"), Code::Ok); // i64::MAX
            assert_eq!(i.eval_str(b"incr big"), Code::Ok);
            assert_eq!(i.result_bytes(), b"9223372036854775808");
            // incrementing a bignum cell keeps working, and demotes when it fits
            assert_eq!(i.eval_str(b"incr big -1"), Code::Ok);
            assert_eq!(i.result_bytes(), b"9223372036854775807");
            // a non-integer value is rejected verbatim
            assert_eq!(i.eval_str(b"set f 1.5"), Code::Ok);
            assert_eq!(i.eval_str(b"incr f"), Code::Error);
            assert_eq!(i.result_bytes(), b"expected integer but got \"1.5\"");
        });
    }

    #[test]
    fn array_element_via_set_and_subst() {
        leak_free(|i| {
            assert_eq!(i.eval_str(b"set a(k) hello"), Code::Ok);
            assert_eq!(i.eval_str(b"set out $a(k)"), Code::Ok);
            assert_eq!(i.result_bytes(), b"hello");
        });
    }

    #[test]
    fn expand_marker_splits_arguments() {
        // A recording builtin proves {*} produced the right argv.
        leak_free(|i| {
            fn record(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
                let joined: Vec<Vec<u8>> = argv[1..].iter().map(|&o| obj_bytes(o)).collect();
                interp.set_result_bytes(&joined.join(&b'|'));
                Code::Ok
            }
            i.register_builtin(b"record", record);
            i.eval_str(b"set lst {a b c}");
            assert_eq!(i.eval_str(b"record {*}$lst tail"), Code::Ok);
            assert_eq!(i.result_bytes(), b"a|b|c|tail");
        });
    }

    #[test]
    fn return_sets_code() {
        leak_free(|i| {
            assert_eq!(i.eval_str(b"return done"), Code::Return);
            assert_eq!(i.result_bytes(), b"done");
        });
    }

    #[test]
    fn multi_command_script() {
        leak_free(|i| {
            assert_eq!(i.eval_str(b"set a 1; set b 2\nset c $a$b"), Code::Ok);
            assert_eq!(i.result_bytes(), b"12");
        });
    }
}
