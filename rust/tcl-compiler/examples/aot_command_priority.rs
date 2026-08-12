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

//! `aot_command_priority` — real-corpus command-usage census for the AOT WASM
//! compiler priority study (issue #1181).
//!
//! Segments every `.tcl` / `.test` file under the given corpus roots with the
//! **real segmenter** (`tcl_compiler::segmenter::segment_commands`, the same
//! machinery the lowerer and analyser use — not a regex), recursing into:
//!
//! * `[...]` command substitutions found in any word's fragments (always
//!   counted as **value position**),
//! * `ArgRole::Body` arguments resolved via `tcl_registry::CommandRegistry`
//!   (proc bodies, `if`/`while`/`for`/`foreach`/`catch`/`try`/`namespace
//!   eval`/… bodies — **statement position**), when the body word is a
//!   literal brace-string, and
//! * clause-list bodies (`switch`'s combined `{pat body pat body …}` form),
//!   which are a **list**, not a script — the pattern half is match text,
//!   never a command. These are split with `tcl_syntax::case_list`, the
//!   real registry-shaped clause-list parser (per-command shape read from
//!   `CommandSpec::case_list`), not the command segmenter. Segmenting a
//!   switch blob directly (rather than clause-splitting it first) reads
//!   every arm's *pattern* word as a spurious command name — this showed up
//!   as `file`/`default`/other pattern text ranking as "commands" during
//!   development. Only the body half of each clause is walked.
//!
//! Known gap: a braced `expr`/condition string's *own* mini-language may
//! contain further `[...]` substitutions (`expr {$x + [foo]}`); the Tcl
//! word-level parser does not substitute inside braces, so those are not
//! walked by this tool and are undercounted. See the design doc's Method
//! section.
//!
//! Usage:
//!
//! ```text
//! cargo run --release -p tcl-compiler --example aot_command_priority -- \
//!     experiments/aot-corpus/georgtree experiments/aot-corpus/nico-robert \
//!     experiments/aot-corpus/tcltk experiments/aot-corpus/aplsimple \
//!     experiments/aot-corpus/eda \
//!     --csv docs/design/compiler/data/aot-command-usage.csv
//! ```
//!
//! Each positional argument is a *group* directory whose immediate
//! subdirectories are treated as one repo apiece (matching
//! `experiments/aot-corpus/fetch_corpus.sh`'s layout).

use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use tcl_compiler::segmenter::{SegmentedCommand, segment_commands};
use tcl_registry::{ArgRole, CommandRegistry};
use tcl_syntax::case_list::{CallShape, CaseListShape, clause_list_call, split_case_list};

const MAX_DEPTH: u32 = 96;
const MAX_FILE_BYTES: u64 = 8_000_000;

#[derive(Default, Clone)]
struct FormStats {
    total: u64,
    value_position: u64,
    statement_position: u64,
    repos: HashSet<String>,
    arity_hist: HashMap<usize, u64>,
    all_args_literal: u64,
    some_args_substituted: u64,
    /// Whether `command` resolves in `CommandRegistry` — `false` means the
    /// literal head names a proc defined by the corpus itself (a project
    /// macro/helper), not a Tcl builtin, so it is out of scope for AOT
    /// direct-emission prioritisation.
    in_registry: bool,
}

impl FormStats {
    /// The most frequently seen argument count for this form, with how many
    /// call sites had it. `(0, 0)` when no site was recorded.
    fn dominant_arity(&self) -> (usize, u64) {
        self.arity_hist
            .iter()
            .max_by_key(|(_, count)| **count)
            .map_or((0, 0), |(arity, count)| (*arity, *count))
    }
}

#[derive(Default)]
struct DynamicStats {
    total: u64,
    value_position: u64,
    statement_position: u64,
}

fn collect_tcl_files(root: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if matches!(name, ".git" | "target" | "node_modules" | ".venv") {
                continue;
            }
            collect_tcl_files(&path, out);
        } else if path
            .extension()
            .and_then(|s| s.to_str())
            .is_some_and(|ext| ext == "tcl" || ext == "test")
        {
            out.push(path);
        }
    }
}

