# command_mocks.tcl -- iRule command mock implementations
#
# Implements the iRule commands that the unknown handler dispatches to.
# These read from and write to the ::state:: namespace.
#
# Commands are organised by namespace:
#   IP::       - IP addressing
#   TCP::      - TCP connection
#   HTTP::     - HTTP request/response
#   SSL::      - TLS/SSL
#   LB::       - Load-balancing
#   pool       - Pool selection (top-level)
#   node       - Node selection (top-level)
#   snat       - SNAT (top-level)
#   snatpool   - SNAT pool selection (top-level)
#   class      - Data group operations (top-level)
#   table      - Session table operations (top-level)
#   persist    - Persistence (top-level)
#   event      - Event control (top-level)
#   log        - Logging (top-level)
#   after      - Delayed execution (top-level)
#   reject     - Connection reject (top-level)
#   drop       - Connection drop (top-level)
#   discard    - Connection discard (top-level)
#
# Copyright (c) 2024 tcl-lsp contributors.  MIT licence.

namespace eval ::itest {

    # ── Decision log ──────────────────────────────────────────────────
    #
    # Records every significant action the iRule takes so tests can
    # assert on behaviour rather than just state.

    variable decisions [list]

    proc log_decision {category action args} {
        variable decisions
        lappend decisions [list $category $action $args]
    }

    proc get_decisions {{category ""}} {
        variable decisions
        if {$category eq ""} {
            return $decisions
        }
        set filtered [list]
        foreach d $decisions {
            if {[lindex $d 0] eq $category} {
                lappend filtered $d
            }
        }
        return $filtered
    }

    proc reset_decisions {} {
        variable decisions [list]
    }

    # ── Current event context ─────────────────────────────────────────
    #
    # Set by the orchestrator before firing each event.

    variable current_event ""
    variable current_priority 500

    # ── unknown handler ───────────────────────────────────────────────
    #
    # This is the core dispatch mechanism.  When Tcl encounters a command
    # it doesn't know (like "HTTP::host" or "pool"), the unknown handler
    # routes it to our mock implementation.

    proc install_unknown {} {
        # Save the original unknown handler
        if {[llength [::tmm::_orig_info commands ::tmm::_orig_unknown]]} {
            # Already saved
        } elseif {[llength [::tmm::_orig_info commands ::unknown]]} {
            ::tmm::_orig_rename ::unknown ::tmm::_orig_unknown
        }

        proc ::unknown {cmd args} {
            # Try our command dispatch first
            set resolved [::itest::_resolve_command $cmd]
            if {$resolved ne ""} {
                # Fast path: when the profiler is off (the default), dispatch
                # exactly as before so error propagation is untouched.
                if {!$::itest::profiler::enabled} {
                    return [eval $resolved $args]
                }
                # Profiler on: record the command under its real iRule
                # spelling around execution, then re-raise preserving the
                # full error context (-errorinfo / -errorcode) so a failing
                # mock is no harder to debug than the direct dispatch.
                ::itest::profiler::emit RP_CMD_ENTRY $cmd
                set _rc [catch {eval $resolved $args} _res]
                ::itest::profiler::emit RP_CMD_EXIT $cmd
                if {$_rc == 1} {
                    return -code error -errorinfo $::errorInfo -errorcode $::errorCode $_res
                } elseif {$_rc != 0} {
                    return -code $_rc $_res
                }
                return $_res
            }

            # Fall through to original unknown if it exists
            if {[llength [::tmm::_orig_info commands ::tmm::_orig_unknown]]} {
                return [eval [list ::tmm::_orig_unknown $cmd] $args]
            }

            error "invalid command name \"$cmd\""
        }
    }

    # Command resolution table: maps iRule command names to our procs
    variable _command_map
    array set _command_map {}

    proc register_command {name proc_name} {
        variable _command_map
        set _command_map($name) $proc_name
    }

    proc _resolve_command {cmd} {
        variable _command_map
        if {[::tmm::_orig_info exists _command_map($cmd)]} {
            return $_command_map($cmd)
        }
        return ""
    }
}

# ══════════════════════════════════════════════════════════════════════
# IP:: namespace commands
# ══════════════════════════════════════════════════════════════════════

namespace eval ::itest::cmd {

    # ── Byte-array helpers for *::payload mocks ───────────────────────
    # TMM treats payloads as raw byte buffers: `length`, the `<size>`
    # getter, and `replace` offsets/lengths are all BYTE counts, and a
    # write-back is interpreted as a byte array.  These helpers force a
    # byte-array intrep so the mocks are byte-accurate (and binary-safe)
    # rather than character-based — matching real TMM and the S110
    # diagnostic's model.  `binary format a*` maps each character to its
    # low byte (latin-1), which is identity for an existing byte array
    # and the byte view for an ASCII string.

    proc _payload_bytes {s} {
        return [binary format a* $s]
    }

    proc _payload_bytelength {s} {
        return [string length [_payload_bytes $s]]
    }

    proc _payload_first {s n} {
        if {$n <= 0} {
            return [_payload_bytes ""]
        }
        return [string range [_payload_bytes $s] 0 [expr {$n - 1}]]
    }

    proc _payload_splice {s offset len data} {
        set b [_payload_bytes $s]
        set before [string range $b 0 [expr {$offset - 1}]]
        set after  [string range $b [expr {$offset + $len}] end]
        # Re-wrap as a byte array so the splice does not re-encode the
        # surrounding high bytes (the F5 KB K22406348 corruption).
        return [_payload_bytes "${before}[_payload_bytes $data]${after}"]
    }

    proc ip_client_addr {args} {
        return $::state::connection::client_addr
    }

    proc ip_client_port {args} {
        return $::state::connection::client_port
    }

    proc ip_server_addr {args} {
        if {$::state::lb::selected} {
            return $::state::lb::node_addr
        }
        return $::state::connection::local_addr
    }

    proc ip_server_port {args} {
        if {$::state::lb::selected} {
            return $::state::lb::node_port
        }
        return $::state::connection::local_port
    }

    proc ip_local_addr {args} {
        return $::state::connection::local_addr
    }

