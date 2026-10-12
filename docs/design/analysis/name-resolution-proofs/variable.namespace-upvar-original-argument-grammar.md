# namespace upvar original argument grammar

ID: `naming.variable.namespace-upvar-original-argument-grammar`

## Problem statement

A namespace-upvar argument vector can be unavailable, require at least one pair, accept an empty pair vector, or be padded by a helper. Reusing one arity rule across these dialects would skip an actual error or manufacture a local alias from the wrong effective arguments.

## Question

What completion does namespace upvar return for zero, one and two operands after the namespace in each captured engine?

## Answers

### tcl8.4 — unsupported

{namespace upvar n} 1 {bad option "upvar": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which}
{namespace upvar n a} 1 {bad option "upvar": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which}
{namespace upvar n a b} 1 {bad option "upvar": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which}

Version: C84 label; exact patch/revision not recorded. Build: Binary path /workspace/tcl-lsp/tmp/tcl8.4.20/unix/tclsh; SHA not recorded. Input channel: Exact ASCII source.tcl via native shell.

### tcl8.5 — observed

{namespace upvar n} 1 {wrong # args: should be "namespace upvar ns otherVar myVar ?otherVar myVar ...?"}
{namespace upvar n a} 1 {wrong # args: should be "namespace upvar ns otherVar myVar ?otherVar myVar ...?"}
{namespace upvar n a b} 0 {}

Version: C85 label; exact patch/revision not recorded. Build: Binary path /workspace/tcl-lsp/tmp/tcl8.5.19/unix/tclsh; SHA not recorded. Input channel: Exact ASCII source.tcl via native shell.

### tcl8.6 — observed

{namespace upvar n} 0 {}
{namespace upvar n a} 1 {wrong # args: should be "namespace upvar ns ?otherVar myVar ...?"}
{namespace upvar n a b} 0 {}

Version: C86 label; exact patch/revision not recorded. Build: Binary path /workspace/tcl-lsp/tmp/tcl8.6.18/unix/tclsh; SHA not recorded. Input channel: Exact ASCII source.tcl via native shell.

### tcl9.0 — observed

{namespace upvar n} 0 {}
{namespace upvar n a} 1 {wrong # args: should be "namespace upvar ns ?otherVar myVar ...?"}
{namespace upvar n a b} 0 {}

Version: C90 label; exact patch/revision not recorded. Build: Binary path /workspace/tcl-lsp/tmp/tcl9.0.4/unix/tclsh; SHA not recorded. Input channel: Exact ASCII source.tcl via native shell.

### tcl9.1 — observed

{namespace upvar n} 0 {}
{namespace upvar n a} 1 {wrong # args: should be "namespace upvar ns ?otherVar myVar ...?"}
{namespace upvar n a b} 0 {}

Version: C91 label; exact patch/revision not recorded. Build: Binary path /workspace/tcl-lsp/tmp/tcl9.1.0/unix/tclsh; SHA not recorded. Input channel: Exact ASCII source.tcl via native shell.

### jim — observed

{namespace upvar n} 1 {wrong # args: should be "upvar ?level? otherVar myVar ?otherVar myVar ...?"}
{namespace upvar n a} 0 {}
{namespace upvar n a b} 0 {}

Version: Jim label; exact patch/revision not recorded. Build: Binary path /tmp/2286-oracles/jimtcl/jimsh; SHA not recorded. Input channel: Exact ASCII source.tcl via native shell.

### bigip — not-tested

No appliance observation for this question.

Version: not recorded. Build: not recorded. Input channel: not recorded.


## Conclusion

C8.4 rejects the unavailable subcommand. C8.5 requires at least one complete pair. C8.6/9.0/9.1 accept zero pairs and reject the odd vector. The measured Jim helper errors on zero operands but accepts the one-operand padded and complete-pair vectors. This is original argument grammar, not compiled opcode admission or proof of a linked cell.

## Scope

Exact ASCII namespace creation and three catch-presented original argument vectors. Captured C84/C85/C86/C90/C91/Jim labels and binary paths; exact patches/revisions and binary SHAs are not retained. BIG-IP not queried.

## Exact retained evidence

- `rust/tcl-registry/tests/data/native_namespace_upvar_arguments/manifest.json` SHA256 `13464962121fe902ff87cd5377c28ec8671ce8bb64fb0fce5f0c880923a9d8e8`: Exact original native provenance/captured observations.
- `rust/tcl-registry/tests/data/native_namespace_upvar_arguments/source.tcl` SHA256 `938e4ee3c0e7ed17eec76d45991555841fb29cfb3a549b4bb6e8115099e41e59`: Retained exact script/case bytes for the stated question.

## Replay

```sh
/path/to/recorded/native/tclsh rust/tcl-registry/tests/data/native_namespace_upvar_arguments/source.tcl
```

Run this retained ASCII script with the recorded native shell in a fresh process. The script supplies its own catch/result presenter. Require every original stdout row, empty stderr and recorded process completion. Exact provider builds are not retained and no fresh rerun is claimed.

## Rust comparison

`cmd_namespace::native_upvar_tests::namespace_upvar_original_vectors_match_eighteen_native_arity_windows` in `runtime/rust/src/cmd_namespace/native_upvar_tests.rs`. No Rust execution result is recorded here.
