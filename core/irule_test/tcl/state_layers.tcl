# state_layers.tcl -- Protocol state machines for iRule test framework
#
# Each layer maintains the state that iRule commands can read and modify.
# The orchestrator sets state before firing events; iRule commands read
# from and write to these state dictionaries.
#
# Layers:
#   ::state::connection   - L4 TCP/UDP connection properties
#   ::state::tls          - TLS handshake state (client + server side)
#   ::state::http         - HTTP request/response state
#   ::state::lb           - Load-balancing decisions
#   ::state::persist      - Persistence state
#   ::state::stats        - Counters and statistics
#
# The orchestrator resets per-request state between HTTP transactions
# while preserving connection-scoped state.

namespace eval ::state {

    # ── Connection layer (L4) ─────────────────────────────────────────
    #
    # Stable for the lifetime of the TCP connection.
    # Set by the orchestrator before CLIENT_ACCEPTED.

    namespace eval connection {
        variable client_addr    "10.0.0.1"
        variable client_port    12345
        variable server_addr    "0.0.0.0"
        variable server_port    0
        variable local_addr     "192.168.1.100"
        variable local_port     443
        variable remote_addr    ""
        variable remote_port    0
        variable vip_addr       "192.168.1.100"
        variable vip_port       443
        variable protocol       6   ;# IPPROTO_TCP
        variable transport      "tcp"
        variable mss            1460
        variable ttl            64
        variable tos            0
        variable bandwidth      0
        variable rtt            0
        variable idle_timeout   300
        variable client_payload ""
        variable server_payload ""

        # Connection state machine
        variable state          "new"   ;# new, established, closing, closed

        proc reset {} {
            variable client_addr    "10.0.0.1"
            variable client_port    12345
            variable server_addr    "0.0.0.0"
            variable server_port    0
            variable local_addr     "192.168.1.100"
            variable local_port     443
            variable remote_addr    ""
            variable remote_port    0
            variable vip_addr       "192.168.1.100"
            variable vip_port       443
            variable protocol       6
            variable transport      "tcp"
            variable mss            1460
            variable ttl            64
            variable tos            0
            variable bandwidth      0
            variable rtt            0
            variable idle_timeout   300
            variable client_payload ""
            variable server_payload ""
            variable state          "new"
        }

        proc configure {args} {
            foreach {key val} $args {
                set varname [string trimleft $key -]
                variable $varname
                set $varname $val
            }
        }
    }

    # ── TLS layer ─────────────────────────────────────────────────────
    #
    # Populated during TLS handshake events.
    # Client-side TLS state (CLIENTSSL_* events):

    namespace eval tls {

        namespace eval client {
            variable sni            ""
            variable cipher_name    "ECDHE-RSA-AES128-GCM-SHA256"
            variable cipher_bits    128
            variable cipher_version "TLSv1.2"
            variable cert_subject   ""
            variable cert_issuer    ""
            variable cert_serial    ""
            variable cert_hash      ""
            variable cert_count     0
            variable extensions     {}
            variable alpn           ""
            variable handshake_done 0
            variable session_id     ""

            proc reset {} {
                variable sni            ""
                variable cipher_name    "ECDHE-RSA-AES128-GCM-SHA256"
                variable cipher_bits    128
                variable cipher_version "TLSv1.2"
                variable cert_subject   ""
                variable cert_issuer    ""
                variable cert_serial    ""
                variable cert_hash      ""
                variable cert_count     0
                variable extensions     {}
                variable alpn           ""
                variable handshake_done 0
                variable session_id     ""
            }

            proc configure {args} {
                foreach {key val} $args {
                    set varname [string trimleft $key -]
                    variable $varname
                    set $varname $val
                }
            }
        }

        namespace eval server {
            variable sni            ""
            variable cipher_name    "ECDHE-RSA-AES128-GCM-SHA256"
            variable cipher_bits    128
            variable cipher_version "TLSv1.2"
            variable cert_subject   ""
            variable cert_issuer    ""
            variable cert_serial    ""
            variable cert_hash      ""
            variable cert_count     0
            variable extensions     {}
            variable alpn           ""
            variable handshake_done 0
            variable session_id     ""

            proc reset {} {
                variable sni            ""
                variable cipher_name    "ECDHE-RSA-AES128-GCM-SHA256"
                variable cipher_bits    128
                variable cipher_version "TLSv1.2"
                variable cert_subject   ""
                variable cert_issuer    ""
                variable cert_serial    ""
                variable cert_hash      ""
                variable cert_count     0
                variable extensions     {}
                variable alpn           ""
                variable handshake_done 0
                variable session_id     ""
            }

