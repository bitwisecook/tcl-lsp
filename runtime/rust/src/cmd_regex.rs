//! `regexp` / `regsub` — Tcl ARE matching/substitution, on the real Tcl 9
//! Henry-Spencer engine (linked via `build.rs`, gated `have_regex`).
//!
//! Semantics mirror `tmp/tcl9.0.3/generic/tclCmdMZ.c` (`Tcl_RegexpObjCmd` /
//! `Tcl_RegsubObjCmd`); the engine driving (UTF-8 → codepoints, offset
//! advancing, `REG_NOTBOL`) lives in [`crate::regex`]. The Zig runtime
//! (`runtime/zig/valtypes/tcl_regex.zig`) is the behavioural oracle. This is
//! the M3 regex wall in `docs/design/runtime/tcltest-bringup.md`.
//!
//! See `list.rs` for the module-level `not_unsafe_ptr_arg_deref` rationale.
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use crate::interp::{drop_fresh, obj_bytes, Code, Interp};
use crate::obj::{new_string_bytes, new_wide_int_obj, TclObj};
use crate::regex::{
    decode_utf8, Regex, NO_MATCH, REG_EXPANDED, REG_ICASE, REG_NEWLINE, REG_NLANCH, REG_NLSTOP,
};

/// Register `regexp` and `regsub`.
pub fn install(interp: &mut Interp) {
    interp.register_builtin(b"regexp", regexp_cmd);
    interp.register_builtin(b"regsub", regsub_cmd);
}

fn wrong_args(interp: &mut Interp, usage: &[u8]) -> Code {
    let mut m = b"wrong # args: should be \"".to_vec();
    m.extend_from_slice(usage);
    m.push(b'"');
    interp.set_error(&m)
}

/// The compile flags shared by both commands' option sets (`-nocase`,
/// `-expanded`, `-line`/`-linestop`/`-lineanchor`).
#[derive(Default)]
struct Common {
    all: bool,
    cflags: i32,
    start: Option<Vec<u8>>,
}

/// Try to consume one shared option `name` (already known to start with `-`).
/// Returns `Some(Ok(consumed_extra))` where `consumed_extra` is whether it ate
/// the following argument (`-start`), `Some(Err)` on a malformed `-start`, or
/// `None` if `name` isn't one of the shared options.
fn shared_option(c: &mut Common, name: &[u8], next: Option<&[u8]>) -> Option<Result<bool, ()>> {
    match name {
        b"-all" => {
            c.all = true;
            Some(Ok(false))
        }
        b"-nocase" => {
            c.cflags |= REG_ICASE;
            Some(Ok(false))
        }
        b"-expanded" => {
            c.cflags |= REG_EXPANDED;
            Some(Ok(false))
        }
        b"-line" => {
            c.cflags |= REG_NEWLINE;
            Some(Ok(false))
        }
        b"-linestop" => {
            c.cflags |= REG_NLSTOP;
            Some(Ok(false))
        }
        b"-lineanchor" => {
            c.cflags |= REG_NLANCH;
            Some(Ok(false))
        }
        b"-start" => match next {
            Some(v) => {
                c.start = Some(v.to_vec());
                Some(Ok(true))
            }
            None => Some(Err(())),
        },
        _ => None,
    }
}

/// Resolve a `-start` index spec (integer / `end` / `end±N`) against the
/// character length, clamped to `0` (Tcl resets negatives to the start).
fn resolve_start(spec: &[u8], char_len: usize) -> usize {
    let len = char_len as isize;
    let idx = if spec == b"end" {
        len - 1
    } else if let Some(rest) = spec.strip_prefix(b"end") {
        match rest.first() {
            Some(b'-') => parse_isize(&rest[1..]).map_or(0, |n| len - 1 - n),
            Some(b'+') => parse_isize(&rest[1..]).map_or(0, |n| len - 1 + n),
            _ => 0,
        }
    } else {
        parse_isize(spec).unwrap_or(0)
    };
    if idx < 0 {
        0
    } else {
        idx as usize
    }
}

fn parse_isize(b: &[u8]) -> Option<isize> {
    core::str::from_utf8(b).ok()?.trim().parse::<isize>().ok()
}

