# Original source Text controls

Six exact ASCII scripts select unquoted and quoted `p\uD800`, an empty quoted word, a procedure declared and invoked by the escaped opaque name, an unknown variable, and an unknown command before a later literal. `\uD800` is an ASCII source escape for the tested surrogate; the result `70 ed a0 80` contains bytes which are not valid Unicode UTF-8. The source contains no raw NUL or encoded surrogate.

Each case uses a fresh interpreter. C 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0 use explicit `Tcl_EvalEx` flags zero and `TCL_EVAL_DIRECT`. Jim 0.84-9-g5bac7c9 uses `Jim_Eval`; a C Direct recipe is explicitly unavailable. The receipts preserve source, probe, provider, executable and stream hashes. The source excerpts explain Jim parser token construction separately from the measured completion and result bytes.

`python3 replay.py --verify-only` checks exact retained bytes and controls without running an interpreter. `--output /tmp/new-absent-directory` performs a fresh capture after checking every original provider pin. The exact original `capture.py` and queue contain archival absolute paths.

These finite source observations do not prove a physical literal pool, object identity, custom object conversion closure, compiler admission, arbitrary host state, or any Rust execution result. The Rust text-world contract keeps those obligations separate.
