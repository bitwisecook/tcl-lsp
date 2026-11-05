# BIG-IP Tcl log: nh20261012native1, tmm-RULE_INIT-1

## Question

The following cases investigate Tcl behavior relevant to name resolution and diagnostics in the LSP, compiler and VM. Each observation is limited to its own case and recorded context.

- **bare-local:** For identical procedure tails in isolated rules and partitions, what does call select through absolute partition/rule::proc, relative rule::proc and a bare local proc?

- **computed-list-target:** Does a variable/list-built call target with exactly the literal-control bytes reach the same rule procedure?

- **computed-variable-target:** Does a variable/list-built call target with exactly the literal-control bytes reach the same rule procedure?

- **connection-identity-write-read:** Which local/static::/qualified cells do rule-procedure calls read or write across caller and provider rules?

- **folder-absolute:** For identical procedure tails in isolated rules and partitions, what does call select through absolute partition/rule::proc, relative rule::proc and a bare local proc?

- **folder-leaf-relative:** For identical procedure tails in isolated rules and partitions, what does call select through absolute partition/rule::proc, relative rule::proc and a bare local proc?

- **list-prefix-arguments:** How does call pass fixed/list-expanded arguments containing spaces, braces and namespace separators?

- **literal-matched-arguments:** How does call pass fixed/list-expanded arguments containing spaces, braces and namespace separators?

- **missing-proc:** For identical procedure tails in isolated rules and partitions, what does call select through absolute partition/rule::proc, relative rule::proc and a bare local proc?

- **missing-rule:** For identical procedure tails in isolated rules and partitions, what does call select through absolute partition/rule::proc, relative rule::proc and a bare local proc?

- **other-owner:** For identical procedure tails in isolated rules and partitions, what does call select through absolute partition/rule::proc, relative rule::proc and a bare local proc?

- **owned-state-seed:** Which local/static::/qualified cells do rule-procedure calls read or write across caller and provider rules?

- **partition-absolute:** For identical procedure tails in isolated rules and partitions, what does call select through absolute partition/rule::proc, relative rule::proc and a bare local proc?

- **provider-lifetime:** Does binding change at load, attachment, or each invocation when an isolated provider rule is replaced or detached?

- **provider-local-upvar-write:** Which local/static::/qualified cells do rule-procedure calls read or write across caller and provider rules?

- **provider-shared-write-readback:** Which local/static::/qualified cells do rule-procedure calls read or write across caller and provider rules?

- **root-absolute:** For identical procedure tails in isolated rules and partitions, what does call select through absolute partition/rule::proc, relative rule::proc and a bare local proc?

- **root-relative:** For identical procedure tails in isolated rules and partitions, what does call select through absolute partition/rule::proc, relative rule::proc and a bare local proc?

- **space-joined-near-miss:** How does call pass fixed/list-expanded arguments containing spaces, braces and namespace separators?

## Assumptions

- **bare-local:** New exact native materialisation. Actual context/event/unit and administrative phase retained. Nested completions are list data. Fresh observer activation is distinct from continuing traffic caller; no suspended-frame claim.
- **computed-list-target:** New exact native materialisation. Actual context/event/unit and administrative phase retained. Nested completions are list data. Fresh observer activation is distinct from continuing traffic caller; no suspended-frame claim.
- **computed-variable-target:** New exact native materialisation. Actual context/event/unit and administrative phase retained. Nested completions are list data. Fresh observer activation is distinct from continuing traffic caller; no suspended-frame claim.
- **connection-identity-write-read:** New exact native materialisation. Actual context/event/unit and administrative phase retained. Nested completions are list data. Fresh observer activation is distinct from continuing traffic caller; no suspended-frame claim.
- **folder-absolute:** New exact native materialisation. Actual context/event/unit and administrative phase retained. Nested completions are list data. Fresh observer activation is distinct from continuing traffic caller; no suspended-frame claim.
- **folder-leaf-relative:** New exact native materialisation. Actual context/event/unit and administrative phase retained. Nested completions are list data. Fresh observer activation is distinct from continuing traffic caller; no suspended-frame claim.
- **list-prefix-arguments:** New exact native materialisation. Actual context/event/unit and administrative phase retained. Nested completions are list data. Fresh observer activation is distinct from continuing traffic caller; no suspended-frame claim.
- **literal-matched-arguments:** New exact native materialisation. Actual context/event/unit and administrative phase retained. Nested completions are list data. Fresh observer activation is distinct from continuing traffic caller; no suspended-frame claim.
- **missing-proc:** New exact native materialisation. Actual context/event/unit and administrative phase retained. Nested completions are list data. Fresh observer activation is distinct from continuing traffic caller; no suspended-frame claim.
- **missing-rule:** New exact native materialisation. Actual context/event/unit and administrative phase retained. Nested completions are list data. Fresh observer activation is distinct from continuing traffic caller; no suspended-frame claim.
- **other-owner:** New exact native materialisation. Actual context/event/unit and administrative phase retained. Nested completions are list data. Fresh observer activation is distinct from continuing traffic caller; no suspended-frame claim.
- **owned-state-seed:** New exact native materialisation. Actual context/event/unit and administrative phase retained. Nested completions are list data. Fresh observer activation is distinct from continuing traffic caller; no suspended-frame claim.
- **partition-absolute:** New exact native materialisation. Actual context/event/unit and administrative phase retained. Nested completions are list data. Fresh observer activation is distinct from continuing traffic caller; no suspended-frame claim.
- **provider-lifetime:** New exact native materialisation. Actual context/event/unit and administrative phase retained. Nested completions are list data. Fresh observer activation is distinct from continuing traffic caller; no suspended-frame claim.
- **provider-local-upvar-write:** New exact native materialisation. Actual context/event/unit and administrative phase retained. Nested completions are list data. Fresh observer activation is distinct from continuing traffic caller; no suspended-frame claim.
- **provider-shared-write-readback:** New exact native materialisation. Actual context/event/unit and administrative phase retained. Nested completions are list data. Fresh observer activation is distinct from continuing traffic caller; no suspended-frame claim.
- **root-absolute:** New exact native materialisation. Actual context/event/unit and administrative phase retained. Nested completions are list data. Fresh observer activation is distinct from continuing traffic caller; no suspended-frame claim.
- **root-relative:** New exact native materialisation. Actual context/event/unit and administrative phase retained. Nested completions are list data. Fresh observer activation is distinct from continuing traffic caller; no suspended-frame claim.
- **space-joined-near-miss:** New exact native materialisation. Actual context/event/unit and administrative phase retained. Nested completions are list data. Fresh observer activation is distinct from continuing traffic caller; no suspended-frame claim.

Other assumptions were not separately recorded.

## Script

```tcl-visible
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
```

## Results

### tmm-RULE_INIT, unit 0:0, invocation load-old-init

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- owned-state-seed, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- owned-state-seed, lookup, completion 0, result:

```text-visible
seeded nh20261012native1⟦EOF⟧
```

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call __tcl_lsp_probe_nh20261012native1_folded::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call __tcl_lsp_probe_nh20261012native1_folded::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-variable-target: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- owned-state-seed: intended operation reached.

- partition-absolute: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

### tmm-RULE_INIT, unit 0:1, invocation load-old-init

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- owned-state-seed, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- owned-state-seed, lookup, completion 0, result:

```text-visible
seeded nh20261012native1⟦EOF⟧
```

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call __tcl_lsp_probe_nh20261012native1_folded::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call __tcl_lsp_probe_nh20261012native1_folded::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("e⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- owned-state-seed: intended operation reached.

- partition-absolute: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.

### tmm-RULE_INIT, unit 0:2, invocation load-old-init

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- owned-state-seed, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- owned-state-seed, lookup, completion 0, result:

```text-visible
seeded nh20261012native1⟦EOF⟧
```

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call __tcl_lsp_probe_nh20261012native1_folded::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)⟦EOF⟧
```

- bare-local: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- other-owner: intended operation reached.

- owned-state-seed: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

### tmm-RULE_INIT, unit 0:3, invocation load-old-init

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- owned-state-seed, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- owned-state-seed, lookup, completion 0, result:

```text-visible
seeded nh20261012native1⟦EOF⟧
```

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call __tcl_lsp_probe_nh20261012native1_folded::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call __tcl_lsp_probe_nh20261012native1_folded::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- owned-state-seed: intended operation reached.

- partition-absolute: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

### tmm-procedure, unit 0:0, invocation 0

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-procedure.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("eval" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result:

```text-visible
phase UNLABELLED_STANDARD_REQUEST value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL 0} before CALLER_BEFORE after {CALLER_WRITE 0} level 4} caller {CALLER_WRITE 0}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase UNLABELLED_STANDARD_REQUEST action {} provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE phase-0} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE phase-0} static::__nh_nh20261012native1_static 0 {STATIC_WRITE phase-0}} level 4} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE phase-0} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE phase-0} static::__nh_nh20261012native1_static 0 {STATIC_WRITE phase-0}}⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- partition-absolute: intended operation reached.

- provider-lifetime: intended operation reached.

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.

### tmm-procedure, unit 0:0, invocation 1

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-procedure.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("eval" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result:

```text-visible
phase UNLABELLED_STANDARD_REQUEST value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL 1} before CALLER_BEFORE after {CALLER_WRITE 1} level 4} caller {CALLER_WRITE 1}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase UNLABELLED_STANDARD_REQUEST action {} provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}} level 4} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}}⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- partition-absolute: intended operation reached.

- provider-lifetime: intended operation reached.

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.

### tmm-procedure, unit 0:0, invocation 10

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-procedure.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("eval" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result:

```text-visible
phase UNLABELLED_STANDARD_REQUEST value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL 10} before CALLER_BEFORE after {CALLER_WRITE 10} level 4} caller {CALLER_WRITE 10}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase UNLABELLED_STANDARD_REQUEST action {} provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}} level 4} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}}⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- partition-absolute: intended operation reached.

- provider-lifetime: intended operation reached.

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.

### tmm-procedure, unit 0:0, invocation 11

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-procedure.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("eval" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result:

```text-visible
phase UNLABELLED_STANDARD_REQUEST value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL 11} before CALLER_BEFORE after {CALLER_WRITE 11} level 4} caller {CALLER_WRITE 11}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase UNLABELLED_STANDARD_REQUEST action {} provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}} level 4} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}}⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- partition-absolute: intended operation reached.

- provider-lifetime: intended operation reached.

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.

### tmm-procedure, unit 0:0, invocation 12

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-procedure.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("eval" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result:

```text-visible
phase UNLABELLED_STANDARD_REQUEST value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL 12} before CALLER_BEFORE after {CALLER_WRITE 12} level 4} caller {CALLER_WRITE 12}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase UNLABELLED_STANDARD_REQUEST action {} provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}} level 4} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}}⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- partition-absolute: intended operation reached.

- provider-lifetime: intended operation reached.

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.

### tmm-procedure, unit 0:0, invocation 13

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-procedure.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("eval" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result:

```text-visible
phase UNLABELLED_STANDARD_REQUEST value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL 13} before CALLER_BEFORE after {CALLER_WRITE 13} level 4} caller {CALLER_WRITE 13}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase UNLABELLED_STANDARD_REQUEST action {} provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}} level 4} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}}⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- partition-absolute: intended operation reached.

- provider-lifetime: intended operation reached.

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.

### tmm-procedure, unit 0:0, invocation 14

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-procedure.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("eval" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result:

```text-visible
phase UNLABELLED_STANDARD_REQUEST value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL 14} before CALLER_BEFORE after {CALLER_WRITE 14} level 4} caller {CALLER_WRITE 14}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase UNLABELLED_STANDARD_REQUEST action {} provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}} level 4} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}}⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- partition-absolute: intended operation reached.

- provider-lifetime: intended operation reached.

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.

### tmm-procedure, unit 0:0, invocation 15

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-procedure.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner RECREATED argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner RECREATED argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner RECREATED argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner RECREATED argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner RECREATED argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner RECREATED argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("eval" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result:

```text-visible
phase UNLABELLED_STANDARD_REQUEST value {owner RECREATED argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL 15} before CALLER_BEFORE after {CALLER_WRITE 15} level 4} caller {CALLER_WRITE 15}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase UNLABELLED_STANDARD_REQUEST action {} provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}} level 4} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}}⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- partition-absolute: intended operation reached.

- provider-lifetime: intended operation reached.

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.

### tmm-procedure, unit 0:0, invocation 2

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-procedure.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("eval" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result:

```text-visible
phase UNLABELLED_STANDARD_REQUEST value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL 2} before CALLER_BEFORE after {CALLER_WRITE 2} level 4} caller {CALLER_WRITE 2}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase UNLABELLED_STANDARD_REQUEST action {} provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}} level 4} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}}⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- partition-absolute: intended operation reached.

- provider-lifetime: intended operation reached.

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.

### tmm-procedure, unit 0:0, invocation 3

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-procedure.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("eval" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result:

```text-visible
phase UNLABELLED_STANDARD_REQUEST value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL 3} before CALLER_BEFORE after {CALLER_WRITE 3} level 4} caller {CALLER_WRITE 3}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase UNLABELLED_STANDARD_REQUEST action {} provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}} level 4} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}}⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- partition-absolute: intended operation reached.

- provider-lifetime: intended operation reached.

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.

### tmm-procedure, unit 0:0, invocation 4

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-procedure.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("eval" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result:

```text-visible
phase UNLABELLED_STANDARD_REQUEST value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL 4} before CALLER_BEFORE after {CALLER_WRITE 4} level 4} caller {CALLER_WRITE 4}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase UNLABELLED_STANDARD_REQUEST action {} provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}} level 4} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}}⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- partition-absolute: intended operation reached.

- provider-lifetime: intended operation reached.

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.

### tmm-procedure, unit 0:0, invocation 5

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-procedure.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("eval" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result:

```text-visible
phase UNLABELLED_STANDARD_REQUEST value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL 5} before CALLER_BEFORE after {CALLER_WRITE 5} level 4} caller {CALLER_WRITE 5}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase UNLABELLED_STANDARD_REQUEST action {} provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}} level 4} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}}⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- partition-absolute: intended operation reached.

- provider-lifetime: intended operation reached.

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.

### tmm-procedure, unit 0:0, invocation 6

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-procedure.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("eval" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result:

```text-visible
phase UNLABELLED_STANDARD_REQUEST value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL 6} before CALLER_BEFORE after {CALLER_WRITE 6} level 4} caller {CALLER_WRITE 6}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase UNLABELLED_STANDARD_REQUEST action {} provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}} level 4} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}}⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- partition-absolute: intended operation reached.

