# BIG-IP Tcl log: nh20261012pat3, tmm-traffic-event-0

## Question

The following cases investigate Tcl behavior relevant to name resolution and diagnostics in the LSP, compiler and VM. Each observation is limited to its own case and recorded context.

- **alias-prefix-lifetime:** After an interpreter alias with a fixed multiword argument is renamed, what does querying the old alias name return, and how does the renamed command apply its retained prefix to a new argument?

- **computed-binding-mutation:** When a command name is assembled into a variable and the command at that name is replaced, does executing the unchanged variable use the original or the replacement binding?

- **computed-qualified:** Can a qualified command name assembled from a namespace variable be invoked as a computed command head, and which command does that spelling reach?

## Assumptions

- **alias-prefix-lifetime:** Adapted wrapper execution namespace/frame; not exact portable input. Setup includes all original commands before the final one.
- **computed-binding-mutation:** Adapted wrapper execution namespace/frame; not exact portable input. Setup includes all original commands before the final one.
- **computed-qualified:** Adapted wrapper execution namespace/frame; not exact portable input. Setup includes all original commands before the final one.

Other assumptions were not separately recorded.

## Script

```tcl-visible
set nh_options_available unavailable
set nh_return_options {}
set nh_unit "[TMM::cmp_group]:[TMM::cmp_unit]"
set nh_invocation once
set nh_invocation [HTTP::header value X-NH-Invocation]
set nh_setup [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617433375f616c6961735f7072656669785f6c69666574696d65207b7d0a6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617433375f616c6961735f7072656669785f6c69666574696d65207b70726f6320656d6974207b66697865642076616c75657d207b6c69737420246669786564202476616c75657d7d0a696e7465727020616c696173207b7d203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617433375f616c6961735f7072656669785f6c69666574696d653a3a6c6f63616c207b7d203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617433375f616c6961735f7072656669785f6c69666574696d653a3a656d6974207b6120627d0a72656e616d65203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617433375f616c6961735f7072656669785f6c69666574696d653a3a6c6f63616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617433375f616c6961735f7072656669785f6c69666574696d653a3a72656e616d65640a736574206465736372697074696f6e205b696e7465727020616c696173207b7d203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617433375f616c6961735f7072656669785f6c69666574696d653a3a6c6f63616c5d0a]
set nh_operation [binary format H* 6c69737420246465736372697074696f6e205b3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617433375f616c6961735f7072656669785f6c69666574696d653a3a72656e616d6564207b6320647d5d0a]
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
        log local0. "NHO|nh20261012pat3|alias-prefix-lifetime|tmm-traffic-event|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012pat3|alias-prefix-lifetime|tmm-traffic-event|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 6966207b5b6e616d65737061636520657869737473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617433375f616c6961735f7072656669785f6c69666574696d655d7d207b6e616d6573706163652064656c657465203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617433375f616c6961735f7072656669785f6c69666574696d657d0a]
set nh_code [catch {eval $nh_cleanup} nh_result]

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
        log local0. "NHO|nh20261012pat3|alias-prefix-lifetime|tmm-traffic-event|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
set nh_setup [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617433375f636f6d70757465645f62696e64696e675f6d75746174696f6e207b7d0a6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617433375f636f6d70757465645f62696e64696e675f6d75746174696f6e207b70726f6320656d6974207b7d207b72657475726e2066697273747d7d0a736574206e73203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617433375f636f6d70757465645f62696e64696e675f6d75746174696f6e0a736574206865616420247b6e737d3a3a656d69740a72656e616d65203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617433375f636f6d70757465645f62696e64696e675f6d75746174696f6e3a3a656d6974203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617433375f636f6d70757465645f62696e64696e675f6d75746174696f6e3a3a6f6c640a70726f63203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617433375f636f6d70757465645f62696e64696e675f6d75746174696f6e3a3a656d6974207b7d207b72657475726e207265706c6163656d656e747d0a]
set nh_operation [binary format H* 24686561640a]
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
        log local0. "NHO|nh20261012pat3|computed-binding-mutation|tmm-traffic-event|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012pat3|computed-binding-mutation|tmm-traffic-event|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 6966207b5b6e616d65737061636520657869737473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617433375f636f6d70757465645f62696e64696e675f6d75746174696f6e5d7d207b6e616d6573706163652064656c657465203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617433375f636f6d70757465645f62696e64696e675f6d75746174696f6e7d0a]
set nh_code [catch {eval $nh_cleanup} nh_result]

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
        log local0. "NHO|nh20261012pat3|computed-binding-mutation|tmm-traffic-event|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
set nh_setup [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617433375f636f6d70757465645f7175616c6966696564207b7d0a6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617433375f636f6d70757465645f7175616c6966696564207b70726f6320656d6974207b7d207b72657475726e2066697273747d7d0a736574206e73203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617433375f636f6d70757465645f7175616c69666965640a736574206865616420247b6e737d3a3a656d69740a]
set nh_operation [binary format H* 24686561640a]
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
        log local0. "NHO|nh20261012pat3|computed-qualified|tmm-traffic-event|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012pat3|computed-qualified|tmm-traffic-event|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 6966207b5b6e616d65737061636520657869737473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617433375f636f6d70757465645f7175616c69666965645d7d207b6e616d6573706163652064656c657465203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617433375f636f6d70757465645f7175616c69666965647d0a]
set nh_code [catch {eval $nh_cleanup} nh_result]

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
        log local0. "NHO|nh20261012pat3|computed-qualified|tmm-traffic-event|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
```

## Results

### tmm-traffic-event, unit 0:0, invocation 0

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:43:17 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|alias-prefix-lifetime|tmm-traffic-event|setup|0|0:0|0|result|binary-scan-tcl-value|0|1|3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617433375f616c6961735f7072656669785f6c69666574696d653a3a656d6974207b6120627d
Oct 12 08:43:17 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|alias-prefix-lifetime|tmm-traffic-event|lookup|0|0:0|0|result|binary-scan-tcl-value|0|1|7b3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617433375f616c6961735f7072656669785f6c69666574696d653a3a656d6974207b6120627d7d207b7b6120627d207b6320647d7d
Oct 12 08:43:17 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|alias-prefix-lifetime|tmm-traffic-event|runtime-cleanup|0|0:0|0|result|binary-scan-tcl-value|0|1|
Oct 12 08:43:17 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-binding-mutation|tmm-traffic-event|setup|0|0:0|0|result|binary-scan-tcl-value|0|1|
Oct 12 08:43:17 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-binding-mutation|tmm-traffic-event|lookup|0|0:0|0|result|binary-scan-tcl-value|0|1|7265706c6163656d656e74
Oct 12 08:43:17 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-binding-mutation|tmm-traffic-event|runtime-cleanup|0|0:0|0|result|binary-scan-tcl-value|0|1|
Oct 12 08:43:17 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-qualified|tmm-traffic-event|setup|0|0:0|0|result|binary-scan-tcl-value|0|1|3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617433375f636f6d70757465645f7175616c69666965643a3a656d6974
Oct 12 08:43:17 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-qualified|tmm-traffic-event|lookup|0|0:0|0|result|binary-scan-tcl-value|0|1|6669727374
Oct 12 08:43:17 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-qualified|tmm-traffic-event|runtime-cleanup|0|0:0|0|result|binary-scan-tcl-value|0|1|
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- alias-prefix-lifetime, setup, completion 0, result, chunk 0 of 1:

