//! The `scan` command + the `format` adapter.
//!
//! `format`'s rendering logic now lives in the shared `tcl_cmd_core::format`
//! (over `ValueOps`); this module keeps `scan` (the inverse parse) and a thin
//! `format` adapter onto the core.

use tcl_runtime_api::Completion;

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
            c.is_ascii_digit()
                || c == '.'
                || (n == 0 && (c == '-' || c == '+'))
                || c == 'e'
                || c == 'E'
        })
        .parse::<f64>()
        .ok()
        .map(Value::double),
        's' => {
            let s = take(ip, &|_, c| !c.is_whitespace());
            if s.is_empty() {
                None
            } else {
                Some(Value::string(s))
            }
        }
        other => return Err(format!("bad scan conversion character \"%{other}\"")),
    };
    Ok(value)
}

fn cmd_format(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    match tcl_cmd_core::format::format_cmd(vm, args) {
        Ok(v) => ok(v),
        Err(e) => err(e.into_message()),
    }
}
