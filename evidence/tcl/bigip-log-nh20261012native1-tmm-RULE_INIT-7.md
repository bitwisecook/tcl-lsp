# BIG-IP Tcl log: nh20261012native1, tmm-RULE_INIT-7

## Question

The following cases investigate Tcl behavior relevant to name resolution and diagnostics in the LSP, compiler and VM. Each observation is limited to its own case and recorded context.

- **bare-local:** For identical procedure tails in isolated rules and partitions, what does call select through absolute partition/rule::proc, relative rule::proc and a bare local proc?

- **computed-list-target:** Does a variable/list-built call target with exactly the literal-control bytes reach the same rule procedure?

- **computed-variable-target:** Does a variable/list-built call target with exactly the literal-control bytes reach the same rule procedure?

- **folder-absolute:** For identical procedure tails in isolated rules and partitions, what does call select through absolute partition/rule::proc, relative rule::proc and a bare local proc?

- **folder-leaf-relative:** For identical procedure tails in isolated rules and partitions, what does call select through absolute partition/rule::proc, relative rule::proc and a bare local proc?

- **list-prefix-arguments:** How does call pass fixed/list-expanded arguments containing spaces, braces and namespace separators?

- **literal-matched-arguments:** How does call pass fixed/list-expanded arguments containing spaces, braces and namespace separators?

- **missing-proc:** For identical procedure tails in isolated rules and partitions, what does call select through absolute partition/rule::proc, relative rule::proc and a bare local proc?

- **missing-rule:** For identical procedure tails in isolated rules and partitions, what does call select through absolute partition/rule::proc, relative rule::proc and a bare local proc?

- **other-owner:** For identical procedure tails in isolated rules and partitions, what does call select through absolute partition/rule::proc, relative rule::proc and a bare local proc?

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
- **folder-absolute:** New exact native materialisation. Actual context/event/unit and administrative phase retained. Nested completions are list data. Fresh observer activation is distinct from continuing traffic caller; no suspended-frame claim.
- **folder-leaf-relative:** New exact native materialisation. Actual context/event/unit and administrative phase retained. Nested completions are list data. Fresh observer activation is distinct from continuing traffic caller; no suspended-frame claim.
- **list-prefix-arguments:** New exact native materialisation. Actual context/event/unit and administrative phase retained. Nested completions are list data. Fresh observer activation is distinct from continuing traffic caller; no suspended-frame claim.
- **literal-matched-arguments:** New exact native materialisation. Actual context/event/unit and administrative phase retained. Nested completions are list data. Fresh observer activation is distinct from continuing traffic caller; no suspended-frame claim.
- **missing-proc:** New exact native materialisation. Actual context/event/unit and administrative phase retained. Nested completions are list data. Fresh observer activation is distinct from continuing traffic caller; no suspended-frame claim.
- **missing-rule:** New exact native materialisation. Actual context/event/unit and administrative phase retained. Nested completions are list data. Fresh observer activation is distinct from continuing traffic caller; no suspended-frame claim.
- **other-owner:** New exact native materialisation. Actual context/event/unit and administrative phase retained. Nested completions are list data. Fresh observer activation is distinct from continuing traffic caller; no suspended-frame claim.
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
ltm rule /Common/__tcl_lsp_probe_nh20261012native1_folder/__tcl_lsp_probe_nh20261012native1_observer_recreated {
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
set nh_phase recreated
set nh_invocation recreated-init
set nh_action read
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
call measure $nh_phase recreated-init-procedure read
}
}
```

## Results

### tmm-RULE_INIT, unit 0:0, invocation recreated-init

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
NHO|nh20261012native1|bare-local|tmm-RULE_INIT|setup|0|0:0|recreated-init|result|binary-scan-tcl-value|0|1|
NHO|nh20261012native1|bare-local|tmm-RULE_INIT|lookup|0|0:0|recreated-init|result|binary-scan-tcl-value|0|1|6f776e65722043414c4c4552206172676320322061726776207b7b6120627d207b7b787d203a3a7461696c7d7d
NHO|nh20261012native1|root-relative|tmm-RULE_INIT|setup|0|0:0|recreated-init|result|binary-scan-tcl-value|0|1|
NHO|nh20261012native1|root-relative|tmm-RULE_INIT|lookup|0|0:0|recreated-init|result|binary-scan-tcl-value|0|1|6f776e657220524543524541544544206172676320322061726776207b7b6120627d207b7b787d203a3a7461696c7d7d20617267762d686578207b3631323036322037623738376432303361336137343631363936637d
NHO|nh20261012native1|root-absolute|tmm-RULE_INIT|setup|0|0:0|recreated-init|result|binary-scan-tcl-value|0|1|
NHO|nh20261012native1|root-absolute|tmm-RULE_INIT|lookup|0|0:0|recreated-init|result|binary-scan-tcl-value|0|1|6f776e657220524543524541544544206172676320322061726776207b7b6120627d207b7b787d203a3a7461696c7d7d20617267762d686578207b3631323036322037623738376432303361336137343631363936637d
NHO|nh20261012native1|other-owner|tmm-RULE_INIT|setup|0|0:0|recreated-init|result|binary-scan-tcl-value|0|1|
NHO|nh20261012native1|other-owner|tmm-RULE_INIT|lookup|0|0:0|recreated-init|result|binary-scan-tcl-value|0|1|6f776e6572204f54484552206172676320322061726776207b7b6120627d207b7b787d203a3a7461696c7d7d20617267762d686578207b3631323036322037623738376432303361336137343631363936637d
NHO|nh20261012native1|folder-absolute|tmm-RULE_INIT|setup|0|0:0|recreated-init|result|binary-scan-tcl-value|0|1|
NHO|nh20261012native1|folder-absolute|tmm-RULE_INIT|lookup|0|0:0|recreated-init|result|binary-scan-tcl-value|0|1|6f776e657220464f4c444552206172676320322061726776207b7b6120627d207b7b787d203a3a7461696c7d7d20617267762d686578207b3631323036322037623738376432303361336137343631363936637d
NHO|nh20261012native1|folder-leaf-relative|tmm-RULE_INIT|setup|0|0:0|recreated-init|result|binary-scan-tcl-value|0|1|
NHO|nh20261012native1|folder-leaf-relative|tmm-RULE_INIT|lookup|1|0:0|recreated-init|result|binary-scan-tcl-value|0|2|70726f63205f5f6e685f6e6832303236313031326e6174697665315f746172676574206e6f7420666f756e6420286c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a2263616c6c205f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f666f6c6465643a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574207b6120627d207b7b787d203a3a7461696c7d220a2020202028226576616c2220626f6479206c696e65203129
NHO|nh20261012native1|folder-leaf-relative|tmm-RULE_INIT|lookup|1|0:0|recreated-init|result|binary-scan-tcl-value|1|2|0a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
NHO|nh20261012native1|folder-leaf-relative|tmm-RULE_INIT|lookup|1|0:0|recreated-init|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result, chunk 0 of 1:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result, chunk 0 of 1:

```text-visible
owner RECREATED argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result, chunk 0 of 1:

```text-visible
owner RECREATED argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result, chunk 0 of 1:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result, chunk 0 of 1:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 1, result, chunk 0 of 2:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call __tcl_lsp_probe_nh20261012native1_folded::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 1, result, chunk 1 of 2:

```text-visible

    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- bare-local: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- other-owner: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

### tmm-RULE_INIT, unit 0:1, invocation recreated-init

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
NHO|nh20261012native1|bare-local|tmm-RULE_INIT|setup|0|0:1|recreated-init|result|binary-scan-tcl-value|0|1|
NHO|nh20261012native1|bare-local|tmm-RULE_INIT|lookup|0|0:1|recreated-init|result|binary-scan-tcl-value|0|1|6f776e65722043414c4c4552206172676320322061726776207b7b6120627d207b7b787d203a3a7461696c7d7d
NHO|nh20261012native1|root-relative|tmm-RULE_INIT|setup|0|0:1|recreated-init|result|binary-scan-tcl-value|0|1|
NHO|nh20261012native1|root-relative|tmm-RULE_INIT|lookup|0|0:1|recreated-init|result|binary-scan-tcl-value|0|1|6f776e657220524543524541544544206172676320322061726776207b7b6120627d207b7b787d203a3a7461696c7d7d20617267762d686578207b3631323036322037623738376432303361336137343631363936637d
NHO|nh20261012native1|root-absolute|tmm-RULE_INIT|setup|0|0:1|recreated-init|result|binary-scan-tcl-value|0|1|
NHO|nh20261012native1|root-absolute|tmm-RULE_INIT|lookup|0|0:1|recreated-init|result|binary-scan-tcl-value|0|1|6f776e657220524543524541544544206172676320322061726776207b7b6120627d207b7b787d203a3a7461696c7d7d20617267762d686578207b3631323036322037623738376432303361336137343631363936637d
NHO|nh20261012native1|other-owner|tmm-RULE_INIT|setup|0|0:1|recreated-init|result|binary-scan-tcl-value|0|1|
NHO|nh20261012native1|other-owner|tmm-RULE_INIT|lookup|0|0:1|recreated-init|result|binary-scan-tcl-value|0|1|6f776e6572204f54484552206172676320322061726776207b7b6120627d207b7b787d203a3a7461696c7d7d20617267762d686578207b3631323036322037623738376432303361336137343631363936637d
NHO|nh20261012native1|folder-absolute|tmm-RULE_INIT|setup|0|0:1|recreated-init|result|binary-scan-tcl-value|0|1|
NHO|nh20261012native1|folder-absolute|tmm-RULE_INIT|lookup|0|0:1|recreated-init|result|binary-scan-tcl-value|0|1|6f776e657220464f4c444552206172676320322061726776207b7b6120627d207b7b787d203a3a7461696c7d7d20617267762d686578207b3631323036322037623738376432303361336137343631363936637d
NHO|nh20261012native1|folder-leaf-relative|tmm-RULE_INIT|setup|0|0:1|recreated-init|result|binary-scan-tcl-value|0|1|
NHO|nh20261012native1|folder-leaf-relative|tmm-RULE_INIT|lookup|1|0:1|recreated-init|result|binary-scan-tcl-value|0|2|70726f63205f5f6e685f6e6832303236313031326e6174697665315f746172676574206e6f7420666f756e6420286c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a2263616c6c205f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f666f6c6465643a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574207b6120627d207b7b787d203a3a7461696c7d220a2020202028226576616c2220626f6479206c696e65203129
NHO|nh20261012native1|folder-leaf-relative|tmm-RULE_INIT|lookup|1|0:1|recreated-init|result|binary-scan-tcl-value|1|2|0a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
NHO|nh20261012native1|folder-leaf-relative|tmm-RULE_INIT|lookup|1|0:1|recreated-init|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
NHO|nh20261012native1|folder-leaf-relative|tmm-RULE_INIT|lookup|1|0:1|recreated-init|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
NHO|nh20261012native1|folder-leaf-relative|tmm-RULE_INIT|lookup|1|0:1|recreated-init|errorinfo|binary-scan-tcl-value|0|2|70726f63205f5f6e685f6e6832303236313031326e6174697665315f746172676574206e6f7420666f756e6420286c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a2263616c6c205f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f666f6c6465643a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574207b6120627d207b7b787d203a3a7461696c7d220a2020202028226576616c2220626f6479206c696e65203129
NHO|nh20261012native1|folder-leaf-relative|tmm-RULE_INIT|lookup|1|0:1|recreated-init|errorinfo|binary-scan-tcl-value|1|2|0a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
NHO|nh20261012native1|partition-absolute|tmm-RULE_INIT|setup|0|0:1|recreated-init|result|binary-scan-tcl-value|0|1|
NHO|nh20261012native1|partition-absolute|tmm-RULE_INIT|lookup|0|0:1|recreated-init|result|binary-scan-tcl-value|0|1|6f776e657220504152544954494f4e206172676320322061726776207b7b6120627d207b7b787d203a3a7461696c7d7d20617267762d686578207b3631323036322037623738376432303361336137343631363936637d
NHO|nh20261012native1|missing-rule|tmm-RULE_INIT|setup|0|0:1|recreated-init|result|binary-scan-tcl-value|0|1|
NHO|nh20261012native1|missing-rule|tmm-RULE_INIT|lookup|1|0:1|recreated-init|result|binary-scan-tcl-value|0|2|70726f63205f5f6e685f6e6832303236313031326e6174697665315f746172676574206e6f7420666f756e6420286c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a2263616c6c202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f616273656e743a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574207b6120627d207b7b787d203a3a7461696c7d220a2020202028226576616c2220626f6479
NHO|nh20261012native1|missing-rule|tmm-RULE_INIT|lookup|1|0:1|recreated-init|result|binary-scan-tcl-value|1|2|206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
NHO|nh20261012native1|missing-rule|tmm-RULE_INIT|lookup|1|0:1|recreated-init|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
NHO|nh20261012native1|missing-rule|tmm-RULE_INIT|lookup|1|0:1|recreated-init|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
NHO|nh20261012native1|missing-rule|tmm-RULE_INIT|lookup|1|0:1|recreated-init|errorinfo|binary-scan-tcl-value|0|2|70726f63205f5f6e685f6e6832303236313031326e6174697665315f746172676574206e6f7420666f756e6420286c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a2263616c6c202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f616273656e743a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574207b6120627d207b7b787d203a3a7461696c7d220a2020202028226576616c2220626f6479
NHO|nh20261012native1|missing-rule|tmm-RULE_INIT|lookup|1|0:1|recreated-init|errorinfo|binary-scan-tcl-value|1|2|206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
NHO|nh20261012native1|missing-proc|tmm-RULE_INIT|setup|0|0:1|recreated-init|result|binary-scan-tcl-value|0|1|
NHO|nh20261012native1|missing-proc|tmm-RULE_INIT|lookup|1|0:1|recreated-init|result|binary-scan-tcl-value|0|2|70726f63205f5f6e685f6e6832303236313031326e6174697665315f616273656e74206e6f7420666f756e6420286c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a2263616c6c202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f616273656e74207b6120627d207b7b787d203a3a7461696c7d220a2020202028226576616c2220626f
NHO|nh20261012native1|missing-proc|tmm-RULE_INIT|lookup|1|0:1|recreated-init|result|binary-scan-tcl-value|1|2|6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
NHO|nh20261012native1|missing-proc|tmm-RULE_INIT|lookup|1|0:1|recreated-init|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
NHO|nh20261012native1|missing-proc|tmm-RULE_INIT|lookup|1|0:1|recreated-init|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
NHO|nh20261012native1|missing-proc|tmm-RULE_INIT|lookup|1|0:1|recreated-init|errorinfo|binary-scan-tcl-value|0|2|70726f63205f5f6e685f6e6832303236313031326e6174697665315f616273656e74206e6f7420666f756e6420286c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a2263616c6c202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f616273656e74207b6120627d207b7b787d203a3a7461696c7d220a2020202028226576616c2220626f
NHO|nh20261012native1|missing-proc|tmm-RULE_INIT|lookup|1|0:1|recreated-init|errorinfo|binary-scan-tcl-value|1|2|6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
NHO|nh20261012native1|computed-variable-target|tmm-RULE_INIT|setup|0|0:1|recreated-init|result|binary-scan-tcl-value|0|1|
NHO|nh20261012native1|computed-variable-target|tmm-RULE_INIT|lookup|0|0:1|recreated-init|result|binary-scan-tcl-value|0|2|7461726765742d686578203266343336663664366436663665326635663566373436333663356636633733373035663730373236663632363535663665363833323330333233363331333033313332366536313734363937363635333135663730373236663736363936343635373233613361356635663665363835663665363833323330333233363331333033313332366536313734363937363635333135663734363137323637363537342076616c7565207b6f776e65722052454352454154454420617267
NHO|nh20261012native1|computed-variable-target|tmm-RULE_INIT|lookup|0|0:1|recreated-init|result|binary-scan-tcl-value|1|2|6320322061726776207b7b6120627d207b7b787d203a3a7461696c7d7d20617267762d686578207b3631323036322037623738376432303361336137343631363936637d7d
NHO|nh20261012native1|computed-list-target|tmm-RULE_INIT|setup|0|0:1|recreated-init|result|binary-scan-tcl-value|0|1|
NHO|nh20261012native1|computed-list-target|tmm-RULE_INIT|lookup|0|0:1|recreated-init|result|binary-scan-tcl-value|0|2|7461726765742d686578203266343336663664366436663665326635663566373436333663356636633733373035663730373236663632363535663665363833323330333233363331333033313332366536313734363937363635333135663730373236663736363936343635373233613361356635663665363835663665363833323330333233363331333033313332366536313734363937363635333135663734363137323637363537342076616c7565207b6f776e65722052454352454154454420617267
NHO|nh20261012native1|computed-list-target|tmm-RULE_INIT|lookup|0|0:1|recreated-init|result|binary-scan-tcl-value|1|2|6320322061726776207b7b6120627d207b7b787d203a3a7461696c7d7d20617267762d686578207b3631323036322037623738376432303361336137343631363936637d7d
NHO|nh20261012native1|literal-matched-arguments|tmm-RULE_INIT|setup|0|0:1|recreated-init|result|binary-scan-tcl-value|0|1|
NHO|nh20261012native1|literal-matched-arguments|tmm-RULE_INIT|lookup|0|0:1|recreated-init|result|binary-scan-tcl-value|0|1|6f776e657220524543524541544544206172676320322061726776207b7b6120627d207b7b787d203a3a7461696c7d7d20617267762d686578207b3631323036322037623738376432303361336137343631363936637d
NHO|nh20261012native1|list-prefix-arguments|tmm-RULE_INIT|setup|0|0:1|recreated-init|result|binary-scan-tcl-value|0|1|
NHO|nh20261012native1|list-prefix-arguments|tmm-RULE_INIT|lookup|0|0:1|recreated-init|result|binary-scan-tcl-value|0|1|6f776e657220524543524541544544206172676320322061726776207b7b6120627d207b7b787d203a3a7461696c7d7d20617267762d686578207b3631323036322037623738376432303361336137343631363936637d
NHO|nh20261012native1|space-joined-near-miss|tmm-RULE_INIT|setup|0|0:1|recreated-init|result|binary-scan-tcl-value|0|1|
NHO|nh20261012native1|space-joined-near-miss|tmm-RULE_INIT|lookup|1|0:1|recreated-init|result|binary-scan-tcl-value|0|1|77726f6e67202320617267733a2073686f756c64206265202263616c6c205f5f6e685f6e6832303236313031326e6174697665315f746172676574203c66697865643e203c76616c75653e
NHO|nh20261012native1|space-joined-near-miss|tmm-RULE_INIT|lookup|1|0:1|recreated-init|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
NHO|nh20261012native1|space-joined-near-miss|tmm-RULE_INIT|lookup|1|0:1|recreated-init|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
NHO|nh20261012native1|space-joined-near-miss|tmm-RULE_INIT|lookup|1|0:1|recreated-init|errorinfo|binary-scan-tcl-value|0|2|77726f6e67202320617267733a2073686f756c64206265202263616c6c205f5f6e685f6e6832303236313031326e6174697665315f746172676574203c66697865643e203c76616c75653e0a202020207768696c6520657865637574696e670a2263616c6c202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f7461726765742061206220632064220a20202020282265
NHO|nh20261012native1|space-joined-near-miss|tmm-RULE_INIT|lookup|1|0:1|recreated-init|errorinfo|binary-scan-tcl-value|1|2|76616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c2022247072656669782063206422220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
NHO|nh20261012native1|provider-lifetime|tmm-RULE_INIT|setup|0|0:1|recreated-init|result|binary-scan-tcl-value|0|1|
NHO|nh20261012native1|provider-lifetime|tmm-RULE_INIT|lookup|0|0:1|recreated-init|result|binary-scan-tcl-value|0|1|7068617365207265637265617465642076616c7565207b6f776e657220524543524541544544206172676320322061726776207b7b6120627d207b7b787d203a3a7461696c7d7d20617267762d686578207b3631323036322037623738376432303361336137343631363936637d7d
NHO|nh20261012native1|provider-local-upvar-write|tmm-RULE_INIT|setup|0|0:1|recreated-init|result|binary-scan-tcl-value|0|1|
NHO|nh20261012native1|provider-local-upvar-write|tmm-RULE_INIT|lookup|0|0:1|recreated-init|result|binary-scan-tcl-value|0|1|70726f7669646572207b6c6f63616c207b50524f56494445525f4c4f43414c207265637265617465642d696e69747d206265666f72652043414c4c45525f4245464f5245206166746572207b43414c4c45525f5752495445207265637265617465642d696e69747d206c6576656c20317d2063616c6c6572207b43414c4c45525f5752495445207265637265617465642d696e69747d
NHO|nh20261012native1|provider-shared-write-readback|tmm-RULE_INIT|setup|0|0:1|recreated-init|result|binary-scan-tcl-value|0|1|
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result, chunk 0 of 1:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result, chunk 0 of 1:

```text-visible
owner RECREATED argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result, chunk 0 of 1:

```text-visible
owner RECREATED argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result, chunk 0 of 1:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result, chunk 0 of 1:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 1, result, chunk 0 of 2:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call __tcl_lsp_probe_nh20261012native1_folded::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 1, result, chunk 1 of 2:

```text-visible

    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call __tcl_lsp_probe_nh20261012native1_folded::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible

    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- partition-absolute, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result, chunk 0 of 1:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result, chunk 0 of 2:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body⟦EOF⟧
```

- missing-rule, lookup, completion 1, result, chunk 1 of 2:

```text-visible
 line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
 line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result, chunk 0 of 2:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" bo⟦EOF⟧
```

- missing-proc, lookup, completion 1, result, chunk 1 of 2:

```text-visible
dy line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" bo⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
dy line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result, chunk 0 of 2:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner RECREATED arg⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result, chunk 1 of 2:

```text-visible
c 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result, chunk 0 of 2:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner RECREATED arg⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result, chunk 1 of 2:

```text-visible
c 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result, chunk 0 of 1:

```text-visible
owner RECREATED argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result, chunk 0 of 1:

```text-visible
owner RECREATED argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result, chunk 0 of 1:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("e⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
val" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result, chunk 0 of 1:

```text-visible
phase recreated value {owner RECREATED argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result, chunk 0 of 1:

```text-visible
provider {local {PROVIDER_LOCAL recreated-init} before CALLER_BEFORE after {CALLER_WRITE recreated-init} level 1} caller {CALLER_WRITE recreated-init}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
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

- provider-shared-write-readback: intended operation not recorded.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

- space-joined-near-miss: intended operation reached.

### tmm-RULE_INIT, unit 0:2, invocation recreated-init

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
NHO|nh20261012native1|bare-local|tmm-RULE_INIT|setup|0|0:2|recreated-init|result|binary-scan-tcl-value|0|1|
NHO|nh20261012native1|bare-local|tmm-RULE_INIT|lookup|0|0:2|recreated-init|result|binary-scan-tcl-value|0|1|6f776e65722043414c4c4552206172676320322061726776207b7b6120627d207b7b787d203a3a7461696c7d7d
NHO|nh20261012native1|root-relative|tmm-RULE_INIT|setup|0|0:2|recreated-init|result|binary-scan-tcl-value|0|1|
NHO|nh20261012native1|root-relative|tmm-RULE_INIT|lookup|0|0:2|recreated-init|result|binary-scan-tcl-value|0|1|6f776e657220524543524541544544206172676320322061726776207b7b6120627d207b7b787d203a3a7461696c7d7d20617267762d686578207b3631323036322037623738376432303361336137343631363936637d
NHO|nh20261012native1|root-absolute|tmm-RULE_INIT|setup|0|0:2|recreated-init|result|binary-scan-tcl-value|0|1|
NHO|nh20261012native1|root-absolute|tmm-RULE_INIT|lookup|0|0:2|recreated-init|result|binary-scan-tcl-value|0|1|6f776e657220524543524541544544206172676320322061726776207b7b6120627d207b7b787d203a3a7461696c7d7d20617267762d686578207b3631323036322037623738376432303361336137343631363936637d
NHO|nh20261012native1|other-owner|tmm-RULE_INIT|setup|0|0:2|recreated-init|result|binary-scan-tcl-value|0|1|
NHO|nh20261012native1|other-owner|tmm-RULE_INIT|lookup|0|0:2|recreated-init|result|binary-scan-tcl-value|0|1|6f776e6572204f54484552206172676320322061726776207b7b6120627d207b7b787d203a3a7461696c7d7d20617267762d686578207b3631323036322037623738376432303361336137343631363936637d
NHO|nh20261012native1|folder-absolute|tmm-RULE_INIT|setup|0|0:2|recreated-init|result|binary-scan-tcl-value|0|1|
NHO|nh20261012native1|folder-absolute|tmm-RULE_INIT|lookup|0|0:2|recreated-init|result|binary-scan-tcl-value|0|1|6f776e657220464f4c444552206172676320322061726776207b7b6120627d207b7b787d203a3a7461696c7d7d20617267762d686578207b3631323036322037623738376432303361336137343631363936637d
NHO|nh20261012native1|folder-leaf-relative|tmm-RULE_INIT|setup|0|0:2|recreated-init|result|binary-scan-tcl-value|0|1|
NHO|nh20261012native1|folder-leaf-relative|tmm-RULE_INIT|lookup|1|0:2|recreated-init|result|binary-scan-tcl-value|0|2|70726f63205f5f6e685f6e6832303236313031326e6174697665315f746172676574206e6f7420666f756e6420286c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a2263616c6c205f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f666f6c6465643a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574207b6120627d207b7b787d203a3a7461696c7d220a2020202028226576616c2220626f6479206c696e65203129
NHO|nh20261012native1|folder-leaf-relative|tmm-RULE_INIT|lookup|1|0:2|recreated-init|result|binary-scan-tcl-value|1|2|0a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
NHO|nh20261012native1|folder-leaf-relative|tmm-RULE_INIT|lookup|1|0:2|recreated-init|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
NHO|nh20261012native1|folder-leaf-relative|tmm-RULE_INIT|lookup|1|0:2|recreated-init|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
NHO|nh20261012native1|folder-leaf-relative|tmm-RULE_INIT|lookup|1|0:2|recreated-init|errorinfo|binary-scan-tcl-value|0|2|70726f63205f5f6e685f6e6832303236313031326e6174697665315f746172676574206e6f7420666f756e6420286c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a2263616c6c205f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f666f6c6465643a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574207b6120627d207b7b787d203a3a7461696c7d220a2020202028226576616c2220626f6479206c696e65203129
NHO|nh20261012native1|folder-leaf-relative|tmm-RULE_INIT|lookup|1|0:2|recreated-init|errorinfo|binary-scan-tcl-value|1|2|0a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
NHO|nh20261012native1|partition-absolute|tmm-RULE_INIT|setup|0|0:2|recreated-init|result|binary-scan-tcl-value|0|1|
NHO|nh20261012native1|partition-absolute|tmm-RULE_INIT|lookup|0|0:2|recreated-init|result|binary-scan-tcl-value|0|1|6f776e657220504152544954494f4e206172676320322061726776207b7b6120627d207b7b787d203a3a7461696c7d7d20617267762d686578207b3631323036322037623738376432303361336137343631363936637d
NHO|nh20261012native1|missing-rule|tmm-RULE_INIT|setup|0|0:2|recreated-init|result|binary-scan-tcl-value|0|1|
NHO|nh20261012native1|missing-rule|tmm-RULE_INIT|lookup|1|0:2|recreated-init|result|binary-scan-tcl-value|0|2|70726f63205f5f6e685f6e6832303236313031326e6174697665315f746172676574206e6f7420666f756e6420286c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a2263616c6c202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f616273656e743a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574207b6120627d207b7b787d203a3a7461696c7d220a2020202028226576616c2220626f6479
NHO|nh20261012native1|missing-rule|tmm-RULE_INIT|lookup|1|0:2|recreated-init|result|binary-scan-tcl-value|1|2|206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
NHO|nh20261012native1|missing-rule|tmm-RULE_INIT|lookup|1|0:2|recreated-init|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
NHO|nh20261012native1|missing-rule|tmm-RULE_INIT|lookup|1|0:2|recreated-init|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
NHO|nh20261012native1|missing-rule|tmm-RULE_INIT|lookup|1|0:2|recreated-init|errorinfo|binary-scan-tcl-value|0|2|70726f63205f5f6e685f6e6832303236313031326e6174697665315f746172676574206e6f7420666f756e6420286c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a2263616c6c202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f616273656e743a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574207b6120627d207b7b787d203a3a7461696c7d220a2020202028226576616c2220626f6479
NHO|nh20261012native1|missing-rule|tmm-RULE_INIT|lookup|1|0:2|recreated-init|errorinfo|binary-scan-tcl-value|1|2|206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
NHO|nh20261012native1|missing-proc|tmm-RULE_INIT|setup|0|0:2|recreated-init|result|binary-scan-tcl-value|0|1|
NHO|nh20261012native1|missing-proc|tmm-RULE_INIT|lookup|1|0:2|recreated-init|result|binary-scan-tcl-value|0|2|70726f63205f5f6e685f6e6832303236313031326e6174697665315f616273656e74206e6f7420666f756e6420286c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a2263616c6c202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f616273656e74207b6120627d207b7b787d203a3a7461696c7d220a2020202028226576616c2220626f
NHO|nh20261012native1|missing-proc|tmm-RULE_INIT|lookup|1|0:2|recreated-init|result|binary-scan-tcl-value|1|2|6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
NHO|nh20261012native1|missing-proc|tmm-RULE_INIT|lookup|1|0:2|recreated-init|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
NHO|nh20261012native1|missing-proc|tmm-RULE_INIT|lookup|1|0:2|recreated-init|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
NHO|nh20261012native1|missing-proc|tmm-RULE_INIT|lookup|1|0:2|recreated-init|errorinfo|binary-scan-tcl-value|0|2|70726f63205f5f6e685f6e6832303236313031326e6174697665315f616273656e74206e6f7420666f756e6420286c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a2263616c6c202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f616273656e74207b6120627d207b7b787d203a3a7461696c7d220a2020202028226576616c2220626f
NHO|nh20261012native1|missing-proc|tmm-RULE_INIT|lookup|1|0:2|recreated-init|errorinfo|binary-scan-tcl-value|1|2|6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
NHO|nh20261012native1|computed-variable-target|tmm-RULE_INIT|setup|0|0:2|recreated-init|result|binary-scan-tcl-value|0|1|
NHO|nh20261012native1|computed-variable-target|tmm-RULE_INIT|lookup|0|0:2|recreated-init|result|binary-scan-tcl-value|0|2|7461726765742d686578203266343336663664366436663665326635663566373436333663356636633733373035663730373236663632363535663665363833323330333233363331333033313332366536313734363937363635333135663730373236663736363936343635373233613361356635663665363835663665363833323330333233363331333033313332366536313734363937363635333135663734363137323637363537342076616c7565207b6f776e65722052454352454154454420617267
NHO|nh20261012native1|computed-variable-target|tmm-RULE_INIT|lookup|0|0:2|recreated-init|result|binary-scan-tcl-value|1|2|6320322061726776207b7b6120627d207b7b787d203a3a7461696c7d7d20617267762d686578207b3631323036322037623738376432303361336137343631363936637d7d
NHO|nh20261012native1|computed-list-target|tmm-RULE_INIT|setup|0|0:2|recreated-init|result|binary-scan-tcl-value|0|1|
NHO|nh20261012native1|computed-list-target|tmm-RULE_INIT|lookup|0|0:2|recreated-init|result|binary-scan-tcl-value|0|2|7461726765742d686578203266343336663664366436663665326635663566373436333663356636633733373035663730373236663632363535663665363833323330333233363331333033313332366536313734363937363635333135663730373236663736363936343635373233613361356635663665363835663665363833323330333233363331333033313332366536313734363937363635333135663734363137323637363537342076616c7565207b6f776e65722052454352454154454420617267
NHO|nh20261012native1|computed-list-target|tmm-RULE_INIT|lookup|0|0:2|recreated-init|result|binary-scan-tcl-value|1|2|6320322061726776207b7b6120627d207b7b787d203a3a7461696c7d7d20617267762d686578207b3631323036322037623738376432303361336137343631363936637d7d
NHO|nh20261012native1|literal-matched-arguments|tmm-RULE_INIT|setup|0|0:2|recreated-init|result|binary-scan-tcl-value|0|1|
NHO|nh20261012native1|literal-matched-arguments|tmm-RULE_INIT|lookup|0|0:2|recreated-init|result|binary-scan-tcl-value|0|1|6f776e657220524543524541544544206172676320322061726776207b7b6120627d207b7b787d203a3a7461696c7d7d20617267762d686578207b3631323036322037623738376432303361336137343631363936637d
NHO|nh20261012native1|list-prefix-arguments|tmm-RULE_INIT|setup|0|0:2|recreated-init|result|binary-scan-tcl-value|0|1|
NHO|nh20261012native1|list-prefix-arguments|tmm-RULE_INIT|lookup|0|0:2|recreated-init|result|binary-scan-tcl-value|0|1|6f776e657220524543524541544544206172676320322061726776207b7b6120627d207b7b787d203a3a7461696c7d7d20617267762d686578207b3631323036322037623738376432303361336137343631363936637d
NHO|nh20261012native1|space-joined-near-miss|tmm-RULE_INIT|setup|0|0:2|recreated-init|result|binary-scan-tcl-value|0|1|
NHO|nh20261012native1|space-joined-near-miss|tmm-RULE_INIT|lookup|1|0:2|recreated-init|result|binary-scan-tcl-value|0|1|77726f6e67202320617267733a2073686f756c64206265202263616c6c205f5f6e685f6e6832303236313031326e6174697665315f746172676574203c66697865643e203c76616c75653e
NHO|nh20261012native1|space-joined-near-miss|tmm-RULE_INIT|lookup|1|0:2|recreated-init|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
NHO|nh20261012native1|space-joined-near-miss|tmm-RULE_INIT|lookup|1|0:2|recreated-init|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
NHO|nh20261012native1|space-joined-near-miss|tmm-RULE_INIT|lookup|1|0:2|recreated-init|errorinfo|binary-scan-tcl-value|0|2|77726f6e67202320617267733a2073686f756c64206265202263616c6c205f5f6e685f6e6832303236313031326e6174697665315f746172676574203c66697865643e203c76616c75653e0a202020207768696c6520657865637574696e670a2263616c6c202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f7461726765742061206220632064220a20202020282265
NHO|nh20261012native1|space-joined-near-miss|tmm-RULE_INIT|lookup|1|0:2|recreated-init|errorinfo|binary-scan-tcl-value|1|2|76616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c2022247072656669782063206422220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
NHO|nh20261012native1|provider-lifetime|tmm-RULE_INIT|setup|0|0:2|recreated-init|result|binary-scan-tcl-value|0|1|
NHO|nh20261012native1|provider-lifetime|tmm-RULE_INIT|lookup|0|0:2|recreated-init|result|binary-scan-tcl-value|0|1|7068617365207265637265617465642076616c7565207b6f776e657220524543524541544544206172676320322061726776207b7b6120627d207b7b787d203a3a7461696c7d7d20617267762d686578207b3631323036322037623738376432303361336137343631363936637d7d
NHO|nh20261012native1|provider-local-upvar-write|tmm-RULE_INIT|setup|0|0:2|recreated-init|result|binary-scan-tcl-value|0|1|
NHO|nh20261012native1|provider-local-upvar-write|tmm-RULE_INIT|lookup|0|0:2|recreated-init|result|binary-scan-tcl-value|0|1|70726f7669646572207b6c6f63616c207b50524f56494445525f4c4f43414c207265637265617465642d696e69747d206265666f72652043414c4c45525f4245464f5245206166746572207b43414c4c45525f5752495445207265637265617465642d696e69747d206c6576656c20317d2063616c6c6572207b43414c4c45525f5752495445207265637265617465642d696e69747d
NHO|nh20261012native1|provider-shared-write-readback|tmm-RULE_INIT|setup|0|0:2|recreated-init|result|binary-scan-tcl-value|0|1|
NHO|nh20261012native1|provider-shared-write-readback|tmm-RULE_INIT|lookup|0|0:2|recreated-init|result|binary-scan-tcl-value|0|3|70686173652072656372656174656420616374696f6e20726561642070726f7669646572207b726f7773207b3a3a5f5f6e685f6e6832303236313031326e6174697665313a3a63656c6c2030207b4e414d4553504143455f5752495445206c6f61642d6f6c642d696e69742d70726f6365647572657d203a3a5f5f6e685f6e6832303236313031326e6174697665315f676c6f62616c2030207b474c4f42414c5f5752495445206c6f61642d6f6c642d696e69742d70726f6365647572657d207374617469633a3a
NHO|nh20261012native1|provider-shared-write-readback|tmm-RULE_INIT|lookup|0|0:2|recreated-init|result|binary-scan-tcl-value|1|3|5f5f6e685f6e6832303236313031326e6174697665315f7374617469632030207b5354415449435f5752495445206c6f61642d6f6c642d696e69742d70726f6365647572657d7d206c6576656c20317d2063616c6c6572207b3a3a5f5f6e685f6e6832303236313031326e6174697665313a3a63656c6c2030207b4e414d4553504143455f5752495445206c6f61642d6f6c642d696e69742d70726f6365647572657d203a3a5f5f6e685f6e6832303236313031326e6174697665315f676c6f62616c2030207b47
NHO|nh20261012native1|provider-shared-write-readback|tmm-RULE_INIT|lookup|0|0:2|recreated-init|result|binary-scan-tcl-value|2|3|4c4f42414c5f5752495445206c6f61642d6f6c642d696e69742d70726f6365647572657d207374617469633a3a5f5f6e685f6e6832303236313031326e6174697665315f7374617469632030207b5354415449435f5752495445206c6f61642d6f6c642d696e69742d70726f6365647572657d7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result, chunk 0 of 1:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result, chunk 0 of 1:

```text-visible
owner RECREATED argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result, chunk 0 of 1:

```text-visible
owner RECREATED argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result, chunk 0 of 1:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result, chunk 0 of 1:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 1, result, chunk 0 of 2:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call __tcl_lsp_probe_nh20261012native1_folded::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 1, result, chunk 1 of 2:

```text-visible

    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call __tcl_lsp_probe_nh20261012native1_folded::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible

    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- partition-absolute, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result, chunk 0 of 1:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- missing-rule, lookup, completion 1, result, chunk 0 of 2:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body⟦EOF⟧
```

- missing-rule, lookup, completion 1, result, chunk 1 of 2:

```text-visible
 line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-rule, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_absent::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body⟦EOF⟧
```

- missing-rule, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
 line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- missing-proc, lookup, completion 1, result, chunk 0 of 2:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" bo⟦EOF⟧
```

- missing-proc, lookup, completion 1, result, chunk 1 of 2:

```text-visible
dy line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- missing-proc, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
proc __nh_nh20261012native1_absent not found (line 1)
    invoked from within
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_absent {a b} {{x} ::tail}"
    ("eval" bo⟦EOF⟧
```

- missing-proc, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
dy line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- computed-variable-target, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result, chunk 0 of 2:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner RECREATED arg⟦EOF⟧
```

- computed-variable-target, lookup, completion 0, result, chunk 1 of 2:

```text-visible
c 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- computed-list-target, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result, chunk 0 of 2:

```text-visible
target-hex 2f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574 value {owner RECREATED arg⟦EOF⟧
```

- computed-list-target, lookup, completion 0, result, chunk 1 of 2:

```text-visible
c 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- literal-matched-arguments, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- literal-matched-arguments, lookup, completion 0, result, chunk 0 of 1:

```text-visible
owner RECREATED argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- list-prefix-arguments, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- list-prefix-arguments, lookup, completion 0, result, chunk 0 of 1:

```text-visible
owner RECREATED argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- space-joined-near-miss, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, result, chunk 0 of 1:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
wrong # args: should be "call __nh_nh20261012native1_target <fixed> <value>
    while executing
"call /Common/__tcl_lsp_probe_nh20261012native1_provider::__nh_nh20261012native1_target a b c d"
    ("e⟦EOF⟧
```

- space-joined-near-miss, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
val" body line 1)
    invoked from within
"eval "$prefix c d""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- provider-lifetime, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- provider-lifetime, lookup, completion 0, result, chunk 0 of 1:

```text-visible
phase recreated value {owner RECREATED argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result, chunk 0 of 1:

```text-visible
provider {local {PROVIDER_LOCAL recreated-init} before CALLER_BEFORE after {CALLER_WRITE recreated-init} level 1} caller {CALLER_WRITE recreated-init}⟦EOF⟧
```

- provider-shared-write-readback, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result, chunk 0 of 3:

```text-visible
phase recreated action read provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE load-old-init-procedure} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE load-old-init-procedure} static::⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result, chunk 1 of 3:

```text-visible
__nh_nh20261012native1_static 0 {STATIC_WRITE load-old-init-procedure}} level 1} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE load-old-init-procedure} ::__nh_nh20261012native1_global 0 {G⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result, chunk 2 of 3:

```text-visible
LOBAL_WRITE load-old-init-procedure} static::__nh_nh20261012native1_static 0 {STATIC_WRITE load-old-init-procedure}}⟦EOF⟧
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

### tmm-RULE_INIT, unit 0:3, invocation recreated-init

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
NHO|nh20261012native1|bare-local|tmm-RULE_INIT|setup|0|0:3|recreated-init|result|binary-scan-tcl-value|0|1|
NHO|nh20261012native1|bare-local|tmm-RULE_INIT|lookup|0|0:3|recreated-init|result|binary-scan-tcl-value|0|1|6f776e65722043414c4c4552206172676320322061726776207b7b6120627d207b7b787d203a3a7461696c7d7d
NHO|nh20261012native1|root-relative|tmm-RULE_INIT|setup|0|0:3|recreated-init|result|binary-scan-tcl-value|0|1|
NHO|nh20261012native1|root-relative|tmm-RULE_INIT|lookup|0|0:3|recreated-init|result|binary-scan-tcl-value|0|1|6f776e657220524543524541544544206172676320322061726776207b7b6120627d207b7b787d203a3a7461696c7d7d20617267762d686578207b3631323036322037623738376432303361336137343631363936637d
NHO|nh20261012native1|root-absolute|tmm-RULE_INIT|setup|0|0:3|recreated-init|result|binary-scan-tcl-value|0|1|
NHO|nh20261012native1|root-absolute|tmm-RULE_INIT|lookup|0|0:3|recreated-init|result|binary-scan-tcl-value|0|1|6f776e657220524543524541544544206172676320322061726776207b7b6120627d207b7b787d203a3a7461696c7d7d20617267762d686578207b3631323036322037623738376432303361336137343631363936637d
NHO|nh20261012native1|other-owner|tmm-RULE_INIT|setup|0|0:3|recreated-init|result|binary-scan-tcl-value|0|1|
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result, chunk 0 of 1:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result, chunk 0 of 1:

```text-visible
owner RECREATED argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result, chunk 0 of 1:

```text-visible
owner RECREATED argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- bare-local: intended operation reached.

- other-owner: intended operation not recorded.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.

### tmm-procedure, unit 0:0, invocation recreated-init-procedure

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-procedure.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
NHO|nh20261012native1|provider-lifetime|tmm-procedure|lookup|0|0:0|recreated-init-procedure|result|binary-scan-tcl-value|0|1|7068617365207265637265617465642076616c7565207b6f776e657220524543524541544544206172676320322061726776207b7b6120627d207b7b787d203a3a7461696c7d7d20617267762d686578207b3631323036322037623738376432303361336137343631363936637d7d
NHO|nh20261012native1|provider-local-upvar-write|tmm-procedure|setup|0|0:0|recreated-init-procedure|result|binary-scan-tcl-value|0|1|
NHO|nh20261012native1|provider-local-upvar-write|tmm-procedure|lookup|0|0:0|recreated-init-procedure|result|binary-scan-tcl-value|0|1|70726f7669646572207b6c6f63616c207b50524f56494445525f4c4f43414c207265637265617465642d696e69742d70726f6365647572657d206265666f72652043414c4c45525f4245464f5245206166746572207b43414c4c45525f5752495445207265637265617465642d696e69742d70726f6365647572657d206c6576656c20327d2063616c6c6572207b43414c4c45525f5752495445207265637265617465642d696e69742d70726f6365647572657d
NHO|nh20261012native1|provider-shared-write-readback|tmm-procedure|lookup|0|0:0|recreated-init-procedure|result|binary-scan-tcl-value|0|3|70686173652072656372656174656420616374696f6e20726561642070726f7669646572207b726f7773207b3a3a5f5f6e685f6e6832303236313031326e6174697665313a3a63656c6c2030207b4e414d4553504143455f575249544520706169722d612d70726f76696465722d77726974657d203a3a5f5f6e685f6e6832303236313031326e6174697665315f676c6f62616c2030207b474c4f42414c5f575249544520706169722d612d70726f76696465722d77726974657d207374617469633a3a5f5f6e68
NHO|nh20261012native1|provider-shared-write-readback|tmm-procedure|lookup|0|0:0|recreated-init-procedure|result|binary-scan-tcl-value|1|3|5f6e6832303236313031326e6174697665315f7374617469632030207b5354415449435f575249544520706169722d612d70726f76696465722d77726974657d7d206c6576656c20327d2063616c6c6572207b3a3a5f5f6e685f6e6832303236313031326e6174697665313a3a63656c6c2030207b4e414d4553504143455f575249544520706169722d612d70726f76696465722d77726974657d203a3a5f5f6e685f6e6832303236313031326e6174697665315f676c6f62616c2030207b474c4f42414c5f5752
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- provider-lifetime, lookup, completion 0, result, chunk 0 of 1:

```text-visible
phase recreated value {owner RECREATED argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}}⟦EOF⟧
```

- provider-local-upvar-write, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- provider-local-upvar-write, lookup, completion 0, result, chunk 0 of 1:

```text-visible
provider {local {PROVIDER_LOCAL recreated-init-procedure} before CALLER_BEFORE after {CALLER_WRITE recreated-init-procedure} level 2} caller {CALLER_WRITE recreated-init-procedure}⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result, chunk 0 of 3:

```text-visible
phase recreated action read provider {rows {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WRITE pair-a-provider-write} static::__nh⟦EOF⟧
```

- provider-shared-write-readback, lookup, completion 0, result, chunk 1 of 3:

```text-visible
_nh20261012native1_static 0 {STATIC_WRITE pair-a-provider-write}} level 2} caller {::__nh_nh20261012native1::cell 0 {NAMESPACE_WRITE pair-a-provider-write} ::__nh_nh20261012native1_global 0 {GLOBAL_WR⟦EOF⟧
```

- provider-lifetime: intended operation reached.

- provider-local-upvar-write: intended operation reached.

- provider-shared-write-readback: intended operation reached.

### tmm-procedure, unit 0:2, invocation recreated-init-procedure

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-procedure.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
NHO|nh20261012native1|bare-local|tmm-procedure|setup|0|0:2|recreated-init-procedure|result|binary-scan-tcl-value|0|1|
NHO|nh20261012native1|bare-local|tmm-procedure|lookup|0|0:2|recreated-init-procedure|result|binary-scan-tcl-value|0|1|6f776e65722043414c4c4552206172676320322061726776207b7b6120627d207b7b787d203a3a7461696c7d7d
NHO|nh20261012native1|root-relative|tmm-procedure|setup|0|0:2|recreated-init-procedure|result|binary-scan-tcl-value|0|1|
NHO|nh20261012native1|root-relative|tmm-procedure|lookup|0|0:2|recreated-init-procedure|result|binary-scan-tcl-value|0|1|6f776e657220524543524541544544206172676320322061726776207b7b6120627d207b7b787d203a3a7461696c7d7d20617267762d686578207b3631323036322037623738376432303361336137343631363936637d
NHO|nh20261012native1|root-absolute|tmm-procedure|setup|0|0:2|recreated-init-procedure|result|binary-scan-tcl-value|0|1|
NHO|nh20261012native1|root-absolute|tmm-procedure|lookup|0|0:2|recreated-init-procedure|result|binary-scan-tcl-value|0|1|6f776e657220524543524541544544206172676320322061726776207b7b6120627d207b7b787d203a3a7461696c7d7d20617267762d686578207b3631323036322037623738376432303361336137343631363936637d
NHO|nh20261012native1|other-owner|tmm-procedure|setup|0|0:2|recreated-init-procedure|result|binary-scan-tcl-value|0|1|
NHO|nh20261012native1|other-owner|tmm-procedure|lookup|0|0:2|recreated-init-procedure|result|binary-scan-tcl-value|0|1|6f776e6572204f54484552206172676320322061726776207b7b6120627d207b7b787d203a3a7461696c7d7d20617267762d686578207b3631323036322037623738376432303361336137343631363936637d
NHO|nh20261012native1|folder-absolute|tmm-procedure|setup|0|0:2|recreated-init-procedure|result|binary-scan-tcl-value|0|1|
NHO|nh20261012native1|folder-absolute|tmm-procedure|lookup|0|0:2|recreated-init-procedure|result|binary-scan-tcl-value|0|1|6f776e657220464f4c444552206172676320322061726776207b7b6120627d207b7b787d203a3a7461696c7d7d20617267762d686578207b3631323036322037623738376432303361336137343631363936637d
NHO|nh20261012native1|folder-leaf-relative|tmm-procedure|setup|0|0:2|recreated-init-procedure|result|binary-scan-tcl-value|0|1|
NHO|nh20261012native1|folder-leaf-relative|tmm-procedure|lookup|1|0:2|recreated-init-procedure|result|binary-scan-tcl-value|0|1|
NHO|nh20261012native1|folder-leaf-relative|tmm-procedure|lookup|1|0:2|recreated-init-procedure|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
NHO|nh20261012native1|folder-leaf-relative|tmm-procedure|lookup|1|0:2|recreated-init-procedure|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
NHO|nh20261012native1|folder-leaf-relative|tmm-procedure|lookup|1|0:2|recreated-init-procedure|errorinfo|binary-scan-tcl-value|0|2|70726f63205f5f6e685f6e6832303236313031326e6174697665315f746172676574206e6f7420666f756e6420286c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a2263616c6c205f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665315f666f6c6465643a3a5f5f6e685f6e6832303236313031326e6174697665315f746172676574207b6120627d207b7b787d203a3a7461696c7d220a2020202028226576616c2220626f6479206c696e65203129
NHO|nh20261012native1|folder-leaf-relative|tmm-procedure|lookup|1|0:2|recreated-init-procedure|errorinfo|binary-scan-tcl-value|1|2|0a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
NHO|nh20261012native1|partition-absolute|tmm-procedure|setup|0|0:2|recreated-init-procedure|result|binary-scan-tcl-value|0|1|
NHO|nh20261012native1|partition-absolute|tmm-procedure|lookup|0|0:2|recreated-init-procedure|result|binary-scan-tcl-value|0|1|6f776e657220504152544954494f4e206172676320322061726776207b7b6120627d207b7b787d203a3a7461696c7d7d20617267762d686578207b3631323036322037623738376432303361336137343631363936637d
NHO|nh20261012native1|missing-rule|tmm-procedure|setup|0|0:2|recreated-init-procedure|result|binary-scan-tcl-value|0|1|
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- bare-local, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- bare-local, lookup, completion 0, result, chunk 0 of 1:

```text-visible
owner CALLER argc 2 argv {{a b} {{x} ::tail}}⟦EOF⟧
```

- root-relative, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- root-relative, lookup, completion 0, result, chunk 0 of 1:

```text-visible
owner RECREATED argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- root-absolute, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- root-absolute, lookup, completion 0, result, chunk 0 of 1:

```text-visible
owner RECREATED argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- other-owner, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- other-owner, lookup, completion 0, result, chunk 0 of 1:

```text-visible
owner OTHER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-absolute, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- folder-absolute, lookup, completion 0, result, chunk 0 of 1:

```text-visible
owner FOLDER argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- folder-leaf-relative, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 1, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
proc __nh_nh20261012native1_target not found (line 1)
    invoked from within
"call __tcl_lsp_probe_nh20261012native1_folded::__nh_nh20261012native1_target {a b} {{x} ::tail}"
    ("eval" body line 1)⟦EOF⟧
```

- folder-leaf-relative, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible

    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- partition-absolute, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- partition-absolute, lookup, completion 0, result, chunk 0 of 1:

```text-visible
owner PARTITION argc 2 argv {{a b} {{x} ::tail}} argv-hex {612062 7b787d203a3a7461696c}⟦EOF⟧
```

- missing-rule, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- bare-local: intended operation reached.

- folder-absolute: intended operation reached.

- folder-leaf-relative: intended operation reached.

- missing-rule: intended operation not recorded.

- other-owner: intended operation reached.

- partition-absolute: intended operation reached.

- root-absolute: intended operation reached.

- root-relative: intended operation reached.
