//! `scan` — parse a string under a format (the `sscanf` analogue), per
//! `tmp/tcl9.0.3/generic/tclScan.c` (`Tcl_ScanObjCmd`).
//!
//! This is the **pure matching engine**: [`scan_match`] takes the input and
//! format as code-point slices and returns the scanned values *typed*
//! (int/double/string), the conversion count, and whether EOF preceded any
//! conversion. It names no value model and no interp, so each runtime keeps a
//! thin `scan` adapter that reads argv, calls [`scan_match`], then either
//! assigns the values to `varName`s (returning the count, `-1` on EOF before
//! any conversion) or, with no vars, collects them into a list (*inline* mode).
//!
//! Conversions: `%d`/`%i`/`%u`/`%o`/`%x`/`%b`/`%c`/`%s`/`%e`/`%f`/`%g`/`%[...]`/
//! `%n`/`%%`, with `*` (suppress), a field width, and ignored size modifiers
//! (`h`/`l`/`L`/…). Operates on Unicode code points (so `%c` and widths count
//! characters). The specifier *grammar* is shared
//! ([`tcl_syntax::scan::parse_conversion`]); this module is the matcher and the
//! cross-specifier validator that build on it.

use tcl_syntax::scan::{CharSet, ScanConversion, parse_conversion};

/// One scanned value, typed so each runtime builds its natural object
/// (`%d`/`%x`/…→int, `%e`/`%f`/`%g`→double, `%s`/`%[`→string, `%c`→the code
/// point as an int, `%n`→the count consumed).
#[derive(Debug, Clone, PartialEq)]
pub enum Scanned {
    /// An integer conversion (`%d`/`%i`/`%u`/`%o`/`%x`/`%b`/`%c`/`%n`).
    Int(i64),
    /// A floating-point conversion (`%e`/`%f`/`%g`).
    Double(f64),
    /// A string conversion (`%s`/`%[...]`).
    Str(String),
}

/// The result of matching a format against an input.
pub struct ScanOutcome {
    /// Per non-suppressed conversion, in order: the value, or `None` if the
    /// conversion failed (inline callers pad a failure with an empty string).
    pub values: Vec<Option<Scanned>>,
    /// Successful, non-suppressed, non-`%n` conversions — the variable-mode
    /// return count.
    pub nconv: usize,
    /// Whether the input reached EOF before any conversion matched (variable
    /// mode then returns `-1`).
    pub eof_before_conv: bool,
}

/// Validate a scan `fmt` the way C's `ValidateFormat` (`tclScan.c`) does, before
/// any matching: `num_vars` is the count of `varName` arguments (`0` for the
/// inline form). On success returns the number of variables the format requires;
/// on failure the diagnostic message (the runtime turns it into an error).
///
/// This catches the malformed-format cases (scan-3.x / scan-8.x): mixing `%` and
/// `%n$`, an out-of-range / duplicated `%n$` index, a width on `%c`, an
/// unterminated `%[`, a bad conversion character, and a variable/specifier count
/// mismatch.
pub fn validate_format(fmt: &[char], num_vars: usize) -> Result<usize, String> {
    let mut nassign: Vec<u32> = vec![0; num_vars];
    let mut got_xpg = false;
    let mut got_sequential = false;
    let mut obj_index: usize = 0;
    let mut xpg_size: usize = 0;

    let mixed = || "cannot mix \"%\" and \"%n$\" conversion specifiers".to_string();
    // `got_xpg` distinguishes the XPG index error from the plain count mismatch.
    let bad_index = |xpg: bool| {
        if xpg {
            "\"%n$\" argument index out of range".to_string()
        } else {
            "different numbers of variable names and field specifiers".to_string()
        }
    };

    let mut fi = 0;
    while fi < fmt.len() {
        if fmt[fi] != '%' {
            fi += 1;
            continue;
        }
        fi += 1;
        // Per-specifier grammar + validation (width on `%c`, bad verb, an
        // unterminated `%[`, …) is the shared parser's job.
        let conv = parse_conversion(fmt, &mut fi).map_err(|e| e.message())?;
        if conv.verb == '%' {
            continue;
        }

        // Cross-specifier rules: `%` and `%n$` cannot be mixed, and an XPG index
        // is bounded by the variable count (or grows the inline total).
        if let Some(idx) = conv.xpg_index {
            got_xpg = true;
            if got_sequential {
                return Err(mixed());
            }
            // 0 and >= INT_MAX (9.0 supports < INT_MAX args) are out of range.
            if idx == 0 || idx >= 2_147_483_647 {
                return Err(bad_index(true));
            }
            obj_index = idx as usize - 1;
            if num_vars != 0 && obj_index >= num_vars {
                return Err(bad_index(true));
            }
            if num_vars == 0 {
                xpg_size = xpg_size.max(idx as usize);
            }
        } else {
            got_sequential = true;
            if got_xpg {
                return Err(mixed());
            }
        }

        if !conv.suppress {
            if num_vars != 0 && obj_index >= num_vars {
                return Err(bad_index(got_xpg));
            }
            if obj_index >= nassign.len() {
                nassign.resize(obj_index + 1, 0);
            }
            nassign[obj_index] += 1;
            obj_index += 1;
        }
    }

    let total = if num_vars != 0 {
        num_vars
    } else if xpg_size != 0 {
        xpg_size
    } else {
        obj_index
    };
    for k in 0..total {
        let cnt = nassign.get(k).copied().unwrap_or(0);
        if cnt > 1 {
            return Err(
                "variable is assigned by multiple \"%n$\" conversion specifiers".to_string(),
            );
        } else if xpg_size == 0 && cnt == 0 {
            return Err("variable is not assigned by any conversion specifiers".to_string());
        }
    }
    Ok(total)
}

