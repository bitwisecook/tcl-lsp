# itest_core.tcl -- iRule loader and event firer
#
# Parses `when EVENT ?priority N? { body }` blocks from iRule source
# and registers them as procs that the event orchestrator can call.
#
# This is sourced by both runner.tcl (Python subprocess bridge) and
# orchestrator.tcl (pure-Tcl usage) to avoid duplicating the core
# iRule loading and event dispatch logic.
#
# Depends on: ::state::event_ctl (from state_layers.tcl)
#
# Copyright (c) 2024 tcl-lsp contributors.  MIT licence.

namespace eval ::itest {

    # event_name -> list of {priority body}
    variable event_handlers
    if {![info exists event_handlers]} {
        array set event_handlers {}
    }

    # List of events actually fired (for fluent assertions)
    variable fired_events [list]

    # Counter for unique handler proc names
    variable _handler_counter 0

    proc load_irule {source} {
        variable event_handlers

        # Parse when blocks from the source
        # Pattern: when EVENT_NAME ?priority N? ?timing on|off? { body }
        set remaining $source
        while {[regexp -indices {(?:^|\n)\s*when\s+([A-Z_][A-Z0-9_]*)} $remaining match event_match]} {
            set event_start [lindex $event_match 0]
            set event_end [lindex $event_match 1]
            set event_name [string range $remaining $event_start $event_end]

            # Advance past event name
            set after_event [expr {[lindex $match 1] + 1}]
            set rest [string range $remaining $after_event end]

            # Parse optional priority and timing
            set priority 500
            set rest [string trimleft $rest]
            if {[regexp {^priority\s+(\d+)\s*} $rest pmatch pval]} {
                set priority $pval
                set rest [string range $rest [string length $pmatch] end]
            }
            set rest [string trimleft $rest]
            if {[regexp {^timing\s+(?:on|off|enable|disable)\s*} $rest tmatch]} {
                set rest [string range $rest [string length $tmatch] end]
            }

            # Find the body (brace-balanced)
            set rest [string trimleft $rest]
            if {[string index $rest 0] eq "\{"} {
                set depth 1
                set pos 1
                set rlen [string length $rest]
                while {$pos < $rlen && $depth > 0} {
                    set ch [string index $rest $pos]
                    if {$ch eq "\{"} { incr depth }
                    if {$ch eq "\}"} { incr depth -1 }
                    if {$ch eq "\\"} { incr pos }
                    incr pos
                }
                set body [string range $rest 1 [expr {$pos - 2}]]

                # Register the handler
                if {![info exists event_handlers($event_name)]} {
                    set event_handlers($event_name) [list]
                }
                # Create a proc for this handler (unique name via counter)
                variable _handler_counter
                incr _handler_counter
                set proc_name "::itest::_handler_${event_name}_${priority}_${_handler_counter}"
                proc $proc_name {} $body

                lappend event_handlers($event_name) [list $priority $proc_name]

                set remaining [string range $rest $pos end]
            } else {
                # No body found, skip
                set remaining $rest
            }
        }
    }

    proc fire_event {event_name} {
        variable event_handlers
        variable current_event
        variable current_priority
        variable fired_events

        set current_event $event_name

        # Track that this event was actually fired
        lappend fired_events $event_name

        # Check if event is disabled
        if {[::state::event_ctl::is_disabled $event_name]} {
            return [list fired 0 reason "disabled"]
        }

        if {![info exists event_handlers($event_name)]} {
            return [list fired 0 reason "no_handler"]
        }

        # Sort handlers by priority (lowest first)
        set handlers $event_handlers($event_name)
        set sorted [lsort -index 0 -integer $handlers]

        set results [list]
        foreach handler $sorted {
            set priority [lindex $handler 0]
            set proc_name [lindex $handler 1]
            set current_priority $priority

            set code [catch {$proc_name} result]
            if {$code == 1} {
                # Error -- single entry with error info
                lappend results [list priority $priority code $code error $result errorInfo $::errorInfo]
            } else {
                lappend results [list priority $priority code $code result $result]
            }
        }

        return [list fired 1 handlers $results]
    }

    # Return list of events that were actually fired (not just registered)
    proc get_fired_events {} {
        variable fired_events
        return $fired_events
    }

    proc clear_irule {} {
        variable event_handlers
        variable fired_events
        # Remove handler procs
        foreach event [array names event_handlers] {
            foreach handler $event_handlers($event) {
                set proc_name [lindex $handler 1]
                catch { ::tmm::_orig_rename $proc_name {} }
            }
        }
        array unset event_handlers
        set fired_events [list]
    }

    proc registered_events {} {
        variable event_handlers
        return [array names event_handlers]
    }
}
