auth partition NH_nh20261012native1 { default-route-domain 0 }
sys folder /Common/__tcl_lsp_probe_nh20261012native1_folder {}
ltm rule /Common/__tcl_lsp_probe_nh20261012native1_provider {
proc __nh_nh20261012native1_target {fixed value} {set rows {}; foreach value [list $fixed $value] {binary scan $value H* h; lappend rows $h}; list owner OLD argc 2 argv [list $fixed $value] argv-hex $rows}
proc cells {action invocation} {
 if {$action == {provider-write}} {set ::__nh_nh20261012native1::cell [list NAMESPACE_WRITE $invocation]; set ::__nh_nh20261012native1_global [list GLOBAL_WRITE $invocation]; set static::__nh_nh20261012native1_static [list STATIC_WRITE $invocation]}
 set rows {}
 foreach cell [list ::__nh_nh20261012native1::cell ::__nh_nh20261012native1_global static::__nh_nh20261012native1_static] {set c [catch {set $cell} v]; lappend rows $cell $c $v}
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
ltm rule /Common/__tcl_lsp_probe_nh20261012native1_other {
proc __nh_nh20261012native1_target {fixed value} {set rows {}; foreach value [list $fixed $value] {binary scan $value H* h; lappend rows $h}; list owner OTHER argc 2 argv [list $fixed $value] argv-hex $rows}
}
ltm rule /Common/__tcl_lsp_probe_nh20261012native1_folder/__tcl_lsp_probe_nh20261012native1_folded {
proc __nh_nh20261012native1_target {fixed value} {set rows {}; foreach value [list $fixed $value] {binary scan $value H* h; lappend rows $h}; list owner FOLDER argc 2 argv [list $fixed $value] argv-hex $rows}
}
ltm rule /NH_nh20261012native1/__tcl_lsp_probe_nh20261012native1_foreign {
proc __nh_nh20261012native1_target {fixed value} {set rows {}; foreach value [list $fixed $value] {binary scan $value H* h; lappend rows $h}; list owner PARTITION argc 2 argv [list $fixed $value] argv-hex $rows}
}
ltm rule /Common/__tcl_lsp_probe_nh20261012native1_folder/__tcl_lsp_probe_nh20261012native1_caller {
proc record {nh_context nh_case nh_stage nh_code nh_value nh_invocation} {
set nh_options_available unavailable
set nh_return_options {}
set nh_unit [TMM::cmp_group]:[TMM::cmp_unit]

set nh_ec {}
set nh_ei {}
set nh_ec_present 0
set nh_ei_present 0
if {$nh_code == 1} {
    if {[info exists ::errorCode]} {set nh_ec $::errorCode; set nh_ec_present 1}
    if {[info exists ::errorInfo]} {set nh_ei $::errorInfo; set nh_ei_present 1}
}
foreach {nh_field nh_value} [list result $nh_value metadata [list errorcode-present $nh_ec_present errorinfo-present $nh_ei_present return-options $nh_options_available] returnoptions $nh_return_options errorcode $nh_ec errorinfo $nh_ei] {
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
        log local0. "NHO|nh20261012native1|$nh_case|$nh_context|$nh_stage|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}

}
proc __nh_nh20261012native1_target {fixed value} {list owner CALLER argc 2 argv [list $fixed $value]}
proc measure {nh_phase nh_invocation nh_action} {
# |bare-local|tmm-procedure|
set nh_setup {}
set nh_operation [binary format H* 63616c6c205f5f6e685f6e6832303236313031326e6174697665315f746172676574207b6120627d207b7b787d203a3a7461696c7d]
set nh_c [catch {eval $nh_setup} nh_v]
call record tmm-procedure bare-local setup $nh_c $nh_v $nh_invocation
if {$nh_c == 0} {set nh_c [catch {eval $nh_operation} nh_v]
call record tmm-procedure bare-local lookup $nh_c $nh_v $nh_invocation
}
# |root-relative|tmm-procedure|
set nh_setup {}
set nh_operation [binary format H* 63616c6c205f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574207b6120627d207b7b787d203a3a7461696c7d]
set nh_c [catch {eval $nh_setup} nh_v]
call record tmm-procedure root-relative setup $nh_c $nh_v $nh_invocation
if {$nh_c == 0} {set nh_c [catch {eval $nh_operation} nh_v]
call record tmm-procedure root-relative lookup $nh_c $nh_v $nh_invocation
}
# |root-absolute|tmm-procedure|
set nh_setup {}
set nh_operation [binary format H* 63616c6c202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574207b6120627d207b7b787d203a3a7461696c7d]
set nh_c [catch {eval $nh_setup} nh_v]
call record tmm-procedure root-absolute setup $nh_c $nh_v $nh_invocation
if {$nh_c == 0} {set nh_c [catch {eval $nh_operation} nh_v]
call record tmm-procedure root-absolute lookup $nh_c $nh_v $nh_invocation
}
# |other-owner|tmm-procedure|
set nh_setup {}
set nh_operation [binary format H* 63616c6c202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f6f746865723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574207b6120627d207b7b787d203a3a7461696c7d]
set nh_c [catch {eval $nh_setup} nh_v]
call record tmm-procedure other-owner setup $nh_c $nh_v $nh_invocation
if {$nh_c == 0} {set nh_c [catch {eval $nh_operation} nh_v]
call record tmm-procedure other-owner lookup $nh_c $nh_v $nh_invocation
}
# |folder-absolute|tmm-procedure|
set nh_setup {}
set nh_operation [binary format H* 63616c6c202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f666f6c6465722f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f666f6c6465643a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574207b6120627d207b7b787d203a3a7461696c7d]
set nh_c [catch {eval $nh_setup} nh_v]
call record tmm-procedure folder-absolute setup $nh_c $nh_v $nh_invocation
if {$nh_c == 0} {set nh_c [catch {eval $nh_operation} nh_v]
call record tmm-procedure folder-absolute lookup $nh_c $nh_v $nh_invocation
}
# |folder-leaf-relative|tmm-procedure|
set nh_setup {}
set nh_operation [binary format H* 63616c6c205f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f666f6c6465643a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574207b6120627d207b7b787d203a3a7461696c7d]
set nh_c [catch {eval $nh_setup} nh_v]
call record tmm-procedure folder-leaf-relative setup $nh_c $nh_v $nh_invocation
if {$nh_c == 0} {set nh_c [catch {eval $nh_operation} nh_v]
call record tmm-procedure folder-leaf-relative lookup $nh_c $nh_v $nh_invocation
}
# |partition-absolute|tmm-procedure|
set nh_setup {}
set nh_operation [binary format H* 63616c6c202f4e485f6e6832303236313031326e6174697665312f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f666f726569676e3a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574207b6120627d207b7b787d203a3a7461696c7d]
set nh_c [catch {eval $nh_setup} nh_v]
call record tmm-procedure partition-absolute setup $nh_c $nh_v $nh_invocation
if {$nh_c == 0} {set nh_c [catch {eval $nh_operation} nh_v]
call record tmm-procedure partition-absolute lookup $nh_c $nh_v $nh_invocation
}
# |missing-rule|tmm-procedure|
set nh_setup {}
set nh_operation [binary format H* 63616c6c202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f616273656e743a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574207b6120627d207b7b787d203a3a7461696c7d]
set nh_c [catch {eval $nh_setup} nh_v]
call record tmm-procedure missing-rule setup $nh_c $nh_v $nh_invocation
if {$nh_c == 0} {set nh_c [catch {eval $nh_operation} nh_v]
call record tmm-procedure missing-rule lookup $nh_c $nh_v $nh_invocation
}
# |missing-proc|tmm-procedure|
set nh_setup {}
set nh_operation [binary format H* 63616c6c202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f616273656e74207b6120627d207b7b787d203a3a7461696c7d]
set nh_c [catch {eval $nh_setup} nh_v]
call record tmm-procedure missing-proc setup $nh_c $nh_v $nh_invocation
if {$nh_c == 0} {set nh_c [catch {eval $nh_operation} nh_v]
call record tmm-procedure missing-proc lookup $nh_c $nh_v $nh_invocation
}
# |computed-variable-target|tmm-procedure|
set nh_setup {}
set nh_operation [binary format H* 73657420746172676574202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f7461726765743b2062696e617279207363616e202474617267657420482a207461726765745f6865783b206c697374207461726765742d68657820247461726765745f6865782076616c7565205b6576616c205b6c6973742063616c6c2024746172676574207b6120627d207b7b787d203a3a7461696c7d5d5d]
set nh_c [catch {eval $nh_setup} nh_v]
call record tmm-procedure computed-variable-target setup $nh_c $nh_v $nh_invocation
if {$nh_c == 0} {set nh_c [catch {eval $nh_operation} nh_v]
call record tmm-procedure computed-variable-target lookup $nh_c $nh_v $nh_invocation
}
# |computed-list-target|tmm-procedure|
set nh_setup {}
set nh_operation [binary format H* 73657420746172676574205b6a6f696e205b6c697374202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f7669646572205f5f6e685f6e6832303236313031326e6174697665315f7461726765745d203a3a5d3b2062696e617279207363616e202474617267657420482a207461726765745f6865783b206c697374207461726765742d68657820247461726765745f6865782076616c7565205b6576616c205b6c6973742063616c6c2024746172676574207b6120627d207b7b787d203a3a7461696c7d5d5d]
set nh_c [catch {eval $nh_setup} nh_v]
call record tmm-procedure computed-list-target setup $nh_c $nh_v $nh_invocation
if {$nh_c == 0} {set nh_c [catch {eval $nh_operation} nh_v]
call record tmm-procedure computed-list-target lookup $nh_c $nh_v $nh_invocation
}
# |literal-matched-arguments|tmm-procedure|
set nh_setup {}
set nh_operation [binary format H* 63616c6c202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574207b6120627d207b7b787d203a3a7461696c7d]
set nh_c [catch {eval $nh_setup} nh_v]
call record tmm-procedure literal-matched-arguments setup $nh_c $nh_v $nh_invocation
if {$nh_c == 0} {set nh_c [catch {eval $nh_operation} nh_v]
call record tmm-procedure literal-matched-arguments lookup $nh_c $nh_v $nh_invocation
}
# |list-prefix-arguments|tmm-procedure|
set nh_setup {}
set nh_operation [binary format H* 73657420707265666978205b6c6973742063616c6c202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574207b6120627d5d3b206576616c205b6c696e73657274202470726566697820656e64207b7b787d203a3a7461696c7d5d]
set nh_c [catch {eval $nh_setup} nh_v]
call record tmm-procedure list-prefix-arguments setup $nh_c $nh_v $nh_invocation
if {$nh_c == 0} {set nh_c [catch {eval $nh_operation} nh_v]
call record tmm-procedure list-prefix-arguments lookup $nh_c $nh_v $nh_invocation
}
# |space-joined-near-miss|tmm-procedure|
set nh_setup {}
set nh_operation [binary format H* 73657420707265666978202263616c6c202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f74617267657420612062223b206576616c2022247072656669782063206422]
set nh_c [catch {eval $nh_setup} nh_v]
call record tmm-procedure space-joined-near-miss setup $nh_c $nh_v $nh_invocation
if {$nh_c == 0} {set nh_c [catch {eval $nh_operation} nh_v]
call record tmm-procedure space-joined-near-miss lookup $nh_c $nh_v $nh_invocation
}
# |provider-lifetime|tmm-procedure|
set nh_setup {}
set nh_operation [binary format H* 6c69737420706861736520246e685f70686173652076616c7565205b63616c6c202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574207b6120627d207b7b787d203a3a7461696c7d5d]
set nh_c [catch {eval $nh_setup} nh_v]
call record tmm-procedure provider-lifetime setup $nh_c $nh_v $nh_invocation
if {$nh_c == 0} {set nh_c [catch {eval $nh_operation} nh_v]
call record tmm-procedure provider-lifetime lookup $nh_c $nh_v $nh_invocation
}
# |provider-local-upvar-write|tmm-procedure|
set nh_setup {}
set nh_operation [binary format H* 736574205f5f6e685f6e6832303236313031326e6174697665315f6c6f63616c2043414c4c45525f4245464f52453b207365742076205b63616c6c202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a6c696e6b205f5f6e685f6e6832303236313031326e6174697665315f6c6f63616c20246e685f696e766f636174696f6e5d3b206c6973742070726f76696465722024762063616c6c6572205b736574205f5f6e685f6e6832303236313031326e6174697665315f6c6f63616c5d]
set nh_c [catch {eval $nh_setup} nh_v]
call record tmm-procedure provider-local-upvar-write setup $nh_c $nh_v $nh_invocation
if {$nh_c == 0} {set nh_c [catch {eval $nh_operation} nh_v]
call record tmm-procedure provider-local-upvar-write lookup $nh_c $nh_v $nh_invocation
}
# |provider-shared-write-readback|tmm-procedure|
set nh_setup {}
set nh_operation [binary format H* 7365742070726f76696465725f76616c7565205b63616c6c202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a63656c6c7320246e685f616374696f6e20246e685f696e766f636174696f6e5d3b2073657420726f7773207b7d3b20666f72656163682063656c6c205b6c697374203a3a5f5f6e685f6e6832303236313031326e6174697665313a3a63656c6c203a3a5f5f6e685f6e6832303236313031326e6174697665315f676c6f62616c207374617469633a3a5f5f6e685f6e6832303236313031326e6174697665315f7374617469635d207b7365742063205b6361746368207b736574202463656c6c7d20765d3b206c617070656e6420726f7773202463656c6c2024632024767d3b206c69737420706861736520246e685f706861736520616374696f6e20246e685f616374696f6e2070726f7669646572202470726f76696465725f76616c75652063616c6c65722024726f7773]
set nh_c [catch {eval $nh_setup} nh_v]
call record tmm-procedure provider-shared-write-readback setup $nh_c $nh_v $nh_invocation
if {$nh_c == 0} {set nh_c [catch {eval $nh_operation} nh_v]
call record tmm-procedure provider-shared-write-readback lookup $nh_c $nh_v $nh_invocation
}
}
when RULE_INIT {
set nh_phase old
set nh_invocation load-old-init
set nh_action provider-write
# |owned-state-seed|tmm-RULE_INIT|
set nh_setup {}
set nh_operation [binary format H* 736574206e206e616d6573706163650a6966207b5b246e20657869737473203a3a5f5f6e685f6e6832303236313031326e6174697665315d207c7c205b696e666f20657869737473207374617469633a3a5f5f6e685f6e6832303236313031326e6174697665315f6f776e65725d7d207b6572726f72207b4f776e6564207374617465207365656420636f6c6c6973696f6e7d7d0a666f72656163682063656c6c205b6c697374203a3a5f5f6e685f6e6832303236313031326e6174697665315f676c6f62616c207374617469633a3a5f5f6e685f6e6832303236313031326e6174697665315f7374617469635d207b6966207b5b696e666f20657869737473202463656c6c5d7d207b6572726f72205b6c697374204f776e65642d63656c6c2d636f6c6c6973696f6e202463656c6c5d7d7d0a246e206576616c203a3a5f5f6e685f6e6832303236313031326e617469766531207b7d0a736574207374617469633a3a5f5f6e685f6e6832303236313031326e6174697665315f6f776e6572206e6832303236313031326e6174697665310a736574203a3a5f5f6e685f6e6832303236313031326e6174697665313a3a63656c6c204e414d4553504143455f534545440a736574203a3a5f5f6e685f6e6832303236313031326e6174697665315f676c6f62616c20474c4f42414c5f534545440a736574207374617469633a3a5f5f6e685f6e6832303236313031326e6174697665315f737461746963205354415449435f534545440a6c69737420736565646564206e6832303236313031326e6174697665310a]
set nh_c [catch {eval $nh_setup} nh_v]
call record tmm-RULE_INIT owned-state-seed setup $nh_c $nh_v $nh_invocation
if {$nh_c == 0} {set nh_c [catch {eval $nh_operation} nh_v]
call record tmm-RULE_INIT owned-state-seed lookup $nh_c $nh_v $nh_invocation
}
if {$nh_c != 0} {return}
# |bare-local|tmm-RULE_INIT|
set nh_setup {}
set nh_operation [binary format H* 63616c6c205f5f6e685f6e6832303236313031326e6174697665315f746172676574207b6120627d207b7b787d203a3a7461696c7d]
set nh_c [catch {eval $nh_setup} nh_v]
call record tmm-RULE_INIT bare-local setup $nh_c $nh_v $nh_invocation
if {$nh_c == 0} {set nh_c [catch {eval $nh_operation} nh_v]
call record tmm-RULE_INIT bare-local lookup $nh_c $nh_v $nh_invocation
}
# |root-relative|tmm-RULE_INIT|
set nh_setup {}
set nh_operation [binary format H* 63616c6c205f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574207b6120627d207b7b787d203a3a7461696c7d]
set nh_c [catch {eval $nh_setup} nh_v]
call record tmm-RULE_INIT root-relative setup $nh_c $nh_v $nh_invocation
if {$nh_c == 0} {set nh_c [catch {eval $nh_operation} nh_v]
call record tmm-RULE_INIT root-relative lookup $nh_c $nh_v $nh_invocation
}
# |root-absolute|tmm-RULE_INIT|
set nh_setup {}
set nh_operation [binary format H* 63616c6c202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574207b6120627d207b7b787d203a3a7461696c7d]
set nh_c [catch {eval $nh_setup} nh_v]
call record tmm-RULE_INIT root-absolute setup $nh_c $nh_v $nh_invocation
if {$nh_c == 0} {set nh_c [catch {eval $nh_operation} nh_v]
call record tmm-RULE_INIT root-absolute lookup $nh_c $nh_v $nh_invocation
}
# |other-owner|tmm-RULE_INIT|
set nh_setup {}
set nh_operation [binary format H* 63616c6c202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f6f746865723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574207b6120627d207b7b787d203a3a7461696c7d]
set nh_c [catch {eval $nh_setup} nh_v]
call record tmm-RULE_INIT other-owner setup $nh_c $nh_v $nh_invocation
if {$nh_c == 0} {set nh_c [catch {eval $nh_operation} nh_v]
call record tmm-RULE_INIT other-owner lookup $nh_c $nh_v $nh_invocation
}
# |folder-absolute|tmm-RULE_INIT|
set nh_setup {}
set nh_operation [binary format H* 63616c6c202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f666f6c6465722f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f666f6c6465643a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574207b6120627d207b7b787d203a3a7461696c7d]
set nh_c [catch {eval $nh_setup} nh_v]
call record tmm-RULE_INIT folder-absolute setup $nh_c $nh_v $nh_invocation
if {$nh_c == 0} {set nh_c [catch {eval $nh_operation} nh_v]
call record tmm-RULE_INIT folder-absolute lookup $nh_c $nh_v $nh_invocation
}
# |folder-leaf-relative|tmm-RULE_INIT|
set nh_setup {}
set nh_operation [binary format H* 63616c6c205f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f666f6c6465643a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574207b6120627d207b7b787d203a3a7461696c7d]
set nh_c [catch {eval $nh_setup} nh_v]
call record tmm-RULE_INIT folder-leaf-relative setup $nh_c $nh_v $nh_invocation
if {$nh_c == 0} {set nh_c [catch {eval $nh_operation} nh_v]
call record tmm-RULE_INIT folder-leaf-relative lookup $nh_c $nh_v $nh_invocation
}
# |partition-absolute|tmm-RULE_INIT|
set nh_setup {}
set nh_operation [binary format H* 63616c6c202f4e485f6e6832303236313031326e6174697665312f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f666f726569676e3a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574207b6120627d207b7b787d203a3a7461696c7d]
set nh_c [catch {eval $nh_setup} nh_v]
call record tmm-RULE_INIT partition-absolute setup $nh_c $nh_v $nh_invocation
if {$nh_c == 0} {set nh_c [catch {eval $nh_operation} nh_v]
call record tmm-RULE_INIT partition-absolute lookup $nh_c $nh_v $nh_invocation
}
# |missing-rule|tmm-RULE_INIT|
set nh_setup {}
set nh_operation [binary format H* 63616c6c202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f616273656e743a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574207b6120627d207b7b787d203a3a7461696c7d]
set nh_c [catch {eval $nh_setup} nh_v]
call record tmm-RULE_INIT missing-rule setup $nh_c $nh_v $nh_invocation
if {$nh_c == 0} {set nh_c [catch {eval $nh_operation} nh_v]
call record tmm-RULE_INIT missing-rule lookup $nh_c $nh_v $nh_invocation
}
# |missing-proc|tmm-RULE_INIT|
set nh_setup {}
set nh_operation [binary format H* 63616c6c202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f616273656e74207b6120627d207b7b787d203a3a7461696c7d]
set nh_c [catch {eval $nh_setup} nh_v]
call record tmm-RULE_INIT missing-proc setup $nh_c $nh_v $nh_invocation
if {$nh_c == 0} {set nh_c [catch {eval $nh_operation} nh_v]
call record tmm-RULE_INIT missing-proc lookup $nh_c $nh_v $nh_invocation
}
# |computed-variable-target|tmm-RULE_INIT|
set nh_setup {}
set nh_operation [binary format H* 73657420746172676574202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f7461726765743b2062696e617279207363616e202474617267657420482a207461726765745f6865783b206c697374207461726765742d68657820247461726765745f6865782076616c7565205b6576616c205b6c6973742063616c6c2024746172676574207b6120627d207b7b787d203a3a7461696c7d5d5d]
set nh_c [catch {eval $nh_setup} nh_v]
call record tmm-RULE_INIT computed-variable-target setup $nh_c $nh_v $nh_invocation
if {$nh_c == 0} {set nh_c [catch {eval $nh_operation} nh_v]
call record tmm-RULE_INIT computed-variable-target lookup $nh_c $nh_v $nh_invocation
}
# |computed-list-target|tmm-RULE_INIT|
set nh_setup {}
set nh_operation [binary format H* 73657420746172676574205b6a6f696e205b6c697374202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f7669646572205f5f6e685f6e6832303236313031326e6174697665315f7461726765745d203a3a5d3b2062696e617279207363616e202474617267657420482a207461726765745f6865783b206c697374207461726765742d68657820247461726765745f6865782076616c7565205b6576616c205b6c6973742063616c6c2024746172676574207b6120627d207b7b787d203a3a7461696c7d5d5d]
set nh_c [catch {eval $nh_setup} nh_v]
call record tmm-RULE_INIT computed-list-target setup $nh_c $nh_v $nh_invocation
if {$nh_c == 0} {set nh_c [catch {eval $nh_operation} nh_v]
call record tmm-RULE_INIT computed-list-target lookup $nh_c $nh_v $nh_invocation
}
# |literal-matched-arguments|tmm-RULE_INIT|
set nh_setup {}
set nh_operation [binary format H* 63616c6c202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574207b6120627d207b7b787d203a3a7461696c7d]
set nh_c [catch {eval $nh_setup} nh_v]
call record tmm-RULE_INIT literal-matched-arguments setup $nh_c $nh_v $nh_invocation
if {$nh_c == 0} {set nh_c [catch {eval $nh_operation} nh_v]
call record tmm-RULE_INIT literal-matched-arguments lookup $nh_c $nh_v $nh_invocation
}
# |list-prefix-arguments|tmm-RULE_INIT|
set nh_setup {}
set nh_operation [binary format H* 73657420707265666978205b6c6973742063616c6c202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574207b6120627d5d3b206576616c205b6c696e73657274202470726566697820656e64207b7b787d203a3a7461696c7d5d]
set nh_c [catch {eval $nh_setup} nh_v]
call record tmm-RULE_INIT list-prefix-arguments setup $nh_c $nh_v $nh_invocation
if {$nh_c == 0} {set nh_c [catch {eval $nh_operation} nh_v]
call record tmm-RULE_INIT list-prefix-arguments lookup $nh_c $nh_v $nh_invocation
}
# |space-joined-near-miss|tmm-RULE_INIT|
set nh_setup {}
set nh_operation [binary format H* 73657420707265666978202263616c6c202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f74617267657420612062223b206576616c2022247072656669782063206422]
set nh_c [catch {eval $nh_setup} nh_v]
call record tmm-RULE_INIT space-joined-near-miss setup $nh_c $nh_v $nh_invocation
if {$nh_c == 0} {set nh_c [catch {eval $nh_operation} nh_v]
call record tmm-RULE_INIT space-joined-near-miss lookup $nh_c $nh_v $nh_invocation
}
# |provider-lifetime|tmm-RULE_INIT|
set nh_setup {}
set nh_operation [binary format H* 6c69737420706861736520246e685f70686173652076616c7565205b63616c6c202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574207b6120627d207b7b787d203a3a7461696c7d5d]
set nh_c [catch {eval $nh_setup} nh_v]
call record tmm-RULE_INIT provider-lifetime setup $nh_c $nh_v $nh_invocation
if {$nh_c == 0} {set nh_c [catch {eval $nh_operation} nh_v]
call record tmm-RULE_INIT provider-lifetime lookup $nh_c $nh_v $nh_invocation
}
# |provider-local-upvar-write|tmm-RULE_INIT|
set nh_setup {}
set nh_operation [binary format H* 736574205f5f6e685f6e6832303236313031326e6174697665315f6c6f63616c2043414c4c45525f4245464f52453b207365742076205b63616c6c202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a6c696e6b205f5f6e685f6e6832303236313031326e6174697665315f6c6f63616c20246e685f696e766f636174696f6e5d3b206c6973742070726f76696465722024762063616c6c6572205b736574205f5f6e685f6e6832303236313031326e6174697665315f6c6f63616c5d]
set nh_c [catch {eval $nh_setup} nh_v]
call record tmm-RULE_INIT provider-local-upvar-write setup $nh_c $nh_v $nh_invocation
if {$nh_c == 0} {set nh_c [catch {eval $nh_operation} nh_v]
call record tmm-RULE_INIT provider-local-upvar-write lookup $nh_c $nh_v $nh_invocation
}
# |provider-shared-write-readback|tmm-RULE_INIT|
set nh_setup {}
set nh_operation [binary format H* 7365742070726f76696465725f76616c7565205b63616c6c202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a63656c6c7320246e685f616374696f6e20246e685f696e766f636174696f6e5d3b2073657420726f7773207b7d3b20666f72656163682063656c6c205b6c697374203a3a5f5f6e685f6e6832303236313031326e6174697665313a3a63656c6c203a3a5f5f6e685f6e6832303236313031326e6174697665315f676c6f62616c207374617469633a3a5f5f6e685f6e6832303236313031326e6174697665315f7374617469635d207b7365742063205b6361746368207b736574202463656c6c7d20765d3b206c617070656e6420726f7773202463656c6c2024632024767d3b206c69737420706861736520246e685f706861736520616374696f6e20246e685f616374696f6e2070726f7669646572202470726f76696465725f76616c75652063616c6c65722024726f7773]
set nh_c [catch {eval $nh_setup} nh_v]
call record tmm-RULE_INIT provider-shared-write-readback setup $nh_c $nh_v $nh_invocation
if {$nh_c == 0} {set nh_c [catch {eval $nh_operation} nh_v]
call record tmm-RULE_INIT provider-shared-write-readback lookup $nh_c $nh_v $nh_invocation
}
call measure $nh_phase load-old-init-procedure provider-write
}
when CLIENT_ACCEPTED {
if {[info exists __nh_nh20261012native1_connection]} {error {Connection cell collision}}
set __nh_nh20261012native1_connection [list CONNECTION_SEED [IP::client_addr] [TCP::client_port]]
}
when HTTP_REQUEST {
if {![info exists static::__nh_nh20261012native1_owner] || [set static::__nh_nh20261012native1_owner] != "nh20261012native1"} {HTTP::respond 503 content {UNOWNED} Connection close; return}
set nh_phase [HTTP::header value X-NH-Phase]
if {$nh_phase == {}} {set nh_phase UNLABELLED_STANDARD_REQUEST}
set nh_invocation [HTTP::header value X-NH-Invocation]
set nh_action [HTTP::header value X-NH-Action]
# |bare-local|tmm-traffic-event|
set nh_setup {}
set nh_operation [binary format H* 63616c6c205f5f6e685f6e6832303236313031326e6174697665315f746172676574207b6120627d207b7b787d203a3a7461696c7d]
set nh_c [catch {eval $nh_setup} nh_v]
call record tmm-traffic-event bare-local setup $nh_c $nh_v $nh_invocation
if {$nh_c == 0} {set nh_c [catch {eval $nh_operation} nh_v]
call record tmm-traffic-event bare-local lookup $nh_c $nh_v $nh_invocation
}
# |root-relative|tmm-traffic-event|
set nh_setup {}
set nh_operation [binary format H* 63616c6c205f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574207b6120627d207b7b787d203a3a7461696c7d]
set nh_c [catch {eval $nh_setup} nh_v]
call record tmm-traffic-event root-relative setup $nh_c $nh_v $nh_invocation
if {$nh_c == 0} {set nh_c [catch {eval $nh_operation} nh_v]
call record tmm-traffic-event root-relative lookup $nh_c $nh_v $nh_invocation
}
# |root-absolute|tmm-traffic-event|
set nh_setup {}
set nh_operation [binary format H* 63616c6c202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574207b6120627d207b7b787d203a3a7461696c7d]
set nh_c [catch {eval $nh_setup} nh_v]
call record tmm-traffic-event root-absolute setup $nh_c $nh_v $nh_invocation
if {$nh_c == 0} {set nh_c [catch {eval $nh_operation} nh_v]
call record tmm-traffic-event root-absolute lookup $nh_c $nh_v $nh_invocation
}
# |other-owner|tmm-traffic-event|
set nh_setup {}
set nh_operation [binary format H* 63616c6c202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f6f746865723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574207b6120627d207b7b787d203a3a7461696c7d]
set nh_c [catch {eval $nh_setup} nh_v]
call record tmm-traffic-event other-owner setup $nh_c $nh_v $nh_invocation
if {$nh_c == 0} {set nh_c [catch {eval $nh_operation} nh_v]
call record tmm-traffic-event other-owner lookup $nh_c $nh_v $nh_invocation
}
# |folder-absolute|tmm-traffic-event|
set nh_setup {}
set nh_operation [binary format H* 63616c6c202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f666f6c6465722f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f666f6c6465643a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574207b6120627d207b7b787d203a3a7461696c7d]
set nh_c [catch {eval $nh_setup} nh_v]
call record tmm-traffic-event folder-absolute setup $nh_c $nh_v $nh_invocation
if {$nh_c == 0} {set nh_c [catch {eval $nh_operation} nh_v]
call record tmm-traffic-event folder-absolute lookup $nh_c $nh_v $nh_invocation
}
# |folder-leaf-relative|tmm-traffic-event|
set nh_setup {}
set nh_operation [binary format H* 63616c6c205f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f666f6c6465643a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574207b6120627d207b7b787d203a3a7461696c7d]
set nh_c [catch {eval $nh_setup} nh_v]
call record tmm-traffic-event folder-leaf-relative setup $nh_c $nh_v $nh_invocation
if {$nh_c == 0} {set nh_c [catch {eval $nh_operation} nh_v]
call record tmm-traffic-event folder-leaf-relative lookup $nh_c $nh_v $nh_invocation
}
# |partition-absolute|tmm-traffic-event|
set nh_setup {}
set nh_operation [binary format H* 63616c6c202f4e485f6e6832303236313031326e6174697665312f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f666f726569676e3a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574207b6120627d207b7b787d203a3a7461696c7d]
set nh_c [catch {eval $nh_setup} nh_v]
call record tmm-traffic-event partition-absolute setup $nh_c $nh_v $nh_invocation
if {$nh_c == 0} {set nh_c [catch {eval $nh_operation} nh_v]
call record tmm-traffic-event partition-absolute lookup $nh_c $nh_v $nh_invocation
}
# |missing-rule|tmm-traffic-event|
set nh_setup {}
set nh_operation [binary format H* 63616c6c202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f616273656e743a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574207b6120627d207b7b787d203a3a7461696c7d]
set nh_c [catch {eval $nh_setup} nh_v]
call record tmm-traffic-event missing-rule setup $nh_c $nh_v $nh_invocation
if {$nh_c == 0} {set nh_c [catch {eval $nh_operation} nh_v]
call record tmm-traffic-event missing-rule lookup $nh_c $nh_v $nh_invocation
}
# |missing-proc|tmm-traffic-event|
set nh_setup {}
set nh_operation [binary format H* 63616c6c202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f616273656e74207b6120627d207b7b787d203a3a7461696c7d]
set nh_c [catch {eval $nh_setup} nh_v]
call record tmm-traffic-event missing-proc setup $nh_c $nh_v $nh_invocation
if {$nh_c == 0} {set nh_c [catch {eval $nh_operation} nh_v]
call record tmm-traffic-event missing-proc lookup $nh_c $nh_v $nh_invocation
}
# |computed-variable-target|tmm-traffic-event|
set nh_setup {}
set nh_operation [binary format H* 73657420746172676574202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f7461726765743b2062696e617279207363616e202474617267657420482a207461726765745f6865783b206c697374207461726765742d68657820247461726765745f6865782076616c7565205b6576616c205b6c6973742063616c6c2024746172676574207b6120627d207b7b787d203a3a7461696c7d5d5d]
set nh_c [catch {eval $nh_setup} nh_v]
call record tmm-traffic-event computed-variable-target setup $nh_c $nh_v $nh_invocation
if {$nh_c == 0} {set nh_c [catch {eval $nh_operation} nh_v]
call record tmm-traffic-event computed-variable-target lookup $nh_c $nh_v $nh_invocation
}
# |computed-list-target|tmm-traffic-event|
set nh_setup {}
set nh_operation [binary format H* 73657420746172676574205b6a6f696e205b6c697374202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f7669646572205f5f6e685f6e6832303236313031326e6174697665315f7461726765745d203a3a5d3b2062696e617279207363616e202474617267657420482a207461726765745f6865783b206c697374207461726765742d68657820247461726765745f6865782076616c7565205b6576616c205b6c6973742063616c6c2024746172676574207b6120627d207b7b787d203a3a7461696c7d5d5d]
set nh_c [catch {eval $nh_setup} nh_v]
call record tmm-traffic-event computed-list-target setup $nh_c $nh_v $nh_invocation
if {$nh_c == 0} {set nh_c [catch {eval $nh_operation} nh_v]
call record tmm-traffic-event computed-list-target lookup $nh_c $nh_v $nh_invocation
}
# |literal-matched-arguments|tmm-traffic-event|
set nh_setup {}
set nh_operation [binary format H* 63616c6c202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574207b6120627d207b7b787d203a3a7461696c7d]
set nh_c [catch {eval $nh_setup} nh_v]
call record tmm-traffic-event literal-matched-arguments setup $nh_c $nh_v $nh_invocation
if {$nh_c == 0} {set nh_c [catch {eval $nh_operation} nh_v]
call record tmm-traffic-event literal-matched-arguments lookup $nh_c $nh_v $nh_invocation
}
# |list-prefix-arguments|tmm-traffic-event|
set nh_setup {}
set nh_operation [binary format H* 73657420707265666978205b6c6973742063616c6c202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574207b6120627d5d3b206576616c205b6c696e73657274202470726566697820656e64207b7b787d203a3a7461696c7d5d]
set nh_c [catch {eval $nh_setup} nh_v]
call record tmm-traffic-event list-prefix-arguments setup $nh_c $nh_v $nh_invocation
if {$nh_c == 0} {set nh_c [catch {eval $nh_operation} nh_v]
call record tmm-traffic-event list-prefix-arguments lookup $nh_c $nh_v $nh_invocation
}
# |space-joined-near-miss|tmm-traffic-event|
set nh_setup {}
set nh_operation [binary format H* 73657420707265666978202263616c6c202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f74617267657420612062223b206576616c2022247072656669782063206422]
set nh_c [catch {eval $nh_setup} nh_v]
call record tmm-traffic-event space-joined-near-miss setup $nh_c $nh_v $nh_invocation
if {$nh_c == 0} {set nh_c [catch {eval $nh_operation} nh_v]
call record tmm-traffic-event space-joined-near-miss lookup $nh_c $nh_v $nh_invocation
}
# |provider-lifetime|tmm-traffic-event|
set nh_setup {}
set nh_operation [binary format H* 6c69737420706861736520246e685f70686173652076616c7565205b63616c6c202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574207b6120627d207b7b787d203a3a7461696c7d5d]
set nh_c [catch {eval $nh_setup} nh_v]
call record tmm-traffic-event provider-lifetime setup $nh_c $nh_v $nh_invocation
if {$nh_c == 0} {set nh_c [catch {eval $nh_operation} nh_v]
call record tmm-traffic-event provider-lifetime lookup $nh_c $nh_v $nh_invocation
}
# |provider-local-upvar-write|tmm-traffic-event|
set nh_setup {}
set nh_operation [binary format H* 736574205f5f6e685f6e6832303236313031326e6174697665315f6c6f63616c2043414c4c45525f4245464f52453b207365742076205b63616c6c202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a6c696e6b205f5f6e685f6e6832303236313031326e6174697665315f6c6f63616c20246e685f696e766f636174696f6e5d3b206c6973742070726f76696465722024762063616c6c6572205b736574205f5f6e685f6e6832303236313031326e6174697665315f6c6f63616c5d]
set nh_c [catch {eval $nh_setup} nh_v]
call record tmm-traffic-event provider-local-upvar-write setup $nh_c $nh_v $nh_invocation
if {$nh_c == 0} {set nh_c [catch {eval $nh_operation} nh_v]
call record tmm-traffic-event provider-local-upvar-write lookup $nh_c $nh_v $nh_invocation
}
# |provider-shared-write-readback|tmm-traffic-event|
set nh_setup {}
set nh_operation [binary format H* 7365742070726f76696465725f76616c7565205b63616c6c202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a63656c6c7320246e685f616374696f6e20246e685f696e766f636174696f6e5d3b2073657420726f7773207b7d3b20666f72656163682063656c6c205b6c697374203a3a5f5f6e685f6e6832303236313031326e6174697665313a3a63656c6c203a3a5f5f6e685f6e6832303236313031326e6174697665315f676c6f62616c207374617469633a3a5f5f6e685f6e6832303236313031326e6174697665315f7374617469635d207b7365742063205b6361746368207b736574202463656c6c7d20765d3b206c617070656e6420726f7773202463656c6c2024632024767d3b206c69737420706861736520246e685f706861736520616374696f6e20246e685f616374696f6e2070726f7669646572202470726f76696465725f76616c75652063616c6c65722024726f7773]
set nh_c [catch {eval $nh_setup} nh_v]
call record tmm-traffic-event provider-shared-write-readback setup $nh_c $nh_v $nh_invocation
if {$nh_c == 0} {set nh_c [catch {eval $nh_operation} nh_v]
call record tmm-traffic-event provider-shared-write-readback lookup $nh_c $nh_v $nh_invocation
}
# |connection-identity-write-read|tmm-traffic-event|
set nh_setup {}
set nh_operation [binary format H* 6966207b246e685f616374696f6e203d3d207b636f6e6e656374696f6e2d77726974657d7d207b736574205f5f6e685f6e6832303236313031326e6174697665315f636f6e6e656374696f6e205b6c69737420434f4e4e454354494f4e5f575249544520246e685f696e766f636174696f6e5d7d3b206c6973742063656c6c205b736574205f5f6e685f6e6832303236313031326e6174697665315f636f6e6e656374696f6e5d2070656572205b49503a3a636c69656e745f616464725d205b5443503a3a636c69656e745f706f72745d20756e6974205b544d4d3a3a636d705f67726f75705d3a5b544d4d3a3a636d705f756e69745d]
set nh_c [catch {eval $nh_setup} nh_v]
call record tmm-traffic-event connection-identity-write-read setup $nh_c $nh_v $nh_invocation
if {$nh_c == 0} {set nh_c [catch {eval $nh_operation} nh_v]
call record tmm-traffic-event connection-identity-write-read lookup $nh_c $nh_v $nh_invocation
}
call measure $nh_phase $nh_invocation $nh_action
HTTP::respond 200 content "__tcl_lsp_probe_nh20261012native1 [TMM::cmp_group]:[TMM::cmp_unit]" X-NH-TMM "[TMM::cmp_group]:[TMM::cmp_unit]" Connection keep-alive
}
}
ltm virtual /Common/__tcl_lsp_probe_nh20261012native1_folder/__tcl_lsp_probe_nh20261012native1_caller_vs { destination 192.0.2.10:19912 mask 255.255.255.255 ip-protocol tcp profiles { tcp {} http {} } rules { /Common/__tcl_lsp_probe_nh20261012native1_provider /Common/__tcl_lsp_probe_nh20261012native1_other /Common/__tcl_lsp_probe_nh20261012native1_folder/__tcl_lsp_probe_nh20261012native1_folded /Common/__tcl_lsp_probe_nh20261012native1_folder/__tcl_lsp_probe_nh20261012native1_caller } }
