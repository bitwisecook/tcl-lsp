# RUST_ISSUE_017: `backslash_end`'s `_ => start + 2` fallthrough splits a multi-byte UTF-8 char, so `&template[i..j]` panics

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | high |
| **Subsystem** | Compiler front-end (segmenter/expr/subst) |
| **Location** | `rust/tcl-compiler/src/subst_nocommands.rs:206 (panics at line 71)` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-compiler/src/subst_nocommands.rs:206 (panics at line 71) — `backslash_end`'s `_ => start + 2` fallthrough splits a multi-byte UTF-8 char, so `&template[i..j]` panics.
A `subst -nocommands` template with a backslash before any non-ASCII char (`\é`, `\€`, `\🎉`) reaches the lowering hook (lowering/mod.rs:1573, specialise_factories.rs:417) and crashes the compiler front-end on arbitrary source. For `\é` = `[0x5C,0xC3,0xA9]`, `c=0xC3` hits `_ => start+2`, and `template[0..2]` cuts the 2-byte `é` mid-char → panic. The VM's equivalent does `other => 1 + utf8_char_len(other)` (subst.rs:191). `_ => start + 2,` Confidence: high
