# Broad cleanup sample: five unrelated authored passages

Read-only source audit, 2026-10-12. Only this review record was written; source
edits remain deferred while shared source snapshots/builds run. The parent
confirmed these F5 CLI and VS Code paths lie outside current naming changes.
This is a five-item sample, not the final repository-wide cleanup audit.

Rule: `AGENTS.md` § Documentation requires removing development history and
stale claims while preserving current contracts, necessary how/why, legal
headers, reproducible experiments and oracle provenance. No proof record or
retained experiment is a removal target below. No builds or tests were run.

Search scope: started with `rg --files` and contextual `rg` searches across
authored design docs, KCS, Rust/runtime code, scripts, editors and AI templates.
A narrower candidate search excluding vendor, data, generated directories,
appliance probes, perf comparisons and canonical name-resolution proofs yielded
635 matching lines: Rust 501, docs 44, scripts 29, editors 59, AI 2. These are
candidate counts, not findings or audit completion. A focused F5/editor/script
search yielded 84 lines; five verified items follow. Actual installer phase
ordering and runtime loop/step semantics were retained as legitimate terms.

1. **Obsolete unimplemented-engine claim** — `rust/f5-cli/src/lib.rs:24–26`,
   crate docs preceding `run`/`dispatch`.
   Current evidence: `dispatch` at line 88 exhaustively routes `Command` arms
   into engines through line 464, with shell completion generated directly;
   `run` at lines 46–60 prints actual errors and maps them to exit code 2.
   `rg -n 'not yet implemented|unimplemented!|todo!' rust/f5-cli/src --glob '*.rs'`
   finds only these two doc lines, no corresponding stub engine/error.
   Proposed exact edit: replace lines 24–26 with the blank `//!` line and
   `//! Dispatch errors are printed and return exit code 2.`
   Scope: correct stale crate docs; preserve command ownership and error policy.

2. **Completed duplicated-set diary** —
   `rust/f5-cli/src/commands/remote/object_io.rs:67–72`, `OBJECT_KINDS` docs.
   Current evidence: the seven rows at lines 75–83 feed `object_kind_names`
   (line 87) and `kind_to_endpoint` (line 93). `rust/f5-cli/src/cli.rs:597` and
   `:636` both use `object_kind_names()` as their value parser.
   Proposed exact edit: end line 67 after `/// against these rows.` and remove
   lines 68–72, including the issue-number history. Keep lines 62–66 and 73–74.
   Scope: remove migration diary; retain single-owner and sorting rationale.

3. **Grammar implementation diary** —
   `editors/vscode/syntaxes/apl.tmLanguage.json:8`, top-level `comment` value.
   Current evidence: `editors/vscode/package.json:333–339` selects this APL
   grammar and declares embedded Tcl; lines 2471–2486 map APL semantic scopes.
   Grammar `repository.field` at lines 165–177 uses a whitespace boundary,
   while `repository.embedded-tcl` at lines 63–84 includes `source.tcl`.
   Proposed exact edit: remove the sentence beginning `Every Tcl-family
   language used to be pointed at` through `(issue #903). `; remove only
   ` — a real bug the first draft of this grammar had` later in that value.
   Scope: leave the APL/Tcl distinction, scope agreement, keyword-boundary
   example and user-defined-field-type limitation unchanged. Keep all grammar
   rules and repository comments; preserve valid JSON.

4. **Prior agent-build contention anecdote** —
   `editors/vscode/src/test/signal.ts:49–50`, module deadline docs.
   Current evidence: `runQueuePressure` (line 154), `loadFactor` (line 206),
   `measuredLoadFactor` (line 225), and `scaledTimeout` (line 251) measure and
   apply contention. `awaitSignal` (line 436) bounds extensions and throws on
   timeout. The particular old build-tree count is not a present contract.
   Proposed exact edit: replace lines 49–50 with
   `//! *content* test fails as a *timeout*.`
   Scope: remove the run anecdote; retain the CPU-starvation explanation and
   all measurement, timeout-scaling and bounded-wait guidance.

5. **Obsolete before/after timeout aside** —
   `editors/vscode/src/test/diagnostics.test.ts:495–499`, test
   `clean file produces no diagnostics` (line 458).
   Current evidence: line 500 awaits the URI-scoped deep marker before the
   empty-result assertion. `helper.ts:243–257` implements that marker wait;
   `helper.ts:1247–1269` defaults diagnostic waits to `minCount: 1`.
   Proposed exact edit: keep line 495 through `vacuous empty set.` and replace
   its remaining sentence plus lines 496–499 with:
   `// A clean file cannot satisfy waitForDiagnostics's default minCount: 1;`
   `// the deep-pass marker confirms that analysis completed.`
   Scope: remove historical behaviour comparison; preserve the positive
   completion signal and reason an empty diagnostic set alone is insufficient.

All five proposals change authored prose only. Legal headers, configuration
directives, grammar matches, runtime behaviour and test assertions stay intact.
The final audit must cover remaining areas and generated copies/links before
merge; this temporary coordination record itself belongs to that cleanup.

Coordinator applied these five prose-only corrections. The grammar object is
exactly unchanged after excluding its comment value; executable tokens and test
assertions are unchanged. No tests were added or executed for this low-impact
cleanup. Remaining repository-wide audit and final work-document removal remain
pending.
