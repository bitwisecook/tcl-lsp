# First diagnostic-callee candidate: red validation

This snapshot is not a verified fix. Base
`2a413918de001e01418e67d31b4e8b28470c4550`, tracked patch
`7a5ddc68c6bef35a00bbad8b2d665e773a5511dd6ca43450b2a973988babf973`;
`candidate.patch` additionally retains the exact untracked integration source.
All 42,662 present tracked/untracked source files are byte-identical before
and after build, Clippy and runtime. The compressed source manifest, raw logs,
gate commands, executable hashes and result records are retained here.

| Check | Actual result |
| --- | --- |
| Clean unit and two integration `--no-run` build | Pass, 384.17 seconds |
| Production compiler Clippy | Fail: two unchanged baseline functions exceed 100 lines |
| New integration | Both tests fail at their first Tcl 8.4 case; procedure x still missing |
| New ownership controls | Three fail in their fixture selector before reaching the predicate; two CFG statements share the source offset |
| Existing ownership controls | Three pass |
| Existing short-circuit integration | Both tests pass: 185 source/admission rows |
| Existing read-local controls | Seven tests pass, covering 14 guard cases |
| Existing existence-query owner | Nine tests pass |

The 75 planned new rows did not all execute and are not counted as passing.
The original integration result still contains only top-level y at 105..107,
missing procedure x at 79..81. No source or authority guard was relaxed after
these failures. The next fixture revision must select the genuine original
word geometry rather than one of the generated loop statements at that offset.

Clippy fails in unchanged `command_binding.rs` functions
`walk_logical_called_source` (147 lines) and `walk_logical_namespace_source`
(145 lines). That file's frozen SHA-256
`03076691c9f8231a13de86fd23d10a7c8974d2720d0e9cb7623dcc2d8ce3cc9e`
matches HEAD; the candidate does not edit it. No new lint allowance was added.

No ENOSPC occurred. The build watchdog recorded minimum workspace free space
755,159,040 bytes and did not abort. Runtime used the emitted binaries directly
outside the compiler lease; their hashes remained unchanged. Independent
copies are recorded by `/tmp/lsp-2441-frozen-first-attempt/binaries.json` before
any cached retry. This is Rust implementation evidence; the separate native
Tcl record remains the actual interpreter observation.
