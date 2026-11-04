# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook)
# SPDX-License-Identifier: AGPL-3.0-or-later
puts [list VERSION [info patchlevel]]
set provision_code [catch {package provide Tcl} provided]
puts [list INITIAL_TCL_PROVISION_CODE $provision_code VALUE $provided]
set commands_before [lsort [info commands ::*]]
set header_code [catch {package require Tcl 8.5 9} header_result]
set commands_after [lsort [info commands ::*]]
puts [list EXACT_HEADER_CODE $header_code RESULT $header_result]
puts [list GLOBAL_COMMAND_NAMES_UNCHANGED [string equal $commands_before $commands_after]]
set provide_code [catch {package provide imap4 0.5.5} provide_result]
puts [list IMAP4_PROVIDE_CODE $provide_code RESULT $provide_result]
