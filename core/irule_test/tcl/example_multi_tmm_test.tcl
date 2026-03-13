#!/usr/bin/env tclsh
#
# Example: Multi-TMM testing -- finding CMP bugs
#
# On real BIG-IP, each TMM core has its own copy of static:: variables.
# RULE_INIT fires independently per TMM.  The table command is CMP-shared.
# This example demonstrates testing patterns that catch real CMP bugs:
#
#   1. A rate limiter test that FAILS because static:: is per-TMM
#   2. The same test PASSING after switching to table (CMP-shared)
#   3. Testing TMM init divergence
#
# Expected output (run with: tclsh example_multi_tmm_test.tcl):
#
#   ==== rate-1.0 global rate limit enforced across TMMs FAILED
#   ---- rate-1.0: total 120 requests, but 0 rejected (expected >= 20)
#
#   ==== rate-2.0 global rate limit enforced across TMMs (table fix) PASSED
#
#   Tests failing: [1]
#   FAILED: rate-1.0
#       ...
#   Total  7  Passed  6  Skipped  0  Failed  1

set script_dir [file dirname [info script]]
source [file join $script_dir compat84.tcl]
source [file join $script_dir state_layers.tcl]
source [file join $script_dir tmm_shim.tcl]
source [file join $script_dir expr_ops.tcl]
source [file join $script_dir command_mocks.tcl]
if {[file exists [file join $script_dir _mock_stubs.tcl]]} {
    source [file join $script_dir _mock_stubs.tcl]
}
source [file join $script_dir itest_core.tcl]
source [file join $script_dir orchestrator.tcl]

# ══════════════════════════════════════════════════════════════════
# Scenario 1: BUGGY rate limiter using static:: (per-TMM, not global)
# ══════════════════════════════════════════════════════════════════
#
# This iRule tries to limit each client to 100 requests using a
# static:: variable.  BUG: static:: is per-TMM, so with 4 TMMs a
# client can make 400 requests before being rate-limited.
#
# The test is written the way a developer SHOULD write it: assert
# that the total reject count across all TMMs is correct.  The
# assertion FAILS because the buggy iRule doesn't reject enough.

::orch::configure_tests \
    -tmm_count 4 \
    -profiles {TCP HTTP} \
    -irule {
        when RULE_INIT {
            set static::rate_limit 100
            set static::req_count 0
        }
        when HTTP_REQUEST {
            incr static::req_count

            if { $static::req_count > $static::rate_limit } {
                reject
            } else {
                pool web_pool
            }
        }
    } \
    -setup {
        ::orch::add_pool web_pool {10.0.1.1:80 10.0.1.2:80}
    }

# This test INTENTIONALLY FAILS to demonstrate the CMP bug.
# On a real BIG-IP with 4 TMMs, 120 requests would be distributed
# across TMMs.  Each TMM only sees ~30, so none hit the 100 limit.
::orch::test "rate-1.0" "global rate limit enforced across TMMs" -body {
    # Send 30 requests per TMM = 120 total (exceeds limit of 100)
    set total_rejects 0
    for {set tmm 0} {$tmm < 4} {incr tmm} {
        ::orch::tmm_select $tmm
        for {set i 0} {$i < 30} {incr i} {
            ::orch::run_http_request -host app.example.com
        }
        # Count rejects on this TMM
        set decisions [::itest::get_decisions connection]
        foreach d $decisions {
            if {[lindex $d 1] eq "reject"} {
                incr total_rejects
            }
        }
    }

    # 120 requests sent, limit is 100, so at least 20 should be rejected.
    # BUG: total_rejects will be 0 because each TMM only saw 30 < 100.
    ::orch::assert {$total_rejects >= 20} \
        "rate-1.0: total $total_rejects rejected (expected >= 20)"
}

# ══════════════════════════════════════════════════════════════════
# Scenario 2: CORRECT rate limiter using table (CMP-shared)
# ══════════════════════════════════════════════════════════════════
#
# Same test, but the iRule uses the table command (CMP-shared).
# This test PASSES because all TMMs share the counter.

::orch::configure_tests \
    -tmm_count 4 \
    -profiles {TCP HTTP} \
    -irule {
        when RULE_INIT {
            set static::rate_limit 100
        }
        when HTTP_REQUEST {
            set client [IP::client_addr]
            set count [table incr -subtable rate_limits $client]
            if { $count > $static::rate_limit } {
                reject
            } else {
                pool web_pool
            }
        }
    } \
    -setup {
        ::orch::add_pool web_pool {10.0.1.1:80 10.0.1.2:80}
    }

# This test PASSES -- the table-based counter works across TMMs.
::orch::test "rate-2.0" "global rate limit enforced across TMMs (table fix)" -body {
    set total_rejects 0
    for {set tmm 0} {$tmm < 4} {incr tmm} {
        ::orch::tmm_select $tmm
        for {set i 0} {$i < 30} {incr i} {
            ::orch::run_http_request -host app.example.com
        }
        set decisions [::itest::get_decisions connection]
        foreach d $decisions {
            if {[lindex $d 1] eq "reject"} {
                incr total_rejects
            }
        }
    }

    # CMP-shared table: 120 requests, limit 100, so 20 rejected
    ::orch::assert {$total_rejects >= 20} \
        "rate-2.0: total $total_rejects rejected (expected >= 20)"
}

::orch::test "rate-2.1" "table counter reflects total across all TMMs" -body {
    for {set tmm 0} {$tmm < 4} {incr tmm} {
        ::orch::tmm_select $tmm
        for {set i 0} {$i < 10} {incr i} {
            ::orch::run_http_request -host app.example.com
        }
    }

    # Shared counter: 4 TMMs * 10 = 40 total
    ::orch::tmm_select 0
    set total [table lookup rate_limits 10.0.0.1]
    ::orch::assert_equal $total 40 "table counter should be 40 across all TMMs"
}

