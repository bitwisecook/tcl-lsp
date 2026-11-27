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

//! Name-handling plan item 95: measure W210 after editing one procedure body.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use salsa::Setter as _;
use tcl_compiler::analyser::Analyser;
use tcl_compiler::analyser::types::DiagCode;
use tcl_lexer::{LexerConfig, Span, TokenType};
use tcl_lsp_db::{SourceFile, TclDatabase, TclDb};
use tcl_source::segment::segment_commands_with_offset_and_config;

const DIALECT: &str = "tcl9.0";
const TARGET: &str = "overrides::cmdOverrides::IsOverridden";
const NAMESPACE_SETUP: &str = "namespace eval ::overrides::cmdOverrides {}\n";
const BUDGET: Duration = Duration::from_millis(50);
const TRIALS: usize = 5;

#[derive(Clone, Debug, PartialEq, Eq)]
struct ReadBeforeSet {
    span: Span,
    message: String,
}

#[salsa::interned]
struct Procedure<'db> {
    #[returns(ref)]
    declaration: String,
}

fn analyse_w210(source: &str) -> Vec<ReadBeforeSet> {
    let mut warnings: Vec<_> = Analyser::new()
        .analyse(source, DIALECT)
        .diagnostics
        .into_iter()
        .filter(|diagnostic| diagnostic.code == DiagCode::W210)
        .map(|diagnostic| ReadBeforeSet {
            span: diagnostic.span,
            message: diagnostic.message,
        })
        .collect();
    warnings.sort_unstable_by(|a, b| {
        (a.span.start(), a.span.end(), &a.message).cmp(&(b.span.start(), b.span.end(), &b.message))
    });
    warnings
}

#[salsa::tracked(lru = 64, returns(ref))]
fn procedure_w210<'db>(db: &'db dyn TclDb, procedure: Procedure<'db>) -> Vec<ReadBeforeSet> {
    // A qualified proc declaration needs its namespace to exist.
    let source = format!("{NAMESPACE_SETUP}{}", procedure.declaration(db));
    let setup_len = u32::try_from(NAMESPACE_SETUP.len()).unwrap();
    let mut warnings = analyse_w210(&source);
    for warning in &mut warnings {
        warning.span = Span::new(
            warning.span.start() - setup_len,
            warning.span.end() - setup_len,
        );
    }
    warnings
}

// The pinned fixture declares its procedures at the top level with literal
// names, formals and bodies. Use the complete declaration so the analyser keeps
// the procedure's namespace and formal parameters in the isolated unit.
fn procedures(source: &str) -> Vec<(String, Span)> {
    segment_commands_with_offset_and_config(source, 0, LexerConfig::for_dialect(DIALECT))
        .into_iter()
        .filter(|command| {
            command.name() == "proc"
                && !command.is_partial
                && command.argv.len() == 4
                && command.argv[3].kind == TokenType::Str
        })
        .map(|command| (command.texts[1].clone(), command.execution_span(source)))
        .collect()
}

#[salsa::tracked(returns(ref))]
fn file_procedure_w210(db: &dyn TclDb, file: SourceFile) -> Vec<ReadBeforeSet> {
    let source = file.text(db);
    let mut diagnostics = Vec::new();
    for (_, span) in procedures(source) {
        let procedure = Procedure::new(db, source[span.as_range()].to_owned());
        for diagnostic in procedure_w210(db, procedure) {
            let mut diagnostic = diagnostic.clone();
            diagnostic.span = Span::new(
                span.start() + diagnostic.span.start(),
                span.start() + diagnostic.span.end(),
            );
            diagnostics.push(diagnostic);
        }
    }
    diagnostics
}

fn within(diagnostic: &ReadBeforeSet, span: Span) -> bool {
    diagnostic.span.start() >= span.start() && diagnostic.span.end() <= span.end()
}

fn procedure_diagnostics(source: &str, diagnostics: Vec<ReadBeforeSet>) -> Vec<ReadBeforeSet> {
    let spans = procedures(source);
    diagnostics
        .into_iter()
        .filter(|diagnostic| spans.iter().any(|(_, span)| within(diagnostic, *span)))
        .collect()
}

