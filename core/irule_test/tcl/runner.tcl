# runner.tcl -- Test runner and Python-Tcl communication protocol
#
# This is the entry point that Python launches via tclsh.  It:
#   1. Sources the framework components (compat, shim, state, mocks, expr_ops)
#   2. Listens on stdin for JSON-like commands from the Python orchestrator
#   3. Executes commands and returns results on stdout
#
# Protocol (line-oriented, one JSON object per line):
#
#   Python -> Tcl:
#     {"cmd": "init", "tmos_version": "16.1"}
#     {"cmd": "load_irule", "source": "when HTTP_REQUEST { ... }"}
#     {"cmd": "set_state", "layer": "connection", "values": {"client_addr": "10.0.0.1"}}
#     {"cmd": "fire_event", "event": "HTTP_REQUEST", "priority": 500}
#     {"cmd": "get_state", "layer": "http"}
#     {"cmd": "get_decisions"}
#     {"cmd": "get_logs"}
#     {"cmd": "reset", "scope": "connection|request|all"}
#     {"cmd": "add_pool", "name": "web_pool", "members": ["10.0.1.1:80"]}
#     {"cmd": "add_datagroup", "name": "allowed_hosts", "type": "string", "records": {"host1": "", "host2": ""}}
#     {"cmd": "eval", "script": "..."}
#     {"cmd": "quit"}
#
#   Tcl -> Python:
#     {"status": "ok", "result": ...}
#     {"status": "error", "message": "...", "errorInfo": "..."}
#
# Copyright (c) 2024 tcl-lsp contributors.  MIT licence.

# Determine our directory
set _runner_dir [file dirname [info script]]

# Source framework components in order
source [file join $_runner_dir compat84.tcl]
source [file join $_runner_dir state_layers.tcl]
source [file join $_runner_dir tmm_shim.tcl]
source [file join $_runner_dir expr_ops.tcl]
source [file join $_runner_dir command_mocks.tcl]
if {[file exists [file join $_runner_dir _mock_stubs.tcl]]} {
    source [file join $_runner_dir _mock_stubs.tcl]
}
source [file join $_runner_dir itest_core.tcl]
source [file join $_runner_dir orchestrator.tcl]

# ── Minimal JSON parser/emitter ───────────────────────────────────
#
# We need JSON for the protocol but can't require external packages.
# This is deliberately minimal -- handles the protocol's needs only.

namespace eval ::proto {

    # Parse a JSON-ish line into a Tcl dict.
    # Handles: strings, numbers, booleans, null, arrays, nested objects.
    # This is not a full JSON parser but handles our protocol.
    proc json_parse {json_str} {
        set json_str [string trim $json_str]

        # Simple recursive-descent parser
        set pos 0
        set result [_parse_value $json_str pos]
        return $result
    }

    proc _skip_ws {str posvar} {
        upvar 1 $posvar pos
        set len [string length $str]
        while {$pos < $len && [string is space [string index $str $pos]]} {
            incr pos
        }
    }

    proc _parse_value {str posvar} {
        upvar 1 $posvar pos
        _skip_ws $str pos
        set ch [string index $str $pos]

        switch -exact -- $ch {
            "\{" { return [_parse_object $str pos] }
            "\[" { return [_parse_array $str pos] }
            "\"" { return [_parse_string $str pos] }
            "t" - "f" { return [_parse_bool $str pos] }
            "n" {
                # null
                incr pos 4
                return ""
            }
            default {
                # Number
                return [_parse_number $str pos]
            }
        }
    }

    proc _parse_string {str posvar} {
        upvar 1 $posvar pos
        incr pos  ;# skip opening quote
        set result ""
        set len [string length $str]

        while {$pos < $len} {
            set ch [string index $str $pos]
            if {$ch eq "\""} {
                incr pos
                return $result
            }
            if {$ch eq "\\"} {
                incr pos
                set esc [string index $str $pos]
                switch -exact -- $esc {
                    "n" { append result "\n" }
                    "t" { append result "\t" }
                    "r" { append result "\r" }
                    "\\" { append result "\\" }
                    "\"" { append result "\"" }
                    "/" { append result "/" }
                    default { append result $esc }
                }
            } else {
                append result $ch
            }
            incr pos
        }
        return $result
    }

    proc _parse_number {str posvar} {
        upvar 1 $posvar pos
        set start $pos
        set len [string length $str]
        while {$pos < $len} {
            set ch [string index $str $pos]
            if {[string is digit $ch] || $ch eq "." || $ch eq "-" ||
                $ch eq "+" || $ch eq "e" || $ch eq "E"} {
                incr pos
            } else {
                break
            }
        }
        return [string range $str $start [expr {$pos - 1}]]
    }

