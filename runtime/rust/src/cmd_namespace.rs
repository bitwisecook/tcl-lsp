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
//! See `docs/design/runtime/namespace-tree.md` for the model.
//!
//! See `list.rs` for the module-level `not_unsafe_ptr_arg_deref` rationale.
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use crate::ensemble::{EnsembleConfig, EnsembleMap};
use crate::interp::{error_code_list, obj_bytes, Code, Command, Interp};
use crate::list;
use crate::namespace::NsId;
use crate::obj::{self, TclObj};

/// Register the `namespace` command.
pub fn install(interp: &mut Interp) {
    interp.register_builtin(b"namespace", namespace_cmd);
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
    b"unknown",
    b"upvar",
    b"which",
];

fn namespace_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 {
        return interp.wrong_args(b"namespace subcommand ?arg ...?");
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
                m.extend_from_slice(b"\": must be children, code, current, delete, ensemble, eval, exists, export, forget, import, inscope, origin, parent, path, qualifiers, tail, unknown, upvar, or which");
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
        b"unknown" => ns_unknown(interp, argv),
        b"upvar" => ns_upvar(interp, argv),
        _ => unreachable!("subcommand resolved to a canonical name above"),
    }
}

// -- current / eval / exists / parent / children ---------------------------

/// `namespace unknown ?handler?` — get or set the current namespace's
/// unknown-command handler. The global namespace's default is `::unknown`; a
/// sub-namespace with no handler reports the empty string (and falls back to
/// `::unknown` at dispatch). Mirrors `NamespaceUnknownCmd` (`tclNamesp.c`).
fn ns_unknown(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() > 3 {
        return interp.wrong_args(b"namespace unknown ?script?");
    }
    let cur = interp.current_ns();
    if argv.len() == 3 {
        let handler = obj_bytes(argv[2]);
        // The handler must be a well-formed list (command prefix); a parse error
        // is reported *without* changing the current handler (namespace-52.12).
        if let Err(e) = crate::parse::split_list(&handler) {
            return interp.set_error(e.message());
        }
        interp.namespaces_mut().set_unknown_handler(cur, &handler);
        interp.set_result_bytes(b"");
        return Code::Ok;
    }
    // Get: the stored handler, or the interpreter default for the global ns.
    let h = interp.namespaces().unknown_handler(cur).map(<[u8]>::to_vec);
    match h {
        Some(h) => interp.set_result_bytes(&h),
        None if cur == crate::namespace::GLOBAL => interp.set_result_bytes(b"::unknown"),
        None => interp.set_result_bytes(b""),
    }
    Code::Ok
}

/// `namespace current` — the FQN of the current namespace.
fn ns_current(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 2 {
        return interp.wrong_args(b"namespace current");
    }
    // The shared Family-B core over `Namespaces::{current, name}`.
    let v = tcl_cmd_core::namespace::current(interp);
    interp.set_result(v);
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
        // Delete by id so variable unset traces in the namespace fire as it is
        // torn down (the named `delete_namespace` path does not).
        interp.delete_namespace_by_id(ns_id);
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

/// `namespace eval name arg ?arg ...?` — evaluate a body in `name` (multiple
/// `arg`s are concatenated with spaces, like `eval`).
fn ns_eval(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 4 {
        return interp.wrong_args(b"namespace eval name arg ?arg...?");
    }
    let name = obj_bytes(argv[2]);
    // A single body argument keeps its `Tcl_Obj` so a located literal runs as
    // `type source` (TIP 280); multiple args concatenate into a dynamic body.
    if argv.len() == 4 {
        return interp.ns_eval_obj(&name, argv[3]);
    }
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
        return interp.wrong_args(b"namespace exists name");
    }
    let name = obj_bytes(argv[2]);
    let v = tcl_cmd_core::namespace::exists_bytes(interp, &name);
    interp.set_result(v);
    Code::Ok
}

/// `namespace parent ?name?` — the FQN of the (named, or current) ns's parent,
/// via the shared `tcl_cmd_core::namespace` core over `Namespaces`.
fn ns_parent(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() > 3 {
        return interp.wrong_args(b"namespace parent ?name?");
    }
    let name = argv.get(2).map(|&arg| obj_bytes(arg));
    match tcl_cmd_core::namespace::parent_bytes(interp, name.as_deref()) {
        Ok(v) => {
            interp.set_result(v);
            Code::Ok
        }
        Err(error) => ns_not_found(interp, error.name()),
    }
}

/// `namespace children ?name? ?pattern?` — child namespace FQNs (glob-filtered),
/// via the shared core.
fn ns_children(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() > 4 {
        return interp.wrong_args(b"namespace children ?name? ?pattern?");
    }
    let name = argv.get(2).map(|&arg| obj_bytes(arg));
    let pattern = argv.get(3).map(|&arg| obj_bytes(arg));
    match tcl_cmd_core::namespace::children_bytes(interp, name.as_deref(), pattern.as_deref()) {
        Ok(v) => {
            interp.set_result(v);
            Code::Ok
        }
        Err(error) => ns_not_found(interp, error.name()),
    }
}

/// `namespace qualifiers string` — everything before the last `::` (pure text).
fn ns_qualifiers(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 3 {
        return interp.wrong_args(b"namespace qualifiers string");
    }
    let s = obj_bytes(argv[2]);
    interp.set_result_bytes(tcl_cmd_core::namespace::qualifiers(&s));
    Code::Ok
}

/// `namespace tail string` — the simple name after the last `::` (pure text).
fn ns_tail(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 3 {
        return interp.wrong_args(b"namespace tail string");
    }
    let s = obj_bytes(argv[2]);
    interp.set_result_bytes(tcl_cmd_core::namespace::tail(&s));
    Code::Ok
}

/// `namespace which ?-command? ?-variable? name` — the FQN `name` resolves to.
/// Only `-command` resolution is implemented (variables aren't ns-scoped yet);
/// `-variable` always yields the empty string.
fn ns_which(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let args: Vec<Vec<u8>> = argv[2..].iter().map(|&arg| obj_bytes(arg)).collect();
    let Some((kind, name_index)) = tcl_cmd_core::namespace::which_request(&args) else {
        return interp.wrong_args(b"namespace which ?-command? ?-variable? name");
    };
    let name = &args[name_index];
    if kind == tcl_cmd_core::namespace::WhichKind::Variable {
        // `-variable` through the shared `Tcl_FindNamespaceVar` core — the
        // 8.x global-fallback candidate is a release axis, so the profile
        // goes with it.
        let profile = interp.dialect_profile();
        let fqn = tcl_cmd_core::namespace::variable_fqn_bytes(interp, name, profile);
        interp.set_result_bytes(&fqn.unwrap_or_default());
    } else {
        // `-command` via the shared `Namespaces` resolution core.
        let fqn = tcl_cmd_core::namespace::which_command_bytes(interp, name);
        interp.set_result_bytes(&fqn.unwrap_or_default());
    }
    Code::Ok
}

// -- export / import / forget ----------------------------------------------

