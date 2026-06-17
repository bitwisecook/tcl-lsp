//! Filesystem commands: `pwd`, `cd`, `file`, `glob`.
//!
//! Path manipulation subcommands (`join`/`dirname`/`tail`/…) are pure string
//! operations; the query/mutation subcommands (`exists`/`mtime`/`mkdir`/…) and
//! `pwd`/`cd`/`glob` touch the real filesystem (POSIX layout — the test host).

use std::path::{Path, PathBuf};

use tcl_platform::Filesystem;
use tcl_runtime_api::Completion;

use crate::interp::{Vm, err, ok};
use crate::value::Value;

pub(crate) fn register(vm: &mut Vm) {
    vm.register("pwd", cmd_pwd);
    vm.register("cd", cmd_cd);
    vm.register("file", cmd_file);
    vm.register("glob", cmd_glob);
}

fn cmd_pwd(vm: &mut Vm, _args: &[Value]) -> Completion<Value> {
    match vm.host().env().cwd() {
        Ok(p) => ok(Value::string(p)),
        Err(e) => err(format!("error getting working directory name: {e}")),
    }
}

fn cmd_cd(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let env = vm.host().env();
    let dir = match args {
        [] => env.get("HOME").unwrap_or_else(|| "/".to_string()),
        [d] => d.to_str().to_string(),
        _ => return err("wrong # args: should be \"cd ?dirName?\""),
    };
    match env.chdir(&dir) {
        Ok(()) => ok(Value::empty()),
        Err(e) => err(format!(
            "couldn't change working directory to \"{dir}\": {e}"
        )),
    }
}

#[allow(clippy::too_many_lines)]
fn cmd_file(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some((sub, rest)) = args.split_first() else {
        return err("wrong # args: should be \"file subcommand ?arg ...?\"");
    };
    let s = |v: &Value| v.to_str().to_string();
    match &*sub.to_str() {
        // -- pure path manipulation --
        "join" => ok(Value::string(file_join(rest))),
        "dirname" => match rest {
            [p] => ok(Value::string(dirname(&s(p)))),
            _ => err("wrong # args: should be \"file dirname name\""),
        },
        "tail" => match rest {
            [p] => ok(Value::string(tail(&s(p)))),
            _ => err("wrong # args: should be \"file tail name\""),
        },
        "extension" => match rest {
            [p] => ok(Value::string(extension(&s(p)))),
            _ => err("wrong # args: should be \"file extension name\""),
        },
        "rootname" => match rest {
            [p] => {
                let t = s(p);
                let ext = extension(&t);
                ok(Value::string(t[..t.len() - ext.len()].to_string()))
            }
            _ => err("wrong # args: should be \"file rootname name\""),
        },
        "split" => match rest {
            [p] => ok(Value::list(
                split_path(&s(p)).into_iter().map(Value::string).collect(),
            )),
            _ => err("wrong # args: should be \"file split name\""),
        },
        "normalize" => match rest {
            [p] => {
                let cwd = vm.host().env().cwd().unwrap_or_else(|_| "/".to_string());
                ok(Value::string(normalize(&s(p), &cwd)))
            }
            _ => err("wrong # args: should be \"file normalize name\""),
        },
        "nativename" => match rest {
            [p] => ok(Value::string(s(p))),
            _ => err("wrong # args: should be \"file nativename name\""),
        },
        "pathtype" => match rest {
            [p] => ok(Value::string(
                if Path::new(&s(p)).is_absolute() {
                    "absolute"
                } else {
                    "relative"
                }
                .to_string(),
            )),
            _ => err("wrong # args: should be \"file pathtype name\""),
        },
        "separator" => ok(Value::string("/")),
        // -- filesystem queries --
        // `readable`/`writable`/`executable` only check existence (good enough
        // for the test host where files are owned by the runner).
        "exists" | "readable" | "writable" | "executable" => {
            bool_query(vm.host().filesystem(), rest, |fs, p| fs.exists(p))
        }
        "isdirectory" | "isdir" => bool_query(vm.host().filesystem(), rest, |fs, p| {
            fs.metadata(p).is_ok_and(|m| m.is_dir)
        }),
        "isfile" => bool_query(vm.host().filesystem(), rest, |fs, p| {
            fs.metadata(p).is_ok_and(|m| m.is_file)
        }),
        "size" => match rest {
            [p] => match vm.host().filesystem().and_then(|fs| fs.metadata(&s(p)).ok()) {
                Some(m) => ok(Value::int(i64::try_from(m.len).unwrap_or(i64::MAX))),
                None => err(format!("could not read \"{}\": no such file", s(p))),
            },
            _ => err("wrong # args: should be \"file size name\""),
        },
        "mtime" => match rest {
            [p] => file_mtime(vm.host().filesystem(), &s(p)),
            _ => err("wrong # args: should be \"file mtime name ?time?\""),
        },
        // -- filesystem mutation --
        "mkdir" => {
            for p in rest {
                if let Err(e) = std::fs::create_dir_all(s(p)) {
                    return err(format!("can't create directory \"{}\": {e}", s(p)));
                }
            }
            ok(Value::empty())
        }
        "delete" => {
            // `file delete ?-force? ?--? name ...`
            let mut paths = rest;
            while let Some(first) = paths.first() {
                match &*first.to_str() {
                    "-force" | "--" => paths = &paths[1..],
                    _ => break,
                }
            }
            for p in paths {
                let path = s(p);
                let pp = Path::new(&path);
                let _ = if pp.is_dir() {
                    std::fs::remove_dir_all(pp)
                } else {
                    std::fs::remove_file(pp)
                };
            }
            ok(Value::empty())
        }
        other => err(format!("unknown or ambiguous subcommand \"{other}\"")),
    }
}

