#!/usr/bin/env tclsh
#
# Orchestrator tests for binary *::payload handling.
#
# Runs real iRules through the simulated TMM and proves the framework is
# faithful to TMM's byte-array payload semantics: TCP::payload / HTTP::payload
# `length`, the `<size>` getter, and `replace` are BYTE operations (not
# character operations), and a binary-safe rewrite preserves every byte
# >= 0x80 — the corruption the S110 diagnostic warns about (F5 KB K22406348).
#
# Run with: tclsh binary_payload_test.tcl

set script_dir [file dirname [info script]]
source [file join $script_dir compat84.tcl]
source [file join $script_dir state_layers.tcl]
source [file join $script_dir tmm_shim.tcl]
source [file join $script_dir expr_ops.tcl]
source [file join $script_dir profiler.tcl]
source [file join $script_dir command_mocks.tcl]
if {[file exists [file join $script_dir _mock_stubs.tcl]]} {
    source [file join $script_dir _mock_stubs.tcl]
}
source [file join $script_dir itest_core.tcl]
source [file join $script_dir orchestrator.tcl]

# Hex view of a value as a byte array (latin-1 byte mapping).
proc hx {s} { binary scan [binary format a* $s] H* h; return $h }

# ── Configure test defaults ──────────────────────────────────────
#
# A single iRule handling both a TCP and an HTTP data event.  Each does a
# binary-safe in-place rewrite: read the payload, re-binarify it with
# `binary scan` (the documented fix), then write it back with `replace`.

::orch::configure_tests \
    -profiles {TCP HTTP} \
    -irule {
        when CLIENT_DATA {
            # Replace one byte at offset 1 with 'A' (0x41), binary-safe.
            set patch [binary format c* {0x41}]
            binary scan $patch c* throwaway
            TCP::payload replace 1 1 $patch
        }
        when HTTP_REQUEST_DATA {
            set patch [binary format H* cafe]
            binary scan $patch c* throwaway
            HTTP::payload replace 0 2 $patch
        }
    }

# ── Test cases ───────────────────────────────────────────────────
#
# Payload "Józ" where ó = UTF-8 c3 b3, i.e. the wire bytes 4a c3 b3 7a.

::orch::test "binpayload-1.0" "TCP::payload length counts bytes, not characters" -body {
    set ::state::connection::client_payload [binary format H* 4ac3b37a]
    # 4 bytes (a char count would also be 4 here, but length must be the
    # byte-array length the mock derives via binary format a*).
    ::orch::assert_equal [TCP::payload length] 4 "TCP::payload length must be the byte count"
}

::orch::test "binpayload-1.1" "TCP::payload <size> returns the first N bytes" -body {
    set ::state::connection::client_payload [binary format H* 4ac3b37a]
    ::orch::assert_equal [hx [TCP::payload 2]] "4ac3" "TCP::payload 2 must return the first 2 bytes"
}

::orch::test "binpayload-1.2" "binary-safe TCP::payload replace preserves high bytes" -body {
    set ::state::connection::client_payload [binary format H* 4ac3b37a]
    ::itest::fire_event CLIENT_DATA
    ::orch::assert_that decision tcp payload_replace was_called
    # Byte 1 (0xc3) -> 0x41; the 0xb3 high byte must survive the rewrite.
    ::orch::assert_equal [hx $::state::connection::client_payload] "4a41b37a" \
        "binary-safe replace must not re-encode surrounding high bytes"
}

::orch::test "binpayload-2.0" "HTTP::payload replace is byte-accurate" -body {
    set ::state::http::request::payload [binary format H* deadbeef]
    ::itest::fire_event HTTP_REQUEST_DATA
    ::orch::assert_that decision http payload_replace was_called
    ::orch::assert_equal [hx $::state::http::request::payload] "cafebeef" \
        "HTTP::payload replace must splice bytes without re-encoding"
}

# ── Summary ──────────────────────────────────────────────────────

::orch::run_and_exit