```text-visible
::__tcl_lsp_probe_nh20261012pat37_alias_prefix_lifetime::emit {a b}⟦EOF⟧
```

- alias-prefix-lifetime, lookup, completion 0, result, chunk 0 of 1:

```text-visible
{::__tcl_lsp_probe_nh20261012pat37_alias_prefix_lifetime::emit {a b}} {{a b} {c d}}⟦EOF⟧
```

- alias-prefix-lifetime, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- computed-binding-mutation, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- computed-binding-mutation, lookup, completion 0, result, chunk 0 of 1:

```text-visible
replacement⟦EOF⟧
```

- computed-binding-mutation, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- computed-qualified, setup, completion 0, result, chunk 0 of 1:

```text-visible
::__tcl_lsp_probe_nh20261012pat37_computed_qualified::emit⟦EOF⟧
```

- computed-qualified, lookup, completion 0, result, chunk 0 of 1:

```text-visible
first⟦EOF⟧
```

- computed-qualified, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- alias-prefix-lifetime: intended operation reached.

- computed-binding-mutation: intended operation reached.

- computed-qualified: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 1

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:43:18 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|alias-prefix-lifetime|tmm-traffic-event|setup|0|0:0|1|result|binary-scan-tcl-value|0|1|3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617433375f616c6961735f7072656669785f6c69666574696d653a3a656d6974207b6120627d
Oct 12 08:43:18 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|alias-prefix-lifetime|tmm-traffic-event|lookup|0|0:0|1|result|binary-scan-tcl-value|0|1|7b3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617433375f616c6961735f7072656669785f6c69666574696d653a3a656d6974207b6120627d7d207b7b6120627d207b6320647d7d
Oct 12 08:43:18 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|alias-prefix-lifetime|tmm-traffic-event|runtime-cleanup|0|0:0|1|result|binary-scan-tcl-value|0|1|
Oct 12 08:43:18 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-binding-mutation|tmm-traffic-event|setup|0|0:0|1|result|binary-scan-tcl-value|0|1|
Oct 12 08:43:18 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-binding-mutation|tmm-traffic-event|lookup|0|0:0|1|result|binary-scan-tcl-value|0|1|7265706c6163656d656e74
Oct 12 08:43:18 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-binding-mutation|tmm-traffic-event|runtime-cleanup|0|0:0|1|result|binary-scan-tcl-value|0|1|
Oct 12 08:43:18 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-qualified|tmm-traffic-event|setup|0|0:0|1|result|binary-scan-tcl-value|0|1|3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617433375f636f6d70757465645f7175616c69666965643a3a656d6974
Oct 12 08:43:18 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-qualified|tmm-traffic-event|lookup|0|0:0|1|result|binary-scan-tcl-value|0|1|6669727374
Oct 12 08:43:18 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-qualified|tmm-traffic-event|runtime-cleanup|0|0:0|1|result|binary-scan-tcl-value|0|1|
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- alias-prefix-lifetime, setup, completion 0, result, chunk 0 of 1:

```text-visible
::__tcl_lsp_probe_nh20261012pat37_alias_prefix_lifetime::emit {a b}⟦EOF⟧
```

- alias-prefix-lifetime, lookup, completion 0, result, chunk 0 of 1:

```text-visible
{::__tcl_lsp_probe_nh20261012pat37_alias_prefix_lifetime::emit {a b}} {{a b} {c d}}⟦EOF⟧
```

- alias-prefix-lifetime, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- computed-binding-mutation, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- computed-binding-mutation, lookup, completion 0, result, chunk 0 of 1:

```text-visible
replacement⟦EOF⟧
```

- computed-binding-mutation, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- computed-qualified, setup, completion 0, result, chunk 0 of 1:

```text-visible
::__tcl_lsp_probe_nh20261012pat37_computed_qualified::emit⟦EOF⟧
```

- computed-qualified, lookup, completion 0, result, chunk 0 of 1:

```text-visible
first⟦EOF⟧
```

- computed-qualified, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- alias-prefix-lifetime: intended operation reached.

- computed-binding-mutation: intended operation reached.

- computed-qualified: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 10

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:43:19 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|alias-prefix-lifetime|tmm-traffic-event|setup|0|0:0|10|result|binary-scan-tcl-value|0|1|3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617433375f616c6961735f7072656669785f6c69666574696d653a3a656d6974207b6120627d
Oct 12 08:43:19 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|alias-prefix-lifetime|tmm-traffic-event|lookup|0|0:0|10|result|binary-scan-tcl-value|0|1|7b3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617433375f616c6961735f7072656669785f6c69666574696d653a3a656d6974207b6120627d7d207b7b6120627d207b6320647d7d
Oct 12 08:43:19 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|alias-prefix-lifetime|tmm-traffic-event|runtime-cleanup|0|0:0|10|result|binary-scan-tcl-value|0|1|
Oct 12 08:43:19 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-binding-mutation|tmm-traffic-event|setup|0|0:0|10|result|binary-scan-tcl-value|0|1|
Oct 12 08:43:19 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-binding-mutation|tmm-traffic-event|lookup|0|0:0|10|result|binary-scan-tcl-value|0|1|7265706c6163656d656e74
Oct 12 08:43:19 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-binding-mutation|tmm-traffic-event|runtime-cleanup|0|0:0|10|result|binary-scan-tcl-value|0|1|
Oct 12 08:43:19 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-qualified|tmm-traffic-event|setup|0|0:0|10|result|binary-scan-tcl-value|0|1|3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617433375f636f6d70757465645f7175616c69666965643a3a656d6974
Oct 12 08:43:19 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-qualified|tmm-traffic-event|lookup|0|0:0|10|result|binary-scan-tcl-value|0|1|6669727374
Oct 12 08:43:19 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-qualified|tmm-traffic-event|runtime-cleanup|0|0:0|10|result|binary-scan-tcl-value|0|1|
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- alias-prefix-lifetime, setup, completion 0, result, chunk 0 of 1:

```text-visible
::__tcl_lsp_probe_nh20261012pat37_alias_prefix_lifetime::emit {a b}⟦EOF⟧
```

- alias-prefix-lifetime, lookup, completion 0, result, chunk 0 of 1:

```text-visible
{::__tcl_lsp_probe_nh20261012pat37_alias_prefix_lifetime::emit {a b}} {{a b} {c d}}⟦EOF⟧
```

- alias-prefix-lifetime, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- computed-binding-mutation, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- computed-binding-mutation, lookup, completion 0, result, chunk 0 of 1:

```text-visible
replacement⟦EOF⟧
```

- computed-binding-mutation, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- computed-qualified, setup, completion 0, result, chunk 0 of 1:

```text-visible
::__tcl_lsp_probe_nh20261012pat37_computed_qualified::emit⟦EOF⟧
```

- computed-qualified, lookup, completion 0, result, chunk 0 of 1:

```text-visible
first⟦EOF⟧
```

- computed-qualified, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- alias-prefix-lifetime: intended operation reached.

- computed-binding-mutation: intended operation reached.

- computed-qualified: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 11

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:43:19 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|alias-prefix-lifetime|tmm-traffic-event|setup|0|0:0|11|result|binary-scan-tcl-value|0|1|3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617433375f616c6961735f7072656669785f6c69666574696d653a3a656d6974207b6120627d
Oct 12 08:43:19 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|alias-prefix-lifetime|tmm-traffic-event|lookup|0|0:0|11|result|binary-scan-tcl-value|0|1|7b3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617433375f616c6961735f7072656669785f6c69666574696d653a3a656d6974207b6120627d7d207b7b6120627d207b6320647d7d
Oct 12 08:43:19 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|alias-prefix-lifetime|tmm-traffic-event|runtime-cleanup|0|0:0|11|result|binary-scan-tcl-value|0|1|
Oct 12 08:43:19 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-binding-mutation|tmm-traffic-event|setup|0|0:0|11|result|binary-scan-tcl-value|0|1|
Oct 12 08:43:19 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-binding-mutation|tmm-traffic-event|lookup|0|0:0|11|result|binary-scan-tcl-value|0|1|7265706c6163656d656e74
Oct 12 08:43:19 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-binding-mutation|tmm-traffic-event|runtime-cleanup|0|0:0|11|result|binary-scan-tcl-value|0|1|
Oct 12 08:43:19 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-qualified|tmm-traffic-event|setup|0|0:0|11|result|binary-scan-tcl-value|0|1|3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617433375f636f6d70757465645f7175616c69666965643a3a656d6974
Oct 12 08:43:19 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-qualified|tmm-traffic-event|lookup|0|0:0|11|result|binary-scan-tcl-value|0|1|6669727374
Oct 12 08:43:19 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-qualified|tmm-traffic-event|runtime-cleanup|0|0:0|11|result|binary-scan-tcl-value|0|1|
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- alias-prefix-lifetime, setup, completion 0, result, chunk 0 of 1:

```text-visible
::__tcl_lsp_probe_nh20261012pat37_alias_prefix_lifetime::emit {a b}⟦EOF⟧
```

- alias-prefix-lifetime, lookup, completion 0, result, chunk 0 of 1:

```text-visible
{::__tcl_lsp_probe_nh20261012pat37_alias_prefix_lifetime::emit {a b}} {{a b} {c d}}⟦EOF⟧
```

- alias-prefix-lifetime, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- computed-binding-mutation, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- computed-binding-mutation, lookup, completion 0, result, chunk 0 of 1:

```text-visible
replacement⟦EOF⟧
```

- computed-binding-mutation, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- computed-qualified, setup, completion 0, result, chunk 0 of 1:

```text-visible
::__tcl_lsp_probe_nh20261012pat37_computed_qualified::emit⟦EOF⟧
```

- computed-qualified, lookup, completion 0, result, chunk 0 of 1:

```text-visible
first⟦EOF⟧
```

- computed-qualified, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- alias-prefix-lifetime: intended operation reached.

- computed-binding-mutation: intended operation reached.

- computed-qualified: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 12

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:43:19 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|alias-prefix-lifetime|tmm-traffic-event|setup|0|0:0|12|result|binary-scan-tcl-value|0|1|3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617433375f616c6961735f7072656669785f6c69666574696d653a3a656d6974207b6120627d
Oct 12 08:43:19 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|alias-prefix-lifetime|tmm-traffic-event|lookup|0|0:0|12|result|binary-scan-tcl-value|0|1|7b3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617433375f616c6961735f7072656669785f6c69666574696d653a3a656d6974207b6120627d7d207b7b6120627d207b6320647d7d
Oct 12 08:43:19 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|alias-prefix-lifetime|tmm-traffic-event|runtime-cleanup|0|0:0|12|result|binary-scan-tcl-value|0|1|
Oct 12 08:43:19 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-binding-mutation|tmm-traffic-event|setup|0|0:0|12|result|binary-scan-tcl-value|0|1|
Oct 12 08:43:19 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-binding-mutation|tmm-traffic-event|lookup|0|0:0|12|result|binary-scan-tcl-value|0|1|7265706c6163656d656e74
Oct 12 08:43:19 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-binding-mutation|tmm-traffic-event|runtime-cleanup|0|0:0|12|result|binary-scan-tcl-value|0|1|
Oct 12 08:43:19 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-qualified|tmm-traffic-event|setup|0|0:0|12|result|binary-scan-tcl-value|0|1|3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617433375f636f6d70757465645f7175616c69666965643a3a656d6974
Oct 12 08:43:19 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-qualified|tmm-traffic-event|lookup|0|0:0|12|result|binary-scan-tcl-value|0|1|6669727374
Oct 12 08:43:19 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-qualified|tmm-traffic-event|runtime-cleanup|0|0:0|12|result|binary-scan-tcl-value|0|1|
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- alias-prefix-lifetime, setup, completion 0, result, chunk 0 of 1:

```text-visible
::__tcl_lsp_probe_nh20261012pat37_alias_prefix_lifetime::emit {a b}⟦EOF⟧
```

- alias-prefix-lifetime, lookup, completion 0, result, chunk 0 of 1:

```text-visible
{::__tcl_lsp_probe_nh20261012pat37_alias_prefix_lifetime::emit {a b}} {{a b} {c d}}⟦EOF⟧
```

- alias-prefix-lifetime, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- computed-binding-mutation, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- computed-binding-mutation, lookup, completion 0, result, chunk 0 of 1:

```text-visible
replacement⟦EOF⟧
```

