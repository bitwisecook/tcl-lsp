# IMAP4 diagnostic coverage

The exact Tcllib 2.0 `modules/imap4/imap4.tcl` file does not establish a fix for
[#2243](https://github.com/bitwisecook/tcl-lsp/issues/2243). Its returned memoised
and uncached reports agree, but the actual diagnostic solve does not retain the
first reported `$argc` read. The issue remains open.

The input is Tcllib tag `tcllib-2-0`, SHA-256
`5837f0296ab3a8efe67d4a385ea7da722099abcc406937141589d75628154b66`.
The shared `FixedCorpusSource` owner rejects missing or changed bytes. The two
reported substitution extents are 43921–43926 and 43960–43965.

The diagnostic patch was built on
`ad4335a83992de1b674fc74baf243e3cd100a46c`; [build-seal.txt](build-seal.txt)
records the exact patch and source hashes, and [binaries.sha256](binaries.sha256)
records the copied executable hashes. [audit.patch.json](audit.patch.json) preserves the
failing probes and their build inputs as UTF-8 lines with the original patch digest.
Concatenating the JSON `lines` values reconstructs the exact sealed patch. The digest file
`rust/tcl-test-support/data/tcllib-2.0-imap4.sha256` must also be present when
reproducing that patch. Logs omit only redundant blank lines at EOF. These probes are outside active CI
tests; their failures
are evidence of missing coverage, not passing abstention assertions.

| Check | Actual result |
| --- | --- |
| Combined DB library and corpus test build | Exit 0; [build.log](build.log) |
| Checked diagnostic provider | Available, zero checks |
| Actual solve top-level unit | Not complexity guarded; no opaque native accesses; replay report says complete |
| Original-source selector at 43921–43926 | Missing; [source-owner.log](source-owner.log), exit 101 |
| Original-source selector at 43960–43965 | Not reached after the first missing-read assertion |
| Document taint solve executions | One |
| Procedure `function_lattice` executions | Zero; [memo-counter.log](memo-counter.log), exit 101 |
| Warm report reuse assertion | Not reached after the missing lattice-work assertion |
| Quiet startup and active `gets`/`argc` rebind/unset-rebind controls | Pass; [active-controls.log](active-controls.log), exit 0 |

[Source tracing](source-phases.log) records zero procedures, one function, two
blocks and thirteen statements. The first `package require Tcl 8.5 9` at byte
2661 has one known target. The following `package provide` at 2687, `namespace
eval` at 2716 and outer `if` at 43652 have no known targets and retain
`unknown=true`. The loss of command-table knowledge precedes the reported reads.
The trace does not establish the package invocation's native completion.

To reproduce after restoring the sealed patch and fixed input:

```sh
source scripts/dev/agent-build-env.sh
cargo test --locked -j 1 -p tcl-lsp-db --lib --test compiler_check_corpus --no-run
```

Run the emitted library test executable with
`--exact tests::imap4_argc_reads_have_the_diagnostic_solve_ssa_owner_2243 --nocapture`.
Run the emitted corpus executable with
`--exact compiler_check_memo_matches_uncached_imap4_argc --nocapture`, and then
`--exact compiler_check_memo_matches_uncached_argc_t100_controls --nocapture`.
The source-phase trace uses the same library command with
`TCL_LSP_TRACE_SOURCE_PHASES=1 TCL_LSP_TRACE_ORIGINAL_FUNCTION_LATTICE=1`.

The earlier 24 fresh-process quiet comparisons did not test this source-coverage
condition. They prove returned-report agreement for that bounded sample only.
No compiler behaviour or startup-variable exemption was changed.
