//! `regexp` / `regsub` — a thin adapter over the shared
//! [`tcl_cmd_core::regex`] plumbing, driven by the Rust `regex` crate.
//!
//! The command logic (option parsing, the match/advance loop, `-indices`/
//! `-inline`/`-start`/`-all`, submatch assignment, the `regsub` spec expansion)
//! is shared with `runtime/rust`; only the **engine** differs. The runtime links
//! the real Tcl ARE engine; the VM uses the `regex` crate, which is close enough
//! for the common patterns library scripts and tests use but is **not** full ARE
//! (e.g. `\m`/`\M`/`[[:<:]]` word edges are out of scope). Sharing the plumbing
//! lifted the VM to the runtime's full option set and char-correct offsets — it
//! previously had only `-nocase`/`-all`/`-inline`/`-line`, lacked `-indices`/
//! `-start`, set match vars to empty on a failed match (tclsh leaves them
//! untouched), and used the wrong "couldn't compile…" error prefix.
//!
//! The seam is **character**-offset based (Tcl's index model); the crate is
//! byte-offset based, so the engine encodes the codepoint subject to a `&str`
//! once per command and maps the crate's byte offsets back to char offsets.
//! `captures_at` searches with full-haystack context, so `^`/`\b` anchoring is
//! correct at resumed offsets and the `notbol` hint is not needed.

use tcl_cmd_core::regex::{
    self as core_re, NO_MATCH, RegMatch, RegexEngine, RegexFlags, RegexpResult, RegsubResult,
};
use tcl_runtime_api::Completion;

use crate::interp::{Vm, err, ok};
use crate::value::Value;

pub(crate) fn register(vm: &mut Vm) {
    vm.register("regexp", cmd_regexp);
    vm.register("regsub", cmd_regsub);
}

/// The `regex` crate as the shared plumbing's [`RegexEngine`] provider. Reused
/// by `lsearch -regexp`.
pub(crate) struct CrateEngine;

/// A compiled crate regex plus the lazily-encoded subject (constant within one
/// command invocation) used to translate codepoint↔byte offsets.
pub(crate) struct CrateRe {
    re: regex::Regex,
    nsub: usize,
    cache: Option<Encoded>,
}

/// The subject re-encoded as a `&str` for the crate, with a char→byte table.
struct Encoded {
    s: String,
    /// `char2byte[i]` is the byte offset of char `i`; length is `nchars + 1`.
    char2byte: Vec<usize>,
}

fn encode(cps: &[i32]) -> Encoded {
    let mut s = String::new();
    let mut char2byte = Vec::with_capacity(cps.len() + 1);
    for &cp in cps {
        char2byte.push(s.len());
        // A codepoint came from `decode_utf8`, so it is a valid scalar value;
        // the `unwrap_or` guards surrogates/out-of-range defensively.
        #[allow(clippy::cast_sign_loss)]
        s.push(char::from_u32(cp as u32).unwrap_or('\u{FFFD}'));
    }
    char2byte.push(s.len());
    Encoded { s, char2byte }
}

/// Translate the Tcl ARE word-edge constructs the `regex` crate spells
/// differently. Tcl's `\m`/`\M` (begin/end of word) and the bracket forms
/// `[[:<:]]`/`[[:>:]]` map to the crate's `\b{start}`/`\b{end}` assertions;
/// `\y`/`\Y` (word boundary / non-boundary) map to `\b`/`\B`. Other escapes and
/// bracket expressions pass through untouched (backslash is significant inside
/// classes in both engines, so we copy `\x` pairs verbatim there).
fn translate_are(pat: &str) -> String {
    let chars: Vec<char> = pat.chars().collect();
    let mut out = String::with_capacity(pat.len());
    let mut i = 0;
    let mut in_class = false;
    // Does the char slice at `i` begin with the literal `lit`?
    let at = |i: usize, lit: &str| {
        chars[i..]
            .iter()
            .take(lit.chars().count())
            .copied()
            .eq(lit.chars())
    };
    while i < chars.len() {
        let c = chars[i];
        if in_class {
            if c == '\\' && i + 1 < chars.len() {
                out.push('\\');
                out.push(chars[i + 1]);
                i += 2;
                continue;
            }
            if c == ']' {
                in_class = false;
            }
            out.push(c);
            i += 1;
            continue;
        }
        // The standalone word-edge bracket forms (not real character classes).
        if at(i, "[[:<:]]") {
            out.push_str(r"\b{start}");
            i += 7;
            continue;
        }
        if at(i, "[[:>:]]") {
            out.push_str(r"\b{end}");
            i += 7;
            continue;
        }
        if c == '\\' && i + 1 < chars.len() {
            let n = chars[i + 1];
            let mapped = match n {
                'm' => Some(r"\b{start}"),
                'M' => Some(r"\b{end}"),
                'y' => Some(r"\b"),
                'Y' => Some(r"\B"),
                _ => None,
            };
            if let Some(rep) = mapped {
                out.push_str(rep);
            } else {
                out.push('\\');
                out.push(n);
            }
            i += 2;
            continue;
        }
        if c == '[' {
            in_class = true;
        }
        out.push(c);
        i += 1;
    }
    out
}

