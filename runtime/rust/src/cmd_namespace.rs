//! The `namespace` command (T1.5) — `eval` / `current` / `path` / `export` /
//! `import` / `forget` / `which`, plus the cheap introspection forms
//! (`exists` / `parent` / `children` / `qualifiers` / `tail`).
//!
//! Every form is a thin driver over the one namespace tree + resolver in
//! [`crate::namespace`] (the command-binding contract's A1/A2). `import`/`export`
//! match with the shared `string match` glob ([`tcl_syntax::glob`]); an `import`
//! installs a transparent [`Command::Imported`](crate::interp::Command) redirect
//! that dispatch re-resolves by the source FQN, and `forget` removes those
//! redirects by matching the same FQN.
//!
//! See `docs/design/runtime/namespace-tree.md` for the model and the deferred
//! list (variable namespaces, ensembles, traces, `namespace delete`).
//!
//! See `list.rs` for the module-level `not_unsafe_ptr_arg_deref` rationale.
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use crate::ensemble::{EnsembleConfig, EnsembleMap};
use crate::interp::{obj_bytes, Code, Command, Interp};
use crate::list;
use crate::namespace::NsId;
use crate::obj::{self, TclObj};

/// Register the `namespace` command.
pub fn install(interp: &mut Interp) {
    interp.register_builtin(b"namespace", namespace_cmd);
}

fn wrong_args(interp: &mut Interp, usage: &[u8]) -> Code {
    let mut m = b"wrong # args: should be \"".to_vec();
    m.extend_from_slice(usage);
    m.push(b'"');
    interp.set_error(&m)
}

/// Canonical `namespace` subcommands (alphabetical — the ensemble order, used
/// for unique-prefix resolution and the error message).
const NAMESPACE_SUBS: &[&[u8]] = &[
    b"children",
    b"code",
    b"current",
    b"delete",
    b"ensemble",
    b"eval",
    b"exists",
    b"export",
    b"forget",
    b"import",
    b"inscope",
    b"origin",
    b"parent",
    b"path",
    b"qualifiers",
    b"tail",
    b"upvar",
    b"which",
];

fn namespace_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 {
        return wrong_args(interp, b"namespace subcommand ?arg ...?");
    }
    // Resolve the subcommand by exact name or unambiguous prefix (the ensemble
    // contract), so e.g. `namespace exist` → `exists`.
    let raw = obj_bytes(argv[1]);
    let sub: &[u8] = if let Some(c) = NAMESPACE_SUBS.iter().find(|c| **c == raw.as_slice()) {
        c
    } else {
        let mut it = NAMESPACE_SUBS
            .iter()
            .filter(|c| c.starts_with(raw.as_slice()));
        match (it.next(), it.next()) {
            (Some(c), None) => c,
            _ => {
                let mut m = b"unknown or ambiguous subcommand \"".to_vec();
                m.extend_from_slice(&raw);
                m.extend_from_slice(b"\": must be children, code, current, delete, ensemble, eval, exists, export, forget, import, inscope, origin, parent, path, qualifiers, tail, upvar, or which");
                return interp.set_error(&m);
            }
        }
    };
    match sub {
        b"current" => ns_current(interp, argv),
        b"delete" => ns_delete(interp, argv),
        b"eval" => ns_eval(interp, argv),
        b"exists" => ns_exists(interp, argv),
        b"parent" => ns_parent(interp, argv),
        b"children" => ns_children(interp, argv),
        b"qualifiers" => ns_qualifiers(interp, argv),
        b"tail" => ns_tail(interp, argv),
        b"which" => ns_which(interp, argv),
        b"origin" => ns_origin(interp, argv),
        b"export" => ns_export(interp, argv),
        b"import" => ns_import(interp, argv),
        b"forget" => ns_forget(interp, argv),
        b"path" => ns_path(interp, argv),
        b"ensemble" => ns_ensemble(interp, argv),
        b"inscope" => ns_inscope(interp, argv),
        b"code" => ns_code(interp, argv),
        b"upvar" => ns_upvar(interp, argv),
        _ => unreachable!("subcommand resolved to a canonical name above"),
    }
}

// -- current / eval / exists / parent / children ---------------------------

/// `namespace current` — the FQN of the current namespace.
fn ns_current(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 2 {
        return wrong_args(interp, b"namespace current");
    }
    let cur = interp.current_ns();
    let name = interp.namespaces().qualified_name(cur);
    interp.set_result_bytes(&name);
    Code::Ok
}

