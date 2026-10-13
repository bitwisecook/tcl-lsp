// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Bounded original root command operands for external script inventory.

/// Original simple/table words and explicit root-context operands; never report inverses.
/// Namespace or query ambiguity makes the whole inventory unavailable.
#[must_use]
pub const fn original_script_command_inventory_support() -> &'static str {
    SUPPORT
}

const SUPPORT: &str = r#"namespace eval ::__tcl_command_inventory {}
proc ::__tcl_command_inventory::in_root {excluded} {
    if {[::__tcl_name_query::family] eq "jim"} {
        set found {}
        foreach word [uplevel #0 {::info -nons commands}] {
            set private 0
            foreach ns [concat $excluded {::__tcl_command_inventory ::__tcl_name_query}] {
                set prefix [string range $ns 2 end]::
                if {[string first $prefix $word] == 0} {set private 1; break}
            }
            if {$private} continue
            if {[string first "\0" $word] >= 0 || ![uplevel #0 [list ::exists -command $word]]} {
                error "command inventory has no original callable root word"
            }
            lappend found $word
        }
        return [lsort -unique $found]
    }
    set found {}
    set pending [list ::]
    while {[llength $pending]} {
        set ns [lindex $pending 0]
        set pending [lrange $pending 1 end]
        if {[lsearch -exact $excluded $ns] >= 0 || [lsearch -exact {::__tcl_command_inventory ::__tcl_name_query} $ns] >= 0} continue
        if {[string first "\0" $ns] >= 0 || [string range $ns 0 1] ne "::" || [string first ":::" $ns] >= 0 || ($ns ne "::" && [string index $ns end] eq ":")} {
            error "command inventory has no unambiguous namespace address"
        }
        foreach child [::namespace children $ns] {lappend pending $child}
        foreach word [::namespace inscope $ns {::info commands}] {
            if {[string first "\0" $word] >= 0 || [string first :: $word] >= 0} {
                error "command inventory has no original simple command word"
            }
            set selected [::namespace inscope $ns [list ::namespace which -command $word]]
            set expected [expr {$ns eq "::" ? "::$word" : "${ns}::$word"}]
            if {$selected ne $expected} continue
            if {$ns eq "::"} {
                set operand $word
            } else {
                if {[string index $word 0] eq ":"} {error "command inventory has no callable root operand"}
                set operand ${ns}::$word
                if {[uplevel #0 [list ::namespace which -command $operand]] ne $selected} {
                    error "command inventory root lookup disagrees with its original context"
                }
            }
            lappend found $operand
        }
    }
    return [lsort -unique $found]
}
"#;