    proc _parse_bool {str posvar} {
        upvar 1 $posvar pos
        if {[string range $str $pos [expr {$pos + 3}]] eq "true"} {
            incr pos 4
            return 1
        }
        if {[string range $str $pos [expr {$pos + 4}]] eq "false"} {
            incr pos 5
            return 0
        }
        error "invalid boolean at position $pos"
    }

    proc _parse_object {str posvar} {
        upvar 1 $posvar pos
        incr pos  ;# skip {
        set result [list]
        _skip_ws $str pos
        set len [string length $str]

        while {$pos < $len && [string index $str $pos] ne "\}"} {
            _skip_ws $str pos
            # Parse key
            set key [_parse_string $str pos]
            _skip_ws $str pos
            incr pos  ;# skip :
            _skip_ws $str pos
            # Parse value
            set val [_parse_value $str pos]
            lappend result $key $val
            _skip_ws $str pos
            if {[string index $str $pos] eq ","} {
                incr pos
            }
        }
        if {$pos < $len} { incr pos }  ;# skip }
        return $result
    }

    proc _parse_array {str posvar} {
        upvar 1 $posvar pos
        incr pos  ;# skip [
        set result [list]
        _skip_ws $str pos
        set len [string length $str]

        while {$pos < $len && [string index $str $pos] ne "\]"} {
            _skip_ws $str pos
            lappend result [_parse_value $str pos]
            _skip_ws $str pos
            if {[string index $str $pos] eq ","} {
                incr pos
            }
        }
        if {$pos < $len} { incr pos }  ;# skip ]
        return $result
    }

    # Emit a Tcl value as JSON
    proc json_emit {value {type "auto"}} {
        if {$type eq "dict" || ($type eq "auto" && [_looks_like_dict $value])} {
            return [_emit_dict $value]
        }
        if {$type eq "list" || ($type eq "auto" && [llength $value] > 1 && ![string is double $value])} {
            return [_emit_list $value]
        }
        if {$value eq ""} { return "null" }
        if {[string is integer -strict $value]} { return $value }
        if {[string is double -strict $value]} { return $value }
        if {$value eq "1" || $value eq "true"} { return "true" }
        if {$value eq "0" || $value eq "false"} { return "false" }
        return [_emit_string $value]
    }

    proc _emit_string {str} {
        set result "\""
        set len [string length $str]
        for {set i 0} {$i < $len} {incr i} {
            set ch [string index $str $i]
            switch -exact -- $ch {
                "\"" { append result "\\\"" }
                "\\" { append result "\\\\" }
                "\n" { append result "\\n" }
                "\r" { append result "\\r" }
                "\t" { append result "\\t" }
                default { append result $ch }
            }
        }
        append result "\""
        return $result
    }

    proc _emit_dict {d} {
        set pairs [list]
        foreach {k v} $d {
            lappend pairs "[_emit_string $k]: [json_emit $v]"
        }
        return "\{[join $pairs ", "]\}"
    }

    proc _emit_list {lst} {
        set items [list]
        foreach item $lst {
            lappend items [json_emit $item]
        }
        return "\[[join $items ", "]\]"
    }

    proc _looks_like_dict {value} {
        if {[llength $value] == 0} { return 0 }
        if {[llength $value] % 2 != 0} { return 0 }
        # Require ALL even-indexed elements to look like identifier keys
        # (start with alpha/underscore, contain no spaces).  This avoids
        # mis-encoding log entries like {"info" "some message"} as objects.
        foreach {k v} $value {
            if {![regexp {^[A-Za-z_][A-Za-z0-9_]*$} $k]} {
                return 0
            }
        }
        return 1
    }

    # Send a response to Python
    proc respond_ok {{result ""}} {
        set resp [list status ok result $result]
        puts [json_emit $resp dict]
        flush stdout
    }

    proc respond_error {message {error_info ""}} {
        set resp [list status error message $message errorInfo $error_info]
        puts [json_emit $resp dict]
        flush stdout
    }
}

# ── Main command loop ─────────────────────────────────────────────

proc main_loop {} {
    # Signal ready
    puts {{"status": "ready", "version": "0.1.0"}}
    flush stdout

    while {1} {
        if {[gets stdin line] < 0} {
            # EOF -- Python side closed
            break
        }

        set line [string trim $line]
        if {$line eq ""} continue

        if {[catch {::proto::json_parse $line} msg]} {
            ::proto::respond_error "JSON parse error: $msg"
            continue
        }
        set msg_dict $msg

        set cmd ""
        foreach {k v} $msg_dict {
            if {$k eq "cmd"} { set cmd $v }
        }

        if {[catch {dispatch_command $cmd $msg_dict} result]} {
            ::proto::respond_error $result $::errorInfo
        } else {
            ::proto::respond_ok $result
        }
    }
}

