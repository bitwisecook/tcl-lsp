# BPF command and symbol names

BPF source is a closed static language. `bpf-tcl-ir::source` extracts a literal
original head from retained words and delegates descriptor selection to
`CommandRegistry::get_for_surface` with the Registry's own surface query.
Computed or expanded heads decline. A call's `canonical_command` is a report
label and cannot select an emitted operation or consumed declaration.

Consumers read `CommandSpec::bpf_op` and dispatch on `BpfOpKind` and
`BpfDeclKind`. Framework declarations, conditional syntax, template uses,
bounded loops, operations and capability policies share that selection.
A capability operand selects a descriptor through the same Registry;
allow/deny membership uses its `CanonicalCommandDescriptorKey`. A rooted
written spelling cannot bypass a policy covering the selected descriptor.

An independently selected invocation fact supplies `descriptor_key()`.
`CommandRegistry::bpf_op_for_descriptor` queries that exact label without
written-name fallback. Descriptor metadata establishes no Tcl runtime command,
current binding, successful invocation or native entry.

Template parameter bindings, loop variables and profile loads request typed
operations through `source::synthetic_operation_call`. Registry reverse lookup
supplies their generated command labels. These calls have no retained original
Tcl execution receipts. Missing operations produce an explicit error.

Map, local variable, template and profile-field labels are exact identifiers in
separate BPF symbol spaces. Their maps use their original bytes; Tcl namespace
qualification is not applied to those labels. Generated eBPF block/map indexes
select actual operands and relocations. ELF/BTF symbol names and diagnostics are
output labels and confer no Tcl command selection.
