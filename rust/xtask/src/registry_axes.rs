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

//! `registry-axes` — the per-axis lint and its ledger
//! (`docs/design/compiler/registry-consumer-contracts.md` § *The per-axis
//! lint and ledger*).
//!
//! The registry declares every word a consumer could recognise a construct
//! by: command and subcommand names, option spellings, definition-body member
//! keywords, clause keywords, and special-variable names. A crate outside
//! `tcl-registry` compares a word against one of them only while the fact it
//! stands for has no descriptor the consumer can ask instead, so every such
//! site is either gone or reviewed. The gate has three parts:
//!
//! 1. **The vocabulary**, derived from the registry every loadable dialect
//!    and the shipped `.tclspec` packs build together — the registry
//!    `rust/tcl-registry/tests/analyser_hooks.rs` sweeps — so a word a pack
//!    adds joins the lint with no edit here.
//! 2. **A source lint** over the analysis and tooling tiers (the roots the
//!    value-transfer gate scans). A vocabulary word is a site when it is the
//!    operand of `==` / `!=`, a pattern of `matches!`, a string-literal
//!    pattern of a `match` arm, the argument of `.eq(`, an element of an
//!    inline array searched with `.contains(`, or an entry of a `&[&str]`
//!    table the file refers to again. The scan runs over the lexer's tokens,
//!    so a word inside a comment or a longer string is never a site, and it
//!    skips the item a `#[cfg(test)]` attribute guards. A reviewed site
//!    carries `// registry-axis-ok: <axis> — <reason>; until <expiry>` on
//!    its line or in the comment block directly above it — for an arm or a
//!    `matches!` pattern, above the enclosing `match` or `matches!` too — and
//!    a file whose sites all wait on one axis may carry one
//!    `// registry-axis-ok(file): <axis> — <reason>; until <expiry>` in its
//!    header. The axis is one of [`AXES`], the names of the migration plan's
//!    § *Debt on other axes* table, so a site waived for both this gate and
//!    the value-transfer gate names one axis. The expiry is `step N` (a step
//!    of the consumer-contracts build order), `slice N` (a value-transfer
//!    slice), or `never`, which only `irreducible` may carry; an expiry that
//!    names a landed step or slice ([`LANDED`]) fails the gate, so a waiver
//!    cannot outlive the change that was to retire it.
//! 3. **A ratchet and the generated ledger.** A file in [`CLEAN_FILES`] has
//!    every site waived or gone. Every other scanned file's count of unwaived
//!    sites is pinned in [`RATCHET`]: `--check` fails when a count rises above
//!    its pin, when a pin is above its count (lower it with the review that
//!    removed the sites), and when an unpinned file gains a site. The report,
//!    `docs/generated/registry-axes.md`, lists every waiver by axis with its
//!    expiry, and the ratchet table.
//!
//! The lint is a warning mechanism, as the value-transfer gate's is: a word
//! reached through a helper or a renamed binding evades it, so the
//! registry's contract tests and review are its other half.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result};
use rustc_lexer::{LiteralKind, TokenKind};
use tcl_registry::CommandRegistry;
use tcl_registry::hover::OptionSpec;

use crate::util::repo_root;

const REPORT_PATH: &str = "docs/generated/registry-axes.md";

/// The roots the lint scans — the value-transfer gate's analysis and tooling
/// tiers. The registry is outside them by construction: it is where the
/// vocabulary lives.
const LINT_ROOTS: &[&str] = &[
    "rust/tcl-compiler/src",
    "rust/tcl-lsp-core/src",
    "rust/tcl-mcp/src",
    "rust/tcl-cli/src",
    "rust/tcl-diagram/src",
    "rust/tcl-irules/src",
    "rust/tcl-irule-test/src",
    "rust/tcl-bigip/src",
    "rust/tcl-sslictcl/src",
    "rust/tcl-syntax/src",
];

/// The axes a waiver may name: the migration plan's § *Debt on other axes*
/// destinations for vocabulary sites, and the sanctioned exception.
const AXES: &[&str] = &[
    "command",
    "subcommands",
    "clause_grammar",
    "definition_body",
    "options",
    "special_vars",
    "irreducible",
];

/// The build steps and value-transfer slices that have landed. A waiver whose
/// expiry names one of them is stale: the change that was to retire the site
/// has shipped without it. A lane bumps this when its step or slice lands.
const LANDED: &[&str] = &["step 1", "slice 1", "slice 2", "slice 3", "slice 4"];

/// The files the lint holds clean: every site waived or gone. A step that
/// rewrites a file adds it here with its pin removed; a file never leaves.
const CLEAN_FILES: &[&str] = &[];