- provider-lifetime: intended operation reached.

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.

### tmm-procedure, unit 0:0, invocation 7

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-procedure.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("eval" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result:

```text-visible
phase UNLABELLED_STANDARD_REQUEST value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL 7} before CALLER_BEFORE after {CALLER_WRITE 7} level 4} caller {CALLER_WRITE 7}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase UNLABELLED_STANDARD_REQUEST action {} provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}} level 4} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}}⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- partition-absolute: intended operation reached.

- provider-lifetime: intended operation reached.

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.

### tmm-procedure, unit 0:0, invocation 8

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-procedure.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("eval" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result:

```text-visible
phase UNLABELLED_STANDARD_REQUEST value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL 8} before CALLER_BEFORE after {CALLER_WRITE 8} level 4} caller {CALLER_WRITE 8}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase UNLABELLED_STANDARD_REQUEST action {} provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}} level 4} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}}⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- partition-absolute: intended operation reached.

- provider-lifetime: intended operation reached.

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.

### tmm-procedure, unit 0:0, invocation 9

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-procedure.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("eval" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result:

```text-visible
phase UNLABELLED_STANDARD_REQUEST value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL 9} before CALLER_BEFORE after {CALLER_WRITE 9} level 4} caller {CALLER_WRITE 9}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase UNLABELLED_STANDARD_REQUEST action {} provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}} level 4} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}}⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- partition-absolute: intended operation reached.

- provider-lifetime: intended operation reached.

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.

### tmm-procedure, unit 0:0, invocation load-old-init-procedure

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-procedure.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL load-old-init-procedure} before CALLER_BEFORE after {CALLER_WRITE load-old-init-procedure} level 2} caller {CALLER_WRITE load-old-init-procedure}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase old action provider-write provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE load-old-init-procedure} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE load-old-init-procedure} static::__nh_nh20261012native1_static 0 {STATIC_WRITE load-old-init-procedure}} level 2} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE load-old-init-procedure} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE load-old-init-procedure} static::__nh_nh20261012native1_static 0 {STATIC_WRITE load-old-init-procedure}}⟦EOF⟧
```

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

- space-joined-near-miss: intended operation not recorded.

### tmm-procedure, unit 0:0, invocation pair-a-connection-read

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-procedure.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("eval" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result:

```text-visible
phase old value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL pair-a-connection-read} before CALLER_BEFORE after {CALLER_WRITE pair-a-connection-read} level 4} caller {CALLER_WRITE pair-a-connection-read}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase old action read provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE phase-0} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE phase-0} static::__nh_nh20261012native1_static 0 {STATIC_WRITE phase-0}} level 4} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE phase-0} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE phase-0} static::__nh_nh20261012native1_static 0 {STATIC_WRITE phase-0}}⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- partition-absolute: intended operation reached.

- provider-lifetime: intended operation reached.

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.

### tmm-procedure, unit 0:0, invocation pair-a-connection-write

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-procedure.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("eval" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result:

```text-visible
phase old value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL pair-a-connection-write} before CALLER_BEFORE after {CALLER_WRITE pair-a-connection-write} level 4} caller {CALLER_WRITE pair-a-connection-write}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase old action connection-write provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE phase-0} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE phase-0} static::__nh_nh20261012native1_static 0 {STATIC_WRITE phase-0}} level 4} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE phase-0} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE phase-0} static::__nh_nh20261012native1_static 0 {STATIC_WRITE phase-0}}⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- partition-absolute: intended operation reached.

- provider-lifetime: intended operation reached.

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.

### tmm-procedure, unit 0:0, invocation pair-a-provider-write

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-procedure.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("eval" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result:

```text-visible
phase old value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL pair-a-provider-write} before CALLER_BEFORE after {CALLER_WRITE pair-a-provider-write} level 4} caller {CALLER_WRITE pair-a-provider-write}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase old action provider-write provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}} level 4} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}}⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- partition-absolute: intended operation reached.

- provider-lifetime: intended operation reached.

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.

### tmm-procedure, unit 0:0, invocation pair-b-connection-read

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-procedure.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("eval" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result:

```text-visible
phase old value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL pair-b-connection-read} before CALLER_BEFORE after {CALLER_WRITE pair-b-connection-read} level 4} caller {CALLER_WRITE pair-b-connection-read}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase old action read provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE phase-0} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE phase-0} static::__nh_nh20261012native1_static 0 {STATIC_WRITE phase-0}} level 4} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE phase-0} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE phase-0} static::__nh_nh20261012native1_static 0 {STATIC_WRITE phase-0}}⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- partition-absolute: intended operation reached.

- provider-lifetime: intended operation reached.

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.

### tmm-procedure, unit 0:0, invocation pair-b-provider-read

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-procedure.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("eval" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result:

```text-visible
phase old value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL pair-b-provider-read} before CALLER_BEFORE after {CALLER_WRITE pair-b-provider-read} level 4} caller {CALLER_WRITE pair-b-provider-read}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase old action read provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}} level 4} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}}⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- partition-absolute: intended operation reached.

- provider-lifetime: intended operation reached.

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.

### tmm-procedure, unit 0:0, invocation pair-candidate-0

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-procedure.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("eval" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result:

```text-visible
phase old value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL pair-candidate-0} before CALLER_BEFORE after {CALLER_WRITE pair-candidate-0} level 4} caller {CALLER_WRITE pair-candidate-0}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase old action read provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE phase-0} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE phase-0} static::__nh_nh20261012native1_static 0 {STATIC_WRITE phase-0}} level 4} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE phase-0} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE phase-0} static::__nh_nh20261012native1_static 0 {STATIC_WRITE phase-0}}⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- partition-absolute: intended operation reached.

- provider-lifetime: intended operation reached.

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.

### tmm-procedure, unit 0:0, invocation pair-candidate-1

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-procedure.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("eval" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result:

```text-visible
phase old value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL pair-candidate-1} before CALLER_BEFORE after {CALLER_WRITE pair-candidate-1} level 4} caller {CALLER_WRITE pair-candidate-1}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase old action read provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE phase-0} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE phase-0} static::__nh_nh20261012native1_static 0 {STATIC_WRITE phase-0}} level 4} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE phase-0} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE phase-0} static::__nh_nh20261012native1_static 0 {STATIC_WRITE phase-0}}⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- partition-absolute: intended operation reached.

- provider-lifetime: intended operation reached.

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.

### tmm-procedure, unit 0:0, invocation pair-fresh-provider-read

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-procedure.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("eval" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result:

```text-visible
phase old value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL pair-fresh-provider-read} before CALLER_BEFORE after {CALLER_WRITE pair-fresh-provider-read} level 4} caller {CALLER_WRITE pair-fresh-provider-read}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase old action read provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}} level 4} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}}⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- partition-absolute: intended operation reached.

- provider-lifetime: intended operation reached.

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.

