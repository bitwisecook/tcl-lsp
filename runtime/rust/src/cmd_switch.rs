//! `switch` — multi-way branch (toward running tcltest). C ref `tclCmdMZ.c`
//! (`Tcl_SwitchObjCmd`).
//!
//! `switch ?options? string pattern body ?pattern body ...?` or
//! `switch ?options? string {pattern body ...}`. A `default` pattern matches
//! anything; a body of `-` falls through to the next pattern's body. The chosen
//! body is a **script** evaluated in the current scope (transparent — its code,
//! incl. `return`/`break`/`continue`, propagates). Modes: `-exact` (default),
//! `-glob`, `-nocase`, `--`. (`-regexp` lands with the regex engine.)
//!
//! See `list.rs` for the module-level `not_unsafe_ptr_arg_deref` rationale.
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use crate::interp::{obj_bytes, Code, Interp};
use crate::obj::TclObj;

/// Register `switch`.
pub fn install(interp: &mut Interp) {
    interp.register_builtin(b"switch", switch_cmd);
}

#[derive(Clone, Copy)]
enum Mode {
    Exact,
    Glob,
}

fn switch_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let usage = b"switch ?-exact|-glob|-nocase|--? string {?pattern body ...?}";
    // Parse leading options.
    let mut mode = Mode::Exact;
    let mut nocase = false;
    let mut i = 1;
    while i < argv.len() {
        let a = obj_bytes(argv[i]);
        match a.as_slice() {
            b"-exact" => mode = Mode::Exact,
            b"-glob" => mode = Mode::Glob,
            b"-nocase" => nocase = true,
            b"-regexp" => return interp.set_error(b"switch -regexp is not yet supported"),
            b"--" => {
                i += 1;
                break;
            }
            opt if opt.starts_with(b"-") => {
                let mut m = b"bad option \"".to_vec();
                m.extend_from_slice(opt);
                m.extend_from_slice(b"\": must be -exact, -glob, -nocase, or --");
                return interp.set_error(&m);
            }
            _ => break,
        }
        i += 1;
    }
    if i >= argv.len() {
        return wrong_args(interp, usage);
    }
    let string = obj_bytes(argv[i]);
    i += 1;

    // Two forms: a single `{pattern body ...}` list argument, or the rest as
    // inline pattern/body words. The inline form keeps each body's `Tcl_Obj`
    // (whose TIP 280 location the LABC table already carries); the list form
    // re-splits the literal, so each body's source line must be derived from its
    // offset within the list word (C's `TclListLines`).
    if i + 1 == argv.len() {
        return switch_list_form(interp, mode, nocase, &string, argv[i]);
    }
    switch_inline_form(interp, mode, nocase, &string, &argv[i..])
}

