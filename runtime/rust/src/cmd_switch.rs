//! `switch` — multi-way branch (toward running tcltest). C ref `tclCmdMZ.c`
//! (`TclNRSwitchObjCmd`).
//!
//! `switch ?options? string pattern body ?pattern body ...?` or
//! `switch ?options? string {pattern body ...}`. A `default` pattern matches
//! anything; a body of `-` falls through to the next pattern's body. The chosen
//! body is a **script** evaluated in the current scope (transparent — its code,
//! incl. `return`/`break`/`continue`, propagates). Modes: `-exact` (default),
//! `-glob`, `-regexp`; plus `-nocase`, `--`, and the TIP #75 regexp side-channel
//! options `-matchvar`/`-indexvar`.
//!
//! See `list.rs` for the module-level `not_unsafe_ptr_arg_deref` rationale.
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use crate::interp::{obj_bytes, Code, Interp};
use crate::obj::TclObj;

#[cfg(have_regex)]
use crate::frame::split_array_ref;
#[cfg(have_regex)]
use crate::interp::drop_fresh;
#[cfg(have_regex)]
use crate::list::new_list_obj;
#[cfg(have_regex)]
use crate::obj::{new_string_bytes, new_wide_int_obj};
#[cfg(have_regex)]
use crate::regex::{Regex, NO_MATCH, REG_ICASE};

/// Register `switch`.
pub fn install(interp: &mut Interp) {
    interp.register_builtin(b"switch", switch_cmd);
}

/// The matching mode (`-exact` default, `-glob`, `-regexp`).
#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Exact,
    Glob,
    Regexp,
}

/// Parsed option state shared by both pattern/body forms.
struct Opts {
    mode: Mode,
    nocase: bool,
    /// TIP #75 `-matchvar`/`-indexvar` target names (regexp mode only).
    match_var: Option<Vec<u8>>,
    index_var: Option<Vec<u8>>,
}

// Option table indices (must mirror C's `options[]` / `switchOptionsEnum`, so
// `OPT_NAMES[mode]` is the canonical name for the "option already found" error).
const OPT_EXACT: usize = 0;
const OPT_GLOB: usize = 1;
const OPT_INDEXV: usize = 2;
const OPT_MATCHV: usize = 3;
const OPT_NOCASE: usize = 4;
const OPT_REGEXP: usize = 5;
const OPT_LAST: usize = 6;
const OPT_NAMES: [&[u8]; 7] = [
    b"-exact",
    b"-glob",
    b"-indexvar",
    b"-matchvar",
    b"-nocase",
    b"-regexp",
    b"--",
];

const USAGE_INLINE: &[u8] = b"switch ?-option ...? string ?pattern body ...? ?default body?";
const USAGE_LIST: &[u8] = b"switch ?-option ...? string {?pattern body ...? ?default body?}";

fn switch_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let objc = argv.len();

    let mut mode = OPT_EXACT;
    let mut found_mode = false;
    let mut nocase = false;
    let mut match_var: Option<Vec<u8>> = None;
    let mut index_var: Option<Vec<u8>> = None;

    // Option scan. C's loop bound is `i < objc-2`, leaving the string plus at
    // least one pattern/body word unparsed; an option `--` ends the scan.
    let mut i = 1;
    while i + 2 < objc {
        let arg = obj_bytes(argv[i]);
        if arg.first() != Some(&b'-') {
            break;
        }
        let idx = match get_index(&arg) {
            Ok(x) => x,
            Err(IdxErr::Bad) => return bad_option(interp, &arg),
            Err(IdxErr::Ambiguous) => return ambiguous_option(interp, &arg),
        };
        match idx {
            OPT_LAST => {
                i += 1;
                break;
            }
            OPT_NOCASE => nocase = true,
            OPT_INDEXV | OPT_MATCHV => {
                i += 1;
                if i + 2 > objc {
                    let name: &[u8] = if idx == OPT_INDEXV {
                        b"-indexvar"
                    } else {
                        b"-matchvar"
                    };
                    return missing_var(interp, name);
                }
                if idx == OPT_INDEXV {
                    index_var = Some(obj_bytes(argv[i]));
                } else {
                    match_var = Some(obj_bytes(argv[i]));
                }
            }
            _ => {
                // A mode option (`-exact`/`-glob`/`-regexp`).
                if found_mode {
                    return double_option(interp, &arg, OPT_NAMES[mode]);
                }
                found_mode = true;
                mode = idx;
            }
        }
        i += 1;
    }

    if i + 2 > objc {
        return wrong_args(interp, USAGE_INLINE);
    }
    if index_var.is_some() && mode != OPT_REGEXP {
        return mode_restriction(interp, b"-indexvar");
    }
    if match_var.is_some() && mode != OPT_REGEXP {
        return mode_restriction(interp, b"-matchvar");
    }

    let opts = Opts {
        mode: match mode {
            OPT_GLOB => Mode::Glob,
            OPT_REGEXP => Mode::Regexp,
            _ => Mode::Exact,
        },
        nocase,
        match_var,
        index_var,
    };

    let string = obj_bytes(argv[i]);
    let rest = &argv[i + 1..];

    // A single trailing argument is the `{pattern body ...}` list form; anything
    // else is inline pattern/body words.
    if rest.len() == 1 {
        switch_list_form(interp, &opts, &string, rest[0])
    } else {
        switch_inline_form(interp, &opts, &string, rest)
    }
}

