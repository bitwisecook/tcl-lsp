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
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::{CHANGED, CREATED, DELETED, file_uri, workspace, write};
use crate::common::Lsp;
use crate::common::helpers::{decode_semantic_tokens, hover_text, locations};
use serde_json::{Value, json};
use std::path::PathBuf;
use std::time::Duration;

const TIMEOUT: Duration = Duration::from_secs(30);
const ENTRY: &str = "\
rename auto_import savedPrelude
namespace eval ::seed {
    source [file join [file dirname [info script]] provider.tcl]
}
source [file join [file dirname [info script]] consumer.tcl]
namespace eval ::client {
    namespace import ::seed::callback
    proc plain {} {
        callback
    }
    proc invoke {} {
        d4_callback callback
    }
    proc hook {} {
        if {[d4_value 0] == 8} {puts hook}
    }
    proc release {} {
        if {010 == 8} {puts release}
    }
    proc role {} {
        d4_with_var written {set written 1}
    }
    proc sidecar {} {
        d4_scope {set sidecar_missing}
    }
    proc ordinary {} {
        set ordinary 1
        return $ordinary
    }
}
";
const PROVIDER: &str = "\
proc callback {} {return yes}
namespace export callback
oo::class create Widget {
    method run {} {d4_callback ::seed::callback}
}
";
const CONSUMER: &str =
    "proc consume {} {\n    set instance [::seed::Widget new]\n    $instance run\n}\n";
const CONTROL: &str = "\
proc b_hook {} {
    if {[d4_value 0] == 10} {puts hook}
}
proc b_release {} {
    if {010 == 8} {puts release}
}
proc b_role {} {
    d4_with_var written {set written 1}
}
proc b_sidecar {} {
    d4_scope {set sidecar_missing}
}
proc b_ordinary {} {
    set ordinary 1
    return $ordinary
}
";

#[derive(Clone, Copy, Debug)]
enum Phase {
    Initial,
    Changed,
    Removed,
    Restored,
}

impl Phase {
    const fn installed(self) -> bool {
        !matches!(self, Self::Removed)
    }

    const fn body(self) -> bool {
        matches!(self, Self::Initial | Self::Restored)
    }

    const fn dialect(self) -> &'static str {
        if matches!(self, Self::Changed) {
            "tcl9.0"
        } else {
            "tcl8.6"
        }
    }
}

struct Document {
    path: PathBuf,
    text: &'static str,
}

struct Fixture {
    root: PathBuf,
    folder: PathBuf,
    control: PathBuf,
    provider: PathBuf,
    documents: [Document; 3],
}

impl Fixture {
    fn new() -> Self {
        let root = workspace("environment-lifecycle");
        let folder = root.join("A");
        let control = root.join("B");
        let provider = folder.join("provider.tcl");
        let documents = [
            Document {
                path: folder.join("entry.tcl"),
                text: ENTRY,
            },
            Document {
                path: folder.join("consumer.tcl"),
                text: CONSUMER,
            },
            Document {
                path: control.join("control.tcl"),
                text: CONTROL,
            },
        ];
        write(&provider, PROVIDER);
        for document in &documents {
            write(&document.path, document.text);
        }
        write(
            &control.join(".tcl-lsp.ini"),
            "[project]\ndialect = tcl9.0\n",
        );
        write(
            &control.join("tcl9.0.tcl.stubs"),
            "stub d4_scope {script:body}\n",
        );
        let fixture = Self {
            root,
            folder,
            control,
            provider,
            documents,
        };
        drop(fixture.configure(Phase::Initial));
        fixture
    }

    fn configure(&self, phase: Phase) -> Vec<Value> {
        let paths = [
            self.folder.join(".tcl-lsp.ini"),
            self.folder.join(".tcl-lsp/d4.tclspec"),
            self.folder.join("tcl8.6.tcl.stubs"),
            self.folder.join("tcl9.0.tcl.stubs"),
        ];
        let event = match phase {
            Phase::Initial | Phase::Restored => CREATED,
            Phase::Changed => CHANGED,
            Phase::Removed => DELETED,
        };
        if phase.installed() {
            write(
                &paths[0],
                &format!(
                    "[project]\ndialect = {}\nentryPoints = entry.tcl\n",
                    phase.dialect()
                ),
            );
            write(&paths[1], &pack(phase));
            let role = if phase.body() { "body" } else { "value" };
            for path in &paths[2..] {
                write(path, &format!("stub d4_scope {{script:{role}}}\n"));
            }
        } else {
            for path in &paths {
                std::fs::remove_file(path).expect("remove only environment fixture inputs");
            }
        }
        paths
            .iter()
            .map(|path| json!({"uri": file_uri(path), "type": event}))
            .collect()
    }