    proc ip_local_port {args} {
        return $::state::connection::local_port
    }

    proc ip_remote_addr {args} {
        return $::state::connection::remote_addr
    }

    proc ip_remote_port {args} {
        return $::state::connection::remote_port
    }

    proc ip_protocol {args} {
        return $::state::connection::protocol
    }

    proc ip_tos {args} {
        if {[llength $args] > 0} {
            set ::state::connection::tos [lindex $args 0]
            ::itest::log_decision ip tos_set [lindex $args 0]
            return
        }
        return $::state::connection::tos
    }

    proc ip_ttl {args} {
        if {[llength $args] > 0} {
            set ::state::connection::ttl [lindex $args 0]
            return
        }
        return $::state::connection::ttl
    }

    # ── TCP:: commands ────────────────────────────────────────────────

    proc tcp_client_port {args} {
        return $::state::connection::client_port
    }

    proc tcp_local_port {args} {
        return $::state::connection::local_port
    }

    proc tcp_server_port {args} {
        return $::state::connection::server_port
    }

    proc tcp_remote_port {args} {
        return $::state::connection::remote_port
    }

    proc tcp_mss {args} {
        return $::state::connection::mss
    }

    proc tcp_rtt {args} {
        return $::state::connection::rtt
    }

    proc tcp_bandwidth {args} {
        return $::state::connection::bandwidth
    }

    proc tcp_collect {args} {
        # Schedule data collection -- the orchestrator handles the
        # actual data delivery via CLIENT_DATA/SERVER_DATA events
        set length 0
        if {[llength $args] > 0} {
            set length [lindex $args 0]
        }
        ::itest::log_decision tcp collect $length
        return
    }

    proc tcp_release {args} {
        ::itest::log_decision tcp release
        return
    }

    proc tcp_payload {args} {
        # TCP::payload ?<size>?
        # TCP::payload replace <offset> <length> <data>
        # TCP::payload length

        if {[llength $args] == 0} {
            return $::state::connection::client_payload
        }

        set subcmd [lindex $args 0]

        switch -exact -- $subcmd {
            length {
                return [_payload_bytelength $::state::connection::client_payload]
            }
            replace {
                set offset [lindex $args 1]
                set len    [lindex $args 2]
                set data   [lindex $args 3]
                set ::state::connection::client_payload \
                    [_payload_splice $::state::connection::client_payload $offset $len $data]
                ::itest::log_decision tcp payload_replace [list $offset $len $data]
                return
            }
            default {
                # TCP::payload <size> -- return first N bytes (byte-accurate).
                if {[string is integer -strict $subcmd]} {
                    return [_payload_first $::state::connection::client_payload $subcmd]
                }
                return $::state::connection::client_payload
            }
        }
    }

    proc tcp_respond {args} {
        set data [lindex $args 0]
        ::itest::log_decision tcp respond $data
        return
    }

    proc tcp_close {args} {
        ::itest::log_decision tcp close
        set ::state::connection::state "closing"
        return
    }

    proc tcp_notify {args} {
        # TCP::notify request|response|eom
        set subcmd [lindex $args 0]
        ::itest::log_decision tcp notify $subcmd
        return
    }

    proc tcp_option {args} {
        # TCP::option get <kind>
        # TCP::option set <kind> <value> ?next|all?
        # TCP::option noset <kind>
        set subcmd [lindex $args 0]
        set rest [lrange $args 1 end]
        switch -exact -- $subcmd {
            get {
                set kind [lindex $rest 0]
                if {[info exists ::state::connection::tcp_options($kind)]} {
                    return $::state::connection::tcp_options($kind)
                }
                return ""
            }
            set {
                set kind  [lindex $rest 0]
                set value [lindex $rest 1]
                set ::state::connection::tcp_options($kind) $value
                ::itest::log_decision tcp option_set [list $kind $value [lrange $rest 2 end]]
                return 0
            }
            noset {
                set kind [lindex $rest 0]
                ::itest::log_decision tcp option_noset $kind
                return
            }
        }
    }

    # ── HTTP:: commands ───────────────────────────────────────────────

    proc http_method {args} {
        return $::state::http::request::method
    }

    proc http_uri {args} {
        if {[llength $args] > 0} {
            set ::state::http::request::uri [lindex $args 0]
            # Update path/query
            set uri [lindex $args 0]
            set qpos [string first "?" $uri]
            if {$qpos >= 0} {
                set ::state::http::request::path [string range $uri 0 [expr {$qpos - 1}]]
                set ::state::http::request::query [string range $uri [expr {$qpos + 1}] end]
            } else {
                set ::state::http::request::path $uri
                set ::state::http::request::query ""
            }
            ::itest::log_decision http uri_set $uri
            return
        }
        return $::state::http::request::uri
    }

    proc http_path {args} {
        if {[llength $args] > 0} {
            set ::state::http::request::path [lindex $args 0]
            ::itest::log_decision http path_set [lindex $args 0]
            return
        }
        return $::state::http::request::path
    }

    proc http_query {args} {
        # HTTP::query -- getter
        # HTTP::query <string> -- setter (v11.5+)
        # HTTP::query -normalized -- normalized getter
        if {[llength $args] > 0} {
            set arg [lindex $args 0]
            if {$arg eq "-normalized"} {
                # Simplified: just return the query as-is
                return $::state::http::request::query
            }
            # Setter
            set ::state::http::request::query $arg
            # Update the full URI
            set path $::state::http::request::path
            if {$arg ne ""} {
                set ::state::http::request::uri "${path}?${arg}"
            } else {
                set ::state::http::request::uri $path
            }
            ::itest::log_decision http query_set $arg
            return
        }
        return $::state::http::request::query
    }

    proc http_host {args} {
        if {[llength $args] > 0} {
            set ::state::http::request::host [lindex $args 0]
            ::itest::log_decision http host_set [lindex $args 0]
            return
        }
        return $::state::http::request::host
    }