/// The pinned count of unwaived sites per scanned file, from the gate's first
/// run. A count may only fall: a pin is lowered beside the review that
/// removes or waives its sites, and is never raised or added.
const RATCHET: &[(&str, usize)] = &[
    ("rust/tcl-bigip/src/apl/iapp_diagnostics.rs", 1),
    ("rust/tcl-bigip/src/apl/parser.rs", 3),
    ("rust/tcl-bigip/src/apl/tokens.rs", 5),
    ("rust/tcl-bigip/src/conf_tokens.rs", 10),
    ("rust/tcl-bigip/src/graph.rs", 33),
    ("rust/tcl-bigip/src/grep.rs", 3),
    ("rust/tcl-bigip/src/irule_context.rs", 6),
    ("rust/tcl-bigip/src/lint.rs", 6),
    ("rust/tcl-bigip/src/model/gen/dispatch.rs", 16),
    ("rust/tcl-bigip/src/parser/bespoke.rs", 28),
    ("rust/tcl-bigip/src/parser/driver.rs", 13),
    ("rust/tcl-bigip/src/pcap_enrich.rs", 3),
    ("rust/tcl-bigip/src/policy_eval.rs", 9),
    ("rust/tcl-bigip/src/redact.rs", 3),
    ("rust/tcl-bigip/src/secrets.rs", 1),
    ("rust/tcl-bigip/src/tls.rs", 10),
    ("rust/tcl-bigip/src/tmsh_emit.rs", 26),
    ("rust/tcl-bigip/src/validator.rs", 4),
    ("rust/tcl-bigip/src/value/attachments.rs", 2),
    ("rust/tcl-bigip/src/value/cert_key_chain.rs", 3),
    ("rust/tcl-bigip/src/value/data_group_record.rs", 1),
    ("rust/tcl-bigip/src/value/firewall_rule.rs", 5),
    ("rust/tcl-bigip/src/value/monitor_expression.rs", 3),
    ("rust/tcl-bigip/src/value/network.rs", 2),
    ("rust/tcl-bigip/src/value/policy.rs", 21),
    ("rust/tcl-bigip/src/value/port.rs", 1),
    ("rust/tcl-bigip/src/value/port_set.rs", 1),
    ("rust/tcl-bigip/src/value/snat_mode.rs", 4),
    ("rust/tcl-bigip/src/wireshark_profile.rs", 7),
    ("rust/tcl-cli/src/commands/diff.rs", 1),
    ("rust/tcl-cli/src/commands/graphs.rs", 1),
    ("rust/tcl-cli/src/commands/help.rs", 1),
    ("rust/tcl-cli/src/commands/minimize.rs", 13),
    ("rust/tcl-cli/src/commands/pkg_discover.rs", 1),
    ("rust/tcl-cli/src/commands/spec.rs", 1),
    ("rust/tcl-cli/src/commands/transform.rs", 2),
    ("rust/tcl-cli/src/lib.rs", 1),
    ("rust/tcl-compiler/src/analyser/bounds_checks.rs", 34),
    ("rust/tcl-compiler/src/analyser/class_lattice.rs", 5),
    ("rust/tcl-compiler/src/analyser/commands.rs", 12),
    ("rust/tcl-compiler/src/analyser/diagnostics/dataflow.rs", 9),
    ("rust/tcl-compiler/src/analyser/diagnostics/fp/sh.rs", 1),
    ("rust/tcl-compiler/src/analyser/diagnostics/helpers.rs", 6),
    ("rust/tcl-compiler/src/analyser/diagnostics/security.rs", 8),
    ("rust/tcl-compiler/src/analyser/diagnostics/usage.rs", 6),
    ("rust/tcl-compiler/src/analyser/diagnostics/validity.rs", 22),
    (
        "rust/tcl-compiler/src/analyser/diagnostics/var_command.rs",
        7,
    ),
    (
        "rust/tcl-compiler/src/analyser/diagnostics/version_gate.rs",
        1,
    ),
    ("rust/tcl-compiler/src/analyser/handlers.rs", 20),
    ("rust/tcl-compiler/src/analyser/irules_event_checks.rs", 8),
    ("rust/tcl-compiler/src/analyser/oo.rs", 43),
    ("rust/tcl-compiler/src/analyser/param_traits.rs", 16),
    ("rust/tcl-compiler/src/analyser/per_item.rs", 4),
    ("rust/tcl-compiler/src/analyser/recovery.rs", 4),
    ("rust/tcl-compiler/src/analyser/tk_checks.rs", 2),
    ("rust/tcl-compiler/src/auto_path_eval.rs", 6),
    ("rust/tcl-compiler/src/cfg_builder/cfg_lower.rs", 1),
    ("rust/tcl-compiler/src/cfg_builder/upvar_info.rs", 1),
    ("rust/tcl-compiler/src/codegen/cmd_subst.rs", 21),
    ("rust/tcl-compiler/src/codegen/control_flow.rs", 3),
    ("rust/tcl-compiler/src/codegen/emitter/bytecoded.rs", 13),
    ("rust/tcl-compiler/src/codegen/emitter/loop_blocks.rs", 2),
    ("rust/tcl-compiler/src/codegen/emitter/ordering.rs", 2),
    ("rust/tcl-compiler/src/codegen/emitter/try_blocks.rs", 2),
    ("rust/tcl-compiler/src/codegen/statements.rs", 5),
    ("rust/tcl-compiler/src/codegen/structured.rs", 2),
    ("rust/tcl-compiler/src/common_aot_plan.rs", 1),
    ("rust/tcl-compiler/src/connection_scope.rs", 1),
    ("rust/tcl-compiler/src/executable_ir.rs", 1),
    ("rust/tcl-compiler/src/inline_uplevel.rs", 1),
    ("rust/tcl-compiler/src/inlining/mod.rs", 1),
    ("rust/tcl-compiler/src/interprocedural.rs", 6),
    ("rust/tcl-compiler/src/interval_bounds.rs", 5),
    ("rust/tcl-compiler/src/ir.rs", 5),
    ("rust/tcl-compiler/src/irules_checks.rs", 7),
    ("rust/tcl-compiler/src/lowering/mod.rs", 25),
    ("rust/tcl-compiler/src/lowering/structured.rs", 19),
    ("rust/tcl-compiler/src/lowering_hooks.rs", 2),
    ("rust/tcl-compiler/src/object_types.rs", 2),
    ("rust/tcl-compiler/src/optimiser/end_offset.rs", 7),
    ("rust/tcl-compiler/src/optimiser/profiles.rs", 1),
    ("rust/tcl-compiler/src/optimiser/propagation.rs", 5),
    ("rust/tcl-compiler/src/place_bridge.rs", 3),
    ("rust/tcl-compiler/src/realm.rs", 1),
    ("rust/tcl-compiler/src/regex_source.rs", 2),
    ("rust/tcl-compiler/src/scan_predicate.rs", 2),
    ("rust/tcl-compiler/src/segmenter.rs", 2),
    ("rust/tcl-compiler/src/shimmer/thunking.rs", 1),
    ("rust/tcl-compiler/src/signature_scan/arity.rs", 1),
    ("rust/tcl-compiler/src/signature_scan/ctx.rs", 1),
    ("rust/tcl-compiler/src/signature_scan/factory.rs", 1),
    ("rust/tcl-compiler/src/signature_scan/handlers.rs", 1),
    ("rust/tcl-compiler/src/signature_scan/params.rs", 1),
    ("rust/tcl-compiler/src/signature_scan/walker.rs", 10),
    ("rust/tcl-compiler/src/specialise_factories.rs", 1),
    ("rust/tcl-compiler/src/ssa.rs", 1),
    ("rust/tcl-compiler/src/taint.rs", 12),
    ("rust/tcl-compiler/src/taint_interproc.rs", 2),
    ("rust/tcl-compiler/src/unit_scope.rs", 2),
    ("rust/tcl-compiler/src/uri_split.rs", 10),
    ("rust/tcl-compiler/src/value_provenance.rs", 1),
    ("rust/tcl-compiler/src/var_escape/handlers.rs", 2),
    ("rust/tcl-compiler/src/var_escape/helpers.rs", 1),
    ("rust/tcl-compiler/src/var_escape/info_subcommands.rs", 29),
    ("rust/tcl-compiler/src/var_escape/slot_resolution.rs", 7),
    ("rust/tcl-compiler/src/var_scoping.rs", 2),
    ("rust/tcl-diagram/src/attach.rs", 7),
    ("rust/tcl-diagram/src/data.rs", 5),
    ("rust/tcl-diagram/src/graph.rs", 9),
    ("rust/tcl-irules/src/ilx.rs", 2),
    ("rust/tcl-irules/src/lib.rs", 26),
    ("rust/tcl-irules/src/walker.rs", 1),
    ("rust/tcl-lsp-core/src/call_hierarchy.rs", 4),
    ("rust/tcl-lsp-core/src/completion.rs", 4),
    ("rust/tcl-lsp-core/src/config_ini.rs", 4),
    ("rust/tcl-lsp-core/src/definition.rs", 4),
    ("rust/tcl-lsp-core/src/diagnostic_policy.rs", 1),
    ("rust/tcl-lsp-core/src/diagnostic_policy/truth_table.rs", 2),
    ("rust/tcl-lsp-core/src/document_links.rs", 7),
    ("rust/tcl-lsp-core/src/document_symbols.rs", 1),
    ("rust/tcl-lsp-core/src/formatting/config.rs", 1),
    ("rust/tcl-lsp-core/src/formatting/docstring.rs", 1),
    ("rust/tcl-lsp-core/src/formatting/engine.rs", 2),
    ("rust/tcl-lsp-core/src/formatting/keywords.rs", 1),
    ("rust/tcl-lsp-core/src/hover.rs", 6),
    ("rust/tcl-lsp-core/src/inlay_hints.rs", 3),
    ("rust/tcl-lsp-core/src/minify.rs", 7),
    ("rust/tcl-lsp-core/src/oo_body.rs", 2),
    ("rust/tcl-lsp-core/src/oo_dispatch.rs", 4),
    ("rust/tcl-lsp-core/src/package_resolver.rs", 9),
    ("rust/tcl-lsp-core/src/package_resolver/reachability.rs", 4),
    ("rust/tcl-lsp-core/src/refactor/datagroup.rs", 10),
    ("rust/tcl-lsp-core/src/refactor/if_to_switch.rs", 6),
    ("rust/tcl-lsp-core/src/refactor/inline_proc.rs", 3),
    ("rust/tcl-lsp-core/src/refactor/mod.rs", 4),
    ("rust/tcl-lsp-core/src/refactor/switch_to_dict.rs", 4),
    ("rust/tcl-lsp-core/src/references.rs", 6),
    ("rust/tcl-lsp-core/src/semantic_tokens.rs", 17),
    ("rust/tcl-lsp-core/src/tk_preview.rs", 1),
    ("rust/tcl-lsp-core/src/workspace_index.rs", 8),
    ("rust/tcl-mcp/src/bigip.rs", 1),
    ("rust/tcl-mcp/src/datagroup.rs", 11),
    ("rust/tcl-mcp/src/irule_gen.rs", 18),
    ("rust/tcl-mcp/src/irule_test.rs", 21),
    ("rust/tcl-mcp/src/spectcl.rs", 4),
    ("rust/tcl-mcp/src/tools.rs", 1),
    ("rust/tcl-sslictcl/src/dsl.rs", 24),
    ("rust/tcl-sslictcl/src/estimate.rs", 2),
    ("rust/tcl-sslictcl/src/nginx.rs", 10),
    ("rust/tcl-sslictcl/src/openssl_config.rs", 1),
    ("rust/tcl-sslictcl/src/testssl.rs", 1),
    ("rust/tcl-sslictcl/src/trust.rs", 1),
    ("rust/tcl-syntax/src/boolean.rs", 3),
    ("rust/tcl-syntax/src/case_list.rs", 2),
    ("rust/tcl-syntax/src/event_handler.rs", 7),
    ("rust/tcl-syntax/src/expr/mathfunc.rs", 6),
    ("rust/tcl-syntax/src/expr/parser.rs", 43),
    ("rust/tcl-syntax/src/formal_params.rs", 1),
    ("rust/tcl-syntax/src/glob.rs", 1),
    ("rust/tcl-syntax/src/naming.rs", 2),
    ("rust/tcl-syntax/src/ns_op_conformance.rs", 21),
    ("rust/tcl-syntax/src/var_conformance.rs", 10),
    ("rust/tcl-syntax/src/vector_ops.rs", 1),
];

