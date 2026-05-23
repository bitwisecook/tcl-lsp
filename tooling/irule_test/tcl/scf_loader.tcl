# scf_loader.tcl -- Load BIG-IP SCF/bigip.conf into test framework state
#
# Parses a BIG-IP configuration file and populates the test framework
# with:
#   - Virtual server definitions (profiles, pools, rules, destination)
#   - Pool definitions (members, monitors, load-balancing mode)
#   - Data groups (internal: string, ip, integer)
#   - Node definitions (addresses)
#   - iRule source (loaded into the event handler registry)
#   - Profile types (HTTP, TCP, SSL, DNS, etc.)
#
# Usage:
#   ::scf::load_file "/path/to/bigip.conf"
#   ::scf::load_string $config_text
#   ::scf::apply_virtual "/Common/my_vs"
#      -- configures profiles, pools, and loads attached iRules
#
# Copyright (c) 2024 tcl-lsp contributors.  MIT licence.

namespace eval ::scf {

    # ── Parsed configuration storage ──────────────────────────────────

    # vs_path -> {destination pool profiles rules persist snat_type snatpool}
    variable virtual_servers
    array set virtual_servers {}

    # pool_path -> {members {addr:port ...} monitor lb_mode}
    variable pools
    array set pools {}

    # dg_path -> {type "string" records {key val ...}}
    variable data_groups
    array set data_groups {}

    # node_path -> {address "10.0.0.1"}
    variable nodes
    array set nodes {}

    # rule_path -> {source "when HTTP_REQUEST { ... }"}
    variable rules
    array set rules {}

    # profile_path -> {type "http|tcp|client-ssl|..."}
    variable profiles
    array set profiles {}

    # ── SCF parser ────────────────────────────────────────────────────
    #
    # Parses the brace-delimited BIG-IP config format.

    proc load_file {path} {
        # Use framework-internal file access (not blocked by TMM shim)
        if {[llength [info commands ::tmm::_orig_open]]} {
            set fd [::tmm::_orig_open $path r]
            set data [::tmm::_orig_read $fd]
            ::tmm::_orig_close $fd
        } else {
            set fd [open $path r]
            set data [read $fd]
            close $fd
        }
        load_string $data
    }

    proc load_string {source} {
        set blocks [_extract_blocks $source]

        foreach block $blocks {
            set header [lindex $block 0]
            set body [lindex $block 1]

            set parsed [_parse_header $header]
            if {$parsed eq ""} continue

            set module [lindex $parsed 0]
            set obj_type [lindex $parsed 1]
            set full_path [lindex $parsed 2]

            if {$module ne "ltm" && $module ne "gtm"} continue

            switch -exact -- $obj_type {
                "virtual" {
                    _parse_virtual $full_path $body
                }
                "pool" {
                    _parse_pool $full_path $body
                }
                "rule" {
                    _parse_rule $full_path $body
                }
                "node" {
                    _parse_node $full_path $body
                }
                "snatpool" {
                    _parse_snatpool $full_path $body
                }
                default {
                    # Check for two-word types
                    if {[string match "data-group*" $obj_type]} {
                        _parse_data_group $full_path $body $obj_type
                    } elseif {[string match "profile*" $obj_type]} {
                        _parse_profile $full_path $obj_type
                    } elseif {[string match "persistence*" $obj_type]} {
                        # persistence type
                    }
                }
            }
        }
    }

    # ── Block extraction ──────────────────────────────────────────────

