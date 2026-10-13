# Original definer heads and materialised names

Both fixed programs produce `ORIGINAL {}` on C Tcl 8.4.20, 8.5.19,
8.6.18, 9.0.4, 9.1.0 and the pinned Jim interpreter. All twelve
processes exit zero with empty stderr.

The renamed-stock-definer program invokes `stock_proc` after renaming the
original `proc` command. The constant-name program passes the original `$name`
word to `proc`. Both install and call `:audit`; neither creates `audit`.
Compiler analysis may retain a resolved name, but executable statements must
preserve the actual definer head and original argument words.

Exact programs, raw streams, original argv, environment change, input hashes
and interpreter binary hashes are in `answers.json`. The recorded paths are
the actual scratch locations used for execution; retained program bytes are
identical. Jim upstream freshness is independently recorded by the root audit,
not checked by these probes. These are direct interpreter answers, not native
entry or emitted-WASM acceptance. No Tcltest or F5 execution was used.
