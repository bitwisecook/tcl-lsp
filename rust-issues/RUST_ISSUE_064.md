# RUST_ISSUE_064: whole command families are never generated, so never differentially tested even for the one live pair

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | Backend parity (WASM/VM/eBPF/registry) |
| **Location** | `fuzzer` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

fuzzer — whole command families are never generated, so never differentially tested even for the one live pair.
generator.rs emits no `array` ops (despite arrays claimed in-scope), no upvar/uplevel/global/variable, apply/lambda, eval/subst, format/scan, regexp/regsub, trace, coroutines, floats, or string-comparison operators (eq/ne/in/ni/<=/>=). Confidence: high
