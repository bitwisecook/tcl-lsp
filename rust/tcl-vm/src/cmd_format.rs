//! The `format` command — Tcl's `sprintf`-style string formatting.
//!
//! Reuses [`tcl_syntax::format::parse_spec`] for the `%…` conversion grammar
//! (flags / width / `.precision` / verb) and renders each conversion against
//! the next argument, mirroring C Tcl's `Tcl_FormatObjCmd` for the common
//! verb set (`d i u s x X o b c f e E g G %`).

use tcl_runtime_api::Completion;
use tcl_syntax::format::{FmtFlags, Spec, parse_spec};

use crate::interp::{Vm, err, ok};
use crate::value::Value;

pub(crate) fn register(vm: &mut Vm) {
    vm.register("format", cmd_format);
    vm.register("scan", cmd_scan);
}

/// `scan string format ?varName ...?` — parse `string` per the conversion
/// `format`. With `varName`s, assign each conversion and return the count; with
/// none ("inline" form), return the conversions as a list. Supports the common
/// verbs (`d i o x c s f e g`) with optional width and `*` (assignment
/// suppression).
fn cmd_scan(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let (input, fmt, vars) = match args {
        [s, f, vars @ ..] => (s.to_str(), f.to_str(), vars),
        _ => return err("wrong # args: should be \"scan string format ?varName ...?\""),
    };
    let results = match scan_parse(&input, &fmt) {
        Ok(r) => r,
        Err(e) => return err(e),
    };
    if vars.is_empty() {
        // Inline form: return the conversions as a list (empty string for a
        // field that did not match).
        return ok(Value::list(
            results
                .into_iter()
                .map(|o| o.unwrap_or_else(Value::empty))
                .collect(),
        ));
    }
    let mut count = 0;
    for (i, var) in vars.iter().enumerate() {
        match results.get(i).cloned().flatten() {
            Some(v) => {
                if let Err(c) = vm.set_var(&var.to_str(), v) {
                    return c;
                }
                count += 1;
            }
            None => break,
        }
    }
    ok(Value::int(count))
}

/// Apply a `scan` format to `input`, returning one slot per non-suppressed
/// conversion (`None` when the field failed to match).
fn scan_parse(input: &str, fmt: &str) -> Result<Vec<Option<Value>>, String> {
    let inp: Vec<char> = input.chars().collect();
    let fch: Vec<char> = fmt.chars().collect();
    let mut ip = 0usize;
    let mut fp = 0usize;
    let mut out: Vec<Option<Value>> = Vec::new();
    while fp < fch.len() {
        let c = fch[fp];
        if c.is_whitespace() {
            fp += 1;
            while ip < inp.len() && inp[ip].is_whitespace() {
                ip += 1;
            }
            continue;
        }
        if c != '%' {
            // Literal: must match the next input char.
            if ip < inp.len() && inp[ip] == c {
                ip += 1;
                fp += 1;
                continue;
            }
            break;
        }
        // Conversion specifier: `%[*][width]verb`.
        fp += 1;
        if fp < fch.len() && fch[fp] == '%' {
            if ip < inp.len() && inp[ip] == '%' {
                ip += 1;
                fp += 1;
                continue;
            }
            break;
        }
        let suppress = fp < fch.len() && fch[fp] == '*';
        if suppress {
            fp += 1;
        }
        let mut width = 0usize;
        let mut has_width = false;
        while fp < fch.len() && fch[fp].is_ascii_digit() {
            has_width = true;
            width = width * 10 + (fch[fp] as usize - '0' as usize);
            fp += 1;
        }
        let Some(&verb) = fch.get(fp) else {
            return Err("bad scan conversion".to_string());
        };
        fp += 1;
        // Most verbs skip leading whitespace first (not `c`).
        if verb != 'c' {
            while ip < inp.len() && inp[ip].is_whitespace() {
                ip += 1;
            }
        }
        let w = if has_width { Some(width) } else { None };
        let value = scan_field(verb, &inp, &mut ip, w)?;
        if !suppress {
            out.push(value);
        }
    }
    Ok(out)
}

