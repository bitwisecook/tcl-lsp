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
//! characters).

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
            break; // literal mismatch
        }
        // A conversion specifier.
        fi += 1;
        if fi >= fmt.len() {
            break;
        }
        if fmt[fi] == '%' {
            if ii < input.len() && input[ii] == '%' {
                ii += 1;
                fi += 1;
                continue;
            }
            break;
        }
        let mut suppress = false;
        if fmt[fi] == '*' {
            suppress = true;
            fi += 1;
        }
        let mut width = 0usize;
        let mut has_width = false;
        while fi < fmt.len() && fmt[fi].is_ascii_digit() {
            has_width = true;
            // The field width is taken from the format string, so a pathological
            // run of digits (`%999…9d`) would overflow usize here. Saturate
            // instead: a width past `usize::MAX` is already effectively unbounded
            // (it can never exceed the input length used as the real cap).
            width = width
                .saturating_mul(10)
                .saturating_add(fmt[fi].to_digit(10).unwrap_or(0) as usize);
            fi += 1;
        }
        // Ignore size modifiers (we scan into i64/f64).
        while fi < fmt.len() && matches!(fmt[fi], 'h' | 'l' | 'L' | 'q' | 'j' | 'z' | 't') {
            fi += 1;
        }
        if fi >= fmt.len() {
            break;
        }
        let conv = fmt[fi];
        fi += 1;

        // `%n` reports the characters consumed so far; it doesn't consume input
        // or count as a conversion.
        if conv == 'n' {
            if !suppress {
                values.push(Some(Scanned::Int(i64::try_from(ii).unwrap_or(i64::MAX))));
            }
            continue;
        }

        // Most conversions skip leading whitespace; `%c` and `%[` do not.
        if !matches!(conv, 'c' | '[') {
            while ii < input.len() && input[ii].is_whitespace() {
                ii += 1;
            }
        }
        if ii >= input.len() && nconv == 0 {
            eof_before_conv = true;
        }

        let field_max = if has_width { width } else { usize::MAX };
        if let Some(v) = scan_one(input, &mut ii, conv, field_max, fmt, &mut fi) {
            if !suppress {
                values.push(Some(v));
                nconv += 1;
            }
        } else {
            // Conversion failed: stop. Inline mode keeps a hole.
            if !suppress {
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

/// Scan one field for `conv`, advancing `ii`. Returns the typed value, or
/// `None` if nothing valid was read. `fi` is advanced past a `%[...]` set.
fn scan_one(
    input: &[char],
    ii: &mut usize,
    conv: char,
    width: usize,
    fmt: &[char],
    fi: &mut usize,
) -> Option<Scanned> {
    match conv {
        'c' => {
            // One character → its code point. (No width, no whitespace skip.)
            if *ii >= input.len() {
                return None;
            }
            let cp = i64::from(input[*ii] as u32);
            *ii += 1;
            Some(Scanned::Int(cp))
        }
        'd' | 'u' => scan_int(input, ii, width, 10, true),
        'i' => scan_int_auto(input, ii, width),
        'o' => scan_int(input, ii, width, 8, false),
        'x' | 'X' => scan_int(input, ii, width, 16, false),
        'b' => scan_int(input, ii, width, 2, false),
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
        '[' => scan_set(input, ii, width, fmt, fi),
        _ => None, // unknown conversion
    }
}

/// `%d`/`%o`/`%x`/`%b`/`%u`: an optionally-signed integer in `radix`.
fn scan_int(
    input: &[char],
    ii: &mut usize,
    width: usize,
    radix: u32,
    signed: bool,
) -> Option<Scanned> {
    let start = *ii;
    let mut s = String::new();
    let mut n = 0;
    if signed && *ii < input.len() && (input[*ii] == '+' || input[*ii] == '-') {
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

/// `%i`: an integer with C-style base detection (`0x`→16, `0`→8, else 10).
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
    let radix = if *ii + 1 < input.len() && input[*ii] == '0' {
        match input[*ii + 1].to_ascii_lowercase() {
            'x' => {
                *ii += 2;
                n += 2;
                16u32
            }
            'b' => {
                *ii += 2;
                n += 2;
                2u32
            }
            _ => 8u32,
        }
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

/// `%[...]`: a run of characters in (or, with a leading `^`, not in) the set.
/// `fi` points just past the `[`; it is advanced past the closing `]`.
fn scan_set(
    input: &[char],
    ii: &mut usize,
    width: usize,
    fmt: &[char],
    fi: &mut usize,
) -> Option<Scanned> {
    let mut negate = false;
    if *fi < fmt.len() && fmt[*fi] == '^' {
        negate = true;
        *fi += 1;
    }
    // Collect set members (a `]` immediately after `[`/`[^` is a literal member).
    let mut members: Vec<char> = Vec::new();
    let mut ranges: Vec<(char, char)> = Vec::new();
    let mut first = true;
    while *fi < fmt.len() {
        let c = fmt[*fi];
        if c == ']' && !first {
            *fi += 1;
            break;
        }
        first = false;
        // A range `a-z` (but not a trailing `-`).
        if *fi + 2 < fmt.len() && fmt[*fi + 1] == '-' && fmt[*fi + 2] != ']' {
            ranges.push((c, fmt[*fi + 2]));
            *fi += 3;
        } else {
            members.push(c);
            *fi += 1;
        }
    }
    let in_set = |c: char| {
        members.contains(&c)
            || ranges
                .iter()
                .any(|&(a, b)| (a..=b).contains(&c) || (b..=a).contains(&c))
    };
    let start = *ii;
    let mut n = 0;
    while *ii < input.len() && n < width {
        let c = input[*ii];
        // Stop at the first character whose membership doesn't match the set's
        // polarity (`negate` flips it for `%[^...]`).
        if in_set(c) == negate {
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

    #[test]
    fn huge_field_width_saturates_without_overflow() {
        // A giant width in the format string must not overflow the usize
        // accumulator; it saturates and behaves as an unbounded field (the input
        // length is the real cap), so the whole string scans as one `%s`.
        let o = scan_match(&chars("hello"), &chars("%999999999999999999999s"));
        assert_eq!(o.values, vec![Some(Scanned::Str("hello".into()))]);
        assert_eq!(o.nconv, 1);
    }
}