    fn server(&self) -> Lsp {
        Lsp::multi_root(
            json!({"dialect": "tcl8.6"}),
            &[
                (self.folder.as_path(), json!({})),
                (self.control.as_path(), json!({})),
            ],
        )
    }

    fn open(&self, lsp: &mut Lsp) {
        for document in &self.documents {
            lsp.open_document(&file_uri(&document.path), document.text);
        }
    }

    fn assert_sources_unchanged(&self) {
        for (path, expected) in self
            .documents
            .iter()
            .map(|document| (&document.path, document.text))
            .chain(std::iter::once((&self.provider, PROVIDER)))
        {
            assert_eq!(
                std::fs::read_to_string(path).expect("read fixed source"),
                expected
            );
        }
    }
}

fn pack(phase: Phase) -> String {
    let (role, argument, number) = if phase.body() {
        ("Body", "script", "010")
    } else {
        ("Value", "value", "020")
    };
    format!(
        r#"speclib d4 2.2 {{
    command d4_with_var {{
        arity 2
        arg 0 -role VarWrite
        arg 1 -role {role}
        form Default {{d4_with_var variable {argument}}}
        hover {{summary {{D4 variable {argument}.}}}}
    }}
    command d4_callback {{
        arity 1
        arg 0 -role CommandPrefix -appends {{Exactly 0}}
        script_timing_resolver {{words ctx}} {{timing 0 SameInvocation}}
    }}
    command d4_value {{
        arity 1
        semantics {{
            effects {{no_store_writes no_external_io}}
            result -semantic string
        }}
        evaluate -implementation d4.value.v1 -host bounded_tcl {{
            inputs {{arg 0 exact}}
            depends {{tcl_profile implementation_identity}}
            budget {{-commands 2000 -wall-clock 20 -value-bytes 65536}}
            body {{value}} {{fold [expr {{$value + {number}}}]}}
        }}
    }}
}}
"#
    )
}

fn number(value: &Value, field: &str) -> u64 {
    value[field]
        .as_str()
        .expect("owner reports decimal string")
        .parse()
        .expect("decimal owner generation")
}

fn await_environment(lsp: &mut Lsp, fixture: &Fixture, phase: Phase, old: Option<&Value>) -> Value {
    // Only actual applied configuration/publication is polled, never a naming answer.
    let configs = lsp.await_query_settled(
        TIMEOUT,
        |server| {
            json!([
                server.effective_config(&file_uri(&fixture.documents[0].path)),
                server.effective_config(&file_uri(&fixture.documents[2].path)),
            ])
        },
        |configs| {
            let publication = &configs[0]["spec_pack_publication"];
            configs[0]["dialect_id"] == phase.dialect()
                && configs[1]["dialect_id"] == "tcl9.0"
                && publication["status"] == "complete"
                && configs[1]["spec_pack_publication"]["status"] == "complete"
                && configs[1]["spec_pack_publication"]["registry_key"]
                    == publication["registry_key"]
                && old.is_none_or(|previous| {
                    number(publication, "generation") > number(previous, "generation")
                })
                && configs[0]["spec_packs_loaded"]
                    .as_array()
                    .is_some_and(|packs| {
                        packs.iter().any(|loaded| loaded["name"] == "d4") == phase.installed()
                    })
        },
    );
    let publication = configs[0]["spec_pack_publication"].clone();
    publication
}

fn publication_identity(report: &Value) -> Value {
    let mut identity = report
        .as_object()
        .expect("complete publisher report")
        .clone();
    identity.remove("status");
    identity.remove("diagnostics");
    Value::Object(identity)
}

fn validate_owner(lsp: &mut Lsp, report: &Value, current: bool) {
    let identity = publication_identity(report);
    let validation =
        lsp.execute_command("tcl-lsp.validateDiagnosticPublication", json!([identity]));
    assert_eq!(
        validation["status"],
        if current { "current" } else { "unavailable" },
        "{validation}"
    );
}