### tmm-procedure, unit 0:0, invocation phase-0

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-procedure.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("eval" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result:

```text-visible
phase old value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL phase-0} before CALLER_BEFORE after {CALLER_WRITE phase-0} level 4} caller {CALLER_WRITE phase-0}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase old action provider-write provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE phase-0} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE phase-0} static::__nh_nh20261012native1_static 0 {STATIC_WRITE phase-0}} level 4} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE phase-0} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE phase-0} static::__nh_nh20261012native1_static 0 {STATIC_WRITE phase-0}}⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- partition-absolute: intended operation reached.

- provider-lifetime: intended operation reached.

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.

### tmm-procedure, unit 0:0, invocation phase-1

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-procedure.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("eval" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result:

```text-visible
phase old value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL phase-1} before CALLER_BEFORE after {CALLER_WRITE phase-1} level 4} caller {CALLER_WRITE phase-1}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase old action read provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}} level 4} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}}⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- partition-absolute: intended operation reached.

- provider-lifetime: intended operation reached.

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.

### tmm-procedure, unit 0:0, invocation phase-10

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-procedure.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("eval" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result:

```text-visible
phase detached value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL phase-10} before CALLER_BEFORE after {CALLER_WRITE phase-10} level 4} caller {CALLER_WRITE phase-10}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase detached action read provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}} level 4} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}}⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- partition-absolute: intended operation reached.

- provider-lifetime: intended operation reached.

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.

### tmm-procedure, unit 0:0, invocation phase-11

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-procedure.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("eval" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result:

```text-visible
phase detached value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL phase-11} before CALLER_BEFORE after {CALLER_WRITE phase-11} level 4} caller {CALLER_WRITE phase-11}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase detached action read provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}} level 4} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}}⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- partition-absolute: intended operation reached.

- provider-lifetime: intended operation reached.

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.

### tmm-procedure, unit 0:0, invocation phase-12

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-procedure.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("eval" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result:

```text-visible
phase deleted value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL phase-12} before CALLER_BEFORE after {CALLER_WRITE phase-12} level 4} caller {CALLER_WRITE phase-12}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase deleted action read provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}} level 4} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}}⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- partition-absolute: intended operation reached.

- provider-lifetime: intended operation reached.

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.

### tmm-procedure, unit 0:0, invocation phase-13

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-procedure.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("eval" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result:

```text-visible
phase deleted value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL phase-13} before CALLER_BEFORE after {CALLER_WRITE phase-13} level 4} caller {CALLER_WRITE phase-13}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase deleted action read provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}} level 4} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}}⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- partition-absolute: intended operation reached.

- provider-lifetime: intended operation reached.

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.

### tmm-procedure, unit 0:0, invocation phase-14

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-procedure.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("eval" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result:

```text-visible
phase deleted value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL phase-14} before CALLER_BEFORE after {CALLER_WRITE phase-14} level 4} caller {CALLER_WRITE phase-14}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase deleted action read provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}} level 4} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}}⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- partition-absolute: intended operation reached.

- provider-lifetime: intended operation reached.

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.

### tmm-procedure, unit 0:0, invocation phase-15

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-procedure.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner RECREATED argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner RECREATED argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner RECREATED argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner RECREATED argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner RECREATED argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner RECREATED argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("eval" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result:

```text-visible
phase recreated value {owner RECREATED argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL phase-15} before CALLER_BEFORE after {CALLER_WRITE phase-15} level 4} caller {CALLER_WRITE phase-15}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase recreated action read provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}} level 4} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}}⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- partition-absolute: intended operation reached.

- provider-lifetime: intended operation reached.

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.

### tmm-procedure, unit 0:0, invocation phase-2

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-procedure.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("eval" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result:

```text-visible
phase old value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL phase-2} before CALLER_BEFORE after {CALLER_WRITE phase-2} level 4} caller {CALLER_WRITE phase-2}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase old action read provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}} level 4} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}}⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- partition-absolute: intended operation reached.

- provider-lifetime: intended operation reached.

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.

### tmm-procedure, unit 0:0, invocation phase-3

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-procedure.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("eval" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result:

```text-visible
phase new value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL phase-3} before CALLER_BEFORE after {CALLER_WRITE phase-3} level 4} caller {CALLER_WRITE phase-3}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase new action read provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}} level 4} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}}⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- partition-absolute: intended operation reached.

- provider-lifetime: intended operation reached.

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.

### tmm-procedure, unit 0:0, invocation phase-4

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-procedure.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("eval" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result:

```text-visible
phase new value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL phase-4} before CALLER_BEFORE after {CALLER_WRITE phase-4} level 4} caller {CALLER_WRITE phase-4}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase new action read provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}} level 4} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}}⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- partition-absolute: intended operation reached.

- provider-lifetime: intended operation reached.

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.

### tmm-procedure, unit 0:0, invocation phase-5

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-procedure.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("eval" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result:

```text-visible
phase new value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL phase-5} before CALLER_BEFORE after {CALLER_WRITE phase-5} level 4} caller {CALLER_WRITE phase-5}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase new action read provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}} level 4} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}}⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- partition-absolute: intended operation reached.

- provider-lifetime: intended operation reached.

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.

### tmm-procedure, unit 0:0, invocation phase-6

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-procedure.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("eval" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result:

```text-visible
phase rejected value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL phase-6} before CALLER_BEFORE after {CALLER_WRITE phase-6} level 4} caller {CALLER_WRITE phase-6}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase rejected action read provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}} level 4} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}}⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- partition-absolute: intended operation reached.

- provider-lifetime: intended operation reached.

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.

### tmm-procedure, unit 0:0, invocation phase-7

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-procedure.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("eval" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result:

```text-visible
phase rejected value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL phase-7} before CALLER_BEFORE after {CALLER_WRITE phase-7} level 4} caller {CALLER_WRITE phase-7}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase rejected action read provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}} level 4} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}}⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- partition-absolute: intended operation reached.

- provider-lifetime: intended operation reached.

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.

### tmm-procedure, unit 0:0, invocation phase-8

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-procedure.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("eval" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result:

```text-visible
phase rejected value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL phase-8} before CALLER_BEFORE after {CALLER_WRITE phase-8} level 4} caller {CALLER_WRITE phase-8}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase rejected action read provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}} level 4} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}}⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- partition-absolute: intended operation reached.

- provider-lifetime: intended operation reached.

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.

### tmm-procedure, unit 0:0, invocation phase-9

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-procedure.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("eval" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result:

```text-visible
phase detached value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL phase-9} before CALLER_BEFORE after {CALLER_WRITE phase-9} level 4} caller {CALLER_WRITE phase-9}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase detached action read provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}} level 4} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}}⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- partition-absolute: intended operation reached.

- provider-lifetime: intended operation reached.

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.

### tmm-procedure, unit 0:1, invocation load-old-init-procedure

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-procedure.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
ic::__nh_nh20261012native1_static 0 {STATIC_WRITE load-old-init-procedure}} level 2} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE load-old-init-procedure} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE load-old-init-procedure} static::__nh_nh20261012native1_static 0 {STATIC_WRITE load-old-init-procedure}}⟦EOF⟧
```

- provider-shared-write-readback: intended operation reached.

### tmm-procedure, unit 0:3, invocation load-old-init-procedure

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-procedure.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase old action provider-write provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE load-old-init-procedure} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE load-old-init-procedure} stat⟦EOF⟧
```

