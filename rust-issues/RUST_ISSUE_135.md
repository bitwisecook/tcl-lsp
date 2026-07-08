# RUST_ISSUE_135: fuzz-findings skill points at a dead path and is a retired-Python leftover

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | Build tooling & CI |
| **Location** | `.claude/skills/fuzz-findings/fuzz_findings.py` |
| **Status** | Fixed |
| **Verification** | Verified firsthand by reviewer |

## Finding

`.claude/skills/fuzz-findings/fuzz_findings.py` hardcodes `FINDINGS_DIR = tooling/fuzzing/findings/`, which does not exist anywhere on the branch (there is no `tooling/` dir at all); every command crashes with a raw `FileNotFoundError` traceback. It is also a Python script in a repo whose AGENTS.md states Python was fully retired; rust/tcl-fuzz has its own native findings registry (rust/tcl-fuzz/src/findings.rs) the skill was never migrated to.

Confidence: high (verified firsthand)