# ══════════════════════════════════════════════════════════════════
# Scenario 3: Static variable divergence between TMMs
# ══════════════════════════════════════════════════════════════════
#
# Each TMM initializes independently.  If one TMM processes traffic
# that changes static state, other TMMs won't see that change.

::orch::configure_tests \
    -tmm_count 3 \
    -profiles {TCP HTTP} \
    -irule {
        when RULE_INIT {
            set static::mode "standby"
            set static::version "2.1"
        }
        when HTTP_REQUEST {
            if { $static::mode eq "standby" } {
                set static::mode "active"
            }
            pool web_pool
        }
    } \
    -setup {
        ::orch::add_pool web_pool {10.0.1.1:80}
    }

::orch::test "init-3.0" "each TMM initializes independently" -body {
    # Select all 3 TMMs (fires RULE_INIT on each)
    ::orch::tmm_select 0
    ::orch::tmm_select 1
    ::orch::tmm_select 2

    # All start in "standby"
    ::orch::assert_that tmm_var 0 mode equals standby
    ::orch::assert_that tmm_var 1 mode equals standby
    ::orch::assert_that tmm_var 2 mode equals standby
}

::orch::test "init-3.1" "static mode change is per-TMM only" -body {
    # Select all TMMs, then send traffic only to TMM 1
    ::orch::tmm_select 0
    ::orch::tmm_select 1
    ::orch::tmm_select 2

    ::orch::tmm_select 1
    ::orch::run_http_request -host app.example.com

    # Only TMM 1 transitions to "active"
    ::orch::assert_that tmm_var 0 mode equals standby
    ::orch::assert_that tmm_var 1 mode equals active
    ::orch::assert_that tmm_var 2 mode equals standby
}

::orch::test "init-3.2" "unvisited TMMs have no state" -body {
    # Only visit TMM 0
    ::orch::tmm_select 0
    ::orch::run_http_request -host app.example.com

    # TMM 2 was never selected -- no RULE_INIT, no statics
    set tmm2_mode [::orch::tmm_get_static 2 mode]
    ::orch::assert_equal $tmm2_mode "" \
        "TMM 2 should have no statics (never selected)"
}

# ══════════════════════════════════════════════════════════════════
# Scenario 4: fakeCMP auto-select mode
# ══════════════════════════════════════════════════════════════════
#
# With -tmm_select auto, the framework uses fakeCMP (a simulated
# hash, NOT the real BIG-IP CMP algorithm) to pick the TMM from
# (src_ip, src_port, dst_ip, dst_port).  Same client always lands
# on the same TMM.  Different clients get distributed across TMMs.
#
# fakecmp_suggest_sources finds client addresses that hit each TMM,
# so you don't have to guess.  fakecmp_which_tmm tells you where
# a specific tuple will land.

::orch::configure_tests \
    -tmm_count 4 \
    -tmm_select auto \
    -profiles {TCP HTTP} \
    -irule {
        when RULE_INIT { set static::hits 0 }
        when HTTP_REQUEST {
            incr static::hits
            pool web_pool
        }
    } \
    -setup {
        ::orch::add_pool web_pool {10.0.1.1:80}
    }

::orch::test "fakecmp-4.0" "suggest_sources guarantees coverage" -body {
    # Ask fakeCMP for 2 source tuples per TMM
    set plan [::orch::fakecmp_suggest_sources -count 2]

    # Send traffic using the suggested sources
    foreach tmm_id [::orch::tmm_ids] {
        set sources [dict get $plan $tmm_id]
        foreach {addr port} $sources {
            ::orch::configure -client_addr $addr -client_port $port
            ::orch::run_http_request -host app.example.com
        }
    }

    # Every TMM should have received exactly 2 hits
    foreach tmm_id [::orch::tmm_ids] {
        set hits [::orch::tmm_get_static $tmm_id hits]
        ::orch::assert_equal $hits 2 \
            "TMM $tmm_id should have 2 hits (got $hits)"
    }
}

::orch::test "fakecmp-4.1" "which_tmm matches actual routing" -body {
    # Predict which TMM a tuple will hit, then verify
    set predicted [::orch::fakecmp_which_tmm 10.0.0.42 54321 192.168.1.100 443]

    ::orch::configure -client_addr 10.0.0.42 -client_port 54321
    ::orch::run_http_request -host app.example.com
    set actual [::orch::tmm_current]

    ::orch::assert_equal $predicted $actual \
        "predicted TMM $predicted but routed to TMM $actual"
}

::orch::test "fakecmp-4.2" "same client always lands on same TMM" -body {
    ::orch::configure -client_addr 10.0.0.42 -client_port 54321
    ::orch::run_http_request -host app.example.com
    set first_tmm [::orch::tmm_current]

    ::orch::configure -client_addr 10.0.0.42 -client_port 54321
    ::orch::run_http_request -host app.example.com
    set second_tmm [::orch::tmm_current]

    ::orch::assert_equal $first_tmm $second_tmm \
        "same client should land on same TMM ($first_tmm vs $second_tmm)"
}

::orch::test "fakecmp-4.3" "plan shows human-readable distribution" -body {
    set text [::orch::fakecmp_plan -count 1]
    # Should contain the header and all TMM lines
    ::orch::assert {[string match "*fakeCMP distribution plan*" $text]} \
        "plan output should have header"
    ::orch::assert {[string match "*TMM 0:*" $text]} \
        "plan output should list TMM 0"
    ::orch::assert {[string match "*TMM 3:*" $text]} \
        "plan output should list TMM 3"
}

# ── Summary ──────────────────────────────────────────────────────

exit [::orch::done]
