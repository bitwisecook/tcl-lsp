# Example: inline dialect stubs for a custom EDA workflow
#
# Stubs are declared inside stubs-begin/end markers.
# Multiple stubs blocks are allowed in a single file.

# tcl-lsp: stubs-begin
# tcl-lsp: stub foreach_in_collection {varName:var collection body:body} -loop
# tcl-lsp: stub get_cells {?-hierarchical? ?-filter? pattern:pattern} -pure
# tcl-lsp: stub get_ports {?-filter? pattern:pattern} -pure
# tcl-lsp: stub get_attribute {object attrName:name} -pure
# tcl-lsp: stub sizeof_collection {collection} -pure
# tcl-lsp: stub expr-func sizeof 1
# tcl-lsp: stubs-end

# Now the LSP understands these commands
set all_cells [get_cells -hierarchical *]
set count [sizeof_collection $all_cells]

foreach_in_collection cell $all_cells {
    set name [get_attribute $cell full_name]
    puts "Cell: $name"
}

# A second stubs block for a different subsystem
# tcl-lsp: stubs-begin
# tcl-lsp: stub redirect {?-variable? ?-file? target body:body}
# tcl-lsp: stub report_timing {?-from? ?-to? ?-delay_type?}
# tcl-lsp: stub expr-op contains 2
# tcl-lsp: stubs-end

redirect -file timing.rpt {
    report_timing -from [get_ports in*] -to [get_ports out*]
}