/// The inline form: `switch ?opts? str pat body ?pat body ...?`. Each body is a
/// live argument object, so a located literal runs as a `type source` frame via
/// [`Interp::eval_control_body`] (the same path as `if`/`while` bodies).
fn switch_inline_form(
    interp: &mut Interp,
    opts: &Opts,
    string: &[u8],
    words: &[*mut TclObj],
) -> Code {
    let objc = words.len();
    if objc % 2 != 0 {
        return extra_pattern(interp, false);
    }
    let npairs = objc / 2;
    // C rejects a trailing `-` body up front, citing the last *pattern*.
    if obj_bytes(words[objc - 1]).as_slice() == b"-" {
        return no_body(interp, &obj_bytes(words[objc - 2]));
    }
    let patterns: Vec<Vec<u8>> = (0..npairs).map(|p| obj_bytes(words[p * 2])).collect();
    let matched = match find_matching_pair(interp, opts, string, &patterns) {
        Ok(Some(p)) => p,
        Ok(None) => {
            interp.set_result_bytes(b"");
            return Code::Ok;
        }
        Err(()) => return Code::Error,
    };
    // Resolve a `-` fall-through to the next non-`-` body (guaranteed to exist).
    let mut b = matched;
    while obj_bytes(words[b * 2 + 1]).as_slice() == b"-" {
        b += 1;
    }
    let code = interp.eval_control_body(words[b * 2 + 1]);
    if code == Code::Error {
        arm_error_info(interp, &patterns[matched]);
    }
    code
}

/// The list form: `switch ?opts? str {pat body ...}`. The body is a sub-element
/// of the single list literal `list_obj`, so it has no `Tcl_Obj` of its own; its
/// `info frame` line is the list word's line plus the newlines preceding the
/// element (C's `TclListLines`). A `default` pattern (last) matches anything; a
/// `-` body falls through.
fn switch_list_form(
    interp: &mut Interp,
    opts: &Opts,
    string: &[u8],
    list_obj: *mut TclObj,
) -> Code {
    let list_str = obj_bytes(list_obj);
    let elems = match scan_elements(&list_str) {
        Ok(e) => e,
        Err(e) => return interp.set_error(e),
    };
    if elems.is_empty() {
        return wrong_args(interp, USAGE_LIST);
    }
    if elems.len() % 2 != 0 {
        // The infamous "comment in switch" heuristic: a pattern beginning with
        // `#` in a braced body is almost certainly a misplaced comment.
        let has_comment = (0..elems.len())
            .step_by(2)
            .any(|p| list_str.get(elems[p].start()) == Some(&b'#'));
        return extra_pattern(interp, has_comment);
    }
    let last = elems.len() - 1;
    if element_value(&list_str, &elems[last]).as_slice() == b"-" {
        let pat = element_value(&list_str, &elems[last - 1]);
        return no_body(interp, &pat);
    }
    let npairs = elems.len() / 2;
    let patterns: Vec<Vec<u8>> = (0..npairs)
        .map(|p| element_value(&list_str, &elems[p * 2]))
        .collect();
    let loc = interp.arg_location(list_obj);
    let matched = match find_matching_pair(interp, opts, string, &patterns) {
        Ok(Some(p)) => p,
        Ok(None) => {
            interp.set_result_bytes(b"");
            return Code::Ok;
        }
        Err(()) => return Code::Error,
    };
    let mut b = matched;
    while element_value(&list_str, &elems[b * 2 + 1]).as_slice() == b"-" {
        b += 1;
    }
    let body_elem = &elems[b * 2 + 1];
    let body = element_value(&list_str, body_elem);
    // Source-track only a literal body in a located list (a body with backslash
    // collapse, or a dynamic list, reverts to body-relative — C sets such lines
    // to -1).
    let code = match (&loc, body_elem.literal) {
        (Some((file, bline)), true) => {
            let line = bline + count_newlines(&list_str[..body_elem.start()]);
            interp.eval_located_body(file.clone(), line, &body)
        }
        _ => interp.eval_unlocated_body(&body),
    };
    if code == Code::Error {
        arm_error_info(interp, &patterns[matched]);
    }
    code
}