/// `namespace delete ?name name ...?` — delete each named namespace (with its
/// children, commands, and variables). A missing namespace is an error; with no
/// names it is a no-op. Mirrors C's `NamespaceDeleteCmd` (`tclNamesp.c`).
fn ns_delete(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    for &a in &argv[2..] {
        let name = obj_bytes(a);
        // An object lives in its own namespace: deleting that namespace destroys
        // the object (running its destructor while the namespace is still
        // intact), matching C's `ObjectNamespaceDeleted`.
        let Some(ns_id) = interp.find_namespace_id(&name) else {
            let mut m = b"unknown namespace \"".to_vec();
            m.extend_from_slice(&name);
            m.extend_from_slice(b"\" in namespace delete command");
            return interp.set_error(&m);
        };
        interp.oo_namespace_deleted(ns_id);
        let cur = interp.current_ns();
        interp.namespaces_mut().delete_namespace(cur, &name);
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

/// `namespace eval name arg ?arg ...?` — evaluate a body in `name` (multiple
/// `arg`s are concatenated with spaces, like `eval`).
fn ns_eval(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 4 {
        return wrong_args(interp, b"namespace eval name arg ?arg...?");
    }
    let name = obj_bytes(argv[2]);
    let mut body = Vec::new();
    for (i, &a) in argv[3..].iter().enumerate() {
        if i > 0 {
            body.push(b' ');
        }
        body.extend_from_slice(&obj_bytes(a));
    }
    interp.ns_eval(&name, &body)
}

/// `namespace exists name` — whether the namespace resolves.
fn ns_exists(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 3 {
        return wrong_args(interp, b"namespace exists name");
    }
    let name = obj_bytes(argv[2]);
    let cur = interp.current_ns();
    let exists = interp.namespaces().find_namespace(cur, &name).is_some();
    interp.set_result_bytes(if exists { b"1" } else { b"0" });
    Code::Ok
}

/// `namespace parent ?name?` — the FQN of the (named, or current) ns's parent.
fn ns_parent(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() > 3 {
        return wrong_args(interp, b"namespace parent ?name?");
    }
    let cur = interp.current_ns();
    let ns = match resolve_arg_ns(interp, argv.get(2).copied(), cur) {
        Ok(ns) => ns,
        Err(code) => return code,
    };
    let parent = interp.namespaces().parent(ns);
    let name = match parent {
        Some(p) => interp.namespaces().qualified_name(p),
        None => Vec::new(), // global's parent is the empty string
    };
    interp.set_result_bytes(&name);
    Code::Ok
}

/// `namespace children ?name? ?pattern?` — child namespace FQNs (glob-filtered).
fn ns_children(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() > 4 {
        return wrong_args(interp, b"namespace children ?name? ?pattern?");
    }
    let cur = interp.current_ns();
    let ns = match resolve_arg_ns(interp, argv.get(2).copied(), cur) {
        Ok(ns) => ns,
        Err(code) => return code,
    };
    let pattern = argv.get(3).map(|&a| obj_bytes(a));
    let mut names: Vec<Vec<u8>> = interp
        .namespaces()
        .children(ns)
        .into_iter()
        .map(|c| interp.namespaces().qualified_name(c))
        .filter(|fqn| match &pattern {
            None => true,
            Some(p) => glob_match_bytes(p, fqn),
        })
        .collect();
    names.sort();
    set_list_bytes(interp, &names);
    Code::Ok
}

/// `namespace qualifiers string` — everything before the last `::` (pure text).
fn ns_qualifiers(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 3 {
        return wrong_args(interp, b"namespace qualifiers string");
    }
    let s = obj_bytes(argv[2]);
    interp.set_result_bytes(qualifiers(&s));
    Code::Ok
}

/// `namespace tail string` — the simple name after the last `::` (pure text).
fn ns_tail(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 3 {
        return wrong_args(interp, b"namespace tail string");
    }
    let s = obj_bytes(argv[2]);
    interp.set_result_bytes(tail(&s));
    Code::Ok
}

/// `namespace which ?-command? ?-variable? name` — the FQN `name` resolves to.
/// Only `-command` resolution is implemented (variables aren't ns-scoped yet);
/// `-variable` always yields the empty string.
fn ns_which(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let mut want_variable = false;
    let mut name: Option<Vec<u8>> = None;
    // The flags accept unambiguous prefix abbreviations (`-var`, `-com`), as
    // Tcl's option table does; a non-flag argument (or one after another flag)
    // is the name. Only one name is allowed.
    for &a in &argv[2..] {
        let b = obj_bytes(a);
        let is_flag = b.first() == Some(&b'-') && name.is_none();
        if is_flag && b.len() > 1 && b"-command".starts_with(b.as_slice()) {
            // -command (default behaviour)
        } else if is_flag && b.len() > 1 && b"-variable".starts_with(b.as_slice()) {
            want_variable = true;
        } else {
            if name.is_some() {
                return wrong_args(interp, b"namespace which ?-command? ?-variable? name");
            }
            name = Some(b);
        }
    }
    let Some(name) = name else {
        return wrong_args(interp, b"namespace which ?-command? ?-variable? name");
    };
    let cur = interp.current_ns();
    let fqn = if want_variable {
        interp.namespaces().which_variable(cur, &name)
    } else {
        interp.namespaces().which_command(cur, &name)
    };
    interp.set_result_bytes(&fqn.unwrap_or_default());
    Code::Ok
}

// -- export / import / forget ----------------------------------------------

/// `namespace export ?-clear? ?pattern ...?` — query / append (or clear+set)
/// the current namespace's export patterns.
fn ns_export(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let cur = interp.current_ns();
    let mut patterns: Vec<Vec<u8>> = Vec::new();
    let mut clear = false;
    for &a in &argv[2..] {
        let b = obj_bytes(a);
        if patterns.is_empty() && b == b"-clear" {
            clear = true;
        } else {
            patterns.push(b);
        }
    }
    // No patterns and no -clear ⇒ query.
    if patterns.is_empty() && !clear {
        let pats = interp.namespaces().exports(cur).to_vec();
        set_list_bytes(interp, &pats);
        return Code::Ok;
    }
    if clear {
        interp.namespaces_mut().clear_exports(cur);
    }
    for p in &patterns {
        interp.namespaces_mut().export(cur, p);
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

/// `namespace import ?-force? pattern ?pattern ...?` — install transparent
/// redirects in the current ns for the exported commands matching each pattern.
fn ns_import(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let dest = interp.current_ns();
    let mut force = false;
    let mut patterns: Vec<Vec<u8>> = Vec::new();
    for &a in &argv[2..] {
        let b = obj_bytes(a);
        if patterns.is_empty() && b == b"-force" {
            force = true;
        } else {
            patterns.push(b);
        }
    }
    if patterns.is_empty() {
        interp.set_result_bytes(b"");
        return Code::Ok;
    }
    for pat in &patterns {
        // Split into the source-namespace qualifier and the simple glob tail.
        let q = qualifiers(pat);
        let tail_pat = tail(pat);
        let Some(src_ns) = interp.namespaces().find_namespace(dest, q) else {
            let mut m = b"unknown namespace in import pattern \"".to_vec();
            m.extend_from_slice(pat);
            m.push(b'"');
            return interp.set_error(&m);
        };
        // Collect the matching, exported source commands first (borrow ends).
        let src_fqn = interp.namespaces().qualified_name(src_ns);
        let mut to_import: Vec<Vec<u8>> = Vec::new();
        for name in interp.namespaces().command_names(src_ns) {
            if glob_match_bytes(tail_pat, name) && interp.namespaces().is_exported(src_ns, name) {
                to_import.push(name.to_vec());
            }
        }
        for simple in to_import {
            let mut source = src_fqn.clone();
            source.extend_from_slice(b"::");
            source.extend_from_slice(&simple);
            // Re-importing the *same* command from the *same* source is a silent
            // no-op (C's `TclGetOriginalCommand` reimport check, tclNamesp.c) —
            // common when a file and its sourced helper both `namespace import
            // ::tcltest::*`. Only a clobber of a different command is a conflict.
            let existing_import = interp
                .namespaces()
                .imported_in(dest)
                .into_iter()
                .find(|(k, _)| k == &simple)
                .map(|(_, s)| s);
            if existing_import.as_deref() == Some(source.as_slice()) {
                continue;
            }
            // Reject clobbering an existing (different) command unless -force.
            if !force && (existing_import.is_some() || dest_has_own(interp, dest, &simple)) {
                let mut m = b"can't import command \"".to_vec();
                m.extend_from_slice(&simple);
                m.extend_from_slice(b"\": already exists");
                return interp.set_error(&m);
            }
            interp
                .namespaces_mut()
                .bind(dest, &simple, Command::Imported { source });
        }
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

/// `namespace forget ?pattern ...?` — remove imported redirects in the current
/// ns whose source FQN matches each (resolved) pattern.
fn ns_forget(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let dest = interp.current_ns();
    for &a in &argv[2..] {
        let pat = obj_bytes(a);
        // Resolve the pattern's namespace to an absolute FQN so we match against
        // the redirect's stored source FQN.
        let q = qualifiers(&pat);
        let tail_pat = tail(&pat);
        let Some(src_ns) = interp.namespaces().find_namespace(dest, q) else {
            continue; // unknown source ns ⇒ nothing to forget
        };
        let mut src_fqn = interp.namespaces().qualified_name(src_ns);
        src_fqn.extend_from_slice(b"::");
        src_fqn.extend_from_slice(tail_pat);
        let victims: Vec<Vec<u8>> = interp
            .namespaces()
            .imported_in(dest)
            .into_iter()
            .filter(|(_, source)| glob_match_bytes(&src_fqn, source))
            .map(|(simple, _)| simple)
            .collect();
        for simple in victims {
            interp.namespaces_mut().remove_in(dest, &simple);
        }
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

// -- path ------------------------------------------------------------------

/// `namespace path ?nsList?` — query (FQN list) or set the current ns's path.
fn ns_path(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() > 3 {
        return wrong_args(interp, b"namespace path ?nsList?");
    }
    let cur = interp.current_ns();
    if argv.len() == 2 {
        let path = interp.namespaces().path(cur).to_vec();
        let names: Vec<Vec<u8>> = path
            .into_iter()
            .map(|p| interp.namespaces().qualified_name(p))
            .collect();
        set_list_bytes(interp, &names);
        return Code::Ok;
    }
    // Set: parse the arg as a Tcl list of namespace names.
    let raw = obj_bytes(argv[2]);
    let elems = match crate::parse::split_list(&raw) {
        Ok(e) => e,
        Err(_) => return interp.set_error(b"unmatched open brace in list"),
    };
    let mut path: Vec<NsId> = Vec::with_capacity(elems.len());
    for e in &elems {
        let Some(ns) = interp.namespaces().find_namespace(cur, e) else {
            let mut m = b"namespace \"".to_vec();
            m.extend_from_slice(e);
            m.extend_from_slice(b"\" not found");
            return interp.set_error(&m);
        };
        path.push(ns);
    }
    interp.namespaces_mut().set_path(cur, path);
    interp.set_result_bytes(b"");
    Code::Ok
}

// -- helpers ---------------------------------------------------------------

/// Resolve an optional namespace-name argument to an `NsId`, defaulting to
/// `current`. Errors (as a builtin `Code`) if a given name doesn't exist.
fn resolve_arg_ns(
    interp: &mut Interp,
    arg: Option<*mut TclObj>,
    current: NsId,
) -> Result<NsId, Code> {
    match arg {
        None => Ok(current),
        Some(a) => {
            let name = obj_bytes(a);
            let found = interp.namespaces().find_namespace(current, &name);
            match found {
                Some(ns) => Ok(ns),
                None => {
                    let mut m = b"namespace \"".to_vec();
                    m.extend_from_slice(&name);
                    m.extend_from_slice(b"\" not found");
                    Err(interp.set_error(&m))
                }
            }
        }
    }
}

/// Does the dest namespace hold a command of this name?
fn dest_has_own(interp: &Interp, dest: NsId, simple: &[u8]) -> bool {
    interp.namespaces().command_names(dest).contains(&simple)
}

/// `namespace qualifiers` text op: everything before the last `::` (trailing
/// `::`-runs trimmed); empty if unqualified.
fn qualifiers(s: &[u8]) -> &[u8] {
    match find_last_sep(s) {
        Some(i) => &s[..i],
        None => b"",
    }
}

/// `namespace tail` text op: the simple name after the last `::`.
fn tail(s: &[u8]) -> &[u8] {
    match find_last_sep(s) {
        Some(i) => &s[i + 2..],
        None => s,
    }
}

/// Index of the last `::` separator in `s`, or `None`.
fn find_last_sep(s: &[u8]) -> Option<usize> {
    if s.len() < 2 {
        return None;
    }
    let mut i = s.len() - 1;
    while i >= 1 {
        if s[i] == b':' && s[i - 1] == b':' {
            return Some(i - 1);
        }
        i -= 1;
    }
    None
}

/// `string match` over bytes (UTF-8 → shared [`tcl_syntax::glob`]; a non-UTF-8
/// pattern or text can only match byte-identically, handled by the equality
/// fallback).
fn glob_match_bytes(pattern: &[u8], text: &[u8]) -> bool {
    match (core::str::from_utf8(pattern), core::str::from_utf8(text)) {
        (Ok(p), Ok(t)) => tcl_syntax::glob::string_match(p, t),
        _ => pattern == text,
    }
}

/// Set the interp result to a Tcl list of the given byte strings.
fn set_list_bytes(interp: &mut Interp, items: &[Vec<u8>]) {
    let elems: Vec<*mut TclObj> = items.iter().map(|n| obj::new_string_bytes(n)).collect();
    interp.set_result(list::new_list_obj(&elems));
    for e in elems {
        drop_fresh(e);
    }
}

/// Free a freshly created (`rc 0`) object once `new_list_obj` retained it.
fn drop_fresh(obj: *mut TclObj) {
    // SAFETY: `obj` is a live rc-0 object; retain-then-release frees it cleanly.
    unsafe {
        obj::incr_ref_count(obj);
        obj::decr_ref_count(obj);
    }
}

/// `namespace inscope ns cmd ?arg ...?` — evaluate `cmd` (with the extra args
/// appended) in namespace `ns`. Like `namespace eval` but used by
/// `namespace code` scripts. (The extra args are space-appended — the
/// list-element-quoting refinement matters only for the multi-arg form.)
fn ns_inscope(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 4 {
        return wrong_args(interp, b"namespace inscope name arg ?arg...?");
    }
    let name = obj_bytes(argv[2]);
    let mut script = obj_bytes(argv[3]);
    for &a in &argv[4..] {
        script.push(b' ');
        script.extend_from_slice(&obj_bytes(a));
    }
    interp.ns_eval(&name, &script)
}

/// `namespace origin command` — the fully-qualified original name of `command`
/// (following `namespace import` chains to the source).
fn ns_origin(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 3 {
        return wrong_args(interp, b"namespace origin name");
    }
    let name = obj_bytes(argv[2]);
    let cur = interp.current_ns();
    let origin = interp.namespaces().command_origin(cur, &name);
    match origin {
        Some(fqn) => {
            interp.set_result_bytes(&fqn);
            Code::Ok
        }
        None => {
            let mut m = b"invalid command name \"".to_vec();
            m.extend_from_slice(&name);
            m.push(b'"');
            interp.set_error(&m)
        }
    }
}

/// `namespace code script` — capture `script` together with the current
/// namespace so it can be evaluated later in the right context (used by
/// callbacks). Returns `::namespace inscope <currentNs> <script>`, built as a
/// proper list so `script` is correctly quoted. A script that is already such a
/// capture is returned unchanged (`NamespaceCodeCmd`, `tclNamesp.c`).
fn ns_code(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 3 {
        return wrong_args(interp, b"namespace code arg");
    }
    let script = obj_bytes(argv[2]);
    // Idempotent for an existing capture (matches C's leading-token check).
    if script.starts_with(b"::namespace inscope ") || script.starts_with(b"namespace inscope ") {
        interp.set_result(argv[2]);
        return Code::Ok;
    }
    let cur = interp.current_ns();
    let ns_name = interp.namespaces().qualified_name(cur);
    let elems = [
        crate::interp::new_string(b"::namespace"),
        crate::interp::new_string(b"inscope"),
        crate::interp::new_string(&ns_name),
        crate::interp::new_string(&script),
    ];
    interp.set_result(crate::list::new_list_obj(&elems));
    Code::Ok
}

/// `namespace upvar ns ?otherVar myVar ...?` — link each `myVar` in the current
/// frame to `otherVar`, a variable resolved in namespace `ns` (mirrors C's
/// `NamespaceUpvarCmd`: the other-var is looked up with the var frame's
/// namespace temporarily set to `ns`).
fn ns_upvar(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    // objc<2 || objc&1 in C (argv[0] is "namespace"): need ns + even #pairs.
    if argv.len() < 3 || argv.len() % 2 == 0 {
        return wrong_args(interp, b"namespace upvar ns ?otherVar myVar ...?");
    }
    let ns_name = obj_bytes(argv[2]);
    let Some(ns) = interp
        .namespaces()
        .find_namespace(interp.current_ns(), &ns_name)
    else {
        let mut m = b"namespace \"".to_vec();
        m.extend_from_slice(&ns_name);
        m.extend_from_slice(b"\" not found");
        return interp.set_error(&m);
    };

    let mut i = 3;
    while i + 1 < argv.len() {
        let other = obj_bytes(argv[i]);
        let local = obj_bytes(argv[i + 1]);
        let (base, elem) = crate::frame::split_array_ref(&other);
        // The other-var resolves in `ns` (a qualified `base` resolves relative to
        // it, an unqualified one names a var of `ns` directly).
        let Some((home_ns, simple)) = interp.resolve_var_target(ns, &base) else {
            let mut m = b"can't access \"".to_vec();
            m.extend_from_slice(&other);
            m.extend_from_slice(b"\": parent namespace doesn't exist");
            return interp.set_error(&m);
        };
        let link = crate::frame::Link {
            home: crate::frame::VarHome::Namespace(home_ns),
            name: simple,
            elem,
        };
        interp.make_upvar(link, &local);
        i += 2;
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

// -- ensemble --------------------------------------------------------------

/// `namespace ensemble create|exists ...` — the canonical `ens sub`→target
/// redirect (the generalised `dict for`→`::tcl::dict::for` mechanism).
fn ns_ensemble(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 3 {
        return wrong_args(interp, b"namespace ensemble subcommand ?arg ...?");
    }
    match obj_bytes(argv[2]).as_slice() {
        b"create" => ens_create(interp, argv),
        b"exists" => ens_exists(interp, argv),
        other => {
            // `configure` is a follow-up; only create/exists are modelled.
            let mut m = b"unknown or ambiguous subcommand \"".to_vec();
            m.extend_from_slice(other);
            m.extend_from_slice(b"\": must be create, or exists");
            interp.set_error(&m)
        }
    }
}

/// `namespace ensemble create ?-command name? ?-map dict? ?-subcommands list?
/// ?-prefixes bool?` — register an ensemble over the current namespace.
fn ens_create(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let ns = interp.current_ns();
    // Default ensemble command is the namespace's own FQN.
    let mut command = interp.namespaces().qualified_name(ns);
    let mut map: Option<EnsembleMap> = None;
    let mut subcommands: Option<Vec<Vec<u8>>> = None;
    let mut prefixes = true;

    let opts = &argv[3..];
    if opts.len() % 2 != 0 {
        return interp.set_error(b"missing value for option");
    }
    for pair in opts.chunks_exact(2) {
        let (opt, val) = (obj_bytes(pair[0]), pair[1]);
        match opt.as_slice() {
            b"-command" => command = obj_bytes(val),
            b"-subcommands" => match crate::parse::split_list(&obj_bytes(val)) {
                Ok(list) => subcommands = Some(list),
                Err(e) => return interp.set_error(e.message()),
            },
            b"-map" => match parse_map(&obj_bytes(val)) {
                Ok(m) => map = Some(m),
                Err(e) => return interp.set_error(&e),
            },
            b"-prefixes" => match parse_bool(&obj_bytes(val)) {
                Some(b) => prefixes = b,
                None => {
                    let mut m = b"expected boolean value but got \"".to_vec();
                    m.extend_from_slice(&obj_bytes(val));
                    m.push(b'"');
                    return interp.set_error(&m);
                }
            },
            _ => {
                let mut m = b"bad option \"".to_vec();
                m.extend_from_slice(&opt);
                m.extend_from_slice(b"\": should be -command, -map, -prefixes, or -subcommands");
                return interp.set_error(&m);
            }
        }
    }

    interp.create_ensemble(
        &command,
        EnsembleConfig {
            ns,
            map,
            subcommands,
            prefixes,
        },
    );
    interp.set_result_bytes(&command);
    Code::Ok
}

/// `namespace ensemble exists command` — 1 if it resolves to an ensemble.
fn ens_exists(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 4 {
        return wrong_args(interp, b"namespace ensemble exists cmd");
    }
    let exists = interp.is_ensemble(&obj_bytes(argv[3]));
    interp.set_result_bytes(if exists { b"1" } else { b"0" });
    Code::Ok
}

/// Parse a `-map` dict (`sub {target prefix} …`) into (subcommand, prefix-words).
fn parse_map(bytes: &[u8]) -> Result<EnsembleMap, Vec<u8>> {
    let kvs = crate::parse::split_list(bytes).map_err(|e| e.message().to_vec())?;
    if kvs.len() % 2 != 0 {
        return Err(b"missing value to go with key".to_vec());
    }
    let mut map = Vec::with_capacity(kvs.len() / 2);
    for pair in kvs.chunks_exact(2) {
        let prefix = crate::parse::split_list(&pair[1]).map_err(|e| e.message().to_vec())?;
        map.push((pair[0].clone(), prefix));
    }
    Ok(map)
}

/// Tcl boolean literal (`Tcl_GetBoolean`) for `-prefixes`.
fn parse_bool(bytes: &[u8]) -> Option<bool> {
    match bytes.to_ascii_lowercase().as_slice() {
        b"1" | b"true" | b"yes" | b"on" => Some(true),
        b"0" | b"false" | b"no" | b"off" => Some(false),
        _ => None,
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
    fn current_is_global_at_top_level() {
        leak_free(|i| {
            assert_eq!(i.eval_str(b"namespace current"), Code::Ok);
            assert_eq!(i.result_bytes(), b"::");
        });
    }

    #[test]
    fn eval_switches_current_and_defines_in_ns() {
        leak_free(|i| {
            // A command defined inside `namespace eval` lands in that ns.
            assert_eq!(
                i.eval_str(b"namespace eval foo { set ::probe [namespace current] }"),
                Code::Ok
            );
            assert_eq!(i.eval_str(b"set ::probe"), Code::Ok);
            assert_eq!(i.result_bytes(), b"::foo");
            // Back at top level current is global again.
            assert_eq!(i.eval_str(b"namespace current"), Code::Ok);
            assert_eq!(i.result_bytes(), b"::");
        });
    }

    #[test]
    fn exists_and_parent_and_children() {
        leak_free(|i| {
            i.eval_str(b"namespace eval a { namespace eval b {} }");
            assert_eq!(i.eval_str(b"namespace exists ::a::b"), Code::Ok);
            assert_eq!(i.result_bytes(), b"1");
            assert_eq!(i.eval_str(b"namespace exists ::a::nope"), Code::Ok);
            assert_eq!(i.result_bytes(), b"0");
            assert_eq!(i.eval_str(b"namespace parent ::a::b"), Code::Ok);
            assert_eq!(i.result_bytes(), b"::a");
            assert_eq!(i.eval_str(b"namespace children ::a"), Code::Ok);
            assert_eq!(i.result_bytes(), b"::a::b");
        });
    }

    #[test]
    fn qualifiers_and_tail() {
        leak_free(|i| {
            assert_eq!(
                i.eval_str(b"namespace qualifiers ::foo::bar::baz"),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"::foo::bar");
            assert_eq!(i.eval_str(b"namespace tail ::foo::bar::baz"), Code::Ok);
            assert_eq!(i.result_bytes(), b"baz");
            assert_eq!(i.eval_str(b"namespace qualifiers plain"), Code::Ok);
            assert_eq!(i.result_bytes(), b"");
            assert_eq!(i.eval_str(b"namespace tail plain"), Code::Ok);
            assert_eq!(i.result_bytes(), b"plain");
        });
    }

    #[test]
    fn subcommand_prefix_abbreviation() {
        leak_free(|i| {
            // `namespace` subcommands accept unambiguous prefixes.
            assert_eq!(i.eval_str(b"namespace exist ::nope"), Code::Ok);
            assert_eq!(i.result_bytes(), b"0");
            assert_eq!(i.eval_str(b"namespace cur"), Code::Ok);
            assert_eq!(i.result_bytes(), b"::");
            // An ambiguous prefix still errors.
            assert_eq!(i.eval_str(b"namespace ex foo"), Code::Error);
        });
    }

    #[test]
    fn which_resolves_command_fqn() {
        leak_free(|i| {
            assert_eq!(i.eval_str(b"namespace which -command set"), Code::Ok);
            assert_eq!(i.result_bytes(), b"::set");
            assert_eq!(i.eval_str(b"namespace which nope"), Code::Ok);
            assert_eq!(i.result_bytes(), b"");
            // Prefix abbreviation of the flags (`-com`, `-var`).
            assert_eq!(i.eval_str(b"namespace which -com set"), Code::Ok);
            assert_eq!(i.result_bytes(), b"::set");
        });
    }

    #[test]
    fn which_variable_resolves_namespace_var_fqn() {
        leak_free(|i| {
            // `namespace which -variable` resolves a namespace variable to its
            // FQN (ignoring local proc links); a missing one yields "".
            assert_eq!(
                i.eval_str(b"namespace eval ::n { variable gv 1 }"),
                Code::Ok
            );
            assert_eq!(i.eval_str(b"namespace which -variable ::n::gv"), Code::Ok);
            assert_eq!(i.result_bytes(), b"::n::gv");
            assert_eq!(i.eval_str(b"namespace which -var ::n::gv"), Code::Ok);
            assert_eq!(i.result_bytes(), b"::n::gv");
            assert_eq!(i.eval_str(b"namespace which -variable ::n::nope"), Code::Ok);
            assert_eq!(i.result_bytes(), b"");
        });
    }

    #[test]
    fn path_fallback_resolves_unqualified() {
        leak_free(|i| {
            // A command in ::lib (the alias creates ::lib), then ::lib on ::app's path.
            i.eval_str(b"interp alias {} ::lib::ping {} set pinged");
            assert_eq!(
                i.eval_str(b"namespace eval app { namespace path ::lib }"),
                Code::Ok
            );
            // Now from ::app, bare `ping` resolves through the path. Its body
            // `set pinged` runs with the current namespace = ::app, so the
            // variable lands in ::app's table (NOT global — the T1.5 var-namespace
            // fix; before it, every unqualified `set` leaked to the global frame).
            assert_eq!(i.eval_str(b"namespace eval app { ping yes }"), Code::Ok);
            assert_eq!(i.result_bytes(), b"yes");
            assert_eq!(i.eval_str(b"set ::app::pinged"), Code::Ok);
            assert_eq!(i.result_bytes(), b"yes");
            // …and it is NOT visible as a bare global.
            assert_eq!(i.eval_str(b"set pinged"), Code::Error);
        });
    }

    #[test]
    fn export_import_forget_roundtrip() {
        leak_free(|i| {
            // ::lib exports g* ; provide a command ::lib::greet via an alias.
            i.eval_str(b"interp alias {} ::lib::greet {} set greeted");
            i.eval_str(b"namespace eval lib { namespace export g* }");
            // Import into ::app, then call the bare name there.
            assert_eq!(
                i.eval_str(b"namespace eval app { namespace import ::lib::* }"),
                Code::Ok
            );
            assert_eq!(i.eval_str(b"namespace eval app { greet hi }"), Code::Ok);
            // `set greeted` ran in ::app (the call's current namespace).
            assert_eq!(i.eval_str(b"set ::app::greeted"), Code::Ok);
            assert_eq!(i.result_bytes(), b"hi");
            // Forget removes the redirect.
            assert_eq!(
                i.eval_str(b"namespace eval app { namespace forget ::lib::* }"),
                Code::Ok
            );
            assert_eq!(i.eval_str(b"namespace eval app { greet hi }"), Code::Error);
        });
    }

    #[test]
    fn reimport_same_source_is_idempotent() {
        // Re-importing the *same* command from the *same* source is a silent
        // no-op (C's reimport check) — the common case where a file and its
        // sourced helper both `namespace import ::lib::*` (e.g. tcltest). Only a
        // clobber of a *different* command is a conflict (without -force).
        leak_free(|i| {
            i.eval_str(b"namespace eval lib { proc g {} {return G} ; namespace export g }");
            assert_eq!(i.eval_str(b"namespace import ::lib::*"), Code::Ok);
            // Second import of the same command from the same source: no error.
            assert_eq!(i.eval_str(b"namespace import ::lib::*"), Code::Ok);
            assert_eq!(i.eval_str(b"namespace import ::lib::g"), Code::Ok);
            // A different command of the same simple name does conflict.
            i.eval_str(b"namespace eval other { proc g {} {return O} ; namespace export g }");
            assert_eq!(i.eval_str(b"namespace import ::other::*"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"can't import command \"g\": already exists"
            );
            // -force overrides the clobber.
            assert_eq!(i.eval_str(b"namespace import -force ::other::*"), Code::Ok);
        });
    }

    #[test]
    fn qualified_command_falls_back_to_global() {
        // A relative qualified command name resolves against the current
        // namespace, then the global one (C's `TclGetNamespaceForQualName`): so
        // `foo::bar` from inside `::a::b` finds `::foo::bar` when `::a::b::foo`
        // doesn't exist. (The bug: `tcl::build-info` failing inside a namespace.)
        leak_free(|i| {
            i.eval_str(b"namespace eval foo { proc bar {} {return BAR} }");
            assert_eq!(
                i.eval_str(b"namespace eval ::a::b { set ::r [foo::bar] }"),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"BAR");
            // A relative qualifier that *does* exist locally still wins.
            i.eval_str(b"namespace eval x::foo { proc bar {} {return LOCAL} }");
            assert_eq!(i.eval_str(b"namespace eval x { foo::bar }"), Code::Ok);
            assert_eq!(i.result_bytes(), b"LOCAL");
            i.eval_str(b"unset -nocomplain ::r");
        });
    }

    #[test]
    fn namespace_delete_removes_subtree() {
        leak_free(|i| {
            i.eval_str(b"namespace eval foo { proc p {} {return P} ; namespace eval bar {} }");
            assert_eq!(i.eval_str(b"namespace exists ::foo::bar"), Code::Ok);
            assert_eq!(i.result_bytes(), b"1");
            assert_eq!(i.eval_str(b"namespace delete ::foo"), Code::Ok);
            // The namespace, its child, and its commands are gone.
            assert_eq!(i.eval_str(b"namespace exists ::foo"), Code::Ok);
            assert_eq!(i.result_bytes(), b"0");
            assert_eq!(i.eval_str(b"namespace exists ::foo::bar"), Code::Ok);
            assert_eq!(i.result_bytes(), b"0");
            assert_eq!(i.eval_str(b"::foo::p"), Code::Error);
            // Deleting a missing namespace errors (tclsh message).
            assert_eq!(i.eval_str(b"namespace delete ::nope"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"unknown namespace \"::nope\" in namespace delete command"
            );
        });
    }

    #[test]
    fn build_info_queries() {
        // `tcl::build-info` (the tcltest constraint source): version/patchlevel
        // parse the build string; feature flags we don't set report 0.
        leak_free(|i| {
            assert_eq!(i.eval_str(b"tcl::build-info version"), Code::Ok);
            assert_eq!(i.result_bytes(), b"9.0");
            assert_eq!(i.eval_str(b"tcl::build-info patchlevel"), Code::Ok);
            assert_eq!(i.result_bytes(), b"9.0.3");
            for feat in [&b"debug"[..], b"purify", b"memdebug", b"no-deprecate"] {
                let mut cmd = b"tcl::build-info ".to_vec();
                cmd.extend_from_slice(feat);
                assert_eq!(i.eval_str(&cmd), Code::Ok);
                assert_eq!(i.result_bytes(), b"0", "feature {feat:?} should be absent");
            }
        });
    }

    #[test]
    fn import_only_takes_exported_commands() {
        leak_free(|i| {
            i.eval_str(b"interp alias {} ::lib::secret {} set s");
            i.eval_str(b"interp alias {} ::lib::pub {} set p");
            i.eval_str(b"namespace eval lib { namespace export pub }");
            i.eval_str(b"namespace eval app { namespace import ::lib::* }");
            // `pub` imported, `secret` not.
            assert_eq!(i.eval_str(b"namespace eval app { pub 1 }"), Code::Ok);
            assert_eq!(i.eval_str(b"namespace eval app { secret 1 }"), Code::Error);
        });
    }

    #[test]
    fn export_query_returns_patterns() {
        leak_free(|i| {
            i.eval_str(b"namespace eval lib { namespace export a* b* }");
            assert_eq!(
                i.eval_str(b"namespace eval lib { namespace export }"),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"a* b*");
        });
    }

    #[test]
    fn path_query_returns_list() {
        leak_free(|i| {
            i.eval_str(b"namespace eval lib {}");
            i.eval_str(b"namespace eval app { namespace path ::lib }");
            assert_eq!(
                i.eval_str(b"namespace eval app { namespace path }"),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"::lib");
        });
    }

    // -- ensembles (targets are aliases, since procs aren't available yet) -----

    #[test]
    fn ensemble_default_dispatches_to_namespace_commands() {
        leak_free(|i| {
            // ::foo::{bar,baz} as alias→set; export + ensemble over ::foo.
            i.eval_str(b"interp alias {} ::foo::bar {} set");
            i.eval_str(b"interp alias {} ::foo::baz {} set");
            assert_eq!(
                i.eval_str(
                    b"namespace eval foo { namespace export bar baz; namespace ensemble create }"
                ),
                Code::Ok
            );
            // `foo bar v 42` → ::foo::bar v 42 → set v 42.
            assert_eq!(i.eval_str(b"foo bar v 42"), Code::Ok);
            assert_eq!(i.result_bytes(), b"42");
            assert_eq!(i.eval_str(b"set v"), Code::Ok);
            assert_eq!(i.result_bytes(), b"42");
            i.eval_str(b"unset v");
        });
    }

    #[test]
    fn ensemble_prefix_match_and_ambiguity() {
        leak_free(|i| {
            i.eval_str(b"interp alias {} ::foo::bar {} set");
            i.eval_str(b"interp alias {} ::foo::baz {} set");
            i.eval_str(
                b"namespace eval foo { namespace export bar baz; namespace ensemble create }",
            );
            // `ba`/`b` are ambiguous between bar and baz.
            assert_eq!(i.eval_str(b"foo ba v 1"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"unknown or ambiguous subcommand \"ba\": must be bar, or baz"
            );
            assert_eq!(i.eval_str(b"foo nope v 1"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"unknown or ambiguous subcommand \"nope\": must be bar, or baz"
            );
            i.eval_str(b"unset v"); // bar/baz never ran; v is unset → ignore result
        });
    }

    #[test]
    fn ensemble_map_and_subcommands() {
        leak_free(|i| {
            // -map a subcommand to a concrete (builtin) target.
            assert_eq!(
                i.eval_str(b"namespace eval m { namespace ensemble create -map {go ::set} }"),
                Code::Ok
            );
            assert_eq!(i.eval_str(b"m go v 7"), Code::Ok); // → ::set v 7
            assert_eq!(i.result_bytes(), b"7");
            // a non-mapped subcommand is unknown (the map keys are the set).
            assert_eq!(i.eval_str(b"m set v 7"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"unknown or ambiguous subcommand \"set\": must be go"
            );
            i.eval_str(b"unset v");
        });
    }

    #[test]
    fn ensemble_command_option_and_prefixes_off() {
        leak_free(|i| {
            // -command names the ensemble cmd; -prefixes 0 disables prefix match.
            assert_eq!(
                i.eval_str(
                    b"namespace eval q { namespace ensemble create -command ::top -subcommands longname -map {longname ::set} -prefixes 0 }"
                ),
                Code::Ok
            );
            assert_eq!(i.eval_str(b"top longname v 9"), Code::Ok);
            assert_eq!(i.result_bytes(), b"9");
            assert_eq!(i.eval_str(b"top long v 9"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"unknown subcommand \"long\": must be longname"
            );
            i.eval_str(b"unset v");
        });
    }

    #[test]
    fn ensemble_exists() {
        leak_free(|i| {
            i.eval_str(b"namespace eval foo { namespace ensemble create }");
            assert_eq!(i.eval_str(b"namespace ensemble exists foo"), Code::Ok);
            assert_eq!(i.result_bytes(), b"1");
            assert_eq!(i.eval_str(b"namespace ensemble exists ::nope"), Code::Ok);
            assert_eq!(i.result_bytes(), b"0");
        });
    }

    #[test]
    fn namespace_upvar_links_local_to_ns_var() {
        leak_free(|i| {
            i.eval_str(b"namespace eval a { variable x 42 }");
            // `namespace upvar a x lx` links a frame-local `lx` to `::a::x`.
            assert_eq!(
                i.eval_str(
                    b"proc p {} { namespace upvar a x lx; set lx [expr {$lx+1}]; return $lx }"
                ),
                Code::Ok
            );
            assert_eq!(i.eval_str(b"p"), Code::Ok);
            assert_eq!(i.result_bytes(), b"43");
            // The write went through to the namespace variable.
            assert_eq!(i.eval_str(b"set ::a::x"), Code::Ok);
            assert_eq!(i.result_bytes(), b"43");
            // A missing namespace is an error.
            assert_eq!(i.eval_str(b"namespace upvar nope v lv"), Code::Error);
        });
    }

    #[test]
    fn ensemble_command_is_qualified_relative_to_current_ns() {
        leak_free(|i| {
            // A relative `-command` binds in the current namespace, not global
            // (the `tcl::tm::path` / safe-base case): `-command path` inside
            // `::a::b` creates `::a::b::path`, resolvable by its FQN.
            assert_eq!(
                i.eval_str(
                    b"namespace eval a::b { namespace export path; namespace ensemble create -command path -map {list ::set} }"
                ),
                Code::Ok
            );
            assert_eq!(
                i.eval_str(b"namespace which -command ::a::b::path"),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"::a::b::path");
            // No bare `::path` leaked into the global namespace.
            assert_eq!(i.eval_str(b"namespace which -command ::path"), Code::Ok);
            assert_eq!(i.result_bytes(), b"");
            assert_eq!(i.eval_str(b"::a::b::path list v 5"), Code::Ok);
            assert_eq!(i.result_bytes(), b"5");
            i.eval_str(b"unset v");
        });
    }
}
