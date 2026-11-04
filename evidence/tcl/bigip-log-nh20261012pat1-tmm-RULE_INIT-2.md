# BIG-IP Tcl log: nh20261012pat1, tmm-RULE_INIT-2

## Question

The following cases investigate Tcl behavior relevant to name resolution and diagnostics in the LSP, compiler and VM. Each observation is limited to its own case and recorded context.

- **frame-wrapper:** Does a forwarding procedure using uplevel 1 let a subsequent upvar 1 reach a spaced variable name in the original caller's frame?

- **frame-wrapper-near-miss:** When the same forwarding operation uses eval rather than uplevel 1, which caller frame does upvar 1 reach, and is the original caller's spaced variable changed?

- **keyed-variable-link:** Can upvar link to a namespace array whose constructed name contains a space, and does writing an element through the link update the intended namespace variable?

## Assumptions

- **frame-wrapper:** Adapted wrapper execution namespace/frame; not exact portable input. Setup includes all original commands before the final one.
- **frame-wrapper-near-miss:** Adapted wrapper execution namespace/frame; not exact portable input. Setup includes all original commands before the final one.
- **keyed-variable-link:** Adapted wrapper execution namespace/frame; not exact portable input. Setup includes all original commands before the final one.

Other assumptions were not separately recorded.

## Script