const WAIVER: &str = "registry-axis-ok:";
const FILE_WAIVER: &str = "registry-axis-ok(file):";

/// Run the gate: the vocabulary, the lint, the ratchet, and the report,
/// written or verified.
pub fn run(check: bool) -> Result<ExitCode> {
    let root = repo_root();
    let vocabulary = vocabulary(&full_registry(&root));
    let lint = lint(&root, &vocabulary);
    let mut failed = false;
    if !lint.clean_hits.is_empty() {
        failed = true;
        eprintln!(
            "registry-axes: {} site(s) compare a registry word in a file the gate holds clean. \
             Ask the axis query the registry answers instead, or mark a reviewed site \
             `// registry-axis-ok: <axis> — <reason>; until <step N | slice N | never>`:",
            lint.clean_hits.len()
        );
        for hit in &lint.clean_hits {
            eprintln!("  {hit}");
        }
    }
    for problem in ratchet_verdicts(&lint, RATCHET, CLEAN_FILES)
        .into_iter()
        .chain(lint.problems.iter().cloned())
    {
        failed = true;
        eprintln!("registry-axes: {problem}");
    }
    let report = render_report(&lint);
    let path = root.join(REPORT_PATH);
    if check {
        let on_disk = fs::read_to_string(&path).unwrap_or_default();
        if on_disk != report {
            failed = true;
            eprintln!("registry-axes: {REPORT_PATH} is stale — run `cargo xtask registry-axes`");
        }
    } else {
        fs::write(&path, &report).with_context(|| format!("writing {REPORT_PATH}"))?;
    }
    if failed {
        return Ok(ExitCode::FAILURE);
    }
    println!(
        "registry-axes: OK ({} vocabulary word(s), {} file(s) clean, {} site(s) waived, {} \
         site(s) pinned across {} ratcheted file(s))",
        vocabulary.words.len(),
        CLEAN_FILES.len(),
        lint.waived.len(),
        lint.ratchet_hits.values().map(Vec::len).sum::<usize>(),
        lint.ratchet_hits.len(),
    );
    Ok(ExitCode::SUCCESS)
}

// The vocabulary

/// The compiled-in dialects whose packs the registry merges — the list
/// `rust/tcl-registry/tests/analyser_hooks.rs` sweeps.
const LOADABLE_DIALECTS: &[&str] = &[
    "tcl8.4",
    "tcl8.5",
    "tcl8.6",
    "tcl9.0",
    "f5-irules",
    "f5-iapps",
    "expect",
    "bpf",
];

/// Every loadable dialect's commands merged into one registry, with the
/// shipped `.tclspec` packs inserted — the vocabulary's source.
fn full_registry(root: &Path) -> CommandRegistry {
    let mut registry = CommandRegistry::build_default();
    for name in LOADABLE_DIALECTS {
        if let Some(profile) = tcl_dialect::DialectProfile::find(name) {
            for &layer in profile.base_layers {
                registry.load_surface(layer);
            }
        }
    }
    for pack in tcl_spectcl::bundled::load_from(&root.join("specs")).packs {
        for command in pack.commands {
            registry.insert(command.spec.clone());
        }
    }
    registry
}

/// Every word the registry declares, with the axes that declare it.
#[derive(Debug, Default)]
struct Vocabulary {
    words: BTreeMap<String, BTreeSet<&'static str>>,
}

impl Vocabulary {
    fn add(&mut self, word: &str, axis: &'static str) {
        if !word.is_empty() {
            self.words.entry(word.to_owned()).or_default().insert(axis);
        }
    }

    fn add_options(&mut self, options: &[OptionSpec]) {
        for option in options {
            self.add(option.name, "options");
            for alias in option.aliases {
                self.add(alias, "options");
            }
        }
    }

    fn axes(&self, word: &str) -> Option<&BTreeSet<&'static str>> {
        self.words.get(word)
    }
}

/// The registry's vocabulary: every command and subcommand name, every
/// option spelling and alias at command, subcommand and form level, every
/// member keyword of every definition-body grammar a spec references, every
/// word a clause grammar matches by value (its rows' keywords and its noise
/// words), and every special-variable name.
fn vocabulary(registry: &CommandRegistry) -> Vocabulary {
    let mut vocabulary = Vocabulary::default();
    let mut grammars: Vec<&'static tcl_registry::definer::DefinitionBodyGrammar> = Vec::new();
    for name in registry.command_names() {
        vocabulary.add(name, "command");
        for spec in registry.specs(name) {
            vocabulary.add_options(spec.options);
            for form in spec.command_forms {
                vocabulary.add_options(form.options);
            }
            for sub in spec.subcommands {
                vocabulary.add(sub.name, "subcommands");
                vocabulary.add_options(sub.options);
                for sub_sub in sub.sub_subcommands {
                    vocabulary.add(sub_sub.name, "subcommands");
                    if let Some(options) = sub_sub.options {
                        vocabulary.add_options(options);
                    }
                }
            }
            if let Some(grammar) = spec.definition_body {
                grammars.push(grammar);
            }
        }
    }
    for grammar in grammars {
        for member in grammar.members {
            vocabulary.add(member.keyword, "definition_body");
        }
    }
    for keyword in tcl_registry::clause_grammar::clause_keywords(registry) {
        vocabulary.add(keyword.word, "clause_grammar");
    }
    for var in tcl_registry::special_vars::SPECIAL_VARS {
        vocabulary.add(var.name, "special_vars");
    }
    vocabulary
}

