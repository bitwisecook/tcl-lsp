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

//! Item 93's probe of the compiler's original, positioned binding.

use std::collections::BTreeMap;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use tcl_compiler::analyser::{Analyser, AnalysisResult, ResolvedAnalysisInput};
use tcl_compiler::command_binding::{
    BindingKind, SourceInvocationBinding, SourceRuntimeReachability,
};
use tcl_compiler::compilation_unit::{CompilationUnit, UnitBuildOptions};
use tcl_dialect::TclVersion;
use tcl_lexer::{LexerConfig, LineIndex};
use tcl_registry::CommandRegistry;
use tcl_test_support::{Tclsh, required_tclshs, run_script_file};

const PROBE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../docs/work/name-handling/handover/oo-dispatch-probe"
);
const MISSING_COMMAND: &str = "1:invalid command name \"helper\"";

struct Marker<'a> {
    head: &'a str,
    command: u32,
    cursor: u32,
}

// The frame fixtures put a single space-delimited call before the marker.
// For `my b` the editor queries the selector and binding queries the head.
fn marker(source: &str) -> Marker<'_> {
    let (prefix, suffix) = source
        .split_once(";# CALL")
        .expect("fixture has a CALL marker");
    assert!(!suffix.contains(";# CALL"), "one marker per fixture");
    let call = prefix.lines().last().expect("marker has a call").trim_end();
    let word = call
        .split_ascii_whitespace()
        .last()
        .expect("call has a word");
    let cursor = prefix.trim_end().len() - word.len();
    let (head, command) = if call.ends_with("my b") {
        ("my", cursor - "my ".len())
    } else {
        (word, cursor)
    };
    Marker {
        head,
        command: u32::try_from(command).expect("fixture offset fits u32"),
        cursor: u32::try_from(cursor).expect("fixture offset fits u32"),
    }
}

fn command_records(
    source: &str,
    input: &ResolvedAnalysisInput,
    registry: &CommandRegistry,
    config: LexerConfig,
) -> BTreeMap<u32, SourceInvocationBinding> {
    let unit = CompilationUnit::build_with_analysis_input(
        source,
        UnitBuildOptions {
            registry,
            defer_top_level: false,
            config,
            dialect: Some(input.unit_profile()),
            external_call_sites: None,
            declared_commands: None,
        },
        None,
        input,
    );
    let module = &unit.ir_module;
    let bindings = module
        .retained_source_bindings
        .as_ref()
        .and_then(|owner| owner.positioned_for_module(module, registry))
        .expect("compiler retained the original source binding");
    // Enumerate exact byte positions in these short fixtures. The binding
    // owner determines which positions reached dispatch.
    source
        .char_indices()
        .map(|(offset, _)| u32::try_from(offset).expect("fixture offset fits u32"))
        .filter(|&offset| bindings.reached_site(offset))
        .filter_map(|offset| {
            // The positioned query reads the original head at this offset.
            let record = bindings.invocation_at_source("", offset);
            (record.runtime_reachability() == SourceRuntimeReachability::Reached)
                .then_some((offset, record))
        })
        .collect()
}

#[derive(Debug, PartialEq, Eq)]
enum Answer {
    Procedure(String),
    Method(String, String),
    Absent,
}

impl Answer {
    fn from_oracle(stdout: &str) -> Self {
        match stdout {
            "B" => Self::Method("::B".into(), "m".into()),
            "C.b" => Self::Method("::C".into(), "b".into()),
            MISSING_COMMAND => Self::Absent,
            name if name.starts_with("::") && !name.contains(char::is_whitespace) => {
                Self::Procedure(name.into())
            }
            unexpected => panic!("unrecognised frame witness {unexpected:?}"),
        }
    }

    fn declaration_offset(&self, analysis: &AnalysisResult) -> Option<u32> {
        match self {
            Self::Procedure(name) => Some(analysis.all_procs.get(name)?.name_span.start()),
            Self::Method(class, name) => Some(
                analysis
                    .all_classes
                    .get(class)?
                    .methods
                    .get(name)?
                    .name_span
                    .start(),
            ),
            Self::Absent => None,
        }
    }

    fn matches_hover(&self, hover: &str) -> bool {
        match self {
            Self::Procedure(name) => hover.starts_with(&format!("proc {name} ")),
            Self::Method(class, name) => {
                hover.starts_with(&format!("**method** `{class}::{name}`"))
            }
            Self::Absent => false,
        }
    }
}

fn binding_answer(record: &SourceInvocationBinding, registry: &CommandRegistry) -> Option<Answer> {
    if let Some((_, method, _)) = record.receiver_self_method_entry(registry) {
        let owner = method.declaring_class()?;
        return Some(Answer::Method(owner.command.clone(), method.name().into()));
    }
    if let Some(target) = record.proved_target()
        && target.kind == BindingKind::Proc
    {
        return Some(Answer::Procedure(target.command.clone()));
    }
    (record.may_be_absent && !record.unknown && record.targets.is_empty()).then_some(Answer::Absent)
}

// The archived probe stored definition lines and diagnostic codes. The fresh
// consumers below also use line numbers to compare with the same archive.
fn archived_match(
    archive: &str,
    file: &str,
    dialect: &str,
    line: Option<u32>,
    expected: &Answer,
) -> Option<bool> {
    let key = format!("FRAME {file} {dialect} ");
    let row = archive.lines().find_map(|row| row.strip_prefix(&key))?;
    let (_, definition) = row.split_once(" | def=").expect("archived definition");
    let (definition, hover) = definition.split_once(" | hover=").expect("archived hover");
    let (hover, diagnostics) = hover.rsplit_once(" | diag=").expect("archived diagnostics");
    let definitions: Vec<String> = serde_json::from_str(definition).expect("definition array");
    let diagnostics: Vec<String> = serde_json::from_str(diagnostics).expect("diagnostic array");
    Some(if *expected == Answer::Absent {
        definitions.is_empty() && diagnostics.iter().any(|code| code == "W123")
    } else {
        expected.matches_hover(hover)
            || line.is_some_and(|line| {
                definitions.len() == 1 && definitions[0].starts_with(&format!("L{line}:"))
            })
    })
}

