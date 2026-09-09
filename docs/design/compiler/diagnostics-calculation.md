# Diagnostics calculation — the two tiers

Which diagnostics appear instantly and which after a delay, how the server
keeps a slow pass from publishing stale results, and where a new diagnostic
belongs.

Source: `rust/tcl-lsp-server/src/lib.rs` (scheduling and publication),
`rust/tcl-lsp-db/src/lib.rs` (the salsa queries).

### Fast tier

Published first only when the deep pass overruns its 40 ms budget on a
document of 500 lines or more (see
[async-diagnostics-tiering.md](async-diagnostics-tiering.md)):

- every analyser diagnostic no workspace pass can retract — syntax (`E*`),
  semantic (`W*`, `H*`, `I*`), iRules (`IRULE*`) — i.e. every code for which
  `DiagCode::refined_by_workspace()` is false; only W120 and W123 are
  excluded;
- the pure source-style lints from `tcl_lsp_core::source_style` (W111, W112,
  W115, W118).

### Deep tier

The authoritative publish for a version. Three whole-file analyses run
concurrently off the event loop, then the results are refined, lifted, and
published once:

| Analysis | Query | Codes |
|---|---|---|
| per-file analyser walk | `file_analysis` | every analyser code, including W120/W123 |
| compiler checks + optimiser | `compiler_check_diagnostics` — `run_all_checks` and `optimise_unit` over one `CompilationUnit` | shimmer S100–S102/S110, sharing/thunking, taint T1xx and IRULE3xxx, iRules flow IRULE1xxx–5xxx, GVN O105/O106, SCCP constant branches; optimiser O1xx as HINT-severity suggestions |
| cross-file resolution | `project_diagnostics`, `project_callback_diagnostics` | W120/W123 refinement, cross-file arity |

### Suppression

```tcl
set x 42    ;# noqa: O109  — suppress the dead-store warning
eval $cmd   ;# noqa: *     — suppress every code on this line
```

The analyser builds the suppression map (`AnalysisResult::suppressed_lines:
HashMap<i32, HashSet<String>>`, `rust/tcl-compiler/src/analyser/types.rs`) —
inline `# noqa` per line, a top-of-file `# tcl-lsp: disable=…` in the `-1`
bucket — and every lift applies it (`line_suppressed` in the server; the same
contract in `tcl_lsp_core::source_style`). Codes disabled with
`tclLsp.diagnostics.<CODE> = false` are filtered at the same point.

### Grouped optimisations

Related optimisation edits share a `group` id (`Optimisation::group`), carried
in the diagnostic's `data`, so the client applies a group's edits as one code
action.

## Decision rule

- A diagnostic computed from tokens, the CST, or the per-file analyser is in
  the fast tier automatically — unless a workspace pass can retract it, in
  which case add its code to `DiagCode::refined_by_workspace`.
- A diagnostic that needs `CompilationUnit` facts (CFG, SSA, lattices) is a
  compiler check or an optimiser pass and is deep-tier only.

## Related docs

- [Diagnostics section in walkthroughs](example-walkthroughs.md#how-diagnostics-are-calculated)
- [async-diagnostics-tiering.md](async-diagnostics-tiering.md)
- [diagnostics-integration.md](diagnostics-integration.md)