- computed-binding-mutation, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- computed-qualified, setup, completion 0, result, chunk 0 of 1:

```text-visible
::__tcl_lsp_probe_nh20261012pat37_computed_qualified::emit⟦EOF⟧
```

- computed-qualified, lookup, completion 0, result, chunk 0 of 1:

```text-visible
first⟦EOF⟧
```

- computed-qualified, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- alias-prefix-lifetime: intended operation reached.

- computed-binding-mutation: intended operation reached.

- computed-qualified: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 13

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:43:19 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|alias-prefix-lifetime|tmm-traffic-event|setup|0|0:0|13|result|binary-scan-tcl-value|0|1|3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617433375f616c6961735f7072656669785f6c69666574696d653a3a656d6974207b6120627d
Oct 12 08:43:19 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|alias-prefix-lifetime|tmm-traffic-event|lookup|0|0:0|13|result|binary-scan-tcl-value|0|1|7b3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617433375f616c6961735f7072656669785f6c69666574696d653a3a656d6974207b6120627d7d207b7b6120627d207b6320647d7d
Oct 12 08:43:19 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|alias-prefix-lifetime|tmm-traffic-event|runtime-cleanup|0|0:0|13|result|binary-scan-tcl-value|0|1|
Oct 12 08:43:19 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-binding-mutation|tmm-traffic-event|setup|0|0:0|13|result|binary-scan-tcl-value|0|1|
Oct 12 08:43:19 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-binding-mutation|tmm-traffic-event|lookup|0|0:0|13|result|binary-scan-tcl-value|0|1|7265706c6163656d656e74
Oct 12 08:43:19 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-binding-mutation|tmm-traffic-event|runtime-cleanup|0|0:0|13|result|binary-scan-tcl-value|0|1|
Oct 12 08:43:19 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-qualified|tmm-traffic-event|setup|0|0:0|13|result|binary-scan-tcl-value|0|1|3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617433375f636f6d70757465645f7175616c69666965643a3a656d6974
Oct 12 08:43:19 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-qualified|tmm-traffic-event|lookup|0|0:0|13|result|binary-scan-tcl-value|0|1|6669727374
Oct 12 08:43:19 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-qualified|tmm-traffic-event|runtime-cleanup|0|0:0|13|result|binary-scan-tcl-value|0|1|
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- alias-prefix-lifetime, setup, completion 0, result, chunk 0 of 1:

```text-visible
::__tcl_lsp_probe_nh20261012pat37_alias_prefix_lifetime::emit {a b}⟦EOF⟧
```

- alias-prefix-lifetime, lookup, completion 0, result, chunk 0 of 1:

```text-visible
{::__tcl_lsp_probe_nh20261012pat37_alias_prefix_lifetime::emit {a b}} {{a b} {c d}}⟦EOF⟧
```

- alias-prefix-lifetime, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- computed-binding-mutation, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- computed-binding-mutation, lookup, completion 0, result, chunk 0 of 1:

```text-visible
replacement⟦EOF⟧
```

- computed-binding-mutation, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- computed-qualified, setup, completion 0, result, chunk 0 of 1:

```text-visible
::__tcl_lsp_probe_nh20261012pat37_computed_qualified::emit⟦EOF⟧
```

- computed-qualified, lookup, completion 0, result, chunk 0 of 1:

```text-visible
first⟦EOF⟧
```

- computed-qualified, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- alias-prefix-lifetime: intended operation reached.

- computed-binding-mutation: intended operation reached.

- computed-qualified: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 14

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:43:19 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|alias-prefix-lifetime|tmm-traffic-event|setup|0|0:0|14|result|binary-scan-tcl-value|0|1|3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617433375f616c6961735f7072656669785f6c69666574696d653a3a656d6974207b6120627d
Oct 12 08:43:19 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|alias-prefix-lifetime|tmm-traffic-event|lookup|0|0:0|14|result|binary-scan-tcl-value|0|1|7b3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617433375f616c6961735f7072656669785f6c69666574696d653a3a656d6974207b6120627d7d207b7b6120627d207b6320647d7d
Oct 12 08:43:19 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|alias-prefix-lifetime|tmm-traffic-event|runtime-cleanup|0|0:0|14|result|binary-scan-tcl-value|0|1|
Oct 12 08:43:19 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-binding-mutation|tmm-traffic-event|setup|0|0:0|14|result|binary-scan-tcl-value|0|1|
Oct 12 08:43:19 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-binding-mutation|tmm-traffic-event|lookup|0|0:0|14|result|binary-scan-tcl-value|0|1|7265706c6163656d656e74
Oct 12 08:43:19 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-binding-mutation|tmm-traffic-event|runtime-cleanup|0|0:0|14|result|binary-scan-tcl-value|0|1|
Oct 12 08:43:19 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-qualified|tmm-traffic-event|setup|0|0:0|14|result|binary-scan-tcl-value|0|1|3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617433375f636f6d70757465645f7175616c69666965643a3a656d6974
Oct 12 08:43:19 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-qualified|tmm-traffic-event|lookup|0|0:0|14|result|binary-scan-tcl-value|0|1|6669727374
Oct 12 08:43:19 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-qualified|tmm-traffic-event|runtime-cleanup|0|0:0|14|result|binary-scan-tcl-value|0|1|
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- alias-prefix-lifetime, setup, completion 0, result, chunk 0 of 1:

```text-visible
::__tcl_lsp_probe_nh20261012pat37_alias_prefix_lifetime::emit {a b}⟦EOF⟧
```

- alias-prefix-lifetime, lookup, completion 0, result, chunk 0 of 1:

```text-visible
{::__tcl_lsp_probe_nh20261012pat37_alias_prefix_lifetime::emit {a b}} {{a b} {c d}}⟦EOF⟧
```

- alias-prefix-lifetime, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- computed-binding-mutation, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- computed-binding-mutation, lookup, completion 0, result, chunk 0 of 1:

```text-visible
replacement⟦EOF⟧
```

- computed-binding-mutation, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- computed-qualified, setup, completion 0, result, chunk 0 of 1:

```text-visible
::__tcl_lsp_probe_nh20261012pat37_computed_qualified::emit⟦EOF⟧
```

- computed-qualified, lookup, completion 0, result, chunk 0 of 1:

```text-visible
first⟦EOF⟧
```

- computed-qualified, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- alias-prefix-lifetime: intended operation reached.

- computed-binding-mutation: intended operation reached.