/// Match `fmt` against `input` (both code-point slices), producing the scanned
/// values. Pure: no value model, no interp.
#[must_use]
pub fn scan_match(input: &[char], fmt: &[char]) -> ScanOutcome {
    let mut ii = 0; // input cursor (chars)
    let mut fi = 0; // format cursor (chars)
    let mut values: Vec<Option<Scanned>> = Vec::new();
    let mut nconv = 0; // successful, non-suppressed, non-`%n` conversions
    let mut eof_before_conv = false;

    while fi < fmt.len() {
        let fc = fmt[fi];
        if fc.is_whitespace() {
            while ii < input.len() && input[ii].is_whitespace() {
                ii += 1;
            }
            fi += 1;
            continue;
        }
        if fc != '%' {
            if ii < input.len() && input[ii] == fc {
                ii += 1;
                fi += 1;
                continue;
            }
            // Running out of input (rather than a wrong char) before any
            // conversion is an underflow → -1 (scan-4.15).
            if ii >= input.len() && nconv == 0 {
                eof_before_conv = true;
            }
            break; // literal mismatch
        }
        // A conversion specifier — parsed through the shared grammar (the
        // runtime rejects malformed formats before matching, so a parse error
        // here just stops the scan).
        fi += 1;
        let mut ci = fi;
        let Ok(conv) = parse_conversion(fmt, &mut ci) else {
            break;
        };
        fi = ci;

        // `%%` matches a literal `%`.
        if conv.verb == '%' {
            if ii < input.len() && input[ii] == '%' {
                ii += 1;
                continue;
            }
            break;
        }
        // `%n` reports the characters consumed so far; it doesn't consume input
        // or count as a conversion.
        if conv.verb == 'n' {
            if !conv.suppress {
                values.push(Some(Scanned::Int(i64::try_from(ii).unwrap_or(i64::MAX))));
            }
            continue;
        }

        // Most conversions skip leading whitespace; `%c` and `%[` do not.
        if !matches!(conv.verb, 'c' | '[') {
            while ii < input.len() && input[ii].is_whitespace() {
                ii += 1;
            }
        }
        if ii >= input.len() && nconv == 0 {
            eof_before_conv = true;
        }

        if let Some(v) = scan_one(input, &mut ii, &conv) {
            if !conv.suppress {
                values.push(Some(v));
                nconv += 1;
            }
        } else {
            // Conversion failed: stop. A numeric conversion that consumed only a
            // leading sign / `0x` before EOF is an underflow (-1), unlike a stop
            // on a non-matching character (scan-4.44/4.55).
            if nconv == 0 && numeric_underflow(input, ii, conv.verb) {
                eof_before_conv = true;
            }
            // Inline mode keeps a hole.
            if !conv.suppress {
                values.push(None);
            }
            break;
        }
    }

    ScanOutcome {
        values,
        nconv,
        eof_before_conv,
    }
}

