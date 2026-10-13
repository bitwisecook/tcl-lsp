source /workspace/tcl-lsp/rust/tcl-irule-test/tcl/_registry_data.tcl
fconfigure stdout -translation binary
foreach label [concat $::itest::cmd::_gen_namespaced_commands $::itest::cmd::_gen_toplevel_commands] {puts -nonewline "${label}\0"}