- computed-qualified: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 15

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:43:19 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|alias-prefix-lifetime|tmm-traffic-event|setup|0|0:0|15|result|binary-scan-tcl-value|0|1|3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617433375f616c6961735f7072656669785f6c69666574696d653a3a656d6974207b6120627d
Oct 12 08:43:19 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|alias-prefix-lifetime|tmm-traffic-event|lookup|0|0:0|15|result|binary-scan-tcl-value|0|1|7b3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617433375f616c6961735f7072656669785f6c69666574696d653a3a656d6974207b6120627d7d207b7b6120627d207b6320647d7d
Oct 12 08:43:19 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|alias-prefix-lifetime|tmm-traffic-event|runtime-cleanup|0|0:0|15|result|binary-scan-tcl-value|0|1|
Oct 12 08:43:19 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-binding-mutation|tmm-traffic-event|setup|0|0:0|15|result|binary-scan-tcl-value|0|1|
Oct 12 08:43:19 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-binding-mutation|tmm-traffic-event|lookup|0|0:0|15|result|binary-scan-tcl-value|0|1|7265706c6163656d656e74
Oct 12 08:43:19 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-binding-mutation|tmm-traffic-event|runtime-cleanup|0|0:0|15|result|binary-scan-tcl-value|0|1|
Oct 12 08:43:19 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-qualified|tmm-traffic-event|setup|0|0:0|15|result|binary-scan-tcl-value|0|1|3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617433375f636f6d70757465645f7175616c69666965643a3a656d6974
Oct 12 08:43:19 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-qualified|tmm-traffic-event|lookup|0|0:0|15|result|binary-scan-tcl-value|0|1|6669727374
Oct 12 08:43:19 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-qualified|tmm-traffic-event|runtime-cleanup|0|0:0|15|result|binary-scan-tcl-value|0|1|
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- alias-prefix-lifetime, setup, completion 0, result, chunk 0 of 1:

```text-visible
::__tcl_lsp_probe_nh20261012pat37_alias_prefix_lifetime::emit {a b}⟦EOF⟧
```

- alias-prefix-lifetime, lookup, completion 0, result, chunk 0 of 1:

```text-visible
{::__tcl_lsp_probe_nh20261012pat37_alias_prefix_lifetime::emit {a b}} {{a b} {c d}}⟦EOF⟧
```

- alias-prefix-lifetime, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- computed-binding-mutation, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- computed-binding-mutation, lookup, completion 0, result, chunk 0 of 1:

```text-visible
replacement⟦EOF⟧
```

- computed-binding-mutation, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- computed-qualified, setup, completion 0, result, chunk 0 of 1:

```text-visible
::__tcl_lsp_probe_nh20261012pat37_computed_qualified::emit⟦EOF⟧
```

- computed-qualified, lookup, completion 0, result, chunk 0 of 1:

```text-visible
first⟦EOF⟧
```

- computed-qualified, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- alias-prefix-lifetime: intended operation reached.

- computed-binding-mutation: intended operation reached.

- computed-qualified: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 2

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:43:18 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|alias-prefix-lifetime|tmm-traffic-event|setup|0|0:0|2|result|binary-scan-tcl-value|0|1|3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617433375f616c6961735f7072656669785f6c69666574696d653a3a656d6974207b6120627d
Oct 12 08:43:18 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|alias-prefix-lifetime|tmm-traffic-event|lookup|0|0:0|2|result|binary-scan-tcl-value|0|1|7b3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617433375f616c6961735f7072656669785f6c69666574696d653a3a656d6974207b6120627d7d207b7b6120627d207b6320647d7d
Oct 12 08:43:18 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|alias-prefix-lifetime|tmm-traffic-event|runtime-cleanup|0|0:0|2|result|binary-scan-tcl-value|0|1|
Oct 12 08:43:18 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-binding-mutation|tmm-traffic-event|setup|0|0:0|2|result|binary-scan-tcl-value|0|1|
Oct 12 08:43:18 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-binding-mutation|tmm-traffic-event|lookup|0|0:0|2|result|binary-scan-tcl-value|0|1|7265706c6163656d656e74
Oct 12 08:43:18 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-binding-mutation|tmm-traffic-event|runtime-cleanup|0|0:0|2|result|binary-scan-tcl-value|0|1|
Oct 12 08:43:18 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-qualified|tmm-traffic-event|setup|0|0:0|2|result|binary-scan-tcl-value|0|1|3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617433375f636f6d70757465645f7175616c69666965643a3a656d6974
Oct 12 08:43:18 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-qualified|tmm-traffic-event|lookup|0|0:0|2|result|binary-scan-tcl-value|0|1|6669727374
Oct 12 08:43:18 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-qualified|tmm-traffic-event|runtime-cleanup|0|0:0|2|result|binary-scan-tcl-value|0|1|
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- alias-prefix-lifetime, setup, completion 0, result, chunk 0 of 1:

```text-visible
::__tcl_lsp_probe_nh20261012pat37_alias_prefix_lifetime::emit {a b}⟦EOF⟧
```

- alias-prefix-lifetime, lookup, completion 0, result, chunk 0 of 1:

```text-visible
{::__tcl_lsp_probe_nh20261012pat37_alias_prefix_lifetime::emit {a b}} {{a b} {c d}}⟦EOF⟧
```

- alias-prefix-lifetime, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- computed-binding-mutation, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- computed-binding-mutation, lookup, completion 0, result, chunk 0 of 1:

```text-visible
replacement⟦EOF⟧
```

- computed-binding-mutation, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- computed-qualified, setup, completion 0, result, chunk 0 of 1:

```text-visible
::__tcl_lsp_probe_nh20261012pat37_computed_qualified::emit⟦EOF⟧
```

- computed-qualified, lookup, completion 0, result, chunk 0 of 1:

```text-visible
first⟦EOF⟧
```

- computed-qualified, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- alias-prefix-lifetime: intended operation reached.

- computed-binding-mutation: intended operation reached.

- computed-qualified: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 3

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:43:18 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|alias-prefix-lifetime|tmm-traffic-event|setup|0|0:0|3|result|binary-scan-tcl-value|0|1|3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617433375f616c6961735f7072656669785f6c69666574696d653a3a656d6974207b6120627d
Oct 12 08:43:18 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|alias-prefix-lifetime|tmm-traffic-event|lookup|0|0:0|3|result|binary-scan-tcl-value|0|1|7b3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617433375f616c6961735f7072656669785f6c69666574696d653a3a656d6974207b6120627d7d207b7b6120627d207b6320647d7d
Oct 12 08:43:18 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|alias-prefix-lifetime|tmm-traffic-event|runtime-cleanup|0|0:0|3|result|binary-scan-tcl-value|0|1|
Oct 12 08:43:18 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-binding-mutation|tmm-traffic-event|setup|0|0:0|3|result|binary-scan-tcl-value|0|1|
Oct 12 08:43:18 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-binding-mutation|tmm-traffic-event|lookup|0|0:0|3|result|binary-scan-tcl-value|0|1|7265706c6163656d656e74
Oct 12 08:43:18 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-binding-mutation|tmm-traffic-event|runtime-cleanup|0|0:0|3|result|binary-scan-tcl-value|0|1|
Oct 12 08:43:18 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-qualified|tmm-traffic-event|setup|0|0:0|3|result|binary-scan-tcl-value|0|1|3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617433375f636f6d70757465645f7175616c69666965643a3a656d6974
Oct 12 08:43:18 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-qualified|tmm-traffic-event|lookup|0|0:0|3|result|binary-scan-tcl-value|0|1|6669727374
Oct 12 08:43:18 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-qualified|tmm-traffic-event|runtime-cleanup|0|0:0|3|result|binary-scan-tcl-value|0|1|
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- alias-prefix-lifetime, setup, completion 0, result, chunk 0 of 1:

```text-visible
::__tcl_lsp_probe_nh20261012pat37_alias_prefix_lifetime::emit {a b}⟦EOF⟧
```

- alias-prefix-lifetime, lookup, completion 0, result, chunk 0 of 1:

```text-visible
{::__tcl_lsp_probe_nh20261012pat37_alias_prefix_lifetime::emit {a b}} {{a b} {c d}}⟦EOF⟧
```

- alias-prefix-lifetime, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- computed-binding-mutation, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- computed-binding-mutation, lookup, completion 0, result, chunk 0 of 1:

```text-visible
replacement⟦EOF⟧
```

- computed-binding-mutation, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- computed-qualified, setup, completion 0, result, chunk 0 of 1:

```text-visible
::__tcl_lsp_probe_nh20261012pat37_computed_qualified::emit⟦EOF⟧
```

- computed-qualified, lookup, completion 0, result, chunk 0 of 1:

```text-visible
first⟦EOF⟧
```

- computed-qualified, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- alias-prefix-lifetime: intended operation reached.

- computed-binding-mutation: intended operation reached.

- computed-qualified: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 4

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:43:18 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|alias-prefix-lifetime|tmm-traffic-event|setup|0|0:0|4|result|binary-scan-tcl-value|0|1|3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617433375f616c6961735f7072656669785f6c69666574696d653a3a656d6974207b6120627d
Oct 12 08:43:18 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|alias-prefix-lifetime|tmm-traffic-event|lookup|0|0:0|4|result|binary-scan-tcl-value|0|1|7b3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617433375f616c6961735f7072656669785f6c69666574696d653a3a656d6974207b6120627d7d207b7b6120627d207b6320647d7d
Oct 12 08:43:18 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|alias-prefix-lifetime|tmm-traffic-event|runtime-cleanup|0|0:0|4|result|binary-scan-tcl-value|0|1|
Oct 12 08:43:18 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-binding-mutation|tmm-traffic-event|setup|0|0:0|4|result|binary-scan-tcl-value|0|1|
Oct 12 08:43:18 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-binding-mutation|tmm-traffic-event|lookup|0|0:0|4|result|binary-scan-tcl-value|0|1|7265706c6163656d656e74
Oct 12 08:43:18 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-binding-mutation|tmm-traffic-event|runtime-cleanup|0|0:0|4|result|binary-scan-tcl-value|0|1|
Oct 12 08:43:18 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-qualified|tmm-traffic-event|setup|0|0:0|4|result|binary-scan-tcl-value|0|1|3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617433375f636f6d70757465645f7175616c69666965643a3a656d6974
Oct 12 08:43:18 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-qualified|tmm-traffic-event|lookup|0|0:0|4|result|binary-scan-tcl-value|0|1|6669727374
Oct 12 08:43:18 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-qualified|tmm-traffic-event|runtime-cleanup|0|0:0|4|result|binary-scan-tcl-value|0|1|
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- alias-prefix-lifetime, setup, completion 0, result, chunk 0 of 1:

```text-visible
::__tcl_lsp_probe_nh20261012pat37_alias_prefix_lifetime::emit {a b}⟦EOF⟧
```

- alias-prefix-lifetime, lookup, completion 0, result, chunk 0 of 1:

```text-visible
{::__tcl_lsp_probe_nh20261012pat37_alias_prefix_lifetime::emit {a b}} {{a b} {c d}}⟦EOF⟧
```

- alias-prefix-lifetime, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- computed-binding-mutation, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- computed-binding-mutation, lookup, completion 0, result, chunk 0 of 1:

```text-visible
replacement⟦EOF⟧
```

- computed-binding-mutation, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- computed-qualified, setup, completion 0, result, chunk 0 of 1:

```text-visible
::__tcl_lsp_probe_nh20261012pat37_computed_qualified::emit⟦EOF⟧
```

- computed-qualified, lookup, completion 0, result, chunk 0 of 1:

```text-visible
first⟦EOF⟧
```

- computed-qualified, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- alias-prefix-lifetime: intended operation reached.

- computed-binding-mutation: intended operation reached.

- computed-qualified: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 5

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:43:18 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|alias-prefix-lifetime|tmm-traffic-event|setup|0|0:0|5|result|binary-scan-tcl-value|0|1|3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617433375f616c6961735f7072656669785f6c69666574696d653a3a656d6974207b6120627d
Oct 12 08:43:18 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|alias-prefix-lifetime|tmm-traffic-event|lookup|0|0:0|5|result|binary-scan-tcl-value|0|1|7b3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617433375f616c6961735f7072656669785f6c69666574696d653a3a656d6974207b6120627d7d207b7b6120627d207b6320647d7d
Oct 12 08:43:18 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|alias-prefix-lifetime|tmm-traffic-event|runtime-cleanup|0|0:0|5|result|binary-scan-tcl-value|0|1|
Oct 12 08:43:18 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-binding-mutation|tmm-traffic-event|setup|0|0:0|5|result|binary-scan-tcl-value|0|1|
Oct 12 08:43:18 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-binding-mutation|tmm-traffic-event|lookup|0|0:0|5|result|binary-scan-tcl-value|0|1|7265706c6163656d656e74
Oct 12 08:43:18 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-binding-mutation|tmm-traffic-event|runtime-cleanup|0|0:0|5|result|binary-scan-tcl-value|0|1|
Oct 12 08:43:18 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-qualified|tmm-traffic-event|setup|0|0:0|5|result|binary-scan-tcl-value|0|1|3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617433375f636f6d70757465645f7175616c69666965643a3a656d6974
Oct 12 08:43:18 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-qualified|tmm-traffic-event|lookup|0|0:0|5|result|binary-scan-tcl-value|0|1|6669727374
Oct 12 08:43:18 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-qualified|tmm-traffic-event|runtime-cleanup|0|0:0|5|result|binary-scan-tcl-value|0|1|
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- alias-prefix-lifetime, setup, completion 0, result, chunk 0 of 1:

```text-visible
::__tcl_lsp_probe_nh20261012pat37_alias_prefix_lifetime::emit {a b}⟦EOF⟧
```

- alias-prefix-lifetime, lookup, completion 0, result, chunk 0 of 1:

```text-visible
{::__tcl_lsp_probe_nh20261012pat37_alias_prefix_lifetime::emit {a b}} {{a b} {c d}}⟦EOF⟧
```

- alias-prefix-lifetime, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- computed-binding-mutation, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- computed-binding-mutation, lookup, completion 0, result, chunk 0 of 1:

```text-visible
replacement⟦EOF⟧
```

- computed-binding-mutation, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- computed-qualified, setup, completion 0, result, chunk 0 of 1:

```text-visible
::__tcl_lsp_probe_nh20261012pat37_computed_qualified::emit⟦EOF⟧
```

- computed-qualified, lookup, completion 0, result, chunk 0 of 1:

```text-visible
first⟦EOF⟧
```

- computed-qualified, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- alias-prefix-lifetime: intended operation reached.

- computed-binding-mutation: intended operation reached.

- computed-qualified: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 6

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:43:18 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|alias-prefix-lifetime|tmm-traffic-event|setup|0|0:0|6|result|binary-scan-tcl-value|0|1|3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617433375f616c6961735f7072656669785f6c69666574696d653a3a656d6974207b6120627d
Oct 12 08:43:18 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|alias-prefix-lifetime|tmm-traffic-event|lookup|0|0:0|6|result|binary-scan-tcl-value|0|1|7b3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617433375f616c6961735f7072656669785f6c69666574696d653a3a656d6974207b6120627d7d207b7b6120627d207b6320647d7d
Oct 12 08:43:18 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|alias-prefix-lifetime|tmm-traffic-event|runtime-cleanup|0|0:0|6|result|binary-scan-tcl-value|0|1|
Oct 12 08:43:18 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-binding-mutation|tmm-traffic-event|setup|0|0:0|6|result|binary-scan-tcl-value|0|1|
Oct 12 08:43:18 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-binding-mutation|tmm-traffic-event|lookup|0|0:0|6|result|binary-scan-tcl-value|0|1|7265706c6163656d656e74
Oct 12 08:43:18 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-binding-mutation|tmm-traffic-event|runtime-cleanup|0|0:0|6|result|binary-scan-tcl-value|0|1|
Oct 12 08:43:18 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-qualified|tmm-traffic-event|setup|0|0:0|6|result|binary-scan-tcl-value|0|1|3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617433375f636f6d70757465645f7175616c69666965643a3a656d6974
Oct 12 08:43:18 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-qualified|tmm-traffic-event|lookup|0|0:0|6|result|binary-scan-tcl-value|0|1|6669727374
Oct 12 08:43:18 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-qualified|tmm-traffic-event|runtime-cleanup|0|0:0|6|result|binary-scan-tcl-value|0|1|
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- alias-prefix-lifetime, setup, completion 0, result, chunk 0 of 1:

```text-visible
::__tcl_lsp_probe_nh20261012pat37_alias_prefix_lifetime::emit {a b}⟦EOF⟧
```

- alias-prefix-lifetime, lookup, completion 0, result, chunk 0 of 1:

```text-visible
{::__tcl_lsp_probe_nh20261012pat37_alias_prefix_lifetime::emit {a b}} {{a b} {c d}}⟦EOF⟧
```

- alias-prefix-lifetime, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- computed-binding-mutation, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- computed-binding-mutation, lookup, completion 0, result, chunk 0 of 1:

```text-visible
replacement⟦EOF⟧
```

- computed-binding-mutation, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- computed-qualified, setup, completion 0, result, chunk 0 of 1:

```text-visible
::__tcl_lsp_probe_nh20261012pat37_computed_qualified::emit⟦EOF⟧
```

- computed-qualified, lookup, completion 0, result, chunk 0 of 1:

```text-visible
first⟦EOF⟧
```

- computed-qualified, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- alias-prefix-lifetime: intended operation reached.

- computed-binding-mutation: intended operation reached.

- computed-qualified: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 7

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:43:18 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|alias-prefix-lifetime|tmm-traffic-event|setup|0|0:0|7|result|binary-scan-tcl-value|0|1|3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617433375f616c6961735f7072656669785f6c69666574696d653a3a656d6974207b6120627d
Oct 12 08:43:18 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|alias-prefix-lifetime|tmm-traffic-event|lookup|0|0:0|7|result|binary-scan-tcl-value|0|1|7b3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617433375f616c6961735f7072656669785f6c69666574696d653a3a656d6974207b6120627d7d207b7b6120627d207b6320647d7d
Oct 12 08:43:18 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|alias-prefix-lifetime|tmm-traffic-event|runtime-cleanup|0|0:0|7|result|binary-scan-tcl-value|0|1|
Oct 12 08:43:18 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-binding-mutation|tmm-traffic-event|setup|0|0:0|7|result|binary-scan-tcl-value|0|1|
Oct 12 08:43:18 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-binding-mutation|tmm-traffic-event|lookup|0|0:0|7|result|binary-scan-tcl-value|0|1|7265706c6163656d656e74
Oct 12 08:43:18 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-binding-mutation|tmm-traffic-event|runtime-cleanup|0|0:0|7|result|binary-scan-tcl-value|0|1|
Oct 12 08:43:18 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-qualified|tmm-traffic-event|setup|0|0:0|7|result|binary-scan-tcl-value|0|1|3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617433375f636f6d70757465645f7175616c69666965643a3a656d6974
Oct 12 08:43:18 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-qualified|tmm-traffic-event|lookup|0|0:0|7|result|binary-scan-tcl-value|0|1|6669727374
Oct 12 08:43:18 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-qualified|tmm-traffic-event|runtime-cleanup|0|0:0|7|result|binary-scan-tcl-value|0|1|
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- alias-prefix-lifetime, setup, completion 0, result, chunk 0 of 1:

```text-visible
::__tcl_lsp_probe_nh20261012pat37_alias_prefix_lifetime::emit {a b}⟦EOF⟧
```

- alias-prefix-lifetime, lookup, completion 0, result, chunk 0 of 1:

```text-visible
{::__tcl_lsp_probe_nh20261012pat37_alias_prefix_lifetime::emit {a b}} {{a b} {c d}}⟦EOF⟧
```

- alias-prefix-lifetime, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- computed-binding-mutation, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- computed-binding-mutation, lookup, completion 0, result, chunk 0 of 1:

```text-visible
replacement⟦EOF⟧
```

- computed-binding-mutation, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- computed-qualified, setup, completion 0, result, chunk 0 of 1:

```text-visible
::__tcl_lsp_probe_nh20261012pat37_computed_qualified::emit⟦EOF⟧
```

- computed-qualified, lookup, completion 0, result, chunk 0 of 1:

```text-visible
first⟦EOF⟧
```

- computed-qualified, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- alias-prefix-lifetime: intended operation reached.

- computed-binding-mutation: intended operation reached.

- computed-qualified: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 8

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:43:19 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|alias-prefix-lifetime|tmm-traffic-event|setup|0|0:0|8|result|binary-scan-tcl-value|0|1|3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617433375f616c6961735f7072656669785f6c69666574696d653a3a656d6974207b6120627d
Oct 12 08:43:19 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|alias-prefix-lifetime|tmm-traffic-event|lookup|0|0:0|8|result|binary-scan-tcl-value|0|1|7b3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617433375f616c6961735f7072656669785f6c69666574696d653a3a656d6974207b6120627d7d207b7b6120627d207b6320647d7d
Oct 12 08:43:19 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|alias-prefix-lifetime|tmm-traffic-event|runtime-cleanup|0|0:0|8|result|binary-scan-tcl-value|0|1|
Oct 12 08:43:19 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-binding-mutation|tmm-traffic-event|setup|0|0:0|8|result|binary-scan-tcl-value|0|1|
Oct 12 08:43:19 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-binding-mutation|tmm-traffic-event|lookup|0|0:0|8|result|binary-scan-tcl-value|0|1|7265706c6163656d656e74
Oct 12 08:43:19 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-binding-mutation|tmm-traffic-event|runtime-cleanup|0|0:0|8|result|binary-scan-tcl-value|0|1|
Oct 12 08:43:19 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-qualified|tmm-traffic-event|setup|0|0:0|8|result|binary-scan-tcl-value|0|1|3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617433375f636f6d70757465645f7175616c69666965643a3a656d6974
Oct 12 08:43:19 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-qualified|tmm-traffic-event|lookup|0|0:0|8|result|binary-scan-tcl-value|0|1|6669727374
Oct 12 08:43:19 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-qualified|tmm-traffic-event|runtime-cleanup|0|0:0|8|result|binary-scan-tcl-value|0|1|
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- alias-prefix-lifetime, setup, completion 0, result, chunk 0 of 1:

```text-visible
::__tcl_lsp_probe_nh20261012pat37_alias_prefix_lifetime::emit {a b}⟦EOF⟧
```

- alias-prefix-lifetime, lookup, completion 0, result, chunk 0 of 1:

```text-visible
{::__tcl_lsp_probe_nh20261012pat37_alias_prefix_lifetime::emit {a b}} {{a b} {c d}}⟦EOF⟧
```

- alias-prefix-lifetime, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- computed-binding-mutation, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- computed-binding-mutation, lookup, completion 0, result, chunk 0 of 1:

```text-visible
replacement⟦EOF⟧
```

- computed-binding-mutation, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- computed-qualified, setup, completion 0, result, chunk 0 of 1:

```text-visible
::__tcl_lsp_probe_nh20261012pat37_computed_qualified::emit⟦EOF⟧
```

- computed-qualified, lookup, completion 0, result, chunk 0 of 1:

```text-visible
first⟦EOF⟧
```

- computed-qualified, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- alias-prefix-lifetime: intended operation reached.

- computed-binding-mutation: intended operation reached.

- computed-qualified: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 9

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:43:19 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|alias-prefix-lifetime|tmm-traffic-event|setup|0|0:0|9|result|binary-scan-tcl-value|0|1|3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617433375f616c6961735f7072656669785f6c69666574696d653a3a656d6974207b6120627d
Oct 12 08:43:19 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|alias-prefix-lifetime|tmm-traffic-event|lookup|0|0:0|9|result|binary-scan-tcl-value|0|1|7b3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617433375f616c6961735f7072656669785f6c69666574696d653a3a656d6974207b6120627d7d207b7b6120627d207b6320647d7d
Oct 12 08:43:19 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|alias-prefix-lifetime|tmm-traffic-event|runtime-cleanup|0|0:0|9|result|binary-scan-tcl-value|0|1|
Oct 12 08:43:19 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-binding-mutation|tmm-traffic-event|setup|0|0:0|9|result|binary-scan-tcl-value|0|1|
Oct 12 08:43:19 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-binding-mutation|tmm-traffic-event|lookup|0|0:0|9|result|binary-scan-tcl-value|0|1|7265706c6163656d656e74
Oct 12 08:43:19 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-binding-mutation|tmm-traffic-event|runtime-cleanup|0|0:0|9|result|binary-scan-tcl-value|0|1|
Oct 12 08:43:19 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-qualified|tmm-traffic-event|setup|0|0:0|9|result|binary-scan-tcl-value|0|1|3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313270617433375f636f6d70757465645f7175616c69666965643a3a656d6974
Oct 12 08:43:19 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-qualified|tmm-traffic-event|lookup|0|0:0|9|result|binary-scan-tcl-value|0|1|6669727374
Oct 12 08:43:19 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012pat3_7_0 <HTTP_REQUEST>: NHO|nh20261012pat3|computed-qualified|tmm-traffic-event|runtime-cleanup|0|0:0|9|result|binary-scan-tcl-value|0|1|
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- alias-prefix-lifetime, setup, completion 0, result, chunk 0 of 1:

```text-visible
::__tcl_lsp_probe_nh20261012pat37_alias_prefix_lifetime::emit {a b}⟦EOF⟧
```

- alias-prefix-lifetime, lookup, completion 0, result, chunk 0 of 1:

```text-visible
{::__tcl_lsp_probe_nh20261012pat37_alias_prefix_lifetime::emit {a b}} {{a b} {c d}}⟦EOF⟧
```

- alias-prefix-lifetime, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- computed-binding-mutation, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- computed-binding-mutation, lookup, completion 0, result, chunk 0 of 1:

```text-visible
replacement⟦EOF⟧
```

- computed-binding-mutation, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- computed-qualified, setup, completion 0, result, chunk 0 of 1:

```text-visible
::__tcl_lsp_probe_nh20261012pat37_computed_qualified::emit⟦EOF⟧
```

- computed-qualified, lookup, completion 0, result, chunk 0 of 1:

```text-visible
first⟦EOF⟧
```

- computed-qualified, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- alias-prefix-lifetime: intended operation reached.

- computed-binding-mutation: intended operation reached.

- computed-qualified: intended operation reached.