/// Compile, mapping a bad pattern to the Tcl error result.
fn compile(interp: &mut Interp, pattern: &[u8], cflags: i32) -> Option<Regex> {
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

/// Tcl's per-iteration `eflags` rule: `REG_NOTBOL` unless `offset` is the very
/// start or follows a newline (so `^` behaves correctly in `-line` mode and at
/// resumed offsets).
fn notbol_at(cps: &[i32], offset: usize) -> bool {
    if offset == 0 {
        false
    } else if offset > cps.len() {
        true
    } else {
        cps[offset - 1] != ('\n' as i32)
    }
}

// -- regexp ----------------------------------------------------------------

fn regexp_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    const USAGE: &[u8] = b"regexp ?-option ...? exp string ?matchVar? ?subMatchVar ...?";
    let mut c = Common::default();
    let mut indices = false;
    let mut inline = false;
    let mut about = false;

    // Option scan: stops at the first non-`-` word or after `--`.
    let mut i = 1;
    while i < argv.len() {
        let name = obj_bytes(argv[i]);
        if name.first() != Some(&b'-') {
            break;
        }
        if name == b"--" {
            i += 1;
            break;
        }
        let next = argv.get(i + 1).map(|&o| obj_bytes(o));
        match shared_option(&mut c, &name, next.as_deref()) {
            Some(Ok(ate)) => {
                i += 1 + usize::from(ate);
                continue;
            }
            Some(Err(())) => {
                // `-start` with no argument: Tcl falls through and treats it as
                // end-of-options.
                i += 1;
                break;
            }
            None => {}
        }
        match name.as_slice() {
            b"-indices" => indices = true,
            b"-inline" => inline = true,
            b"-about" => about = true,
            _ => return bad_option(interp, &name),
        }
        i += 1;
    }

    if about {
        return interp.set_error(b"regexp -about is not yet supported");
    }
    let rest = &argv[i..];
    if rest.len() < 2 {
        return wrong_args(interp, USAGE);
    }
    if inline && rest.len() > 2 {
        interp.set_error(b"regexp match variables not allowed when using -inline");
        return Code::Error;
    }

    let pattern = obj_bytes(rest[0]);
    let (cps, byteoff) = decode_utf8(&obj_bytes(rest[1]));
    let str_bytes = obj_bytes(rest[1]);
    let char_len = cps.len();
    let match_vars: Vec<Vec<u8>> = rest[2..].iter().map(|&o| obj_bytes(o)).collect();

    let mut re = match compile(interp, &pattern, c.cflags) {
        Some(re) => re,
        None => return Code::Error,
    };
    let nsubs = re.nsub();

    let mut offset = match &c.start {
        Some(spec) => resolve_start(spec, char_len),
        None => 0,
    };

    // Tcl's `all` doubles as flag + counter: starts 1 if `-all`, else 0.
    let mut all_count: i64 = if c.all { 1 } else { 0 };
    let mut inline_items: Vec<*mut TclObj> = Vec::new();

    loop {
        let notbol = notbol_at(&cps, offset);
        let matches = match re.exec(&cps, offset, notbol) {
            Some(m) => m,
            None => {
                if all_count <= 1 {
                    // First time through with no match.
                    if inline {
                        interp.set_result(crate::list::new_list_obj(&[]));
                    } else {
                        set_int(interp, 0);
                    }
                    return Code::Ok;
                }
                break;
            }
        };

        let nitems = if inline { nsubs + 1 } else { match_vars.len() };
        for (k, item) in (0..nitems)
            .map(|k| build_match_item(&matches, k, nsubs, indices, &str_bytes, &byteoff))
            .enumerate()
        {
            if inline {
                inline_items.push(item);
            } else if interp.var_set(&match_vars[k], item).is_err() {
                drop_fresh(item);
                return interp.set_error(b"couldn't set match variable");
            }
        }

        if !c.all {
            break;
        }
        let m0 = matches[0];
        offset = m0.eo;
        if m0.eo == m0.so {
            offset += 1; // zero-length match: always advance to avoid looping
        }
        all_count += 1;
        if offset >= char_len {
            break;
        }
    }

    if inline {
        interp.set_result(crate::list::new_list_obj(&inline_items));
    } else {
        set_int(interp, if all_count > 0 { all_count - 1 } else { 1 });
    }
    Code::Ok
}

/// Build the value for submatch `k`: an `{start end}` index pair (`-indices`)
/// or the matched substring (default). A non-participating or empty group
/// yields `{-1 -1}` / the empty string, per `Tcl_RegexpObjCmd`.
fn build_match_item(
    matches: &[crate::regex::RegMatch],
    k: usize,
    nsubs: usize,
    indices: bool,
    str_bytes: &[u8],
    byteoff: &[usize],
) -> *mut TclObj {
    let m = if k <= nsubs {
        matches.get(k).copied()
    } else {
        None
    };
    if indices {
        let (start, end): (i64, i64) = match m {
            Some(rm) if rm.so != NO_MATCH => (rm.so as i64, rm.eo as i64 - 1),
            _ => (-1, -1),
        };
        let a = new_wide_int_obj(start);
        let b = new_wide_int_obj(end);
        crate::list::new_list_obj(&[a, b])
    } else {
        match m {
            Some(rm) if rm.so != NO_MATCH && rm.eo > 0 => {
                new_string_bytes(&str_bytes[byteoff[rm.so]..byteoff[rm.eo]])
            }
            _ => new_string_bytes(b""),
        }
    }
}

