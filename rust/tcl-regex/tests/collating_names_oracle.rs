//! Oracle test for the named-collating-element table, verified against a real
//! `tclsh9.0`.
//!
//! Each named collating element `[[.NAME.]]` must (a) compile and (b) match
//! exactly the character C Tcl binds the name to. We ask a real `tclsh9.0`
//! which ASCII codepoint `[[.NAME.]]` matches and assert our engine agrees.
//!
//! Skips cleanly when no `tclsh9.0` is on `PATH`.

use std::io::Write;
use std::process::{Command, Stdio};

use tcl_regex::Regex;
use tcl_regex::defs::REG_ADVANCED;

fn run_tcl(tclsh: &str, script: &str) -> Option<String> {
    let mut child = Command::new(tclsh)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    child.stdin.take()?.write_all(script.as_bytes()).ok()?;
    let out = child.wait_with_output().ok()?;
    Some(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn find_tclsh9() -> Option<String> {
    for cand in ["tclsh9.0", "tclsh"] {
        if let Some(out) = run_tcl(cand, "puts -nonewline [info patchlevel]")
            && out.starts_with("9.")
        {
            return Some(cand.to_string());
        }
    }
    None
}

/// Does our engine's `[[.name.]]` match exactly the single codepoint `cp`?
fn engine_matches_only(name: &str, cp: u32) -> bool {
    let pat = format!("[[.{name}.]]");
    let Ok(re) = Regex::compile_str(&pat, REG_ADVANCED) else {
        return false;
    };
    // Matches its own codepoint …
    let subj = [cp];
    if re.exec(&subj, 0, 0).is_none() {
        return false;
    }
    // … and does not match a different one (0x00..0x7f, skipping cp).
    (0u32..0x80)
        .filter(|&o| o != cp)
        .all(|o| re.exec(&[o], 0, 0).is_none())
}

#[test]
fn named_collating_elements_match_tclsh9() {
    let Some(tclsh) = find_tclsh9() else {
        eprintln!("skipping collating-name oracle: no tclsh9.0 on PATH");
        return;
    };

    // Ask tclsh which ASCII codepoint each name maps to (REJECT if the name is
    // not a collating element). The name list is our table's key set plus a few
    // controls tclsh is known to reject, to confirm we reject them too.
    let names = "NUL SOH STX ETX EOT ENQ ACK alert BEL BS tab HT LF newline VT \
                 FF CR SO SI DLE DC1 DC2 DC3 DC4 NAK SYN ETB CAN EM SUB ESC IS4 \
                 FS IS3 GS IS2 RS IS1 US space exclamation-mark quotation-mark \
                 number-sign dollar-sign percent-sign ampersand apostrophe \
                 left-parenthesis right-parenthesis asterisk plus-sign comma \
                 hyphen hyphen-minus period full-stop slash solidus colon \
                 semicolon less-than-sign equals-sign greater-than-sign \
                 question-mark commercial-at left-square-bracket backslash \
                 reverse-solidus right-square-bracket circumflex circumflex-accent \
                 underscore low-line grave-accent left-brace left-curly-bracket \
                 vertical-line right-brace right-curly-bracket tilde DEL zero one \
                 two three four five six seven eight nine NL bogus-name";
    let script = format!(
        "foreach n {{{names}}} {{\n\
         \x20 if {{[catch {{regexp \"\\[\\[.$n.\\]\\]\" x}}]}} {{ puts \"$n REJECT\"; continue }}\n\
         \x20 set hit REJECT\n\
         \x20 for {{set i 0}} {{$i < 128}} {{incr i}} {{\n\
         \x20   if {{[regexp \"\\[\\[.$n.\\]\\]\" [format %c $i]]}} {{ set hit $i; break }}\n\
         \x20 }}\n\
         \x20 puts \"$n $hit\"\n\
         }}\n"
    );
    let out = run_tcl(&tclsh, &script).expect("tclsh runs");

    let mut checked = 0usize;
    for line in out.lines() {
        let Some((name, val)) = line.split_once(' ') else {
            continue;
        };
        checked += 1;
        if val == "REJECT" {
            // tclsh does not recognise this name → our engine must not either.
            let compiles = Regex::compile_str(&format!("[[.{name}.]]"), REG_ADVANCED).is_ok();
            assert!(
                !compiles,
                "`[[.{name}.]]` rejected by tclsh9.0 but accepted by our engine"
            );
        } else {
            let cp: u32 = val.parse().expect("codepoint");
            assert!(
                engine_matches_only(name, cp),
                "`[[.{name}.]]` should match only U+{cp:04X} (tclsh9.0), our engine disagrees"
            );
        }
    }
    assert!(checked > 80, "oracle produced too few names ({checked})");
    eprintln!("collating-name oracle: {checked} names checked against tclsh9.0");
}