/// `namespace export ?-clear? ?pattern ...?` — query / append (or clear+set)
/// the current namespace's export patterns.
fn ns_export(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let cur = interp.current_ns();
    // No words at all ⇒ query (C's `objc == 1` arm, before `-clear` is even
    // looked at).
    if argv.len() == 2 {
        let pats = interp.namespaces().exports(cur).to_vec();
        set_list_bytes(interp, &pats);
        return Code::Ok;
    }
    // `-clear` is *positional*: C tests only `objv[1]`, so a second `-clear`
    // is an ordinary pattern (the registry says the same with
    // `max_leading_option_words: Some(1)`).
    let mut words = argv[2..].iter().map(|&a| obj_bytes(a));
    let mut first = words.next();
    let clear = first.as_deref() == Some(b"-clear");
    if clear {
        first = words.next();
    }
    let patterns: Vec<Vec<u8>> = first.into_iter().chain(words).collect();
    // `-clear` commits before any pattern is even looked at: C spends a whole
    // `Tcl_Export(…, "::", 1)` call on it, which resets the list and then
    // fails its own qualifier check — an error `NamespaceExportCmd` discards
    // with `Tcl_ResetResult`. So a later invalid pattern cannot undo it.
    if clear {
        interp.namespaces_mut().clear_exports(cur);
    }
    // An export pattern names commands in the *current* namespace only, so it
    // may not be namespace-qualified (C's `NamespaceExportCmd`). C calls
    // `Tcl_Export` once per pattern and returns on the first failure, leaving
    // the earlier patterns committed — this is a per-pattern loop, not a
    // batch gate.
    for p in &patterns {
        if tcl_syntax::naming::is_qualified(p) {
            let mut m = b"invalid export pattern \"".to_vec();
            m.extend_from_slice(p);
            m.extend_from_slice(b"\": pattern can't specify a namespace");
            return interp.set_error(&m);
        }
        interp.namespaces_mut().export(cur, p);
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

/// `namespace import ?-force? pattern ?pattern ...?` — install transparent
/// redirects in the current ns for the exported commands matching each pattern.
fn ns_import(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let dest = interp.current_ns();
    // The introspection form is `objc == 1` — literally no words after
    // `import` — and C tests it *before* looking at `-force`. So a bare
    // `namespace import -force` is NOT a query: it takes the flag, imports
    // nothing, and yields "".
    if argv.len() == 2 {
        let names = interp.imported_command_tails(dest);
        set_list_bytes(interp, &names);
        return Code::Ok;
    }
    // `-force` is positional in the same way `-clear` is: C reads it from
    // `objv[1]` only, so a trailing `-force` becomes a pattern — and then
    // fails the "the pattern must name a source namespace" check below.
    let mut words = argv[2..].iter().map(|&a| obj_bytes(a));
    let mut first = words.next();
    let force = first.as_deref() == Some(b"-force");
    if force {
        first = words.next();
    }
    let patterns: Vec<Vec<u8>> = first.into_iter().chain(words).collect();
    if patterns.is_empty() {
        interp.set_result_bytes(b"");
        return Code::Ok;
    }
    for pat in &patterns {
        let destination = tcl_runtime_api::Namespaces::current(interp);
        let validated = match tcl_cmd_core::namespace::import_pattern(interp, destination, pat) {
            Ok(validated) => validated,
            Err(problem @ tcl_cmd_core::namespace::ImportPatternError::Unknown(_)) => {
                // A namespace being deleted is absent from the public tree but
                // its command table remains token-addressable during delete
                // callbacks. Tcl_Import can therefore import a callback-created
                // command from that dying table even though `namespace exists`
                // is false. Keep the shared qualifier/tail parser and only
                // substitute this lifecycle-specific namespace lookup.
                let qualifier = match tcl_cmd_core::namespace::qualifier(pat) {
                    tcl_cmd_core::namespace::Qualifier::Absolute(prefix)
                    | tcl_cmd_core::namespace::Qualifier::Relative(prefix) => prefix,
                    tcl_cmd_core::namespace::Qualifier::Unqualified => {
                        return interp.set_error(&problem.message());
                    }
                };
                let Some(source) = interp.namespaces().dying_namespace(dest, qualifier) else {
                    return interp.set_error(&problem.message());
                };
                tcl_cmd_core::namespace::ImportPattern {
                    source: tcl_runtime_api::NsId(source as u32),
                    tail: tcl_cmd_core::namespace::tail(pat).to_vec(),
                }
            }
            Err(problem) => return interp.set_error(&problem.message()),
        };
        let src_ns = validated.source.0 as usize;
        let tail_pat = validated.tail;
        // Collect the matching, exported source commands first (borrow ends).
        let src_fqn = interp.namespaces().qualified_name(src_ns);
        let mut to_import: Vec<Vec<u8>> = Vec::new();
        for name in interp.visible_command_names_in(src_ns) {
            if glob_match_bytes(&tail_pat, &name) && interp.namespaces().is_exported(src_ns, &name)
            {
                to_import.push(name);
            }
        }
        for simple in to_import {
            let mut source = src_fqn.clone();
            if source != b"::" {
                source.extend_from_slice(b"::");
            }
            source.extend_from_slice(&simple);
            let Some((source, ensemble)) = interp.import_metadata_at(&source) else {
                continue;
            };
            // Without `-force`, re-importing the *same* command from the *same*
            // source is a silent no-op (C's `TclGetOriginalCommand` reimport
            // check, tclNamesp.c) — common when a file and its sourced helper
            // both import `::tcltest::*`. `-force` deliberately replaces even
            // that same-origin import with a fresh command token.
            let existing_import = interp
                .namespaces()
                .imported_in(dest)
                .into_iter()
                .find(|(k, _)| k == &simple)
                .map(|(_, s)| s);
            if !force && existing_import.as_deref() == Some(source.as_slice()) {
                continue;
            }
            // Reject clobbering an existing (different) command unless -force.
            if !force && (existing_import.is_some() || dest_has_own(interp, dest, &simple)) {
                let mut m = b"can't import command \"".to_vec();
                m.extend_from_slice(&simple);
                m.extend_from_slice(b"\": already exists");
                return interp.error_with_code(&m, b"TCL IMPORT OVERWRITE");
            }
            let mut destination_fqn = interp.namespaces().qualified_name(dest);
            if destination_fqn != b"::" {
                destination_fqn.extend_from_slice(b"::");
            }
            destination_fqn.extend_from_slice(&simple);
            // Follow immediate import origins before mutating the destination.
            // If the source chain already reaches the command being replaced,
            // this new edge would close Tcl's ImportRef graph into a cycle.
            if interp
                .namespaces()
                .import_chain_contains(&source, &destination_fqn)
            {
                let mut message = b"import pattern \"".to_vec();
                message.extend_from_slice(pat);
                message.extend_from_slice(b"\" would create a loop containing command \"");
                message.extend_from_slice(&destination_fqn);
                message.push(b'"');
                let error_code = error_code_list(&[b"TCL", b"IMPORT", b"LOOP"]);
                return interp.error_with_code(&message, &error_code);
            }
            interp.bind_command_replacement(
                dest,
                &simple,
                Command::Imported {
                    source,
                    ensemble,
                    identity: std::rc::Rc::new(crate::interp::ImportToken),
                },
            );
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
        let q = match tcl_cmd_core::namespace::qualifier(&pat) {
            tcl_cmd_core::namespace::Qualifier::Absolute(q)
            | tcl_cmd_core::namespace::Qualifier::Relative(q) => q,
            tcl_cmd_core::namespace::Qualifier::Unqualified => b"",
        };
        let tail_pat = tcl_cmd_core::namespace::tail(&pat);
        let Some(src_ns) = interp.namespaces().find_namespace(dest, q) else {
            // C's `Tcl_ForgetImport` errors if the pattern's namespace qualifier
            // names a namespace that does not exist (an unqualified pattern
            // resolves to the current namespace, which always exists).
            let mut m = b"unknown namespace in namespace forget pattern \"".to_vec();
            m.extend_from_slice(&pat);
            m.push(b'"');
            return interp.set_error(&m);
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
        return interp.wrong_args(b"namespace path ?nsList?");
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
        let found = interp.namespaces().find_namespace(cur, e);
        let Some(ns) = found else {
            return ns_not_found(interp, e);
        };
        path.push(ns);
    }
    interp.namespaces_mut().set_path(cur, path);
    interp.set_result_bytes(b"");
    Code::Ok
}

// -- helpers ---------------------------------------------------------------

/// The `TclGetNamespaceFromObj` not-found error: a *relative* name names the
/// current namespace context (`… not found in "::ns"`), an absolute one does not
/// (`… not found`). Sets `-errorcode TCL LOOKUP NAMESPACE <name>`.
fn ns_not_found(interp: &mut Interp, name: &[u8]) -> Code {
    let mut m = b"namespace \"".to_vec();
    m.extend_from_slice(name);
    if name.starts_with(b"::") {
        m.extend_from_slice(b"\" not found");
    } else {
        m.extend_from_slice(b"\" not found in \"");
        let cur = interp.namespaces().qualified_name(interp.current_ns());
        m.extend_from_slice(&cur);
        m.push(b'"');
    }
    let code = error_code_list(&[b"TCL", b"LOOKUP", b"NAMESPACE", name]);
    interp.error_with_code(&m, &code)
}

/// Does the dest namespace hold a command of this name?
fn dest_has_own(interp: &Interp, dest: NsId, simple: &[u8]) -> bool {
    interp.namespaces().command_names(dest).contains(&simple)
}

/// `namespace qualifiers` text op: everything before the last `::` separator,
/// with the whole trailing colon-run trimmed (`foo:::` → `foo`, `:::::` → ``);
/// empty if unqualified.
/// `string match` over bytes (UTF-8 → shared [`tcl_syntax::glob`]; a non-UTF-8
/// pattern or text can only match byte-identically, handled by the equality
/// fallback).
fn glob_match_bytes(pattern: &[u8], text: &[u8]) -> bool {
    tcl_syntax::glob::string_match_bytes(pattern, text)
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
/// `namespace code` scripts.
///
/// `NamespaceInscopeCmd` (`generic/tclNamesp.c`) is the one member of the
/// `Tcl_ConcatObj` eval family whose trailing words are **not** space-joined
/// into the script text: it collects the extra args into a **list object**
/// and concatenates that list's string rep onto the script, so each extra
/// word reaches the invoked command as exactly one argument however much
/// whitespace or list punctuation it holds:
///
/// ```text
/// namespace inscope :: {puts} {a b}   → prints "a b"  (one argument)
/// namespace eval    :: {puts} {a b}   → error: can not find channel named "a"
/// ```
///
/// (Twin of #1056/#1067, already fixed for the bytecode VM.) With no extra
/// args, C takes the `objc == 3` arm and evaluates the script verbatim — no
/// concat, so no trim and no trailing space, which the early return mirrors.
fn ns_inscope(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 4 {
        return interp.wrong_args(b"namespace inscope name arg ?arg...?");
    }
    let name = obj_bytes(argv[2]);
    let script = inscope_script(argv[3], &argv[4..]);
    interp.ns_eval(&name, &script)
}

/// Build the script `ns_inscope` evaluates: `Tcl_ConcatObj(script, list(tail))`.
/// No tail args → `script` verbatim (C's `objc == 3` arm). Otherwise the
/// tail's list-element quoting reuses the crate's canonical
/// [`list::append_list_element`] (`TclScanElement`/`TclConvertElement` — the
/// same helper the list type's own string rep uses), and the two-part concat
/// reuses [`list::trim_concat_element_bytes`] (`Tcl_ConcatObj`'s
/// backslash-aware right trim + drop-empty-part rule, operating on raw
/// bytes — this runtime's Tcl strings are arbitrary byte slices, not
/// necessarily UTF-8, so a lossy `&str` round-trip here would mangle a
/// non-UTF-8 script byte instead of passing it through): a whitespace-padded
/// script is trimmed, and an all-whitespace script contributes no leading
/// separator (the tail becomes the whole command).
fn inscope_script(script: *mut TclObj, tail: &[*mut TclObj]) -> Vec<u8> {
    let script_bytes = obj_bytes(script);
    if tail.is_empty() {
        return script_bytes;
    }
    let mut tail_list = Vec::new();
    for (i, &a) in tail.iter().enumerate() {
        if i > 0 {
            tail_list.push(b' ');
        }
        list::append_list_element(&mut tail_list, &obj_bytes(a), i == 0);
    }
    let trimmed = list::trim_concat_element_bytes(&script_bytes);
    if trimmed.is_empty() {
        return tail_list;
    }
    let mut out = trimmed.to_vec();
    out.push(b' ');
    out.extend_from_slice(&tail_list);
    out
}

/// `namespace origin command` — the fully-qualified original name of `command`
/// (following `namespace import` chains to the source).
fn ns_origin(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 3 {
        return interp.wrong_args(b"namespace origin name");
    }
    let name = obj_bytes(argv[2]);
    // The shared `TclGetOriginalCommand` walk (`tcl_cmd_core::namespace`).
    let origin = tcl_cmd_core::namespace::origin_bytes(interp, &name);
    match origin {
        Some(fqn) => {
            interp.set_result_bytes(&fqn);
            Code::Ok
        }
        None => {
            let mut m = b"invalid command name \"".to_vec();
            m.extend_from_slice(&name);
            m.push(b'"');
            let error_code = error_code_list(&[b"TCL", b"LOOKUP", b"COMMAND", &name]);
            interp.error_with_code(&m, &error_code)
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
        return interp.wrong_args(b"namespace code arg");
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
        return interp.wrong_args(b"namespace upvar ns ?otherVar myVar ...?");
    }
    let ns_name = obj_bytes(argv[2]);
    let Some(ns) = interp
        .namespaces()
        .find_namespace(interp.current_ns(), &ns_name)
    else {
        return ns_not_found(interp, &ns_name);
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
        return interp.wrong_args(b"namespace ensemble subcommand ?arg ...?");
    }
    // The shared `ensembleSubcommands` table (flags 0, so `cr` is `create`).
    let sub = obj_bytes(argv[2]);
    match tcl_cmd_core::ensemble::SUBCOMMANDS.index_of(&sub) {
        Ok(0) => ens_configure(interp, argv),
        Ok(1) => ens_create(interp, argv),
        Ok(_) => ens_exists(interp, argv),
        Err(message) => interp.set_error(&message),
    }
}

/// `namespace ensemble create ?-command name? ?-map dict? ?-subcommands list?
/// ?-prefixes bool?` — register an ensemble over the current namespace.
fn ens_create(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let ns = interp.current_ns();
    // Default ensemble command is the namespace's own FQN. The same FQN is
    // what CRT_MAP qualifies relative `-map` targets against, so keep it
    // separately — `command` is overwritten by an explicit `-command`.
    let ns_fqn = interp.namespaces().qualified_name(ns);
    let mut command = ns_fqn.clone();
    let mut cfg = EnsembleConfig {
        ns,
        map: None,
        subcommands: None,
        prefixes: true,
        parameters: Vec::new(),
        unknown: Vec::new(),
    };

    let opts = &argv[3..];
    // C checks the pair arity before it looks at any option word
    // (`if (objc & 1)` → `wrong # args`, `tclEnsemble.c:192-196`).
    if opts.len() % 2 != 0 {
        return interp.wrong_args(b"namespace ensemble create ?option value ...?");
    }
    for pair in opts.chunks_exact(2) {
        let opt = obj_bytes(pair[0]);
        // `ensembleCreateOptions`: `-command` (create-only) names the ensemble
        // command, the rest are the shared configuration options, and there is
        // deliberately no `-namespace`.
        let resolved = match tcl_cmd_core::ensemble::CreateOption::resolve(&opt) {
            Ok(resolved) => resolved,
            Err(message) => return interp.set_error(&message),
        };
        let Some(shared) = resolved.shared() else {
            // `-command` names the command rather than configuring it. C
            // creates that command in the ensemble's namespace (`cxtPtr =
            // nsPtr`) and reports it via `Tcl_GetCommandFullName`, so a
            // relative name binds — and reads back — fully qualified.
            command = qualify_in_ns(&ns_fqn, &obj_bytes(pair[1]));
            continue;
        };
        if let Err(e) = apply_ensemble_option(&mut cfg, shared, &obj_bytes(pair[1]), &ns_fqn) {
            return interp.set_error(&e);
        }
    }

    interp.create_ensemble(&command, cfg);
    interp.set_result_bytes(&command);
    Code::Ok
}

/// Apply one already-resolved shared `-option value` to an
/// [`EnsembleConfig`] (`namespace ensemble create` and `configure` both land
/// here; the option word itself is resolved by the caller's own table).
/// Returns the C error text on a bad value.
///
/// `map_ns` is the namespace unqualified `-map` targets are resolved against.
/// C uses the ensemble's own namespace on `create` and the namespace current
/// at the call on `configure`; both are the current namespace at the point
/// each command runs, which is what every caller passes.
fn apply_ensemble_option(
    cfg: &mut EnsembleConfig,
    opt: tcl_cmd_core::ensemble::SharedOption,
    val: &[u8],
    map_ns: &[u8],
) -> Result<(), Vec<u8>> {
    use tcl_cmd_core::ensemble::SharedOption;
    match opt {
        SharedOption::Subcommands => {
            cfg.subcommands =
                Some(crate::parse::split_list(val).map_err(|e| e.message().to_vec())?);
        }
        SharedOption::Map => {
            // An empty `-map` clears it (C: a zero-length dict ⇒ no map).
            let m = parse_map(val, map_ns)?;
            cfg.map = if m.is_empty() { None } else { Some(m) };
        }
        SharedOption::Parameters => {
            cfg.parameters = crate::parse::split_list(val).map_err(|e| e.message().to_vec())?;
        }
        SharedOption::Unknown => {
            cfg.unknown = crate::parse::split_list(val).map_err(|e| e.message().to_vec())?;
        }
        SharedOption::Prefixes => {
            // The one typed-read owner, so `-prefixes tru` / `-prefixes 2`
            // are accepted here exactly as `tclsh9.0` accepts them.
            cfg.prefixes = crate::typed_value::boolean_bytes(val).map_err(|e| e.message)?;
        }
    }
    Ok(())
}

/// `namespace ensemble configure cmd ?-option? ?value …?` — read or update an
/// existing ensemble's configuration (`tclEnsemble.c`). No options: a dict of
/// all settings; one bare `-option`: its value; `-option value …` pairs: update.
fn ens_configure(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 4 {
        return interp
            .wrong_args(b"namespace ensemble configure cmdname ?-option value ...? ?arg ...?");
    }
    let cmd = obj_bytes(argv[3]);
    // Follow a `namespace import` alias to the ensemble that owns the config:
    // reads and writes both act on the origin, so both spellings observe one
    // configuration and the alias stays an alias.
    let Some(token) = interp.ensemble_config_at(&cmd) else {
        // Distinguish a missing command from a non-ensemble one (C's wording).
        if interp.command_exists(&cmd) {
            let mut m = b"\"".to_vec();
            m.extend_from_slice(&cmd);
            m.extend_from_slice(b"\" is not an ensemble command");
            let error_code = error_code_list(&[b"TCL", b"LOOKUP", b"ENSEMBLE", &cmd]);
            return interp.error_with_code(&m, &error_code);
        }
        let mut m = b"unknown command \"".to_vec();
        m.extend_from_slice(&cmd);
        m.push(b'"');
        let error_code = error_code_list(&[b"TCL", b"LOOKUP", b"COMMAND", &cmd]);
        return interp.error_with_code(&m, &error_code);
    };
    let mut cfg = token.config();
    let rest = &argv[4..];
    // Read all options as a dict.
    if rest.is_empty() {
        let d = ensemble_config_dict(interp, &cfg);
        interp.set_result_bytes(&d);
        return Code::Ok;
    }
    // Read a single option's value (`ensembleConfigOptions`, abbreviating).
    if rest.len() == 1 {
        let opt = obj_bytes(rest[0]);
        return match tcl_cmd_core::ensemble::ConfigOption::resolve(&opt) {
            Ok(option) => {
                let v = ensemble_option_value(interp, &cfg, option);
                interp.set_result_bytes(&v);
                Code::Ok
            }
            Err(message) => interp.set_error(&message),
        };
    }
    // Update: `-option value` pairs. C's arity gate is
    // `objc != 4 && !(objc & 1)`, i.e. anything but 0, 1, or an even number of
    // trailing words is `wrong # args`.
    if rest.len() % 2 != 0 {
        return interp
            .wrong_args(b"namespace ensemble configure cmdname ?-option value ...? ?arg ...?");
    }
    // CONF_MAP qualifies against `TclGetCurrentNamespace(interp)` — the
    // namespace current at the `configure` call, NOT the ensemble's own
    // namespace (which CRT_MAP uses at create time). They coincide in the
    // common `namespace eval M {namespace ensemble configure …}` shape, but
    // configuring an ensemble from outside its namespace resolves relative
    // targets against the caller.
    let map_ns = interp.namespaces().qualified_name(interp.current_ns());
    for pair in rest.chunks_exact(2) {
        let resolved = match tcl_cmd_core::ensemble::ConfigOption::resolve(&obj_bytes(pair[0])) {
            Ok(resolved) => resolved,
            Err(message) => return interp.set_error(&message),
        };
        let Some(shared) = resolved.shared() else {
            return interp
                .error_with_code(b"option -namespace is read-only", b"TCL ENSEMBLE READ_ONLY");
        };
        if let Err(e) = apply_ensemble_option(&mut cfg, shared, &obj_bytes(pair[1]), &map_ns) {
            return interp.set_error(&e);
        }
    }
    token.configure(cfg);
    interp.set_result_bytes(b"");
    Code::Ok
}

/// One ensemble `configure`/cget option's value (string form).
fn ensemble_option_value(
    interp: &Interp,
    cfg: &EnsembleConfig,
    opt: tcl_cmd_core::ensemble::ConfigOption,
) -> Vec<u8> {
    use tcl_cmd_core::ensemble::ConfigOption;
    match opt {
        ConfigOption::Namespace => interp.namespaces().qualified_name(cfg.ns),
        ConfigOption::Prefixes => {
            if cfg.prefixes {
                b"1".to_vec()
            } else {
                b"0".to_vec()
            }
        }
        ConfigOption::Parameters => join_words(&cfg.parameters),
        ConfigOption::Unknown => join_words(&cfg.unknown),
        ConfigOption::Subcommands => cfg
            .subcommands
            .as_deref()
            .map(join_words)
            .unwrap_or_default(),
        ConfigOption::Map => match &cfg.map {
            Some(m) => {
                let mut flat: Vec<Vec<u8>> = Vec::with_capacity(m.len() * 2);
                for (k, prefix) in m {
                    flat.push(k.clone());
                    flat.push(join_words(prefix));
                }
                join_words(&flat)
            }
            None => Vec::new(),
        },
    }
}

/// Join words into a Tcl list string.
fn join_words(words: &[Vec<u8>]) -> Vec<u8> {
    let strs: Vec<std::borrow::Cow<str>> =
        words.iter().map(|w| String::from_utf8_lossy(w)).collect();
    tcl_syntax::list::join_list(strs.iter()).into_bytes()
}

/// The full `-option value …` dict an ensemble's `configure` (no args) returns,
/// in C's alphabetical option order.
fn ensemble_config_dict(interp: &Interp, cfg: &EnsembleConfig) -> Vec<u8> {
    let mut pairs: Vec<Vec<u8>> = Vec::new();
    for opt in tcl_cmd_core::ensemble::ConfigOption::all() {
        pairs.push(opt.name().as_bytes().to_vec());
        pairs.push(ensemble_option_value(interp, cfg, opt));
    }
    join_words(&pairs)
}

/// `namespace ensemble exists command` — 1 if it resolves to an ensemble.
fn ens_exists(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 4 {
        return interp.wrong_args(b"namespace ensemble exists cmdname");
    }
    let exists = interp.is_ensemble(&obj_bytes(argv[3]));
    interp.set_result_bytes(if exists { b"1" } else { b"0" });
    Code::Ok
}

/// Parse a `-map` dict (`sub {target prefix} …`) into (subcommand, prefix-words).
fn parse_map(bytes: &[u8], map_ns: &[u8]) -> Result<EnsembleMap, Vec<u8>> {
    let kvs = crate::parse::split_list(bytes).map_err(|error| {
        tcl_cmd_core::dict::worded_parse_error(&String::from_utf8_lossy(error.message()))
            .into_bytes()
    })?;
    if kvs.len() % 2 != 0 {
        return Err(b"missing value to go with key".to_vec());
    }
    let mut map = Vec::with_capacity(kvs.len() / 2);
    for pair in kvs.chunks_exact(2) {
        let mut prefix = crate::parse::split_list(&pair[1]).map_err(|e| e.message().to_vec())?;
        // Only the target word is qualified; the rest of the prefix is fixed
        // leading arguments. An empty prefix is left alone — the "must be
        // non-empty lists" check is a separate concern.
        if let Some(target) = prefix.first_mut() {
            *target = qualify_in_ns(map_ns, target);
        }
        // The `-map` value is a *dict*, so a repeated key collapses: the last
        // value wins but keeps the first occurrence's position. Pushing blindly
        // would leave both copies, making the read-back disagree with tclsh and
        // dispatch pick the stale first target.
        match map.iter_mut().find(|(k, _)| *k == pair[0]) {
            Some((_, slot)) => *slot = prefix,
            None => map.push((pair[0].clone(), prefix)),
        }
    }
    tcl_cmd_core::ensemble::validate_map_targets(&map)
        .map_err(|error| error.into_message().into_bytes())?;
    Ok(map)
}

/// Namespace-qualify one `-map` target the way C does (`tclEnsemble.c` CRT_MAP
/// / CONF_MAP): a target already starting with `::` is left alone, anything
/// else is prefixed with `ns` (plus a `::` separator unless `ns` is the global
/// namespace, whose name already ends in the separator).
///
/// This runs at *parse* time, so the qualified form is what the ensemble
/// stores, what dispatch calls, and what `-map` reads back — a relative target
/// left raw would be looked up in whatever namespace happened to be current at
/// call time, and so would usually be uncallable.
fn qualify_in_ns(ns: &[u8], target: &[u8]) -> Vec<u8> {
    if target.starts_with(b"::") {
        return target.to_vec();
    }
    let mut out = ns.to_vec();
    if ns != b"::" {
        out.extend_from_slice(b"::");
    }
    out.extend_from_slice(target);
    out
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

    /// `-prefixes` reads its value through the runtime's one boolean acceptor,
    /// so every spelling `tclsh9.0` accepts here is accepted (issue #1425's
    /// runtime half): a unique word prefix, and any number against zero. Only
    /// the ambiguous `o` — shared by `on` and `off` — is refused.
    #[test]
    fn ensemble_prefixes_accepts_every_boolean_spelling_tclsh_does() {
        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"namespace eval ens { proc bar {} {}; namespace export bar; \
                      namespace ensemble create }"
                ),
                Code::Ok
            );
            for (spelling, expected) in [
                (&b"tru"[..], b"1".as_slice()),
                (b"ye", b"1"),
                (b"of", b"0"),
                (b"2", b"1"),
                (b"0.0", b"0"),
            ] {
                let mut script = b"namespace ensemble configure ::ens -prefixes ".to_vec();
                script.extend_from_slice(spelling);
                assert_eq!(
                    i.eval_str(&script),
                    Code::Ok,
                    "{}: {}",
                    String::from_utf8_lossy(spelling),
                    String::from_utf8_lossy(&i.result_bytes())
                );
                assert_eq!(
                    i.eval_str(b"namespace ensemble configure ::ens -prefixes"),
                    Code::Ok
                );
                assert_eq!(i.result_bytes(), expected);
            }
            assert_eq!(
                i.eval_str(b"namespace ensemble configure ::ens -prefixes o"),
                Code::Error
            );
            assert_eq!(i.result_bytes(), b"expected boolean value but got \"o\"");
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

            assert_eq!(
                i.eval_str(
                    b"namespace eval ::a {
                          catch {namespace parent {not here}} pm po
                          catch {namespace children {also not here}} cm co
                          list $pm [dict get $po -errorcode] \
                               $cm [dict get $co -errorcode]
                      }"
                ),
                Code::Ok
            );
            assert_eq!(
                i.result_bytes(),
                b"{namespace \"not here\" not found in \"::a\"} \
                  {TCL LOOKUP NAMESPACE {not here}} \
                  {namespace \"also not here\" not found in \"::a\"} \
                  {TCL LOOKUP NAMESPACE {also not here}}"
            );
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

    /// Tcl 9.0.4 oracle vectors from issue #1584. These exercise the runtime
    /// adapter through the same shared namespace grammar as the bytecode VM.
    #[test]
    fn namespace_issue_1584_oracle_vectors() {
        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"namespace eval declared {variable only
                      list [namespace which -variable only] [info vars] [info exists only]}"
                ),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"::declared::only only 0");

            for script in [
                &b"namespace which -zork puts"[..],
                b"namespace which -command puts extra",
            ] {
                assert_eq!(i.eval_str(script), Code::Error);
                assert_eq!(
                    i.result_bytes(),
                    b"wrong # args: should be \"namespace which ?-command? ?-variable? name\""
                );
            }

            assert_eq!(
                i.eval_str(b"namespace eval self {namespace import ::self::*}"),
                Code::Error
            );
            assert_eq!(
                i.result_bytes(),
                b"import pattern \"::self::*\" tries to import from namespace \"self\" into itself"
            );
            assert_eq!(
                i.eval_str(b"namespace eval dest {namespace import ::nosuch::*}"),
                Code::Error
            );
            assert_eq!(
                i.result_bytes(),
                b"unknown namespace in import pattern \"::nosuch::*\""
            );

            for script in [&b"namespace origin"[..], b"namespace origin set extra"] {
                assert_eq!(i.eval_str(script), Code::Error);
                assert_eq!(
                    i.result_bytes(),
                    b"wrong # args: should be \"namespace origin name\""
                );
            }

            assert_eq!(
                i.eval_str(
                    b"namespace eval order {}
                      foreach n {one two three four five six seven eight nine ten} {
                          namespace eval ::order::$n {}
                      }
                      namespace children ::order"
                ),
                Code::Ok
            );
            assert_eq!(
                i.result_bytes(),
                b"::order::six ::order::four ::order::three ::order::eight \
                  ::order::seven ::order::nine ::order::five ::order::two \
                  ::order::one ::order::ten"
            );

            assert_eq!(
                i.eval_str(
                    b"namespace eval p {}
                      foreach n {a0 a1 a2 a3 a4 a5 a6 a7 a8 a9 a10 a11} {
                          namespace eval ::p::$n {}
                      }
                      foreach i {1 2 4 5 6 7 8 9 10 11} {namespace delete ::p::a$i}
                      namespace children ::p"
                ),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"::p::a0 ::p::a3");
        });

        // The active command survives long enough to return even though the
        // global namespace and command table have been torn down.
        leak_free(|i| {
            assert_eq!(i.eval_str(b"namespace delete ::"), Code::Ok);
            assert_eq!(i.result_bytes(), b"");
            assert_eq!(i.eval_str(b"puts hi"), Code::Error);
            assert_eq!(i.result_bytes(), b"invalid command name \"puts\"");
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
    fn qualifier_namespace_existing_does_not_commit_resolution() {
        // tclsh 8.6/9.0 (PR #924): the fallback is *command*-existence-
        // checked, not namespace-existence-checked.  `inner::p` from
        // `::outer` must dispatch `::inner::p` even though the namespace
        // `::outer::inner` exists — it merely holds no `p`.
        leak_free(|i| {
            i.eval_str(b"namespace eval inner { proc p {} {return GLOBAL} }");
            i.eval_str(b"namespace eval outer { namespace eval inner { proc other {} {} } }");
            assert_eq!(
                i.eval_str(b"namespace eval outer { inner::p }"),
                Code::Ok,
                "resolution must fall through to ::inner::p: {:?}",
                String::from_utf8_lossy(&i.result_bytes()),
            );
            assert_eq!(i.result_bytes(), b"GLOBAL");
        });
    }

    /// Every shared command-resolution vector
    /// (`tcl_syntax::naming::conformance`, pinned against real tclsh by
    /// the `tcl-syntax` conformance test) must dispatch identically
    /// through this runtime's namespace tree — the anti-drift gate for
    /// `Namespaces::home_of`.
    ///
    /// Issue #1058: this used to be skipped whenever libtommath was not
    /// vendored, because the shared `vector_script` renderer captures the
    /// call with `if {[catch {…} __r]} {…}` and `builtins::install` only
    /// registers `if`/`while`/`for` under `have_tommath` — so the whole
    /// gate reported `invalid command name "if"` on the first vector and
    /// was ignored. That is a *capture-script* dependency, not a dispatch
    /// one: nothing about command resolution needs the numeric tower.
    ///
    /// So the capture is composed here from the tower-free half of the
    /// renderer (`vector_setup` + `vector_call`) with `set` and `catch`
    /// standing in for `if` — `set __r -` then `catch {set __r [call]}`
    /// leaves the dispatched name in `__r`, or `-` when the call raised,
    /// which is exactly what the `if` form computes. The vectors now run in
    /// **every** build of this crate, tower or no tower, and the skip is
    /// gone.
    #[test]
    fn dispatch_matches_every_conformance_vector() {
        use tcl_syntax::naming::conformance::{vector_call, vector_setup, vectors};
        for v in vectors() {
            let script = format!(
                "{}set __r -\ncatch {{set __r [{}]}}\n",
                vector_setup(&v),
                vector_call(&v),
            );
            let body = script.clone();
            counters::reset();
            {
                let mut i = Interp::new();
                let code = i.eval_str(body.as_bytes());
                assert_eq!(
                    code,
                    Code::Ok,
                    "vector line {}: runtime errored on script:\n{script}\nerror: {}",
                    v.line,
                    String::from_utf8_lossy(&i.result_bytes()),
                );
                assert_eq!(i.eval_str(b"set __r"), Code::Ok);
                let got = String::from_utf8_lossy(&i.result_bytes()).to_string();
                let want = v.want().unwrap_or_else(|| "-".to_string());
                assert_eq!(
                    got, want,
                    "vector line {} (ns={} path={:?} defs={:?} call={}): runtime dispatch \
                     disagrees with C Tcl\nscript:\n{script}",
                    v.line, v.ns, v.path, v.defs, v.call,
                );
            }
            assert_eq!(counters::finalize(), 0, "vector line {}: leak", v.line);
            assert_eq!(counters::double_free_count(), 0);
        }
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
            assert_eq!(i.result_bytes(), b"9.0.4");
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

    /// C imports hold the source's command *token*: renaming the source
    /// keeps the import working and `namespace origin` reports the NEW name;
    /// deleting the source leaves the import dangling. Pinned on tclsh
    /// 8.6.16 / 9.0.4 (`rename ::src::e ::src::e2` → `::dst::e` still runs,
    /// origin `::src::e2`; `rename ::src2::f ""` → `::dst2::f` errors).
    #[test]
    fn import_follows_source_rename_but_not_delete() {
        leak_free(|i| {
            i.eval_str(b"namespace eval src { namespace export e; proc e {} { return E } }");
            i.eval_str(b"namespace eval dst { namespace import ::src::e }");
            assert_eq!(i.eval_str(b"rename ::src::e ::src::e2"), Code::Ok);
            assert_eq!(i.eval_str(b"::dst::e"), Code::Ok);
            assert_eq!(i.result_bytes(), b"E");
            assert_eq!(i.eval_str(b"namespace origin ::dst::e"), Code::Ok);
            assert_eq!(i.result_bytes(), b"::src::e2");
            // Delete dangles (lazy miss), as in C.
            assert_eq!(i.eval_str(b"rename ::src::e2 {}"), Code::Ok);
            assert_eq!(i.eval_str(b"::dst::e"), Code::Error);
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

    /// Tcl 9.0.4 oracle vectors from issue #1583: dict validation, callback
    /// prefix redispatch/reparse, and the user-facing default target name.
    #[test]
    fn ensemble_issue_1583_oracle_vectors() {
        leak_free(|i| {
            for (script, message) in [
                (
                    &b"namespace ensemble create -map {go}"[..],
                    &b"missing value to go with key"[..],
                ),
                (
                    b"namespace ensemble create -map {go {}}",
                    b"ensemble subcommand implementations must be non-empty lists",
                ),
                (
                    b"set badmap \"go \\{\"; namespace ensemble create -map $badmap",
                    b"unmatched open brace in dict",
                ),
            ] {
                assert_eq!(i.eval_str(script), Code::Error);
                assert_eq!(i.result_bytes(), message);
            }

            assert_eq!(
                i.eval_str(
                    b"proc uh {ens args} {return [list list REPLACED $ens]}
                      namespace eval se5 {
                          namespace ensemble create -command ::se5 -subcommands {} -unknown ::uh
                      }
                      ::se5 nope 1 2"
                ),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"REPLACED ::se5 1 2");

            assert_eq!(
                i.eval_str(
                    b"proc define {ens args} {
                          namespace eval se6 {
                              proc nope args {return DEFINED}; namespace export nope
                          }
                          return {}
                      }
                      namespace eval se6 {
                          namespace ensemble create -command ::se6 -unknown ::define
                      }
                      ::se6 nope"
                ),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"DEFINED");

            assert_eq!(
                i.eval_str(
                    b"proc target args {return TARGET}
                      proc repair {ens args} {
                          namespace ensemble configure $ens -map {nope ::target}; return {}
                      }
                      namespace eval se7 {
                          namespace ensemble create -command ::se7 -unknown ::repair
                      }
                      ::se7 nope"
                ),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"TARGET");

            assert_eq!(
                i.eval_str(
                    b"namespace eval k1 {namespace ensemble create -subcommands ghost}
                      ::k1 ghost"
                ),
                Code::Error
            );
            assert_eq!(i.result_bytes(), b"invalid command name \"ghost\"");
        });
    }

    #[test]
    fn ensemble_unknown_retains_the_live_command_token() {
        // Same-name create is replacement, not mutation: the callback's old
        // active token is dead even though a new ensemble occupies `::E`.
        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"proc new args {return NEW}
                      proc rebuild {ens args} {
                          namespace eval ::N {
                              namespace ensemble create -command ::E -map {x ::new}
                          }
                          return {}
                      }
                      namespace eval ::N {
                          namespace ensemble create -command ::E -unknown ::rebuild
                      }
                      set c [catch {::E nope} m o]
                      list $c $m [dict get $o -errorcode] [::E x]"
                ),
                Code::Ok
            );
            assert_eq!(
                i.result_bytes(),
                b"1 {unknown subcommand handler deleted its ensemble} {TCL ENSEMBLE UNKNOWN_DELETED} NEW"
            );
        });

        // Replacement fires and drops the old command's delete trace.
        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"set seen {}
                      proc deleted args {lappend ::seen $args}
                      namespace ensemble create -command ::E
                      trace add command ::E delete deleted
                      set replacement [namespace ensemble create -command ::E]
                      list $seen $replacement"
                ),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"{{::E {} delete}} ::E");
        });

        // The delete trace may move the captured old token. Replacement must
        // retire that identity at its new location, then install the fresh
        // ensemble at the original binding.
        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"proc move_old {old new op} {rename $old ::OLD}
                      namespace eval N {namespace ensemble create -command ::E}
                      trace add command ::E delete move_old
                      namespace eval N {namespace ensemble create -command ::E}
                      list [info commands ::E] [info commands ::OLD] \
                           [namespace ensemble exists ::E] \
                           [namespace ensemble exists ::OLD]"
                ),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"::E {} 1 0");
        });

        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"proc target args {return TARGET}
                      namespace eval ER {
                          proc repair {ens args} {
                              namespace ensemble configure $ens -map {nope ::target}
                              rename $ens ::ER2
                              return {}
                          }
                          namespace ensemble create -command ::ER -unknown ::ER::repair
                      }
                      list [::ER nope] [namespace ensemble configure ::ER2 -map] [::ER2 nope]"
                ),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"TARGET {nope ::target} TARGET");
        });

        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"set seen {}
                      proc target args {return TARGET}
                      proc repair {ens args} {
                          set ::seen $ens
                          namespace ensemble configure $ens -map {nope ::target}
                          return {}
                      }
                      namespace eval S {
                          namespace export E
                          namespace ensemble create -command E -unknown ::repair
                      }
                      namespace eval I {namespace import ::S::E}
                      list [::I::E nope] $seen [namespace origin ::I::E]"
                ),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"TARGET ::S::E ::S::E");
        });

        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"proc target args {return TARGET}
                      proc replace {ens args} {
                          rename $ens {}
                          namespace ensemble create -command $ens -map {nope ::target}
                          return \\{
                      }
                      namespace eval D {
                          namespace ensemble create -command E -unknown ::replace
                      }
                      set c [catch {::D::E nope} m o]
                      list $c $m [dict get $o -errorcode] [::D::E nope]"
                ),
                Code::Ok
            );
            assert_eq!(
                i.result_bytes(),
                b"1 {unknown subcommand handler deleted its ensemble} {TCL ENSEMBLE UNKNOWN_DELETED} TARGET"
            );
        });

        leak_free(|i| {
            let code = i.eval_str(
                b"proc target args {return TARGET}
                      set seen {}
                      proc hide_repair {ens args} {
                          namespace ensemble configure $ens -map {nope ::target} -unknown ::hidden_repair
                          interp hide {} $ens heldE
                          return {}
                      }
                      proc hidden_repair {ens args} {
                          set ::seen $ens
                          return [list ::target]
                      }
                      namespace ensemble create -command ::EH -unknown ::hide_repair
                      set first [::EH nope]
                      set seen {}
                      set second [interp invokehidden {} heldE other]
                      list $first $second $seen [info commands ::EH] [interp hidden {}]",
            );
            assert_eq!(
                code,
                Code::Ok,
                "{}",
                String::from_utf8_lossy(&i.result_bytes())
            );
            assert_eq!(i.result_bytes(), b"TARGET TARGET ::heldE {} heldE");
        });

        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"proc zap {ens args} {namespace delete ::ND; return {}}
                      namespace eval ND {
                          namespace ensemble create -command ::NDE -unknown ::zap
                      }
                      set c [catch {::NDE nope} m o]
                      list $c $m [dict get $o -errorcode] [info commands ::NDE]"
                ),
                Code::Ok
            );
            assert_eq!(
                i.result_bytes(),
                b"1 {unknown subcommand handler deleted its ensemble} {TCL ENSEMBLE UNKNOWN_DELETED} {}"
            );
        });
    }

    /// Exact Tcl 9.0.4 command-token oracles: an ensemble import follows its
    /// source token through hide/expose, not a replacement installed at the
    /// vacated name; recreating an occupied ensemble retargets the import to a
    /// new token; true deletion removes the import permanently.
    #[test]
    fn imported_ensemble_retains_source_token_identity() {
        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"proc tgt_old args {return OLD}
                      proc tgt_new args {return NEW}
                      namespace eval S {
                          namespace export E
                          namespace ensemble create -command ::S::E -map {x ::tgt_old}
                      }
                      namespace eval I {namespace import ::S::E}
                      namespace eval S {
                          namespace ensemble create -command ::S::E -map {x ::tgt_new}
                      }
                      list [::I::E x] [namespace origin ::I::E] \
                           [namespace ensemble configure ::I::E -map]"
                ),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"NEW ::S::E {x ::tgt_new}");

            assert_eq!(
                i.eval_str(
                    b"rename ::S::E {}
                      set before [list [info commands ::I::E] \
                                           [namespace eval I {namespace import}]]
                      namespace eval S {
                          namespace ensemble create -command ::S::E -map {x ::tgt_new}
                      }
                      list {*}$before [info commands ::I::E] \
                           [namespace eval I {namespace import}]"
                ),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"{} {} {} {}");
        });

        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"proc tgt_old args {return OLD}
                      namespace eval Source {
                          namespace ensemble create -command ::E -map {x ::tgt_old}
                      }
                      namespace export E
                      namespace eval I {namespace import ::E}
                      interp hide {} E held
                      proc E args {return REPLACEMENT}
                      list [::I::E x] [namespace origin ::I::E] \
                           [namespace ensemble configure ::I::E -map] \
                           [namespace ensemble exists ::I::E]"
                ),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"OLD ::held {x ::tgt_old} 1");

            assert_eq!(
                i.eval_str(
                    b"interp expose {} held E2
                      list [::I::E x] [namespace origin ::I::E] \
                           [namespace ensemble configure ::I::E -map] [::E2 x]"
                ),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"OLD ::E2 {x ::tgt_old} OLD");
        });

        // Replacing an occupied source binding with a non-ensemble keeps the
        // import attached to the source command token. The retained ensemble
        // token is now dead, so dispatch and origin fall back to the by-name
        // source, including after that replacement is renamed.
        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"proc tgt_old args {return OLD}
                      namespace eval S {
                          namespace export E
                          namespace ensemble create -command E -map {x ::tgt_old}
                      }
                      namespace eval I {namespace import ::S::E}
                      proc ::S::E args {return PROC}
                      set first [list [::I::E x] [namespace origin ::I::E] \
                                          [namespace ensemble exists ::I::E] \
                                          [info commands ::I::E]]
                      rename ::S::E ::S::E2
                      list {*}$first [::I::E x] [namespace origin ::I::E]"
                ),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"PROC ::S::E 0 ::I::E PROC ::S::E2");
        });

        // The by-name source shadow moves even while the retained token is
        // live. Replacing the renamed ensemble with a proc retires the token;
        // fallback and origin therefore use the renamed source, and keep
        // following it across a subsequent proc rename.
        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"namespace eval S {
                          namespace export E
                          namespace ensemble create -command E
                      }
                      namespace eval I {namespace import ::S::E}
                      rename ::S::E ::S::Moved
                      proc ::S::Moved args {return PROC}
                      set first [list [::I::E] [namespace origin ::I::E] \
                                          [namespace ensemble exists ::I::E]]
                      rename ::S::Moved ::S::Final
                      list {*}$first [::I::E] [namespace origin ::I::E]"
                ),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"PROC ::S::Moved 0 PROC ::S::Final");
        });
    }

    /// True command deletion removes every import of the source command,
    /// including transitive and hidden ordinary-proc aliases. Recreating the
    /// source binding does not resurrect any of them. Namespace teardown uses
    /// the same generic origin-deletion seam.
    #[test]
    fn true_source_deletion_purges_generic_import_origins() {
        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"namespace eval S {
                          namespace export p
                          proc p {} {return OLD}
                      }
                      namespace eval I {
                          namespace import ::S::p
                          namespace export p
                      }
                      namespace eval J {namespace import ::I::p}
                      namespace import ::S::p
                      interp hide {} p heldImport
                      set before [list [namespace origin ::I::p] \
                                           [namespace origin ::J::p] [interp hidden {}]]
                      rename ::S::p {}
                      namespace eval S {proc p {} {return NEW}}
                      list {*}$before [info commands ::I::p] \
                           [info commands ::J::p] [interp hidden {}] \
                           [namespace eval I {namespace import}] \
                           [namespace eval J {namespace import}] \
                           [info commands ::p]"
                ),
                Code::Ok
            );
            assert_eq!(
                i.result_bytes(),
                b"::S::p ::S::p heldImport {} {} {} {} {} {}"
            );
        });

        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"namespace eval S {
                          namespace export p
                          proc p {} {return OLD}
                      }
                      namespace eval I {namespace import ::S::p}
                      namespace delete ::S
                      namespace eval S {
                          namespace export p
                          proc p {} {return NEW}
                      }
                      list [info commands ::I::p] \
                           [namespace eval I {namespace import}]"
                ),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"{} {}");
        });
    }

    /// Each import retains its immediate source command. Replacing the
    /// intermediate import changes what downstream aliases invoke and report as
    /// their origin; true deletion of that intermediate command removes them.
    #[test]
    fn transitive_import_retains_intermediate_binding_lifecycle() {
        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"namespace eval S {
                          namespace export p
                          proc p {} {return S}
                      }
                      namespace eval A {
                          namespace import ::S::p
                          namespace export p
                      }
                      namespace eval B {namespace import ::A::p}
                      proc ::A::p {} {return A}
                      set before [list [::B::p] [namespace origin ::B::p]]
                      rename ::A::p {}
                      list {*}$before [info commands ::B::p] \
                           [namespace eval B {namespace import}]"
                ),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"A ::A::p {} {}");
        });

        // Independent true-delete path: A is still an imported command when it
        // is deleted. B is therefore deleted with A, while the original source
        // command in S remains live.
        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"namespace eval S {
                          namespace export p
                          proc p {} {return S}
                      }
                      namespace eval A {
                          namespace import ::S::p
                          namespace export p
                      }
                      namespace eval B {namespace import ::A::p}
                      rename ::A::p {}
                      list [::S::p] [info commands ::B::p] \
                           [namespace eval B {namespace import}]"
                ),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"S {} {}");
        });
    }

    #[test]
    fn namespace_import_rejects_a_cycle_before_mutating_the_graph() {
        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"namespace eval S {
                          proc p {} {return S}
                          namespace export p
                      }
                      namespace eval A {
                          namespace import ::S::p
                          namespace export p
                      }
                      namespace eval B {
                          namespace import ::A::p
                          namespace export p
                      }
                      set no_force_code [catch {
                          namespace eval S {namespace import ::B::p}
                      } no_force_message no_force_options]
                      set no_force [list $no_force_code $no_force_message \
                          [dict get $no_force_options -errorcode] \
                          [namespace origin ::S::p] \
                          [namespace origin ::A::p] \
                          [namespace origin ::B::p]]
                      set force_code [catch {
                          namespace eval S {namespace import -force ::B::p}
                      } force_message force_options]
                      list {*}$no_force $force_code $force_message \
                           [dict get $force_options -errorcode] \
                           [namespace origin ::S::p] \
                           [namespace origin ::A::p] \
                           [namespace origin ::B::p]"
                ),
                Code::Ok
            );
            assert_eq!(
                i.result_bytes(),
                b"1 {can't import command \"p\": already exists} \
                  {TCL IMPORT OVERWRITE} ::S::p ::S::p ::S::p \
                  1 {import pattern \"::B::p\" would create a loop containing command \"::S::p\"} \
                  {TCL IMPORT LOOP} ::S::p ::S::p ::S::p"
            );
        });
    }

    /// `namespace import -force` is command replacement, so it uses the same
    /// lifecycle as `proc` redefinition: the displaced token's delete trace
    /// runs while the old command remains visible, and none of its command or
    /// execution trace sidecars transfer to the fresh imported token.
    #[test]
    fn forced_import_replacement_runs_command_lifecycle() {
        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"set seen {}
                      proc deleted {old new op} {
                          lappend ::seen [list delete $old [info commands $old]]
                      }
                      proc entered {cmd op} {
                          lappend ::seen [list enter $cmd $op]
                      }
                      namespace eval S {
                          proc p {} {return SRC}
                          namespace export p
                      }
                      namespace eval D {proc p {} {return OLD}}
                      trace add command ::D::p delete deleted
                      trace add execution ::D::p enter entered
                      namespace eval D {namespace import -force ::S::p}
                      set traces [list [trace info command ::D::p] \
                                           [trace info execution ::D::p]]
                      set result [::D::p]
                      list $seen $traces $result [namespace origin ::D::p]"
                ),
                Code::Ok
            );
            assert_eq!(
                i.result_bytes(),
                b"{{delete ::D::p ::D::p}} {{} {}} SRC ::S::p"
            );
        });
    }

    /// Imported-command delete traces run before unbinding, and a command
    /// recreated by the trace callback has a new identity that survives the old
    /// import's deletion. Downstream imports then resolve through that live
    /// intermediate replacement.
    #[test]
    fn imported_delete_trace_observes_and_can_replace_the_binding() {
        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"namespace eval S {
                          namespace export p
                          proc p {} {return S}
                      }
                      namespace eval I {
                          namespace import ::S::p
                          namespace export p
                      }
                      namespace eval B {namespace import ::I::p}
                      set seen {}
                      proc cb {old new op} {
                          lappend ::seen [info commands $old]
                          proc $old {} {return REBORN}
                      }
                      trace add command ::I::p delete cb
                      rename ::S::p {}
                      list $seen [::I::p] [::B::p] [namespace origin ::B::p]"
                ),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"::I::p REBORN REBORN ::I::p");
        });

        // Re-importing the same origin from the delete callback creates a fresh
        // imported-command identity. The outer deletion retires only the old
        // import even though both identities carry identical source metadata.
        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"namespace eval S {
                          namespace export p
                          proc p {} {return S}
                      }
                      namespace eval A {namespace import ::S::p}
                      proc reimport {old new op} {
                          namespace eval ::A {namespace import -force ::S::p}
                      }
                      trace add command ::A::p delete reimport
                      rename ::A::p {}
                      list [info commands ::A::p] [::A::p] \
                           [namespace origin ::A::p]"
                ),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"::A::p S ::S::p");
        });

        // A trace may move the dying import itself. Identity cleanup follows
        // it and silently drops the moved sidecar, so a later unrelated
        // command at that name cannot fire the old trace.
        leak_free(|i| {
            let code = i.eval_str(
                b"namespace eval S {
                          namespace export p
                          proc p {} {return S}
                      }
                      namespace eval I {namespace import ::S::p}
                      set seen {}
                      proc move_import {old new op} {
                          lappend ::seen [list $old $new $op [info commands $old]]
                          rename $old ::I::q
                      }
                      trace add command ::I::p delete move_import
                      rename ::S::p {}
                      set first $seen
                      proc ::I::q {} {return unrelated}
                      rename ::I::q {}
                      list $first $seen [info commands ::I::p] \
                           [info commands ::I::q]",
            );
            assert_eq!(
                code,
                Code::Ok,
                "{}",
                String::from_utf8_lossy(&i.result_bytes())
            );
            assert_eq!(
                i.result_bytes(),
                b"{{::I::p {} delete ::I::p}} {{::I::p {} delete ::I::p}} {} {}"
            );
        });
    }

    /// Tcl 9.0.4 deletes a hidden real ensemble when its configured namespace
    /// dies, including its hidden-table binding and every import that retains
    /// the token. The active unknown callback therefore observes a dead token.
    #[test]
    fn namespace_delete_retires_hidden_ensemble_token() {
        // Namespace deletion fires a visible namespace-owned ensemble's delete
        // trace before marking/detaching the namespace. Tcl's ensemble list is
        // retired first, so the callback still sees both the owning namespace
        // and the captured command token; teardown removes both afterwards.
        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"set seen {}
                      proc observe {old new op} {
                          lappend ::seen [list [namespace exists ::N] $old \
                              [info commands $old] \
                              [namespace ensemble exists $old]]
                      }
                      namespace eval N {namespace ensemble create -command ::E}
                      trace add command ::E delete observe
                      namespace delete ::N
                      list $seen [namespace exists ::N] [info commands ::E]"
                ),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"{{1 ::E ::E 1}} 0 {}");
        });

        // Exposing a hidden victim from its delete callback only moves the old
        // identity; teardown follows it and removes both the command and its
        // moved trace sidecar.
        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"set seen {}
                      proc expose_dying {old new op} {
                          lappend ::seen [list $old [info commands $old]]
                          interp expose {} held E2
                      }
                      namespace eval N {namespace ensemble create -command ::E}
                      trace add command ::E delete expose_dying
                      interp hide {} E held
                      namespace delete ::N
                      list $seen [interp hidden {}] [info commands ::E2]"
                ),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"{{::held {}}} {} {}");
        });

        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"proc zap {ens args} {
                          interp hide {} NDE held
                          namespace delete ::ND
                          return {}
                      }
                      namespace eval ND {
                          namespace ensemble create -command ::NDE -unknown ::zap
                      }
                      namespace export NDE
                      namespace eval I {namespace import ::NDE}
                      set c [catch {::NDE nope} m o]
                      list $c $m [dict get $o -errorcode] [interp hidden {}] \
                           [info commands ::NDE] [info commands ::I::NDE] \
                           [namespace eval I {namespace import}]"
                ),
                Code::Ok
            );
            assert_eq!(
                i.result_bytes(),
                b"1 {unknown subcommand handler deleted its ensemble} \
                  {TCL ENSEMBLE UNKNOWN_DELETED} {} {} {} {}"
            );
        });

        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"proc tgt args {return OK}
                      namespace eval HS {
                          namespace export E
                          namespace ensemble create -command E -map {x ::tgt}
                      }
                      namespace import ::HS::E
                      interp hide {} E heldImport
                      namespace delete ::HS
                      set c [catch {interp invokehidden {} heldImport x} m]
                      list [interp hidden {}] $c $m"
                ),
                Code::Ok
            );
            assert_eq!(
                i.result_bytes(),
                b"{} 1 {invalid hidden command name \"heldImport\"}"
            );
        });

        // Hidden real ensembles and hidden imported commands both carry their
        // command trace sidecars to the hidden live name. Namespace-driven
        // retirement fires each once and drops it before an unrelated command
        // later occupies the old visible name.
        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"set seen {}
                      proc cb {old new op} {lappend ::seen [list $old $new $op]}
                      namespace eval N {namespace ensemble create -command ::E}
                      trace add command ::E delete cb
                      interp hide {} E held
                      namespace delete ::N
                      proc ::E {} {}
                      list $seen [trace info command ::E] [interp hidden {}]"
                ),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"{{::held {} delete}} {} {}");
        });

        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"set seen {}
                      proc cb {old new op} {lappend ::seen [list $old $new $op]}
                      namespace eval S {
                          namespace export p
                          proc p {} {return P}
                      }
                      namespace import ::S::p
                      trace add command ::p delete cb
                      interp hide {} p heldImport
                      namespace delete ::S
                      proc ::p {} {}
                      list $seen [trace info command ::p] [interp hidden {}]"
                ),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"{{::heldImport {} delete}} {} {}");
        });
    }

    /// A nonempty unknown result is spliced using the post-callback parameter
    /// count. Tcl 9.0.4 treats `missing` as the newly-added second parameter and
    /// removes `newsub` as the live subcommand word.
    #[test]
    fn ensemble_unknown_prefix_uses_live_parameter_layout() {
        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"proc target args {return $args}
                      proc mutate {ens args} {
                          namespace ensemble configure $ens -parameters {p q}
                          return ::target
                      }
                      namespace eval M {
                          namespace ensemble create -command ::M \
                              -parameters p -unknown ::mutate
                      }
                      ::M P missing newsub tail"
                ),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"P missing tail");
        });
    }

    /// Exact Tcl 9.0.4 result-code and list-parser diagnostics for an ensemble
    /// `-unknown` callback.
    #[test]
    fn ensemble_unknown_normalizes_bad_results() {
        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"proc u {mode ens args} {return -code $mode RESULT}
                      namespace ensemble create -command ::E -unknown {::u break}
                      set out {}
                      foreach mode {break continue return 7} {
                          namespace ensemble configure ::E -unknown [list ::u $mode]
                          set c [catch {::E nope} m o]
                          lappend out $c $m [dict get $o -errorcode]
                      }
                      set out"
                ),
                Code::Ok
            );
            assert_eq!(
                i.result_bytes(),
                b"1 {unknown subcommand handler returned bad code: break} \
                  {TCL ENSEMBLE UNKNOWN_RESULT} \
                  1 {unknown subcommand handler returned bad code: continue} \
                  {TCL ENSEMBLE UNKNOWN_RESULT} \
                  1 {unknown subcommand handler returned bad code: return} \
                  {TCL ENSEMBLE UNKNOWN_RESULT} \
                  1 {unknown subcommand handler returned bad code: 7} \
                  {TCL ENSEMBLE UNKNOWN_RESULT}"
            );

            assert_eq!(
                i.eval_str(
                    b"namespace ensemble configure ::E -unknown {::u break}
                      catch {::E nope} m o
                      list $m [dict get $o -errorcode] \
                           [join [lrange [split [dict get $o -errorinfo] \\n] 0 1] \\n]"
                ),
                Code::Ok
            );
            assert_eq!(
                i.result_bytes(),
                b"{unknown subcommand handler returned bad code: break} \
                  {TCL ENSEMBLE UNKNOWN_RESULT} \
                  {unknown subcommand handler returned bad code: break\n    result of ensemble unknown subcommand handler: ::u break ::E nope}"
            );
        });

        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"proc malformed {ens args} {return \\{}
                      namespace ensemble create -command ::E -unknown ::malformed
                      catch {::E nope} m o
                      list $m [dict get $o -errorcode] \
                           [join [lrange [split [dict get $o -errorinfo] \\n] 0 1] \\n]"
                ),
                Code::Ok
            );
            assert_eq!(
                i.result_bytes(),
                b"{unmatched open brace in list} {TCL VALUE LIST BRACE} \
                  {unmatched open brace in list\n    while parsing result of ensemble unknown subcommand handler}"
            );
        });

        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"proc malformed {ens args} {return \"{a}junk\"}
                      namespace ensemble create -command ::E -unknown ::malformed
                      catch {::E nope} m o
                      list $m [dict get $o -errorcode] \
                           [dict get $o -errorinfo]"
                ),
                Code::Ok
            );
            assert_eq!(
                i.result_bytes(),
                b"{list element in braces followed by \"junk\" instead of space} \
                  {TCL VALUE LIST JUNK} \
                  {list element in braces followed by \"junk\" instead of space\n    while parsing result of ensemble unknown subcommand handler\n    invoked from within\n\"::E nope\"}"
            );
        });

        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"proc baderr args {
                          return -code error -errorcode {CUSTOM CODE} BOOM
                      }
                      namespace ensemble create -command ::E -unknown ::baderr
                      catch {::E nope} m o
                      list $m [dict get $o -errorcode] [dict get $o -errorinfo]"
                ),
                Code::Ok
            );
            assert_eq!(
                i.result_bytes(),
                b"BOOM {CUSTOM CODE} {BOOM\n    while executing\n\"::baderr ::E nope\"\n    (ensemble unknown subcommand handler)\n    invoked from within\n\"::E nope\"}"
            );
        });

        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"proc delete_unknown {ens args} {rename $ens {}; return {}}
                      namespace ensemble create -command ::E -unknown ::delete_unknown
                      catch {::E nope} m o
                      list $m [dict get $o -errorcode] [dict get $o -errorinfo]"
                ),
                Code::Ok
            );
            assert_eq!(
                i.result_bytes(),
                b"{unknown subcommand handler deleted its ensemble} \
                  {TCL ENSEMBLE UNKNOWN_DELETED} \
                  {unknown subcommand handler deleted its ensemble\n    (ensemble unknown subcommand handler)\n    invoked from within\n\"::E nope\"}"
            );
        });
    }

    #[test]
    fn ensemble_default_miss_preserves_custom_unknown_options() {
        leak_free(|i| {
            let code = i.eval_str(
                b"proc ::unknown {cmd args} {
                          return -code error -errorcode {CUSTOM CODE} \
                              -errorinfo CUSTOMINFO \
                              -errorstack {INNER foo CALL bar} \
                              \"invalid command name \\\"$cmd\\\"\"
                      }
                      namespace eval N {
                          namespace ensemble create -command ::E -subcommands x
                      }
                      set c [catch {::E x} m o]
                      list $c $m [dict get $o -errorcode] \
                           [dict get $o -errorinfo] [dict get $o -errorstack]",
            );
            assert_eq!(
                code,
                Code::Ok,
                "{}",
                String::from_utf8_lossy(&i.result_bytes())
            );
            assert_eq!(
                i.result_bytes(),
                b"1 {invalid command name \"x\"} {CUSTOM CODE} \
                  {CUSTOMINFO\n    invoked from within\n\"::E x\"} \
                  {INNER foo CALL bar}"
            );
        });
    }

    /// Exact Tcl 9.0.4 lookup/read-only error taxonomy for ensemble configure
    /// and namespace origin.
    #[test]
    fn ensemble_configure_and_origin_error_codes() {
        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"proc plain {} {}
                      namespace ensemble create -command ::E
                      set out {}
                      foreach script {
                          {namespace ensemble configure ::missing}
                          {namespace ensemble configure ::plain}
                          {namespace ensemble configure ::E -namespace ::N}
                          {namespace origin ::missing}
                      } {
                          catch $script m o
                          lappend out $m [dict get $o -errorcode]
                      }
                      set out"
                ),
                Code::Ok
            );
            assert_eq!(
                i.result_bytes(),
                b"{unknown command \"::missing\"} {TCL LOOKUP COMMAND ::missing} \
                  {\"::plain\" is not an ensemble command} {TCL LOOKUP ENSEMBLE ::plain} \
                  {option -namespace is read-only} {TCL ENSEMBLE READ_ONLY} \
                  {invalid command name \"::missing\"} {TCL LOOKUP COMMAND ::missing}"
            );
        });

        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"set out {}
                      foreach script {
                          {namespace origin {not here}}
                          {namespace ensemble configure {not here}}
                      } {
                          catch $script m o
                          lappend out $m [dict get $o -errorcode] \
                              [llength [dict get $o -errorcode]]
                      }
                      namespace ensemble create -command ::Q \
                          -subcommands {{not here}}
                      catch {::Q {also not here}} m o
                      lappend out $m [dict get $o -errorcode] \
                          [llength [dict get $o -errorcode]]
                      set out"
                ),
                Code::Ok
            );
            assert_eq!(
                i.result_bytes(),
                b"{invalid command name \"not here\"} \
                  {TCL LOOKUP COMMAND {not here}} 4 \
                  {unknown command \"not here\"} \
                  {TCL LOOKUP COMMAND {not here}} 4 \
                  {unknown or ambiguous subcommand \"also not here\": must be not here} \
                  {TCL LOOKUP SUBCOMMAND {also not here}} 4"
            );
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

    // Needs the numeric tower: the linked var is bumped via `expr`.
    #[cfg(have_tommath)]
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

    // -- braced defining word (issue #1058) -----------------------------------
    //
    // #1058 read the conformance gate's `invalid command name "if"` as the
    // *braced* proc name in `proc {p} {} {…}` corrupting the command table —
    // "the builtin `if` stops resolving after a proc is defined via a braced
    // name word". It does not: the braces are word quoting, the parser strips
    // them, and `define_proc` never sees them. The error was only ever the
    // tower-gated `if` being absent from a libtommath-less build (see
    // `dispatch_matches_every_conformance_vector`, which now runs without it).
    //
    // These pin the real behaviour directly, in the tower-free command shapes
    // the hypothesis was about, so the claim stays verifiable in *every* build
    // rather than only where the numeric tower happens to be linked.

    #[test]
    fn braced_defining_word_defines_the_unbraced_name() {
        leak_free(|i| {
            // Real Tcl defines a command literally named `p`: the braces quote
            // the word, they are not part of the name.
            assert_eq!(i.eval_str(b"proc {p} {} { return {::p} }"), Code::Ok);
            assert_eq!(i.eval_str(b"::p"), Code::Ok, "absolute call must dispatch");
            assert_eq!(i.result_bytes(), b"::p");
            assert_eq!(i.eval_str(b"p"), Code::Ok, "bare call must dispatch");
            assert_eq!(i.result_bytes(), b"::p");
            assert_eq!(i.eval_str(b"namespace which -command ::p"), Code::Ok);
            assert_eq!(i.result_bytes(), b"::p");
            // ... and no command named with the braces survives anywhere.
            assert_eq!(i.eval_str(b"namespace which -command {{p}}"), Code::Ok);
            assert_eq!(i.result_bytes(), b"");
        });
    }

    #[test]
    fn braced_defining_word_leaves_builtin_dispatch_intact() {
        leak_free(|i| {
            // The #1058 hypothesis in its strongest form: after defining a proc
            // through a braced name word, *unrelated builtins* must still
            // resolve. Only tower-free builtins are exercised so this holds in
            // a libtommath-less build too — the exact configuration the issue
            // was reported from.
            assert_eq!(i.eval_str(b"proc {p} {} { return {::p} }"), Code::Ok);
            assert_eq!(i.eval_str(b"set __x 1"), Code::Ok);
            assert_eq!(i.result_bytes(), b"1");
            assert_eq!(i.eval_str(b"catch {::nosuch} __e"), Code::Ok);
            assert_eq!(i.result_bytes(), b"1");
            assert_eq!(i.eval_str(b"llength [list a b c]"), Code::Ok);
            assert_eq!(i.result_bytes(), b"3");
            assert_eq!(i.eval_str(b"string length abcd"), Code::Ok);
            assert_eq!(i.result_bytes(), b"4");
            // The global command table still holds the builtins by name.
            assert_eq!(i.eval_str(b"namespace which -command ::set"), Code::Ok);
            assert_eq!(i.result_bytes(), b"::set");
            i.eval_str(b"unset __x");
            i.eval_str(b"unset __e");
        });
    }

    #[test]
    fn braced_defining_word_binds_in_the_current_namespace() {
        leak_free(|i| {
            // The same quoting rule inside a namespace: `proc {q}` in `::ns`
            // binds `::ns::q`, resolvable bare from inside and absolutely from
            // outside, with the global table untouched.
            assert_eq!(
                i.eval_str(b"namespace eval ns { proc {q} {} { return {::ns::q} } }"),
                Code::Ok
            );
            assert_eq!(i.eval_str(b"::ns::q"), Code::Ok);
            assert_eq!(i.result_bytes(), b"::ns::q");
            assert_eq!(i.eval_str(b"namespace eval ns { q }"), Code::Ok);
            assert_eq!(i.result_bytes(), b"::ns::q");
            assert_eq!(i.eval_str(b"namespace which -command ::q"), Code::Ok);
            assert_eq!(i.result_bytes(), b"");
        });
    }

    // -- namespace inscope (issue #1058's twin of #1056/#1067) -----------------
    //
    // These avoid `if`/`while`/`for`/`expr` (and anything else `have_tommath`-
    // gated) entirely, so they run identically with or without the bignum
    // tower.

    #[test]
    fn inscope_zero_tail_args_evaluates_script_verbatim() {
        leak_free(|i| {
            i.eval_str(b"proc probe {args} { return $args }");
            // C's `objc == 3` arm: no tail, so no list is appended and no
            // concat/trim/trailing space happens — the script runs as-is.
            assert_eq!(i.eval_str(b"namespace inscope :: probe"), Code::Ok);
            assert_eq!(i.result_bytes(), b"");
        });
    }

    #[test]
    fn inscope_tail_args_become_list_elements_not_joined_words() {
        // `NamespaceInscopeCmd` (`generic/tclNamesp.c`) collects the tail
        // into a LIST and concatenates its string rep onto `script`
        // (`Tcl_ConcatObj` over `[script, list(tail)]`), so however many
        // words a tail argument holds, it reaches the invoked command as
        // exactly one argument.
        leak_free(|i| {
            i.eval_str(b"proc probe {args} { return $args }");

            // A tail word with an embedded space stays ONE argument (the
            // pre-fix bug: it split into two words, "x" and "y").
            assert_eq!(
                i.eval_str(b"llength [namespace inscope :: probe {x y}]"),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"1");
            assert_eq!(
                i.eval_str(b"lindex [namespace inscope :: probe {x y}] 0"),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"x y");

            // An empty-string tail arg round-trips as one empty argument.
            assert_eq!(
                i.eval_str(b"llength [namespace inscope :: probe {}]"),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"1");
            assert_eq!(
                i.eval_str(b"lindex [namespace inscope :: probe {}] 0"),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"");

            // Multiple tail words each keep their own arg boundary.
            assert_eq!(
                i.eval_str(b"llength [namespace inscope :: probe a {b c} d]"),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"3");

            // Special characters round-trip through the list-element
            // quoting (`list::append_list_element`): an unbalanced brace, a
            // lone backslash, and an embedded double quote. `lindex`
            // recovers the raw element value regardless of which of the
            // four renderings (none/brace/mask/escape) quoting picked.
            i.eval_str(b"set v1 \"a\\{b\"");
            assert_eq!(
                i.eval_str(b"lindex [namespace inscope :: probe $v1] 0"),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"a{b");

            i.eval_str(b"set v2 \"\\\\\"");
            assert_eq!(
                i.eval_str(b"lindex [namespace inscope :: probe $v2] 0"),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"\\");

            i.eval_str(b"set v3 {a\"b}");
            assert_eq!(
                i.eval_str(b"lindex [namespace inscope :: probe $v3] 0"),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"a\"b");

            i.eval_str(b"unset v1 v2 v3");
        });
    }

    #[test]
    fn inscope_uses_a_byte_arrays_string_rep_in_both_arms() {
        // `binary format` returns a typed byte array.  When it becomes a
        // script, both `namespace inscope` forms reach the same unknown
        // command.  Capture the error through `binary encode hex`: this is the
        // C Tcl observable, and proves that the byte array's 0x80 payload is
        // preserved rather than becoming U+FFFD or UTF-8's `c2 80` payload.
        // The vector is identical on tclsh 8.6.18 and 9.0.4.
        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"set name [binary format a3c cmd 128]; \
                      catch {namespace inscope :: $name} result; \
                      binary encode hex $result",
                ),
                Code::Ok,
                "zero-tail arm"
            );
            assert_eq!(
                i.result_bytes(),
                b"696e76616c696420636f6d6d616e64206e616d652022636d648022",
                "zero-tail arm preserves the raw command-name byte"
            );

            assert_eq!(
                i.eval_str(
                    b"catch {namespace inscope :: $name extra} result; \
                      binary encode hex $result",
                ),
                Code::Ok,
                "tail arm must agree with the zero-tail arm"
            );
            assert_eq!(
                i.result_bytes(),
                b"696e76616c696420636f6d6d616e64206e616d652022636d648022",
                "tail arm preserves the same raw command-name byte"
            );

            i.eval_str(b"unset name result");
        });
    }

    // -- issue regression vectors (#1442, #1446, #1453, #1463) -------------
    // Each expectation is pinned against tclsh 8.6.16 and 9.0.4; the
    // interpreter emulates 9.0 by default, so a release-axis vector says so.

    /// `namespace which -variable` is `Tcl_FindNamespaceVar`: namespace
    /// variable tables only, never a call frame (#1442).
    #[test]
    fn which_variable_never_answers_with_a_proc_local() {
        leak_free(|i| {
            assert_eq!(
                i.eval_str(b"proc t {} {set loc 1; namespace which -variable loc}; t"),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"");
            // A local shadowing a real namespace variable still reports the
            // namespace one.
            assert_eq!(
                i.eval_str(
                    b"namespace eval ns2 {variable shadow 5\n\
                      proc q {} {set shadow 9; namespace which -variable shadow}}\n\
                      ns2::q"
                ),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"::ns2::shadow");
        });
    }

    /// `namespace origin` follows `namespace import` links to their source
    /// through the shared `TclGetOriginalCommand` core (#1442).
    #[test]
    fn origin_follows_import_chains() {
        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"namespace eval src {namespace export p; proc p {} {return P}}\n\
                      namespace eval dst {namespace import ::src::p}\n\
                      namespace origin ::dst::p"
                ),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"::src::p");
            assert_eq!(i.eval_str(b"namespace origin set"), Code::Ok);
            assert_eq!(i.result_bytes(), b"::set");
        });
    }

    /// Only `objv[1]` is the `-clear` / `-force` flag: the registry pins
    /// `max_leading_option_words: Some(1)` and C tests that one word (#1446).
    #[test]
    fn only_the_first_word_is_the_export_or_import_flag() {
        leak_free(|i| {
            // tclsh 8.6.16 / 9.0.4: `-clear x`.
            assert_eq!(
                i.eval_str(
                    b"namespace eval e {namespace export -clear -clear x}\n\
                      namespace eval e {namespace export}"
                ),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"-clear x");
            // A trailing `-force` is an ordinary pattern, and an unqualified
            // pattern names no source namespace.
            i.eval_str(b"namespace eval s3 {namespace export q; proc q {} {return Q}}");
            assert_eq!(
                i.eval_str(b"namespace eval t3 {catch {namespace import ::s3::q -force} m; set m}"),
                Code::Ok
            );
            assert_eq!(
                i.result_bytes(),
                b"no namespace specified in import pattern \"-force\""
            );
            // …and the import that preceded it still happened.
            assert_eq!(i.eval_str(b"info commands ::t3::*"), Code::Ok);
            assert_eq!(i.result_bytes(), b"::t3::q");
            i.eval_str(b"unset -nocomplain m");
        });
    }

    /// `namespace export -clear` empties the list before the patterns that
    /// follow it are added (#1446).
    #[test]
    fn export_clear_resets_the_pattern_list() {
        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"namespace eval e {namespace export a b; namespace export -clear}\n\
                      namespace eval e {namespace export}"
                ),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"");
            assert_eq!(
                i.eval_str(
                    b"namespace eval f {namespace export a b; namespace export -clear c}\n\
                      namespace eval f {namespace export}"
                ),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"c");
        });
    }

    /// The ensemble option tables come from the shared owner: `create` has no
    /// `-namespace`, both tables abbreviate, and the `namespace ensemble`
    /// subcommand word abbreviates too (#1453).
    #[test]
    fn ensemble_option_tables_match_c() {
        leak_free(|i| {
            i.eval_str(b"namespace eval e5 {namespace export *; proc go {} {return G}}");
            assert_eq!(
                i.eval_str(b"namespace eval e5 {namespace ensemble create -comm ::ab5 -sub go}"),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"::ab5");
            assert_eq!(i.eval_str(b"namespace ensemble ex ::ab5"), Code::Ok);
            assert_eq!(i.result_bytes(), b"1");
            assert_eq!(
                i.eval_str(b"catch {namespace ensemble frobnicate} m; set m"),
                Code::Ok
            );
            assert_eq!(
                i.result_bytes(),
                b"bad subcommand \"frobnicate\": must be configure, create, or exists"
            );
            assert_eq!(
                i.eval_str(b"catch {namespace ensemble create -namespace ::x} m; set m"),
                Code::Ok
            );
            assert_eq!(
                i.result_bytes(),
                b"bad option \"-namespace\": must be -command, -map, -parameters, \
                  -prefixes, -subcommands, or -unknown"
            );
            i.eval_str(b"unset -nocomplain m");
        });
    }

    /// C's `if (objc & 1)` fires before any option word is looked at, so an
    /// odd tail is `wrong # args`, never `bad option` (#1453).
    #[test]
    fn ensemble_create_checks_pair_arity_first() {
        leak_free(|i| {
            for src in [
                &b"catch {namespace ensemble create -command} m; set m"[..],
                b"catch {namespace ensemble create -bogus} m; set m",
            ] {
                assert_eq!(i.eval_str(src), Code::Ok);
                assert_eq!(
                    i.result_bytes(),
                    b"wrong # args: should be \"namespace ensemble create ?option value ...?\""
                );
            }
            i.eval_str(b"unset -nocomplain m");
        });
    }

    /// `namespace ensemble configure` reads through the shared config table:
    /// `-namespace` is readable but never writable, `-command` is not in it,
    /// and abbreviations resolve (#1453).
    #[test]
    fn ensemble_configure_uses_the_config_table() {
        leak_free(|i| {
            i.eval_str(
                b"namespace eval e5 {namespace export *; proc go {} {return G}\n\
                  namespace ensemble create -command ::ab5 -subcommands go}",
            );
            assert_eq!(
                i.eval_str(b"namespace ensemble configure ::ab5 -sub"),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"go");
            assert_eq!(
                i.eval_str(b"catch {namespace ensemble configure ::ab5 -namespace ::e5} m; set m"),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"option -namespace is read-only");
            assert_eq!(
                i.eval_str(b"catch {namespace ensemble configure ::ab5 -command ::zz} m; set m"),
                Code::Ok
            );
            assert_eq!(
                i.result_bytes(),
                b"bad option \"-command\": must be -map, -namespace, -parameters, \
                  -prefixes, -subcommands, or -unknown"
            );
            i.eval_str(b"unset -nocomplain m");
        });
    }

    /// TclOO's root object commands are engine-installed on the registry's
    /// behalf, so they follow their introducing release; a script-created
    /// object command does not (#1463).
    #[test]
    fn tcloo_roots_follow_their_introducing_release() {
        leak_free(|i| {
            i.set_runtime_version(tcl_dialect::TclVersion::V8_4);
            assert_eq!(
                i.eval_str(b"catch {oo::class create C {}} m; set m"),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"invalid command name \"oo::class\"");
            i.set_runtime_version(tcl_dialect::TclVersion::V8_6);
            assert_eq!(
                i.eval_str(b"catch {oo::configurable create C {}} m; set m"),
                Code::Ok
            );
            assert_eq!(
                i.result_bytes(),
                b"invalid command name \"oo::configurable\""
            );
            // A user object named after a 9.0-only builtin stays callable.
            assert_eq!(
                i.eval_str(b"oo::class create lpop {method m {} {return M}}; [lpop new] m"),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"M");
            i.set_runtime_version(tcl_dialect::TclVersion::V9_0);
            i.eval_str(b"unset -nocomplain m");
        });
    }

    /// Taking a gate-hidden root's name must work through *every* registration
    /// verb, not just `create`. The root marking is an identity on the entry,
    /// so it is cleared in the one registration funnel (`ns_register`); these
    /// two vectors are the funnels that do not go through `create`.
    #[test]
    fn taking_a_hidden_root_name_works_through_copy_and_rename() {
        // `oo::copy` onto the hidden name: the copy must be callable and listed.
        leak_free(|i| {
            i.set_runtime_version(tcl_dialect::TclVersion::V8_6);
            assert_eq!(
                i.eval_str(b"oo::class create Src {method m {} {return SRC}}"),
                Code::Ok
            );
            assert_eq!(i.eval_str(b"oo::copy ::Src ::oo::configurable"), Code::Ok);
            assert_eq!(i.eval_str(b"[oo::configurable new] m"), Code::Ok);
            assert_eq!(i.result_bytes(), b"SRC");
            assert_eq!(i.eval_str(b"info commands ::oo::configurable"), Code::Ok);
            assert_eq!(i.result_bytes(), b"::oo::configurable");
            i.set_runtime_version(tcl_dialect::TclVersion::V9_0);
        });
        // `rename` onto the hidden name: same contract.
        leak_free(|i| {
            i.set_runtime_version(tcl_dialect::TclVersion::V8_6);
            assert_eq!(i.eval_str(b"oo::object create ::mysrc"), Code::Ok);
            assert_eq!(i.eval_str(b"rename ::mysrc ::oo::configurable"), Code::Ok);
            assert_eq!(i.eval_str(b"info commands ::oo::configurable"), Code::Ok);
            assert_eq!(i.result_bytes(), b"::oo::configurable");
            assert_eq!(i.eval_str(b"::oo::configurable destroy"), Code::Ok);
            i.set_runtime_version(tcl_dialect::TclVersion::V9_0);
        });
    }

    /// Taking a gate-hidden root's name via `rename` keeps working — the root
    /// marking is cleared on the rename path itself, not only by the OO
    /// re-registration that follows it.
    #[test]
    fn renaming_onto_a_hidden_root_name_keeps_the_replacement_live() {
        leak_free(|i| {
            i.set_runtime_version(tcl_dialect::TclVersion::V8_6);
            assert_eq!(
                i.eval_str(b"oo::class create ::Src {method m {} {return SRC}}"),
                Code::Ok
            );
            assert_eq!(i.eval_str(b"rename ::Src ::oo::configurable"), Code::Ok);
            assert_eq!(i.eval_str(b"info commands ::oo::configurable"), Code::Ok);
            assert_eq!(i.result_bytes(), b"::oo::configurable");
            assert_eq!(i.eval_str(b"[::oo::configurable new] m"), Code::Ok);
            assert_eq!(i.result_bytes(), b"SRC");
            i.set_runtime_version(tcl_dialect::TclVersion::V9_0);
        });
    }

    /// Configuring an ensemble through a `namespace import` alias configures
    /// the ORIGIN, so both spellings observe one config and the alias stays an
    /// alias (tclsh 9.0.4-pinned).
    #[test]
    fn configuring_an_imported_ensemble_updates_the_origin() {
        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"namespace eval S {namespace export ens\n\
                       proc impl {} {return ORIG}\n\
                       proc impl2 {} {return NEW}\n\
                       namespace ensemble create -command ::S::ens -map {go impl}}\n\
                       namespace eval T {namespace import ::S::ens}"
                ),
                Code::Ok
            );
            assert_eq!(
                i.eval_str(
                    b"namespace eval S {namespace ensemble configure ::T::ens -map {go impl2}}"
                ),
                Code::Ok
            );
            for spelling in [b"::T::ens go".as_slice(), b"::S::ens go"] {
                assert_eq!(i.eval_str(spelling), Code::Ok);
                assert_eq!(i.result_bytes(), b"NEW");
            }
            for spelling in [
                b"namespace ensemble configure ::T::ens -map".as_slice(),
                b"namespace ensemble configure ::S::ens -map",
            ] {
                assert_eq!(i.eval_str(spelling), Code::Ok);
                assert_eq!(i.result_bytes(), b"go ::S::impl2");
            }
            // Still an alias: configuring it did not fork a second ensemble.
            assert_eq!(i.eval_str(b"namespace origin ::T::ens"), Code::Ok);
            assert_eq!(i.result_bytes(), b"::S::ens");
        });
    }

    /// `namespace which -variable` is byte-preserving: a name carrying bytes
    /// that are not valid UTF-8 round-trips, and two such names stay distinct
    /// (a lossy map would collapse both onto U+FFFD).
    #[test]
    fn which_variable_preserves_byte_valued_names() {
        leak_free(|i| {
            assert_eq!(i.eval_str(b"namespace eval nb {}"), Code::Ok);
            assert_eq!(
                i.eval_str(b"namespace eval nb [list variable [binary format H* ff41] 7]"),
                Code::Ok
            );
            assert_eq!(
                i.eval_str(
                    b"binary scan [namespace eval nb [list namespace which -variable \
                       [binary format H* ff41]]] H* h; set h"
                ),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"3a3a6e623a3aff41");
            // Two distinct invalid-UTF-8 names must not collide.
            assert_eq!(
                i.eval_str(
                    b"namespace eval nb [list variable [binary format H* ff] 1]\n\
                       namespace eval nb [list variable [binary format H* fe] 2]\n\
                       namespace eval nb {list [set [binary format H* ff]] \
                       [set [binary format H* fe]]}"
                ),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"1 2");
        });
    }

    /// The same byte fidelity for the *command* half of the namespace surface:
    /// `namespace origin`, `namespace which -command`, and the TclOO
    /// object-name resolution behind them. These reach their tables through
    /// the shared core's byte-valued entry points, so an invalid-UTF-8 name is
    /// never routed through `str`. tclsh 9.0.4-pinned (`binary encode hex` of
    /// each answer).
    #[test]
    fn origin_and_which_command_preserve_byte_valued_names() {
        leak_free(|i| {
            // An imported command whose simple name is the single byte 0xFF.
            assert_eq!(
                i.eval_str(
                    b"set n [binary format H* ff]\n\
                      namespace eval src [list namespace export $n]\n\
                      namespace eval src [list proc $n {} {return P}]\n\
                      namespace eval dst [list namespace import ::src::$n]"
                ),
                Code::Ok
            );
            assert_eq!(
                i.eval_str(b"binary encode hex [namespace origin ::dst::$n]"),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"3a3a7372633a3aff");
            assert_eq!(i.eval_str(b"::dst::$n"), Code::Ok);
            assert_eq!(i.result_bytes(), b"P");
            // Two distinct byte names must stay distinct through `origin` —
            // a lossy map would collapse both onto U+FFFD and collide them.
            assert_eq!(
                i.eval_str(
                    b"set a [binary format H* ff]; set b [binary format H* fe]\n\
                      namespace eval s2 [list namespace export $a $b]\n\
                      namespace eval s2 [list proc $a {} {return A}]\n\
                      namespace eval s2 [list proc $b {} {return B}]\n\
                      list [binary encode hex [namespace origin ::s2::$a]] \
                           [binary encode hex [namespace origin ::s2::$b]]"
                ),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"3a3a73323a3aff 3a3a73323a3afe");
            // `namespace which -command` over the same name.
            assert_eq!(
                i.eval_str(b"binary encode hex [namespace which -command ::s2::$a]"),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"3a3a73323a3aff");
            i.eval_str(b"unset -nocomplain n a b");
        });
    }

    /// The byte-valued `Namespaces` spellings resolve a name the `&str` ones
    /// cannot express at all.
    ///
    /// Everything reachable from a *script* arrives here as valid UTF-8 — a
    /// byte array's string rep is one U+00XX per byte (`bytearray.rs`), so the
    /// old `from_utf8_lossy` hop was a no-op on every scripted path, which is
    /// why the surface test above passes either way. The hole it leaves is the
    /// one `binary_bytes` already documents: an embedder may hand the C ABI a
    /// plain string that is not UTF-8. This exercises that seam directly, so it
    /// fails if the byte-valued entry points are ever routed back through
    /// `str`.
    #[test]
    fn byte_valued_command_names_resolve_without_a_utf8_round_trip() {
        use tcl_runtime_api::Namespaces;

        leak_free(|i| {
            // A command name that is not valid UTF-8 in any encoding.
            let raw: &[u8] = b"::raw\xff\xfename";
            i.ns_register(
                raw,
                crate::interp::Command::Builtin(|interp, _| {
                    interp.set_result_bytes(b"RAW");
                    Code::Ok
                }),
            );
            let global = tcl_runtime_api::NsId(crate::namespace::GLOBAL as u32);
            let id = i
                .find_command_bytes(global, raw)
                .expect("a byte-valued name resolves through the byte-valued spelling");
            assert_eq!(i.command_name_bytes(id).as_deref(), Some(raw));
            // The shared cores reach the same answer.
            assert_eq!(
                tcl_cmd_core::namespace::origin_bytes(i, raw).as_deref(),
                Some(raw)
            );
            assert_eq!(
                tcl_cmd_core::namespace::which_command_bytes(i, raw).as_deref(),
                Some(raw)
            );
            // The lossy `&str` spelling genuinely cannot: the replacement
            // characters name a different (absent) command. This is the
            // divergence the byte-valued entry points exist to remove.
            assert_eq!(
                tcl_cmd_core::namespace::origin(i, &String::from_utf8_lossy(raw)),
                None
            );
        });
    }

    /// #1613: an embedder can supply a plain string whose bytes are not UTF-8.
    /// Drive the real `namespace` adapter with such objects rather than using a
    /// script-created byte array (whose string shimmer is valid UTF-8), so any
    /// lossy `&str` hop makes these two distinct namespace names disappear or
    /// collide.
    #[test]
    fn byte_valued_namespace_navigation_uses_the_embedder_bytes_verbatim() {
        fn invoke(interp: &mut Interp, words: &[&[u8]]) -> Code {
            let argv: Vec<*mut crate::obj::TclObj> = words
                .iter()
                .map(|word| crate::obj::new_string_bytes(word))
                .collect();
            let code = super::namespace_cmd(interp, &argv);
            for object in argv {
                super::drop_fresh(object);
            }
            code
        }

        leak_free(|interp| {
            let parent = b"::raw\xff";
            let sibling = b"::raw\xfe";
            let child = b"::raw\xff::child\xfd";
            {
                let mut namespaces = interp.namespaces_mut();
                namespaces.ensure_namespace(crate::namespace::GLOBAL, parent);
                namespaces.ensure_namespace(crate::namespace::GLOBAL, sibling);
                namespaces.ensure_namespace(crate::namespace::GLOBAL, child);
            }

            assert_eq!(invoke(interp, &[b"namespace", b"exists", parent]), Code::Ok);
            assert_eq!(interp.result_bytes(), b"1");
            assert_eq!(
                invoke(interp, &[b"namespace", b"exists", sibling]),
                Code::Ok
            );
            assert_eq!(interp.result_bytes(), b"1");

            assert_eq!(invoke(interp, &[b"namespace", b"parent", child]), Code::Ok);
            assert_eq!(interp.result_bytes(), parent);

            assert_eq!(
                invoke(interp, &[b"namespace", b"children", parent]),
                Code::Ok
            );
            assert_eq!(interp.result_bytes(), child);
            assert_eq!(
                invoke(interp, &[b"namespace", b"children", parent, child]),
                Code::Ok
            );
            assert_eq!(interp.result_bytes(), child);

            // Established invalid-byte glob policy: identity is defined, while
            // wildcard interpretation is reserved for valid UTF-8 strings.
            assert_eq!(
                invoke(interp, &[b"namespace", b"children", parent, b"*"]),
                Code::Ok
            );
            assert_eq!(interp.result_bytes(), b"");

            let missing = b"::missing\xff";
            assert_eq!(
                invoke(interp, &[b"namespace", b"parent", missing]),
                Code::Error
            );
            let mut message = b"namespace \"".to_vec();
            message.extend_from_slice(missing);
            message.extend_from_slice(b"\" not found");
            assert_eq!(interp.result_bytes(), message);
        });
    }

    /// `-map` is a dict: insertion order round-trips through the read-back and
    /// a repeated key keeps its first position while taking the last value
    /// (tclsh 9.0.4-pinned).
    #[test]
    fn ensemble_map_preserves_dict_order_and_collapses_repeats() {
        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"namespace eval M {namespace export *\n\
                       proc zeta {} {return Z}\n\
                       proc alpha {} {return A}\n\
                       proc mid {} {return M}\n\
                       namespace ensemble create -command ::E -map {zz zeta aa alpha}}"
                ),
                Code::Ok
            );
            // Insertion order, not sorted (`aa` would sort first).
            assert_eq!(
                i.eval_str(b"namespace ensemble configure ::E -map"),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"zz ::M::zeta aa ::M::alpha");
            // A repeated key: last value wins, first position kept.
            assert_eq!(
                i.eval_str(
                    b"namespace eval M {namespace ensemble configure ::E \
                       -map {zz zeta aa alpha zz mid}}"
                ),
                Code::Ok
            );
            assert_eq!(
                i.eval_str(b"namespace ensemble configure ::E -map"),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"zz ::M::mid aa ::M::alpha");
            // Dispatch follows the collapsed entry, not the stale first one.
            assert_eq!(i.eval_str(b"::E zz"), Code::Ok);
            assert_eq!(i.result_bytes(), b"M");
        });
    }
}