- provider-shared-write-readback: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 0

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("eval" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result:

```text-visible
phase UNLABELLED_STANDARD_REQUEST value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL 0} before CALLER_BEFORE after {CALLER_WRITE 0} level 3} caller {CALLER_WRITE 0}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase UNLABELLED_STANDARD_REQUEST action {} provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE phase-0} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE phase-0} static::__nh_nh20261012native1_static 0 {STATIC_WRITE phase-0}} level 3} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE phase-0} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE phase-0} static::__nh_nh20261012native1_static 0 {STATIC_WRITE phase-0}}⟦EOF⟧
```

- connection-identity-write-read, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- connection-identity-write-read, lookup, completion 0, result:

```text-visible
cell {CONNECTION_SEED [redacted-peer-address] 55352} peer [redacted-peer-address] 55352 unit 0:0⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- connection-identity-write-read: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- partition-absolute: intended operation reached.

- provider-lifetime: intended operation reached.

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 1

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("eval" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result:

```text-visible
phase UNLABELLED_STANDARD_REQUEST value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL 1} before CALLER_BEFORE after {CALLER_WRITE 1} level 3} caller {CALLER_WRITE 1}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase UNLABELLED_STANDARD_REQUEST action {} provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}} level 3} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}}⟦EOF⟧
```

- connection-identity-write-read, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- connection-identity-write-read, lookup, completion 0, result:

```text-visible
cell {CONNECTION_SEED [redacted-peer-address] 55357} peer [redacted-peer-address] 55357 unit 0:0⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- connection-identity-write-read: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- partition-absolute: intended operation reached.

- provider-lifetime: intended operation reached.

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 10

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("eval" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result:

```text-visible
phase UNLABELLED_STANDARD_REQUEST value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL 10} before CALLER_BEFORE after {CALLER_WRITE 10} level 3} caller {CALLER_WRITE 10}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase UNLABELLED_STANDARD_REQUEST action {} provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}} level 3} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}}⟦EOF⟧
```

- connection-identity-write-read, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- connection-identity-write-read, lookup, completion 0, result:

```text-visible
cell {CONNECTION_SEED [redacted-peer-address] 55378} peer [redacted-peer-address] 55378 unit 0:0⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- connection-identity-write-read: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- partition-absolute: intended operation reached.

- provider-lifetime: intended operation reached.

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 11

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("eval" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result:

```text-visible
phase UNLABELLED_STANDARD_REQUEST value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL 11} before CALLER_BEFORE after {CALLER_WRITE 11} level 3} caller {CALLER_WRITE 11}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase UNLABELLED_STANDARD_REQUEST action {} provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}} level 3} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}}⟦EOF⟧
```

- connection-identity-write-read, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- connection-identity-write-read, lookup, completion 0, result:

```text-visible
cell {CONNECTION_SEED [redacted-peer-address] 55380} peer [redacted-peer-address] 55380 unit 0:0⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- connection-identity-write-read: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- partition-absolute: intended operation reached.

- provider-lifetime: intended operation reached.

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 12

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("eval" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result:

```text-visible
phase UNLABELLED_STANDARD_REQUEST value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL 12} before CALLER_BEFORE after {CALLER_WRITE 12} level 3} caller {CALLER_WRITE 12}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase UNLABELLED_STANDARD_REQUEST action {} provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}} level 3} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}}⟦EOF⟧
```

- connection-identity-write-read, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- connection-identity-write-read, lookup, completion 0, result:

```text-visible
cell {CONNECTION_SEED [redacted-peer-address] 55384} peer [redacted-peer-address] 55384 unit 0:0⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- connection-identity-write-read: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- partition-absolute: intended operation reached.

- provider-lifetime: intended operation reached.

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 13

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("eval" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result:

```text-visible
phase UNLABELLED_STANDARD_REQUEST value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL 13} before CALLER_BEFORE after {CALLER_WRITE 13} level 3} caller {CALLER_WRITE 13}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase UNLABELLED_STANDARD_REQUEST action {} provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}} level 3} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}}⟦EOF⟧
```

- connection-identity-write-read, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- connection-identity-write-read, lookup, completion 0, result:

```text-visible
cell {CONNECTION_SEED [redacted-peer-address] 55386} peer [redacted-peer-address] 55386 unit 0:0⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- connection-identity-write-read: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- partition-absolute: intended operation reached.

- provider-lifetime: intended operation reached.

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 14

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("eval" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result:

```text-visible
phase UNLABELLED_STANDARD_REQUEST value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL 14} before CALLER_BEFORE after {CALLER_WRITE 14} level 3} caller {CALLER_WRITE 14}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase UNLABELLED_STANDARD_REQUEST action {} provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}} level 3} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}}⟦EOF⟧
```

- connection-identity-write-read, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- connection-identity-write-read, lookup, completion 0, result:

```text-visible
cell {CONNECTION_SEED [redacted-peer-address] 55388} peer [redacted-peer-address] 55388 unit 0:0⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- connection-identity-write-read: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- partition-absolute: intended operation reached.

- provider-lifetime: intended operation reached.

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 15

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner RECREATED argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner RECREATED argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner RECREATED argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner RECREATED argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner RECREATED argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner RECREATED argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("eval" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result:

```text-visible
phase UNLABELLED_STANDARD_REQUEST value {owner RECREATED argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL 15} before CALLER_BEFORE after {CALLER_WRITE 15} level 3} caller {CALLER_WRITE 15}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase UNLABELLED_STANDARD_REQUEST action {} provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}} level 3} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}}⟦EOF⟧
```

- connection-identity-write-read, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- connection-identity-write-read, lookup, completion 0, result:

```text-visible
cell {CONNECTION_SEED [redacted-peer-address] 55390} peer [redacted-peer-address] 55390 unit 0:0⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- connection-identity-write-read: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- partition-absolute: intended operation reached.

- provider-lifetime: intended operation reached.

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 2

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("eval" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result:

```text-visible
phase UNLABELLED_STANDARD_REQUEST value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL 2} before CALLER_BEFORE after {CALLER_WRITE 2} level 3} caller {CALLER_WRITE 2}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase UNLABELLED_STANDARD_REQUEST action {} provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}} level 3} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}}⟦EOF⟧
```

- connection-identity-write-read, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- connection-identity-write-read, lookup, completion 0, result:

```text-visible
cell {CONNECTION_SEED [redacted-peer-address] 55359} peer [redacted-peer-address] 55359 unit 0:0⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- connection-identity-write-read: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- partition-absolute: intended operation reached.

- provider-lifetime: intended operation reached.

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 3

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("eval" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result:

```text-visible
phase UNLABELLED_STANDARD_REQUEST value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL 3} before CALLER_BEFORE after {CALLER_WRITE 3} level 3} caller {CALLER_WRITE 3}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase UNLABELLED_STANDARD_REQUEST action {} provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}} level 3} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}}⟦EOF⟧
```

- connection-identity-write-read, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- connection-identity-write-read, lookup, completion 0, result:

```text-visible
cell {CONNECTION_SEED [redacted-peer-address] 55361} peer [redacted-peer-address] 55361 unit 0:0⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- connection-identity-write-read: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- partition-absolute: intended operation reached.