```tcl-visible
set nh_unit "[TMM::cmp_group]:[TMM::cmp_unit]"
set nh_invocation once
set nh_setup [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f6672616d655f77726170706572207b7d0a70726f63203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f6672616d655f777261707065725f666f7277617264207b7072656669787d207b75706c6576656c203120247072656669787d0a70726f63203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f6672616d655f777261707065725f6368616e6765207b6e616d657d207b7570766172203120246e616d652063656c6c3b207365742063656c6c206368616e6765647d0a70726f63203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f6672616d655f777261707065725f6f75746572207b7d207b0a20202020736574207b6120627d206f726967696e616c0a202020203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f6672616d655f777261707065725f666f7277617264205b6c697374203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f6672616d655f777261707065725f6368616e6765207b6120627d5d0a20202020736574207b6120627d0a7d0a]
set nh_operation [binary format H* 3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f6672616d655f777261707065725f6f757465720a]
set nh_setup_code [catch {eval $nh_setup} nh_setup_result]

set nh_ec {}
set nh_ei {}
set nh_ec_present 0
set nh_ei_present 0
if {$nh_setup_code == 1} {
    if {[info exists ::errorCode]} {set nh_ec $::errorCode; set nh_ec_present 1}
    if {[info exists ::errorInfo]} {set nh_ei $::errorInfo; set nh_ei_present 1}
}
foreach {nh_field nh_value} [list result $nh_setup_result metadata [list errorcode-present $nh_ec_present errorinfo-present $nh_ei_present return-options unavailable] errorcode $nh_ec errorinfo $nh_ei] {
    if {$nh_field != "result" && $nh_setup_code != 1} {continue}
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
        log local0. "NHO|nh20261012pat1|frame-wrapper|tmm-RULE_INIT|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
foreach {nh_field nh_value} [list result $nh_result metadata [list errorcode-present $nh_ec_present errorinfo-present $nh_ei_present return-options unavailable] errorcode $nh_ec errorinfo $nh_ei] {
    if {$nh_field != "result" && $nh_code != 1} {continue}
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
        log local0. "NHO|nh20261012pat1|frame-wrapper|tmm-RULE_INIT|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 6966207b5b6c6c656e677468205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f6672616d655f777261707065725f666f72776172645d5d7d207b72656e616d65203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f6672616d655f777261707065725f666f7277617264207b7d7d0a6966207b5b6c6c656e677468205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f6672616d655f777261707065725f6368616e67655d5d7d207b72656e616d65203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f6672616d655f777261707065725f6368616e6765207b7d7d0a6966207b5b6c6c656e677468205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f6672616d655f777261707065725f6f757465725d5d7d207b72656e616d65203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f6672616d655f777261707065725f6f75746572207b7d7d0a6966207b5b6e616d65737061636520657869737473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f6672616d655f777261707065725d7d207b6e616d6573706163652064656c657465203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f6672616d655f777261707065727d0a]
set nh_code [catch {eval $nh_cleanup} nh_result]

set nh_ec {}
set nh_ei {}
set nh_ec_present 0
set nh_ei_present 0
if {$nh_code == 1} {
    if {[info exists ::errorCode]} {set nh_ec $::errorCode; set nh_ec_present 1}
    if {[info exists ::errorInfo]} {set nh_ei $::errorInfo; set nh_ei_present 1}
}
foreach {nh_field nh_value} [list result $nh_result metadata [list errorcode-present $nh_ec_present errorinfo-present $nh_ei_present return-options unavailable] errorcode $nh_ec errorinfo $nh_ei] {
    if {$nh_field != "result" && $nh_code != 1} {continue}
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
        log local0. "NHO|nh20261012pat1|frame-wrapper|tmm-RULE_INIT|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
set nh_setup [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f6672616d655f777261707065725f6e6561725f6d697373207b7d0a70726f63203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f6672616d655f777261707065725f6e6561725f6d6973735f666f7277617264207b7072656669787d207b6576616c20247072656669787d0a70726f63203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f6672616d655f777261707065725f6e6561725f6d6973735f6368616e6765207b6e616d657d207b7570766172203120246e616d652063656c6c3b207365742063656c6c206368616e6765647d0a70726f63203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f6672616d655f777261707065725f6e6561725f6d6973735f6f75746572207b7d207b0a20202020736574207b6120627d206f726967696e616c0a202020203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f6672616d655f777261707065725f6e6561725f6d6973735f666f7277617264205b6c697374203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f6672616d655f777261707065725f6e6561725f6d6973735f6368616e6765207b6120627d5d0a20202020736574207b6120627d0a7d0a]
set nh_operation [binary format H* 3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f6672616d655f777261707065725f6e6561725f6d6973735f6f757465720a]
set nh_setup_code [catch {eval $nh_setup} nh_setup_result]

set nh_ec {}
set nh_ei {}
set nh_ec_present 0
set nh_ei_present 0
if {$nh_setup_code == 1} {
    if {[info exists ::errorCode]} {set nh_ec $::errorCode; set nh_ec_present 1}
    if {[info exists ::errorInfo]} {set nh_ei $::errorInfo; set nh_ei_present 1}
}
foreach {nh_field nh_value} [list result $nh_setup_result metadata [list errorcode-present $nh_ec_present errorinfo-present $nh_ei_present return-options unavailable] errorcode $nh_ec errorinfo $nh_ei] {
    if {$nh_field != "result" && $nh_setup_code != 1} {continue}
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
        log local0. "NHO|nh20261012pat1|frame-wrapper-near-miss|tmm-RULE_INIT|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
foreach {nh_field nh_value} [list result $nh_result metadata [list errorcode-present $nh_ec_present errorinfo-present $nh_ei_present return-options unavailable] errorcode $nh_ec errorinfo $nh_ei] {
    if {$nh_field != "result" && $nh_code != 1} {continue}
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
        log local0. "NHO|nh20261012pat1|frame-wrapper-near-miss|tmm-RULE_INIT|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 6966207b5b6c6c656e677468205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f6672616d655f777261707065725f6e6561725f6d6973735f666f72776172645d5d7d207b72656e616d65203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f6672616d655f777261707065725f6e6561725f6d6973735f666f7277617264207b7d7d0a6966207b5b6c6c656e677468205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f6672616d655f777261707065725f6e6561725f6d6973735f6368616e67655d5d7d207b72656e616d65203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f6672616d655f777261707065725f6e6561725f6d6973735f6368616e6765207b7d7d0a6966207b5b6c6c656e677468205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f6672616d655f777261707065725f6e6561725f6d6973735f6f757465725d5d7d207b72656e616d65203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f6672616d655f777261707065725f6e6561725f6d6973735f6f75746572207b7d7d0a6966207b5b6e616d65737061636520657869737473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f6672616d655f777261707065725f6e6561725f6d6973735d7d207b6e616d6573706163652064656c657465203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f6672616d655f777261707065725f6e6561725f6d6973737d0a]
set nh_code [catch {eval $nh_cleanup} nh_result]

set nh_ec {}
set nh_ei {}
set nh_ec_present 0
set nh_ei_present 0
if {$nh_code == 1} {
    if {[info exists ::errorCode]} {set nh_ec $::errorCode; set nh_ec_present 1}
    if {[info exists ::errorInfo]} {set nh_ei $::errorInfo; set nh_ei_present 1}
}
foreach {nh_field nh_value} [list result $nh_result metadata [list errorcode-present $nh_ec_present errorinfo-present $nh_ei_present return-options unavailable] errorcode $nh_ec errorinfo $nh_ei] {
    if {$nh_field != "result" && $nh_code != 1} {continue}
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
        log local0. "NHO|nh20261012pat1|frame-wrapper-near-miss|tmm-RULE_INIT|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
set nh_setup [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f6b657965645f7661726961626c655f6c696e6b207b7d0a6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f6b657965645f7661726961626c655f6c696e6b207b0a20202020736574207b73746174652e6120622876616c7565297d206f726967696e616c0a2020202070726f63206368616e6765207b6e616d657d207b0a20202020202020207570766172203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f6b657965645f7661726961626c655f6c696e6b3a3a246e616d652073746174650a20202020202020207365742073746174652876616c756529206368616e6765640a202020207d0a7d0a3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f6b657965645f7661726961626c655f6c696e6b3a3a6368616e6765207b73746174652e6120627d0a]
set nh_operation [binary format H* 736574207b3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f6b657965645f7661726961626c655f6c696e6b3a3a73746174652e6120622876616c7565297d0a]
set nh_setup_code [catch {eval $nh_setup} nh_setup_result]

set nh_ec {}
set nh_ei {}
set nh_ec_present 0
set nh_ei_present 0
if {$nh_setup_code == 1} {
    if {[info exists ::errorCode]} {set nh_ec $::errorCode; set nh_ec_present 1}
    if {[info exists ::errorInfo]} {set nh_ei $::errorInfo; set nh_ei_present 1}
}
foreach {nh_field nh_value} [list result $nh_setup_result metadata [list errorcode-present $nh_ec_present errorinfo-present $nh_ei_present return-options unavailable] errorcode $nh_ec errorinfo $nh_ei] {
    if {$nh_field != "result" && $nh_setup_code != 1} {continue}
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
        log local0. "NHO|nh20261012pat1|keyed-variable-link|tmm-RULE_INIT|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
foreach {nh_field nh_value} [list result $nh_result metadata [list errorcode-present $nh_ec_present errorinfo-present $nh_ei_present return-options unavailable] errorcode $nh_ec errorinfo $nh_ei] {
    if {$nh_field != "result" && $nh_code != 1} {continue}
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
        log local0. "NHO|nh20261012pat1|keyed-variable-link|tmm-RULE_INIT|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 6966207b5b6e616d65737061636520657869737473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f6b657965645f7661726961626c655f6c696e6b5d7d207b6e616d6573706163652064656c657465203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617431355f6b657965645f7661726961626c655f6c696e6b7d0a]
set nh_code [catch {eval $nh_cleanup} nh_result]

set nh_ec {}
set nh_ei {}
set nh_ec_present 0
set nh_ei_present 0
if {$nh_code == 1} {
    if {[info exists ::errorCode]} {set nh_ec $::errorCode; set nh_ec_present 1}
    if {[info exists ::errorInfo]} {set nh_ei $::errorInfo; set nh_ei_present 1}
}
foreach {nh_field nh_value} [list result $nh_result metadata [list errorcode-present $nh_ec_present errorinfo-present $nh_ei_present return-options unavailable] errorcode $nh_ec errorinfo $nh_ei] {
    if {$nh_field != "result" && $nh_code != 1} {continue}
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
        log local0. "NHO|nh20261012pat1|keyed-variable-link|tmm-RULE_INIT|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
```