/// Extract one `scan` conversion of kind `verb` from `inp` starting at `*ip`
/// (advancing it), bounded by an optional field `width`. Returns the converted
/// value, or `None` when the field does not match.
fn scan_field(
    verb: char,
    inp: &[char],
    ip: &mut usize,
    width: Option<usize>,
) -> Result<Option<Value>, String> {
    // Collect the longest prefix (within `width`) whose chars satisfy `pred`.
    let take = |ip: &mut usize, pred: &dyn Fn(usize, char) -> bool| -> String {
        let mut s = String::new();
        let mut n = 0;
        while *ip < inp.len() && width.is_none_or(|w| n < w) && pred(n, inp[*ip]) {
            s.push(inp[*ip]);
            *ip += 1;
            n += 1;
        }
        s
    };
    let value = match verb {
        'c' => {
            if *ip < inp.len() {
                let ch = inp[*ip];
                *ip += 1;
                Some(Value::int(i64::from(u32::from(ch))))
            } else {
                None
            }
        }
        'd' | 'i' | 'u' => take(ip, &|n, c| {
            c.is_ascii_digit() || (n == 0 && (c == '-' || c == '+'))
        })
        .parse::<i64>()
        .ok()
        .map(Value::int),
        'x' | 'X' => {
            let s = take(ip, &|_, c| c.is_ascii_hexdigit());
            i64::from_str_radix(&s, 16).ok().map(Value::int)
        }
        'o' => {
            let s = take(ip, &|_, c| ('0'..='7').contains(&c));
            i64::from_str_radix(&s, 8).ok().map(Value::int)
        }
        'f' | 'e' | 'E' | 'g' | 'G' => take(ip, &|n, c| {
            c.is_ascii_digit() || c == '.' || (n == 0 && (c == '-' || c == '+')) || c == 'e' || c == 'E'
        })
        .parse::<f64>()
        .ok()
        .map(Value::double),
        's' => {
            let s = take(ip, &|_, c| !c.is_whitespace());
            if s.is_empty() { None } else { Some(Value::string(s)) }
        }
        other => return Err(format!("bad scan conversion character \"%{other}\"")),
    };
    Ok(value)
}

fn cmd_format(_vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some((fmt, rest)) = args.split_first() else {
        return err("wrong # args: should be \"format formatString ?arg ...?\"");
    };
    match render(&fmt.to_str(), rest) {
        Ok(s) => ok(Value::string(s)),
        Err(e) => err(e),
    }
}

/// Render `fmt` against `args`, consuming arguments left-to-right.
fn render(fmt: &str, args: &[Value]) -> Result<String, String> {
    let bytes = fmt.as_bytes();
    let mut out = String::new();
    let mut i = 0;
    let mut ai = 0;
    while i < bytes.len() {
        if bytes[i] != b'%' {
            // Copy a whole UTF-8 char, not a single byte.
            let ch_len = utf8_len(bytes[i]);
            out.push_str(&fmt[i..(i + ch_len).min(bytes.len())]);
            i += ch_len;
            continue;
        }
        if bytes.get(i + 1) == Some(&b'%') {
            out.push('%');
            i += 2;
            continue;
        }
        let mut j = i + 1;
        let Some(spec) = parse_spec(bytes, &mut j) else {
            // Unmodelled spec (e.g. `*` width) — emit the `%` literally.
            out.push('%');
            i += 1;
            continue;
        };
        let arg = args.get(ai);
        if spec.verb != b'%' {
            let Some(arg) = arg else {
                return Err("not enough arguments for all format specifiers".to_string());
            };
            out.push_str(&render_spec(&spec, arg)?);
            ai += 1;
        }
        i = j;
    }
    Ok(out)
}

fn utf8_len(b: u8) -> usize {
    match b {
        0x00..=0x7f => 1,
        0xc0..=0xdf => 2,
        0xe0..=0xef => 3,
        _ => 4,
    }
}

/// Render one conversion against `arg`.
fn render_spec(spec: &Spec, arg: &Value) -> Result<String, String> {
    let verb = spec.verb;
    match verb {
        b'd' | b'i' | b'u' => {
            let n = arg
                .as_int()
                .map_err(|_| format!("expected integer but got \"{}\"", arg.to_str()))?;
            Ok(pad_number(
                &int_digits(n, spec),
                n < 0 && verb != b'u',
                spec,
            ))
        }
        b'x' | b'X' | b'o' | b'b' => {
            let n = arg
                .as_int()
                .map_err(|_| format!("expected integer but got \"{}\"", arg.to_str()))?;
            Ok(pad_number(&based_digits(n, spec), false, spec))
        }
        b'c' => {
            let n = arg
                .as_int()
                .map_err(|_| format!("expected integer but got \"{}\"", arg.to_str()))?;
            let ch = u32::try_from(n)
                .ok()
                .and_then(char::from_u32)
                .map_or_else(String::new, |c| c.to_string());
            Ok(justify(&ch, spec))
        }
        b'f' | b'e' | b'E' | b'g' | b'G' => {
            let x = arg.as_double().map_err(|_| {
                format!(
                    "expected floating-point number but got \"{}\"",
                    arg.to_str()
                )
            })?;
            Ok(pad_number(
                &float_digits(x, spec),
                x.is_sign_negative(),
                spec,
            ))
        }
        b's' => {
            let mut s = arg.to_str().to_string();
            if let Some(p) = spec.precision {
                s = s.chars().take(p).collect();
            }
            Ok(justify(&s, spec))
        }
        other => Err(format!("bad field specifier \"{}\"", char::from(other))),
    }
}

/// Decimal digits (no sign) for an integer, honouring `.precision` as the
/// minimum digit count.
fn int_digits(n: i64, spec: &Spec) -> String {
    let mag = n.unsigned_abs().to_string();
    apply_precision(mag, spec)
}