    proc http_header {args} {
        # HTTP::header value <name>
        # HTTP::header values <name>
        # HTTP::header insert ?"lws"? <name> <value> ...
        # HTTP::header replace <name> ?<string>?
        # HTTP::header remove <name>
        # HTTP::header count ?<name>?
        # HTTP::header names
        # HTTP::header exists <name>
        # HTTP::header at <index>
        # HTTP::header lws
        # HTTP::header is_keepalive
        # HTTP::header is_redirect
        # HTTP::header sanitize <name>+
        # HTTP::header insert_modssl_fields <args>
        # Also: HTTP::header <name> (shorthand for value)

        set subcmd [lindex $args 0]
        set rest [lrange $args 1 end]

        # Determine if we're in request or response context
        set in_response [expr {$::itest::current_event eq "HTTP_RESPONSE" ||
                               $::itest::current_event eq "HTTP_RESPONSE_DATA" ||
                               $::itest::current_event eq "HTTP_RESPONSE_RELEASE"}]

        switch -exact -- $subcmd {
            value {
                set name [lindex $rest 0]
                if {$in_response} {
                    return [::state::http::response::header get $name]
                }
                return [::state::http::request::header get $name]
            }
            values {
                set name [lindex $rest 0]
                if {$in_response} {
                    return [::state::http::response::header values $name]
                }
                return [::state::http::request::header values $name]
            }
            insert {
                # Handle optional "lws" prefix
                set idx 0
                if {[lindex $rest 0] eq "lws"} {
                    incr idx
                }
                # Insert one or more name/value pairs
                while {$idx < [llength $rest]} {
                    set name [lindex $rest $idx]
                    set val [lindex $rest [expr {$idx + 1}]]
                    if {$in_response} {
                        ::state::http::response::header insert $name $val
                    } else {
                        ::state::http::request::header insert $name $val
                    }
                    ::itest::log_decision http header_insert [list $name $val]
                    incr idx 2
                }
                return
            }
            replace {
                set name [lindex $rest 0]
                set val ""
                if {[llength $rest] > 1} {
                    set val [lindex $rest 1]
                }
                if {$in_response} {
                    ::state::http::response::header set $name $val
                } else {
                    ::state::http::request::header set $name $val
                }
                ::itest::log_decision http header_replace [list $name $val]
                return
            }
            remove {
                set name [lindex $rest 0]
                if {$in_response} {
                    ::state::http::response::header remove $name
                } else {
                    ::state::http::request::header remove $name
                }
                ::itest::log_decision http header_remove $name
                return
            }
            count {
                if {[llength $rest] > 0} {
                    set name [lindex $rest 0]
                    if {$in_response} {
                        return [::state::http::response::header count $name]
                    }
                    return [::state::http::request::header count $name]
                }
                # No name: count all headers
                if {$in_response} {
                    return [llength [dict keys $::state::http::response::headers]]
                }
                return [llength [dict keys $::state::http::request::headers]]
            }
            names {
                if {$in_response} {
                    return [dict keys $::state::http::response::headers]
                }
                return [dict keys $::state::http::request::headers]
            }
            exists {
                set name [lindex $rest 0]
                if {$in_response} {
                    return [::state::http::response::header exists $name]
                }
                return [::state::http::request::header exists $name]
            }
            at {
                set idx [lindex $rest 0]
                if {$in_response} {
                    set keys [dict keys $::state::http::response::headers]
                } else {
                    set keys [dict keys $::state::http::request::headers]
                }
                if {$idx < [llength $keys]} {
                    return [lindex $keys $idx]
                }
                return ""
            }
            lws {
                # Returns 1 if linear whitespace was encountered
                return 0
            }
            is_keepalive {
                return $::state::http::request::is_keepalive
            }
            is_redirect {
                return $::state::http::response::is_redirect
            }
            sanitize {
                ::itest::log_decision http header_sanitize $rest
                return
            }
            insert_modssl_fields {
                ::itest::log_decision http header_insert_modssl $rest
                return
            }
            default {
                # Shorthand: HTTP::header <name> == HTTP::header value <name>
                if {$in_response} {
                    return [::state::http::response::header get $subcmd]
                }
                return [::state::http::request::header get $subcmd]
            }
        }
    }

    proc http_version {args} {
        return $::state::http::request::version
    }

    proc http_payload {args} {
        # HTTP::payload ?<size>?
        # HTTP::payload length
        # HTTP::payload replace <offset> <length> <data>
        set in_response [expr {$::itest::current_event eq "HTTP_RESPONSE_DATA"}]
        if {$in_response} {
            set var ::state::http::response::payload
        } else {
            set var ::state::http::request::payload
        }
        if {[llength $args] == 0} {
            return [set $var]
        }
        set subcmd [lindex $args 0]
        switch -exact -- $subcmd {
            length {
                return [_payload_bytelength [set $var]]
            }
            replace {
                set offset [lindex $args 1]
                set len    [lindex $args 2]
                set data   [lindex $args 3]
                set $var [_payload_splice [set $var] $offset $len $data]
                ::itest::log_decision http payload_replace [list $offset $len $data]
                return
            }
            default {
                # HTTP::payload <size> -- return the first N bytes.
                if {[string is integer -strict $subcmd]} {
                    return [_payload_first [set $var] $subcmd]
                }
                return [set $var]
            }
        }
    }

    proc http_collect {args} {
        set length 0
        if {[llength $args] > 0} {
            set length [lindex $args 0]
        }
        set in_response [expr {$::itest::current_event eq "HTTP_RESPONSE" ||
                               $::itest::current_event eq "HTTP_RESPONSE_DATA"}]
        if {$in_response} {
            set ::state::http::collect_response 1
            set ::state::http::collect_response_length $length
        } else {
            set ::state::http::collect_request 1
            set ::state::http::collect_request_length $length
        }
        ::itest::log_decision http collect $length
        return
    }

    proc http_release {args} {
        ::itest::log_decision http release
        return
    }

    proc http_respond {args} {
        # HTTP::respond <status> ?content <body>? ?<header-name> <header-value> ...?
        set status [lindex $args 0]
        set ::state::http::response_committed 1
        ::itest::log_decision http respond $args
        return
    }