/// A boolean `file` query through the host filesystem. A host without a
/// filesystem (`None`) answers `false` — nothing exists where there is no fs.
fn bool_query(
    fs: Option<&dyn Filesystem>,
    rest: &[Value],
    pred: impl Fn(&dyn Filesystem, &str) -> bool,
) -> Completion<Value> {
    match rest {
        [p] => ok(Value::bool(fs.is_some_and(|fs| pred(fs, &p.to_str())))),
        _ => err("wrong # args: should be \"file <op> name\""),
    }
}

fn file_mtime(fs: Option<&dyn Filesystem>, path: &str) -> Completion<Value> {
    match fs.and_then(|fs| fs.metadata(path).ok()) {
        Some(m) => ok(Value::int(m.mtime_secs)),
        None => err(format!(
            "could not read \"{path}\": no such file or directory"
        )),
    }
}

/// `file join a b c` — join components, an absolute component resets the path.
fn file_join(parts: &[Value]) -> String {
    let mut buf = PathBuf::new();
    for p in parts {
        let s = p.to_str();
        if s.starts_with('/') {
            buf = PathBuf::from(&*s);
        } else if !s.is_empty() {
            buf.push(&*s);
        }
    }
    buf.to_string_lossy().into_owned()
}

fn dirname(p: &str) -> String {
    Path::new(p).parent().map_or_else(
        || ".".to_string(),
        |d| {
            let s = d.to_string_lossy();
            if s.is_empty() {
                ".".to_string()
            } else {
                s.into_owned()
            }
        },
    )
}

fn tail(p: &str) -> String {
    Path::new(p)
        .file_name()
        .map_or_else(|| p.to_string(), |n| n.to_string_lossy().into_owned())
}

fn extension(p: &str) -> String {
    let t = tail(p);
    t.rfind('.')
        .filter(|&i| i > 0)
        .map_or_else(String::new, |i| t[i..].to_string())
}

fn split_path(p: &str) -> Vec<String> {
    let mut out = Vec::new();
    if p.starts_with('/') {
        out.push("/".to_string());
    }
    for c in p.split('/').filter(|c| !c.is_empty()) {
        out.push(c.to_string());
    }
    out
}

/// `file normalize` — make absolute (against the cwd) and resolve `.`/`..`
/// lexically (no symlink resolution).
fn normalize(p: &str, cwd: &str) -> String {
    let base = if Path::new(p).is_absolute() {
        PathBuf::from(p)
    } else {
        PathBuf::from(cwd).join(p)
    };
    let mut parts: Vec<String> = Vec::new();
    for c in base.components() {
        match c {
            std::path::Component::ParentDir => {
                parts.pop();
            }
            std::path::Component::CurDir => {}
            std::path::Component::RootDir => parts.clear(),
            other => parts.push(other.as_os_str().to_string_lossy().into_owned()),
        }
    }
    format!("/{}", parts.join("/"))
}

/// `glob ?-nocomplain? ?-directory dir? ?--? pattern ...` — minimal matching
/// against the directory's entries.
fn cmd_glob(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    // We never error on no match (effectively always `-nocomplain`).
    let mut dir: Option<String> = None;
    let mut i = 0;
    while i < args.len() {
        match &*args[i].to_str() {
            "-nocomplain" => i += 1,
            "-directory" | "-dir" => {
                dir = args.get(i + 1).map(|v| v.to_str().to_string());
                i += 2;
            }
            "--" => {
                i += 1;
                break;
            }
            s if s.starts_with('-') => i += 1, // ignore other options
            _ => break,
        }
    }
    let patterns = &args[i..];
    let base = dir.clone().unwrap_or_else(|| ".".to_string());
    let mut results: Vec<String> = Vec::new();
    let entries = vm.host().filesystem().and_then(|fs| fs.read_dir(&base).ok());
    if let Some(entries) = entries {
        for name in entries {
            let matches = patterns.is_empty()
                || patterns
                    .iter()
                    .any(|p| tcl_syntax::glob::string_match(&p.to_str(), &name));
            if matches {
                let full = if dir.is_some() {
                    file_join(&[Value::string(base.clone()), Value::string(name.clone())])
                } else {
                    name
                };
                results.push(full);
            }
        }
    }
    results.sort();
    ok(Value::list(
        results.into_iter().map(Value::string).collect(),
    ))
}
