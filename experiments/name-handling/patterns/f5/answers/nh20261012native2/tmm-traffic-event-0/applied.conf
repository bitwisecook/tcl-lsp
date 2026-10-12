ltm rule /Common/__tcl_lsp_probe_nh20261012native2_provider {
proc __nh_nh20261012native2_target {fixed value} {set rows {}; foreach value [list $fixed $value] {binary scan $value H* h; lappend rows $h}; list owner OLD argc 2 argv [list $fixed $value] argv-hex $rows}
proc cells {action invocation} {
 if {$action == {provider-write}} {set ::__nh_nh20261012native2::cell [list NAMESPACE_WRITE $invocation]; set ::__nh_nh20261012native2_global [list GLOBAL_WRITE $invocation]; set static::__nh_nh20261012native2_static [list STATIC_WRITE $invocation]}
 set rows {}
 foreach cell [list ::__nh_nh20261012native2::cell ::__nh_nh20261012native2_global static::__nh_nh20261012native2_static] {set c [catch {set $cell} v]; lappend rows $cell $c $v}
 list rows $rows level [info level]
}
proc link {local_name invocation} {
 set provider_local [list PROVIDER_LOCAL $invocation]
 upvar 1 $local_name linked
 set before $linked
 set linked [list CALLER_WRITE $invocation]
 list local $provider_local before $before after $linked level [info level]
}
}
ltm rule /Common/__tcl_lsp_probe_nh20261012native2_caller {
when RULE_INIT {
set nh_options_available unavailable
set nh_return_options {}
set nh_unit "[TMM::cmp_group]:[TMM::cmp_unit]"
set nh_invocation once
set nh_setup {}
set nh_operation [binary format H* 736574206e206e616d6573706163650a6966207b5b246e20657869737473203a3a5f5f6e685f6e6832303236313031326e6174697665325d207c7c205b696e666f20657869737473207374617469633a3a5f5f6e685f6e6832303236313031326e6174697665325f6f776e65725d7d207b6572726f72207b4f776e6564207374617465207365656420636f6c6c6973696f6e7d7d0a666f72656163682063656c6c205b6c697374203a3a5f5f6e685f6e6832303236313031326e6174697665325f676c6f62616c207374617469633a3a5f5f6e685f6e6832303236313031326e6174697665325f7374617469635d207b6966207b5b696e666f20657869737473202463656c6c5d7d207b6572726f72205b6c697374204f776e65642d63656c6c2d636f6c6c6973696f6e202463656c6c5d7d7d0a246e206576616c203a3a5f5f6e685f6e6832303236313031326e617469766532207b7d0a736574207374617469633a3a5f5f6e685f6e6832303236313031326e6174697665325f6f776e6572206e6832303236313031326e6174697665320a736574203a3a5f5f6e685f6e6832303236313031326e6174697665323a3a63656c6c204e414d4553504143455f534545440a736574203a3a5f5f6e685f6e6832303236313031326e6174697665325f676c6f62616c20474c4f42414c5f534545440a736574207374617469633a3a5f5f6e685f6e6832303236313031326e6174697665325f737461746963205354415449435f534545440a6c69737420736565646564206e6832303236313031326e6174697665320a]
set nh_setup_code [catch {eval $nh_setup} nh_setup_result]

set nh_ec {}
set nh_ei {}
set nh_ec_present 0
set nh_ei_present 0
if {$nh_setup_code == 1} {
    if {[info exists ::errorCode]} {set nh_ec $::errorCode; set nh_ec_present 1}
    if {[info exists ::errorInfo]} {set nh_ei $::errorInfo; set nh_ei_present 1}
}
foreach {nh_field nh_value} [list result $nh_setup_result metadata [list errorcode-present $nh_ec_present errorinfo-present $nh_ei_present return-options $nh_options_available] returnoptions $nh_return_options errorcode $nh_ec errorinfo $nh_ei] {
    if {$nh_field == "returnoptions" && $nh_options_available == "unavailable"} {continue}
    if {$nh_field != "result" && $nh_field != "returnoptions" && $nh_setup_code != 1} {continue}
    set nh_encoding utf-8
    set nh_encoder encoding
    if {[catch {$nh_encoder convertto utf-8 $nh_value} nh_encoded]} {
        set nh_encoded $nh_value
        set nh_encoding binary-scan-tcl-value
    }
    binary scan $nh_encoded H* nh_hex
    set nh_chunks [expr {([string length $nh_hex] + 399) / 400}]
    if {$nh_chunks == 0} {set nh_chunks 1}
    for {set nh_i 0} {$nh_i < $nh_chunks} {incr nh_i} {
        set nh_part [string range $nh_hex [expr {$nh_i * 400}] [expr {$nh_i * 400 + 399}]]
        log local0. "NHO|nh20261012native2|native2-state-seed|tmm-RULE_INIT|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
if {$nh_setup_code == 0} {
set nh_code [catch {eval $nh_operation} nh_result]

set nh_ec {}
set nh_ei {}
set nh_ec_present 0
set nh_ei_present 0
if {$nh_code == 1} {
    if {[info exists ::errorCode]} {set nh_ec $::errorCode; set nh_ec_present 1}
    if {[info exists ::errorInfo]} {set nh_ei $::errorInfo; set nh_ei_present 1}
}
foreach {nh_field nh_value} [list result $nh_result metadata [list errorcode-present $nh_ec_present errorinfo-present $nh_ei_present return-options $nh_options_available] returnoptions $nh_return_options errorcode $nh_ec errorinfo $nh_ei] {
    if {$nh_field == "returnoptions" && $nh_options_available == "unavailable"} {continue}
    if {$nh_field != "result" && $nh_field != "returnoptions" && $nh_code != 1} {continue}
    set nh_encoding utf-8
    set nh_encoder encoding
    if {[catch {$nh_encoder convertto utf-8 $nh_value} nh_encoded]} {
        set nh_encoded $nh_value
        set nh_encoding binary-scan-tcl-value
    }
    binary scan $nh_encoded H* nh_hex
    set nh_chunks [expr {([string length $nh_hex] + 399) / 400}]
    if {$nh_chunks == 0} {set nh_chunks 1}
    for {set nh_i 0} {$nh_i < $nh_chunks} {incr nh_i} {
        set nh_part [string range $nh_hex [expr {$nh_i * 400}] [expr {$nh_i * 400 + 399}]]
        log local0. "NHO|nh20261012native2|native2-state-seed|tmm-RULE_INIT|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
}
when HTTP_REQUEST {
HTTP::respond 200 content {__tcl_lsp_probe_nh20261012native2} Connection close
}
}
ltm virtual /Common/__tcl_lsp_probe_nh20261012native2_caller_vs { destination 192.0.2.10:19912 mask 255.255.255.255 ip-protocol tcp profiles { tcp {} http {} } rules { /Common/__tcl_lsp_probe_nh20261012native2_provider /Common/__tcl_lsp_probe_nh20261012native2_caller } }