/// Is every fragment of this word literal (no `Var`/`Cmd` substitution)?
fn word_is_literal(fragments: &[tcl_compiler::segmenter::WordFragment]) -> bool {
    !fragments.is_empty()
        && fragments.iter().all(|f| {
            matches!(
                f.token.kind,
                tcl_lexer::TokenType::Esc | tcl_lexer::TokenType::Str
            )
        })
}

/// The accumulating census. `repo` names the repository whose files are being
/// walked right now; `forms` and `dynamic` accumulate across the whole corpus.
struct Census<'a> {
    registry: &'a CommandRegistry,
    repo: String,
    forms: HashMap<(String, Option<String>), FormStats>,
    dynamic: DynamicStats,
}

impl Census<'_> {
    fn walk_script(&mut self, source: &str, in_value_position: bool, depth: u32) {
        if depth > MAX_DEPTH {
            return;
        }
        for cmd in &segment_commands(source) {
            if cmd.texts.is_empty() {
                continue;
            }
            let word0_literal = cmd
                .word_fragments
                .first()
                .is_some_and(|f| word_is_literal(f));
            let expanded0 = cmd
                .expand_word
                .as_ref()
                .and_then(|v| v.first())
                .copied()
                .unwrap_or(false);

            if word0_literal && !expanded0 {
                self.walk_literal_head(cmd, in_value_position, depth);
            } else {
                self.dynamic.total += 1;
                if in_value_position {
                    self.dynamic.value_position += 1;
                } else {
                    self.dynamic.statement_position += 1;
                }
            }

            self.walk_substitutions(cmd, depth);
        }
    }

    /// Record one call site whose command word is a literal, then recurse into
    /// whatever bodies it carries.
    fn walk_literal_head(&mut self, cmd: &SegmentedCommand, in_value_position: bool, depth: u32) {
        let name = cmd.texts[0].clone();
        let args: Vec<&str> = cmd.texts[1..].iter().map(String::as_str).collect();
        self.record(cmd, &name, &args, in_value_position);

        // Clause-list commands (`switch`'s combined `{pat body …}` form) must
        // run first so the generic Body-role pass skips the same argument index
        // rather than re-segmenting the whole pattern+body blob as a script.
        let clause_handled_index = self.walk_clause_bodies(cmd, &name, &args, depth);
        self.walk_body_args(cmd, &name, &args, clause_handled_index, depth);
    }

    fn record(
        &mut self,
        cmd: &SegmentedCommand,
        name: &str,
        args: &[&str],
        in_value_position: bool,
    ) {
        let has_subcommands = self
            .registry
            .get(name)
            .is_some_and(|s| !s.subcommands.is_empty());
        let subcommand = if has_subcommands && !args.is_empty() {
            let word1_literal = cmd
                .word_fragments
                .get(1)
                .is_some_and(|f| word_is_literal(f));
            Some(if word1_literal {
                args[0].to_string()
            } else {
                "<dynamic>".to_string()
            })
        } else {
            None
        };

        let literal_args = cmd
            .word_fragments
            .iter()
            .skip(1)
            .all(|f| word_is_literal(f));
        let in_registry = self.registry.get(name).is_some();
        let repo = self.repo.clone();

        let stats = self
            .forms
            .entry((name.to_string(), subcommand))
            .or_default();
        stats.in_registry = in_registry;
        stats.total += 1;
        if in_value_position {
            stats.value_position += 1;
        } else {
            stats.statement_position += 1;
        }
        stats.repos.insert(repo);
        *stats.arity_hist.entry(args.len()).or_default() += 1;
        if literal_args {
            stats.all_args_literal += 1;
        } else {
            stats.some_args_substituted += 1;
        }
    }

    /// Walk the body half of each clause of a clause-list command, returning
    /// the argument index consumed by the clause list so the generic Body-role
    /// pass can skip it.
    fn walk_clause_bodies(
        &mut self,
        cmd: &SegmentedCommand,
        name: &str,
        args: &[&str],
        depth: u32,
    ) -> Option<usize> {
        let case = self.registry.get(name).and_then(|s| s.case_list)?;
        let call_shape = CallShape {
            subject_args: usize::from(case.subject_args),
            regex_option: case.regex_option,
            value_options: case.value_options_require_regex,
        };
        let clause_call = clause_list_call(args, &call_shape)?;
        let fragments = cmd.word_fragments.get(1 + clause_call.index)?;
        if fragments.len() != 1 || fragments[0].token.kind != tcl_lexer::TokenType::Str {
            return Some(clause_call.index);
        }

        let inner = &fragments[0].text;
        let clause_shape = CaseListShape {
            clause_flags: case.clause_flags,
            clause_value_flags: case.clause_value_flags,
        };
        for clause in split_case_list(inner, &clause_shape) {
            let Some(body) = clause.body else { continue };
            // `case_list::Element` includes the opening `{` (for fold-range
            // purposes) but its `end` already sits before the closing `}`, so a
            // braced body's clean content is `[start+1, end)`.
            let text = if body.braced {
                inner.get(body.start + 1..body.end)
            } else {
                inner.get(body.start..body.end)
            };
            let Some(text) = text else { continue };
            // A bare fallthrough marker (switch's `-` body) is not code — it
            // means "fall through to the next clause's body".
            if !body.braced && Some(text) == case.fallthrough_body {
                continue;
            }
            self.walk_script(text, false, depth + 1);
        }
        Some(clause_call.index)
    }

    /// Recurse into Body-role arguments (proc/if/while/for/foreach/catch/try/
    /// namespace eval/… bodies) written as a literal brace-string.
    fn walk_body_args(
        &mut self,
        cmd: &SegmentedCommand,
        name: &str,
        args: &[&str],
        clause_handled_index: Option<usize>,
        depth: u32,
    ) {
        for idx in self
            .registry
            .arg_indices_for_role(name, args, ArgRole::Body)
        {
            if Some(idx) == clause_handled_index {
                continue;
            }
            let Some(fragments) = cmd.word_fragments.get(1 + idx) else {
                continue;
            };
            if fragments.len() == 1 && fragments[0].token.kind == tcl_lexer::TokenType::Str {
                let text = fragments[0].text.clone();
                self.walk_script(&text, false, depth + 1);
            }
        }
    }

    /// Recurse into every `[...]` command substitution in every word, whether
    /// or not the command head itself was literal.
    fn walk_substitutions(&mut self, cmd: &SegmentedCommand, depth: u32) {
        for fragments in &cmd.word_fragments {
            for frag in fragments {
                if frag.token.kind == tcl_lexer::TokenType::Cmd
                    && frag.text.len() >= 2
                    && frag.text.starts_with('[')
                    && frag.text.ends_with(']')
                {
                    let inner = frag.text[1..frag.text.len() - 1].to_string();
                    self.walk_script(&inner, true, depth + 1);
                }
            }
        }
    }
}

fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

fn main() {
    let raw_args: Vec<String> = std::env::args().skip(1).collect();
    let mut roots: Vec<String> = Vec::new();
    let mut csv_path: Option<String> = None;
    let mut i = 0;
    while i < raw_args.len() {
        if raw_args[i] == "--csv" {
            csv_path = raw_args.get(i + 1).cloned();
            i += 2;
        } else {
            roots.push(raw_args[i].clone());
            i += 1;
        }
    }
    if roots.is_empty() {
        eprintln!(
            "usage: aot_command_priority <group-dir>... [--csv path.csv]\n\
             each group-dir's immediate subdirectories are treated as one repo apiece"
        );
        std::process::exit(2);
    }

    let registry = CommandRegistry::build_default();
    let mut census = Census {
        registry: &registry,
        repo: String::new(),
        forms: HashMap::new(),
        dynamic: DynamicStats::default(),
    };
    let mut repo_file_counts: Vec<(String, usize)> = Vec::new();
    let mut total_files = 0usize;
    let mut total_bytes = 0u64;

    for root in &roots {
        let root_path = Path::new(root);
        let Ok(entries) = fs::read_dir(root_path) else {
            eprintln!("skip (not a dir): {root}");
            continue;
        };
        let group_name = root_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(root)
            .to_string();
        let mut repo_dirs: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
        repo_dirs.sort();
        for repo_dir in repo_dirs {
            if !repo_dir.is_dir() {
                continue;
            }
            let repo_name = format!(
                "{group_name}/{}",
                repo_dir.file_name().and_then(|s| s.to_str()).unwrap_or("?")
            );
            let mut files = Vec::new();
            collect_tcl_files(&repo_dir, &mut files);
            census.repo = repo_name.clone();
            let mut file_count = 0usize;
            for f in &files {
                let Ok(meta) = fs::metadata(f) else { continue };
                if meta.len() > MAX_FILE_BYTES {
                    eprintln!("skip oversize file ({} bytes): {}", meta.len(), f.display());
                    continue;
                }
                let Ok(src) = fs::read_to_string(f) else {
                    continue;
                };
                total_bytes += meta.len();
                file_count += 1;
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    census.walk_script(&src, false, 0);
                }));
                if result.is_err() {
                    eprintln!("panic while segmenting, skipped: {}", f.display());
                }
            }
            total_files += file_count;
            repo_file_counts.push((repo_name, file_count));
        }
    }

    eprintln!(
        "== corpus: {} files, {} bytes, {} repos ==",
        total_files,
        total_bytes,
        repo_file_counts.len()
    );
    for (repo, n) in &repo_file_counts {
        eprintln!("  {repo}: {n} files");
    }
    eprintln!(
        "dynamic-head call sites (command word not a literal): total={} value={} stmt={}",
        census.dynamic.total, census.dynamic.value_position, census.dynamic.statement_position
    );

    let mut rows: Vec<(&(String, Option<String>), &FormStats)> = census.forms.iter().collect();
    rows.sort_by_key(|(_, stats)| std::cmp::Reverse(stats.total));

    if let Some(path) = &csv_path {
        let mut out = String::new();
        out.push_str(
            "command,subcommand,total,value_position,statement_position,repo_breadth,dominant_arity,dominant_arity_count,all_args_literal,some_args_substituted,in_registry\n",
        );
        for ((name, sub), stats) in &rows {
            let (dom_arity, dom_count) = stats.dominant_arity();
            let _ = writeln!(
                out,
                "{},{},{},{},{},{},{},{},{},{},{}",
                csv_escape(name),
                csv_escape(sub.as_deref().unwrap_or("")),
                stats.total,
                stats.value_position,
                stats.statement_position,
                stats.repos.len(),
                dom_arity,
                dom_count,
                stats.all_args_literal,
                stats.some_args_substituted,
                stats.in_registry,
            );
        }
        if let Some(parent) = Path::new(path).parent() {
            let _ = fs::create_dir_all(parent);
        }
        fs::write(path, out).expect("write csv");
        eprintln!("wrote {path}");
    }

    println!(
        "{:<24} {:<16} {:>8} {:>8} {:>8} {:>6} {:>6} {:>4}",
        "command", "subcommand", "total", "value", "stmt", "repos", "arity", "reg"
    );
    for ((name, sub), stats) in rows.iter().take(160) {
        let (dom_arity, _) = stats.dominant_arity();
        println!(
            "{:<24} {:<16} {:>8} {:>8} {:>8} {:>6} {:>6} {:>4}",
            name,
            sub.as_deref().unwrap_or(""),
            stats.total,
            stats.value_position,
            stats.statement_position,
            stats.repos.len(),
            dom_arity,
            if stats.in_registry { "y" } else { "n" },
        );
    }
}