    proc _extract_blocks {source} {
        set blocks [list]
        set pos 0
        set len [string length $source]

        while {$pos < $len} {
            # Skip whitespace and comments
            while {$pos < $len} {
                set ch [string index $source $pos]
                if {$ch eq " " || $ch eq "\t" || $ch eq "\n" || $ch eq "\r"} {
                    incr pos
                    continue
                }
                if {$ch eq "#"} {
                    while {$pos < $len && [string index $source $pos] ne "\n"} {
                        incr pos
                    }
                    continue
                }
                break
            }
            if {$pos >= $len} break

            # Read header (up to opening brace)
            set header_start $pos
            set found_brace 0
            while {$pos < $len} {
                set ch [string index $source $pos]
                if {$ch eq "\{"} {
                    set found_brace 1
                    break
                }
                if {$ch eq "\n"} {
                    # No brace on this line -- skip
                    incr pos
                    break
                }
                incr pos
            }

            if {!$found_brace} continue

            set header [string trim [string range $source $header_start [expr {$pos - 1}]]]
            incr pos  ;# skip opening brace

            # Find matching closing brace
            set depth 1
            set body_start $pos
            while {$pos < $len && $depth > 0} {
                set ch [string index $source $pos]
                if {$ch eq "\{"} { incr depth }
                if {$ch eq "\}"} { incr depth -1 }
                if {$ch eq "\""} {
                    incr pos
                    while {$pos < $len && [string index $source $pos] ne "\""} {
                        if {[string index $source $pos] eq "\\"} { incr pos }
                        incr pos
                    }
                }
                incr pos
            }

            set body [string range $source $body_start [expr {$pos - 2}]]
            lappend blocks [list $header $body]
        }

        return $blocks
    }

    # ── Header parsing ────────────────────────────────────────────────

    # Two-word types we recognise
    variable _two_word_types {
        "data-group internal" "data-group external"
        "profile http" "profile http2" "profile tcp" "profile udp"
        "profile client-ssl" "profile server-ssl"
        "profile ftp" "profile dns" "profile sip"
        "profile fasthttp" "profile fastl4"
        "profile one-connect" "profile websocket"
        "profile stream" "profile html"
        "persistence cookie" "persistence source-addr"
        "persistence ssl" "persistence universal"
        "persistence dest-addr" "persistence hash"
        "monitor http" "monitor https" "monitor tcp"
    }

    proc _parse_header {header} {
        variable _two_word_types

        set parts [split $header]
        # Remove empty elements
        set parts [lsearch -all -inline -not $parts ""]

        if {[llength $parts] < 3} { return "" }

        set module [lindex $parts 0]

        # Try two-word type
        if {[llength $parts] >= 4} {
            set two_word "[lindex $parts 1] [lindex $parts 2]"
            if {[lsearch -exact $_two_word_types $two_word] >= 0} {
                return [list $module $two_word [lindex $parts 3]]
            }
        }

        # Single-word type
        return [list $module [lindex $parts 1] [lindex $parts 2]]
    }

    # ── Property parser ───────────────────────────────────────────────

    proc _parse_properties {body} {
        set props [list]
        set pos 0
        set len [string length $body]

        while {$pos < $len} {
            # Skip whitespace
            while {$pos < $len} {
                set ch [string index $body $pos]
                if {$ch eq " " || $ch eq "\t" || $ch eq "\n" || $ch eq "\r"} {
                    incr pos
                } else {
                    break
                }
            }
            if {$pos >= $len} break

            # Read key
            set key_start $pos
            while {$pos < $len} {
                set ch [string index $body $pos]
                if {$ch eq " " || $ch eq "\t" || $ch eq "\n" || $ch eq "\r" || $ch eq "\{"} {
                    break
                }
                incr pos
            }
            set key [string range $body $key_start [expr {$pos - 1}]]
            if {$key eq ""} { incr pos; continue }

            # Skip whitespace
            while {$pos < $len && ([string index $body $pos] eq " " || [string index $body $pos] eq "\t")} {
                incr pos
            }

            if {$pos >= $len || [string index $body $pos] eq "\n"} {
                lappend props $key ""
                continue
            }

            if {[string index $body $pos] eq "\{"} {
                # Braced value
                set val_start $pos
                incr pos
                set depth 1
                while {$pos < $len && $depth > 0} {
                    set ch [string index $body $pos]
                    if {$ch eq "\{"} { incr depth }
                    if {$ch eq "\}"} { incr depth -1 }
                    if {$ch eq "\""} {
                        incr pos
                        while {$pos < $len && [string index $body $pos] ne "\""} {
                            if {[string index $body $pos] eq "\\"} { incr pos }
                            incr pos
                        }
                    }
                    incr pos
                }
                set val [string range $body $val_start [expr {$pos - 1}]]
                lappend props $key $val
            } else {
                # Simple value -- to end of line
                set val_start $pos
                while {$pos < $len && [string index $body $pos] ne "\n"} {
                    incr pos
                }
                set val [string trim [string range $body $val_start [expr {$pos - 1}]]]
                lappend props $key $val
            }
        }

        return $props
    }