struct ConsumerAnswers {
    lines: Vec<u32>,
    hover: Option<String>,
    diagnostics: Vec<&'static str>,
}

fn consumer_answers(
    source: &str,
    cursor: u32,
    index: &LineIndex,
    analysis: &AnalysisResult,
    registry: &CommandRegistry,
) -> ConsumerAnswers {
    let pos = index.position_at_utf16(cursor, source);
    let character = pos.character.get();
    let lines = tcl_lsp_core::definition::definition(source, pos.line, character, analysis)
        .into_iter()
        .map(|range| range.start_line + 1)
        .collect();
    let hover = tcl_lsp_core::hover::hover(source, pos.line, character, analysis, Some(registry))
        .and_then(|hover| {
            hover
                .value
                .lines()
                .find(|line| !line.is_empty() && !line.starts_with("```"))
                .map(str::to_owned)
        });
    let diagnostics = analysis
        .diagnostics
        .iter()
        .filter(|diag| index.position_at_utf16(diag.span.start(), source).line == pos.line)
        .map(|diag| diag.code.as_str())
        .collect();
    ConsumerAnswers {
        lines,
        hover,
        diagnostics,
    }
}

fn run_lane(oracle: &Tclsh, paths: &[PathBuf], archive: &str) -> Result<bool, Box<dyn Error>> {
    let dialect = oracle.version.dialect_name();
    let environment = tcl_registry::model::ingress::resolve_environment(dialect);
    let context = environment.default_context_registry();
    let registry = context.commands().as_ref();
    let config = LexerConfig::for_file_grammar(environment.grammar());
    let input = ResolvedAnalysisInput::new(
        environment.analyser_profile(),
        environment.unit_profile(),
        Arc::clone(&context),
        config,
    );
    let mut pass = true;
    println!(
        "C Tcl {}\nfixture\tsite\tcommand records\tbinding\theads\tLSP def lines\thover\tdiagnostics\tarchive matches Tcl\tLSP matches Tcl\tTcl stdout\tcriterion",
        oracle.patchlevel
    );
    for path in paths {
        let file = path
            .file_name()
            .and_then(|name| name.to_str())
            .expect("UTF-8 fixture name");
        let source = fs::read_to_string(path)?;
        let marker = marker(&source);
        let stdout = run_script_file(&oracle.path, path)?
            .strict_text()
            .map_err(|error| format!("{file}: {error}"))?;
        let expected = Answer::from_oracle(&stdout);
        let records = command_records(&source, &input, registry, config);
        let record = records.get(&marker.command);
        let answer = record.and_then(|record| binding_answer(record, registry));
        let heads = record.map(|record| {
            record
                .targets
                .iter()
                .map(|target| target.command.as_str())
                .collect::<Vec<_>>()
        });

        let analysis = Analyser::new()
            .with_resolved_input(input.clone())
            .analyse(&source, dialect);
        let index = LineIndex::new(&source);
        let pos = index.position_at_utf16(marker.command, &source);
        let ConsumerAnswers {
            lines,
            hover,
            diagnostics,
        } = consumer_answers(&source, marker.cursor, &index, &analysis, registry);
        let oracle_line = expected
            .declaration_offset(&analysis)
            .map(|offset| index.position_at_utf16(offset, &source).line + 1);
        let archived = archived_match(archive, file, dialect, oracle_line, &expected);
        assert_eq!(
            archived.is_some(),
            oracle.version != TclVersion::V9_1,
            "{file} {dialect}"
        );
        let live = if expected == Answer::Absent {
            lines.is_empty() && diagnostics.contains(&"W123")
        } else {
            oracle_line.is_some_and(|line| lines.as_slice() == [line])
                || hover
                    .as_deref()
                    .is_some_and(|hover| expected.matches_hover(hover))
        };
        let judgement = if record.is_none() {
            "missing record"
        } else if (live || archived == Some(true)) && answer.as_ref() != Some(&expected) {
            "command answer gap"
        } else {
            "pass"
        };
        pass &= judgement == "pass";
        println!(
            "{file}\t{}:{} {}\t{}\t{}\t{heads:?}\t{lines:?}\t{hover:?}\t{diagnostics:?}\t{archived:?}\t{live}\t{stdout:?}\t{judgement}",
            pos.line + 1,
            pos.character.get() + 1,
            marker.head,
            records.len(),
            answer.as_ref().map_or_else(
                || if record.is_some() {
                    "unresolved".into()
                } else {
                    "no record".into()
                },
                |answer| format!("{answer:?}"),
            ),
        );
    }
    Ok(pass)
}

fn main() -> Result<(), Box<dyn Error>> {
    let directory = Path::new(PROBE);
    let archive = fs::read_to_string(directory.join("frames.out"))?;
    let mut paths = fs::read_dir(directory.join("frames"))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()?;
    paths.retain(|path| path.extension().is_some_and(|extension| extension == "tcl"));
    paths.sort();
    assert_eq!(paths.len(), 16, "the frame spike expects all 16 fixtures");
    let mut pass = true;
    for oracle in required_tclshs(&[TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1])? {
        pass &= run_lane(&oracle, &paths, &archive)?;
    }
    println!("Item 93 criterion: {}", if pass { "pass" } else { "fail" });
    Ok(())
}
