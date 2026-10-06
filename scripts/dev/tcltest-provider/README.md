# Auditing stock tcltest lookup prerequisites

The provider data in `rust/tcl-registry/src/body_execution.rs` is an authored
implementation contract, independent of command catalogue availability. The
installed command/export surfaces were queried after loading each pinned C Tcl
standard package. Package versions, not core versions, key those contracts.

`lookup-parser.c` uses the real `Tcl_ParseCommand` implementation to collect
literal command heads from the loader source and installed procedure bodies.
It recurses into command substitutions and braced potential scripts. Braced data
also contributes candidates, making this a conservative closure prerequisite;
it is not an exact reachable call graph. Computed heads need separately proved
callback/phase contracts. No alternate Tcl interpreter is implemented here.

To reproduce one release from the repository root:

```sh
cc -Itmp/tcl8.6.18/generic scripts/dev/tcltest-provider/lookup-parser.c \
  tmp/tcl8.6.18/unix/libtcl8.6.a -lm -ldl -lpthread -lz \
  -o /tmp/tcltest-lookup-parser
(cd /tmp && /workspace/tcl-lsp/tmp/tcl8.6.18/unix/tclsh \
  /workspace/tcl-lsp/scripts/dev/tcltest-provider/lookup-source.tcl \
  /tmp/tcltest-bodies /tmp/tcltest-core \
  /workspace/tcl-lsp/tmp/tcl8.6.18/library/tcltest/tcltest.tcl)
/tmp/tcltest-lookup-parser /tmp/tcltest-bodies
```

Intersect the resulting heads with the fresh interpreter's `/tmp/tcltest-core`
command list, and normalise root qualification. Lookup dependencies are checked
in both the root and provider namespaces; a namespace-local shadow, namespace
path, alias prefix, deletion or replacement withdraws the stock implementation
proof. Version-dependent core lookups permit original baseline absence only;
a source deletion is never accepted as optional absence.

The audited core/package pairs are 8.4.20/2.2.11, 8.5.19/2.3.8,
8.6.18/2.5.11, 9.0.4/2.5.11 and 9.1.0/2.6.0. Their conservative core-head
counts are respectively 64, 66, 67, 67 and 67. The two 2.5.11 contracts agree.