/// Scan one field for the parsed `conv`, advancing `ii`. Returns the typed
/// value, or `None` if nothing valid was read.
fn scan_one(input: &[char], ii: &mut usize, conv: &ScanConversion) -> Option<Scanned> {
    // A width of 0 means "no limit" (`%0s` reads the whole word, scan-4.21/4.25).
    let width = conv.width.filter(|&w| w != 0).unwrap_or(usize::MAX);
    match conv.verb {
        'c' => {
            // One character → its code point. (No width, no whitespace skip.)
            if *ii >= input.len() {
                return None;
            }
            let cp = i64::from(input[*ii] as u32);
            *ii += 1;
            Some(Scanned::Int(cp))
        }
        'd' | 'u' => scan_int(input, ii, width, 10),
        'i' => scan_int_auto(input, ii, width),
        'o' => scan_int(input, ii, width, 8),
        'x' | 'X' => scan_int(input, ii, width, 16),
        'b' => scan_int(input, ii, width, 2),
        's' => {
            let start = *ii;
            let mut n = 0;
            while *ii < input.len() && !input[*ii].is_whitespace() && n < width {
                *ii += 1;
                n += 1;
            }
            if *ii == start {
                return None;
            }
            Some(Scanned::Str(input[start..*ii].iter().collect()))
        }
        'e' | 'E' | 'f' | 'g' | 'G' => scan_float(input, ii, width),
        '[' => scan_set(input, ii, width, conv.charset.as_ref()?),
        _ => None, // unknown conversion
    }
}

/// Whether a *failed* numeric conversion at `start` ran out of input — it
/// consumed only a leading sign (and, for `%x`/`%i`, a `0x` prefix) before EOF.
/// This is C's underflow (which yields -1), as opposed to stopping on a present
/// non-matching character (which yields the conversion count).
fn numeric_underflow(input: &[char], start: usize, verb: char) -> bool {
    if !matches!(
        verb,
        'd' | 'i' | 'o' | 'x' | 'X' | 'b' | 'u' | 'e' | 'E' | 'f' | 'g' | 'G'
    ) {
        return false;
    }
    let mut k = start;
    if matches!(input.get(k), Some('+' | '-')) {
        k += 1;
    }
    if matches!(verb, 'x' | 'X' | 'i')
        && input.get(k) == Some(&'0')
        && matches!(input.get(k + 1), Some('x' | 'X'))
    {
        k += 2;
    }
    k >= input.len()
}

/// `%d`/`%o`/`%x`/`%b`/`%u`: an optionally-signed integer in `radix`. Every
/// conversion reads a leading sign (C's `scanf` does, even for `%x`/`%o`/`%u`).
fn scan_int(input: &[char], ii: &mut usize, width: usize, radix: u32) -> Option<Scanned> {
    let start = *ii;
    let mut s = String::new();
    let mut n = 0;
    if *ii < input.len() && (input[*ii] == '+' || input[*ii] == '-') {
        s.push(input[*ii]);
        *ii += 1;
        n += 1;
    }
    // Allow a `0x`/`0b` prefix for hex/binary.
    if (radix == 16 || radix == 2) && *ii + 1 < input.len() && input[*ii] == '0' {
        let p = input[*ii + 1].to_ascii_lowercase();
        if (radix == 16 && p == 'x') || (radix == 2 && p == 'b') {
            *ii += 2;
            n += 2;
        }
    }
    let digits_start = *ii;
    while *ii < input.len() && n < width && input[*ii].is_digit(radix) {
        s.push(input[*ii]);
        *ii += 1;
        n += 1;
    }
    if *ii == digits_start {
        *ii = start;
        return None;
    }
    let val = i64::from_str_radix(&s, radix).ok()?;
    Some(Scanned::Int(val))
}

/// `%i`: an integer with C base-0 detection — `0x`/`0X` (when a hex digit
/// follows) is hex, a leading `0` is octal, else decimal. `0b` is *not*
/// recognised by scan's `%i` (scan-4.41), and a `0x` with no hex digit folds
/// back to the `0` (scan-4.45).
fn scan_int_auto(input: &[char], ii: &mut usize, width: usize) -> Option<Scanned> {
    let save = *ii;
    let mut sign = 1i64;
    let mut n = 0;
    if *ii < input.len() && (input[*ii] == '+' || input[*ii] == '-') {
        if input[*ii] == '-' {
            sign = -1;
        }
        *ii += 1;
        n += 1;
    }
    let radix = if input.get(*ii) == Some(&'0')
        && matches!(input.get(*ii + 1), Some('x' | 'X'))
        && input.get(*ii + 2).is_some_and(char::is_ascii_hexdigit)
    {
        *ii += 2; // consume the `0x` prefix
        n += 2;
        16u32
    } else if input.get(*ii) == Some(&'0') {
        8u32 // a leading `0` (the `0` itself is the first octal digit)
    } else {
        10u32
    };
    let ds = *ii;
    let mut s = String::new();
    while *ii < input.len() && n < width && input[*ii].is_digit(radix) {
        s.push(input[*ii]);
        *ii += 1;
        n += 1;
    }
    if *ii == ds {
        *ii = save;
        return None;
    }
    let val = i64::from_str_radix(&s, radix).ok()? * sign;
    Some(Scanned::Int(val))
}

