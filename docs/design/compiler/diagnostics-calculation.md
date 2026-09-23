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
bucket — as a directive fact. `tcl_lsp_core::diagnostic_policy::apply`
applies it, and every other suppression step besides it — the five
configuration scopes, the default-off seed, severity, the optimiser and
shimmer switches, overlap precedence, encoding abstention — for every
producer's finding on every surface: the editor's push and pull, `tcl diag`
/ `lint` / `validate` / `opt`, the MCP tools and the code actions all read
one report. Codes disabled with `tclLsp.diagnostics.<CODE> = false` are
decided at the same point. A code the analyser skips computing at
production — its own permitted saving, over the policy's own disabled set —
is declared to the report (`Report::declare_analyser_skip`) with the reason
the policy gives, so a gap reads as explained rather than as clean. Design:
[diagnostic-policy.md](diagnostic-policy.md).

### Grouped optimisations

Related optimisation edits share a `group` id (`Optimisation::group`), carried
in the diagnostic's `data`, so the client applies a group's edits as one code
action.

A group's edits are **all-or-nothing**, so a grouped diagnostic never carries
the single-edit payload an ungrouped one does. Its `data` is
`{group, edits: [{replacement, startOffset, endOffset}, …]}` — every member's
edit, on every member's diagnostic — and the flat `replacement` /
`startOffset` / `endOffset` triple is absent. A client that does not
understand `edits` therefore finds no auto-apply payload, which is the safe
way to fail: applying one member alone corrupts the program. O127's pair is
an inline of the whole assignment at the use site plus a delete of the
original, and the delete carries an empty replacement, so publishing per
member offered only the inline — which runs the assignment twice (#2149).

A group that loses a member to a `# noqa` or a per-code toggle can no longer
be applied whole, so its survivors are published as advice with no payload at
all rather than as a partial edit — `Report::applicable_rewrites` (and
`applicable_items` for the rewrite loop) is the only door to a rewrite, and
it excludes such a group entirely, member by member.

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
