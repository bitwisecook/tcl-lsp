# Hand-over: analysis performance

Written 17 November 2026 by the first analysis-performance session (`session_017u5g3LF3ibFZGW5YWpNcQa`). A second session (`session_017u5g3JhfJTsPSscoP6KGPp`) continued this area from 15 November and writes its own notes; where the two disagree, the later session and the "Analysis performance" section of [current state](../current-state.md) win. Everything this session changed is on `work`; nothing is held back.

## Scope

Owns: the cost of `tcl_compiler::analyser::Analyser::analyse` (structure-only and full) and of the source-binding realm (`document_command_realm`, `SourceCommandBindings`), measured on a generated file. Files this session changed: `rust/tcl-compiler/src/command_binding.rs` and, under `rust/tcl-compiler/src/command_binding/`, `root_layout_extents.rs`, `variable_access_owners.rs`, `point_extents.rs`, `shared_inventory.rs`, `namespace_context.rs`, `origin_inventory.rs`, `declaration_layout.rs`, `declaration_preview.rs`, `original_command_table.rs`, `jim_stock_class/source_advice.rs`; `rust/tcl-compiler/src/place_bridge.rs`; `rust/tcl-compiler/src/analyser/state.rs` (`Analyser::source` is `Arc<str>`); `rust/tcl-lexer/src/source_map.rs` (`SourceImage::try_text`, `has_bytes_of`, `shared_text`, `SourceMap::new`, `SourceMap::from_image`).