            proc configure {args} {
                foreach {key val} $args {
                    set varname [string trimleft $key -]
                    variable $varname
                    set $varname $val
                }
            }
        }

        proc reset {} {
            client::reset
            server::reset
        }
    }

    # ── HTTP layer ────────────────────────────────────────────────────
    #
    # Per-request state: reset between HTTP transactions on keep-alive.

    namespace eval http {

        namespace eval request {
            variable method     "GET"
            variable uri        "/"
            variable path       "/"
            variable query      ""
            variable version    "1.1"
            variable host       ""
            variable headers    {}  ;# dict: header-name -> value-list
            variable payload    ""
            variable payload_length 0
            variable content_type ""
            variable is_keepalive 1
            variable truncated  0
            variable redirect   ""  ;# set by HTTP::redirect

            proc reset {} {
                variable method     "GET"
                variable uri        "/"
                variable path       "/"
                variable query      ""
                variable version    "1.1"
                variable host       ""
                variable headers    {}
                variable payload    ""
                variable payload_length 0
                variable content_type ""
                variable is_keepalive 1
                variable truncated  0
                variable redirect   ""
            }

            proc configure {args} {
                foreach {key val} $args {
                    set varname [string trimleft $key -]
                    variable $varname
                    set $varname $val
                }
                # Derive path and query from uri if not explicitly set
                variable uri
                variable path
                variable query
                if {"-uri" in $args && "-path" ni $args} {
                    set qpos [string first "?" $uri]
                    if {$qpos >= 0} {
                        set path [string range $uri 0 $qpos-1]
                        set query [string range $uri $qpos+1 end]
                    } else {
                        set path $uri
                        set query ""
                    }
                }
            }

            # Get/set a header value (case-insensitive)
            proc header {op name args} {
                variable headers
                set lname [string tolower $name]
                switch -exact -- $op {
                    get {
                        if {[dict exists $headers $lname]} {
                            return [lindex [dict get $headers $lname] 0]
                        }
                        return ""
                    }
                    values {
                        if {[dict exists $headers $lname]} {
                            return [dict get $headers $lname]
                        }
                        return {}
                    }
                    set {
                        dict set headers $lname [list [lindex $args 0]]
                    }
                    insert {
                        if {[dict exists $headers $lname]} {
                            set existing [dict get $headers $lname]
                            lappend existing [lindex $args 0]
                            dict set headers $lname $existing
                        } else {
                            dict set headers $lname [list [lindex $args 0]]
                        }
                    }
                    remove {
                        dict unset headers $lname
                    }
                    count {
                        if {[dict exists $headers $lname]} {
                            return [llength [dict get $headers $lname]]
                        }
                        return 0
                    }
                    exists {
                        return [dict exists $headers $lname]
                    }
                }
            }
        }

        namespace eval response {
            variable status     200
            variable reason     "OK"
            variable version    "1.1"
            variable headers    {}  ;# dict: header-name -> value-list
            variable payload    ""
            variable payload_length 0
            variable content_type ""
            variable is_redirect 0

            proc reset {} {
                variable status     200
                variable reason     "OK"
                variable version    "1.1"
                variable headers    {}
                variable payload    ""
                variable payload_length 0
                variable content_type ""
                variable is_redirect 0
            }

            proc configure {args} {
                foreach {key val} $args {
                    set varname [string trimleft $key -]
                    variable $varname
                    set $varname $val
                }
            }

            # Same header ops as request
            proc header {op name args} {
                variable headers
                set lname [string tolower $name]
                switch -exact -- $op {
                    get {
                        if {[dict exists $headers $lname]} {
                            return [lindex [dict get $headers $lname] 0]
                        }
                        return ""
                    }
                    values {
                        if {[dict exists $headers $lname]} {
                            return [dict get $headers $lname]
                        }
                        return {}
                    }
                    set {
                        dict set headers $lname [list [lindex $args 0]]
                    }
                    insert {
                        if {[dict exists $headers $lname]} {
                            set existing [dict get $headers $lname]
                            lappend existing [lindex $args 0]
                            dict set headers $lname $existing
                        } else {
                            dict set headers $lname [list [lindex $args 0]]
                        }
                    }
                    remove {
                        dict unset headers $lname
                    }
                    count {
                        if {[dict exists $headers $lname]} {
                            return [llength [dict get $headers $lname]]
                        }
                        return 0
                    }
                    exists {
                        return [dict exists $headers $lname]
                    }
                }
            }
        }