- provider-lifetime: intended operation reached.

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 4

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("eval" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result:

```text-visible
phase UNLABELLED_STANDARD_REQUEST value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL 4} before CALLER_BEFORE after {CALLER_WRITE 4} level 3} caller {CALLER_WRITE 4}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase UNLABELLED_STANDARD_REQUEST action {} provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}} level 3} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}}⟦EOF⟧
```

- connection-identity-write-read, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- connection-identity-write-read, lookup, completion 0, result:

```text-visible
cell {CONNECTION_SEED [redacted-peer-address] 55363} peer [redacted-peer-address] 55363 unit 0:0⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- connection-identity-write-read: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- partition-absolute: intended operation reached.

- provider-lifetime: intended operation reached.

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 5

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("eval" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result:

```text-visible
phase UNLABELLED_STANDARD_REQUEST value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL 5} before CALLER_BEFORE after {CALLER_WRITE 5} level 3} caller {CALLER_WRITE 5}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase UNLABELLED_STANDARD_REQUEST action {} provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}} level 3} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}}⟦EOF⟧
```

- connection-identity-write-read, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- connection-identity-write-read, lookup, completion 0, result:

```text-visible
cell {CONNECTION_SEED [redacted-peer-address] 55365} peer [redacted-peer-address] 55365 unit 0:0⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- connection-identity-write-read: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- partition-absolute: intended operation reached.

- provider-lifetime: intended operation reached.

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 6

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("eval" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result:

```text-visible
phase UNLABELLED_STANDARD_REQUEST value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL 6} before CALLER_BEFORE after {CALLER_WRITE 6} level 3} caller {CALLER_WRITE 6}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase UNLABELLED_STANDARD_REQUEST action {} provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}} level 3} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}}⟦EOF⟧
```

- connection-identity-write-read, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- connection-identity-write-read, lookup, completion 0, result:

```text-visible
cell {CONNECTION_SEED [redacted-peer-address] 55368} peer [redacted-peer-address] 55368 unit 0:0⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- connection-identity-write-read: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- partition-absolute: intended operation reached.

- provider-lifetime: intended operation reached.

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 7

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("eval" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result:

```text-visible
phase UNLABELLED_STANDARD_REQUEST value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL 7} before CALLER_BEFORE after {CALLER_WRITE 7} level 3} caller {CALLER_WRITE 7}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase UNLABELLED_STANDARD_REQUEST action {} provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}} level 3} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}}⟦EOF⟧
```

- connection-identity-write-read, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- connection-identity-write-read, lookup, completion 0, result:

```text-visible
cell {CONNECTION_SEED [redacted-peer-address] 55370} peer [redacted-peer-address] 55370 unit 0:0⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- connection-identity-write-read: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- partition-absolute: intended operation reached.

- provider-lifetime: intended operation reached.

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 8

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("eval" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result:

```text-visible
phase UNLABELLED_STANDARD_REQUEST value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL 8} before CALLER_BEFORE after {CALLER_WRITE 8} level 3} caller {CALLER_WRITE 8}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase UNLABELLED_STANDARD_REQUEST action {} provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}} level 3} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}}⟦EOF⟧
```

- connection-identity-write-read, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- connection-identity-write-read, lookup, completion 0, result:

```text-visible
cell {CONNECTION_SEED [redacted-peer-address] 55372} peer [redacted-peer-address] 55372 unit 0:0⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- connection-identity-write-read: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- partition-absolute: intended operation reached.

- provider-lifetime: intended operation reached.

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 9

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("eval" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result:

```text-visible
phase UNLABELLED_STANDARD_REQUEST value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL 9} before CALLER_BEFORE after {CALLER_WRITE 9} level 3} caller {CALLER_WRITE 9}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase UNLABELLED_STANDARD_REQUEST action {} provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}} level 3} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}}⟦EOF⟧
```

- connection-identity-write-read, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- connection-identity-write-read, lookup, completion 0, result:

```text-visible
cell {CONNECTION_SEED [redacted-peer-address] 55376} peer [redacted-peer-address] 55376 unit 0:0⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- connection-identity-write-read: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- partition-absolute: intended operation reached.

- provider-lifetime: intended operation reached.

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation pair-a-connection-read

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("eval" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result:

```text-visible
phase old value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL pair-a-connection-read} before CALLER_BEFORE after {CALLER_WRITE pair-a-connection-read} level 3} caller {CALLER_WRITE pair-a-connection-read}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase old action read provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE phase-0} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE phase-0} static::__nh_nh20261012native1_static 0 {STATIC_WRITE phase-0}} level 3} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE phase-0} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE phase-0} static::__nh_nh20261012native1_static 0 {STATIC_WRITE phase-0}}⟦EOF⟧
```

- connection-identity-write-read, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- connection-identity-write-read, lookup, completion 0, result:

```text-visible
cell {CONNECTION_WRITE pair-a-connection-write} peer [redacted-peer-address] 55353 unit 0:0⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- connection-identity-write-read: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- partition-absolute: intended operation reached.

- provider-lifetime: intended operation reached.

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation pair-a-connection-write

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("eval" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result:

```text-visible
phase old value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL pair-a-connection-write} before CALLER_BEFORE after {CALLER_WRITE pair-a-connection-write} level 3} caller {CALLER_WRITE pair-a-connection-write}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase old action connection-write provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE phase-0} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE phase-0} static::__nh_nh20261012native1_static 0 {STATIC_WRITE phase-0}} level 3} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE phase-0} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE phase-0} static::__nh_nh20261012native1_static 0 {STATIC_WRITE phase-0}}⟦EOF⟧
```

- connection-identity-write-read, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- connection-identity-write-read, lookup, completion 0, result:

```text-visible
cell {CONNECTION_WRITE pair-a-connection-write} peer [redacted-peer-address] 55353 unit 0:0⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- connection-identity-write-read: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- partition-absolute: intended operation reached.

- provider-lifetime: intended operation reached.

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation pair-a-provider-write

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("eval" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result:

```text-visible
phase old value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL pair-a-provider-write} before CALLER_BEFORE after {CALLER_WRITE pair-a-provider-write} level 3} caller {CALLER_WRITE pair-a-provider-write}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase old action provider-write provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}} level 3} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}}⟦EOF⟧
```

- connection-identity-write-read, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- connection-identity-write-read, lookup, completion 0, result:

```text-visible
cell {CONNECTION_WRITE pair-a-connection-write} peer [redacted-peer-address] 55353 unit 0:0⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- connection-identity-write-read: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- partition-absolute: intended operation reached.

- provider-lifetime: intended operation reached.

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation pair-b-connection-read

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("eval" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result:

```text-visible
phase old value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL pair-b-connection-read} before CALLER_BEFORE after {CALLER_WRITE pair-b-connection-read} level 3} caller {CALLER_WRITE pair-b-connection-read}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase old action read provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE phase-0} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE phase-0} static::__nh_nh20261012native1_static 0 {STATIC_WRITE phase-0}} level 3} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE phase-0} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE phase-0} static::__nh_nh20261012native1_static 0 {STATIC_WRITE phase-0}}⟦EOF⟧
```

- connection-identity-write-read, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- connection-identity-write-read, lookup, completion 0, result:

```text-visible
cell {CONNECTION_SEED [redacted-peer-address] 55354} peer [redacted-peer-address] 55354 unit 0:0⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- connection-identity-write-read: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- partition-absolute: intended operation reached.

