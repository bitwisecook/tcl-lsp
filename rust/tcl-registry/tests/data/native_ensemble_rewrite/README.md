# Native ensemble rewrite reset order

`probe.c` uses the exact engine-private interpreter field to observe the active
rewrite without evaluating an observer command. Four controls run through a
real namespace ensemble: command lookup with a custom original name updater,
inline bytecode, internal `TCL_EVAL_INVOKE`, and a missing command with no
unknown handler. Results and the engine/library hashes are in `manifest.json`.

Tcl 8.5 clears the rewrite after successful lookup, preserving it during the
name updater, an inline-only bytecode body and a failed lookup. Tcl 8.6 and 9.x
clear before ordinary lookup and on bytecode entry. Internal invocation retains
the rewrite in all four releases. Tcl 8.4 and Jim have no C ensemble rewrite
field and are separate non-reset recipes.

Compile with the selected source tree's generic/unix include directories and
its original static Tcl library, linking `-ldl -lz -lpthread -lm`. Set
`TCL_LIBRARY` to that tree's library directory before running. The fixture
contains 16 native observations; no public error globals or return-option
observers are read.