/// The inline form: `switch ?opts? str pat body ?pat body ...?`. Each body is a
/// live argument object, so a located literal runs as a `type source` frame via
/// [`Interp::eval_control_body`] (the same path as `if`/`while` bodies).
fn switch_inline_form(
    interp: &mut Interp,
    mode: Mode,
    nocase: bool,
    string: &[u8],
    pairs: &[*mut TclObj],
) -> Code {
    if pairs.is_empty() || pairs.len() % 2 != 0 {
        return interp.set_error(b"extra switch pattern with no body");
    }
    let npairs = pairs.len() / 2;
    for p in 0..npairs {
        let pat = obj_bytes(pairs[p * 2]);
        let is_default = pat.as_slice() == b"default" && p == npairs - 1;
        if is_default || matches(mode, nocase, &pat, string) {
            // Resolve a `-` fall-through to the next non-`-` body.
            let mut b = p;
            while obj_bytes(pairs[b * 2 + 1]).as_slice() == b"-" {
                b += 1;
                if b >= npairs {
                    return interp.set_error(b"no body specified for pattern \"-\"");
                }
            }
            return interp.eval_control_body(pairs[b * 2 + 1]);
        }
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

/// The list form: `switch ?opts? str {pat body ...}`. The body is a sub-element
/// of the single list literal `list_obj`, so it has no `Tcl_Obj` of its own; its
/// `info frame` line is the list word's line plus the newlines preceding the
/// element (C's `TclListLines`). A `default` pattern (last) matches anything; a
/// `-` body falls through.
fn switch_list_form(
    interp: &mut Interp,
    mode: Mode,
    nocase: bool,
    string: &[u8],
    list_obj: *mut TclObj,
) -> Code {
    let list_str = obj_bytes(list_obj);
    // Scan the list once for each element's value range + `literal` flag (and so
    // its byte offset), the location-tracking complement to `split_list`.
    let elems = match scan_elements(&list_str) {
        Ok(e) => e,
        Err(e) => return interp.set_error(e),
    };
    if elems.is_empty() || elems.len() % 2 != 0 {
        return interp.set_error(b"extra switch pattern with no body");
    }
    // The list word's file + first line (TIP 280); absent for a dynamic value,
    // in which case bodies are body-relative (`type proc`, no file).
    let loc = interp.arg_location(list_obj);
    let npairs = elems.len() / 2;
    for p in 0..npairs {
        let pat = element_value(&list_str, &elems[p * 2]);
        let is_default = pat.as_slice() == b"default" && p == npairs - 1;
        if is_default || matches(mode, nocase, &pat, string) {
            let mut b = p;
            while element_value(&list_str, &elems[b * 2 + 1]).as_slice() == b"-" {
                b += 1;
                if b >= npairs {
                    return interp.set_error(b"no body specified for pattern \"-\"");
                }
            }
            let body_elem = &elems[b * 2 + 1];
            let body = element_value(&list_str, body_elem);
            // Source-track only a literal body in a located list (a body with
            // backslash collapse, or a dynamic list, reverts to body-relative —
            // C sets such lines to -1).
            match (&loc, body_elem.literal) {
                (Some((file, bline)), true) => {
                    let line = bline + count_newlines(&list_str[..body_elem.start()]);
                    return interp.eval_located_body(file.clone(), line, &body);
                }
                _ => return interp.eval_unlocated_body(&body),
            }
        }
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

/// A located list element: its interior byte range and whether it is `literal`
/// (verbatim, no backslash collapse). `value.start` doubles as the line-tracking
/// anchor — an element's opening brace/quote shares a line with its interior, so
/// counting newlines to the interior start matches C's `element` anchor.
struct Elem {
    value: core::ops::Range<usize>,
    literal: bool,
}

impl Elem {
    fn start(&self) -> usize {
        self.value.start
    }
}

/// Scan `src` into its located list elements (the offset-aware complement to
/// `split_list`, sharing `tcl_syntax`'s element scanner).
fn scan_elements(src: &[u8]) -> Result<Vec<Elem>, &'static [u8]> {
    let s = core::str::from_utf8(src).map_err(|_| b"unmatched open brace in list".as_slice())?;
    let mut elems = Vec::new();
    let mut pos = 0;
    loop {
        match tcl_syntax::list::find_element(s, pos) {
            Ok(Some(e)) => {
                pos = e.next;
                elems.push(Elem {
                    value: e.value,
                    literal: e.literal,
                });
            }
            Ok(None) => break,
            Err(err) => return Err(err.message().as_bytes()),
        }
    }
    Ok(elems)
}

/// The element's value bytes: verbatim for a literal (`{braced}`) element, else
/// backslash-collapsed (matching `split_list`).
fn element_value(src: &[u8], e: &Elem) -> Vec<u8> {
    if e.literal {
        src[e.value.clone()].to_vec()
    } else {
        tcl_syntax::backslash::decode_bytes(&src[e.value.clone()]).into_owned()
    }
}

/// Count the newlines in `s` (line delta between two offsets).
fn count_newlines(s: &[u8]) -> u32 {
    s.iter().filter(|&&b| b == b'\n').count() as u32
}

fn matches(mode: Mode, nocase: bool, pat: &[u8], string: &[u8]) -> bool {
    match mode {
        Mode::Exact => {
            if nocase {
                pat.eq_ignore_ascii_case(string)
            } else {
                pat == string
            }
        }
        Mode::Glob => match (core::str::from_utf8(pat), core::str::from_utf8(string)) {
            (Ok(p), Ok(s)) => tcl_syntax::glob::string_case_match(p, s, nocase),
            _ => false,
        },
    }
}

fn wrong_args(interp: &mut Interp, usage: &[u8]) -> Code {
    let mut m = b"wrong # args: should be \"".to_vec();
    m.extend_from_slice(usage);
    m.push(b'"');
    interp.set_error(&m)
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

    fn run(i: &mut Interp, src: &[u8]) -> Vec<u8> {
        assert_eq!(
            i.eval_str(src),
            Code::Ok,
            "eval {:?}",
            String::from_utf8_lossy(src)
        );
        i.result_bytes()
    }

    #[test]
    fn switch_exact_glob_default_fallthrough() {
        leak_free(|i| {
            // list form, exact.
            assert_eq!(
                run(i, b"switch b {a {set r A} b {set r B} c {set r C}}"),
                b"B"
            );
            // default (last) matches anything.
            assert_eq!(
                run(i, b"switch -- z {a {set r 1} default {set r def}}"),
                b"def"
            );
            // `-` falls through to the next body.
            assert_eq!(
                run(i, b"switch x {a {set r 1} x - y {set r both} z {set r 3}}"),
                b"both"
            );
            // glob mode.
            assert_eq!(
                run(
                    i,
                    b"switch -glob foobar {f* {set r glob} default {set r no}}"
                ),
                b"glob"
            );
            // nocase exact.
            assert_eq!(
                run(i, b"switch -nocase ABC {abc {set r m} default {set r no}}"),
                b"m"
            );
            // inline pattern/body form.
            assert_eq!(run(i, b"switch 2 1 {set r one} 2 {set r two}"), b"two");
            // no match → empty.
            assert_eq!(run(i, b"switch q {a {set r 1} b {set r 2}}"), b"");
            i.eval_str(b"unset r");
        });
    }

    #[test]
    fn switch_propagates_body_code() {
        leak_free(|i| {
            assert_eq!(
                i.eval_str(b"switch a {a {break} b {continue}}"),
                Code::Break
            );
        });
    }
}