// The lint

/// One recognised site.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Site {
    path: String,
    line: usize,
    snippet: String,
}

impl std::fmt::Display for Site {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}: {}", self.path, self.line, self.snippet)
    }
}

/// A waived site, with the axis, reason and expiry its waiver names.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Waived {
    axis: String,
    until: String,
    site: Site,
    reason: String,
    /// Whether the waiver is the file's, not the site's.
    file_level: bool,
}

#[derive(Debug, Default)]
struct Lint {
    /// Unwaived sites in a file the gate holds clean.
    clean_hits: Vec<Site>,
    /// Unwaived sites in every other file, by path, in line order.
    ratchet_hits: BTreeMap<String, Vec<Site>>,
    waived: Vec<Waived>,
    problems: Vec<String>,
}

/// Whether a repository-relative path is one the lint reads: never a registry
/// source (the vocabulary's owner), never a file under a `tests/` directory,
/// and never a `tests.rs` module.
fn is_scanned(rel: &str) -> bool {
    !rel.starts_with("rust/tcl-registry/")
        && !rel.contains("/tests/")
        && !rel.ends_with("/tests.rs")
        && Path::new(rel).extension().is_some_and(|ext| ext == "rs")
}

fn lint(root: &Path, vocabulary: &Vocabulary) -> Lint {
    let mut files = Vec::new();
    for dir in LINT_ROOTS {
        collect_rs_files(&root.join(dir), &mut files);
    }
    files.sort();
    let mut out = Lint::default();
    for path in files {
        let rel = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        if !is_scanned(&rel) {
            continue;
        }
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        lint_file(&rel, &text, vocabulary, &mut out);
    }
    out.clean_hits.sort();
    out.waived.sort();
    out
}

fn lint_file(rel: &str, text: &str, vocabulary: &Vocabulary, out: &mut Lint) {
    let clean = CLEAN_FILES.contains(&rel);
    let comments = line_comments(text);
    let file_waiver = file_waiver(&comments);
    if let Some(Err(problem)) = &file_waiver {
        out.problems.push(format!("{rel}: file waiver {problem}"));
    }
    for hit in scan(text, vocabulary) {
        let site = Site {
            path: rel.to_owned(),
            line: hit.line,
            snippet: snippet(text, hit.line),
        };
        match site_waiver(&comments, &hit) {
            Some(Ok(waiver)) => out.waived.push(Waived {
                axis: waiver.axis,
                until: waiver.until,
                site,
                reason: waiver.reason,
                file_level: false,
            }),
            Some(Err(problem)) => {
                out.problems
                    .push(format!("{rel}:{}: waiver {problem}", hit.line));
            }
            None => match &file_waiver {
                Some(Ok(waiver)) => out.waived.push(Waived {
                    axis: waiver.axis.clone(),
                    until: waiver.until.clone(),
                    site,
                    reason: waiver.reason.clone(),
                    file_level: true,
                }),
                _ if clean => out.clean_hits.push(site),
                _ => out
                    .ratchet_hits
                    .entry(rel.to_owned())
                    .or_default()
                    .push(site),
            },
        }
    }
}

/// Recursively collect `.rs` files under `dir` (skipping `target/`).
fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().is_some_and(|n| n == "target") {
                continue;
            }
            collect_rs_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

fn snippet(text: &str, line: usize) -> String {
    text.lines()
        .nth(line.saturating_sub(1))
        .unwrap_or_default()
        .trim()
        .chars()
        .take(96)
        .collect()
}

// The scan

/// One significant token: its kind, byte range and 1-based line.
#[derive(Debug, Clone, Copy)]
struct Tok {
    kind: TokenKind,
    start: usize,
    end: usize,
    line: usize,
}

/// The file's tokens, trivia dropped.
fn tokens(text: &str) -> Vec<Tok> {
    let mut out = Vec::new();
    let mut offset = 0;
    let mut line = 1;
    for token in rustc_lexer::tokenize(text) {
        let end = offset + token.len;
        if !matches!(
            token.kind,
            TokenKind::Whitespace | TokenKind::LineComment | TokenKind::BlockComment { .. }
        ) {
            out.push(Tok {
                kind: token.kind,
                start: offset,
                end,
                line,
            });
        }
        line += text[offset..end].bytes().filter(|b| *b == b'\n').count();
        offset = end;
    }
    out
}

/// One `//` comment: its text, and whether it stands alone on its line.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Comment {
    text: String,
    standalone: bool,
}

/// Every `//` comment by line — the waiver markers' carrier.
fn line_comments(text: &str) -> BTreeMap<usize, Comment> {
    let mut out = BTreeMap::new();
    let mut offset = 0;
    let mut line = 1;
    let mut line_start = 0;
    for token in rustc_lexer::tokenize(text) {
        let end = offset + token.len;
        if token.kind == TokenKind::LineComment {
            out.insert(
                line,
                Comment {
                    text: text[offset..end].to_owned(),
                    standalone: text[line_start..offset].trim().is_empty(),
                },
            );
        }
        let lexeme = &text[offset..end];
        line += lexeme.bytes().filter(|b| *b == b'\n').count();
        if let Some(last) = lexeme.rfind('\n') {
            line_start = offset + last + 1;
        }
        offset = end;
    }
    out
}

/// A vocabulary word found in a comparison shape.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Hit {
    /// The line the word is on.
    line: usize,
    /// The line of the enclosing `match`, `matches!` or table declaration,
    /// whose comment block may also carry the site's waiver.
    anchor: Option<usize>,
    word: String,
}

/// The unquoted text of a plain string literal token, when it is one.
fn str_literal<'t>(text: &'t str, tok: &Tok) -> Option<&'t str> {
    matches!(
        tok.kind,
        TokenKind::Literal {
            kind: LiteralKind::Str { terminated: true },
            ..
        }
    )
    .then(|| {
        text[tok.start..tok.end]
            .strip_prefix('"')?
            .strip_suffix('"')
    })
    .flatten()
}

fn ident<'t>(text: &'t str, tok: &Tok) -> Option<&'t str> {
    (tok.kind == TokenKind::Ident).then(|| &text[tok.start..tok.end])
}

/// Whether `toks[i]` and `toks[i + 1]` spell the two-character operator
/// `first second` with nothing between them.
fn joined(toks: &[Tok], i: usize, first: TokenKind, second: TokenKind) -> bool {
    toks.get(i).is_some_and(|a| a.kind == first)
        && toks
            .get(i + 1)
            .is_some_and(|b| b.kind == second && b.start == toks[i].end)
}

/// Whether the tokens ending just before `at` are `==` or `!=`.
fn equality_before(toks: &[Tok], at: usize) -> bool {
    at >= 2
        && (joined(toks, at - 2, TokenKind::Eq, TokenKind::Eq)
            || joined(toks, at - 2, TokenKind::Not, TokenKind::Eq))
}

