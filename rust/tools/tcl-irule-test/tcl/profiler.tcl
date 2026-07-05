# profiler.tcl -- synthetic rule-profiler occurrence emitter
#
# Optional, OFF BY DEFAULT.  When enabled, the event firer
# (itest_core.tcl) and the command dispatcher (command_mocks.tcl unknown
# handler) call ::itest::profiler::emit to record occurrences in the F5
# BIG-IP rule-profiler CSV format.  This lets a test run produce a
# profile (compiler/pgo consumes it) WITHOUT a real BIG-IP.
#
# The occurrence stream is count- and sequence-accurate (it reflects the
# commands and events the test actually exercised); timestamps are a
# monotonic ordinal, NOT wall-clock microseconds, so derived "timing" is
# only a proxy for work done -- not real performance data.
#
# Core Tcl commands compiled to bytecode (set, if, expr, ...) do not pass
# through the unknown handler, so -- exactly as on a real BIG-IP -- they
# do not appear as RP_CMD_ENTRY occurrences.
#
# Copyright (c) 2024 tcl-lsp contributors.  MIT licence.

namespace eval ::itest::profiler {

    variable enabled 0
    variable stream [list]
    variable seq 0

    # Connection / flow context applied to every emitted record.  Mirrors
    # the rule-profiler field layout; defaults are placeholders a test
    # harness can override via ::itest::profiler::configure.
    variable ctx
    if {![info exists ctx]} {
        array set ctx {
            vs          /Common/vs_test
            rule        /Common/test_irule
            pid         1
            flow        0x1
            remote_addr 10.0.0.1
            remote_port 0
            remote_rd   0
            local_addr  10.0.0.250
            local_port  80
            local_rd    0
        }
    }

    # Begin a fresh capture.
    proc enable {} {
        variable enabled
        variable stream
        variable seq
        set enabled 1
        set stream [list]
        set seq 0
    }

    proc disable {} {
        variable enabled
        set enabled 0
    }

    # Override one or more context fields: -vs, -flow, -remote_addr, ...
    proc configure {args} {
        variable ctx
        foreach {key value} $args {
            set name [string trimleft $key -]
            if {[info exists ctx($name)]} {
                set ctx($name) $value
            }
        }
    }

    # The rule name used for RP_RULE_ENTRY / RP_RULE_EXIT.
    proc rule {} {
        variable ctx
        return $ctx(rule)
    }

    # Record one occurrence.  No-op (and cheap) unless enabled.
    proc emit {rp_type value {depth {}}} {
        variable enabled
        if {!$enabled} {
            return
        }
        variable stream
        variable seq
        variable ctx
        incr seq
        lappend stream [join [list \
            $seq $rp_type $ctx(vs) $value $ctx(pid) $ctx(flow) \
            $ctx(remote_addr) $ctx(remote_port) $ctx(remote_rd) \
            $ctx(local_addr) $ctx(local_port) $ctx(local_rd) $depth] ,]
    }

    # Return the captured stream as rule-profiler log text.
    proc dump {} {
        variable stream
        return [join $stream "\n"]
    }
}