// -- regsub ----------------------------------------------------------------

fn regsub_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    const USAGE: &[u8] = b"regsub ?-option ...? exp string subSpec ?varName?";
    let mut c = Common::default();

    let mut i = 1;
    while i < argv.len() {
        let name = obj_bytes(argv[i]);
        if name.first() != Some(&b'-') {
            break;
        }
        if name == b"--" {
            i += 1;
            break;
        }
        let next = argv.get(i + 1).map(|&o| obj_bytes(o));
        match shared_option(&mut c, &name, next.as_deref()) {
            Some(Ok(ate)) => {
                i += 1 + usize::from(ate);
                continue;
            }
            Some(Err(())) => {
                i += 1;
                break;
            }
            None => {}
        }
        match name.as_slice() {
            b"-command" => return interp.set_error(b"regsub -command is not yet supported"),
            _ => return bad_option(interp, &name),
        }
    }

    let rest = &argv[i..];
    if rest.len() < 3 || rest.len() > 4 {
        return wrong_args(interp, USAGE);
    }
    let pattern = obj_bytes(rest[0]);
    let str_bytes = obj_bytes(rest[1]);
    let subspec = obj_bytes(rest[2]);
    let var_name: Option<Vec<u8>> = rest.get(3).map(|&o| obj_bytes(o));

    let (cps, byteoff) = decode_utf8(&str_bytes);
    let char_len = cps.len();

    let mut re = match compile(interp, &pattern, c.cflags) {
        Some(re) => re,
        None => return Code::Error,
    };
    let nsubs = re.nsub();

    let mut offset = match &c.start {
        Some(spec) => resolve_start(spec, char_len),
        None => 0,
    };

    let mut result: Vec<u8> = Vec::new();
    let mut num_matches: i64 = 0;

    while offset <= char_len {
        let notbol = offset > 0 && cps[offset - 1] != ('\n' as i32);
        let matches = match re.exec(&cps, offset, notbol) {
            Some(m) => m,
            None => break,
        };
        if num_matches == 0 && offset > 0 {
            // Copy the skipped prefix when a -start offset was given.
            result.extend_from_slice(&str_bytes[..byteoff[offset]]);
        }
        num_matches += 1;

        let m0 = matches[0];
        // Text before this match.
        result.extend_from_slice(&str_bytes[byteoff[offset]..byteoff[m0.so]]);
        // The substitution spec, with &/\\N expanded.
        apply_subspec(&mut result, &subspec, &matches, nsubs, &str_bytes, &byteoff);

        // Advance, always consuming at least one char on an empty match.
        if m0.eo == offset {
            if offset < char_len {
                result.extend_from_slice(&str_bytes[byteoff[offset]..byteoff[offset + 1]]);
            }
            offset += 1;
        } else {
            offset = m0.eo;
            if m0.so == m0.eo {
                if offset < char_len {
                    result.extend_from_slice(&str_bytes[byteoff[offset]..byteoff[offset + 1]]);
                }
                offset += 1;
            }
        }
        if !c.all {
            break;
        }
    }

    // Tail after the last match (or the whole string on no match).
    let final_bytes = if num_matches == 0 {
        str_bytes.clone()
    } else {
        if offset < char_len {
            result.extend_from_slice(&str_bytes[byteoff[offset]..]);
        }
        result
    };

    match var_name {
        Some(name) => {
            let o = new_string_bytes(&final_bytes);
            if interp.var_set(&name, o).is_err() {
                drop_fresh(o);
                return interp.set_error(b"couldn't set variable");
            }
            set_int(interp, num_matches);
        }
        None => interp.set_result(new_string_bytes(&final_bytes)),
    }
    Code::Ok
}