- provider-lifetime: intended operation reached.

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation pair-b-provider-read

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("eval" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result:

```text-visible
phase old value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL pair-b-provider-read} before CALLER_BEFORE after {CALLER_WRITE pair-b-provider-read} level 3} caller {CALLER_WRITE pair-b-provider-read}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase old action read provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}} level 3} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}}⟦EOF⟧
```

- connection-identity-write-read, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- connection-identity-write-read, lookup, completion 0, result:

```text-visible
cell {CONNECTION_SEED [redacted-peer-address] 55354} peer [redacted-peer-address] 55354 unit 0:0⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- connection-identity-write-read: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- partition-absolute: intended operation reached.

- provider-lifetime: intended operation reached.

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation pair-candidate-0

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("eval" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result:

```text-visible
phase old value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL pair-candidate-0} before CALLER_BEFORE after {CALLER_WRITE pair-candidate-0} level 3} caller {CALLER_WRITE pair-candidate-0}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase old action read provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE phase-0} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE phase-0} static::__nh_nh20261012native1_static 0 {STATIC_WRITE phase-0}} level 3} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE phase-0} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE phase-0} static::__nh_nh20261012native1_static 0 {STATIC_WRITE phase-0}}⟦EOF⟧
```

- connection-identity-write-read, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- connection-identity-write-read, lookup, completion 0, result:

```text-visible
cell {CONNECTION_SEED [redacted-peer-address] 55353} peer [redacted-peer-address] 55353 unit 0:0⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- connection-identity-write-read: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- partition-absolute: intended operation reached.

- provider-lifetime: intended operation reached.

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation pair-candidate-1

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("eval" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result:

```text-visible
phase old value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL pair-candidate-1} before CALLER_BEFORE after {CALLER_WRITE pair-candidate-1} level 3} caller {CALLER_WRITE pair-candidate-1}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase old action read provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE phase-0} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE phase-0} static::__nh_nh20261012native1_static 0 {STATIC_WRITE phase-0}} level 3} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE phase-0} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE phase-0} static::__nh_nh20261012native1_static 0 {STATIC_WRITE phase-0}}⟦EOF⟧
```

- connection-identity-write-read, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- connection-identity-write-read, lookup, completion 0, result:

```text-visible
cell {CONNECTION_SEED [redacted-peer-address] 55354} peer [redacted-peer-address] 55354 unit 0:0⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- connection-identity-write-read: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- partition-absolute: intended operation reached.

- provider-lifetime: intended operation reached.

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation pair-fresh-provider-read

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("eval" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result:

```text-visible
phase old value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL pair-fresh-provider-read} before CALLER_BEFORE after {CALLER_WRITE pair-fresh-provider-read} level 3} caller {CALLER_WRITE pair-fresh-provider-read}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase old action read provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}} level 3} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}}⟦EOF⟧
```

- connection-identity-write-read, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- connection-identity-write-read, lookup, completion 0, result:

```text-visible
cell {CONNECTION_SEED [redacted-peer-address] 55355} peer [redacted-peer-address] 55355 unit 0:0⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- connection-identity-write-read: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- partition-absolute: intended operation reached.

- provider-lifetime: intended operation reached.

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation phase-0

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("eval" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result:

```text-visible
phase old value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL phase-0} before CALLER_BEFORE after {CALLER_WRITE phase-0} level 3} caller {CALLER_WRITE phase-0}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase old action provider-write provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE phase-0} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE phase-0} static::__nh_nh20261012native1_static 0 {STATIC_WRITE phase-0}} level 3} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE phase-0} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE phase-0} static::__nh_nh20261012native1_static 0 {STATIC_WRITE phase-0}}⟦EOF⟧
```

- connection-identity-write-read, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- connection-identity-write-read, lookup, completion 0, result:

```text-visible
cell {CONNECTION_SEED [redacted-peer-address] 55351} peer [redacted-peer-address] 55351 unit 0:0⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- connection-identity-write-read: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- partition-absolute: intended operation reached.

- provider-lifetime: intended operation reached.

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation phase-1

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("eval" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result:

```text-visible
phase old value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL phase-1} before CALLER_BEFORE after {CALLER_WRITE phase-1} level 3} caller {CALLER_WRITE phase-1}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase old action read provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}} level 3} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}}⟦EOF⟧
```

- connection-identity-write-read, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- connection-identity-write-read, lookup, completion 0, result:

```text-visible
cell {CONNECTION_SEED [redacted-peer-address] 55356} peer [redacted-peer-address] 55356 unit 0:0⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- connection-identity-write-read: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- partition-absolute: intended operation reached.

- provider-lifetime: intended operation reached.

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation phase-10

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("eval" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result:

```text-visible
phase detached value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL phase-10} before CALLER_BEFORE after {CALLER_WRITE phase-10} level 3} caller {CALLER_WRITE phase-10}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase detached action read provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}} level 3} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}}⟦EOF⟧
```

- connection-identity-write-read, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- connection-identity-write-read, lookup, completion 0, result:

```text-visible
cell {CONNECTION_SEED [redacted-peer-address] 55377} peer [redacted-peer-address] 55377 unit 0:0⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- connection-identity-write-read: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- partition-absolute: intended operation reached.

- provider-lifetime: intended operation reached.

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation phase-11

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("eval" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result:

```text-visible
phase detached value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL phase-11} before CALLER_BEFORE after {CALLER_WRITE phase-11} level 3} caller {CALLER_WRITE phase-11}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase detached action read provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}} level 3} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}}⟦EOF⟧
```

- connection-identity-write-read, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- connection-identity-write-read, lookup, completion 0, result:

```text-visible
cell {CONNECTION_SEED [redacted-peer-address] 55379} peer [redacted-peer-address] 55379 unit 0:0⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- connection-identity-write-read: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- partition-absolute: intended operation reached.

- provider-lifetime: intended operation reached.

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation phase-12

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("eval" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result:

```text-visible
phase deleted value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL phase-12} before CALLER_BEFORE after {CALLER_WRITE phase-12} level 3} caller {CALLER_WRITE phase-12}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase deleted action read provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}} level 3} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}}⟦EOF⟧
```

- connection-identity-write-read, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- connection-identity-write-read, lookup, completion 0, result:

```text-visible
cell {CONNECTION_SEED [redacted-peer-address] 55383} peer [redacted-peer-address] 55383 unit 0:0⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- connection-identity-write-read: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- partition-absolute: intended operation reached.

- provider-lifetime: intended operation reached.

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation phase-13

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("eval" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result:

```text-visible
phase deleted value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL phase-13} before CALLER_BEFORE after {CALLER_WRITE phase-13} level 3} caller {CALLER_WRITE phase-13}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase deleted action read provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}} level 3} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}}⟦EOF⟧
```

- connection-identity-write-read, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- connection-identity-write-read, lookup, completion 0, result:

```text-visible
cell {CONNECTION_SEED [redacted-peer-address] 55385} peer [redacted-peer-address] 55385 unit 0:0⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- connection-identity-write-read: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- partition-absolute: intended operation reached.