/// Digits for `x`/`X`/`o`/`b`, with the `#` alternate-form prefix.
fn based_digits(n: i64, spec: &Spec) -> String {
    #[allow(clippy::cast_sign_loss)]
    let u = n as u64;
    let (mut body, prefix) = match spec.verb {
        b'x' => (
            format!("{u:x}"),
            if spec.flags.contains(FmtFlags::HASH) && u != 0 {
                "0x"
            } else {
                ""
            },
        ),
        b'X' => (
            format!("{u:X}"),
            if spec.flags.contains(FmtFlags::HASH) && u != 0 {
                "0X"
            } else {
                ""
            },
        ),
        b'o' => (
            format!("{u:o}"),
            if spec.flags.contains(FmtFlags::HASH) {
                "0"
            } else {
                ""
            },
        ),
        _ => (format!("{u:b}"), ""),
    };
    body = apply_precision(body, spec);
    format!("{prefix}{body}")
}

/// Magnitude digits for a float verb (sign handled by `pad_number`).
fn float_digits(x: f64, spec: &Spec) -> String {
    let prec = spec.precision.unwrap_or(6);
    let m = x.abs();
    match spec.verb {
        b'f' => format!("{m:.prec$}"),
        b'e' => format!("{m:.prec$e}"),
        b'E' => format!("{m:.prec$E}"),
        // g/G: shortest of %e/%f with `prec` significant digits; approximate
        // with Rust's default float formatting bounded by precision.
        _ => {
            let s = format!("{m:.prec$}");
            // Trim trailing zeros / dot for %g semantics.
            if s.contains('.') {
                s.trim_end_matches('0').trim_end_matches('.').to_string()
            } else {
                s
            }
        }
    }
}

/// Left-pad `mag` with `0` up to `.precision` digits.
fn apply_precision(mag: String, spec: &Spec) -> String {
    match spec.precision {
        Some(p) if mag.len() < p => format!("{}{mag}", "0".repeat(p - mag.len())),
        // Precision 0 with a zero value prints no digits.
        Some(0) if mag == "0" => String::new(),
        _ => mag,
    }
}

/// Apply the sign prefix (`-`/`+`/space) and width to a numeric body.
fn pad_number(body: &str, negative: bool, spec: &Spec) -> String {
    let sign = if negative {
        "-"
    } else if spec.flags.contains(FmtFlags::PLUS) {
        "+"
    } else if spec.flags.contains(FmtFlags::SPACE) {
        " "
    } else {
        ""
    };
    let Some(width) = spec.width else {
        return format!("{sign}{body}");
    };
    let len = sign.len() + body.len();
    if len >= width {
        return format!("{sign}{body}");
    }
    let pad = width - len;
    if spec.flags.contains(FmtFlags::MINUS) {
        format!("{sign}{body}{}", " ".repeat(pad))
    } else if spec.flags.contains(FmtFlags::ZERO) && spec.precision.is_none() {
        // Zero-pad goes between the sign and the digits.
        format!("{sign}{}{body}", "0".repeat(pad))
    } else {
        format!("{}{sign}{body}", " ".repeat(pad))
    }
}

/// Apply width / left-justify to a string conversion.
fn justify(s: &str, spec: &Spec) -> String {
    let Some(width) = spec.width else {
        return s.to_string();
    };
    let len = s.chars().count();
    if len >= width {
        return s.to_string();
    }
    let pad = " ".repeat(width - len);
    if spec.flags.contains(FmtFlags::MINUS) {
        format!("{s}{pad}")
    } else {
        format!("{pad}{s}")
    }
}

#[cfg(test)]
mod tests {
    use super::render;
    use crate::value::Value;

    fn fmt(f: &str, args: &[Value]) -> String {
        render(f, args).unwrap()
    }

    #[test]
    fn basics() {
        assert_eq!(fmt("%d-%s", &[Value::int(5), Value::string("hi")]), "5-hi");
        assert_eq!(fmt("%05d", &[Value::int(42)]), "00042");
        assert_eq!(fmt("%05d", &[Value::int(-42)]), "-0042");
        assert_eq!(fmt("%+d", &[Value::int(42)]), "+42");
        assert_eq!(fmt("%-5d|", &[Value::int(42)]), "42   |");
        assert_eq!(fmt("%x", &[Value::int(255)]), "ff");
        assert_eq!(fmt("%#x", &[Value::int(255)]), "0xff");
        assert_eq!(fmt("%o", &[Value::int(8)]), "10");
        assert_eq!(fmt("%c", &[Value::int(65)]), "A");
        assert_eq!(fmt("%.2f", &[Value::double(1.23456)]), "1.23");
        assert_eq!(fmt("%8.2f", &[Value::double(1.23456)]), "    1.23");
        assert_eq!(fmt("%.3s", &[Value::string("hello")]), "hel");
        assert_eq!(fmt("100%%", &[]), "100%");
    }
}