Does not own: the semantic-token walk (`rust/tcl-lsp-core/src/semantic_tokens*`, the coordinator's), the first-paint tier, the CFG, SSA and SCCP stages of the full analysis (dataflow area), and any change that alters an analysis answer. It has no test group of its own; its tests are the stack and time budgets in `rust/tcl-compiler/src/depth_guard.rs` and `rust/tcl-compiler/tests/it_2/phi_undef_memo.rs`, plus the equivalence tests beside each index.

## How to measure

Environment, as in the [fixing guide](../fixing-guide.md): `source scripts/dev/agent-build-env.sh; export CARGO_PROFILE_TEST_OPT_LEVEL=1 CARGO_PROFILE_TEST_DEBUG=0 RUST_BACKTRACE=0`.

Behaviour: `cargo nextest run -p tcl-compiler -p tcl-irules --no-fail-fast --status-level fail --final-status-level none --failure-output never > run.log`, about 11 minutes to run after the build. Failing list: `grep -a "^ *\(FAIL\|TIMEOUT\|SIGABRT\|SIGSEGV\|LEAK\)" run.log | sed 's/^.*) //' | sort -u`, compared with `comm` against the list from the unchanged base. This session compared whole lines; since the file split compare by the last path segment. Lexer changes also need `cargo nextest run -p tcl-lexer` (817 tests, seconds).

Speed: the probe below, saved as `rust/tcl-compiler/tests/zz_perf_probe.rs` and never committed. Build with `cargo test -p tcl-compiler -p tcl-irules --test zz_perf_probe --no-run`, copy the binary out of `target/`, delete the test file, then run `PROBE_N=100,200,400,600,800 TCL_LSP_TRACE_WORKSPACE_SCAN=1 <bin> --exact zz_tmp_scaling_probe --nocapture`. `ANALYSER_PHASE … stage=realm-complete` and `walk-complete` split the source walk from the analyser walk; `PROBE_FULL=1` runs the full analysis. Take the best of three: the host varies by 10 to 15 per cent. A procedure is six walked commands, so ms per command is total divided by 6N+2.

```rust
use std::fmt::Write as _;
use std::sync::Arc;

#[test]
fn zz_tmp_scaling_probe() {
    let dialect = tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
    let registry = tcl_registry::CommandRegistry::build_default().project_for_profile(dialect);
    let full = std::env::var("PROBE_FULL").is_ok();
    let ns: Vec<usize> = std::env::var("PROBE_N").map_or_else(
        |_| vec![100, 200, 400],
        |v| v.split(',').map(|x| x.parse().unwrap()).collect(),
    );
    for n in ns {
        let mut s = String::from("namespace eval ::bench {\n    variable counter 0\n}\n\n");
        for i in 0..n {
            let _ = write!(
                s,
                "# proc number {i}\nproc ::bench::step{i} {{a b}} {{\n    set v{i} [expr {{$a + $b}}]\n    set msg \"step {i} = $v{i}\"\n    if {{$v{i} > 10}} {{\n        set v{i} [expr {{$v{i} + 1}}]\n    }}\n    return $v{i}\n}}\n\n"
            );
        }
        let context = tcl_registry::model::context_for_profile(dialect);
        let context = Arc::new(context.with_command_store(registry.snapshot().shared_registry()));
        let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
            dialect,
            dialect,
            context,
            tcl_lexer::LexerConfig::for_file_grammar(dialect.grammar),
        );
        let t0 = std::time::Instant::now();
        let mut a = tcl_compiler::analyser::Analyser::new();
        if !full {
            a = a.structure_only();
        }
        let analysis = a.with_resolved_input(input).analyse(&s, dialect.name);
        eprintln!(
            "PROBE n={n} lines={} full={full} analysis={:?} diags={}",
            s.lines().count(),
            t0.elapsed(),
            analysis.diagnostics.len()
        );
    }
}
```

Profile: `perf` is not installed. Run the probe in the background and attach `GS_N=3000 GS_OUT=prof.txt setsid gdb -p <pid> -batch -x gsample.py` (the `setsid` matters: the script interrupts the inferior with SIGINT):

```python
import gdb, os, signal, threading
pid = gdb.selected_inferior().pid
assert pid > 1
n = int(os.environ.get("GS_N", "200")); iv = float(os.environ.get("GS_IV", "0.03"))
out = open(os.environ["GS_OUT"], "w")
gdb.execute("set pagination off")
for i in range(n):
    try:
        out.write(gdb.execute("thread apply all bt 90", to_string=True))
    except gdb.error:
        break
    out.write("\n=====SAMPLE\n")
    threading.Timer(iv, lambda: os.kill(pid, signal.SIGINT)).start()
    try:
        gdb.execute("continue")
    except gdb.error:
        break
    if not gdb.selected_inferior().is_valid() or gdb.selected_inferior().pid == 0:
        break
out.close()
gdb.execute("detach")
```

Each sample ends with `=====SAMPLE`. Count, per function, the samples whose analysis thread contains it. Two views found everything in this area: a function's sampled share times wall-clock at N=200 against N=800 (what grows more than four times is super-linear), and the innermost non-library frame of each sample (the flat cost). Call counts per command came from breakpoints with a large ignore count on a small N (`break *<addr>`, `ignore <n> 2000000000`, `info breakpoints` at exit; a PIE binary under gdb loads at `0x555555554000`). Retained memory came from an `LD_PRELOAD` shim that samples `malloc` by bytes, records a backtrace and drops the record on `free`, dumped on SIGUSR1.

Server acceptance: `cargo nextest run -p tcl-lsp-server -E 'test(large_file_)' --test-threads 1` (a separate build of about 20 minutes).

## Current numbers

Last full run by this session: 15 November 2026 at `e0804111` (before the restructure), `tcl-compiler` + `tcl-irules`: 13,207 run, 2,511 failing, new 0 against its base. `tcl-lexer`: 817 pass.

Structure-only, generated file under `tcl8.6`, best of three, at `e0804111`: N=100/200/400/600/800 in 2.3/4.2/9.4/15.9/23.6 s, 3.9 to 4.9 ms per walked command. The target is N=600 under 3 s and at most ×2.4 per doubling; scaling met it (×1.8, ×2.2, ×2.5 for 400 to 800), the absolute time did not. Before this session: 5.1/11.7/41 s for N=100/200/400. Peak memory 158/269/512/967 MB at N=100/200/400/800 (was 345 and 780 MB at 200 and 400). Full analysis, one run, at `4d0f7b03`: 7.2/16.9/43.7 s for N=100/200/400.

Reported by the second session, not verified here: N=600 8.9 s (2.5 ms per command) at `35a7f88c`, all five `large_file_*` e2e tests passing.

## Baseline list

`docs/work/name-handling/lists/analysis-performance-failing-2026-11-16.txt`: the 2,511 failing tests of the 15 November run at `e0804111`, one per line as `<binary> <test path>`. It predates the restructure and other areas' later fixes, so take a fresh baseline before using it for a comparison.

## Queue

1. **Per-command record.** After the fixes, no site is above about 4 per cent; the cost is consumers that each rebuild one command's tokens, words, stamped binding and registry facts. Measured per walked command at N=30 before the second session's work: tokenised about 26 times, words built about 96 times, registry semantics resolved about 71 times, state cloned about 62 times, about 10,000 `malloc` calls. Build the record once in `Analyser::process_command` (`rust/tcl-compiler/src/analyser/commands.rs`) and pass it to `source_structure::source_registry_words*` (`rust/tcl-compiler/src/registry_invocation/source_structure.rs`), `CommandBindingRealm::stamp_original_tokens` (`rust/tcl-compiler/src/realm.rs`), `record_instance_creation`, `record_original_variable_receivers`, `infer_proc_param_traits`, `retained_invocation_tokens`. One consumer per commit. No test turns green; the measure is ms per command.
2. **Per-definition copies.** Each `proc` still copies the command table's keys and handles, `rebound_names`, the original command world and the allocation counts (`ModuleCommandBindings` in `command_binding.rs`). They need structure-sharing maps.
3. **Stack headroom at test opt-level 0.** `depth_guard::tests::the_source_walk_cap_fits_its_stack_budget` needs about 1,510 KB of 1,536 KB. The source walk is about 24 KB a level over some twenty frames (`walk_evaluated_source_command`, `walk_resolved_source_command`, `walk_finite_source_iterations`, `walk_prepared_native_target` are the largest). Box large locals and move cold paths out of line; target 25 per cent headroom.
4. **`phi_undef_memo::nested_if_stages_analyse_in_linear_time` and `…_in_a_loop_…`** pass at opt-level 1 and take 28 s and 37 s at opt-level 0 against 10 s. They time the whole pipeline on a 300-line file; not profiled at opt-level 0.
5. **Full analysis.** A third of the diagnostics phase at N=400 was `SourceCommandBindings::module_projection` (`rust/tcl-compiler/src/command_binding/retained_source.rs`) joining every point's state, most of it in the variable-cell joins of `join_activation_values`; the unit build also walks the source a second time.
6. Smaller: `original_catalogue_bindings_for_slot` reads the command table per head; `extend_conditional_math_invocations` reads every retained expression per script; a default `SourceInvocationBinding` allocates a default `ResolveContext` and a `"::"` key each time it is built.

## Held work

None. No stash, side branch, worktree or uncommitted edit. The only things outside the repository were the probe and the sampler, reproduced above.

## Open questions and rulings

- Ruling (owner, 15 November): the path-constant pass and the conditional-metadata recording may be reordered; "it's only important that the result is correct".
- Ruling (coordinator, 15 November): do not lower the nesting cap (`MAX_SOURCE_NEST_DEPTH`); cut frames instead.
- Ruling (brief): an index or memo must return exactly what the code it replaces returned, and the commit says why.
- Open: whether the registry-words memo should stay on the finished `AnalysisResult` so the token walk does not recompute (memory against time); whether `RealmBinding` may stop answering `Rebound` for sites the walk never reached (it changes an answer). Both were raised by the second session.

## Hand-offs

- To the coordinator: the token walk was not examined here; the per-command query for it is `source_structure::source_registry_words(source, analysis, segment)`.
- To the dataflow area: item 5 above.
- Received and not started: none beyond the queue.

## Hazards

- **Do not edit sources while a test build compiles.** One run here tested a mixture of trees and had to be thrown away.
- **Stack budget.** Adding a 16-byte field to `SourceImage` overflowed the stack-budget test, because recursive frames hold many images by value; the field went into the image's shared geometry. Three small caches on `SourceCommandBindings` were folded into one lazily allocated pointer for the same reason. Check that test after any new field on either type.
- **Shallow clone.** `git fetch --depth=50` can make `origin/work` look force-updated and a rebase replay fifty commits; deepen the fetch and check `git merge-base --is-ancestor` before rebasing.
- **`cargo fmt --all`** reformats files other sessions left unformatted; revert those.
- **Disk.** Opt-level 0 builds plus incremental data filled the disk in the middle of a commit; delete `target/debug/incremental`.
- **A lexer test that looked like a regression was a fixture**: `original_image_positions_reuse_immutable_geometry…` expected a second `SourceImage::document` to be independent after the recent-document list made it shared.
- **Weak-handle indexes.** The indexes in `shared_inventory.rs` key on an inventory's allocation through a `Weak` and are built on the second query. A memo that held a strong handle would make every later write copy the inventory.
- **The profiler helper** must take the probe's pid from `$!`; matching by name once sent SIGINT to the shell's process group.
- Timing tests and the server e2e suite vary between runs; rerun before believing a new failure.

## Oracle habits

None needed: this area changes no expectation. Every change was checked as "same failing list" on `tcl-compiler` and `tcl-irules`, plus a unit test comparing each index with the scan it replaces.