- provider-lifetime: intended operation reached.

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation phase-14

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("eval" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result:

```text-visible
phase deleted value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL phase-14} before CALLER_BEFORE after {CALLER_WRITE phase-14} level 3} caller {CALLER_WRITE phase-14}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase deleted action read provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}} level 3} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}}⟦EOF⟧
```

- connection-identity-write-read, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- connection-identity-write-read, lookup, completion 0, result:

```text-visible
cell {CONNECTION_SEED [redacted-peer-address] 55387} peer [redacted-peer-address] 55387 unit 0:0⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- connection-identity-write-read: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- partition-absolute: intended operation reached.

- provider-lifetime: intended operation reached.

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation phase-15

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner RECREATED argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner RECREATED argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner RECREATED argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner RECREATED argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner RECREATED argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner RECREATED argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("eval" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result:

```text-visible
phase recreated value {owner RECREATED argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL phase-15} before CALLER_BEFORE after {CALLER_WRITE phase-15} level 3} caller {CALLER_WRITE phase-15}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase recreated action read provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}} level 3} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}}⟦EOF⟧
```

- connection-identity-write-read, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- connection-identity-write-read, lookup, completion 0, result:

```text-visible
cell {CONNECTION_SEED [redacted-peer-address] 55389} peer [redacted-peer-address] 55389 unit 0:0⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- connection-identity-write-read: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- partition-absolute: intended operation reached.

- provider-lifetime: intended operation reached.

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation phase-2

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("eval" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result:

```text-visible
phase old value {owner OLD argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL phase-2} before CALLER_BEFORE after {CALLER_WRITE phase-2} level 3} caller {CALLER_WRITE phase-2}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase old action read provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}} level 3} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}}⟦EOF⟧
```

- connection-identity-write-read, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- connection-identity-write-read, lookup, completion 0, result:

```text-visible
cell {CONNECTION_SEED [redacted-peer-address] 55358} peer [redacted-peer-address] 55358 unit 0:0⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- connection-identity-write-read: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- partition-absolute: intended operation reached.

- provider-lifetime: intended operation reached.

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation phase-3

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("eval" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result:

```text-visible
phase new value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL phase-3} before CALLER_BEFORE after {CALLER_WRITE phase-3} level 3} caller {CALLER_WRITE phase-3}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase new action read provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}} level 3} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}}⟦EOF⟧
```

- connection-identity-write-read, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- connection-identity-write-read, lookup, completion 0, result:

```text-visible
cell {CONNECTION_SEED [redacted-peer-address] 55360} peer [redacted-peer-address] 55360 unit 0:0⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- connection-identity-write-read: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- partition-absolute: intended operation reached.

- provider-lifetime: intended operation reached.

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation phase-4

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("eval" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result:

```text-visible
phase new value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL phase-4} before CALLER_BEFORE after {CALLER_WRITE phase-4} level 3} caller {CALLER_WRITE phase-4}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase new action read provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}} level 3} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}}⟦EOF⟧
```

- connection-identity-write-read, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- connection-identity-write-read, lookup, completion 0, result:

```text-visible
cell {CONNECTION_SEED [redacted-peer-address] 55362} peer [redacted-peer-address] 55362 unit 0:0⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- connection-identity-write-read: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- partition-absolute: intended operation reached.

- provider-lifetime: intended operation reached.

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation phase-5

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("eval" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result:

```text-visible
phase new value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL phase-5} before CALLER_BEFORE after {CALLER_WRITE phase-5} level 3} caller {CALLER_WRITE phase-5}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase new action read provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}} level 3} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}}⟦EOF⟧
```

- connection-identity-write-read, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- connection-identity-write-read, lookup, completion 0, result:

```text-visible
cell {CONNECTION_SEED [redacted-peer-address] 55364} peer [redacted-peer-address] 55364 unit 0:0⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- connection-identity-write-read: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- partition-absolute: intended operation reached.

- provider-lifetime: intended operation reached.

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation phase-6

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("eval" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result:

```text-visible
phase rejected value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL phase-6} before CALLER_BEFORE after {CALLER_WRITE phase-6} level 3} caller {CALLER_WRITE phase-6}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase rejected action read provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}} level 3} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}}⟦EOF⟧
```

- connection-identity-write-read, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- connection-identity-write-read, lookup, completion 0, result:

```text-visible
cell {CONNECTION_SEED [redacted-peer-address] 55367} peer [redacted-peer-address] 55367 unit 0:0⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- connection-identity-write-read: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- partition-absolute: intended operation reached.

- provider-lifetime: intended operation reached.

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation phase-7

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("eval" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result:

```text-visible
phase rejected value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL phase-7} before CALLER_BEFORE after {CALLER_WRITE phase-7} level 3} caller {CALLER_WRITE phase-7}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase rejected action read provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}} level 3} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}}⟦EOF⟧
```

- connection-identity-write-read, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- connection-identity-write-read, lookup, completion 0, result:

```text-visible
cell {CONNECTION_SEED [redacted-peer-address] 55369} peer [redacted-peer-address] 55369 unit 0:0⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- connection-identity-write-read: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- partition-absolute: intended operation reached.

- provider-lifetime: intended operation reached.

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation phase-8

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("eval" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result:

```text-visible
phase rejected value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL phase-8} before CALLER_BEFORE after {CALLER_WRITE phase-8} level 3} caller {CALLER_WRITE phase-8}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase rejected action read provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}} level 3} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}}⟦EOF⟧
```

- connection-identity-write-read, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- connection-identity-write-read, lookup, completion 0, result:

```text-visible
cell {CONNECTION_SEED [redacted-peer-address] 55371} peer [redacted-peer-address] 55371 unit 0:0⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- connection-identity-write-read: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- partition-absolute: intended operation reached.

- provider-lifetime: intended operation reached.

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation phase-9

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 0, result:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- partition-absolute, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result:

```text-visible
owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("eval" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result:

```text-visible
phase detached value {owner NEW argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL phase-9} before CALLER_BEFORE after {CALLER_WRITE phase-9} level 3} caller {CALLER_WRITE phase-9}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase detached action read provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}} level 3} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}}⟦EOF⟧
```

- connection-identity-write-read, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- connection-identity-write-read, lookup, completion 0, result:

```text-visible
cell {CONNECTION_SEED [redacted-peer-address] 55375} peer [redacted-peer-address] 55375 unit 0:0⟦EOF⟧
```

- bare-local: intended operation reached.

- computed-list-target: intended operation reached.

- computed-variable-target: intended operation reached.

- connection-identity-write-read: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- list-prefix-arguments: intended operation reached.

- literal-matched-arguments: intended operation reached.

- missing-proc: intended operation reached.

- missing-rule: intended operation reached.

- other-owner: intended operation reached.

- partition-absolute: intended operation reached.

- provider-lifetime: intended operation reached.

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.
