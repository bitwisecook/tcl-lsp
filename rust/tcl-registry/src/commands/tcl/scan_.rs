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

//! `scan` — parse a string using scanf-style conversion.
use crate::prelude::*;

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::Variable,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
    dialects: None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "scan string format ?varName varName ...?",
    dialects: None,
}];

/// `scan string format ?varName ...?` accepts variable-name args from
/// index 2 onward to the end of the call.  Resolve `VarWrite` dynamically for
/// every trailing arg rather than hard-coding a finite slot count, so calls
/// with 20 / 50 / 100 vars don't false-fire W210 on the unmodelled tail.
fn scan_arg_roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
    // Index 1 is the %-string (the §6 argument-DSL rung: `%b` is 8.6+).
    std::iter::once((1u8, ArgRole::ScanFormat))
        .chain((2..args.len()).filter_map(|i| u8::try_from(i).ok().map(|i| (i, ArgRole::VarWrite))))
        .collect()
}

/// Constant-fold the *inline* `scan string format` form
/// (no `varName` — that form writes variables and must never fold).
///
/// The scanf semantics are modelled **directly on tclsh**.  A naive fold
/// that reads no `0x` prefix is unsound: it folds `scan 0xff %x` to `0`
/// where every tclsh gives `255`.
/// Only the dialect-invariant subset folds — verified against
/// `tclsh8.4`/`8.5`/`8.6`/`9.0`/`9.1` during development (the differential
/// harness pins these against the live `tclsh9.0`; the `scan.n` manpage
/// text is byte-for-byte identical between 9.0 and 9.1, so nothing here is
/// 9.1-specific):
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
    // The conversion grammar is shared with the runtime (`tcl_syntax::scan`); we
    // only fold the dialect-invariant plain conversions and bail on the rest.
    let f: Vec<char> = fmt.chars().collect();
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
        if fc != '%' {
            if si < s.len() && s[si] == fc as u8 {
                si += 1;
                fi += 1;
                continue;
            }
            return None; // literal mismatch
        }
        // A conversion specifier — parsed through the shared grammar; a malformed
        // spec bails (the runtime errors on it, so folding it would be unsound).
        fi += 1;
        let mut ci = fi;
        let conv = tcl_syntax::scan::parse_conversion(&f, &mut ci).ok()?;
        fi = ci;
        if conv.verb == '%' {
            if si < s.len() && s[si] == b'%' {
                si += 1;
                continue;
            }
            return None;
        }
        // A width, `*` suppression, a positional `%n$`, a size modifier, or a
        // `%[set]` makes the value version-dependent or unmodelled here — bail.
        if conv.suppress
            || conv.xpg_index.is_some()
            || conv.width.is_some()
            || conv.size.is_some()
            || conv.charset.is_some()
        {
            return None;
        }
        let (value, next) = match conv.verb {
            // One character, no whitespace skip (ASCII → codepoint < 128).
            'c' => (u32::from(*s.get(si)?).to_string(), si + 1),
            's' => scan_str_run(s, si)?,
            'd' | 'o' | 'x' | 'X' => scan_int(s, si, conv.verb as u8)?,
            // `%i` / `%u` / `%f` / `%e` / `%g` / `%b` — value or availability is
            // version-dependent — bail.
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
/// every Tcl 8.4 → 9.1 agrees on for the no-size-modifier case — with the
/// new offset.  Bails on a `0x` prefix, no digits, or an out-of-range /
/// overflowing value.
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
        traits: Traits::BYTE_COMPILED | Traits::FRAME_HASH_BUILTIN,
        arity: Arity::at_least(2),
        // Documented return is the int conversion count (`scan str fmt
        // var ...`). The inline `scan str fmt` form (folded by `fold_scan`)
        // actually yields the *list* of converted values — a per-form
        // refinement deferred to per-form return typing.
        return_type: Some(TclType::Int),
        // `scan` writes format-dependent conversions (`%d` → int, `%s` →
        // string, `%f` → double) to its targets while returning the *count*.
        // Without parsing the format the target intreps are unknown, so they
        // must not be typed `Int` (issue #867).
        var_write_typing: VarWriteTyping::Destructured,
        const_fold: Some(fold_scan),
        hover: Some(HoverSnippet {
            summary: "Parse string using conversion specifiers in the style of sscanf",
            synopsis: &["scan string format ?varName varName ...?"],
            snippet: "Skips whitespace in string before each conversion, except %c and %[chars]/%[^chars], which do not. A %n$ positional specifier sends that conversion's result to argument n (1-based) instead of the next varName in sequence; once one specifier in format is positional, every specifier must be. Integer conversions (%d, %o, %x/%X, %b, %u, %i) take an optional size modifier that bounds the stored range: h or no modifier limits it to 32 bits, l limits it to the wide()/64-bit range, and ll removes the limit entirely (arbitrary precision). L is a synonym for ll (unlimited) on Tcl 9.0 and later, but on Tcl 8.5 and 8.6 it instead meant the same 64-bit range as l; Tcl 8.4 has neither ll nor an unlimited range, only a plain l or L meaning \"at least 64 bits\". Tcl 9.0 and later also accept q and j as synonyms for l (wide/64-bit range) and z and t as platform-pointerSize-dependent (matching h's 32-bit range or l's wide range depending on the tcl_platform(pointerSize) value); none of these four size modifiers are recognized before 9.0. %b (binary integer) needs Tcl 8.6 or later. With one or more varName arguments the command writes each converted value and returns the count of conversions performed, or -1 if string runs out before the first conversion completes; with no varName arguments it returns the converted values as a list instead, or the empty string on that same end-of-input failure.",
            source: "Tcl scan(n)",
            examples: "scan $string {#%2x%2x%2x} r g b\nscan $string {%d:%d} hours minutes\nset count [scan $string {%d %d} x y]\nset values [scan $string {%d %d}]",
            return_value: "The number of successful conversions, or -1 if the end of string is reached before any conversion completes. With no varName arguments, returns the converted values as a list instead, or the empty string on that same end-of-input failure.",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        arg_role_resolver: Some(scan_arg_roles),
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