fn edit_procedure(source: &str) -> String {
    let (_, span) = procedures(source)
        .into_iter()
        .find(|(name, _)| name == TARGET)
        .expect("target procedure");
    let body = &source[span.as_range()];
    let before = "    return [llength [array get LambdaStacks $cmdName]]\n";
    let after = "    set cmdName $missingProbeValue\n";
    assert_eq!(body.matches(before).count(), 1);
    let mut edited = source.to_owned();
    edited.replace_range(span.as_range(), &body.replacen(before, after, 1));
    edited
}

fn assert_breadth(before: &str, after: &str) -> usize {
    let before_procedures = procedures(before);
    let after_procedures = procedures(after);
    assert_eq!(before_procedures.len(), after_procedures.len());
    let mut changed = Vec::new();
    for ((name, before_span), (after_name, after_span)) in
        before_procedures.iter().zip(&after_procedures)
    {
        assert_eq!(name, after_name);
        if before[before_span.as_range()] != after[after_span.as_range()] {
            changed.push(name.as_str());
        }
    }
    assert_eq!(changed, [TARGET]);
    before_procedures.len()
}

fn measure_edit(
    before: &str,
    after: &str,
    original_w210: &[ReadBeforeSet],
    procedure_count: usize,
) -> (Duration, Vec<ReadBeforeSet>) {
    let events = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&events);
    let mut db = TclDatabase::with_event_logger(move |query| sink.lock().unwrap().push(query));
    let file = SourceFile::new(&db, before.to_owned(), DIALECT.to_owned(), None);
    assert_eq!(file_procedure_w210(&db, file), original_w210);
    let queries = |events: &mut Vec<String>| {
        std::mem::take(events)
            .into_iter()
            .filter(|query| query.starts_with("procedure_w210("))
            .collect::<Vec<_>>()
    };
    assert_eq!(queries(&mut events.lock().unwrap()).len(), procedure_count);

    let started = Instant::now();
    file.set_text(&mut db).to(after.to_owned());
    let diagnostics = file_procedure_w210(&db, file);
    let elapsed = started.elapsed();

    let executed = queries(&mut events.lock().unwrap());
    assert_eq!(executed.len(), 1, "{executed:?}");
    println!("{TARGET}: {executed:?}, {elapsed:.2?}");
    (elapsed, diagnostics.clone())
}

fn main() {
    tcl_registry::install_shipped().expect("the shipped surface set installs");
    let path = std::env::args_os()
        .nth(1)
        .expect("usage: w210_body_probe <687-line caching probe Tcl file>");
    let source = std::fs::read_to_string(path).expect("read the caching probe");
    assert_eq!(source.lines().count(), 687);
    let edited = edit_procedure(&source);
    assert_eq!(edited.lines().count(), 687);
    let procedure_count = assert_breadth(&source, &edited);

    let original_w210 = procedure_diagnostics(&source, analyse_w210(&source));
    println!("{procedure_count} procedure queries");
    let trials: Vec<_> = (0..TRIALS)
        .map(|_| measure_edit(&source, &edited, &original_w210, procedure_count))
        .collect();

    // The fresh edited analysis must run after the timed work so it cannot
    // warm any compiler caches for the new body before the first edit.
    let started = Instant::now();
    let edited_w210 = analyse_w210(&edited);
    let whole_file = started.elapsed();
    println!("Warm whole-file analysis: {whole_file:.2?}");
    let edited_w210 = procedure_diagnostics(&edited, edited_w210);
    assert_eq!(edited_w210.len(), original_w210.len() + 1);
    assert!(
        edited_w210
            .iter()
            .any(|diagnostic| edited[diagnostic.span.as_range()].contains("missingProbeValue"))
    );

    let slowest = trials
        .into_iter()
        .map(|(elapsed, warnings)| {
            assert_eq!(warnings, edited_w210);
            elapsed
        })
        .max()
        .unwrap();
    println!("Slowest of {TRIALS} edits: {slowest:.2?} (budget {BUDGET:.0?})");
    assert!(slowest <= BUDGET);
}
