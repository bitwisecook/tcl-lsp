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

//! **P0 harness** — every `samples/wasm` tier script compiled, linked against
//! the real runtime, run under wasmtime, and diffed against its `tclsh9.0`
//! oracle; plus the committed framing budgets each later phase must reduce.
//!
//! This is the acceptance surface for the phased plan in
//! [`docs/design/compiler/wasm-native-lowering-plan.md`] (§2.2 for today's
//! divergences, §7 row P0 for what this file owes the programme). It answers
//! two questions on every commit:
//!
//! 1. **Does the compiled program still mean what Tcl means?** Each
//!    `samples/wasm/t*/*.tcl` is compiled with
//!    [`WasmCompileOptions::standalone(true)`] — a WASI command that creates an
//!    interp, loads the embedded stdlib, and runs `::top` — linked against the
//!    real `tcl_runtime.wasm` (`wasmtime run --preload tcl=…`), and its stdout
//!    compared byte for byte with `samples/wasm/expected/<tier>/<name>.out`.
//!    Both plans are covered: the default one and the opt-in
//!    [`SemanticOptimisationPassId::LegacyAnalysisSpecialisation`] tier.
//! 2. **How much Tcl framing is left?** [`samples/wasm/budgets.tsv`] records,
//!    per sample and plan, the number of `call` sites reaching
//!    `tcl_eval_code` / `tcl_expr_bool` / `tcl_invoke_argv` and the number of
//!    native 64-bit numeric instructions. Every phase's framing reduction then
//!    lands as a *reviewed golden diff* rather than an unmeasured claim.
//!
//! # The expected-divergence table is a defect ledger, not a tolerance
//!
//! Seven of the seventy-two (sample × plan) runs do not match tclsh today, and
//! each one is a *known, filed* defect — see [`EXPECTED_DIVERGENCES`], which
//! carries the reason beside every entry. The table is checked in **both**
//! directions:
//!
//! - a sample that diverges without a table entry fails (a regression);
//! - a sample with a table entry that *stops* diverging also fails (the defect
//!   is fixed, so the entry is stale and must be deleted in the same commit
//!   that fixes it).
//!
//! Only the second half makes the ledger self-cleaning. An expected-failure
//! list that silently absorbs a fix rots into a list of things nobody
//! remembers were ever broken, and the phase that fixed one gets no credit and
//! no gate.
//!
//! # Running it
//!
//! Heavy + gated exactly as `wasm_real_link.rs` is, through the shared
//! [`common::wasm_link`] helpers: the `wasmtime` CLI, the `wasm32-wasip1`
//! target, wasi-sdk and the libtommath source must all be present, the skip
//! names whichever is missing, and **`TCL_REQUIRE_WASM_LINK=1`** turns that
//! skip into a failure. Set it in CI; a silent skip here is indistinguishable
//! from a pass (issue #1542).
//!
//! The budgets test needs none of that — it only compiles — so framing drift
//! is caught even on a machine with no wasm toolchain at all.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;

use tcl_compiler::codegen::wasm::{
    SemanticOptimisationPassId, WasmCompileOptions, WasmModule, WasmOp, compile_wasm,
};
use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_registry::CommandRegistry;
use tcl_runtime_api::codegen_abi::CodegenAbiImportId;

mod common;
use common::wasm_link::{REQUIRE_VAR, real_link_runtime, scratch, workspace_root};

/// Set this to rewrite [`BUDGETS_PATH`] from the current emitter instead of
/// asserting against it. Reviewing that diff is the point of the golden.
const UPDATE_VAR: &str = "UPDATE_WASM_BUDGETS";

/// The committed framing-budget golden, relative to the workspace root.
const BUDGETS_PATH: &str = "samples/wasm/budgets.tsv";

/// Which compilation plan a run used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Plan {
    /// The shipped default: no semantic optimisation pass enabled.
    Default,
    /// The opt-in analysis-derived specialisation tier.
    Analysis,
}

impl Plan {
    /// Both plans, in the order the golden table records them.
    const ALL: [Self; 2] = [Self::Default, Self::Analysis];