## Results

### tmm-RULE_INIT, unit 0:0, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:01:56 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_2 <RULE_INIT>: NHO|nh20261012pat1|frame-wrapper|tmm-RULE_INIT|setup|0|0:0|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:01:56 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_2 <RULE_INIT>: NHO|nh20261012pat1|frame-wrapper|tmm-RULE_INIT|lookup|0|0:0|once|result|binary-scan-tcl-value|0|1|6368616e676564
Oct 12 08:01:56 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_2 <RULE_INIT>: NHO|nh20261012pat1|frame-wrapper|tmm-RULE_INIT|runtime-cleanup|0|0:0|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:01:56 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_2 <RULE_INIT>: NHO|nh20261012pat1|frame-wrapper-near-miss|tmm-RULE_INIT|setup|0|0:0|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:01:56 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_2 <RULE_INIT>: NHO|nh20261012pat1|frame-wrapper-near-miss|tmm-RULE_INIT|lookup|0|0:0|once|result|binary-scan-tcl-value|0|1|6f726967696e616c
Oct 12 08:01:56 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_2 <RULE_INIT>: NHO|nh20261012pat1|frame-wrapper-near-miss|tmm-RULE_INIT|runtime-cleanup|0|0:0|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:01:56 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_2 <RULE_INIT>: NHO|nh20261012pat1|keyed-variable-link|tmm-RULE_INIT|setup|0|0:0|once|result|binary-scan-tcl-value|0|1|6368616e676564
Oct 12 08:01:56 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_2 <RULE_INIT>: NHO|nh20261012pat1|keyed-variable-link|tmm-RULE_INIT|lookup|0|0:0|once|result|binary-scan-tcl-value|0|1|6368616e676564
Oct 12 08:01:56 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_2 <RULE_INIT>: NHO|nh20261012pat1|keyed-variable-link|tmm-RULE_INIT|runtime-cleanup|0|0:0|once|result|binary-scan-tcl-value|0|1|
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- frame-wrapper, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- frame-wrapper, lookup, completion 0, result, chunk 0 of 1:

