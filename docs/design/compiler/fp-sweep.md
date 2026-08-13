# False-positive sweep harness

`cargo xtask fp-sweep` dumps every firing of one or more diagnostic,
optimisation, or shimmer codes across a corpus of real Tcl, groups the firings
by site shape, and prints sample locations. It is the tool used to decide
whether a code is producing false positives before its emitter is changed.

- **Harness:** `rust/xtask/src/fp_sweep.rs`
- **Subcommand definition:** `rust/xtask/src/main.rs` (`FpSweep`)

```
cargo xtask fp-sweep --code CODE [--code CODE...] \
                     --corpus PATH [--corpus PATH...] [--examples N]
```

## What the sweep runs

The sweep reproduces, in one pass, everything the editor can publish for a
file, so no code the user can see is invisible to it:

| Source | What it contributes |
|---|---|
| `Analyser::analyse` | W-, E-, and H-series checks |
| `run_all_checks` + `optimise_unit` over one interprocedural-summarised `CompilationUnit` | O-, S-, and T-series codes |
| `tcl_lsp_core::source_style::style_diagnostics` | the pure-text style pass (W111/W112/W115/W118) |

That is a superset of `tcl diag`'s own collector, which deliberately drops the
O-series. It mirrors `tcl_lsp_db::compiler_check_diagnostics_uncached`, the
no-salsa-input fallback path.

## Dialect awareness

Every corpus file's dialect is resolved with
`tcl_cli_support::InputDocument::effective_dialect` — the same detector
(`# tcl-dialect:` directive, then content signal, then extension, falling back
to `tcl8.6`) that the `tcl` CLI and the LSP server use.

This is not optional bookkeeping. A fixed-dialect sweep reports phantom
W002/W004 on every version-gated command in a Tcl 9 file (`oo::configurable`,
`source -nopkg`), and those phantoms are not false positives of the code under
audit. A sweep that skips dialect resolution measures the harness, not the
analyser.

## Corpus discovery

The walk accepts the normal Tcl-family extensions plus two publication formats
common in public iRules sources:

- a `.txt` file whose content contains a top-level `when EVENT … {` handler is
  swept as one iRules document;
- a reStructuredText `code` / `code-block` directive is extracted when its
  indented body carries that same signature, and its findings keep the original
  document's line numbers.

The `when EVENT` signature is a deliberately strong signal: it keeps prose and
console transcripts out of the analyser. Ordinary source files are decoded by
`tcl_cli_support::read_input_documents`, the same reader as `tcl diag` and
`tcl opt`. Generated and VCS trees (`.git`, `build`, `dist`, `node_modules`, …)
are skipped.

## Grouping

Firings for a code are bucketed by a normalised message — digit runs and
single-quoted identifiers are replaced with a placeholder — so repeated
instances of one pattern collapse into a single row with a count, highest
volume first. The high-volume shapes are where a systematic false positive
lives; the long tail is usually genuine.

## Reading the result

A firing count is a triage lead, not a verdict. In particular an iRule can
depend on a virtual-server profile, traffic direction, or a sibling rule that
is not present in the file being swept, so a high count proves only that a
shape is common.

The workflow for each shape worth investigating:

1. Reduce the highest-volume shape to a minimal reproducer.
2. Verify the reproducer against real C tclsh for the relevant dialect.
3. Either fix the emitter and land paired must-fire (TP) and
   must-stay-silent (FP) regression tests, or record the shape as a confirmed
   true positive.

The paired regression tests live in
`rust/tcl-compiler/src/analyser/diagnostics/fp/`, one module per family
(`rbs.rs`, `ds.rs`, `sh.rs`, `obj.rs`, `opt.rs`, `sty.rs`, `tnt.rs`, `rch.rs`,
`inj.rs`, `nab.rs`, `bnd.rs`). A behaviour change without both arms is not
finished: the must-stay-silent arm is what stops the next precision fix from
reintroducing the false positive.

## Corpus availability

The canonical external corpus (tcllib, the Tcl stdlib, tklib, tdom,
SpiceGenTcl, and the public iRules example repositories) is fetched, not
vendored. `scripts/dev/fetch-irules-corpus.sh` fetches the iRules leg. When
egress is unavailable, the committed tree still provides a usable substitute:

| Path | What it contributes |
|---|---|
| `runtime/rust/vendor/tcl_library` | genuine C-Tcl standard-library source (`init.tcl`, `package.tcl`, `tcltest/tcltest.tcl`, …) |
| `rust/tcl-irule-test/tcl` | a hand-written TMM-simulation orchestrator — real, non-trivial iRules-adjacent Tcl |
| `samples/` | curated single-diagnostic examples and real-shaped smoke fixtures |
| `scripts/dev/diag_parity/corpus` | the diagnostic-parity corpus |
| `editors/vscode/testFixture` | noisy — many fixtures are deliberately malformed to exercise one diagnostic, so co-firings there carry less weight |

A code with no corpus firing is verified synthetically instead: a minimal
reproducer through `tcl diag`, cross-checked against real tclsh wherever
runtime behaviour is load-bearing.