    proc http_redirect {args} {
        set url [lindex $args 0]
        set ::state::http::request::redirect $url
        set ::state::http::response_committed 1
        ::itest::log_decision http redirect $url
        return
    }

    proc http_status {args} {
        return $::state::http::response::status
    }

    proc http_is_keepalive {args} {
        return $::state::http::request::is_keepalive
    }

    proc http_is_redirect {args} {
        return $::state::http::response::is_redirect
    }

    proc http_cookie {args} {
        # HTTP::cookie value <name> ?<string>?
        # HTTP::cookie names
        # HTTP::cookie count
        # HTTP::cookie insert name <name> value <value> ?path <path>? ...
        # HTTP::cookie remove <name>
        # HTTP::cookie exists <name>
        # HTTP::cookie version <name> ?value?
        # HTTP::cookie path <name> ?value?
        # HTTP::cookie domain <name> ?value?
        # HTTP::cookie secure <name> ?enable|disable?
        # HTTP::cookie httponly <name> ?enable|disable?
        # HTTP::cookie maxage <name> ?seconds?
        # HTTP::cookie expires <name> ?seconds? ?absolute|relative?
        # HTTP::cookie encrypt <name> <passphrase> ?128|192|256?
        # HTTP::cookie decrypt <name> <passphrase> ?128|192|256?
        # HTTP::cookie sanitize <name>+
        # HTTP::cookie attribute <name> ?sub-op? ...
        set subcmd [lindex $args 0]
        set rest [lrange $args 1 end]

        # Determine context
        set in_response [expr {$::itest::current_event eq "HTTP_RESPONSE" ||
                               $::itest::current_event eq "HTTP_RESPONSE_DATA" ||
                               $::itest::current_event eq "HTTP_RESPONSE_RELEASE"}]

        # Parse cookies from the Cookie/Set-Cookie header
        proc _parse_cookies {} {
            upvar in_response in_response
            if {$in_response} {
                set hdr [::state::http::response::header get "set-cookie"]
            } else {
                set hdr [::state::http::request::header get "cookie"]
            }
            set cookies [list]
            foreach pair [split $hdr ";"] {
                set pair [string trim $pair]
                set eqpos [string first "=" $pair]
                if {$eqpos >= 0} {
                    set cname [string range $pair 0 [expr {$eqpos - 1}]]
                    set cval [string range $pair [expr {$eqpos + 1}] end]
                    lappend cookies [string trim $cname] [string trim $cval]
                }
            }
            return $cookies
        }

        set cookies [_parse_cookies]

        switch -exact -- $subcmd {
            value {
                set name [lindex $rest 0]
                if {[llength $rest] > 1} {
                    # Setter: HTTP::cookie value <name> <string>
                    ::itest::log_decision http cookie_set [list $name [lindex $rest 1]]
                    return
                }
                foreach {k v} $cookies {
                    if {$k eq $name} { return $v }
                }
                return ""
            }
            names {
                set names [list]
                foreach {k v} $cookies {
                    lappend names $k
                }
                return $names
            }
            count {
                return [expr {[llength $cookies] / 2}]
            }
            exists {
                set name [lindex $rest 0]
                foreach {k v} $cookies {
                    if {$k eq $name} { return 1 }
                }
                return 0
            }
            insert {
                # HTTP::cookie insert name <n> value <v> ?path <p>? ...
                ::itest::log_decision http cookie_insert $rest
                return
            }
            remove {
                set name [lindex $rest 0]
                ::itest::log_decision http cookie_remove $name
                return
            }
            sanitize {
                ::itest::log_decision http cookie_sanitize $rest
                return
            }
            version - path - domain - comment - commenturl - ports - maxage {
                set name [lindex $rest 0]
                if {[llength $rest] > 1} {
                    ::itest::log_decision http cookie_${subcmd}_set [list $name [lindex $rest 1]]
                    return
                }
                return ""
            }
            expires {
                set name [lindex $rest 0]
                if {[llength $rest] > 1} {
                    ::itest::log_decision http cookie_expires_set $rest
                    return
                }
                return ""
            }
            secure - httponly {
                set name [lindex $rest 0]
                if {[llength $rest] > 1} {
                    ::itest::log_decision http cookie_${subcmd}_set [list $name [lindex $rest 1]]
                    return
                }
                return "disable"
            }
            encrypt - decrypt {
                ::itest::log_decision http cookie_${subcmd} $rest
                return ""
            }
            attribute {
                # HTTP::cookie attribute <name> ?insert <attr> ?value? | exists <attr> | ...?
                set name [lindex $rest 0]
                if {[llength $rest] > 1} {
                    set attrcmd [lindex $rest 1]
                    ::itest::log_decision http cookie_attribute [list $name $attrcmd [lrange $rest 2 end]]
                    switch -exact -- $attrcmd {
                        exists { return 0 }
                        names  { return {} }
                        count  { return 0 }
                        value  { return "" }
                        default { return "" }
                    }
                }
                return ""
            }
            replace {
                ::itest::log_decision http cookie_replace $rest
                return
            }
            default {
                # Shorthand: HTTP::cookie <name>
                foreach {k v} $cookies {
                    if {$k eq $subcmd} { return $v }
                }
                return ""
            }
        }
    }

    # ── HTTP2:: commands ────────────────────────────────────────────────

    proc http2_header {args} {
        # HTTP2::header <name> -- get pseudo-header value
        # HTTP2::header replace <name> ?<string>?
        # HTTP2::header remove <name>
        set subcmd [lindex $args 0]
        set rest [lrange $args 1 end]
        switch -exact -- $subcmd {
            replace {
                set name [lindex $rest 0]
                set val ""
                if {[llength $rest] > 1} {
                    set val [lindex $rest 1]
                }
                if {[info exists ::state::http2::pseudo_headers]} {
                    dict set ::state::http2::pseudo_headers $name $val
                }
                ::itest::log_decision http2 header_replace [list $name $val]
                return
            }
            remove {
                set name [lindex $rest 0]
                if {[info exists ::state::http2::pseudo_headers] &&
                    [dict exists $::state::http2::pseudo_headers $name]} {
                    dict unset ::state::http2::pseudo_headers $name
                }
                ::itest::log_decision http2 header_remove $name
                return
            }
            default {
                # HTTP2::header <name> -- getter
                if {[info exists ::state::http2::pseudo_headers] &&
                    [dict exists $::state::http2::pseudo_headers $subcmd]} {
                    return [dict get $::state::http2::pseudo_headers $subcmd]
                }
                return ""
            }
        }
    }