/// The index of the token closing the delimiter opened at `open`.
fn matching_close(toks: &[Tok], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (i, tok) in toks.iter().enumerate().skip(open) {
        match tok.kind {
            TokenKind::OpenParen | TokenKind::OpenBrace | TokenKind::OpenBracket => depth += 1,
            TokenKind::CloseParen | TokenKind::CloseBrace | TokenKind::CloseBracket => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

/// The token ranges a `#[cfg(test)]` attribute guards: the item after it, to
/// its closing brace or semicolon.
fn test_guarded(text: &str, toks: &[Tok]) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut i = 0;
    while i + 6 < toks.len() {
        let is_cfg_test = toks[i].kind == TokenKind::Pound
            && toks[i + 1].kind == TokenKind::OpenBracket
            && ident(text, &toks[i + 2]) == Some("cfg")
            && toks[i + 3].kind == TokenKind::OpenParen
            && ident(text, &toks[i + 4]) == Some("test")
            && toks[i + 5].kind == TokenKind::CloseParen
            && toks[i + 6].kind == TokenKind::CloseBracket;
        if !is_cfg_test {
            i += 1;
            continue;
        }
        let mut j = i + 7;
        let mut end = toks.len() - 1;
        while j < toks.len() {
            match toks[j].kind {
                TokenKind::Semi => {
                    end = j;
                    break;
                }
                TokenKind::OpenBrace => {
                    end = matching_close(toks, j).unwrap_or(toks.len() - 1);
                    break;
                }
                // A further attribute, or the item's own parameter list.
                TokenKind::OpenBracket | TokenKind::OpenParen => {
                    j = matching_close(toks, j).unwrap_or(toks.len() - 1) + 1;
                }
                _ => j += 1,
            }
        }
        out.push((i, end));
        i = end + 1;
    }
    out
}

/// Every vocabulary word in a comparison shape, one hit per line.
fn scan(text: &str, vocabulary: &Vocabulary) -> Vec<Hit> {
    let toks = tokens(text);
    let guarded = test_guarded(text, &toks);
    let mut hits: BTreeMap<usize, Hit> = BTreeMap::new();
    let mut push = |i: usize, anchor: Option<usize>| {
        if guarded
            .iter()
            .any(|&(start, end)| (start..=end).contains(&i))
        {
            return;
        }
        let Some(word) = toks
            .get(i)
            .and_then(|tok| str_literal(text, tok))
            .filter(|word| vocabulary.axes(word).is_some())
        else {
            return;
        };
        hits.entry(toks[i].line).or_insert_with(|| Hit {
            line: toks[i].line,
            anchor,
            word: word.to_owned(),
        });
    };
    for i in 0..toks.len() {
        if compared(text, &toks, i) {
            push(i, None);
        }
    }
    for (i, tok) in toks.iter().enumerate() {
        match (tok.kind, ident(text, tok)) {
            (TokenKind::Ident, Some("matches")) => scan_matches_macro(&toks, i, &mut push),
            (TokenKind::Ident, Some("match")) => {
                let Some(open) = (i + 1..toks.len())
                    .find(|&j| matches!(toks[j].kind, TokenKind::OpenBrace | TokenKind::Semi))
                    .filter(|&j| toks[j].kind == TokenKind::OpenBrace)
                else {
                    continue;
                };
                if let Some(close) = matching_close(&toks, open) {
                    scan_arms(text, &toks, open + 1, close, tok.line, &mut push);
                }
            }
            (TokenKind::Ident, Some("const" | "static")) => scan_table(text, &toks, i, &mut push),
            (TokenKind::OpenBracket, _) => scan_searched_array(text, &toks, i, &mut push),
            _ => {}
        }
    }
    hits.into_values().collect()
}

/// Whether the token at `i` is an operand of `==` / `!=` (`x == "w"`,
/// `"w" != x`, `x == Some("w")`, `x == &"w"`) or the argument of `.eq(` /
/// `.eq_ignore_ascii_case(` (`.eq("w")`, `.eq(&"w")`).
fn compared(text: &str, toks: &[Tok], i: usize) -> bool {
    let before_some = i >= 2
        && toks[i - 1].kind == TokenKind::OpenParen
        && ident(text, &toks[i - 2]) == Some("Some")
        && equality_before(toks, i - 2);
    let before_ref = i >= 1
        && matches!(toks[i - 1].kind, TokenKind::And | TokenKind::Star)
        && equality_before(toks, i - 1);
    let after = joined(toks, i + 1, TokenKind::Eq, TokenKind::Eq)
        || joined(toks, i + 1, TokenKind::Not, TokenKind::Eq);
    let open = if i >= 1 && toks[i - 1].kind == TokenKind::And {
        i.checked_sub(2)
    } else {
        i.checked_sub(1)
    };
    let eq_call = open.is_some_and(|open| {
        open >= 2
            && toks[open].kind == TokenKind::OpenParen
            && matches!(
                ident(text, &toks[open - 1]),
                Some("eq" | "eq_ignore_ascii_case")
            )
            && toks[open - 2].kind == TokenKind::Dot
    });
    equality_before(toks, i) || before_some || before_ref || after || eq_call
}

/// `matches!(scrutinee, "w" | …)` at `at`: every literal after the first
/// top-level comma is a pattern.
fn scan_matches_macro(toks: &[Tok], at: usize, push: &mut impl FnMut(usize, Option<usize>)) {
    let is_macro = toks.get(at + 1).is_some_and(|t| t.kind == TokenKind::Not)
        && toks
            .get(at + 2)
            .is_some_and(|t| t.kind == TokenKind::OpenParen);
    let Some(close) = is_macro.then(|| matching_close(toks, at + 2)).flatten() else {
        return;
    };
    let mut depth = 0usize;
    let mut patterns = false;
    for (j, tok) in toks.iter().enumerate().take(close).skip(at + 3) {
        match tok.kind {
            TokenKind::OpenParen | TokenKind::OpenBrace | TokenKind::OpenBracket => depth += 1,
            TokenKind::CloseParen | TokenKind::CloseBrace | TokenKind::CloseBracket => {
                depth = depth.saturating_sub(1);
            }
            TokenKind::Comma if depth == 0 => patterns = true,
            _ if patterns => push(j, Some(toks[at].line)),
            _ => {}
        }
    }
}

/// `["w", …].contains(…)` at `open`: an inline array of literals searched for
/// a word.
fn scan_searched_array(
    text: &str,
    toks: &[Tok],
    open: usize,
    push: &mut impl FnMut(usize, Option<usize>),
) {
    let Some(close) = matching_close(toks, open) else {
        return;
    };
    let searched = toks
        .get(close + 1)
        .is_some_and(|t| t.kind == TokenKind::Dot)
        && toks
            .get(close + 2)
            .is_some_and(|t| ident(text, t) == Some("contains"));
    let literal_list = toks[open + 1..close]
        .iter()
        .all(|t| matches!(t.kind, TokenKind::Comma | TokenKind::Literal { .. }));
    if searched && literal_list {
        for j in open + 1..close {
            push(j, Some(toks[open].line));
        }
    }
}

/// The arm patterns between `from` and `to` (the tokens inside a `match`'s
/// braces): each pattern runs from the arm's start to its `if` guard or
/// `=>`, and each arm body is skipped.
fn scan_arms(
    text: &str,
    toks: &[Tok],
    from: usize,
    to: usize,
    anchor: usize,
    push: &mut impl FnMut(usize, Option<usize>),
) {
    let mut j = from;
    while j < to {
        // The pattern.
        let mut in_guard = false;
        while j < to && !joined(toks, j, TokenKind::Eq, TokenKind::Gt) {
            match toks[j].kind {
                TokenKind::OpenParen | TokenKind::OpenBracket | TokenKind::OpenBrace => {
                    let close = matching_close(toks, j).unwrap_or(to).min(to);
                    if !in_guard {
                        for k in j + 1..close {
                            push(k, Some(anchor));
                        }
                    }
                    j = close + 1;
                    continue;
                }
                TokenKind::Ident if ident(text, &toks[j]) == Some("if") => in_guard = true,
                _ if !in_guard => push(j, Some(anchor)),
                _ => {}
            }
            j += 1;
        }
        // Past `=>`, the body: a block, or an expression up to its comma.
        j += 2;
        if j < to && toks[j].kind == TokenKind::OpenBrace {
            j = matching_close(toks, j).unwrap_or(to) + 1;
            if j < to && toks[j].kind == TokenKind::Comma {
                j += 1;
            }
            continue;
        }
        while j < to && toks[j].kind != TokenKind::Comma {
            if matches!(
                toks[j].kind,
                TokenKind::OpenParen | TokenKind::OpenBracket | TokenKind::OpenBrace
            ) {
                j = matching_close(toks, j).unwrap_or(to);
            }
            j += 1;
        }
        j += 1;
    }
}

/// A `const` or `static` string table at `at`: `NAME: &[&str] = &[…]` or
/// `NAME: [&str; N] = […]`, whose entries are sites when the file names the
/// table again.
fn scan_table(text: &str, toks: &[Tok], at: usize, push: &mut impl FnMut(usize, Option<usize>)) {
    let Some(name) = toks.get(at + 1).and_then(|t| ident(text, t)) else {
        return;
    };
    if toks.get(at + 2).is_none_or(|t| t.kind != TokenKind::Colon) {
        return;
    }
    // The type, up to `=`: it must be a slice or array of `&str`.
    let Some(eq) = (at + 3..toks.len()).find(|&j| toks[j].kind == TokenKind::Eq) else {
        return;
    };
    let ty: Vec<&str> = (at + 3..eq)
        .map(|j| &text[toks[j].start..toks[j].end])
        .collect();
    let str_table =
        matches!(
            ty.as_slice(),
            ["&", "[", "&", "str", "]"] | ["&", "[", "&", "'static", "str", "]"]
        ) || (ty.len() >= 5 && ty[..4] == ["[", "&", "str", ";"] && ty.last() == Some(&"]"));
    if !str_table {
        return;
    }
    let Some(open) = (eq + 1..toks.len()).find(|&j| toks[j].kind == TokenKind::OpenBracket) else {
        return;
    };
    let Some(close) = matching_close(toks, open) else {
        return;
    };
    // A `pub` table is read elsewhere by construction; a private one when the
    // file names it again.
    let exported = at.checked_sub(1).is_some_and(|j| {
        ident(text, &toks[j]) == Some("pub") || toks[j].kind == TokenKind::CloseParen
    });
    let referred_again = exported
        || toks
            .iter()
            .enumerate()
            .any(|(j, t)| j != at + 1 && ident(text, t) == Some(name));
    if referred_again {
        for j in open + 1..close {
            push(j, Some(toks[at].line));
        }
    }
}

// Waivers

/// What a waiver marker says.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Waiver {
    axis: String,
    reason: String,
    until: String,
}

/// Parse the text after a waiver marker:
/// `<axis> — <reason>; until <step N | slice N | never>`.
fn parse_waiver(after_marker: &str) -> Result<Waiver, String> {
    let text = after_marker.trim();
    let (axis, rest) = text
        .split_once(" — ")
        .or_else(|| text.split_once(" - "))
        .unwrap_or((text, ""));
    let axis = axis.trim();
    if !AXES.contains(&axis) {
        return Err(format!(
            "names unknown axis `{axis}` (one of {})",
            AXES.join(", ")
        ));
    }
    let Some((reason, until)) = rest.rsplit_once("; until ") else {
        return Err(format!(
            "on axis `{axis}` has no expiry: end it with `; until <step N | slice N | never>`"
        ));
    };
    let until = until.trim();
    let well_formed = until == "never"
        || ["step ", "slice "].iter().any(|prefix| {
            until
                .strip_prefix(prefix)
                .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
        });
    if !well_formed {
        return Err(format!(
            "expiry `{until}` is not `step N`, `slice N`, or `never`"
        ));
    }
    if until == "never" && axis != "irreducible" {
        return Err(format!(
            "on axis `{axis}` expires `never`, which only `irreducible` may"
        ));
    }
    if LANDED.contains(&until) {
        return Err(format!(
            "expires with `{until}`, which has landed: retire the site or re-review it"
        ));
    }
    Ok(Waiver {
        axis: axis.to_owned(),
        reason: reason.trim().to_owned(),
        until: until.to_owned(),
    })
}

/// The waiver whose marker is in the comment on `line`, its text continued
/// over the standalone comment lines below until the expiry is written.
fn marker_at(
    comments: &BTreeMap<usize, Comment>,
    line: usize,
    marker: &str,
) -> Option<Result<Waiver, String>> {
    let comment = comments.get(&line)?;
    if marker == WAIVER && comment.text.contains(FILE_WAIVER) {
        return None;
    }
    let at = comment.text.find(marker)?;
    let mut waiver = comment.text[at + marker.len()..].trim().to_owned();
    let mut next = line + 1;
    while comment.standalone && !waiver.contains("; until ") {
        let Some(more) = comments.get(&next).filter(|c| c.standalone) else {
            break;
        };
        let more = more
            .text
            .trim_start_matches('/')
            .trim_start_matches('!')
            .trim();
        if more.is_empty() || more.contains(WAIVER) || more.contains(FILE_WAIVER) {
            break;
        }
        waiver.push(' ');
        waiver.push_str(more);
        next += 1;
    }
    Some(parse_waiver(&waiver))
}

/// The waiver on `line`, or in the comment block directly above it.
fn waiver_at(comments: &BTreeMap<usize, Comment>, line: usize) -> Option<Result<Waiver, String>> {
    if let Some(found) = marker_at(comments, line, WAIVER) {
        return Some(found);
    }
    let mut above = line.checked_sub(1)?;
    let mut block = Vec::new();
    while comments.get(&above).is_some_and(|c| c.standalone) {
        block.push(above);
        above = above.checked_sub(1)?;
    }
    block
        .into_iter()
        .rev()
        .find_map(|at| marker_at(comments, at, WAIVER))
}

/// The site's waiver: on its line or directly above it, or — for an arm, a
/// `matches!` pattern or a table entry — on or above the enclosing construct.
fn site_waiver(comments: &BTreeMap<usize, Comment>, hit: &Hit) -> Option<Result<Waiver, String>> {
    waiver_at(comments, hit.line).or_else(|| hit.anchor.and_then(|at| waiver_at(comments, at)))
}

/// The file-level waiver in the file's leading comments, when it has one.
fn file_waiver(comments: &BTreeMap<usize, Comment>) -> Option<Result<Waiver, String>> {
    comments
        .range(..=80)
        .find(|(_, comment)| comment.text.contains(FILE_WAIVER))
        .and_then(|(&line, _)| marker_at(comments, line, FILE_WAIVER))
}

// The ratchet

/// The ratchet's verdicts on `lint` against `pins` and `clean`: a count above
/// its pin, with the file's sites; a pin above its count; a pin on a clean
/// file; a file pinned twice.
fn ratchet_verdicts(lint: &Lint, pins: &[(&str, usize)], clean: &[&str]) -> Vec<String> {
    let mut verdicts = Vec::new();
    let mut pinned: BTreeMap<&str, usize> = BTreeMap::new();
    for (path, pin) in pins {
        if pinned.insert(path, *pin).is_some() {
            verdicts.push(format!("RATCHET pins `{path}` twice"));
        }
        if clean.contains(path) {
            verdicts.push(format!(
                "`{path}` is held clean and carries no pin; remove it from RATCHET"
            ));
        }
    }
    for (path, sites) in &lint.ratchet_hits {
        let pin = pinned.get(path.as_str()).copied().unwrap_or(0);
        if sites.len() > pin {
            let listed: Vec<String> = sites.iter().map(|site| format!("  {site}")).collect();
            verdicts.push(format!(
                "`{path}` has {} unwaived site(s) against a pin of {pin}; the count may only \
                 fall — ask the registry's axis query, or waive a reviewed site with \
                 `// registry-axis-ok: <axis> — <reason>; until <expiry>`:\n{}",
                sites.len(),
                listed.join("\n")
            ));
        }
    }
    for (path, pin) in &pinned {
        let count = lint.ratchet_hits.get(*path).map_or(0, Vec::len);
        if count < *pin {
            verdicts.push(format!(
                "`{path}` has {count} unwaived site(s) against a pin of {pin}; lower the pin in \
                 RATCHET (rust/xtask/src/registry_axes.rs)"
            ));
        }
    }
    verdicts
}

// The report

fn render_report(lint: &Lint) -> String {
    let mut out = String::new();
    out.push_str("<!-- Generated by `cargo xtask registry-axes`; do not edit. -->\n\n");
    out.push_str("# Registry axes — the per-axis ledger\n\n");
    out.push_str(
        "Every site outside `tcl-registry` that compares a word the registry declares — a \
         command or subcommand name, an option spelling, a definition-body member keyword, a \
         clause keyword, or a special-variable name — found by the source lint of \
         `docs/design/compiler/registry-consumer-contracts.md` § *The per-axis lint and \
         ledger*. A reviewed site names the axis whose query retires it and the build step \
         (`step N`) or value-transfer slice (`slice N`) that will; `never` is the sanctioned \
         irreducible exception. Every other site is counted against its file's pin.\n\n",
    );
    render_ledger(&mut out, lint);
    render_ratchet(&mut out, lint);
    out
}

fn render_ledger(out: &mut String, lint: &Lint) {
    let _ = writeln!(out, "## The ledger\n");
    let mut by_axis: BTreeMap<&str, usize> = BTreeMap::new();
    for waived in &lint.waived {
        *by_axis.entry(waived.axis.as_str()).or_default() += 1;
    }
    let _ = writeln!(out, "| Axis | Waived sites |");
    let _ = writeln!(out, "|---|---|");
    for axis in AXES {
        let _ = writeln!(
            out,
            "| {axis} | {} |",
            by_axis.get(axis).copied().unwrap_or(0)
        );
    }
    let _ = writeln!(out, "\n| Axis | Until | Site | Waiver | Reason |");
    let _ = writeln!(out, "|---|---|---|---|---|");
    for waived in &lint.waived {
        let _ = writeln!(
            out,
            "| {} | {} | `{}:{}` | {} | {} |",
            waived.axis,
            waived.until,
            waived.site.path,
            waived.site.line,
            if waived.file_level { "file" } else { "site" },
            waived.reason.replace('|', "\\|"),
        );
    }
}

fn render_ratchet(out: &mut String, lint: &Lint) {
    let _ = writeln!(out, "\n## The ratchet\n");
    let clean: Vec<String> = CLEAN_FILES.iter().map(|file| format!("`{file}`")).collect();
    let _ = writeln!(
        out,
        "The files the gate holds clean, with every site waived or gone: {}.\n\nEvery other \
         scanned file with an unwaived site, and its count, which is the pin in \
         `rust/xtask/src/registry_axes.rs`. The count may only fall: the change that removes \
         or waives a file's sites lowers its pin.\n",
        if clean.is_empty() {
            "none yet".to_owned()
        } else {
            clean.join(", ")
        }
    );
    let _ = writeln!(out, "| File | Unwaived sites |");
    let _ = writeln!(out, "|---|---|");
    for (path, sites) in &lint.ratchet_hits {
        let _ = writeln!(out, "| `{path}` | {} |", sites.len());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vocabulary_of(words: &[(&str, &'static str)]) -> Vocabulary {
        let mut vocabulary = Vocabulary::default();
        for (word, axis) in words {
            vocabulary.add(word, axis);
        }
        vocabulary
    }

    fn sample() -> Vocabulary {
        vocabulary_of(&[
            ("elseif", "clause_grammar"),
            ("else", "clause_grammar"),
            ("then", "clause_grammar"),
            ("method", "definition_body"),
            ("constructor", "definition_body"),
            ("-nocase", "options"),
            ("-exact", "options"),
            ("set", "command"),
            ("auto_path", "special_vars"),
        ])
    }

    fn lines(src: &str) -> Vec<usize> {
        scan(src, &sample())
            .into_iter()
            .map(|hit| hit.line)
            .collect()
    }

    #[test]
    fn a_clause_keyword_compared_by_spelling_is_a_site() {
        let src = "fn f(word: &str, next: Option<&str>) -> bool {\n    word == \"elseif\"\n        || \"else\" != word\n        || next == Some(\"then\")\n        || word.eq(\"else\")\n        || matches!(word, \"elseif\" | \"else\")\n        || [\"elseif\", \"else\"].contains(&word)\n}\n";
        assert_eq!(lines(src), vec![2, 3, 4, 5, 6, 7]);
        // A word the registry does not declare, a longer string, a comment and
        // a non-comparison use are not sites.
        let quiet = "fn g(word: &str) -> String {\n    // word == \"elseif\"\n    let _ = word == \"elsewhere\";\n    let _ = word == \"elseif branch\";\n    format!(\"elseif {word}\")\n}\n";
        assert!(lines(quiet).is_empty(), "{:?}", lines(quiet));
    }

    #[test]
    fn a_member_keyword_in_a_match_arm_is_a_site() {
        let src = "fn f(keyword: &str, body: &str) -> u8 {\n    match keyword {\n        \"method\" => 1,\n        \"constructor\" | \"destroy\" if body == \"x\" => {\n            2\n        }\n        other => match other { \"set\" => 3, _ => 4 },\n    }\n}\n";
        let hits = scan(src, &sample());
        let found: Vec<(usize, &str)> = hits.iter().map(|h| (h.line, h.word.as_str())).collect();
        assert_eq!(found, vec![(3, "method"), (4, "constructor"), (7, "set")]);
        assert_eq!(
            hits[0].anchor,
            Some(2),
            "an arm's waiver may sit above its match"
        );
        // A literal in an arm body is not a pattern.
        let body = "fn g(x: u8) -> &'static str {\n    match x {\n        0 => \"method\",\n        _ => \"else\",\n    }\n}\n";
        assert!(lines(body).is_empty(), "{:?}", lines(body));
    }

    #[test]
    fn an_option_spelling_in_a_const_table_is_a_site() {
        let src = "const MODES: &[&str] = &[\n    \"-exact\",\n    \"-nocase\",\n    \"-other\",\n];\nfn f(word: &str) -> bool {\n    MODES.contains(&word)\n}\n";
        let hits = scan(src, &sample());
        assert_eq!(hits.iter().map(|h| h.line).collect::<Vec<_>>(), vec![2, 3]);
        assert!(hits.iter().all(|h| h.anchor == Some(1)));
        // A table nothing reads is not compared later.
        let unused = "const MODES: &[&str] = &[\"-exact\", \"-nocase\"];\n";
        assert!(lines(unused).is_empty());
        // A table of another type is not a string table.
        let other =
            "const PAIRS: &[(&str, u8)] = &[(\"-exact\", 1)];\nfn f() -> usize { PAIRS.len() }\n";
        assert!(lines(other).is_empty());
    }

    #[test]
    fn a_waiver_without_an_expiry_fails() {
        assert!(
            parse_waiver("clause_grammar — the orphaned-keyword table")
                .unwrap_err()
                .contains("no expiry")
        );
        assert!(
            parse_waiver("options — the scan; until step 2")
                .is_ok_and(|w| w.axis == "options" && w.until == "step 2")
        );
        assert!(
            parse_waiver("colour — x; until step 2")
                .unwrap_err()
                .contains("unknown axis")
        );
        assert!(
            parse_waiver("options — x; until never")
                .unwrap_err()
                .contains("only `irreducible`")
        );
        assert!(parse_waiver("irreducible — Tcl grammar; until never").is_ok());
        assert!(
            parse_waiver("options — x; until slice 4")
                .unwrap_err()
                .contains("has landed")
        );
        assert!(
            parse_waiver("options — x; until tomorrow")
                .unwrap_err()
                .contains("not `step N`")
        );
        // Found on the line, in the block above, and above an enclosing match.
        let src = "// registry-axis-ok: clause_grammar — the walk; until step 2\nfn f(w: &str) -> bool { w == \"else\" }\nfn g(w: &str) -> bool { w == \"then\" } // registry-axis-ok: irreducible — Tcl grammar; until never\n// registry-axis-ok: definition_body — the arm table; until step 2\nfn h(w: &str) -> u8 {\n    match w {\n        \"method\" => 1,\n        _ => 0,\n    }\n}\nfn i(w: &str) -> bool { w == \"set\" }\n";
        let comments = line_comments(src);
        let hits = scan(src, &sample());
        assert_eq!(hits.len(), 4);
        let axes: Vec<Option<String>> = hits
            .iter()
            .map(|hit| {
                site_waiver(&comments, hit)
                    .and_then(Result::ok)
                    .map(|w| w.axis)
            })
            .collect();
        assert_eq!(
            axes,
            vec![
                Some("clause_grammar".to_owned()),
                Some("irreducible".to_owned()),
                None,
                None
            ]
        );
    }

    #[test]
    fn an_enclosing_match_carries_its_arms_waiver() {
        let src = "fn h(w: &str) -> u8 {\n    // registry-axis-ok: definition_body — the arm table; until step 2\n    match w {\n        \"method\" => 1,\n        _ => 0,\n    }\n}\n";
        let comments = line_comments(src);
        let hits = scan(src, &sample());
        assert_eq!(hits.len(), 1);
        assert!(site_waiver(&comments, &hits[0]).is_some_and(|w| w.is_ok()));
    }

    #[test]
    fn a_registry_owner_file_is_never_scanned() {
        for excluded in [
            "rust/tcl-registry/src/registry.rs",
            "rust/tcl-registry/src/commands/tcl/if_.rs",
            "rust/tcl-compiler/tests/analyser.rs",
            "rust/tcl-lsp-core/src/formatting/tests.rs",
            "rust/tcl-compiler/src/lowering/tests/structured.rs",
        ] {
            assert!(!is_scanned(excluded), "{excluded}");
        }
        assert!(is_scanned("rust/tcl-compiler/src/lowering/structured.rs"));
        // A `#[cfg(test)]` item is skipped, and scanning resumes after it.
        let src = "fn f(w: &str) -> bool { w == \"else\" }\n#[cfg(test)]\nmod tests {\n    fn g(w: &str) -> bool { w == \"then\" }\n}\nfn h(w: &str) -> bool { w == \"set\" }\n";
        assert_eq!(lines(src), vec![1, 6]);
    }

    fn lint_with(hits: &[(&str, usize)]) -> Lint {
        let mut lint = Lint::default();
        for (path, count) in hits {
            let sites = (1..=*count)
                .map(|line| Site {
                    path: (*path).to_owned(),
                    line,
                    snippet: String::new(),
                })
                .collect();
            lint.ratchet_hits.insert((*path).to_owned(), sites);
        }
        lint
    }

    #[test]
    fn the_ratchet_only_falls() {
        let lint = lint_with(&[("rust/x/a.rs", 3), ("rust/x/new.rs", 1)]);
        let verdicts = ratchet_verdicts(
            &lint,
            &[
                ("rust/x/a.rs", 2),
                ("rust/x/gone.rs", 1),
                ("rust/x/clean.rs", 0),
            ],
            &["rust/x/clean.rs"],
        );
        assert!(
            verdicts
                .iter()
                .any(|v| v.contains("3 unwaived site(s) against a pin of 2"))
        );
        assert!(verdicts.iter().any(|v| v.contains("`rust/x/new.rs` has 1")));
        assert!(
            verdicts
                .iter()
                .any(|v| v.contains("gone.rs` has 0") && v.contains("lower the pin"))
        );
        assert!(verdicts.iter().any(|v| v.contains("held clean")));
        assert!(
            ratchet_verdicts(
                &lint_with(&[("rust/x/a.rs", 2)]),
                &[("rust/x/a.rs", 2)],
                &[]
            )
            .is_empty()
        );
    }

    #[test]
    fn the_shipped_vocabulary_covers_every_axis() {
        let root = repo_root();
        let vocabulary = vocabulary(&full_registry(&root));
        for (word, axis) in [
            ("dict", "command"),
            ("length", "subcommands"),
            ("-nocase", "options"),
            ("constructor", "definition_body"),
            ("elseif", "clause_grammar"),
            ("then", "clause_grammar"),
            ("auto_path", "special_vars"),
        ] {
            assert!(
                vocabulary
                    .axes(word)
                    .is_some_and(|axes| axes.contains(axis)),
                "`{word}` should be a `{axis}` word"
            );
        }
    }

    #[test]
    fn every_file_is_clean_or_at_its_pin() {
        let root = repo_root();
        let lint = lint(&root, &vocabulary(&full_registry(&root)));
        assert!(
            lint.clean_hits.is_empty(),
            "unwaived sites in a clean file: {:?}",
            lint.clean_hits
        );
        assert!(lint.problems.is_empty(), "{:?}", lint.problems);
        let verdicts = ratchet_verdicts(&lint, RATCHET, CLEAN_FILES);
        assert!(verdicts.is_empty(), "{verdicts:#?}");
    }
}
