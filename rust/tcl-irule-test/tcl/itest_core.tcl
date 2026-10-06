# itest_core.tcl -- iRule loader and event firer
#
# Tcl evaluates rule declarations through the when command. The orchestrator
# stores handler bodies; real Tcl frames and interpreters own user cells and
# command tables. This file is shared by standalone tclsh and the Rust VM.
#
# Rust source analysis uses tcl-irules::when_blocks and registry facts; this
# runtime does not reproduce their syntax walker with a textual recogniser.
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
    variable _registration_enabled 1
    variable _loading_rule ""
    variable _current_rule ""
    variable _loaded_rules [list]

    # Framework procedures are copied as executable code; user cells and
    # user command tables are owned by real interpreters, never snapshots.
    proc _copy_framework_proc {worker name} {
        set arguments [list]
        foreach argument [::tmm::_orig_info args $name] {
            if {[::tmm::_orig_info default $name $argument value]} {
                lappend arguments [list $argument $value]
            } else {
                lappend arguments $argument
            }
        }
        ::tmm::_orig_interp eval $worker [list proc $name $arguments [::tmm::_orig_info body $name]]
    }

    proc _inspect_static {name} {
        if {[array exists ::static::$name]} { return [array get ::static::$name] }
        if {[info exists ::static::$name]} { return [set ::static::$name] }
        return ""
    }

    proc create_worker {worker} {
        ::tmm::_orig_interp create $worker
        ::tmm::_orig_interp eval $worker {
            namespace eval ::static {}
            namespace eval ::tmm {}
            namespace eval ::itest {
                variable _flow_script ""
                variable _flow_code 0
                variable _flow_result ""
                variable _flow_error_info ""
                variable _flow_options ""
                variable _executing_rule 0
                variable _current_rule ""
            }
        }
        ::tmm::_orig_interp eval $worker {
            if {[llength [info commands ::tmm::_static_enroll]]} { ::tmm::_static_enroll }
        }
        foreach variable {tmos_version hostname platform reported_tcl_version reported_tcl_major disabled_commands post84_commands} {
            ::tmm::_orig_interp eval $worker [list set ::tmm::$variable [set ::tmm::$variable]]
        }
        foreach name [::tmm::_orig_info procs ::tmm::*] {
            if {[string match "::tmm::_orig_*" $name]} { continue }
            _copy_framework_proc $worker $name
        }
        foreach name {::itest::current_worker ::itest::reset_connection_frame ::itest::_execute_flow_script ::itest::_fire_in_connection_frame ::itest::_execute_event_body ::itest::_inspect_static ::itest::_load_source_for_owner ::itest::_resolve_rule_call ::itest::_validate_rule_identity ::_irh_connection_runner ::_irh_event_completion ::call} {
            _copy_framework_proc $worker $name
        }
        ::tmm::_orig_interp eval $worker {::tmm::init}
        ::tmm::_orig_interp alias $worker unknown {} ::itest::_dispatch_worker_mock
        # Register event handlers in the orchestrator while ordinary proc
        # declarations remain in the worker's own command table.
        ::tmm::_orig_interp alias $worker when {} ::itest::_register_when
        return $worker
    }

    proc _dispatch_worker_mock {command args} {
        set resolved [_resolve_command $command]
        if {$resolved eq ""} { error "invalid command name \"$command\"" }
        if {!$::itest::profiler::enabled} { return [eval $resolved $args] }
        ::itest::profiler::emit RP_CMD_ENTRY $command
        set code [catch {eval $resolved $args} result]
        ::itest::profiler::emit RP_CMD_EXIT $command
        if {$code == 1} {
            return -code error -errorinfo $::errorInfo -errorcode $::errorCode $result
        }
        return -code $code $result
    }

    proc current_worker {} {
        if {[info exists ::orch::_tmm_count] && $::orch::_tmm_count > 1} {
            return [lindex $::orch::_tmm_interpreters $::orch::_tmm_current]
        }
        return ""
    }

    proc _execute_event_body {event body {identity ""}} {
        variable _current_rule
        set previous_rule $_current_rule
        set _current_rule $identity
        variable _executing_rule
        set previous_execution $_executing_rule
        set _executing_rule 1
        if {[llength [::tmm::_orig_info commands ::tmm::_timer_context_enter]]} {
            ::tmm::_timer_context_enter $event $identity
        }
        if {$event eq "RULE_INIT"} {
            set code [::tmm::_host_catch {uplevel #0 $body} result options]
            set code [catch {::_irh_event_completion $result $options} result]
            set completion [list $code $result]
            if {$code == 1} { lappend completion $::errorInfo }
        } else {
            set host_code [catch {_fire_in_connection_frame $body} completion]
            if {$host_code} { set completion [list $host_code $completion $::errorInfo] }
        }
        if {[llength [::tmm::_orig_info commands ::tmm::_timer_context_leave]]} {
            ::tmm::_timer_context_leave
        }
        set _executing_rule $previous_execution
        set _current_rule $previous_rule
        return $completion
    }

    proc _register_when {event args} {
        variable event_handlers
        variable _handler_counter
        variable _registration_enabled
        variable _loading_rule
        set priority 500
        if {[lindex $args 0] eq "priority"} {
            set priority [lindex $args 1]
            set args [lrange $args 2 end]
            if {![string is integer -strict $priority]} { error "invalid event priority \"$priority\"" }
        }
        if {[lindex $args 0] eq "timing"} { set args [lrange $args 2 end] }
        if {[llength $args] != 1} { error "wrong # args: should be \"when event ?priority number? ?timing on|off? body\"" }
        set body [lindex $args 0]
        if {![info complete $body]} { error "incomplete handler body for event \"$event\"" }
        if {!$_registration_enabled} { return }
        if {![info exists event_handlers($event)]} { set event_handlers($event) [list] }
        incr _handler_counter
        set proc_name "::_irh_${event}_${priority}_${_handler_counter}"
        proc $proc_name {} $body
        lappend event_handlers($event) [list $priority $proc_name $_loading_rule]
    }

    proc _validate_rule_identity {identity} {
        set components [split $identity /]
        if {![string match "/*" $identity] || [llength $components] < 3 ||
            [string first :: $identity] >= 0} {
            error "an iRule identity requires an absolute folder/name path"
        }
        foreach component [lrange $components 1 end] {
            if {$component eq "" || $component eq "." || $component eq ".."} {
                error "an iRule identity requires a canonical absolute folder/name path"
            }
        }
        return $identity
    }

    # The namespace below is an explicit simulator command-table owner. The
    # documented F5 call forms do not prove an appliance's namespace current.
    proc _load_source_for_owner {identity source} {
        if {[llength [::tmm::_orig_info commands ::tmm::_timer_rule_loaded]]} {
            ::tmm::_timer_rule_loaded $identity
        }
        variable _executing_rule
        variable _current_rule
        set previous_execution $_executing_rule
        set previous_rule $_current_rule
        set _executing_rule 1
        set _current_rule $identity
        if {$identity eq ""} {
            set code [catch {uplevel #0 $source} result]
        } else {
            set code [catch {::tmm::_orig_namespace eval ::$identity $source} result]
        }
        if {$code == 1} { set error_info $::errorInfo; set error_code $::errorCode }
        set _executing_rule $previous_execution
        set _current_rule $previous_rule
        if {$code == 1} { return -code error -errorinfo $error_info -errorcode $error_code $result }
        return -code $code $result
    }

    proc load_irule {source} { load_rule "" $source }

    proc load_rule {identity source {register_events 1}} {
        variable _registration_enabled
        variable _loading_rule
        variable _loaded_rules
        if {$identity ne ""} {
            _validate_rule_identity $identity
            if {[lsearch -exact $_loaded_rules $identity] >= 0} {
                error "iRule identity already loaded: $identity"
            }
        }
        set source [::tmm::expr_ops::rewrite_irule_source $source]
        set previous_rule $_loading_rule
        set _loading_rule $identity
        set _registration_enabled $register_events
        if {[info exists ::orch::_tmm_count] && $::orch::_tmm_count > 1} {
            # The same declarations execute independently in each interpreter.
            # Record event metadata once, preserving its explicit logical owner.
            set code 0
            foreach worker $::orch::_tmm_interpreters {
                set code [catch {::tmm::_orig_interp eval $worker [list ::itest::_load_source_for_owner $identity $source]} result]
                if {$code} { break }
                set _registration_enabled 0
            }
        } else {
            set code [catch {_load_source_for_owner $identity $source} result]
        }
        if {$code == 1} { set error_info $::errorInfo; set error_code $::errorCode }
        set _loading_rule $previous_rule
        set _registration_enabled 1
        if {$code == 1} { return -code error -errorinfo $error_info -errorcode $error_code $result }
        if {$code} { return -code $code $result }
        if {$identity ne ""} { lappend _loaded_rules $identity }
    }

    # Resolve the public logical call forms against explicit event/proc
    # ownership. There is no default partition inferred from source text.
    proc _resolve_rule_call {target} {
        variable _current_rule
        set separator [string last :: $target]
        if {$separator < 0} {
            if {$_current_rule eq ""} { return [list "" $target] }
            set owner $_current_rule
            set procedure $target
        } else {
            set rule [string range $target 0 [expr {$separator - 1}]]
            set procedure [string range $target [expr {$separator + 2}] end]
            if {[string match "/*" $rule]} {
                set owner [_validate_rule_identity $rule]
            } else {
                if {$_current_rule eq ""} { error "an iRule call requires explicit rule ownership" }
                if {[string first / $rule] >= 0 || [string first :: $rule] >= 0 || $rule eq ""} {
                    error "an iRule call requires a rule name or absolute folder/name path"
                }
                set slash [string last / $_current_rule]
                set owner "[string range $_current_rule 0 [expr {$slash - 1}]]/$rule"
            }
        }
        if {$procedure eq "" || [string first / $procedure] >= 0 || [string first :: $procedure] >= 0} {
            error "an iRule call requires an unqualified procedure name"
        }
        return [list $owner ::${owner}::$procedure]
    }

    # A traffic connection retains one real Tcl proc activation. The host's
    # coroutine command is private: it is not an iRules language capability.
    # No user variable is snapshotted, so aliases and traces retain their cells.
    variable _flow_script ""
    variable _flow_code 0
    variable _flow_result ""
    variable _flow_error_info ""
    variable _flow_options ""
    variable _executing_rule 0

    proc reset_connection_frame {} {
        set worker [current_worker]
        if {$worker ne ""} {
            ::tmm::_orig_interp eval $worker {::itest::reset_connection_frame}
            return
        }
        if {![catch {::tmm::_orig_info commands ::_irh_connection_frame} commands] &&
            [llength $commands]} {
            ::tmm::_orig_rename ::_irh_connection_frame {}
        }
    }

    proc _execute_flow_script {} {
        variable _flow_script
        variable _flow_code
        variable _flow_result
        variable _flow_error_info
        variable _flow_options
        # uplevel executes in the retained proc activation, not this helper.
        set _flow_code [::tmm::_host_catch {uplevel 1 $_flow_script} _flow_result _flow_options]
        # A real Tcl procedure consumes return levels and handles illegal
        # break/continue at the event boundary; no completion-code guess.
        set _flow_code [catch {::_irh_event_completion $_flow_result $_flow_options} _flow_result]
        if {$_flow_code == 1} { set _flow_error_info $::errorInfo }
    }

    proc _fire_in_connection_frame {body} {
        variable _flow_script
        variable _flow_code
        variable _flow_result
        variable _flow_error_info
        if {![llength [::tmm::_orig_info commands ::tmm::_orig_coroutine]] ||
            ![llength [::tmm::_orig_info commands ::tmm::_orig_yield]]} {
            error "persistent iRules connection frames require host coroutine support (Tcl 8.6+)"
        }
        if {![llength [::tmm::_orig_info commands ::_irh_connection_frame]]} {
            ::tmm::_orig_coroutine ::_irh_connection_frame ::_irh_connection_runner
        }
        set _flow_script $body
        ::_irh_connection_frame
        return [list $_flow_code $_flow_result $_flow_error_info]
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

        # Profiler hooks (no-op unless enabled): bracket the event and each
        # rule handler so the occurrence stream mirrors the iRule timing
        # hierarchy (event -> rule -> commands).
        ::itest::profiler::emit RP_EVENT_ENTRY $event_name
        set results [list]
        foreach handler $sorted {
            set priority [lindex $handler 0]
            set proc_name [lindex $handler 1]
            set identity [lindex $handler 2]
            set current_priority $priority

            ::itest::profiler::emit RP_RULE_ENTRY [::itest::profiler::rule]
            set body [::tmm::_orig_info body $proc_name]
            set worker [current_worker]
            if {$worker eq ""} {
                set completion [_execute_event_body $event_name $body $identity]
            } else {
                set completion [::tmm::_orig_interp eval $worker [list ::itest::_execute_event_body $event_name $body $identity]]
            }
            set code [lindex $completion 0]
            set result [lindex $completion 1]
            ::itest::profiler::emit RP_RULE_EXIT [::itest::profiler::rule]
            if {$code == 1} {
                # Error -- single entry with error info
                lappend results [list priority $priority code $code error $result errorInfo [lindex $completion 2]]
            } else {
                lappend results [list priority $priority code $code result $result]
            }
        }
        ::itest::profiler::emit RP_EVENT_EXIT $event_name

        return [list fired 1 handlers $results]
    }

    # Return list of events that were actually fired (not just registered)
    proc get_fired_events {} {
        variable fired_events
        return $fired_events
    }

    proc clear_irule {} {
        if {[llength [::tmm::_orig_info commands ::tmm::_timer_retire]]} {
            ::tmm::_timer_retire
        }
        if {[info exists ::orch::_tmm_count] && $::orch::_tmm_count > 1} {
            foreach owned_worker $::orch::_tmm_interpreters {
                ::tmm::_orig_interp eval $owned_worker {
                    if {[llength [::tmm::_orig_info commands ::tmm::_timer_retire]]} {
                        ::tmm::_timer_retire
                    }
                    ::itest::reset_connection_frame
                }
            }
        } else { ::itest::reset_connection_frame }
        variable event_handlers
        variable fired_events
        variable _loaded_rules
        # Retire the named command-table owners, not copied cell contents.
        foreach identity $_loaded_rules {
            if {[info exists ::orch::_tmm_count] && $::orch::_tmm_count > 1} {
                foreach owned_worker $::orch::_tmm_interpreters {
                    catch {::tmm::_orig_interp eval $owned_worker [list ::tmm::_orig_namespace delete ::$identity]}
                }
            } else { catch {::tmm::_orig_namespace delete ::$identity} }
        }
        set _loaded_rules [list]
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

# This proc is deliberately global: the retained flow frame has Tcl's root
# command/namespace context. Its bookkeeping lives in ::itest, not user locals.
proc ::_irh_connection_runner {} {
    if {[llength [::tmm::_orig_info commands ::tmm::_timer_anchor]]} {
        ::tmm::_timer_anchor
    }
    while {1} {
        ::tmm::_orig_yield
        ::itest::_execute_flow_script
    }
}

# This procedure supplies only a completion boundary. User code has already
# executed in the actual global/connection activation with all options retained.
proc ::_irh_event_completion {result options} { ::tmm::_host_return -options $options $result }

# Tcl evaluates the declaration syntax; the registry/compiler owns analysis.
proc ::when {event args} { eval [list ::itest::_register_when $event] $args }

# call executes in the caller's frame before entering the target proc, so a
# helper's upvar 1 observes the retained connection activation.
proc ::call {args} {
    if {[lindex $args 0] eq "-debug"} { set args [lrange $args 1 end] }
    if {![llength $args]} { error "wrong # args: should be \"call ?-debug? proc_name ?arg ...?\"" }
    set resolved [::itest::_resolve_rule_call [lindex $args 0]]
    set previous_rule $::itest::_current_rule
    set ::itest::_current_rule [lindex $resolved 0]
    set command [linsert [lrange $args 1 end] 0 [lindex $resolved 1]]
    set code [catch {uplevel 1 $command} result]
    if {$code == 1} { set error_info $::errorInfo; set error_code $::errorCode }
    set ::itest::_current_rule $previous_rule
    if {$code == 1} { return -code error -errorinfo $error_info -errorcode $error_code $result }
    return -code $code $result
}