    proc http2_stream {args} {
        # HTTP2::stream -- return stream id
        # HTTP2::stream id -- return stream id
        # HTTP2::stream priority ?<priority>?
        if {[llength $args] == 0} {
            if {[info exists ::state::http2::stream_id]} {
                return $::state::http2::stream_id
            }
            return 0
        }
        set subcmd [lindex $args 0]
        switch -exact -- $subcmd {
            id {
                if {[info exists ::state::http2::stream_id]} {
                    return $::state::http2::stream_id
                }
                return 0
            }
            priority {
                if {[llength $args] > 1} {
                    set ::state::http2::stream_priority [lindex $args 1]
                    ::itest::log_decision http2 stream_priority_set [lindex $args 1]
                    return 0
                }
                if {[info exists ::state::http2::stream_priority]} {
                    return $::state::http2::stream_priority
                }
                return 0
            }
        }
        return 0
    }

    # ── SSL:: commands ────────────────────────────────────────────────

    proc ssl_cipher {args} {
        set subcmd [lindex $args 0]
        switch -exact -- $subcmd {
            name    { return $::state::tls::client::cipher_name }
            bits    { return $::state::tls::client::cipher_bits }
            version { return $::state::tls::client::cipher_version }
            default { return $::state::tls::client::cipher_name }
        }
    }

    proc ssl_cert {args} {
        # SSL::cert <index> -- returns cert data
        return ""
    }

    proc ssl_extensions {args} {
        if {[lindex $args 0] eq "count"} {
            return [llength $::state::tls::client::extensions]
        }
        return $::state::tls::client::extensions
    }

    proc ssl_sni {args} {
        # Alias: SSL::extensions -type server_name
        return $::state::tls::client::sni
    }

    proc ssl_sessionid {args} {
        return $::state::tls::client::session_id
    }

    # ── DNS:: commands ────────────────────────────────────────────

    proc dns_answer {args} {
        # DNS::answer insert/replace/clear/count
        set subcmd [lindex $args 0]
        set rest [lrange $args 1 end]

        switch -exact -- $subcmd {
            insert {
                # DNS::answer insert -type <type> -name <name> -rdata <rdata> ?-ttl <ttl>?
                set entry [list]
                set i 0
                while {$i < [llength $rest]} {
                    lappend entry [lindex $rest $i] [lindex $rest [expr {$i+1}]]
                    incr i 2
                }
                lappend ::state::dns::answers $entry
                ::itest::log_decision dns answer_insert $rest
            }
            replace {
                set ::state::dns::answers [list]
                set entry [list]
                set i 0
                while {$i < [llength $rest]} {
                    lappend entry [lindex $rest $i] [lindex $rest [expr {$i+1}]]
                    incr i 2
                }
                lappend ::state::dns::answers $entry
                ::itest::log_decision dns answer_replace $rest
            }
            clear {
                set ::state::dns::answers [list]
                ::itest::log_decision dns answer_clear
            }
            count {
                return [llength $::state::dns::answers]
            }
            default {
                # DNS::answer <index> -- return answer record
                if {[string is integer -strict $subcmd]} {
                    return [lindex $::state::dns::answers $subcmd]
                }
                return ""
            }
        }
    }

    proc dns_header {args} {
        # DNS::header <field> ?value?
        set field [lindex $args 0]
        set rest [lrange $args 1 end]

        switch -exact -- $field {
            opcode {
                if {[llength $rest] > 0} {
                    set ::state::dns::opcode [lindex $rest 0]
                    ::itest::log_decision dns header_opcode [lindex $rest 0]
                    return
                }
                return $::state::dns::opcode
            }
            rcode {
                if {[llength $rest] > 0} {
                    set ::state::dns::rcode [lindex $rest 0]
                    ::itest::log_decision dns header_rcode [lindex $rest 0]
                    return
                }
                return $::state::dns::rcode
            }
            id {
                if {[llength $rest] > 0} {
                    set ::state::dns::id [lindex $rest 0]
                    return
                }
                return $::state::dns::id
            }
            aa {
                if {[llength $rest] > 0} {
                    set ::state::dns::aa [lindex $rest 0]
                    return
                }
                return $::state::dns::aa
            }
            tc {
                if {[llength $rest] > 0} {
                    set ::state::dns::tc [lindex $rest 0]
                    return
                }
                return $::state::dns::tc
            }
            rd {
                if {[llength $rest] > 0} {
                    set ::state::dns::rd [lindex $rest 0]
                    return
                }
                return $::state::dns::rd
            }
            ra {
                if {[llength $rest] > 0} {
                    set ::state::dns::ra [lindex $rest 0]
                    return
                }
                return $::state::dns::ra
            }
            qname {
                return $::state::dns::qname
            }
            qtype {
                return $::state::dns::qtype
            }
            qclass {
                return $::state::dns::qclass
            }
            ancount {
                return [llength $::state::dns::answers]
            }
            default {
                return ""
            }
        }
    }

    proc dns_return {args} {
        # DNS::return -- sends the DNS response and stops processing
        set ::state::dns::response_sent 1
        ::itest::log_decision dns return
        return
    }

    # ── HTTP:: additional commands ────────────────────────────────

    proc http_close {args} {
        # HTTP::close -- closes the HTTP connection
        set ::state::connection::state "closing"
        set ::state::http::response_committed 1
        ::itest::log_decision http close
        return
    }

    proc http_retry {args} {
        # HTTP::retry <uri> -- retries the request to a different URI
        set uri ""
        if {[llength $args] > 0} {
            set uri [lindex $args 0]
        }
        ::itest::log_decision http retry $uri
        return
    }

