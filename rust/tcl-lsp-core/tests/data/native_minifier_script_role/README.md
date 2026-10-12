# Native script-role control

The exact ASCII sources test a substitution which replaces `if` before the body operand is dispatched. Each case uses a fresh interpreter. The proposed candidate changes only separators in the outer braced body; it is not captured minifier output.

The actual C 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0 controls use both `Tcl_EvalEx(..., 0)` and `TCL_EVAL_DIRECT`. Current Jim uses `Jim_Eval`; Direct is explicitly unavailable. Original stdout/compile streams, per-provider and aggregate receipts remain unchanged. Executables are identified by hash but are not copied into the repository.

`python3 replay.py --verify-only` checks the retained exact inputs/streams and finite mode/handler/result controls without launches. `--output /tmp/new-absent-directory` performs a new capture only after validating every original provider pin at its archived path. `capture.py` is the exact original runner and has archival absolute paths.

The native observations answer dispatch and result-byte questions. They do not prove minifier output, arbitrary rewrite equivalence, source compiler admission or Rust execution. The readonly/closed source-region implementation is documented separately.
