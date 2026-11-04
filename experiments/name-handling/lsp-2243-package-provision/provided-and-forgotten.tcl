# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook)
# SPDX-License-Identifier: AGPL-3.0-or-later
puts [list VERSION [info patchlevel]]
set loader_hits 0
package provide __lsp2243_provided 1.2
set registration_code [catch {
    package ifneeded __lsp2243_provided 1.2 {
        incr loader_hits
        proc __lsp2243_unexpected {} {return changed}
        package provide __lsp2243_provided 1.2
    }
} registration_result]
puts [list IFNEEDED_REGISTRATION_CODE $registration_code RESULT $registration_result]
set provided_code [catch {package require __lsp2243_provided 1.0} provided_result]
puts [list PROVIDED_REQUIRE_CODE $provided_code RESULT $provided_result LOADER_HITS $loader_hits]
set incompatible_code [catch {package require __lsp2243_provided 2.0} incompatible_result]
puts [list INCOMPATIBLE_PROVIDED_CODE $incompatible_code RESULT $incompatible_result LOADER_HITS $loader_hits]
if {$registration_code == 0} {
    package ifneeded __lsp2243_external 1.2 {
        incr loader_hits
        proc __lsp2243_loaded {} {return changed}
        package provide __lsp2243_external 1.2
    }
    set external_code [catch {package require __lsp2243_external 1.0} external_result]
    puts [list EXTERNAL_REQUIRE_CODE $external_code RESULT $external_result LOADER_HITS $loader_hits COMMAND_EXISTS [llength [info commands __lsp2243_loaded]]]
    set core_version [package provide Tcl]
    package forget Tcl
    package ifneeded Tcl $core_version {
        incr loader_hits
        proc __lsp2243_core_reloaded {} {return changed}
        package provide Tcl $core_version
    }
    set forgotten_code [catch {package require Tcl $core_version} forgotten_result]
    puts [list FORGOTTEN_CORE_REQUIRE_CODE $forgotten_code RESULT $forgotten_result LOADER_HITS $loader_hits COMMAND_EXISTS [llength [info commands __lsp2243_core_reloaded]]]
} else {
    puts {EXTERNAL_AND_FORGOTTEN_CASES not-executed-ifneeded-unavailable}
}