        # Was a response committed by HTTP::respond or HTTP::redirect?
        variable response_committed 0

        # Collect state
        variable collect_request  0
        variable collect_response 0
        variable collect_request_length  0
        variable collect_response_length 0

        proc reset_request {} {
            request::reset
            variable collect_request  0
            variable collect_request_length  0
            variable response_committed 0
        }

        proc reset_response {} {
            response::reset
            variable collect_response 0
            variable collect_response_length 0
        }

        proc reset {} {
            reset_request
            reset_response
        }
    }

    # ── Load-balancing layer ──────────────────────────────────────────
    #
    # Per-request: reset between HTTP transactions.

    namespace eval lb {
        variable pool           ""  ;# selected pool name
        variable pool_member    ""  ;# selected pool member (addr:port)
        variable node_addr      ""  ;# node address
        variable node_port      0   ;# node port
        variable snat_addr      ""  ;# SNAT translation address
        variable snat_port      0   ;# SNAT translation port
        variable snat_type      ""  ;# automap, snatpool, none
        variable lb_mode        ""  ;# round-robin, etc.
        variable selected       0   ;# has LB decision been made?
        variable detached       0   ;# server-side detached?

        # Pool state: pool_name -> {members {...} monitor "..." lb_mode "..."}
        variable pools
        array set pools {}

        # Node availability: addr:port -> status (up, down, disabled)
        variable node_status
        array set node_status {}

        proc reset {} {
            variable pool           ""
            variable pool_member    ""
            variable node_addr      ""
            variable node_port      0
            variable snat_addr      ""
            variable snat_port      0
            variable snat_type      ""
            variable lb_mode        ""
            variable selected       0
            variable detached       0
        }

        proc reset_all {} {
            reset
            variable pools
            array unset pools
            variable node_status
            array unset node_status
        }

        proc configure {args} {
            foreach {key val} $args {
                set varname [string trimleft $key -]
                variable $varname
                set $varname $val
            }
        }

        proc add_pool {name members args} {
            variable pools
            set pool_info [list members $members]
            foreach {key val} $args {
                lappend pool_info [string trimleft $key -] $val
            }
            set pools($name) $pool_info
        }

        proc set_node_status {addr_port status} {
            variable node_status
            set node_status($addr_port) $status
        }
    }

    # ── Table command state ───────────────────────────────────────────
    #
    # Simulates the session table (table command).
    # Persistent across connection boundaries.

    namespace eval table {
        # table_name -> {key -> {value timeout lifetime}}
        variable tables
        array set tables {}

        proc reset {} {
            variable tables
            array unset tables
        }

        proc set_entry {subtable key value {timeout 0} {lifetime 0}} {
            variable tables
            if {![info exists tables($subtable)]} {
                set tables($subtable) [dict create]
            }
            dict set tables($subtable) $key [list $value $timeout $lifetime]
        }

        proc get_entry {subtable key} {
            variable tables
            if {[info exists tables($subtable)] &&
                [dict exists $tables($subtable) $key]} {
                return [lindex [dict get $tables($subtable) $key] 0]
            }
            return ""
        }

        proc delete_entry {subtable key} {
            variable tables
            if {[info exists tables($subtable)] &&
                [dict exists $tables($subtable) $key]} {
                dict unset tables($subtable) $key
            }
        }

        proc lookup {subtable key} {
            return [get_entry $subtable $key]
        }

        proc keys {subtable args} {
            variable tables
            if {![info exists tables($subtable)]} {
                return {}
            }
            set all_keys [dict keys $tables($subtable)]
            if {[llength $args] > 0} {
                # Filter by pattern
                set pattern [lindex $args 0]
                set filtered {}
                foreach k $all_keys {
                    if {[string match $pattern $k]} {
                        lappend filtered $k
                    }
                }
                return $filtered
            }
            return $all_keys
        }

        proc incr_entry {subtable key {amount 1}} {
            variable tables
            set current [get_entry $subtable $key]
            if {$current eq ""} {
                set current 0
            }
            set new_val [expr {$current + $amount}]
            set_entry $subtable $key $new_val
            return $new_val
        }
    }