    /// The name used in the golden table, the divergence ledger, and messages.
    const fn as_str(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Analysis => "analysis",
        }
    }

    /// Standalone packaging (`_start` + stdlib bootstrap, so the linked module
    /// runs `::top` under wasmtime on its own), plus this plan's pass set.
    fn options(self) -> WasmCompileOptions {
        let options = WasmCompileOptions::standalone(true);
        match self {
            Self::Default => options,
            Self::Analysis => options.with_semantic_optimisation(
                SemanticOptimisationPassId::LegacyAnalysisSpecialisation,
            ),
        }
    }
}

/// One `samples/wasm` script: its tier directory and its stem.
#[derive(Debug, Clone)]
struct Sample {
    /// Tier directory name, e.g. `t3-procs`.
    tier: String,
    /// File stem, e.g. `31_recursion`.
    name: String,
    /// Absolute path to the `.tcl` source.
    source: PathBuf,
    /// Absolute path to the committed `tclsh9.0` oracle.
    oracle: PathBuf,
}

/// A sample/plan pair that is known not to match tclsh today, with the reason.
///
/// `why` is the *defect*, not a shrug: every entry names the issue or the §2.2
/// review finding that explains it and the phase that will remove it. An entry
/// with no such reason does not belong here — it belongs in a bug report.
#[derive(Debug, Clone, Copy)]
struct ExpectedDivergence {
    tier: &'static str,
    name: &'static str,
    plan: Plan,
    why: &'static str,
}

/// The complete ledger of known divergences: 34/36 on the default plan,
/// 30/36 on the analysis plan.
///
/// The default-tier pair are **runtime** defects that the compiled path merely
/// exposes. The analysis-tier additions are all one **codegen** defect, §2.2's
/// `puts` compatibility-text reparse.
///
/// §2.2 of the plan document records 29/36 for the analysis plan and lists
/// `50_catch_error` alongside these. It is no longer here: the P1 lane's
/// "a compiled activation is an eval-loop activation" change closed §2.2's
/// second defect, and this suite's stale-entry check is what caught the
/// ledger row going out of date the moment it did. The plan document's table
/// is the older reading.
const EXPECTED_DIVERGENCES: &[ExpectedDivergence] = &[
    ExpectedDivergence {
        tier: "t7-dynamic",
        name: "70_var_traces",
        plan: Plan::Analysis,
        why: "issue #1772: the script's final `puts \"$a $b $c\"` is a quoted \
              word with three substitutions, and the analysis tier's `puts` \
              fast path re-parses its compatibility text into a bogus variable \
              name, so `::top` stops after the first line. Fixed in P3.",
    },
    ExpectedDivergence {
        tier: "t7-dynamic",
        name: "73_coroutine",
        plan: Plan::Default,
        why: "the wasm build refuses `coroutine` outright (\"coroutines are not \
              supported in the single-threaded wasm build\"). Stack switching \
              for wasm is P9's own design note.",
    },
    ExpectedDivergence {
        tier: "t7-dynamic",
        name: "73_coroutine",
        plan: Plan::Analysis,
        why: "same missing wasm `coroutine` support as the default plan. P9.",
    },
    ExpectedDivergence {
        tier: "t1-expr-control",
        name: "11_while_loop",
        plan: Plan::Analysis,
        why: "§2.2 defect 1 — the `puts` fast path re-parses compatibility \
              text. `try_emit_direct_operation` admits `ChannelWrite` whenever \
              `whole_var_reference` succeeds, and for a quoted word with two \
              substitutions that helper strips the outer `${`…`}` and returns a \
              bogus name, so the emitted var-get fails and `::top` stops \
              silently. Fixed in P3 (structured `WordExpr`, \
              `whole_var_reference` retired from codegen).",
    },
    ExpectedDivergence {
        tier: "t2-values",
        name: "20_lists",
        plan: Plan::Analysis,
        why: "§2.2 defect 1, same `puts` compatibility-text reparse. P3.",
    },
    ExpectedDivergence {
        tier: "t2-values",
        name: "24_regex",
        plan: Plan::Analysis,
        why: "§2.2 defect 1, same `puts` compatibility-text reparse. P3.",
    },
    ExpectedDivergence {
        tier: "t4-scopes",
        name: "41_upvar",
        plan: Plan::Analysis,
        why: "§2.2 defect 1, same `puts` compatibility-text reparse. P3.",
    },
];

