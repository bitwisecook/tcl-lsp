# Original procedure-body reference scopes

`read-scope.tcl` runs unchanged on original C Tcl 8.4.20, 8.5.19,
8.6.18, 9.0.4, 9.1.0 and the pinned original Jim interpreter. `receipt.json`
records exact executables, original C library/init files and output hashes.
No tcltest package or substitute init file is used.

All six reject a BOM-prefixed `namespace` command inside a procedure as a
literal unknown command. Its braced argument remains script data; it does not
read the caller's parameter. An ordinary namespace body cannot read that
parameter either. The direct substitution prints its value; the inert braced
value returns normally without running its contents.

These observations distinguish lexical source scope and literal body data.
They support the W214 source-reference expectation and supply no Rust test,
complete reference census, successful Native read or entered-frame proof.

Original Jim evaluates both immediate expression inputs to `6`, including the
nested original List command. Original C Tcl instead attempts its ordinary
array-reference interpretation and reports missing `(5 + 1)`. The source
candidate walker follows its typed executable Expression component under the
actual complete grammar; command and quote boundaries remain shared owners.