    # ── Data group / class state ──────────────────────────────────────
    #
    # Simulates iRules data groups (class command).
    # Loaded from SCF or configured manually.

    namespace eval datagroup {
        # dg_name -> {type "string|ip|integer" records {key val ...}}
        variable groups
        array set groups {}

        proc reset {} {
            variable groups
            array unset groups
        }

        proc add {name type records} {
            variable groups
            set groups($name) [list type $type records $records]
        }

        proc match {name value args} {
            variable groups
            if {![info exists groups($name)]} {
                error "class \"$name\" not found"
            }
            set dg $groups($name)
            set type [lindex $dg 1]
            set records [lindex $dg 3]

            # -name flag: check if key matches
            set check_name 0
            set check_value 0
            if {"-name" in $args} { set check_name 1 }
            if {"-value" in $args} { set check_value 1 }
            if {!$check_name && !$check_value} { set check_name 1 }

            switch -exact -- $type {
                string {
                    return [_match_string $records $value $check_name $check_value]
                }
                ip {
                    return [_match_ip $records $value]
                }
                integer {
                    return [_match_integer $records $value]
                }
                default {
                    return 0
                }
            }
        }

        proc lookup {name value} {
            variable groups
            if {![info exists groups($name)]} {
                error "class \"$name\" not found"
            }
            set dg $groups($name)
            set records [lindex $dg 3]
            # Return value for matching key
            foreach {k v} $records {
                if {$k eq $value} {
                    return $v
                }
            }
            return ""
        }

        proc names {name} {
            variable groups
            if {![info exists groups($name)]} {
                error "class \"$name\" not found"
            }
            set dg $groups($name)
            set records [lindex $dg 3]
            set result {}
            foreach {k v} $records {
                lappend result $k
            }
            return $result
        }

        proc size {name} {
            variable groups
            if {![info exists groups($name)]} {
                error "class \"$name\" not found"
            }
            set dg $groups($name)
            set records [lindex $dg 3]
            return [expr {[llength $records] / 2}]
        }

        proc _match_string {records value check_name check_value} {
            foreach {k v} $records {
                if {$check_name && [string equal $k $value]} { return 1 }
                if {$check_value && [string equal $v $value]} { return 1 }
            }
            return 0
        }

        proc _match_ip {records value} {
            # Simplified IP matching -- exact match only for now
            # TODO: CIDR subnet matching
            foreach {k v} $records {
                if {$k eq $value} { return 1 }
            }
            return 0
        }

        proc _match_integer {records value} {
            foreach {k v} $records {
                if {$k == $value} { return 1 }
            }
            return 0
        }
    }

    # ── DNS layer ─────────────────────────────────────────────────
    #
    # DNS message state for DNS events (DNS_REQUEST, DNS_RESPONSE).

    namespace eval dns {
        # DNS query/response message fields
        variable qname      ""      ;# query name (FQDN)
        variable qtype      "A"     ;# query type (A, AAAA, CNAME, MX, etc.)
        variable qclass     "IN"    ;# query class
        variable rcode      0       ;# response code (0=NOERROR, 3=NXDOMAIN, etc.)
        variable opcode     0       ;# operation code (0=QUERY, 1=IQUERY, etc.)
        variable id         0       ;# DNS message ID
        variable aa         0       ;# authoritative answer flag
        variable tc         0       ;# truncation flag
        variable rd         1       ;# recursion desired flag
        variable ra         0       ;# recursion available flag
        variable cd         0       ;# checking disabled flag
        variable ad         0       ;# authenticated data flag

        # Answer section: list of {name type class ttl rdata}
        variable answers    {}
        # Authority section
        variable authority  {}
        # Additional section
        variable additional {}

        # Whether a DNS response has been sent
        variable response_sent 0

        proc reset {} {
            variable qname      ""
            variable qtype      "A"
            variable qclass     "IN"
            variable rcode      0
            variable opcode     0
            variable id         0
            variable aa         0
            variable tc         0
            variable rd         1
            variable ra         0
            variable cd         0
            variable ad         0
            variable answers    {}
            variable authority  {}
            variable additional {}
            variable response_sent 0
        }

        proc configure {args} {
            foreach {key val} $args {
                set varname [string trimleft $key -]
                variable $varname
                set $varname $val
            }
        }
    }

    # ── Persistence state ─────────────────────────────────────────────