    # ── List block parser ─────────────────────────────────────────────

    proc _parse_list_block {braced} {
        set inner [string trim $braced]
        if {[string index $inner 0] eq "\{"} {
            set inner [string range $inner 1 end]
        }
        if {[string index $inner end] eq "\}"} {
            set inner [string range $inner 0 end-1]
        }

        set items [list]
        set pos 0
        set len [string length $inner]

        while {$pos < $len} {
            while {$pos < $len} {
                set ch [string index $inner $pos]
                if {$ch eq " " || $ch eq "\t" || $ch eq "\n" || $ch eq "\r"} {
                    incr pos
                } else {
                    break
                }
            }
            if {$pos >= $len} break

            # Read item name
            set name_start $pos
            while {$pos < $len} {
                set ch [string index $inner $pos]
                if {$ch eq " " || $ch eq "\t" || $ch eq "\n" || $ch eq "\r" || $ch eq "\{" || $ch eq "\}"} {
                    break
                }
                incr pos
            }
            set name [string range $inner $name_start [expr {$pos - 1}]]

            # Skip whitespace
            while {$pos < $len && ([string index $inner $pos] eq " " || [string index $inner $pos] eq "\t")} {
                incr pos
            }

            # Skip sub-block if present
            if {$pos < $len && [string index $inner $pos] eq "\{"} {
                incr pos
                set depth 1
                while {$pos < $len && $depth > 0} {
                    if {[string index $inner $pos] eq "\{"} { incr depth }
                    if {[string index $inner $pos] eq "\}"} { incr depth -1 }
                    incr pos
                }
            }

            if {$name ne "" && $name ne "\{" && $name ne "\}"} {
                lappend items $name
            }
        }

        return $items
    }

    # ── Object parsers ────────────────────────────────────────────────

    # Profile type classification
    variable _profile_type_map
    array set _profile_type_map {
        http        HTTP
        http2       HTTP
        tcp         TCP
        udp         UDP
        client-ssl  CLIENTSSL
        server-ssl  SERVERSSL
        ftp         FTP
        dns         DNS
        sip         SIP
        fasthttp    FASTHTTP
        fastl4      FASTL4
        one-connect ONE_CONNECT
        websocket   WS
        stream      STREAM
        html        HTML
    }

    proc _parse_virtual {full_path body} {
        variable virtual_servers

        set props [_parse_properties $body]
        set vs_info [list]

        set pool ""
        set destination ""
        set rules_list [list]
        set profiles_list [list]
        set persist_list [list]
        set snat_type ""
        set snatpool ""

        foreach {k v} $props {
            switch -exact -- $k {
                pool        { set pool $v }
                destination { set destination $v }
                rules {
                    set rules_list [_parse_list_block $v]
                }
                profiles {
                    set profiles_list [_parse_list_block $v]
                }
                persist {
                    set persist_list [_parse_list_block $v]
                }
                source-address-translation {
                    set sat_props [_parse_properties [string trim $v "\{\}"]]
                    foreach {sk sv} $sat_props {
                        if {$sk eq "type"} { set snat_type $sv }
                        if {$sk eq "pool"} { set snatpool $sv }
                    }
                }
                snatpool { set snatpool $v }
            }
        }

        set virtual_servers($full_path) [list \
            destination $destination \
            pool $pool \
            profiles $profiles_list \
            rules $rules_list \
            persist $persist_list \
            snat_type $snat_type \
            snatpool $snatpool \
        ]
    }