    proc http_request {args} {
        # HTTP::request -- returns the full HTTP request data
        # Reconstruct a minimal request line from state
        set method $::state::http::request::method
        set uri $::state::http::request::uri
        set version $::state::http::request::version
        return "$method $uri HTTP/$version\r\n"
    }

    proc http_request_num {args} {
        # HTTP::request_num -- returns the request number on this connection
        # In the test framework, default to 1
        return 1
    }

    proc http_disable {args} {
        # HTTP::disable -- disables HTTP processing
        ::itest::log_decision http disable
        return
    }

    proc http_enable {args} {
        # HTTP::enable -- enables HTTP processing
        ::itest::log_decision http enable
        return
    }

    proc http_fallback {args} {
        # HTTP::fallback <uri> -- sets fallback action
        set uri [lindex $args 0]
        ::itest::log_decision http fallback $uri
        return
    }

    proc http_has_responded {args} {
        # HTTP::has_responded -- returns true if a response has been committed
        return $::state::http::response_committed
    }

    proc http_hsts {args} {
        # HTTP::hsts -- get current HSTS header value
        # HTTP::hsts mode <enable|disable>
        # HTTP::hsts maximum-age <seconds>
        # HTTP::hsts include-subdomains <enable|disable>
        # HTTP::hsts preload <enable|disable>
        if {[llength $args] == 0} {
            # Getter: return current HSTS value
            if {![info exists ::state::http::hsts_mode] ||
                $::state::http::hsts_mode ne "enable"} {
                return ""
            }
            set result "max-age="
            if {[info exists ::state::http::hsts_maxage]} {
                append result $::state::http::hsts_maxage
            } else {
                append result "0"
            }
            if {[info exists ::state::http::hsts_includesub] &&
                $::state::http::hsts_includesub eq "enable"} {
                append result "; includeSubDomains"
            }
            if {[info exists ::state::http::hsts_preload] &&
                $::state::http::hsts_preload eq "enable"} {
                append result "; preload"
            }
            return $result
        }
        set subcmd [lindex $args 0]
        set val [lindex $args 1]
        switch -exact -- $subcmd {
            mode             { set ::state::http::hsts_mode $val }
            maximum-age      { set ::state::http::hsts_maxage $val }
            include-subdomains { set ::state::http::hsts_includesub $val }
            preload          { set ::state::http::hsts_preload $val }
        }
        ::itest::log_decision http hsts_${subcmd} $val
        return
    }

    proc http_class {args} {
        # HTTP::class -- returns selected class name
        # HTTP::class <enable|disable>
        # HTTP::class <asm|wa>
        # HTTP::class select <name>
        if {[llength $args] == 0} {
            if {[info exists ::state::http::class_name]} {
                return $::state::http::class_name
            }
            return ""
        }
        set subcmd [lindex $args 0]
        switch -exact -- $subcmd {
            select {
                set name [lindex $args 1]
                set ::state::http::class_name $name
                ::itest::log_decision http class_select $name
            }
            enable {
                set ::state::http::class_enabled 1
                ::itest::log_decision http class_enable
            }
            disable {
                set ::state::http::class_enabled 0
                ::itest::log_decision http class_disable
            }
            asm - wa {
                # Return whether selected class has ASM/WA enabled
                return 0
            }
            default {
                return ""
            }
        }
        return
    }

    # ── SSL:: additional commands ─────────────────────────────────

    proc ssl_disable {args} {
        # SSL::disable ?side?
        set side "clientside"
        if {[llength $args] > 0} {
            set side [lindex $args 0]
        }
        ::itest::log_decision ssl disable $side
        return
    }

    proc ssl_enable {args} {
        # SSL::enable ?side?
        set side "clientside"
        if {[llength $args] > 0} {
            set side [lindex $args 0]
        }
        ::itest::log_decision ssl enable $side
        return
    }

    proc ssl_respond {args} {
        # SSL::respond -- sends data during SSL handshake
        set data [lindex $args 0]
        ::itest::log_decision ssl respond $data
        return
    }

    # ── virtual command ───────────────────────────────────────────

    proc cmd_virtual {args} {
        if {[llength $args] == 0} {
            # Return current virtual server name
            return $::state::connection::vip_addr
        }
        # virtual <name> -- redirect to a different virtual server
        set vs_name [lindex $args 0]
        ::itest::log_decision lb virtual $vs_name
        return
    }

    # ── Top-level commands ────────────────────────────────────────────

    proc cmd_pool {args} {
        if {[llength $args] == 0} {
            # Return current pool
            return $::state::lb::pool
        }
        set pool_name [lindex $args 0]

        # Check if pool exists in configured state
        if {[::tmm::_orig_info exists ::state::lb::pools($pool_name)]} {
            set ::state::lb::pool $pool_name
            set ::state::lb::selected 1
            set pool_info $::state::lb::pools($pool_name)
            # Select first available member (simplified round-robin)
            set members [lindex $pool_info 1]
            if {[llength $members] > 0} {
                set member [lindex $members 0]
                set ::state::lb::pool_member $member
                # Parse addr:port
                set colonpos [string last ":" $member]
                if {$colonpos >= 0} {
                    set ::state::lb::node_addr [string range $member 0 [expr {$colonpos - 1}]]
                    set ::state::lb::node_port [string range $member [expr {$colonpos + 1}] end]
                }
            }
            ::itest::log_decision lb pool_select $pool_name
        } else {
            # Pool not configured -- still set it (iRule may not care)
            set ::state::lb::pool $pool_name
            set ::state::lb::selected 1
            ::itest::log_decision lb pool_select $pool_name
        }
        return
    }

    proc cmd_node {args} {
        set addr [lindex $args 0]
        set port 0
        if {[llength $args] > 1} {
            set port [lindex $args 1]
        }
        set ::state::lb::node_addr $addr
        set ::state::lb::node_port $port
        set ::state::lb::selected 1
        ::itest::log_decision lb node_select [list $addr $port]
        return
    }