    namespace eval persist {
        variable mode       ""  ;# cookie, source_addr, ssl, universal, hash, etc.
        variable uie        ""  ;# universal inspect entry
        variable cookie_name ""
        variable entries    {}  ;# dict: key -> pool_member

        proc reset {} {
            variable mode       ""
            variable uie        ""
            variable cookie_name ""
            variable entries    {}
        }

        proc add {key pool_member} {
            variable entries
            dict set entries $key $pool_member
        }

        proc lookup_entry {key} {
            variable entries
            if {[dict exists $entries $key]} {
                return [dict get $entries $key]
            }
            return ""
        }
    }

    # ── iRule variables ───────────────────────────────────────────────
    #
    # Connection-scoped and static:: variables are managed here.
    # The orchestrator manages variable scope lifecycle.

    namespace eval vars {
        # static:: variables (persist across all connections)
        variable statics
        array set statics {}

        # Connection-scoped variables (persist across events within
        # a single connection, reset between connections)
        variable connection_vars
        array set connection_vars {}

        proc reset_connection {} {
            variable connection_vars
            array unset connection_vars
        }

        proc reset_all {} {
            variable statics
            variable connection_vars
            array unset statics
            array unset connection_vars
        }
    }

    # ── Event control ─────────────────────────────────────────────────
    #
    # Tracks event disable/enable state and after scheduling.

    namespace eval event_ctl {
        # event_name -> 0|1 (disabled/enabled)
        variable disabled
        array set disabled {}

        # Pending after callbacks: id -> {delay_ms script}
        variable after_queue {}
        variable after_id_counter 0

        proc reset {} {
            variable disabled
            variable after_queue
            variable after_id_counter
            array unset disabled
            set after_queue {}
            set after_id_counter 0
        }

        proc disable {event_name} {
            variable disabled
            set disabled($event_name) 1
        }

        proc enable {event_name} {
            variable disabled
            if {[info exists disabled($event_name)]} {
                unset disabled($event_name)
            }
        }

        proc is_disabled {event_name} {
            variable disabled
            return [info exists disabled($event_name)]
        }

        proc schedule_after {delay_ms script} {
            variable after_queue
            variable after_id_counter
            incr after_id_counter
            set id "after#$after_id_counter"
            lappend after_queue [list $id $delay_ms $script]
            return $id
        }

        proc cancel_after {id} {
            variable after_queue
            set new_queue {}
            foreach entry $after_queue {
                if {[lindex $entry 0] ne $id} {
                    lappend new_queue $entry
                }
            }
            set after_queue $new_queue
        }
    }

    # ── Log capture ───────────────────────────────────────────────────
    #
    # Captures log messages for assertion checking.

    namespace eval log_capture {
        variable entries {}

        proc reset {} {
            variable entries {}
        }

        proc add {facility level message} {
            variable entries
            lappend entries [list $facility $level $message [clock clicks -milliseconds]]
        }

        proc get {args} {
            variable entries
            if {[llength $args] == 0} {
                return $entries
            }
            # Filter by level
            set level [lindex $args 0]
            set filtered {}
            foreach entry $entries {
                if {[lindex $entry 1] eq $level} {
                    lappend filtered $entry
                }
            }
            return $filtered
        }

        proc count {} {
            variable entries
            return [llength $entries]
        }
    }

    # ── Master reset ──────────────────────────────────────────────────

    proc reset_connection_state {} {
        # Reset per-connection state (between test connections).
        # Does NOT reset ::static:: variables (they persist across connections,
        # just like on real TMM where RULE_INIT fires once per device load).
        connection::reset
        tls::reset
        http::reset
        dns::reset
        lb::reset
        persist::reset
        vars::reset_connection
        event_ctl::reset
        log_capture::reset
    }

    proc reset_request_state {} {
        # Reset per-request state (between HTTP transactions)
        http::reset_request
        http::reset_response
        lb::reset
    }

    proc reset_all {} {
        # Full reset including persistent state and static:: variables
        reset_connection_state
        table::reset
        datagroup::reset
        lb::reset_all
        vars::reset_all
        reset_statics
    }

    proc reset_statics {} {
        # Clear all static:: variables (for full test reset)
        foreach var [::info vars ::static::*] {
            catch { unset $var }
        }
    }
}

# ── static:: namespace ────────────────────────────────────────────
#
# iRules use `set static::varname value` to set variables that persist
# across all connections (RULE_INIT fires once per device load).
# We provide the namespace so these variables resolve normally.

namespace eval ::static {}