    proc _parse_pool {full_path body} {
        variable pools

        set props [_parse_properties $body]
        set members [list]
        set monitor ""
        set lb_mode ""

        foreach {k v} $props {
            switch -exact -- $k {
                members {
                    set members [_parse_list_block $v]
                }
                monitor {
                    set monitor $v
                }
                load-balancing-mode {
                    set lb_mode $v
                }
            }
        }

        set pools($full_path) [list \
            members $members \
            monitor $monitor \
            lb_mode $lb_mode \
        ]
    }

    proc _parse_rule {full_path body} {
        variable rules
        set rules($full_path) [list source [string trim $body]]
    }

    proc _parse_node {full_path body} {
        variable nodes
        set props [_parse_properties $body]
        set addr ""
        foreach {k v} $props {
            if {$k eq "address"} { set addr $v }
        }
        set nodes($full_path) [list address $addr]
    }

    proc _parse_snatpool {full_path body} {
        # Stored in pools for simplicity
        set props [_parse_properties $body]
        set members [list]
        foreach {k v} $props {
            if {$k eq "members"} { set members [_parse_list_block $v] }
        }
    }

    proc _parse_data_group {full_path body obj_type} {
        variable data_groups

        set props [_parse_properties $body]
        set dg_type "string"
        set records [list]

        foreach {k v} $props {
            switch -exact -- $k {
                type { set dg_type $v }
                records {
                    set records [_parse_dg_records $v]
                }
            }
        }

        set data_groups($full_path) [list type $dg_type records $records]
    }

    proc _parse_dg_records {braced} {
        # Records format: { key1 { data value1 } key2 { data value2 } }
        set inner [string trim $braced "\{\}"]
        set records [list]

        set pos 0
        set len [string length $inner]

        while {$pos < $len} {
            # Skip whitespace
            while {$pos < $len} {
                set ch [string index $inner $pos]
                if {$ch eq " " || $ch eq "\t" || $ch eq "\n" || $ch eq "\r"} {
                    incr pos
                } else {
                    break
                }
            }
            if {$pos >= $len} break

            # Read key
            set key_start $pos
            while {$pos < $len} {
                set ch [string index $inner $pos]
                if {$ch eq " " || $ch eq "\t" || $ch eq "\n" || $ch eq "\{" || $ch eq "\}"} {
                    break
                }
                incr pos
            }
            set key [string trim [string range $inner $key_start [expr {$pos - 1}]]]
            if {$key eq ""} { incr pos; continue }

            # Skip whitespace
            while {$pos < $len && ([string index $inner $pos] eq " " || [string index $inner $pos] eq "\t")} {
                incr pos
            }

            # Look for value block { data VALUE }
            set val ""
            if {$pos < $len && [string index $inner $pos] eq "\{"} {
                incr pos
                set depth 1
                set val_body_start $pos
                while {$pos < $len && $depth > 0} {
                    if {[string index $inner $pos] eq "\{"} { incr depth }
                    if {[string index $inner $pos] eq "\}"} { incr depth -1 }
                    incr pos
                }
                set val_body [string trim [string range $inner $val_body_start [expr {$pos - 2}]]]
                # Parse "data VALUE" from the value block
                if {[regexp {data\s+"?([^"]*)"?} $val_body -> val_text]} {
                    set val $val_text
                } elseif {[regexp {data\s+(\S+)} $val_body -> val_text]} {
                    set val $val_text
                }
            }

            if {$key ne "\{" && $key ne "\}"} {
                lappend records $key $val
            }
        }

        return $records
    }

    proc _parse_profile {full_path obj_type} {
        variable profiles
        variable _profile_type_map

        # Extract the profile subtype from "profile http" etc.
        set parts [split $obj_type]
        set subtype ""
        if {[llength $parts] >= 2} {
            set subtype [lindex $parts 1]
        }
        set mapped_type "OTHER"
        if {[info exists _profile_type_map($subtype)]} {
            set mapped_type $_profile_type_map($subtype)
        }
        set profiles($full_path) [list type $mapped_type subtype $subtype]
    }

