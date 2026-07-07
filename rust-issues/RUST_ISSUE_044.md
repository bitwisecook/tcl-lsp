# RUST_ISSUE_044: `else` bodies are walked with an empty `EnclosingContext`, discarding all outer match criteria, so actions in an `else` translate to unconditional catch-all routes

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | high |
| **Subsystem** | f5-query / report-gen / f5-xc |
| **Location** | `rust/f5-xc/src/translator.rs:964-965` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/f5-xc/src/translator.rs:964-965 — `else` bodies are walked with an empty `EnclosingContext`, discarding all outer match criteria, so actions in an `else` translate to unconditional catch-all routes.
For `if {[HTTP::host] eq "a.example.com"} { if {[HTTP::path] starts_with "/api"} { pool a-api } else { pool a-web } }`, the `a-web` route is emitted with no host/path criteria and (being ordered before later clauses) hijacks all traffic; contrast `walk_switch`'s default-body, which keeps `enclosing` and clears only its own key. Quote: `walk_script(eb, ctx, registry, depth + 1, &EnclosingContext::default());`. Unrecognised-switch arm bodies (line 1149) have the same drop.
Confidence: high