fn completed_reports(lsp: &mut Lsp, fixture: &Fixture, phase: Phase, old: &[Value]) -> Vec<Value> {
    fixture
        .documents
        .iter()
        .enumerate()
        .map(|(index, document)| {
            let uri = file_uri(&document.path);
            let report = lsp.await_query_settled(
                TIMEOUT,
                |server| {
                    server.execute_command(
                        "tcl-lsp.currentDiagnostics",
                        json!([{
                            "uri": uri, "version": 1, "source": document.text,
                        }]),
                    )
                },
                |answer| match answer["status"].as_str() {
                    Some("complete") => true,
                    Some("unavailable")
                        if matches!(answer["reason"].as_str(), Some("pending" | "stale")) =>
                    {
                        false
                    }
                    _ => panic!("publisher cannot complete the exact source: {answer}"),
                },
            );
            assert_eq!(report["uri"], uri);
            assert_eq!(report["version"], 1);
            assert_eq!(report["source"], document.text);
            assert_eq!(
                report["dialectId"],
                if index == 2 {
                    "tcl9.0"
                } else {
                    phase.dialect()
                }
            );
            let _ = number(&report, "revision");
            let _ = number(&report, "inputsEpoch");
            assert!(report["resultId"].as_str().is_some_and(|id| !id.is_empty()));
            if let Some(previous) = old.get(index) {
                assert!(number(&report, "inputsEpoch") > number(previous, "inputsEpoch"));
                validate_owner(lsp, previous, false);
            }
            validate_owner(lsp, &report, true);
            report
        })
        .collect()
}

fn position(source: &str, prefix: &str, word: &str) -> (u32, u32) {
    let needle = format!("{prefix}{word}");
    let offset = source.find(&needle).expect("unique fixture position") + prefix.len();
    let before = &source[..offset];
    let line = before.bytes().filter(|&byte| byte == b'\n').count();
    let column = before
        .rsplit_once('\n')
        .map_or(before, |(_, tail)| tail)
        .encode_utf16()
        .count();
    (
        u32::try_from(line).expect("fixture line"),
        u32::try_from(column).expect("fixture column"),
    )
}

fn location_values(result: &Value) -> Value {
    let mut values: Vec<_> = locations(result)
        .into_iter()
        .map(|location| json!({"uri": location.uri, "range": location.range}))
        .collect();
    values.sort_by_key(Value::to_string);
    Value::Array(values)
}

fn token_kind(tokens: &Value, legend: &Value, point: (u32, u32)) -> Value {
    decode_semantic_tokens(tokens)
        .iter()
        .find_map(|token| {
            (token.line == i64::from(point.0)
                && token.char <= i64::from(point.1)
                && i64::from(point.1) < token.char + token.length)
                .then(|| legend[usize::try_from(token.ttype).expect("token type index")].clone())
        })
        .unwrap_or(Value::Null)
}

fn role_tokens(lsp: &mut Lsp, fixture: &Fixture) -> Value {
    let legend =
        lsp.initialize_result()["capabilities"]["semanticTokensProvider"]["legend"]["tokenTypes"]
            .clone();
    let rows: Vec<_> = [0, 2].into_iter().map(|index| {
        let document = &fixture.documents[index];
        let tokens = lsp.semantic_tokens(&file_uri(&document.path));
        json!({
            "writer": token_kind(&tokens, &legend, position(document.text, "d4_with_var ", "written")),
            "body": token_kind(&tokens, &legend, position(document.text, "d4_with_var written {", "set")),
            "sidecar": token_kind(&tokens, &legend, position(document.text, "d4_scope {", "set")),
            "control": token_kind(&tokens, &legend, position(document.text, "", "set ordinary")),
        })
    }).collect();
    Value::Array(rows)
}

fn diagnostic_facts(report: &Value) -> Value {
    let mut findings: Vec<_> = report["diagnostics"]
        .as_array()
        .expect("actual findings array")
        .iter()
        .map(|diagnostic| {
            json!({
                "range": diagnostic["range"], "code": diagnostic["code"],
                "severity": diagnostic["severity"], "message": diagnostic["message"],
                "tags": diagnostic["tags"], "relatedInformation": diagnostic["relatedInformation"],
            })
        })
        .collect();
    findings.sort_by_key(Value::to_string);
    Value::Array(findings)
}

