# Nested word-effect native control

The exact script and six actual interpreter results are retained here. The
human-readable execution record is
[`w210-empty-foreach-nested-word-write-2026-11-04`](../../../../../../evidence/tcl/w210-empty-foreach-nested-word-write-2026-11-04.md).
All six return `load 1 fail call 0 7`: the caught load error does not prevent
the later explicit call, and the empty-list-producing substitution writes `x`
before it is read. This is native evidence only; Rust guard checks are separate.
