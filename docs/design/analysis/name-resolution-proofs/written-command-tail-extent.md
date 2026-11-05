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

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim, bigip

Status: `not-tested`.

This precise Rust source contract has no native execution receipt.

## Consumer bindings

- [rust/tcl-syntax/src/naming/command_geometry.rs](../../../../rust/tcl-syntax/src/naming/command_geometry.rs), `NativeNameProtocol::command_tail_extent`: Select original native command input and retain the terminal component byte range independently of existence and editing.
- [rust/tcl-syntax/src/naming/command_geometry.rs](../../../../rust/tcl-syntax/src/naming/command_geometry.rs), `naming::command_geometry::tests::original_command_tail_extents_preserve_native_input_and_written_qualifiers` (linked): The shared command-tail owner first applies the selected native command input at known root coordinates, then partitions its original terminal component. C uses the CString prefix; Jim retains its root-flat object. Empty terminal components decline. The returned byte range is readonly correspondence and grants neither command existence nor edit permission.

## Evidence

- [rust/tcl-syntax/src/naming/command_geometry.rs](../../../../rust/tcl-syntax/src/naming/command_geometry.rs)
