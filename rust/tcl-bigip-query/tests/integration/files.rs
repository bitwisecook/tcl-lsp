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

//! Engine-level tests for the forensic file builtins (`files` / `file` /
//! `glob` / `grep`) driven through `run_query` with a stub `files_reader`, so
//! the DSL surface is exercised without a real UCS archive.

use std::rc::Rc;

use tcl_bigip_query::eval::{FileOp, FilesReader};
use tcl_bigip_query::value::Value;
use tcl_bigip_query::{QueryOptions, run_query};

fn sources() -> Vec<(String, String)> {
    vec![(
        "file:///dev.ucs".to_owned(),
        "ltm pool /Common/p { }\n".to_owned(),
    )]
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
                (
                    "path".to_owned(),
                    Value::Str("root/.ssh/authorized_keys".to_owned()),
                ),
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
        FileOp::Exact(path) => {
            let (content, size, text) = match path.as_str() {
                "etc/passwd" => (
                    "root:x:0:0::/root:/bin/bash\nbackdoor:x:0:0::/root:/bin/bash\n",
                    60,
                    true,
                ),
                "root/.ssh/authorized_keys" => ("ssh-rsa AAAA attacker\n", 40, true),
                "etc/donor" => ("new\u{fffd}", 4, false),
                _ => return Ok(Value::Null),
            };
            Ok(Value::object([
                ("path".to_owned(), Value::Str(path.clone())),
                ("size".to_owned(), Value::Int(size)),
                ("sha256".to_owned(), Value::Str("cc".to_owned())),
                ("is_text".to_owned(), Value::Bool(text)),
                ("content".to_owned(), Value::Str(content.to_owned())),
            ]))
        }
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
    assert!(
        !out.contains("etc/passwd"),
        "glob must not match passwd: {out}"
    );
}

#[test]
fn file_attaches_content() {
    let out = run(r#"file("etc/passwd") | .content"#);
    assert!(out.contains("backdoor"), "content attached: {out}");
}

#[test]
fn file_reads_explicit_lossy_members_without_expanding_discovery() {
    let out = run(r#"file("etc/donor") | .content"#);
    assert!(out.contains("new\u{fffd}"), "{out}");
    assert!(!run("files").contains("etc/donor"));
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&run(r#"file("donor")"#)).unwrap(),
        serde_json::json!([null])
    );
}

#[test]
fn grep_returns_matching_lines_scoped_by_glob() {
    let out = run(r#"grep("backdoor", "etc/passwd")"#);
    assert!(
        out.contains("\"line\"") && out.contains("backdoor"),
        "match row: {out}"
    );
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