proc dispatch_command {cmd msg} {
    switch -exact -- $cmd {
        init {
            set tmos ""
            foreach {k v} $msg { if {$k eq "tmos_version"} { set tmos $v } }
            ::orch::init -tmos_version $tmos
            return "initialised"
        }

        load_irule {
            set source ""
            foreach {k v} $msg { if {$k eq "source"} { set source $v } }
            ::itest::clear_irule
            ::itest::load_irule $source
            return [::itest::registered_events]
        }

        set_state {
            set layer ""
            set values [list]
            foreach {k v} $msg {
                if {$k eq "layer"} { set layer $v }
                if {$k eq "values"} { set values $v }
            }
            _apply_state $layer $values
            return "ok"
        }

        fire_event {
            set event ""
            foreach {k v} $msg { if {$k eq "event"} { set event $v } }
            return [::itest::fire_event $event]
        }

        get_state {
            set layer ""
            foreach {k v} $msg { if {$k eq "layer"} { set layer $v } }
            return [_read_state $layer]
        }

        get_decisions {
            set category ""
            foreach {k v} $msg { if {$k eq "category"} { set category $v } }
            return [::itest::get_decisions $category]
        }

        get_logs {
            return [::state::log_capture::get]
        }

        reset {
            set scope "connection"
            foreach {k v} $msg { if {$k eq "scope"} { set scope $v } }
            switch -exact -- $scope {
                connection { ::state::reset_connection_state }
                request    { ::state::reset_request_state }
                all        { ::state::reset_all ; ::itest::reset_decisions }
            }
            return "ok"
        }

        add_pool {
            set name ""
            set members [list]
            foreach {k v} $msg {
                if {$k eq "name"} { set name $v }
                if {$k eq "members"} { set members $v }
            }
            ::state::lb::add_pool $name $members
            return "ok"
        }

        add_datagroup {
            set name ""
            set type "string"
            set records [list]
            foreach {k v} $msg {
                if {$k eq "name"} { set name $v }
                if {$k eq "type"} { set type $v }
                if {$k eq "records"} { set records $v }
            }
            ::state::datagroup::add $name $type $records
            return "ok"
        }

        eval {
            set script ""
            foreach {k v} $msg { if {$k eq "script"} { set script $v } }
            return [uplevel #0 $script]
        }

        quit {
            exit 0
        }

        default {
            error "unknown command: $cmd"
        }
    }
}

proc _apply_state {layer values} {
    switch -exact -- $layer {
        connection {
            foreach {k v} $values {
                set ::state::connection::$k $v
            }
        }
        tls_client {
            foreach {k v} $values {
                set ::state::tls::client::$k $v
            }
        }
        tls_server {
            foreach {k v} $values {
                set ::state::tls::server::$k $v
            }
        }
        http_request {
            foreach {k v} $values {
                if {$k eq "headers"} {
                    set ::state::http::request::headers $v
                } else {
                    set ::state::http::request::$k $v
                }
            }
        }
        http_response {
            foreach {k v} $values {
                if {$k eq "headers"} {
                    set ::state::http::response::headers $v
                } else {
                    set ::state::http::response::$k $v
                }
            }
        }
        lb {
            foreach {k v} $values {
                set ::state::lb::$k $v
            }
        }
    }
}

proc _read_state {layer} {
    switch -exact -- $layer {
        connection {
            return [list \
                client_addr $::state::connection::client_addr \
                client_port $::state::connection::client_port \
                local_addr  $::state::connection::local_addr \
                local_port  $::state::connection::local_port \
                server_addr $::state::connection::server_addr \
                server_port $::state::connection::server_port \
                state       $::state::connection::state \
            ]
        }
        http_request {
            return [list \
                method  $::state::http::request::method \
                uri     $::state::http::request::uri \
                path    $::state::http::request::path \
                query   $::state::http::request::query \
                host    $::state::http::request::host \
                version $::state::http::request::version \
                headers $::state::http::request::headers \
            ]
        }
        http_response {
            return [list \
                status  $::state::http::response::status \
                reason  $::state::http::response::reason \
                version $::state::http::response::version \
                headers $::state::http::response::headers \
            ]
        }
        lb {
            return [list \
                pool        $::state::lb::pool \
                pool_member $::state::lb::pool_member \
                node_addr   $::state::lb::node_addr \
                node_port   $::state::lb::node_port \
                snat_type   $::state::lb::snat_type \
                selected    $::state::lb::selected \
            ]
        }
        tls_client {
            return [list \
                sni            $::state::tls::client::sni \
                cipher_name    $::state::tls::client::cipher_name \
                cipher_version $::state::tls::client::cipher_version \
                handshake_done $::state::tls::client::handshake_done \
            ]
        }
        all {
            return [list \
                connection [_read_state connection] \
                http_request [_read_state http_request] \
                http_response [_read_state http_response] \
                lb [_read_state lb] \
                tls_client [_read_state tls_client] \
            ]
        }
        default {
            error "unknown state layer: $layer"
        }
    }
}

# Start the main loop
main_loop
