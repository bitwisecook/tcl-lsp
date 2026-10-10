# naming.command.written-tail-extent

Kind: `implementation-contract`

## Problem statement

Linked editing must preserve written namespace qualifiers and ignored native suffix bytes. A Unicode display or a rendered table key cannot identify the corresponding source range.

## Question

Which original byte range denotes the terminal component of a command name under the selected native lookup input?

## Conclusion

The shared command-tail owner first applies the selected native command input at known root coordinates, then partitions its original terminal component. C uses the CString prefix; Jim retains its root-flat object. Empty terminal components decline. The returned byte range is readonly correspondence and grants neither command existence nor edit permission.

## Scope

Current Rust consumer and source geometry contract. Native naming observations retained elsewhere are independently scoped; they do not prove this editor integration. Fixed Rust tests bind the assertions without claiming a native consumer result.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This precise Rust source contract has no native execution receipt.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This precise Rust source contract has no native execution receipt.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This precise Rust source contract has no native execution receipt.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This precise Rust source contract has no native execution receipt.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This precise Rust source contract has no native execution receipt.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Jim Tcl.

This precise Rust source contract has no native execution receipt.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

This precise Rust source contract has no native execution receipt.

## Exact evidence

- `implementation-source` (implementation): [rust/tcl-syntax/src/naming/command_geometry.rs](../../../../rust/tcl-syntax/src/naming/command_geometry.rs). SHA-256 `5ade0f2baf5d03f076ed1b900a8c5bea4937067b62b797b542d2c9d19dec495b`. Current Rust source geometry and consumer assertions; no native or Rust execution receipt.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-syntax/src/naming/command_geometry.rs](../../../../rust/tcl-syntax/src/naming/command_geometry.rs), `NativeNameProtocol::command_tail_extent`: Select original native command input and retain the terminal component byte range independently of existence and editing.
- [rust/tcl-syntax/src/naming/command_geometry.rs](../../../../rust/tcl-syntax/src/naming/command_geometry.rs), `naming::command_geometry::tests::original_command_tail_extents_preserve_native_input_and_written_qualifiers` (linked): The shared command-tail owner first applies the selected native command input at known root coordinates, then partitions its original terminal component. C uses the CString prefix; Jim retains its root-flat object. Empty terminal components decline. The returned byte range is readonly correspondence and grants neither command existence nor edit permission.

A named test is a coverage binding, not a claim that it executed.

## Replay

The named fixed Rust selectors are reproducible source contract checks. Their execution is not asserted by this record. Native experiments are independently scoped.
