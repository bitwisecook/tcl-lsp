//! Engine-level tests for the forensic file builtins (`files` / `file` /
//! `glob` / `grep`) driven through `run_query` with a stub `files_reader`, so
//! the DSL surface is exercised without a real UCS archive.

use std::rc::Rc;

use tcl_bigip_query::eval::{FileOp, FilesReader};
use tcl_bigip_query::value::Value;
use tcl_bigip_query::{QueryOptions, run_query};

fn sources() -> Vec<(String, String)> {
    vec![("file:///dev.ucs".to_owned(), "ltm pool /Common/p { }\n".to_owned())]
}

fn stub_reader() -> FilesReader {
    Rc::new(|_uri: &str, op: &FileOp| match op {
        FileOp::Inventory => Ok(Value::List(vec![
            Value::object([
                ("path".to_owned(), Value::Str("etc/passwd".to_owned())),
                ("size".to_owned(), Value::Int(60)),
                ("sha256".to_owned(), Value::Str("aa".to_owned())),
                ("is_text".to_owned(), Value::Bool(true)),
            ]),
            Value::object([
                ("path".to_owned(), Value::Str("root/.ssh/authorized_keys".to_owned())),
                ("size".to_owned(), Value::Int(40)),
                ("sha256".to_owned(), Value::Str("bb".to_owned())),
                ("is_text".to_owned(), Value::Bool(true)),
            ]),
        ])),
        FileOp::Content(p) => Ok(match p.as_str() {
            "etc/passwd" => Value::Str(
                "root:x:0:0::/root:/bin/bash\nbackdoor:x:0:0::/root:/bin/bash\n".to_owned(),
            ),
            "root/.ssh/authorized_keys" => Value::Str("ssh-rsa AAAA attacker\n".to_owned()),
            _ => Value::Null,
        }),
    })
}

fn opts() -> QueryOptions {
    QueryOptions {
        files_reader: Some(stub_reader()),
        ..QueryOptions::default()
    }
}

fn run(q: &str) -> String {
    let r = run_query(q, &sources(), &opts()).expect("run");
    let vals = r.values_per_file[0].1.clone();
    tcl_bigip_query::jsonfmt::to_pretty(&Value::List(vals))
}

#[test]
fn files_lists_the_inventory() {
    let out = run("files");
    assert!(
        out.contains("etc/passwd") && out.contains("authorized_keys"),
        "inventory listed: {out}"
    );
}

#[test]
fn glob_filters_by_path() {
    let out = run(r#"glob("root/.ssh/*")"#);
    assert!(out.contains("authorized_keys"), "ssh key matched: {out}");
    assert!(!out.contains("etc/passwd"), "glob must not match passwd: {out}");
}

#[test]
fn file_attaches_content() {
    let out = run(r#"file("etc/passwd") | .content"#);
    assert!(out.contains("backdoor"), "content attached: {out}");
}

#[test]
fn grep_returns_matching_lines_scoped_by_glob() {
    let out = run(r#"grep("backdoor", "etc/passwd")"#);
    assert!(out.contains("\"line\"") && out.contains("backdoor"), "match row: {out}");
    // The path glob scopes grep to passwd, not the authorized_keys file.
    assert!(!out.contains("attacker"), "grep scoped by path glob: {out}");
}

#[test]
fn file_builtins_error_without_a_reader() {
    // Default options have no files_reader → a clear, actionable error.
    let err = run_query("files", &sources(), &QueryOptions::default()).unwrap_err();
    assert!(
        err.to_string().contains("no UCS file reader"),
        "clear unwired error: {err}"
    );
}
