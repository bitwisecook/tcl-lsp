Original C catch output-name headers
====================================

Question: does catch use the original result/options name headers, and which
headers reach String conversion and variable-name caching on success, empty names
and a rejected assignment to an existing array?

Both captures compile the retained C API input against each original repository
toolchain: Tcl 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0. Header and live interpreter
patchlevels agree. All ten compile/execution pairs completed with exit 0 and no
stderr, with the selected generic/unix/library provider files unchanged. Manifests
retain actual source/static-library/configuration/compiler/executable/linked-library
hashes, invocations and channels. Each probe uses Tcl_CreateInterp core only;
Tcl_Init, init.tcl and tcltest were not executed. Original init.tcl remains in the
provider closure. There is no substituted or claimed tcltest verdict.

Result-name integer headers become parsedVarName objects, including the empty
name and failed array store. Selected successful options-name headers also become
parsedVarName objects. The failed result store leaves the options header as int;
ordinary error logging can still generate its String representation. Tcl 8.4 has
no options argument: that retained but unpassed control stays stringless int.

Capture01 was exploratory orchestration submitted inline. Its C input and all
actual subprocess inputs/hashes/channels are retained, but a pre-hashed standalone
orchestration driver was not captured. Capture02 executes the retained, pre-hashed
capture.py, and every provider's stdout is byte-identical. Runtime fixtures use
capture02 only. Rebuildable executables are excluded, with captured hashes checked
if present. The verifier runs hashes only, never an interpreter.

These cells prove actual original name-header conversion/getter effects. Native
reference counts, live cell identity, full diagnostics and compiled catch bodies
are outside this capture. Rust controls and broader migration acceptance remain
unexecuted obligations.
