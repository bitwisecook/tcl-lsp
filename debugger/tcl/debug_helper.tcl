# debug_helper.tcl — Tcl-side instrumentation for the tclsh debug backend.
#
# Loaded into a tclsh subprocess before sourcing the user's script.
# Sets up ``trace add execution source enterstep`` so that every command
# evaluated during ``source`` is reported to the Python controller via
# JSON over stdout/stdin.

package require Tcl 8.5

# Namespace for debugger state
namespace eval ::__dbg {
    variable breakpoints {}
    variable step_mode "step_in"
    variable prev_line -1
    variable script_file ""
}

# Minimal JSON encoder (no external deps)
proc ::__dbg::json_encode {dict_val} {
    set parts {}
    dict for {k v} $dict_val {
        set ek [string map {\" \\\" \\ \\\\ \n \\n \r \\r \t \\t} $k]
        set ev [string map {\" \\\" \\ \\\\ \n \\n \r \\r \t \\t} $v]
        lappend parts "\"$ek\":\"$ev\""
    }
    return "\{[join $parts ,]\}"
}

# Encode a list of dicts as a JSON array
proc ::__dbg::json_encode_vars {var_list} {
    set items {}
    foreach item $var_list {
        lappend items [json_encode $item]
    }
    return "\[[join $items ,]\]"
}

# Collect variables in the caller's scope
proc ::__dbg::collect_vars {{level 2}} {
    set result {}
    foreach v [uplevel $level {info vars}] {
        # Skip debugger internals
        if {[string match "::__dbg::*" $v] || [string match "__dbg_*" $v]} continue
        if {[uplevel $level [list array exists $v]]} {
            lappend result [dict create name $v type array value "(array)"]
        } else {
            if {[catch {uplevel $level [list set $v]} val]} {
                set val "<undefined>"
            }
            lappend result [dict create name $v type scalar value $val]
        }
    }
    return $result
}

# The enterstep trace callback
proc ::__dbg::step_callback {args} {
    variable breakpoints
    variable step_mode
    variable prev_line

    # Get the current source location
    set depth [info frame]
    if {$depth < 2} return

    # Walk up the frame stack to find the user's source frame
    set line 0
    set cmd_text ""
    for {set i $depth} {$i >= 1} {incr i -1} {
        set finfo [info frame $i]
        if {[dict exists $finfo line]} {
            set line [dict get $finfo line]
            if {[dict exists $finfo cmd]} {
                set cmd_text [dict get $finfo cmd]
            }
            break
        }
    }

    # Skip if same line as before (unless stepping)
    if {$line == $prev_line && $step_mode eq "continue"} return
    if {$line == 0} return

    # Check whether we should stop
    set should_stop 0
    if {$step_mode eq "step_in"} {
        set should_stop 1
    } elseif {$line in $breakpoints} {
        set should_stop 1
    } elseif {$step_mode eq "step_over" || $step_mode eq "step_out"} {
        # Simplified: always stop for now (proper depth tracking later)
        set should_stop 1
    }

    if {!$should_stop} return

    set prev_line $line

    # Truncate command text
    if {[string length $cmd_text] > 200} {
        set cmd_text "[string range $cmd_text 0 197]..."
    }

    # Collect variables from the caller's scope
    set vars [collect_vars 3]

    # Build stack trace
    set stack_frames {}
    for {set i $depth} {$i >= 1} {incr i -1} {
        catch {
            set fi [info frame $i]
            set fl 0
            set fn "global"
            if {[dict exists $fi line]} { set fl [dict get $fi line] }
            if {[dict exists $fi proc]} { set fn [dict get $fi proc] }
            lappend stack_frames [dict create name $fn line $fl]
        }
    }

    # Send stop event as JSON
    set event [dict create \
        type stopped \
        line $line \
        cmd $cmd_text \
    ]
    puts stdout [json_encode $event]
    flush stdout

    # Wait for command from controller
    while {1} {
        if {[gets stdin response] < 0} {
            # Controller disconnected — exit
            exit 0
        }
        set response [string trim $response]
        if {$response eq ""} continue

        # Parse the JSON command (simple key:value parsing)
        # Expected: {"cmd":"continue"} or {"cmd":"step_in"} etc.
        if {[regexp {"cmd"\s*:\s*"(\w+)"} $response _ cmd]} {
            switch -exact -- $cmd {
                continue {
                    set step_mode "continue"
                    return
                }
                step_in {
                    set step_mode "step_in"
                    return
                }
                step_over {
                    set step_mode "step_over"
                    return
                }
                step_out {
                    set step_mode "step_out"
                    return
                }
                get_vars {
                    set vars [collect_vars 3]
                    puts stdout "\{\"type\":\"vars\",\"data\":[json_encode_vars $vars]\}"
                    flush stdout
                }
                set_breakpoints {
                    if {[regexp {"lines"\s*:\s*\[([^\]]*)\]} $response _ lines_str]} {
                        set breakpoints {}
                        foreach l [split $lines_str ,] {
                            set l [string trim $l " \""]
                            if {$l ne ""} { lappend breakpoints $l }
                        }
                    }
                    puts stdout [json_encode [dict create type ack cmd set_breakpoints]]
                    flush stdout
                }
                terminate {
                    exit 0
                }
                default {
                    puts stdout [json_encode [dict create type error message "unknown command: $cmd"]]
                    flush stdout
                }
            }
        }
    }
}

# Main entry — called by Python with the script path as argument
proc ::__dbg::main {script_path} {
    variable script_file
    set script_file $script_path

    # Signal ready
    puts stdout [json_encode [dict create type ready]]
    flush stdout

    # Wait for initial configuration (breakpoints, etc.)
    while {1} {
        if {[gets stdin response] < 0} { exit 0 }
        set response [string trim $response]
        if {$response eq ""} continue

        if {[regexp {"cmd"\s*:\s*"(\w+)"} $response _ cmd]} {
            if {$cmd eq "start"} {
                # Extract breakpoints if provided
                if {[regexp {"lines"\s*:\s*\[([^\]]*)\]} $response _ lines_str]} {
                    variable breakpoints
                    set breakpoints {}
                    foreach l [split $lines_str ,] {
                        set l [string trim $l " \""]
                        if {$l ne ""} { lappend breakpoints $l }
                    }
                }
                break
            } elseif {$cmd eq "set_breakpoints"} {
                if {[regexp {"lines"\s*:\s*\[([^\]]*)\]} $response _ lines_str]} {
                    variable breakpoints
                    set breakpoints {}
                    foreach l [split $lines_str ,] {
                        set l [string trim $l " \""]
                        if {$l ne ""} { lappend breakpoints $l }
                    }
                }
                puts stdout [json_encode [dict create type ack cmd set_breakpoints]]
                flush stdout
            }
        }
    }

    # Set up the execution trace on 'source'
    trace add execution source enterstep ::__dbg::step_callback

    # Source the user's script
    if {[catch {source $script_path} result opts]} {
        puts stdout [json_encode [dict create type error message $result]]
        flush stdout
    }

    # Signal completion
    puts stdout [json_encode [dict create type finished]]
    flush stdout
}

# Auto-run if script path given as argument
if {$argc > 0} {
    ::__dbg::main [lindex $argv 0]
}
