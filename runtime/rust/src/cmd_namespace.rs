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

fn namespace_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 {
        return wrong_args(interp, b"namespace subcommand ?arg ...?");
    }
    match obj_bytes(argv[1]).as_slice() {
        b"current" => ns_current(interp, argv),
        b"eval" => ns_eval(interp, argv),
        b"exists" => ns_exists(interp, argv),
        b"parent" => ns_parent(interp, argv),
        b"children" => ns_children(interp, argv),
        b"qualifiers" => ns_qualifiers(interp, argv),
        b"tail" => ns_tail(interp, argv),
        b"which" => ns_which(interp, argv),
        b"export" => ns_export(interp, argv),
        b"import" => ns_import(interp, argv),
        b"forget" => ns_forget(interp, argv),
        b"path" => ns_path(interp, argv),
        other => {
            let mut m = b"unknown or ambiguous subcommand \"".to_vec();
            m.extend_from_slice(other);
            m.extend_from_slice(b"\": must be children, current, eval, exists, export, forget, import, parent, path, qualifiers, tail, or which");
            interp.set_error(&m)
        }
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
    for &a in &argv[2..] {
        let b = obj_bytes(a);
        match b.as_slice() {
            b"-command" => {}
            b"-variable" => want_variable = true,
            _ => {
                if name.is_some() {
                    return wrong_args(interp, b"namespace which ?-command? ?-variable? name");
                }
                name = Some(b);
            }
        }
    }
    let Some(name) = name else {
        return wrong_args(interp, b"namespace which ?-command? ?-variable? name");
    };
    if want_variable {
        interp.set_result_bytes(b""); // ns variables not modelled yet
        return Code::Ok;
    }
    let cur = interp.current_ns();
    let fqn = interp.namespaces().which_command(cur, &name);
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
            // Reject clobbering an existing command unless -force.
            if !force && interp.namespaces().which_command(dest, &simple).is_some() {
                // Only a conflict if it would resolve in the dest ns itself.
                if interp
                    .namespaces()
                    .imported_in(dest)
                    .iter()
                    .any(|(k, _)| k == &simple)
                    || dest_has_own(interp, dest, &simple)
                {
                    let mut m = b"can't import command \"".to_vec();
                    m.extend_from_slice(&simple);
                    m.extend_from_slice(b"\": already exists");
                    return interp.set_error(&m);
                }
            }
            let mut source = src_fqn.clone();
            source.extend_from_slice(b"::");
            source.extend_from_slice(&simple);
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
            match interp.namespaces().find_namespace(current, &name) {
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
    fn which_resolves_command_fqn() {
        leak_free(|i| {
            assert_eq!(i.eval_str(b"namespace which -command set"), Code::Ok);
            assert_eq!(i.result_bytes(), b"::set");
            assert_eq!(i.eval_str(b"namespace which nope"), Code::Ok);
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
}