/// The framing counts recorded for one sample/plan pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Budget {
    /// `call` sites reaching `tcl_eval_code` — run-time re-parse of source.
    eval_code: usize,
    /// `call` sites reaching `tcl_expr_bool` — a condition through the interp.
    expr_bool: usize,
    /// `call` sites reaching `tcl_invoke_argv` — compiled words, runtime
    /// dispatch.
    invoke_argv: usize,
    /// Native 64-bit numeric instructions (see [`is_native_numeric`]).
    native_numeric: usize,
}

/// Whether an opcode is native 64-bit numeric *work*.
///
/// Everything the module computes in `i64`/`f64` registers rather than through
/// the Tcl value tower: arithmetic, bitwise and comparison. Pure data movement
/// (`const`/`load`/`store`) and representation changes (`extend_i32_s`) are
/// excluded — they move a value, they do not compute one, and counting them
/// would let a budget rise without a single Tcl operation having been lowered.
///
/// Written as a prefix rule over [`WasmOp::wat_name`] rather than an explicit
/// opcode list so that the `f64.*` arithmetic P3 adds is counted the day it is
/// emitted, with no second edit here to forget.
fn is_native_numeric(op: WasmOp) -> bool {
    let name = op.wat_name();
    let Some(rest) = name
        .strip_prefix("i64.")
        .or_else(|| name.strip_prefix("f64."))
    else {
        return false;
    };
    !matches!(rest, "const" | "load" | "store" | "extend_i32_s")
}

/// Decode one unsigned LEB128 value from an instruction's pre-encoded operand
/// bytes.
///
/// The budget walks the emitter's own [`WasmModule`] rather than its WAT
/// rendering, so a `call`'s target arrives as raw LEB128. Regexing the WAT
/// would count a call named in a comment or a data string, and would silently
/// start counting nothing at all the day the WAT formatter changes shape.
fn decode_uleb128(bytes: &[u8]) -> Option<u64> {
    let mut value: u64 = 0;
    let mut shift = 0;
    for byte in bytes {
        value |= u64::from(byte & 0x7F) << shift;
        if byte & 0x80 == 0 {
            return Some(value);
        }
        shift += 7;
        if shift >= 64 {
            return None;
        }
    }
    None
}

/// Count the framing calls and native numeric work in an emitted module.
///
/// Imports occupy function indices `0..imports.len()`, so a `call` whose
/// operand decodes to an import's position is a call *to that import*. The
/// three framing imports are named through [`CodegenAbiImportId`] rather than
/// by string literal, so renaming one in the shared ABI descriptor breaks this
/// at compile time instead of silently zeroing a budget column.
fn budget_of(module: &WasmModule) -> Budget {
    let index_of = |id: CodegenAbiImportId| -> Option<u64> {
        let want = id.descriptor().name;
        module
            .imports
            .iter()
            .position(|import| import.name == want)
            .map(|index| index as u64)
    };
    let eval_code = index_of(CodegenAbiImportId::EvalCode);
    let expr_bool = index_of(CodegenAbiImportId::ExprBool);
    let invoke_argv = index_of(CodegenAbiImportId::InvokeArgv);

    let mut budget = Budget {
        eval_code: 0,
        expr_bool: 0,
        invoke_argv: 0,
        native_numeric: 0,
    };
    for function in &module.functions {
        for instruction in &function.body {
            if instruction.op == WasmOp::Call {
                let target = decode_uleb128(&instruction.operands);
                if target.is_some() {
                    if target == eval_code {
                        budget.eval_code += 1;
                    }
                    if target == expr_bool {
                        budget.expr_bool += 1;
                    }
                    if target == invoke_argv {
                        budget.invoke_argv += 1;
                    }
                }
            } else if is_native_numeric(instruction.op) {
                budget.native_numeric += 1;
            }
        }
    }
    budget
}