    proc cmd_snat {args} {
        set type [lindex $args 0]
        switch -exact -- $type {
            automap {
                set ::state::lb::snat_type "automap"
                set ::state::lb::snat_addr $::state::connection::local_addr
            }
            none {
                set ::state::lb::snat_type "none"
                set ::state::lb::snat_addr ""
                set ::state::lb::snat_port 0
            }
            default {
                # Explicit SNAT address
                set ::state::lb::snat_type "explicit"
                set ::state::lb::snat_addr $type
                if {[llength $args] > 1} {
                    set ::state::lb::snat_port [lindex $args 1]
                }
            }
        }
        ::itest::log_decision lb snat $args
        return
    }

    proc cmd_snatpool {args} {
        set ::state::lb::snat_type "snatpool"
        set pool_name [lindex $args 0]
        ::itest::log_decision lb snatpool $pool_name
        return
    }

    proc cmd_reject {} {
        set ::state::connection::state "closing"
        ::itest::log_decision connection reject
        return
    }

    proc cmd_drop {} {
        set ::state::connection::state "closing"
        ::itest::log_decision connection drop
        return
    }

    proc cmd_discard {} {
        ::itest::log_decision connection discard
        return
    }

    # ── class command (data groups) ───────────────────────────────────

    proc cmd_class {args} {
        set subcmd [lindex $args 0]
        set rest [lrange $args 1 end]

        switch -exact -- $subcmd {
            match {
                # class match ?options? <value> <operator> <datagroup> ?options?
                #
                # Leading options (`-nocase`, `-element`, `-all`, …) and an
                # explicit `--` terminator may precede the positional
                # <value> <operator> <datagroup> triple. Parse them off the
                # front rather than assuming the value is at index 0, so
                # `class match -- [HTTP::host] equals hosts` no longer treats
                # `equals` as the datagroup name.
                set flags {}
                set idx 0
                set n [llength $rest]
                while {$idx < $n} {
                    set tok [lindex $rest $idx]
                    if {$tok eq "--"} { incr idx; break }
                    if {[string index $tok 0] eq "-"} {
                        lappend flags $tok
                        incr idx
                    } else {
                        break
                    }
                }
                set value [lindex $rest $idx]
                set operator [lindex $rest [expr {$idx + 1}]]
                set dg_name [lindex $rest [expr {$idx + 2}]]
                set options [concat $flags [lrange $rest [expr {$idx + 3}] end]]
                # Thread the comparison operator through to the matcher so
                # starts_with / contains / ends_with are honoured rather than
                # collapsing to exact equality.
                return [::state::datagroup::match $dg_name $value $operator {*}$options]
            }
            lookup {
                set dg_name [lindex $rest 0]
                set value [lindex $rest 1]
                return [::state::datagroup::lookup $dg_name $value]
            }
            names {
                set dg_name [lindex $rest 0]
                return [::state::datagroup::names $dg_name]
            }
            size {
                set dg_name [lindex $rest 0]
                return [::state::datagroup::size $dg_name]
            }
            element {
                # class element -name <index> <dg_name>
                # class element -value <index> <dg_name>
                set flag [lindex $rest 0]
                set idx [lindex $rest 1]
                set dg_name [lindex $rest 2]
                set names [::state::datagroup::names $dg_name]
                set key [lindex $names $idx]
                if {$flag eq "-name"} {
                    return $key
                } else {
                    return [::state::datagroup::lookup $dg_name $key]
                }
            }
            default {
                error "unknown class subcommand \"$subcmd\""
            }
        }
    }

    # ── table command (session tables) ────────────────────────────────

    proc cmd_table {args} {
        set subcmd [lindex $args 0]
        set rest [lrange $args 1 end]

        # Parse -subtable option
        set subtable ""
        set filtered_rest [list]
        set i 0
        while {$i < [llength $rest]} {
            set a [lindex $rest $i]
            if {$a eq "-subtable"} {
                incr i
                set subtable [lindex $rest $i]
            } else {
                lappend filtered_rest $a
            }
            incr i
        }
        set rest $filtered_rest

        switch -exact -- $subcmd {
            set {
                set key [lindex $rest 0]
                set value [lindex $rest 1]
                set timeout 0
                set lifetime 0
                # Parse -notouch, -indef, timeout, lifetime
                if {[llength $rest] > 2} { set timeout [lindex $rest 2] }
                if {[llength $rest] > 3} { set lifetime [lindex $rest 3] }
                ::state::table::set_entry $subtable $key $value $timeout $lifetime
                ::itest::log_decision table set [list $subtable $key $value]
                return $value
            }
            lookup {
                set key [lindex $rest 0]
                return [::state::table::lookup $subtable $key]
            }
            delete {
                set key [lindex $rest 0]
                ::state::table::delete_entry $subtable $key
                ::itest::log_decision table delete [list $subtable $key]
                return
            }
            incr {
                set key [lindex $rest 0]
                set amount 1
                if {[llength $rest] > 1} { set amount [lindex $rest 1] }
                set result [::state::table::incr_entry $subtable $key $amount]
                ::itest::log_decision table incr [list $subtable $key $amount]
                return $result
            }
            keys {
                if {[llength $rest] > 0} {
                    return [::state::table::keys $subtable [lindex $rest 0]]
                }
                return [::state::table::keys $subtable]
            }
            default {
                error "unknown table subcommand \"$subcmd\""
            }
        }
    }

    # ── persist command ───────────────────────────────────────────────

    proc cmd_persist {args} {
        set subcmd [lindex $args 0]
        set rest [lrange $args 1 end]

        switch -exact -- $subcmd {
            uie {
                set ::state::persist::uie [lindex $rest 0]
                set ::state::persist::mode "universal"
                ::itest::log_decision persist uie [lindex $rest 0]
            }
            cookie {
                set ::state::persist::mode "cookie"
                if {[llength $rest] > 0} {
                    set ::state::persist::cookie_name [lindex $rest 0]
                }
            }
            source_addr {
                set ::state::persist::mode "source_addr"
            }
            ssl {
                set ::state::persist::mode "ssl"
            }
            none {
                set ::state::persist::mode "none"
            }
            default {
                set ::state::persist::mode $subcmd
            }
        }
        ::itest::log_decision persist mode $subcmd
        return
    }

