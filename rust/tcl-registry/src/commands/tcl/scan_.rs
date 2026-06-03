//! `scan` — parse a string using scanf-style conversion.
use crate::prelude::*;

/// SYNC-JUN03 follow-up: constant-fold the *inline* `scan string format` form
/// (no `varName` — that form writes variables and must never fold).
///
/// The scanf semantics are modelled **directly on tclsh**, *not* transcribed
/// from main's `dialects/tcl/const_fold.py::_tcl_scan`, which is unsound: it
/// reads no `0x` prefix and so folds `scan 0xff %x` to `0` where every tclsh
/// gives `255`.  Only the dialect-invariant subset folds — verified against
/// `tclsh8.4`/`8.5`/`8.6`/`9.0` during development (the differential harness
/// pins these against the live `tclsh9.0`):
///
/// * conversions `%d` / `%o` / `%x` / `%X` / `%s` / `%c` / `%%` only.  A width,
///   `*` suppression, a `%[set]`, a size modifier (`%ld`), a positional `%n$`,
///   a float conversion, and `%i` (base-0) / `%u` (unsigned wraparound) all
///   bail (their value or availability is version-dependent).
/// * a numeric result must fit the signed 32-bit range every release shares,
///   `[-2³¹, 2³¹-1]` — above it 8.4 wraps, 8.5/8.6 widen, and 9.0 clamps or
///   bignums, so e.g. `scan 2147483648 %d` is three-way divergent.
/// * a `0x` / `0X` prefix on a `%x` input bails: tclsh reads it but its
///   sign handling diverges in 8.4 (`scan -0xff %x` → `0` vs `-255`), and it
///   is the exact case main mis-folds.
/// * `%c` yields one character's codepoint (no whitespace skip; ASCII, since
///   the whole input is ASCII-gated).  `%s` matches a non-whitespace run and
///   bails if that run would need Tcl list-quoting, so the result is a plain
///   space-join that renders identically to tclsh's returned list.
/// * a literal mismatch, a failed / partial / empty conversion set, or a
///   non-ASCII string / format all bail.
fn fold_scan(args: &[&str]) -> Option<String> {
    let [string, fmt] = args else {
        return None; // `scan str fmt var ...` writes vars — never fold
    };
    if !string.is_ascii() || !fmt.is_ascii() {
        return None;
    }
    let s = string.as_bytes();
    let f = fmt.as_bytes();
    let mut out: Vec<String> = Vec::new();
    let mut si = 0;
    let mut fi = 0;
    while fi < f.len() {
        let fc = f[fi];
        if fc.is_ascii_whitespace() {
            // Whitespace in the format matches a (possibly empty) input run.
            while si < s.len() && s[si].is_ascii_whitespace() {
                si += 1;
            }
            fi += 1;
            continue;
        }
        if fc != b'%' {
            if si < s.len() && s[si] == fc {
                si += 1;
                fi += 1;
                continue;
            }
            return None; // literal mismatch
        }
        // A conversion specifier.
        fi += 1;
        let conv = *f.get(fi)?;
        fi += 1;
        if conv == b'%' {
            if si < s.len() && s[si] == b'%' {
                si += 1;
                continue;
            }
            return None;
        }
        let (value, next) = match conv {
            // One character, no whitespace skip (ASCII → codepoint < 128).
            b'c' => (u32::from(*s.get(si)?).to_string(), si + 1),
            b's' => scan_str_run(s, si)?,
            b'd' | b'o' | b'x' | b'X' => scan_int(s, si, conv)?,
            // `%i` / `%u` / `%f` / `%e` / `%g`, a width digit, `*`, `%[`, a
            // size modifier, a positional `$` — all bail.
            _ => return None,
        };
        out.push(value);
        si = next;
    }
    if out.is_empty() {
        return None;
    }
    // Every element is list-safe (numeric, or a `%s` run screened below), so a
    // space-join is byte-identical to the list tclsh's inline `scan` returns.
    Some(out.join(" "))
}

/// `%s`: skip leading whitespace, match a non-whitespace run, and return it
/// with the new input offset.  Bails if the run is empty or would need Tcl
/// list-quoting (so the caller's space-join matches tclsh's list rendering).
fn scan_str_run(s: &[u8], mut si: usize) -> Option<(String, usize)> {
    while si < s.len() && s[si].is_ascii_whitespace() {
        si += 1;
    }
    if si >= s.len() {
        return None;
    }
    let start = si;
    while si < s.len() && !s[si].is_ascii_whitespace() {
        si += 1;
    }
    let word = &s[start..si];
    if word.first() == Some(&b'#')
        || word
            .iter()
            .any(|&b| matches!(b, b'{' | b'}' | b'[' | b']' | b'"' | b'\\' | b'$' | b';'))
    {
        return None;
    }
    Some((String::from_utf8_lossy(word).into_owned(), si))
}