fn first_answers(lsp: &mut Lsp, fixture: &Fixture, reports: &[Value]) -> Value {
    let uri = file_uri(&fixture.documents[0].path);
    let direct = position(ENTRY, "        ", "callback\n");
    let prefix = position(ENTRY, "d4_callback ", "callback");
    let variable = position(ENTRY, "d4_with_var ", "written");
    let method = position(CONSUMER, "$instance ", "run");
    let head = position(ENTRY, "", "d4_with_var");
    // Each naming provider is queried once after independent publisher readiness.
    let definition = lsp.definition(&uri, direct.0, direct.1);
    let callback = lsp.definition(&uri, prefix.0, prefix.1);
    let references = lsp.references(&uri, direct.0, direct.1, true);
    let method_definition =
        lsp.definition(&file_uri(&fixture.documents[1].path), method.0, method.1);
    let signature = lsp.signature_help(&uri, variable.0, variable.1);
    let hover = lsp.hover(&uri, head.0, head.1);
    json!({
        "definition": location_values(&definition),
        "callback": location_values(&callback),
        "references": location_values(&references),
        "method": location_values(&method_definition),
        "signature": signature,
        "hover": hover_text(&hover),
        "roles": role_tokens(lsp, fixture),
        "diagnostics": reports.iter().map(diagnostic_facts).collect::<Vec<_>>(),
    })
}

fn contains_location(locations: &Value, uri: &str, point: (u32, u32)) -> bool {
    locations
        .as_array()
        .expect("normalized locations")
        .iter()
        .any(|location| {
            location["uri"] == uri
                && location["range"]["start"] == json!({"line": point.0, "character": point.1})
        })
}

fn assert_targets(fixture: &Fixture, answer: &Value, phase: Phase) {
    let provider = file_uri(&fixture.provider);
    let uri = file_uri(&fixture.documents[0].path);
    assert_eq!(
        answer["definition"]
            .as_array()
            .expect("definition array")
            .len(),
        1
    );
    assert!(contains_location(
        &answer["definition"],
        &provider,
        position(PROVIDER, "proc ", "callback")
    ));
    assert_eq!(
        answer["method"]
            .as_array()
            .expect("method definition array")
            .len(),
        1
    );
    assert!(contains_location(
        &answer["method"],
        &provider,
        position(PROVIDER, "method ", "run")
    ));
    assert!(contains_location(
        &answer["references"],
        &uri,
        position(ENTRY, "        ", "callback\n")
    ));
    assert_eq!(
        contains_location(
            &answer["callback"],
            &provider,
            position(PROVIDER, "proc ", "callback")
        ),
        phase.installed()
    );
    assert_eq!(
        contains_location(
            &answer["references"],
            &uri,
            position(ENTRY, "d4_callback ", "callback")
        ),
        phase.installed()
    );
    assert_eq!(
        contains_location(
            &answer["references"],
            &provider,
            position(PROVIDER, "d4_callback ", "::seed::callback")
        ),
        phase.installed()
    );
    if phase.installed() {
        let argument = if phase.body() { "script" } else { "value" };
        assert!(
            answer["hover"]
                .as_str()
                .expect("hover text")
                .contains(&format!("D4 variable {argument}."))
        );
        assert!(
            answer["signature"]["signatures"]
                .as_array()
                .is_some_and(|rows| {
                    rows.iter().any(|row| {
                        row["label"]
                            .as_str()
                            .is_some_and(|label| label.contains(argument))
                    })
                })
        );
    } else {
        assert!(
            !answer["hover"]
                .as_str()
                .expect("hover text")
                .contains("D4 variable")
        );
        assert!(
            answer["signature"].is_null()
                || answer["signature"]["signatures"]
                    .as_array()
                    .is_some_and(Vec::is_empty)
        );
        assert_eq!(
            answer["callback"]
                .as_array()
                .expect("callback definitions")
                .as_slice(),
            &[] as &[serde_json::Value]
        );
    }
    assert_roles(answer, phase);
}

fn assert_roles(answer: &Value, phase: Phase) {
    for row in answer["roles"].as_array().expect("role rows") {
        assert_eq!(
            row["control"], "function",
            "the ordinary Tcl control must survive every phase"
        );
        assert_eq!(row["writer"] == "variable", phase.installed());
        assert_eq!(row["body"] == "function", phase.body());
    }
    assert_eq!(answer["roles"][0]["sidecar"] == "function", phase.body());
    assert_eq!(
        answer["roles"][1]["sidecar"], "function",
        "B keeps its own sidecar"
    );
}