/// Find the pattern pair (index into `patterns`) that matches `string`, writing
/// any TIP #75 `-matchvar`/`-indexvar` side-channel data as a consequence.
/// `Ok(Some(p))` on a match, `Ok(None)` if nothing matched, `Err(())` if an
/// error was set (a bad regexp pattern, or an unwritable result variable).
fn find_matching_pair(
    interp: &mut Interp,
    opts: &Opts,
    string: &[u8],
    patterns: &[Vec<u8>],
) -> Result<Option<usize>, ()> {
    let npairs = patterns.len();
    for (p, pat) in patterns.iter().enumerate() {
        // `default` matches anything, but only as the final pattern.
        if p == npairs - 1 && pat.as_slice() == b"default" {
            #[cfg(have_regex)]
            write_default_vars(interp, opts)?;
            return Ok(Some(p));
        }
        match opts.mode {
            Mode::Exact | Mode::Glob => {
                if matches(opts.mode, opts.nocase, pat, string) {
                    return Ok(Some(p));
                }
            }
            Mode::Regexp => {
                #[cfg(have_regex)]
                {
                    let (cps, byteoff) = crate::regex::decode_utf8(string);
                    let mut re = match compile_regex(interp, pat, opts.nocase) {
                        Some(r) => r,
                        None => return Err(()),
                    };
                    if let Some(m) = re.exec(&cps, 0, false) {
                        let nsubs = re.nsub();
                        write_regexp_vars(interp, opts, &m, nsubs, string, &byteoff)?;
                        return Ok(Some(p));
                    }
                }
                #[cfg(not(have_regex))]
                {
                    let _ = (pat, string);
                    interp.set_error(b"switch -regexp is not yet supported");
                    return Err(());
                }
            }
        }
    }
    Ok(None)
}

/// Append the `("PATTERN" arm line N)` errorInfo frame (C's `SwitchPostProc`),
/// then clear the logged flag so the enclosing eval logs the `switch` command's
/// own `invoked from within` frame. `PATTERN` is the matched pattern, truncated
/// to 50 bytes with a trailing `...` (C's `limit`).
fn arm_error_info(interp: &mut Interp, pattern: &[u8]) {
    let overflow = pattern.len() > 50;
    let mut inner = Vec::with_capacity(pattern.len().min(50) + 8);
    inner.push(b'"');
    inner.extend_from_slice(if overflow { &pattern[..50] } else { pattern });
    if overflow {
        inner.extend_from_slice(b"...");
    }
    inner.extend_from_slice(b"\" arm");
    interp.append_frame_line(&inner);
    interp.clear_error_logged();
}

// -- TIP #75 regexp side-channel (-matchvar / -indexvar) --------------------

/// Write empty values to the `-matchvar`/`-indexvar` targets when the default
/// arm is reached in regexp mode (TIP #75: the variables become empty lists).
#[cfg(have_regex)]
fn write_default_vars(interp: &mut Interp, opts: &Opts) -> Result<(), ()> {
    if let Some(name) = opts.index_var.clone() {
        write_var(interp, &name, new_string_bytes(b""))?;
    }
    if let Some(name) = opts.match_var.clone() {
        write_var(interp, &name, new_string_bytes(b""))?;
    }
    Ok(())
}