/// `%e`/`%f`/`%g`: a floating-point number.
fn scan_float(input: &[char], ii: &mut usize, width: usize) -> Option<Scanned> {
    let start = *ii;
    let mut s = String::new();
    let mut n = 0;
    let take = |c: char, ii: &mut usize, s: &mut String, n: &mut usize| {
        s.push(c);
        *ii += 1;
        *n += 1;
    };
    if *ii < input.len() && (input[*ii] == '+' || input[*ii] == '-') {
        take(input[*ii], ii, &mut s, &mut n);
    }
    let mut digits = 0;
    while *ii < input.len() && n < width && input[*ii].is_ascii_digit() {
        digits += 1;
        take(input[*ii], ii, &mut s, &mut n);
    }
    if *ii < input.len() && n < width && input[*ii] == '.' {
        take('.', ii, &mut s, &mut n);
        while *ii < input.len() && n < width && input[*ii].is_ascii_digit() {
            digits += 1;
            take(input[*ii], ii, &mut s, &mut n);
        }
    }
    if digits == 0 {
        *ii = start;
        return None;
    }
    if *ii < input.len() && n < width && matches!(input[*ii], 'e' | 'E') {
        let mut probe = *ii + 1;
        if probe < input.len() && (input[probe] == '+' || input[probe] == '-') {
            probe += 1;
        }
        if probe < input.len() && input[probe].is_ascii_digit() {
            take(input[*ii], ii, &mut s, &mut n); // 'e'
            if *ii < input.len() && (input[*ii] == '+' || input[*ii] == '-') {
                take(input[*ii], ii, &mut s, &mut n);
            }
            while *ii < input.len() && n < width && input[*ii].is_ascii_digit() {
                take(input[*ii], ii, &mut s, &mut n);
            }
        }
    }
    let val: f64 = s.parse().ok()?;
    Some(Scanned::Double(val))
}

/// `%[...]`: a run of characters matching the already-parsed `set` (its `^`
/// negation handled by [`CharSet::matches`]).
fn scan_set(input: &[char], ii: &mut usize, width: usize, set: &CharSet) -> Option<Scanned> {
    let start = *ii;
    let mut n = 0;
    while *ii < input.len() && n < width {
        if !set.matches(input[*ii]) {
            break;
        }
        *ii += 1;
        n += 1;
    }
    if *ii == start {
        return None;
    }
    Some(Scanned::Str(input[start..*ii].iter().collect()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chars(s: &str) -> Vec<char> {
        s.chars().collect()
    }

    #[test]
    fn basic_int_string() {
        let o = scan_match(&chars("42 abc"), &chars("%d %s"));
        assert_eq!(o.nconv, 2);
        assert_eq!(
            o.values,
            vec![Some(Scanned::Int(42)), Some(Scanned::Str("abc".into()))]
        );
        assert!(!o.eof_before_conv);
    }

    #[test]
    fn hex_and_char_and_set() {
        assert_eq!(
            scan_match(&chars("0xff"), &chars("%x")).values,
            vec![Some(Scanned::Int(255))]
        );
        // `%c` is the code point of the first character.
        assert_eq!(
            scan_match(&chars("Z"), &chars("%c")).values,
            vec![Some(Scanned::Int(90))]
        );
        // Scanset runs while in the set.
        assert_eq!(
            scan_match(&chars("hello123"), &chars("%[a-z]")).values,
            vec![Some(Scanned::Str("hello".into()))]
        );
    }

    #[test]
    fn float_suppress_and_width() {
        // Suppressed conversion is not collected; width caps the field.
        let o = scan_match(&chars("3.5 99"), &chars("%f %*d"));
        assert_eq!(o.values, vec![Some(Scanned::Double(3.5))]);
        assert_eq!(o.nconv, 1);
        assert_eq!(
            scan_match(&chars("12345"), &chars("%2d")).values,
            vec![Some(Scanned::Int(12))]
        );
    }

    #[test]
    fn eof_before_any_conversion() {
        let o = scan_match(&chars(""), &chars("%d"));
        assert!(o.eof_before_conv);
        assert_eq!(o.nconv, 0);
    }

    #[test]
    fn percent_n_reports_consumed() {
        let o = scan_match(&chars("abcde"), &chars("%3s%n"));
        assert_eq!(
            o.values,
            vec![Some(Scanned::Str("abc".into())), Some(Scanned::Int(3))]
        );
        // `%n` is not a conversion for the count.
        assert_eq!(o.nconv, 1);
    }
}