/// Every `samples/wasm/t*/*.tcl`, tier-then-name sorted, each paired with the
/// oracle it must reproduce.
///
/// A missing oracle is a hard failure rather than a skipped row: the whole
/// value of this suite is that no sample can quietly stop being checked.
fn samples() -> Vec<Sample> {
    let root = workspace_root().join("samples/wasm");
    let mut tiers: Vec<PathBuf> = std::fs::read_dir(&root)
        .unwrap_or_else(|e| panic!("read {}: {e}", root.display()))
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_dir()
                && path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with('t') && n[1..2].parse::<u8>().is_ok())
        })
        .collect();
    tiers.sort();

    let mut samples = Vec::new();
    for tier_dir in tiers {
        let tier = tier_dir
            .file_name()
            .and_then(|n| n.to_str())
            .expect("tier name")
            .to_owned();
        let mut scripts: Vec<PathBuf> = std::fs::read_dir(&tier_dir)
            .unwrap_or_else(|e| panic!("read {}: {e}", tier_dir.display()))
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "tcl"))
            .collect();
        scripts.sort();
        for source in scripts {
            let name = source
                .file_stem()
                .and_then(|n| n.to_str())
                .expect("sample name")
                .to_owned();
            let oracle = root
                .join("expected")
                .join(&tier)
                .join(format!("{name}.out"));
            assert!(
                oracle.is_file(),
                "{tier}/{name} has no committed tclsh oracle at {} — regenerate it \
                 with the recipe in samples/wasm/README.md; a sample with no \
                 oracle is a sample nothing checks",
                oracle.display()
            );
            samples.push(Sample {
                tier: tier.clone(),
                name,
                source,
                oracle,
            });
        }
    }
    assert!(
        !samples.is_empty(),
        "no samples found under {} — this suite would pass vacuously",
        root.display()
    );
    samples
}

/// Compile one sample under `plan`, returning the emitted module.
fn compile(sample: &Sample, plan: Plan) -> WasmModule {
    let source = std::fs::read_to_string(&sample.source)
        .unwrap_or_else(|e| panic!("read {}: {e}", sample.source.display()));
    let registry = CommandRegistry::build_default();
    let unit = CompilationUnit::build_for_dialect(&source, &registry, false, "tcl9.0");
    compile_wasm(&unit, &registry, plan.options()).module
}

/// What one linked run produced.
struct RunOutcome {
    stdout: String,
    stderr: String,
    exited_cleanly: bool,
}

/// Link one compiled sample against the real runtime and run it under
/// wasmtime.
///
/// The standalone packaging already exports `_start` and imports the runtime
/// ABI from module `"tcl"`, so composition is the same two-module `--preload`
/// the plan document specifies — no hand-written bootstrap is needed here, and
/// none should be added: the point of this suite is that the *shipped*
/// packaging runs.
fn run_linked(runtime: &Path, sample: &Sample, plan: Plan) -> RunOutcome {
    let mut module = compile(sample, plan);
    let tag = format!("{}_{}_{}", sample.tier, sample.name, plan.as_str());
    let path = scratch(&format!("wasm_tiers_{tag}.wasm"));
    std::fs::write(&path, module.to_bytes())
        .unwrap_or_else(|e| panic!("write {}: {e}", path.display()));

    let out = Command::new("wasmtime")
        .arg("run")
        .arg("--preload")
        .arg(format!("tcl={}", runtime.display()))
        .arg(&path)
        .output()
        .expect("run wasmtime");
    let _ = std::fs::remove_file(&path);

    RunOutcome {
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        exited_cleanly: out.status.success(),
    }
}

/// The ledger entry for this sample/plan pair, if there is one.
fn expected_divergence(sample: &Sample, plan: Plan) -> Option<&'static ExpectedDivergence> {
    EXPECTED_DIVERGENCES
        .iter()
        .find(|entry| entry.tier == sample.tier && entry.name == sample.name && entry.plan == plan)
}