/// Expand a `regsub` substitution spec into `out`: `&` / `\0` → whole match,
/// `\N` → capture group N, `\\` → `\`, `\&` → `&`; any other run is copied
/// verbatim. Mirrors the `wsubspec` scan in `Tcl_RegsubObjCmd`.
fn apply_subspec(
    out: &mut Vec<u8>,
    sub: &[u8],
    matches: &[crate::regex::RegMatch],
    nsubs: usize,
    str_bytes: &[u8],
    byteoff: &[usize],
) {
    let mut k = 0;
    let mut run = 0; // start of the current verbatim run
    while k < sub.len() {
        let ch = sub[k];
        let idx: usize;
        if ch == b'&' {
            idx = 0;
        } else if ch == b'\\' {
            match sub.get(k + 1) {
                Some(&d) if d.is_ascii_digit() => idx = (d - b'0') as usize,
                Some(&d) if d == b'\\' || d == b'&' => {
                    // Literal `\` or `&`: flush the run, emit the bare char.
                    out.extend_from_slice(&sub[run..k]);
                    out.push(d);
                    k += 2;
                    run = k;
                    continue;
                }
                _ => {
                    // Backslash before any other char: keep both verbatim.
                    k += 1;
                    continue;
                }
            }
        } else {
            k += 1;
            continue;
        }
        // Reached for `&` or `\N`: flush the verbatim run, then the group.
        out.extend_from_slice(&sub[run..k]);
        if idx <= nsubs {
            if let Some(rm) = matches.get(idx) {
                if rm.so != NO_MATCH {
                    out.extend_from_slice(&str_bytes[byteoff[rm.so]..byteoff[rm.eo]]);
                }
            }
        }
        k += if ch == b'\\' { 2 } else { 1 };
        run = k;
    }
    out.extend_from_slice(&sub[run..]);
}

// -- helpers ---------------------------------------------------------------

fn set_int(interp: &mut Interp, n: i64) {
    interp.set_result(new_wide_int_obj(n));
}

fn bad_option(interp: &mut Interp, name: &[u8]) -> Code {
    let mut m = b"bad option \"".to_vec();
    m.extend_from_slice(name);
    m.extend_from_slice(
        b"\": must be -all, -nocase, -expanded, -line, -linestop, -lineanchor, -start, or --",
    );
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

    fn ok(i: &mut Interp, src: &[u8]) -> Vec<u8> {
        assert_eq!(
            i.eval_str(src),
            Code::Ok,
            "eval {:?} → {:?}",
            String::from_utf8_lossy(src),
            String::from_utf8_lossy(&i.result_bytes())
        );
        i.result_bytes()
    }

    #[test]
    fn regexp_match_and_captures() {
        leak_free(|i| {
            assert_eq!(ok(i, b"regexp {ab+c} xxabbbcyy"), b"1");
            assert_eq!(ok(i, b"regexp {z} abc"), b"0");
            ok(i, br"regexp {(\w+)@(\w+)} user@host m u h");
            assert_eq!(ok(i, b"set m"), b"user@host");
            assert_eq!(ok(i, b"set u"), b"user");
            assert_eq!(ok(i, b"set h"), b"host");
        });
    }

    #[test]
    fn regexp_all_inline_indices_nocase() {
        leak_free(|i| {
            assert_eq!(ok(i, b"regexp -all {a} banana"), b"3");
            assert_eq!(ok(i, br"regexp -inline {(\d+)} abc123def"), b"123 123");
            ok(i, b"regexp -indices {bc} abcd m");
            assert_eq!(ok(i, b"set m"), b"1 2");
            assert_eq!(ok(i, b"regexp -nocase {ABC} xabcy"), b"1");
        });
    }

    #[test]
    fn regsub_basic_all_and_backrefs() {
        leak_free(|i| {
            assert_eq!(ok(i, b"regsub {b} abc X"), b"aXc");
            assert_eq!(ok(i, b"regsub -all {a} banana _"), b"b_n_n_");
            assert_eq!(
                ok(i, br"regsub {(\w+)@(\w+)} user@host {\2.\1}"),
                b"host.user"
            );
            assert_eq!(
                ok(i, b"regsub -all {[aeiou]} {hello world} {}"),
                b"hll wrld"
            );
            // with a result variable, returns the match count.
            assert_eq!(ok(i, b"regsub -all {a} banana _ out"), b"3");
            assert_eq!(ok(i, b"set out"), b"b_n_n_");
            // no match leaves the string unchanged.
            assert_eq!(ok(i, b"regsub {z} abc X"), b"abc");
        });
    }

    #[test]
    fn bad_pattern_errors() {
        leak_free(|i| {
            assert_eq!(i.eval_str(b"regexp {a(} b"), Code::Error);
            assert!(i
                .result_bytes()
                .starts_with(b"cannot compile regular expression pattern"));
        });
    }
}