impl RegexEngine for CrateEngine {
    type Regex = CrateRe;

    fn compile(pattern: &[u8], flags: RegexFlags) -> Result<CrateRe, Vec<u8>> {
        let Ok(raw) = core::str::from_utf8(pattern) else {
            return Err(b"invalid UTF-8 in regular expression".to_vec());
        };
        let translated = translate_are(raw);
        let pat = translated.as_str();
        // Map the engine-neutral flags to the crate's inline flags. Tcl's default
        // (no `-linestop`) is that `.` matches a newline, which is the crate's
        // `s` flag; `-lineanchor` is `m`, `-nocase` is `i`, `-expanded` is `x`.
        let mut fl = String::new();
        if !flags.linestop {
            fl.push('s');
        }
        if flags.nocase {
            fl.push('i');
        }
        if flags.lineanchor {
            fl.push('m');
        }
        if flags.expanded {
            fl.push('x');
        }
        let full = if fl.is_empty() {
            pat.to_string()
        } else {
            format!("(?{fl}){pat}")
        };
        match regex::Regex::new(&full) {
            Ok(re) => {
                let nsub = re.captures_len().saturating_sub(1);
                Ok(CrateRe {
                    re,
                    nsub,
                    cache: None,
                })
            }
            Err(e) => Err(format!("{e}").into_bytes()),
        }
    }

    fn nsub(re: &CrateRe) -> usize {
        re.nsub
    }

    fn exec(re: &mut CrateRe, cps: &[i32], offset: usize, _notbol: bool) -> Option<Vec<RegMatch>> {
        // (Re)build the encoded subject; it is constant across a command's execs.
        if re
            .cache
            .as_ref()
            .is_none_or(|e| e.char2byte.len() != cps.len() + 1)
        {
            re.cache = Some(encode(cps));
        }
        let enc = re.cache.as_ref().expect("just populated");
        let byte_start = enc.char2byte[offset.min(cps.len())];
        let groups = re.re.captures_at(&enc.s, byte_start)?;
        // Map each group's byte span back to char offsets (matches sit on char
        // boundaries, which are exactly the entries of `char2byte`).
        let to_char = |b: usize| enc.char2byte.binary_search(&b).unwrap_or_else(|x| x);
        let out = (0..groups.len())
            .map(|i| match groups.get(i) {
                Some(m) => RegMatch {
                    so: to_char(m.start()),
                    eo: to_char(m.end()),
                },
                None => RegMatch {
                    so: NO_MATCH,
                    eo: NO_MATCH,
                },
            })
            .collect();
        Some(out)
    }
}

/// Compile `pattern` and report whether it matches anywhere in `subject` — the
/// boolean core of the `INST_REGEXP` opcode (`[regexp $pat $str]` in value
/// position). Mirrors C Tcl's `INST_REGEXP`, which compiles with
/// `TCL_REG_ADVANCED` and reports the `Tcl_RegExpExecObj` result; `nocase` adds
/// `TCL_REG_NOCASE`. Returns the compile-error message on a bad pattern.
pub(crate) fn regexp_matches(pattern: &str, subject: &str, nocase: bool) -> Result<bool, String> {
    let flags = RegexFlags {
        nocase,
        expanded: false,
        linestop: false,
        lineanchor: false,
    };
    let mut re = CrateEngine::compile(pattern.as_bytes(), flags)
        .map_err(|e| String::from_utf8_lossy(&e).into_owned())?;
    let cps: Vec<i32> = subject.chars().map(|c| c as i32).collect();
    Ok(CrateEngine::exec(&mut re, &cps, 0, false).is_some())
}

fn cmd_regexp(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let bytes: Vec<Vec<u8>> = args
        .iter()
        .map(|v| v.to_str().as_bytes().to_vec())
        .collect();
    let refs: Vec<&[u8]> = bytes.iter().map(Vec::as_slice).collect();
    match core_re::regexp::<Vm, CrateEngine>(vm, &refs) {
        Ok(RegexpResult::Inline(v)) => ok(v),
        Ok(RegexpResult::Count { assign, count }) => {
            if let Some(pairs) = assign {
                for (name, val) in pairs {
                    if let Err(c) = vm.var_set(&String::from_utf8_lossy(&name), val) {
                        return c;
                    }
                }
            }
            ok(Value::int(count))
        }
        Err(e) => err(String::from_utf8_lossy(&e.0).into_owned()),
    }
}

fn cmd_regsub(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let bytes: Vec<Vec<u8>> = args
        .iter()
        .map(|v| v.to_str().as_bytes().to_vec())
        .collect();
    let refs: Vec<&[u8]> = bytes.iter().map(Vec::as_slice).collect();
    let RegsubResult { text, count, var } = match core_re::regsub::<CrateEngine>(&refs) {
        Ok(r) => r,
        Err(e) => return err(String::from_utf8_lossy(&e.0).into_owned()),
    };
    let result = Value::string(String::from_utf8_lossy(&text).into_owned());
    match var {
        Some(name) => {
            if let Err(c) = vm.var_set(&String::from_utf8_lossy(&name), result) {
                return c;
            }
            ok(Value::int(count))
        }
        None => ok(result),
    }
}