fn assert_condition(report: &Value, source: &str, prefix: &str, expected: Option<bool>) {
    let line = position(source, "", prefix).0;
    let conditions: Vec<_> = report["diagnostics"]
        .as_array()
        .expect("actual diagnostics")
        .iter()
        .filter(|diagnostic| {
            diagnostic["code"] == "I230" && diagnostic["range"]["start"]["line"] == line
        })
        .collect();
    if let Some(value) = expected {
        assert_eq!(conditions.len(), 1, "{report}");
        assert!(
            conditions[0]["message"]
                .as_str()
                .expect("condition message")
                .contains(if value {
                    "is always true"
                } else {
                    "is always false"
                }),
            "{report}"
        );
    } else {
        assert!(
            conditions.is_empty(),
            "removed hooks cannot retain a constant answer: {report}"
        );
    }
}

fn assert_reports(reports: &[Value], phase: Phase) {
    for (index, source) in [(0, ENTRY), (2, CONTROL)] {
        assert_condition(
            &reports[index],
            source,
            "if {[d4_value 0]",
            phase.installed().then_some(phase.body()),
        );
        assert_condition(
            &reports[index],
            source,
            "if {010 == 8}",
            Some(index == 0 && phase.dialect() == "tcl8.6"),
        );
        let line = position(source, "d4_scope {set ", "sidecar_missing").0;
        let missing = reports[index]["diagnostics"]
            .as_array()
            .expect("findings")
            .iter()
            .any(|diagnostic| {
                diagnostic["code"] == "W210" && diagnostic["range"]["start"]["line"] == line
            });
        assert_eq!(
            missing,
            index == 2 || phase.body(),
            "sidecar traversal: {}",
            reports[index]
        );
    }
}

fn check_owners(lsp: &mut Lsp, reports: &[Value], previous: &[Value]) {
    for report in reports {
        validate_owner(lsp, report, true);
    }
    for report in previous {
        validate_owner(lsp, report, false);
    }
}

#[test]
fn unchanged_documents_follow_folder_pack_and_stub_generations() {
    let fixture = Fixture::new();
    let mut warm = fixture.server();
    let mut publication = await_environment(&mut warm, &fixture, Phase::Initial, None);
    warm.await_log(&["workspace_folders_scan"], TIMEOUT, 0);
    fixture.open(&mut warm);
    let mut previous = Vec::new();
    let mut retired = Vec::new();
    let mut initial_key = Value::Null;
    let mut initial_answer = Value::Null;
    for phase in [
        Phase::Initial,
        Phase::Changed,
        Phase::Removed,
        Phase::Restored,
    ] {
        if !matches!(phase, Phase::Initial) {
            retired.extend(previous.iter().cloned());
            let changes = fixture.configure(phase);
            warm.notify(
                "workspace/didChangeWatchedFiles",
                json!({"changes": changes}),
            );
            publication = await_environment(&mut warm, &fixture, phase, Some(&publication));
        }
        fixture.assert_sources_unchanged();
        let reports = completed_reports(&mut warm, &fixture, phase, &previous);
        assert_reports(&reports, phase);
        let answer = first_answers(&mut warm, &fixture, &reports);
        assert_targets(&fixture, &answer, phase);
        check_owners(&mut warm, &reports, &retired);

        let mut fresh = fixture.server();
        let _ = await_environment(&mut fresh, &fixture, phase, None);
        fresh.await_log(&["workspace_folders_scan"], TIMEOUT, 0);
        fixture.open(&mut fresh);
        let fresh_reports = completed_reports(&mut fresh, &fixture, phase, &[]);
        assert_reports(&fresh_reports, phase);
        let fresh_answer = first_answers(&mut fresh, &fixture, &fresh_reports);
        assert_targets(&fixture, &fresh_answer, phase);
        assert_eq!(
            answer, fresh_answer,
            "first warm/fresh answers diverged in {phase:?}"
        );
        check_owners(&mut fresh, &fresh_reports, &[]);

        // A second fixed read checks stability; it never waits for a repaired answer.
        assert_eq!(first_answers(&mut warm, &fixture, &reports), answer);
        check_owners(&mut warm, &reports, &retired);
        if matches!(phase, Phase::Initial) {
            initial_key = publication["registry_key"].clone();
            initial_answer = answer;
        } else if matches!(phase, Phase::Restored) {
            assert_eq!(publication["registry_key"], initial_key);
            assert_eq!(answer, initial_answer);
        }
        previous = reports;
    }
    fixture.assert_sources_unchanged();
    drop(warm);
    std::fs::remove_dir_all(&fixture.root).expect("remove fixture after both servers exit");
}
