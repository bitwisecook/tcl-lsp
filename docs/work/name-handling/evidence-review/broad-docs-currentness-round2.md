# Bounded authored-document currentness cleanup, second set

2026-10-12: changed three durable Markdown docs and this temporary review only.
No Rust, generated files, canonical proofs, source captures, oracle inputs,
tracking ledgers, Cargo, network, Git mutation or full gates were touched.

1. `docs/design/compiler/fp-sweep.md`: replaced the stale `style_diagnostics`
   producer with `style_facts_from_analysis`, added actual W107/W109 encoding
   checks, and scoped the sweep to its per-document producers without suppression
   policy. Owner: `rust/xtask/src/fp_sweep.rs:579–608`; retained-analysis API:
   `rust/tcl-lsp-core/src/source_style.rs:466–476`.
   Oracle procedure now requires the relevant native engine/context, including
   native appliance probes for F5 TMM/MCPD, where Tcl-level tcltest is unavailable
   under the user's supplied constraint. Appliance stock tclsh is separately
   scoped, as in `experiments/name-handling/patterns/f5/README.md:6–12` and
   `answers/hosted-original-source-review.md:3–5`. No new native answer is claimed.

2. `docs/design/compiler/registry-consumer-contracts.md`: removed the pointer
   to the redesign's absent "deliberately not in this ledger" paragraph.
   Kept the current native catalogue / no ahead-of-time SpecTcl-to-Rust boundary
   and its direct `spec-packs.md` / `command-registry.md` destinations.
   `docs/design/registry/spec-packs.md:33–41` states that current contract;
   redesign §11 no longer contains the referenced paragraph.

3. `docs/design/runtime/tcl-test-tiers.md`: recast the completed `info exists`
   fix anecdote as its necessary current read-trace/constraint dependency.
   Owner: `rust/tcl-vm/src/cmd_info.rs:314–332` reads through traced variable
   existence; the native/observed paths remain separately selected.
   Kept tier semantics, backend limitations, scoreboard and user procedure.

Validation: all 57 local Markdown file/anchor links across these three docs
resolve. No headings changed. `git diff --check` passed. No tests were added or
executed for these prose-only changes. No numerical result/provenance was removed.
Repository-wide cleanup remains open; this temporary review is removable at merge.