    # ── Resolve names ─────────────────────────────────────────────────

    proc _resolve_name {name arr_name} {
        upvar 1 $arr_name arr
        # Exact match
        if {[info exists arr($name)]} { return $name }
        # /Common/ prefix
        set candidate "/Common/$name"
        if {[info exists arr($candidate)]} { return $candidate }
        # Suffix match
        foreach key [array names arr] {
            if {[string match "*/$name" $key]} { return $key }
        }
        return ""
    }

    # ── Apply a virtual server config to the test framework ───────────
    #
    # This is the key function: given a VS name from the SCF, it:
    #   1. Determines the profile set
    #   2. Registers all attached pools
    #   3. Loads all attached iRules
    #   4. Configures data groups referenced by the iRules
    #   5. Sets up the orchestrator with the right profile combination

    proc apply_virtual {vs_name} {
        variable virtual_servers
        variable pools
        variable rules
        variable data_groups
        variable profiles

        set resolved [_resolve_name $vs_name virtual_servers]
        if {$resolved eq ""} {
            error "virtual server \"$vs_name\" not found in SCF"
        }
        set vs $virtual_servers($resolved)

        # Extract VS properties
        set vs_profiles [list]
        set vs_rules [list]
        set vs_pool ""
        foreach {k v} $vs {
            switch -exact -- $k {
                profiles { set vs_profiles $v }
                rules    { set vs_rules $v }
                pool     { set vs_pool $v }
            }
        }

        # Resolve profile types
        set profile_types [list]
        foreach pref $vs_profiles {
            set presolved [_resolve_name $pref profiles]
            if {$presolved ne ""} {
                set pinfo $profiles($presolved)
                foreach {pk pv} $pinfo {
                    if {$pk eq "type"} {
                        if {[lsearch -exact $profile_types $pv] < 0} {
                            lappend profile_types $pv
                        }
                    }
                }
            } else {
                # Try to infer from the profile reference name
                set inferred [_infer_profile_type $pref]
                if {$inferred ne "" && [lsearch -exact $profile_types $inferred] < 0} {
                    lappend profile_types $inferred
                }
            }
        }

        # Ensure TCP is present for TCP-based profiles
        if {[lsearch -exact $profile_types "TCP"] < 0} {
            set has_tcp_based 0
            foreach pt $profile_types {
                if {$pt ne "UDP" && $pt ne "DNS"} {
                    set has_tcp_based 1
                    break
                }
            }
            if {$has_tcp_based} {
                set profile_types [linsert $profile_types 0 "TCP"]
            }
        }

        # Configure the orchestrator
        ::orch::configure -profiles $profile_types

        # Parse destination for VIP address/port
        set dest ""
        foreach {k v} $vs { if {$k eq "destination"} { set dest $v } }
        if {$dest ne ""} {
            # Format: /Common/addr:port or addr:port
            set dest_clean [lindex [split $dest "/"] end]
            set colonpos [string last ":" $dest_clean]
            if {$colonpos >= 0} {
                set vip_addr [string range $dest_clean 0 [expr {$colonpos - 1}]]
                set vip_port [string range $dest_clean [expr {$colonpos + 1}] end]
                ::orch::configure -local_addr $vip_addr -local_port $vip_port
            }
        }

        # Register the default pool
        if {$vs_pool ne ""} {
            set pool_resolved [_resolve_name $vs_pool pools]
            if {$pool_resolved ne ""} {
                set pool_info $pools($pool_resolved)
                set members [list]
                foreach {pk pv} $pool_info {
                    if {$pk eq "members"} { set members $pv }
                }
                ::orch::add_pool $vs_pool $members
                # Also register by short name
                set short_name [lindex [split $vs_pool "/"] end]
                if {$short_name ne $vs_pool} {
                    ::orch::add_pool $short_name $members
                }
            }
        }

        # Register all pools (in case iRules reference pools not in the VS default)
        foreach pool_path [array names pools] {
            set pool_info $pools($pool_path)
            set members [list]
            foreach {pk pv} $pool_info {
                if {$pk eq "members"} { set members $pv }
            }
            ::orch::add_pool $pool_path $members
            set short [lindex [split $pool_path "/"] end]
            if {$short ne $pool_path} {
                ::orch::add_pool $short $members
            }
        }

        # Register all data groups
        foreach dg_path [array names data_groups] {
            set dg_info $data_groups($dg_path)
            set dg_type "string"
            set dg_records [list]
            foreach {dk dv} $dg_info {
                if {$dk eq "type"} { set dg_type $dv }
                if {$dk eq "records"} { set dg_records $dv }
            }
            ::orch::add_datagroup $dg_path $dg_type $dg_records
            set short [lindex [split $dg_path "/"] end]
            if {$short ne $dg_path} {
                ::orch::add_datagroup $short $dg_type $dg_records
            }
        }

        # Load attached iRules
        foreach rule_ref $vs_rules {
            set rule_resolved [_resolve_name $rule_ref rules]
            if {$rule_resolved ne ""} {
                set rule_info $rules($rule_resolved)
                set rule_source ""
                foreach {rk rv} $rule_info {
                    if {$rk eq "source"} { set rule_source $rv }
                }
                if {$rule_source ne ""} {
                    ::orch::load_irule $rule_source
                }
            }
        }

        return [list \
            virtual_server $resolved \
            profiles $profile_types \
            rules $vs_rules \
            pool $vs_pool \
        ]
    }

