#!/usr/bin/env tclsh
#
# Example: testing an iRule that routes based on Host header
#
# Demonstrates the structured test runner:
#   - ::orch::configure_tests -- set defaults (iRule, profiles, shared setup)
#   - ::orch::test            -- tcltest-style named test cases
#   - ::orch::done            -- print summary and return exit code
#
# Run with: tclsh example_test.tcl
#
# Output format matches tcltest so editors can parse PASSED/FAILED lines.

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

# ── Configure test defaults ──────────────────────────────────────

::orch::configure_tests \
    -profiles {TCP CLIENTSSL HTTP} \
    -irule {
        when HTTP_REQUEST {
            set host [string tolower [HTTP::host]]

            if { $host eq "api.example.com" } {
                pool api_pool
            } elseif { $host eq "www.example.com" } {
                pool web_pool
            } else {
                if { [class match $host equals allowed_hosts] } {
                    pool default_pool
                } else {
                    log local0. "Rejected request from unknown host: $host"
                    reject
                }
            }

            HTTP::header insert "X-Forwarded-For" [IP::client_addr]
        }
    } \
    -setup {
        ::orch::add_pool api_pool {10.0.1.1:8080 10.0.1.2:8080}
        ::orch::add_pool web_pool {10.0.2.1:80 10.0.2.2:80}
        ::orch::add_pool default_pool {10.0.3.1:80}
        ::orch::add_datagroup allowed_hosts string {
            api.example.com ""
            www.example.com ""
            admin.example.com ""
        }
    }

# ── Test cases ───────────────────────────────────────────────────

::orch::test "routing-1.0" "API request routes to api_pool" -body {
    ::orch::run_http_request -host api.example.com -uri /v1/health
    ::orch::assert_that pool_selected equals api_pool
}

::orch::test "routing-1.1" "Web request routes to web_pool" -body {
    ::orch::run_http_request -host www.example.com -uri /
    ::orch::assert_that pool_selected equals web_pool
}

::orch::test "routing-1.2" "Allowed host routes to default_pool" -body {
    ::orch::run_http_request -host admin.example.com -uri /admin
    ::orch::assert_that pool_selected equals default_pool
}

::orch::test "routing-1.3" "Unknown host gets rejected" -body {
    ::orch::run_http_request -host evil.example.com -uri /
    ::orch::assert_that decision connection reject was_called
    ::orch::assert_that log matches "*Rejected request from unknown host*"
}

::orch::test "headers-2.0" "X-Forwarded-For header inserted" -body {
    ::orch::configure -client_addr 10.0.0.50
    ::orch::run_http_request -host api.example.com -uri /
    ::orch::assert_that decision http header_insert was_called
}

::orch::test "routing-3.0" "Case insensitive host matching" -body {
    ::orch::run_http_request -host API.EXAMPLE.COM -uri /
    ::orch::assert_that pool_selected equals api_pool
}

# ── Summary ──────────────────────────────────────────────────────

exit [::orch::done]