/// Run every sample under one plan and reconcile the results with
/// [`EXPECTED_DIVERGENCES`] in both directions.
fn check_plan(plan: Plan) {
    let Some(runtime) = real_link_runtime() else {
        return;
    };
    let samples = samples();
    let mut regressions: Vec<String> = Vec::new();
    let mut stale: Vec<String> = Vec::new();
    let mut matched = 0usize;

    for sample in &samples {
        let expected = std::fs::read_to_string(&sample.oracle)
            .unwrap_or_else(|e| panic!("read {}: {e}", sample.oracle.display()));
        let outcome = run_linked(&runtime, sample, plan);
        let agrees = outcome.exited_cleanly && outcome.stdout == expected;
        let id = format!("{}/{} [{}]", sample.tier, sample.name, plan.as_str());

        match (agrees, expected_divergence(sample, plan)) {
            (true, None) => matched += 1,
            (true, Some(entry)) => stale.push(format!(
                "  {id} now MATCHES tclsh — delete its EXPECTED_DIVERGENCES entry \
                 in the same commit that fixed it.\n    the entry says: {}",
                entry.why
            )),
            (false, Some(_)) => {}
            (false, None) => regressions.push(format!(
                "  {id} diverges from {}\n    --- expected ---\n{}\n    --- actual (exit \
                 ok: {}) ---\n{}\n    --- stderr ---\n{}",
                sample.oracle.display(),
                indent(&expected),
                outcome.exited_cleanly,
                indent(&outcome.stdout),
                indent(&outcome.stderr),
            )),
        }
    }

    let listed = EXPECTED_DIVERGENCES
        .iter()
        .filter(|entry| entry.plan == plan)
        .count();
    let unknown: Vec<&ExpectedDivergence> = EXPECTED_DIVERGENCES
        .iter()
        .filter(|entry| entry.plan == plan)
        .filter(|entry| {
            !samples
                .iter()
                .any(|s| s.tier == entry.tier && s.name == entry.name)
        })
        .collect();

    let mut report = String::new();
    for line in regressions.iter().chain(stale.iter()) {
        report.push_str(line);
        report.push('\n');
    }
    for entry in unknown {
        let _ = writeln!(
            report,
            "  {}/{} [{}] is in EXPECTED_DIVERGENCES but is not a sample — \
             the ledger names a script that no longer exists",
            entry.tier,
            entry.name,
            plan.as_str()
        );
    }
    assert!(
        report.is_empty(),
        "the {} plan does not agree with the expected-divergence ledger \
         (§2.2 of docs/design/compiler/wasm-native-lowering-plan.md):\n{report}",
        plan.as_str()
    );
    eprintln!(
        "{} plan: {matched}/{} samples byte-identical to tclsh 9.0, {listed} known divergences",
        plan.as_str(),
        samples.len(),
    );
}