    # ── event command ─────────────────────────────────────────────────

    proc cmd_event {args} {
        set subcmd [lindex $args 0]
        set rest [lrange $args 1 end]

        switch -exact -- $subcmd {
            disable {
                if {[llength $rest] > 0} {
                    ::state::event_ctl::disable [lindex $rest 0]
                } else {
                    # Disable current event for other iRules
                    ::state::event_ctl::disable $::itest::current_event
                }
                ::itest::log_decision event disable [lindex $rest 0]
            }
            enable {
                if {[llength $rest] > 0} {
                    ::state::event_ctl::enable [lindex $rest 0]
                }
                ::itest::log_decision event enable [lindex $rest 0]
            }
            count {
                return 0
            }
            default {
                error "unknown event subcommand \"$subcmd\""
            }
        }
        return
    }

    # ── log command ───────────────────────────────────────────────────

    proc cmd_log {args} {
        # log <facility>.<level> <message>
        # log local0. "message"
        if {[llength $args] == 1} {
            # log "message" -- default facility
            ::state::log_capture::add "local0" "notice" [lindex $args 0]
        } elseif {[llength $args] >= 2} {
            set facspec [lindex $args 0]
            set msg [lindex $args 1]
            # Parse facility.level
            set dotpos [string first "." $facspec]
            if {$dotpos >= 0} {
                set facility [string range $facspec 0 [expr {$dotpos - 1}]]
                set level [string range $facspec [expr {$dotpos + 1}] end]
            } else {
                set facility $facspec
                set level "notice"
            }
            ::state::log_capture::add $facility $level $msg
        }
        return
    }

    # ── after command (iRules version) ────────────────────────────────

    proc cmd_after {args} {
        set delay [lindex $args 0]
        set subcmd ""
        if {[llength $args] > 1} {
            set subcmd [lindex $args 1]
        }

        switch -exact -- $delay {
            cancel {
                ::state::event_ctl::cancel_after $subcmd
                return
            }
            info {
                return ""
            }
            default {
                if {[llength $args] > 1} {
                    set script [lindex $args 1]
                    set id [::state::event_ctl::schedule_after $delay $script]
                    ::itest::log_decision after schedule [list $delay $script]
                    return $id
                }
                # Bare delay -- in iRules this suspends the event
                ::itest::log_decision after delay $delay
                return
            }
        }
    }

    # ── LB:: commands ─────────────────────────────────────────────────

    proc lb_select {args} {
        # LB::select pool <name> ?member <addr:port>?
        set pool_name ""
        set member ""
        set i 0
        while {$i < [llength $args]} {
            set a [lindex $args $i]
            switch -exact -- $a {
                pool {
                    incr i
                    set pool_name [lindex $args $i]
                }
                member {
                    incr i
                    set member [lindex $args $i]
                }
            }
            incr i
        }
        if {$pool_name ne ""} {
            cmd_pool $pool_name
        }
        if {$member ne ""} {
            set ::state::lb::pool_member $member
        }
        return
    }

    proc lb_server {args} {
        if {[llength $args] == 0} {
            return $::state::lb::pool_member
        }
        # LB::server pool <name> addr <addr> port <port>
        return $::state::lb::pool_member
    }

    proc lb_status {args} {
        # LB::status pool <name> ?member <addr:port>?
        return "up"
    }

    proc lb_detach {args} {
        set ::state::lb::detached 1
        ::itest::log_decision lb detach
        return
    }

    proc lb_reselect {args} {
        ::itest::log_decision lb reselect $args
        return
    }

    # ══════════════════════════════════════════════════════════════════
    # Command registration
    # ══════════════════════════════════════════════════════════════════

    # Derive mock proc name from iRule command name.
    # Convention: NS::sub -> [tolower ns]_sub ; toplevel -> cmd_toplevel
    # Hyphens and dots in names are converted to underscores.
    proc _mock_proc_name {irule_cmd} {
        if {[string first "::" $irule_cmd] >= 0} {
            set parts [split $irule_cmd "::"]
            # split on :: gives {NS {} sub}
            set ns [string tolower [lindex $parts 0]]
            set sub [lindex $parts end]
            set ns [string map {- _ . _} $ns]
            set sub [string map {- _ . _} $sub]
            return "::itest::cmd::${ns}_${sub}"
        } else {
            set safe [string map {- _ . _} $irule_cmd]
            return "::itest::cmd::cmd_${safe}"
        }
    }

    proc register_all {} {
        variable _gen_namespaced_commands
        variable _gen_toplevel_commands

        # Auto-register all iRule commands for which a mock exists.
        # Command list comes from generated registry data (_registry_data.tcl).
        # Hand-written mocks (procs in this file) take precedence; registry
        # commands without one fall back to the generic `_stub` driven by the
        # generated `_stub_actions` table (_mock_stubs.tcl, sourced during
        # framework load) — formerly ~1500 individual generated stub procs.
        #
        # The table is probed by its fully-qualified name rather than via a
        # `variable` link: `info exists ::itest::cmd::_stub_actions(...)`
        # safely returns 0 if `_mock_stubs.tcl` was never sourced (the runner
        # and example scripts guard it with `[file exists]`), degrading to the
        # pre-table behaviour of simply leaving stub-only commands unregistered.
        foreach irule_cmd [concat $_gen_namespaced_commands $_gen_toplevel_commands] {
            set mock [_mock_proc_name $irule_cmd]
            # Use _orig_info to bypass the TMM shim's info filter
            # (which hides ::itest::* from info commands).
            if {[llength [::tmm::_orig_info commands $mock]]} {
                ::itest::register_command $irule_cmd $mock
            } else {
                set tail [namespace tail $mock]
                if {[info exists ::itest::cmd::_stub_actions($tail)]} {
                    ::itest::register_command $irule_cmd \
                        [list ::itest::cmd::_stub {*}$::itest::cmd::_stub_actions($tail)]
                }
            }
        }
    }
}
