# Original package requirement word values

The unchanged probe ran on the original C Tcl 8.4.20, 8.5.19, 8.6.18,
9.0.4 and 9.1.0 interpreters with their own library/init files, and on the
pinned original Jim interpreter. The receipt records source, executable,
init, output and stderr hashes and actual times. No tcltest was substituted.

C Tcl accepts bare, one braced wrapper, one quoted wrapper and a Unicode
escape spelling of the same `8.6` requirement. Extra literal braces or
quotes remain data and produce an invalid-version error. A dynamic variable
read fails independently. Jim reports this `package vsatisfies` subcommand
as unsupported; its rows supply no successful requirement/loader parity.

These public observations were captured before the new Rust test expectations.
They support preserving literal word values in conditional package-index
advice, without establishing actual source lookup, package registration,
Native evaluation, loader execution or a selected runtime.