/// Build and write the `-indexvar` (`{start end}` pairs) and `-matchvar`
/// (substring list) values for a regexp match. Mirrors `matchFoundRegexp`: the
/// index variable is written first, then the match variable.
#[cfg(have_regex)]
fn write_regexp_vars(
    interp: &mut Interp,
    opts: &Opts,
    matches: &[crate::regex::RegMatch],
    nsubs: usize,
    string: &[u8],
    byteoff: &[usize],
) -> Result<(), ()> {
    if let Some(name) = opts.index_var.clone() {
        let pairs: Vec<*mut TclObj> = (0..=nsubs)
            .map(|j| {
                let (a, b) = match matches.get(j) {
                    // C's `info.matches[j].end > 0`: a non-participating or
                    // start-anchored empty group yields `{-1 -1}`.
                    Some(m) if m.so != NO_MATCH && m.eo > 0 => (m.so as i64, m.eo as i64 - 1),
                    _ => (-1, -1),
                };
                new_list_obj(&[new_wide_int_obj(a), new_wide_int_obj(b)])
            })
            .collect();
        let lst = new_list_obj(&pairs);
        write_var(interp, &name, lst)?;
    }
    if let Some(name) = opts.match_var.clone() {
        let subs: Vec<*mut TclObj> = (0..=nsubs)
            .map(|j| match matches.get(j) {
                Some(m) if m.so != NO_MATCH && m.eo > 0 => {
                    new_string_bytes(&string[byteoff[m.so]..byteoff[m.eo]])
                }
                _ => new_string_bytes(b""),
            })
            .collect();
        let lst = new_list_obj(&subs);
        write_var(interp, &name, lst)?;
    }
    Ok(())
}

/// Set the variable named by the (possibly `arr(idx)`) `name` to `obj`,
/// producing the C `can't set "name": ...` message on failure (and freeing the
/// unstored `obj`).
#[cfg(have_regex)]
fn write_var(interp: &mut Interp, name: &[u8], obj: *mut TclObj) -> Result<(), ()> {
    let (base, elem) = split_array_ref(name);
    let stored = match &elem {
        Some(key) => interp.var_set_elem(&base, key, obj),
        None => interp.var_set(&base, obj),
    };
    match stored {
        Ok(()) => Ok(()),
        Err(e) => {
            drop_fresh(obj);
            crate::builtins::var_error(interp, name, e);
            Err(())
        }
    }
}

/// Compile a switch `-regexp` pattern (Tcl ARE), mapping a malformed pattern to
/// the Tcl error result.
#[cfg(have_regex)]
fn compile_regex(interp: &mut Interp, pattern: &[u8], nocase: bool) -> Option<Regex> {
    let cflags = if nocase { REG_ICASE } else { 0 };
    match Regex::compile(pattern, cflags) {
        Ok(re) => Some(re),
        Err(detail) => {
            let mut m = b"cannot compile regular expression pattern: ".to_vec();
            m.extend_from_slice(&detail);
            interp.set_error(&m);
            None
        }
    }
}

// -- list-element scanning (located bodies) ---------------------------------

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

// -- pattern matching (exact / glob) ----------------------------------------

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
        // Regexp matches are driven separately (it needs the interp + engine).
        Mode::Regexp => false,
    }
}

// -- option parsing helpers -------------------------------------------------

enum IdxErr {
    Bad,
    Ambiguous,
}

/// Resolve an option word against `OPT_NAMES` with Tcl's unambiguous-prefix
/// rule (an exact match always wins; otherwise a unique prefix).
fn get_index(arg: &[u8]) -> Result<usize, IdxErr> {
    let mut found = None;
    let mut count = 0;
    for (k, name) in OPT_NAMES.iter().enumerate() {
        if *name == arg {
            return Ok(k);
        }
        if name.starts_with(arg) {
            found = Some(k);
            count += 1;
        }
    }
    match (count, found) {
        (1, Some(k)) => Ok(k),
        (0, _) => Err(IdxErr::Bad),
        _ => Err(IdxErr::Ambiguous),
    }
}

const OPTION_LIST: &[u8] = b"-exact, -glob, -indexvar, -matchvar, -nocase, -regexp, or --";

fn bad_option(interp: &mut Interp, arg: &[u8]) -> Code {
    let mut m = b"bad option \"".to_vec();
    m.extend_from_slice(arg);
    m.extend_from_slice(b"\": must be ");
    m.extend_from_slice(OPTION_LIST);
    interp.set_error(&m)
}

fn ambiguous_option(interp: &mut Interp, arg: &[u8]) -> Code {
    let mut m = b"ambiguous option \"".to_vec();
    m.extend_from_slice(arg);
    m.extend_from_slice(b"\": must be ");
    m.extend_from_slice(OPTION_LIST);
    interp.set_error(&m)
}