/// `%d` / `%o` / `%x` / `%X`: skip whitespace, read an optional sign and the
/// base's digits, and return the value — bounded to the signed-32-bit range
/// every Tcl 8.4 → 9.0 agrees on — with the new offset.  Bails on a `0x`
/// prefix, no digits, or an out-of-range / overflowing value.
fn scan_int(s: &[u8], mut si: usize, conv: u8) -> Option<(String, usize)> {
    while si < s.len() && s[si].is_ascii_whitespace() {
        si += 1;
    }
    let neg = match s.get(si) {
        Some(b'-') => {
            si += 1;
            true
        }
        Some(b'+') => {
            si += 1;
            false
        }
        _ => false,
    };
    let base: u32 = match conv {
        b'o' => 8,
        b'x' | b'X' => 16,
        _ => 10,
    };
    // A `0x` / `0X` prefix on a `%x` input is unsound to fold (main mis-reads
    // it; `-0x…` diverges in 8.4).
    if base == 16 && s.get(si) == Some(&b'0') && matches!(s.get(si + 1), Some(b'x' | b'X')) {
        return None;
    }
    let start = si;
    while si < s.len() && char::from(s[si]).is_digit(base) {
        si += 1;
    }
    if si == start {
        return None; // no digits — conversion failed
    }
    let mag = i64::from_str_radix(std::str::from_utf8(&s[start..si]).ok()?, base).ok()?;
    let v = if neg { -mag } else { mag };
    if !(-2_147_483_648..=2_147_483_647).contains(&v) {
        return None;
    }
    Some((v.to_string(), si))
}

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "scan",
        traits: Traits::BYTE_COMPILED,
        arity: Arity::at_least(2),
        arg_roles: &[
            (2, ArgRole::VarWrite),
            (3, ArgRole::VarWrite),
            (4, ArgRole::VarWrite),
            (5, ArgRole::VarWrite),
        ],
        // Arity-dependent return: `scan str fmt var ...` returns the int count,
        // but the inline `scan str fmt` (which `fold_scan` folds) returns the
        // *list* of converted values.  The registry can't express that yet, so
        // leave the type overdefined (`None`) rather than wrongly claim `Int`
        // for the inline form.
        return_type: None,
        const_fold: Some(fold_scan),
        hover: Some(HoverSnippet::brief(
            "Parse a string using scanf-style conversion.",
            &["scan string format ?varName ...?"],
            "Tcl scan(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}

#[cfg(test)]
mod tests {
    use super::fold_scan;

    #[test]
    fn scan_folds_dialect_invariant_subset() {
        // Single + multiple integer / hex / octal / char / string conversions.
        assert_eq!(fold_scan(&["42", "%d"]).as_deref(), Some("42"));
        assert_eq!(fold_scan(&["-5", "%d"]).as_deref(), Some("-5"));
        assert_eq!(fold_scan(&[" 42", "%d"]).as_deref(), Some("42"));
        assert_eq!(fold_scan(&["ff", "%x"]).as_deref(), Some("255"));
        assert_eq!(fold_scan(&["-ff", "%x"]).as_deref(), Some("-255"));
        assert_eq!(fold_scan(&["17", "%o"]).as_deref(), Some("15"));
        assert_eq!(fold_scan(&["A", "%c"]).as_deref(), Some("65"));
        assert_eq!(fold_scan(&["abc", "%s"]).as_deref(), Some("abc"));
        assert_eq!(fold_scan(&["1 2 3", "%d %d %d"]).as_deref(), Some("1 2 3"));
        assert_eq!(fold_scan(&["x42", "x%d"]).as_deref(), Some("42"));
        assert_eq!(fold_scan(&["1.5", "%d"]).as_deref(), Some("1"));
        // The 32-bit boundary folds; one past it (per-version) bails.
        assert_eq!(
            fold_scan(&["2147483647", "%d"]).as_deref(),
            Some("2147483647")
        );
        assert_eq!(
            fold_scan(&["-2147483648", "%d"]).as_deref(),
            Some("-2147483648")
        );
        assert_eq!(
            fold_scan(&["7fffffff", "%x"]).as_deref(),
            Some("2147483647")
        );
    }

    #[test]
    fn scan_bails_on_unsound_and_unsupported() {
        // The varName form has side effects.
        assert_eq!(fold_scan(&["42", "%d", "v"]), None);
        // Numeric results outside the shared 32-bit range diverge.
        assert_eq!(fold_scan(&["2147483648", "%d"]), None);
        assert_eq!(fold_scan(&["-2147483649", "%d"]), None);
        assert_eq!(fold_scan(&["ffffffff", "%x"]), None);
        // `0x`-prefixed %x (main's wrong-fold case) bails.
        assert_eq!(fold_scan(&["0xff", "%x"]), None);
        assert_eq!(fold_scan(&["-0xff", "%x"]), None);
        // Unsupported conversions / specs bail.
        assert_eq!(fold_scan(&["10", "%i"]), None);
        assert_eq!(fold_scan(&["10", "%u"]), None);
        assert_eq!(fold_scan(&["3.5", "%f"]), None);
        assert_eq!(fold_scan(&["42", "%5d"]), None);
        assert_eq!(fold_scan(&["42", "%*d"]), None);
        assert_eq!(fold_scan(&["abc", "%[a-c]"]), None);
        // Partial / failed / empty matches and literal mismatch bail.
        assert_eq!(fold_scan(&["1 x", "%d %d"]), None);
        assert_eq!(fold_scan(&["x", "%d"]), None);
        assert_eq!(fold_scan(&["42", "x%d"]), None);
        assert_eq!(fold_scan(&["", "%d"]), None);
        // A `%s` run needing list-quoting, and non-ASCII input, bail.
        assert_eq!(fold_scan(&["a;b", "%s"]), None);
        assert_eq!(fold_scan(&["caf\u{e9}", "%s"]), None);
    }
}
