//! `regexp` and `regsub`, backed by the `regex` crate.
//!
//! Tcl's ARE syntax is close enough to `regex`'s for the common patterns used
//! by library scripts and tests; full ARE parity (e.g. `\m`/`\M` word edges,
//! `[[:<:]]`) is out of scope here. Supports the widely-used switches
//! (`-nocase`, `-all`, `-inline`, `-line`, `--`) and writes match variables.

use regex::Regex;
use tcl_runtime_api::Completion;

use crate::interp::{Vm, err, ok};
use crate::value::Value;

pub(crate) fn register(vm: &mut Vm) {
    vm.register("regexp", cmd_regexp);
    vm.register("regsub", cmd_regsub);
}

/// Parsed leading `-switch` flags common to `regexp`/`regsub`.
#[derive(Default)]
#[allow(clippy::struct_excessive_bools)]
struct Flags {
    nocase: bool,
    all: bool,
    inline: bool,
    line: bool,
}

/// Consume leading `-switch` args; returns `(flags, remaining)`.
fn parse_flags(mut args: &[Value]) -> (Flags, &[Value]) {
    let mut f = Flags::default();
    while let Some(first) = args.first() {
        match &*first.to_str() {
            "-nocase" => f.nocase = true,
            "-all" => f.all = true,
            "-inline" => f.inline = true,
            "-line" | "-lineanchor" | "-linestop" => f.line = true,
            "-expanded" | "-about" | "-indices" => {} // accepted, ignored
            "-start" => {
                args = &args[1..]; // skip the index argument too
            }
            "--" => {
                args = &args[1..];
                break;
            }
            s if s.starts_with('-') => {} // ignore unknown switches
            _ => break,
        }
        args = &args[1..];
    }
    (f, args)
}

/// Compile a Tcl regular expression with the given flags.
fn compile(pat: &str, f: &Flags) -> Result<Regex, Completion<Value>> {
    let mut prefix = String::new();
    if f.nocase {
        prefix.push_str("(?i)");
    }
    if f.line {
        prefix.push_str("(?m)");
    }
    Regex::new(&format!("{prefix}{pat}"))
        .map_err(|e| err(format!("couldn't compile regular expression pattern: {e}")))
}

fn cmd_regexp(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let (flags, rest) = parse_flags(args);
    let Some((pat, rest)) = rest.split_first() else {
        return err(
            "wrong # args: should be \"regexp ?switches? exp string ?matchVar? ?subMatchVar ...?\"",
        );
    };
    let Some((subject, match_vars)) = rest.split_first() else {
        return err(
            "wrong # args: should be \"regexp ?switches? exp string ?matchVar? ?subMatchVar ...?\"",
        );
    };
    let re = match compile(&pat.to_str(), &flags) {
        Ok(r) => r,
        Err(c) => return c,
    };
    let text = subject.to_str();

    if flags.inline {
        // Return the match (and submatches) as a list, or all matches with -all.
        let mut out: Vec<Value> = Vec::new();
        for caps in re.captures_iter(&text) {
            for i in 0..caps.len() {
                out.push(Value::string(caps.get(i).map_or("", |m| m.as_str())));
            }
            if !flags.all {
                break;
            }
        }
        return ok(Value::list(out));
    }

    if flags.all {
        let count = re.find_iter(&text).count();
        return ok(Value::int(i64::try_from(count).unwrap_or(i64::MAX)));
    }

    if let Some(caps) = re.captures(&text) {
        for (i, var) in match_vars.iter().enumerate() {
            let m = caps.get(i).map_or("", |m| m.as_str());
            if let Err(c) = vm.var_set(&var.to_str(), Value::string(m)) {
                return c;
            }
        }
        ok(Value::int(1))
    } else {
        // No match: matchVars are set to empty strings.
        for var in match_vars {
            if let Err(c) = vm.var_set(&var.to_str(), Value::empty()) {
                return c;
            }
        }
        ok(Value::int(0))
    }
}

fn cmd_regsub(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let (flags, rest) = parse_flags(args);
    let [pat, subject, sub, var_opt @ ..] = rest else {
        return err("wrong # args: should be \"regsub ?switches? exp string subSpec ?varName?\"");
    };
    let re = match compile(&pat.to_str(), &flags) {
        Ok(r) => r,
        Err(c) => return c,
    };
    let text = subject.to_str();
    let spec = sub.to_str();

    let mut count = 0usize;
    let replacer = |caps: &regex::Captures| {
        count += 1;
        expand_subspec(&spec, caps)
    };
    let result = if flags.all {
        re.replace_all(&text, replacer).into_owned()
    } else {
        re.replace(&text, replacer).into_owned()
    };

    match var_opt {
        [var] => {
            if let Err(c) = vm.var_set(&var.to_str(), Value::string(result)) {
                return c;
            }
            ok(Value::int(i64::try_from(count).unwrap_or(i64::MAX)))
        }
        [] => ok(Value::string(result)),
        _ => err("wrong # args: should be \"regsub ?switches? exp string subSpec ?varName?\""),
    }
}

/// Expand a Tcl `regsub` replacement spec: `&`/`\0` = whole match, `\1`..`\9` =
/// submatches, `\\` = literal backslash.
fn expand_subspec(spec: &str, caps: &regex::Captures) -> String {
    let group = |n: usize| caps.get(n).map_or("", |m| m.as_str());
    let mut out = String::with_capacity(spec.len());
    let mut chars = spec.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '&' => out.push_str(group(0)),
            '\\' => match chars.next() {
                Some(d @ '0'..='9') => out.push_str(group(d as usize - '0' as usize)),
                Some(other) => out.push(other),
                None => out.push('\\'),
            },
            other => out.push(other),
        }
    }
    out
}