```text-visible
changed⟦EOF⟧
```

- frame-wrapper, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- frame-wrapper-near-miss, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- frame-wrapper-near-miss, lookup, completion 0, result, chunk 0 of 1:

```text-visible
original⟦EOF⟧
```

- frame-wrapper-near-miss, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- keyed-variable-link, setup, completion 0, result, chunk 0 of 1:

```text-visible
changed⟦EOF⟧
```

- keyed-variable-link, lookup, completion 0, result, chunk 0 of 1:

```text-visible
changed⟦EOF⟧
```

- keyed-variable-link, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- frame-wrapper: intended operation reached.

- frame-wrapper-near-miss: intended operation reached.

- keyed-variable-link: intended operation reached.

### tmm-RULE_INIT, unit 0:1, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:01:56 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_2 <RULE_INIT>: NHO|nh20261012pat1|frame-wrapper|tmm-RULE_INIT|setup|0|0:1|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:01:56 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_2 <RULE_INIT>: NHO|nh20261012pat1|frame-wrapper|tmm-RULE_INIT|lookup|0|0:1|once|result|binary-scan-tcl-value|0|1|6368616e676564
Oct 12 08:01:56 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_2 <RULE_INIT>: NHO|nh20261012pat1|frame-wrapper|tmm-RULE_INIT|runtime-cleanup|0|0:1|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:01:56 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_2 <RULE_INIT>: NHO|nh20261012pat1|frame-wrapper-near-miss|tmm-RULE_INIT|setup|0|0:1|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:01:56 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_2 <RULE_INIT>: NHO|nh20261012pat1|frame-wrapper-near-miss|tmm-RULE_INIT|lookup|0|0:1|once|result|binary-scan-tcl-value|0|1|6f726967696e616c
Oct 12 08:01:56 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_2 <RULE_INIT>: NHO|nh20261012pat1|frame-wrapper-near-miss|tmm-RULE_INIT|runtime-cleanup|0|0:1|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:01:56 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_2 <RULE_INIT>: NHO|nh20261012pat1|keyed-variable-link|tmm-RULE_INIT|setup|0|0:1|once|result|binary-scan-tcl-value|0|1|6368616e676564
Oct 12 08:01:56 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_2 <RULE_INIT>: NHO|nh20261012pat1|keyed-variable-link|tmm-RULE_INIT|lookup|0|0:1|once|result|binary-scan-tcl-value|0|1|6368616e676564
Oct 12 08:01:56 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_2 <RULE_INIT>: NHO|nh20261012pat1|keyed-variable-link|tmm-RULE_INIT|runtime-cleanup|0|0:1|once|result|binary-scan-tcl-value|0|1|
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- frame-wrapper, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- frame-wrapper, lookup, completion 0, result, chunk 0 of 1:

```text-visible
changed⟦EOF⟧
```

- frame-wrapper, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- frame-wrapper-near-miss, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- frame-wrapper-near-miss, lookup, completion 0, result, chunk 0 of 1:

```text-visible
original⟦EOF⟧
```

- frame-wrapper-near-miss, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- keyed-variable-link, setup, completion 0, result, chunk 0 of 1:

```text-visible
changed⟦EOF⟧
```

- keyed-variable-link, lookup, completion 0, result, chunk 0 of 1:

```text-visible
changed⟦EOF⟧
```

- keyed-variable-link, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- frame-wrapper: intended operation reached.

- frame-wrapper-near-miss: intended operation reached.

- keyed-variable-link: intended operation reached.

### tmm-RULE_INIT, unit 0:2, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:01:56 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_2 <RULE_INIT>: NHO|nh20261012pat1|frame-wrapper|tmm-RULE_INIT|setup|0|0:2|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:01:56 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_2 <RULE_INIT>: NHO|nh20261012pat1|frame-wrapper|tmm-RULE_INIT|lookup|0|0:2|once|result|binary-scan-tcl-value|0|1|6368616e676564
Oct 12 08:01:56 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_2 <RULE_INIT>: NHO|nh20261012pat1|frame-wrapper|tmm-RULE_INIT|runtime-cleanup|0|0:2|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:01:56 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_2 <RULE_INIT>: NHO|nh20261012pat1|frame-wrapper-near-miss|tmm-RULE_INIT|setup|0|0:2|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:01:56 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_2 <RULE_INIT>: NHO|nh20261012pat1|frame-wrapper-near-miss|tmm-RULE_INIT|lookup|0|0:2|once|result|binary-scan-tcl-value|0|1|6f726967696e616c
Oct 12 08:01:56 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_2 <RULE_INIT>: NHO|nh20261012pat1|frame-wrapper-near-miss|tmm-RULE_INIT|runtime-cleanup|0|0:2|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:01:56 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_2 <RULE_INIT>: NHO|nh20261012pat1|keyed-variable-link|tmm-RULE_INIT|setup|0|0:2|once|result|binary-scan-tcl-value|0|1|6368616e676564
Oct 12 08:01:56 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_2 <RULE_INIT>: NHO|nh20261012pat1|keyed-variable-link|tmm-RULE_INIT|lookup|0|0:2|once|result|binary-scan-tcl-value|0|1|6368616e676564
Oct 12 08:01:56 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_2 <RULE_INIT>: NHO|nh20261012pat1|keyed-variable-link|tmm-RULE_INIT|runtime-cleanup|0|0:2|once|result|binary-scan-tcl-value|0|1|
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- frame-wrapper, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- frame-wrapper, lookup, completion 0, result, chunk 0 of 1:

```text-visible
changed⟦EOF⟧
```

- frame-wrapper, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- frame-wrapper-near-miss, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- frame-wrapper-near-miss, lookup, completion 0, result, chunk 0 of 1:

```text-visible
original⟦EOF⟧
```

- frame-wrapper-near-miss, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- keyed-variable-link, setup, completion 0, result, chunk 0 of 1:

```text-visible
changed⟦EOF⟧
```

- keyed-variable-link, lookup, completion 0, result, chunk 0 of 1:

```text-visible
changed⟦EOF⟧
```

- keyed-variable-link, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- frame-wrapper: intended operation reached.

- frame-wrapper-near-miss: intended operation reached.

- keyed-variable-link: intended operation reached.

### tmm-RULE_INIT, unit 0:3, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:01:56 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_2 <RULE_INIT>: NHO|nh20261012pat1|frame-wrapper|tmm-RULE_INIT|setup|0|0:3|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:01:56 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_2 <RULE_INIT>: NHO|nh20261012pat1|frame-wrapper|tmm-RULE_INIT|lookup|0|0:3|once|result|binary-scan-tcl-value|0|1|6368616e676564
Oct 12 08:01:56 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_2 <RULE_INIT>: NHO|nh20261012pat1|frame-wrapper|tmm-RULE_INIT|runtime-cleanup|0|0:3|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:01:56 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_2 <RULE_INIT>: NHO|nh20261012pat1|frame-wrapper-near-miss|tmm-RULE_INIT|setup|0|0:3|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:01:56 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_2 <RULE_INIT>: NHO|nh20261012pat1|frame-wrapper-near-miss|tmm-RULE_INIT|lookup|0|0:3|once|result|binary-scan-tcl-value|0|1|6f726967696e616c
Oct 12 08:01:56 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_2 <RULE_INIT>: NHO|nh20261012pat1|frame-wrapper-near-miss|tmm-RULE_INIT|runtime-cleanup|0|0:3|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:01:56 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_2 <RULE_INIT>: NHO|nh20261012pat1|keyed-variable-link|tmm-RULE_INIT|setup|0|0:3|once|result|binary-scan-tcl-value|0|1|6368616e676564
Oct 12 08:01:56 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_2 <RULE_INIT>: NHO|nh20261012pat1|keyed-variable-link|tmm-RULE_INIT|lookup|0|0:3|once|result|binary-scan-tcl-value|0|1|6368616e676564
Oct 12 08:01:56 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat1_5_2 <RULE_INIT>: NHO|nh20261012pat1|keyed-variable-link|tmm-RULE_INIT|runtime-cleanup|0|0:3|once|result|binary-scan-tcl-value|0|1|
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- frame-wrapper, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- frame-wrapper, lookup, completion 0, result, chunk 0 of 1:

```text-visible
changed⟦EOF⟧
```

- frame-wrapper, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- frame-wrapper-near-miss, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- frame-wrapper-near-miss, lookup, completion 0, result, chunk 0 of 1:

```text-visible
original⟦EOF⟧
```

- frame-wrapper-near-miss, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- keyed-variable-link, setup, completion 0, result, chunk 0 of 1:

```text-visible
changed⟦EOF⟧
```

- keyed-variable-link, lookup, completion 0, result, chunk 0 of 1:

```text-visible
changed⟦EOF⟧
```

- keyed-variable-link, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- frame-wrapper: intended operation reached.

- frame-wrapper-near-miss: intended operation reached.

- keyed-variable-link: intended operation reached.