fn double_option(interp: &mut Interp, arg: &[u8], found_name: &[u8]) -> Code {
    let mut m = b"bad option \"".to_vec();
    m.extend_from_slice(arg);
    m.extend_from_slice(b"\": ");
    m.extend_from_slice(found_name);
    m.extend_from_slice(b" option already found");
    interp.set_error(&m)
}

fn missing_var(interp: &mut Interp, opt: &[u8]) -> Code {
    let mut m = b"missing variable name argument to ".to_vec();
    m.extend_from_slice(opt);
    m.extend_from_slice(b" option");
    interp.set_error(&m)
}

fn mode_restriction(interp: &mut Interp, opt: &[u8]) -> Code {
    let mut m = opt.to_vec();
    m.extend_from_slice(b" option requires -regexp option");
    interp.set_error(&m)
}

fn extra_pattern(interp: &mut Interp, comment_hint: bool) -> Code {
    let mut m = b"extra switch pattern with no body".to_vec();
    if comment_hint {
        m.extend_from_slice(
            b", this may be due to a comment incorrectly placed outside of a \
              switch body - see the \"switch\" documentation",
        );
    }
    interp.set_error(&m)
}

fn no_body(interp: &mut Interp, pattern: &[u8]) -> Code {
    let mut m = b"no body specified for pattern \"".to_vec();
    m.extend_from_slice(pattern);
    m.push(b'"');
    interp.set_error(&m)
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

    fn err(i: &mut Interp, src: &[u8]) -> Vec<u8> {
        assert_eq!(
            i.eval_str(src),
            Code::Error,
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

    #[test]
    fn switch_option_prefixes_and_double_mode() {
        leak_free(|i| {
            // Unambiguous option prefixes.
            assert_eq!(run(i, b"switch -exa Foo Foo {set result OK}"), b"OK");
            assert_eq!(run(i, b"switch -gl Foo Fo? {set result OK}"), b"OK");
            i.eval_str(b"unset result");
            // Two mode options conflict.
            assert_eq!(
                err(i, b"switch -exact -glob Foo Foo {x}"),
                b"bad option \"-glob\": -exact option already found"
            );
            // Unknown option.
            assert_eq!(
                err(i, b"switch -foo a b c"),
                b"bad option \"-foo\": must be -exact, -glob, -indexvar, -matchvar, -nocase, -regexp, or --"
            );
        });
    }

    #[test]
    fn switch_arg_errors() {
        leak_free(|i| {
            assert_eq!(
                err(i, b"switch"),
                b"wrong # args: should be \"switch ?-option ...? string ?pattern body ...? ?default body?\""
            );
            assert_eq!(
                err(i, b"switch x {}"),
                b"wrong # args: should be \"switch ?-option ...? string {?pattern body ...? ?default body?}\""
            );
            assert_eq!(err(i, b"switch a b"), b"extra switch pattern with no body");
            // Trailing `-` body cites the last pattern.
            assert_eq!(
                err(i, b"switch a {a - b - c -}"),
                b"no body specified for pattern \"c\""
            );
        });
    }

    #[cfg(have_regex)]
    #[test]
    fn switch_regexp_and_matchvars() {
        leak_free(|i| {
            assert_eq!(
                run(
                    i,
                    b"switch -regexp aaaab {^a*b$ {subst regexp} aaaab {subst exact} default {subst none}}"
                ),
                b"regexp"
            );
            // -matchvar captures the whole match and submatches.
            assert_eq!(
                run(i, b"switch -regexp -matchvar x -- abc {.(.). {set x}}"),
                b"abc b"
            );
            // -indexvar reports {start end} pairs.
            assert_eq!(
                run(i, b"switch -regexp -indexvar x -- abc {.(.). {set x}}"),
                b"{0 2} {1 1}"
            );
            // A non-participating group is {-1 -1}.
            assert_eq!(
                run(
                    i,
                    b"switch -regexp -indexvar x -- abcdef {^...(x)? {set x}}"
                ),
                b"{0 2} {-1 -1}"
            );
            // -matchvar without -regexp is rejected.
            assert_eq!(
                err(i, b"switch -glob -matchvar x -- abc . {set x}"),
                b"-matchvar option requires -regexp option"
            );
            // A bad pattern reports the compile error.
            assert_eq!(
                err(
                    i,
                    b"switch -regexp aaaab {*b {subst glob} default {subst none}}"
                ),
                b"cannot compile regular expression pattern: invalid quantifier operand"
            );
            i.eval_str(b"unset -nocomplain x");
        });
    }
}