    # Infer profile type from a profile reference path
    proc _infer_profile_type {pref} {
        variable _profile_type_map
        set name [lindex [split $pref "/"] end]
        set lower [string tolower $name]

        # Check common profile names
        if {[string match "*clientssl*" $lower] || [string match "*client-ssl*" $lower]} {
            return "CLIENTSSL"
        }
        if {[string match "*serverssl*" $lower] || [string match "*server-ssl*" $lower]} {
            return "SERVERSSL"
        }
        if {$lower eq "http" || [string match "http*" $lower]} { return "HTTP" }
        if {$lower eq "tcp" || [string match "tcp*" $lower]} { return "TCP" }
        if {$lower eq "udp" || [string match "udp*" $lower]} { return "UDP" }
        if {[string match "*dns*" $lower]} { return "DNS" }
        if {[string match "*fasthttp*" $lower]} { return "FASTHTTP" }
        if {[string match "*fastl4*" $lower]} { return "FASTL4" }

        return ""
    }

    # ── Reset ─────────────────────────────────────────────────────────

    proc reset {} {
        variable virtual_servers
        variable pools
        variable data_groups
        variable nodes
        variable rules
        variable profiles

        array unset virtual_servers
        array unset pools
        array unset data_groups
        array unset nodes
        array unset rules
        array unset profiles
    }

    # ── Query helpers ─────────────────────────────────────────────────

    proc list_virtual_servers {} {
        variable virtual_servers
        return [array names virtual_servers]
    }

    proc list_pools {} {
        variable pools
        return [array names pools]
    }

    proc list_rules {} {
        variable rules
        return [array names rules]
    }

    proc list_data_groups {} {
        variable data_groups
        return [array names data_groups]
    }

    proc virtual_server_info {vs_name} {
        variable virtual_servers
        set resolved [_resolve_name $vs_name virtual_servers]
        if {$resolved eq ""} { return "" }
        return $virtual_servers($resolved)
    }

    proc pool_members {pool_name} {
        variable pools
        set resolved [_resolve_name $pool_name pools]
        if {$resolved eq ""} { return [list] }
        set info $pools($resolved)
        foreach {k v} $info {
            if {$k eq "members"} { return $v }
        }
        return [list]
    }
}