/// Indent a captured stream so it cannot be mistaken for the assertion's own
/// structure in a failure report.
fn indent(text: &str) -> String {
    if text.is_empty() {
        return "      <empty>".to_owned();
    }
    text.lines()
        .map(|line| format!("      {line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn default_plan_samples_reproduce_the_tclsh_oracles() {
    check_plan(Plan::Default);
}

#[test]
fn analysis_plan_samples_reproduce_the_tclsh_oracles() {
    check_plan(Plan::Analysis);
}

/// Render the current framing budgets as the golden table's exact bytes.
fn render_budgets() -> String {
    let mut out = String::new();
    out.push_str(
        "# Framing budgets for samples/wasm — GOLDEN, regenerate with:\n\
         #   UPDATE_WASM_BUDGETS=1 cargo test -p tcl-compiler --test wasm_tiers \\\n\
         #       framing_budgets\n\
         #\n\
         # One row per sample per compilation plan. eval_code / expr_bool /\n\
         # invoke_argv count `call` sites reaching those runtime imports in the\n\
         # emitted module: source re-parsed at run time, a condition evaluated by\n\
         # the interpreter, and compiled words dispatched at run time. native_i64_f64\n\
         # counts native 64-bit numeric instructions (arithmetic, bitwise and\n\
         # comparison; not const/load/store/extend).\n\
         #\n\
         # Every phase of docs/design/compiler/wasm-native-lowering-plan.md moves\n\
         # these numbers, and a reviewed diff to this file is how that reduction is\n\
         # claimed. A number that goes UP without a matching design change is a\n\
         # framing regression.\n",
    );
    out.push_str("tier\tsample\tplan\teval_code\texpr_bool\tinvoke_argv\tnative_i64_f64\n");
    for sample in &samples() {
        for plan in Plan::ALL {
            let budget = budget_of(&compile(sample, plan));
            let _ = writeln!(
                out,
                "{}\t{}\t{}\t{}\t{}\t{}\t{}",
                sample.tier,
                sample.name,
                plan.as_str(),
                budget.eval_code,
                budget.expr_bool,
                budget.invoke_argv,
                budget.native_numeric,
            );
        }
    }
    out
}

/// The framing budgets are a committed golden: drift fails, and the fix is a
/// reviewed regeneration.
///
/// This test deliberately needs **no** wasm toolchain — it only compiles — so
/// the framing measurement runs in every partition that builds `tcl-compiler`,
/// not just the one job that installs wasmtime.
#[test]
fn framing_budgets_match_the_committed_golden() {
    let path = workspace_root().join(BUDGETS_PATH);
    let rendered = render_budgets();

    if std::env::var(UPDATE_VAR).is_ok_and(|v| v != "0" && !v.is_empty()) {
        std::fs::write(&path, &rendered)
            .unwrap_or_else(|e| panic!("write {}: {e}", path.display()));
        eprintln!("{UPDATE_VAR} set — rewrote {}", path.display());
        return;
    }

    let committed = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "read {}: {e}\nRegenerate it with `{UPDATE_VAR}=1 cargo test -p tcl-compiler \
             --test wasm_tiers framing_budgets`.",
            path.display()
        )
    });
    if committed == rendered {
        return;
    }

    let mut drift = String::new();
    let committed_rows: Vec<&str> = committed.lines().filter(|l| !l.starts_with('#')).collect();
    let rendered_rows: Vec<&str> = rendered.lines().filter(|l| !l.starts_with('#')).collect();
    for index in 0..committed_rows.len().max(rendered_rows.len()) {
        let was = committed_rows.get(index).copied().unwrap_or("<missing>");
        let now = rendered_rows.get(index).copied().unwrap_or("<missing>");
        if was != now {
            let _ = writeln!(drift, "  golden: {was}\n  emitted: {now}");
        }
    }
    panic!(
        "the framing budgets in {BUDGETS_PATH} no longer match what the emitter \
         produces:\n{drift}\nIf this is an intended framing change, regenerate and \
         review the diff:\n  {UPDATE_VAR}=1 cargo test -p tcl-compiler --test wasm_tiers \
         framing_budgets\nA budget that goes UP is a framing regression and needs a \
         reason in the commit message."
    );
}

/// The ledger must stay honest about itself: no duplicate rows, and every
/// entry must carry a reason a reader can act on.
#[test]
fn the_expected_divergence_ledger_is_well_formed() {
    for (index, entry) in EXPECTED_DIVERGENCES.iter().enumerate() {
        assert!(
            !entry.why.trim().is_empty(),
            "{}/{} [{}] has no reason — an expected divergence with no named \
             defect is a tolerance, and this table does not hold tolerances",
            entry.tier,
            entry.name,
            entry.plan.as_str(),
        );
        for other in &EXPECTED_DIVERGENCES[index + 1..] {
            assert!(
                !(other.tier == entry.tier && other.name == entry.name && other.plan == entry.plan),
                "{}/{} [{}] is listed twice",
                entry.tier,
                entry.name,
                entry.plan.as_str(),
            );
        }
    }
}

/// The gate variable is spelled the same here as in `wasm_real_link.rs`, so
/// one CI environment setting covers both suites.
#[test]
fn the_real_link_gate_is_the_shared_one() {
    assert_eq!(REQUIRE_VAR, "TCL_REQUIRE_WASM_LINK");
}
