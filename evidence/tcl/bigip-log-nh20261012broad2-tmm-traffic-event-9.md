# BIG-IP Tcl log: nh20261012broad2, tmm-traffic-event-9

## Question

The following cases investigate Tcl behavior relevant to name resolution and diagnostics in the LSP, compiler and VM. Each observation is limited to its own case and recorded context.

- **import-chain-origin:** Import/export/force/forget/origin support and lifecycle; global-origin imports, source rename/redefinition/deletion, chains and conflicts.

- **import-force-conflict:** Import/export/force/forget/origin support and lifecycle; global-origin imports, source rename/redefinition/deletion, chains and conflicts.

- **import-source-deleted:** Import/export/force/forget/origin support and lifecycle; global-origin imports, source rename/redefinition/deletion, chains and conflicts.

## Assumptions

- **import-chain-origin:** Two import edges with owned namespaces.
- **import-force-conflict:** Force replaces only owned consumer command.
- **import-source-deleted:** Delete owned source command, then invoke imported tail.

Other assumptions were not separately recorded.

## Script

```tcl-visible
set nh_options_available unavailable
set nh_return_options {}
set nh_unit "[TMM::cmp_group]:[TMM::cmp_unit]"
set nh_invocation once
set nh_invocation [HTTP::header value X-NH-Invocation]
set nh_setup [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f736f757263655f64656c65746564207b7d0a6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f736f757263655f64656c657465643a3a736f75726365207b70726f632070207b7d207b72657475726e204f4c447d3b206e616d657370616365206578706f727420707d0a6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f736f757263655f64656c657465643a3a636f6e73756d6572207b6e616d65737061636520696d706f7274203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f736f757263655f64656c657465643a3a736f757263653a3a707d0a72656e616d65203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f736f757263655f64656c657465643a3a736f757263653a3a70207b7d0a]
set nh_operation [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f736f757263655f64656c657465643a3a636f6e73756d6572207b707d0a]
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
        log local0. "NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 666f7265616368206e685f6368696c64205b696e7465727020736c617665735d207b6966207b5b737472696e67206d61746368205f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f736f757263655f64656c657465642a20246e685f6368696c645d7d207b696e746572702064656c65746520246e685f6368696c647d7d0a666f7265616368206e685f6e73205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f736f757263655f64656c657465642a5d207b6e616d6573706163652064656c65746520246e685f6e737d0a666f7265616368206e685f636d64205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f736f757263655f64656c657465642a5d207b72656e616d6520246e685f636d64207b7d7d0a666f7265616368206e685f766172205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f736f757263655f64656c657465642a5d207b756e736574202d6e6f636f6d706c61696e20246e685f7661727d0a6c697374205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f736f757263655f64656c657465642a5d205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f736f757263655f64656c657465642a5d205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f736f757263655f64656c657465642a5d]
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
        log local0. "NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
set nh_setup [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f666f7263655f636f6e666c696374207b7d0a6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f666f7263655f636f6e666c6963743a3a736f75726365207b70726f632070207b7d207b72657475726e20534f555243457d3b206e616d657370616365206578706f727420707d0a6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f666f7263655f636f6e666c6963743a3a636f6e73756d6572207b70726f632070207b7d207b72657475726e204c4f43414c7d7d0a]
set nh_operation [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f666f7263655f636f6e666c6963743a3a636f6e73756d6572207b6e616d65737061636520696d706f7274202d666f726365203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f666f7263655f636f6e666c6963743a3a736f757263653a3a703b20707d0a]
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
        log local0. "NHO|nh20261012broad2|import-force-conflict|tmm-traffic-event|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012broad2|import-force-conflict|tmm-traffic-event|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 666f7265616368206e685f6368696c64205b696e7465727020736c617665735d207b6966207b5b737472696e67206d61746368205f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f666f7263655f636f6e666c6963742a20246e685f6368696c645d7d207b696e746572702064656c65746520246e685f6368696c647d7d0a666f7265616368206e685f6e73205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f666f7263655f636f6e666c6963742a5d207b6e616d6573706163652064656c65746520246e685f6e737d0a666f7265616368206e685f636d64205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f666f7263655f636f6e666c6963742a5d207b72656e616d6520246e685f636d64207b7d7d0a666f7265616368206e685f766172205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f666f7263655f636f6e666c6963742a5d207b756e736574202d6e6f636f6d706c61696e20246e685f7661727d0a6c697374205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f666f7263655f636f6e666c6963742a5d205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f666f7263655f636f6e666c6963742a5d205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f666f7263655f636f6e666c6963742a5d]
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
        log local0. "NHO|nh20261012broad2|import-force-conflict|tmm-traffic-event|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
set nh_setup [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f636861696e5f6f726967696e207b7d0a6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f636861696e5f6f726967696e3a3a736f75726365207b70726f632070207b7d207b72657475726e20534f555243457d3b206e616d657370616365206578706f727420707d0a6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f636861696e5f6f726967696e3a3a6d6964646c65207b6e616d65737061636520696d706f7274203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f636861696e5f6f726967696e3a3a736f757263653a3a703b206e616d657370616365206578706f727420707d0a6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f636861696e5f6f726967696e3a3a636f6e73756d6572207b6e616d65737061636520696d706f7274203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f636861696e5f6f726967696e3a3a6d6964646c653a3a707d0a]
set nh_operation [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f636861696e5f6f726967696e3a3a636f6e73756d6572207b6c697374205b705d205b6e616d657370616365206f726967696e20705d7d0a]
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
        log local0. "NHO|nh20261012broad2|import-chain-origin|tmm-traffic-event|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012broad2|import-chain-origin|tmm-traffic-event|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 666f7265616368206e685f6368696c64205b696e7465727020736c617665735d207b6966207b5b737472696e67206d61746368205f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f636861696e5f6f726967696e2a20246e685f6368696c645d7d207b696e746572702064656c65746520246e685f6368696c647d7d0a666f7265616368206e685f6e73205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f636861696e5f6f726967696e2a5d207b6e616d6573706163652064656c65746520246e685f6e737d0a666f7265616368206e685f636d64205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f636861696e5f6f726967696e2a5d207b72656e616d6520246e685f636d64207b7d7d0a666f7265616368206e685f766172205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f636861696e5f6f726967696e2a5d207b756e736574202d6e6f636f6d706c61696e20246e685f7661727d0a6c697374205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f636861696e5f6f726967696e2a5d205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f636861696e5f6f726967696e2a5d205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f636861696e5f6f726967696e2a5d]
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
        log local0. "NHO|nh20261012broad2|import-chain-origin|tmm-traffic-event|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
Oct 12 08:57:56 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|setup|0|0:0|0|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:56 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|lookup|1|0:0|0|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520227022
Oct 12 08:57:56 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|lookup|1|0:0|0|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:57:56 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|lookup|1|0:0|0|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:57:56 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|lookup|1|0:0|0|errorinfo|binary-scan-tcl-value|0|2|696e76616c696420636f6d6d616e64206e616d65202270220a202020207768696c6520657865637574696e670a2270220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f736f757263655f64656c657465643a3a636f6e73756d65722220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f
Oct 12 08:57:56 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|lookup|1|0:0|0|errorinfo|binary-scan-tcl-value|1|2|5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f736f757263655f64656c657465643a3a636f6e73756d6572207b707d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:57:56 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|runtime-cleanup|0|0:0|0|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:57:56 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-force-conflict|tmm-traffic-event|setup|0|0:0|0|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:56 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-force-conflict|tmm-traffic-event|lookup|0|0:0|0|result|binary-scan-tcl-value|0|1|534f55524345
Oct 12 08:57:56 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-force-conflict|tmm-traffic-event|runtime-cleanup|0|0:0|0|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:57:56 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-chain-origin|tmm-traffic-event|setup|0|0:0|0|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:56 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-chain-origin|tmm-traffic-event|lookup|0|0:0|0|result|binary-scan-tcl-value|0|1|534f55524345203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f636861696e5f6f726967696e3a3a736f757263653a3a70
Oct 12 08:57:56 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-chain-origin|tmm-traffic-event|runtime-cleanup|0|0:0|0|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- import-source-deleted, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "p"⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
invalid command name "p"
    while executing
"p"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_import_source_deleted::consumer" script line 1)
    invoked from within
"namespace eval ::_⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
_tcl_lsp_probe_nh20261012broad27_import_source_deleted::consumer {p}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- import-source-deleted, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- import-force-conflict, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- import-force-conflict, lookup, completion 0, result, chunk 0 of 1:

```text-visible
SOURCE⟦EOF⟧
```

- import-force-conflict, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- import-chain-origin, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- import-chain-origin, lookup, completion 0, result, chunk 0 of 1:

```text-visible
SOURCE ::__tcl_lsp_probe_nh20261012broad27_import_chain_origin::source::p⟦EOF⟧
```

- import-chain-origin, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- import-chain-origin: intended operation reached.

- import-force-conflict: intended operation reached.

- import-source-deleted: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 1

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:57:56 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|setup|0|0:0|1|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:56 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|lookup|1|0:0|1|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520227022
Oct 12 08:57:56 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|lookup|1|0:0|1|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:57:56 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|lookup|1|0:0|1|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:57:56 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|lookup|1|0:0|1|errorinfo|binary-scan-tcl-value|0|2|696e76616c696420636f6d6d616e64206e616d65202270220a202020207768696c6520657865637574696e670a2270220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f736f757263655f64656c657465643a3a636f6e73756d65722220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f
Oct 12 08:57:56 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|lookup|1|0:0|1|errorinfo|binary-scan-tcl-value|1|2|5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f736f757263655f64656c657465643a3a636f6e73756d6572207b707d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:57:56 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|runtime-cleanup|0|0:0|1|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:57:56 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-force-conflict|tmm-traffic-event|setup|0|0:0|1|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:56 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-force-conflict|tmm-traffic-event|lookup|0|0:0|1|result|binary-scan-tcl-value|0|1|534f55524345
Oct 12 08:57:56 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-force-conflict|tmm-traffic-event|runtime-cleanup|0|0:0|1|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:57:56 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-chain-origin|tmm-traffic-event|setup|0|0:0|1|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:56 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-chain-origin|tmm-traffic-event|lookup|0|0:0|1|result|binary-scan-tcl-value|0|1|534f55524345203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f636861696e5f6f726967696e3a3a736f757263653a3a70
Oct 12 08:57:56 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-chain-origin|tmm-traffic-event|runtime-cleanup|0|0:0|1|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- import-source-deleted, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "p"⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
invalid command name "p"
    while executing
"p"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_import_source_deleted::consumer" script line 1)
    invoked from within
"namespace eval ::_⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
_tcl_lsp_probe_nh20261012broad27_import_source_deleted::consumer {p}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- import-source-deleted, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- import-force-conflict, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- import-force-conflict, lookup, completion 0, result, chunk 0 of 1:

```text-visible
SOURCE⟦EOF⟧
```

- import-force-conflict, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- import-chain-origin, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- import-chain-origin, lookup, completion 0, result, chunk 0 of 1:

```text-visible
SOURCE ::__tcl_lsp_probe_nh20261012broad27_import_chain_origin::source::p⟦EOF⟧
```

- import-chain-origin, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- import-chain-origin: intended operation reached.

- import-force-conflict: intended operation reached.

- import-source-deleted: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 10

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|setup|0|0:0|10|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|lookup|1|0:0|10|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520227022
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|lookup|1|0:0|10|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|lookup|1|0:0|10|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|lookup|1|0:0|10|errorinfo|binary-scan-tcl-value|0|2|696e76616c696420636f6d6d616e64206e616d65202270220a202020207768696c6520657865637574696e670a2270220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f736f757263655f64656c657465643a3a636f6e73756d65722220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|lookup|1|0:0|10|errorinfo|binary-scan-tcl-value|1|2|5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f736f757263655f64656c657465643a3a636f6e73756d6572207b707d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|runtime-cleanup|0|0:0|10|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-force-conflict|tmm-traffic-event|setup|0|0:0|10|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-force-conflict|tmm-traffic-event|lookup|0|0:0|10|result|binary-scan-tcl-value|0|1|534f55524345
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-force-conflict|tmm-traffic-event|runtime-cleanup|0|0:0|10|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-chain-origin|tmm-traffic-event|setup|0|0:0|10|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-chain-origin|tmm-traffic-event|lookup|0|0:0|10|result|binary-scan-tcl-value|0|1|534f55524345203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f636861696e5f6f726967696e3a3a736f757263653a3a70
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-chain-origin|tmm-traffic-event|runtime-cleanup|0|0:0|10|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- import-source-deleted, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "p"⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
invalid command name "p"
    while executing
"p"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_import_source_deleted::consumer" script line 1)
    invoked from within
"namespace eval ::_⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
_tcl_lsp_probe_nh20261012broad27_import_source_deleted::consumer {p}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- import-source-deleted, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- import-force-conflict, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- import-force-conflict, lookup, completion 0, result, chunk 0 of 1:

```text-visible
SOURCE⟦EOF⟧
```

- import-force-conflict, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- import-chain-origin, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- import-chain-origin, lookup, completion 0, result, chunk 0 of 1:

```text-visible
SOURCE ::__tcl_lsp_probe_nh20261012broad27_import_chain_origin::source::p⟦EOF⟧
```

- import-chain-origin, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- import-chain-origin: intended operation reached.

- import-force-conflict: intended operation reached.

- import-source-deleted: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 11

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:57:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|setup|0|0:0|11|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|lookup|1|0:0|11|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520227022
Oct 12 08:57:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|lookup|1|0:0|11|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:57:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|lookup|1|0:0|11|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:57:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|lookup|1|0:0|11|errorinfo|binary-scan-tcl-value|0|2|696e76616c696420636f6d6d616e64206e616d65202270220a202020207768696c6520657865637574696e670a2270220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f736f757263655f64656c657465643a3a636f6e73756d65722220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f
Oct 12 08:57:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|lookup|1|0:0|11|errorinfo|binary-scan-tcl-value|1|2|5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f736f757263655f64656c657465643a3a636f6e73756d6572207b707d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:57:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|runtime-cleanup|0|0:0|11|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:57:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-force-conflict|tmm-traffic-event|setup|0|0:0|11|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-force-conflict|tmm-traffic-event|lookup|0|0:0|11|result|binary-scan-tcl-value|0|1|534f55524345
Oct 12 08:57:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-force-conflict|tmm-traffic-event|runtime-cleanup|0|0:0|11|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:57:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-chain-origin|tmm-traffic-event|setup|0|0:0|11|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-chain-origin|tmm-traffic-event|lookup|0|0:0|11|result|binary-scan-tcl-value|0|1|534f55524345203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f636861696e5f6f726967696e3a3a736f757263653a3a70
Oct 12 08:57:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-chain-origin|tmm-traffic-event|runtime-cleanup|0|0:0|11|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- import-source-deleted, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "p"⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
invalid command name "p"
    while executing
"p"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_import_source_deleted::consumer" script line 1)
    invoked from within
"namespace eval ::_⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
_tcl_lsp_probe_nh20261012broad27_import_source_deleted::consumer {p}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- import-source-deleted, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- import-force-conflict, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- import-force-conflict, lookup, completion 0, result, chunk 0 of 1:

```text-visible
SOURCE⟦EOF⟧
```

- import-force-conflict, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- import-chain-origin, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- import-chain-origin, lookup, completion 0, result, chunk 0 of 1:

```text-visible
SOURCE ::__tcl_lsp_probe_nh20261012broad27_import_chain_origin::source::p⟦EOF⟧
```

- import-chain-origin, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- import-chain-origin: intended operation reached.

- import-force-conflict: intended operation reached.

- import-source-deleted: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 12

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:57:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|setup|0|0:0|12|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|lookup|1|0:0|12|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520227022
Oct 12 08:57:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|lookup|1|0:0|12|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:57:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|lookup|1|0:0|12|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:57:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|lookup|1|0:0|12|errorinfo|binary-scan-tcl-value|0|2|696e76616c696420636f6d6d616e64206e616d65202270220a202020207768696c6520657865637574696e670a2270220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f736f757263655f64656c657465643a3a636f6e73756d65722220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f
Oct 12 08:57:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|lookup|1|0:0|12|errorinfo|binary-scan-tcl-value|1|2|5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f736f757263655f64656c657465643a3a636f6e73756d6572207b707d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:57:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|runtime-cleanup|0|0:0|12|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:57:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-force-conflict|tmm-traffic-event|setup|0|0:0|12|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-force-conflict|tmm-traffic-event|lookup|0|0:0|12|result|binary-scan-tcl-value|0|1|534f55524345
Oct 12 08:57:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-force-conflict|tmm-traffic-event|runtime-cleanup|0|0:0|12|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:57:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-chain-origin|tmm-traffic-event|setup|0|0:0|12|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-chain-origin|tmm-traffic-event|lookup|0|0:0|12|result|binary-scan-tcl-value|0|1|534f55524345203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f636861696e5f6f726967696e3a3a736f757263653a3a70
Oct 12 08:57:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-chain-origin|tmm-traffic-event|runtime-cleanup|0|0:0|12|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- import-source-deleted, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "p"⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
invalid command name "p"
    while executing
"p"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_import_source_deleted::consumer" script line 1)
    invoked from within
"namespace eval ::_⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
_tcl_lsp_probe_nh20261012broad27_import_source_deleted::consumer {p}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- import-source-deleted, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- import-force-conflict, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- import-force-conflict, lookup, completion 0, result, chunk 0 of 1:

```text-visible
SOURCE⟦EOF⟧
```

- import-force-conflict, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- import-chain-origin, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- import-chain-origin, lookup, completion 0, result, chunk 0 of 1:

```text-visible
SOURCE ::__tcl_lsp_probe_nh20261012broad27_import_chain_origin::source::p⟦EOF⟧
```

- import-chain-origin, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- import-chain-origin: intended operation reached.

- import-force-conflict: intended operation reached.

- import-source-deleted: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 13

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:57:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|setup|0|0:0|13|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|lookup|1|0:0|13|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520227022
Oct 12 08:57:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|lookup|1|0:0|13|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:57:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|lookup|1|0:0|13|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:57:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|lookup|1|0:0|13|errorinfo|binary-scan-tcl-value|0|2|696e76616c696420636f6d6d616e64206e616d65202270220a202020207768696c6520657865637574696e670a2270220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f736f757263655f64656c657465643a3a636f6e73756d65722220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f
Oct 12 08:57:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|lookup|1|0:0|13|errorinfo|binary-scan-tcl-value|1|2|5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f736f757263655f64656c657465643a3a636f6e73756d6572207b707d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:57:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|runtime-cleanup|0|0:0|13|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:57:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-force-conflict|tmm-traffic-event|setup|0|0:0|13|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-force-conflict|tmm-traffic-event|lookup|0|0:0|13|result|binary-scan-tcl-value|0|1|534f55524345
Oct 12 08:57:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-force-conflict|tmm-traffic-event|runtime-cleanup|0|0:0|13|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:57:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-chain-origin|tmm-traffic-event|setup|0|0:0|13|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-chain-origin|tmm-traffic-event|lookup|0|0:0|13|result|binary-scan-tcl-value|0|1|534f55524345203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f636861696e5f6f726967696e3a3a736f757263653a3a70
Oct 12 08:57:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-chain-origin|tmm-traffic-event|runtime-cleanup|0|0:0|13|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- import-source-deleted, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "p"⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
invalid command name "p"
    while executing
"p"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_import_source_deleted::consumer" script line 1)
    invoked from within
"namespace eval ::_⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
_tcl_lsp_probe_nh20261012broad27_import_source_deleted::consumer {p}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- import-source-deleted, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- import-force-conflict, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- import-force-conflict, lookup, completion 0, result, chunk 0 of 1:

```text-visible
SOURCE⟦EOF⟧
```

- import-force-conflict, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- import-chain-origin, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- import-chain-origin, lookup, completion 0, result, chunk 0 of 1:

```text-visible
SOURCE ::__tcl_lsp_probe_nh20261012broad27_import_chain_origin::source::p⟦EOF⟧
```

- import-chain-origin, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- import-chain-origin: intended operation reached.

- import-force-conflict: intended operation reached.

- import-source-deleted: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 14

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:57:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|setup|0|0:0|14|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|lookup|1|0:0|14|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520227022
Oct 12 08:57:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|lookup|1|0:0|14|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:57:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|lookup|1|0:0|14|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:57:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|lookup|1|0:0|14|errorinfo|binary-scan-tcl-value|0|2|696e76616c696420636f6d6d616e64206e616d65202270220a202020207768696c6520657865637574696e670a2270220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f736f757263655f64656c657465643a3a636f6e73756d65722220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f
Oct 12 08:57:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|lookup|1|0:0|14|errorinfo|binary-scan-tcl-value|1|2|5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f736f757263655f64656c657465643a3a636f6e73756d6572207b707d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:57:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|runtime-cleanup|0|0:0|14|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:57:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-force-conflict|tmm-traffic-event|setup|0|0:0|14|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-force-conflict|tmm-traffic-event|lookup|0|0:0|14|result|binary-scan-tcl-value|0|1|534f55524345
Oct 12 08:57:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-force-conflict|tmm-traffic-event|runtime-cleanup|0|0:0|14|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:57:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-chain-origin|tmm-traffic-event|setup|0|0:0|14|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-chain-origin|tmm-traffic-event|lookup|0|0:0|14|result|binary-scan-tcl-value|0|1|534f55524345203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f636861696e5f6f726967696e3a3a736f757263653a3a70
Oct 12 08:57:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-chain-origin|tmm-traffic-event|runtime-cleanup|0|0:0|14|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- import-source-deleted, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "p"⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
invalid command name "p"
    while executing
"p"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_import_source_deleted::consumer" script line 1)
    invoked from within
"namespace eval ::_⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
_tcl_lsp_probe_nh20261012broad27_import_source_deleted::consumer {p}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- import-source-deleted, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- import-force-conflict, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- import-force-conflict, lookup, completion 0, result, chunk 0 of 1:

```text-visible
SOURCE⟦EOF⟧
```

- import-force-conflict, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- import-chain-origin, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- import-chain-origin, lookup, completion 0, result, chunk 0 of 1:

```text-visible
SOURCE ::__tcl_lsp_probe_nh20261012broad27_import_chain_origin::source::p⟦EOF⟧
```

- import-chain-origin, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- import-chain-origin: intended operation reached.

- import-force-conflict: intended operation reached.

- import-source-deleted: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 15

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:57:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|setup|0|0:0|15|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|lookup|1|0:0|15|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520227022
Oct 12 08:57:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|lookup|1|0:0|15|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:57:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|lookup|1|0:0|15|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:57:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|lookup|1|0:0|15|errorinfo|binary-scan-tcl-value|0|2|696e76616c696420636f6d6d616e64206e616d65202270220a202020207768696c6520657865637574696e670a2270220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f736f757263655f64656c657465643a3a636f6e73756d65722220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f
Oct 12 08:57:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|lookup|1|0:0|15|errorinfo|binary-scan-tcl-value|1|2|5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f736f757263655f64656c657465643a3a636f6e73756d6572207b707d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:57:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|runtime-cleanup|0|0:0|15|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:57:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-force-conflict|tmm-traffic-event|setup|0|0:0|15|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-force-conflict|tmm-traffic-event|lookup|0|0:0|15|result|binary-scan-tcl-value|0|1|534f55524345
Oct 12 08:57:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-force-conflict|tmm-traffic-event|runtime-cleanup|0|0:0|15|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:57:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-chain-origin|tmm-traffic-event|setup|0|0:0|15|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-chain-origin|tmm-traffic-event|lookup|0|0:0|15|result|binary-scan-tcl-value|0|1|534f55524345203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f636861696e5f6f726967696e3a3a736f757263653a3a70
Oct 12 08:57:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-chain-origin|tmm-traffic-event|runtime-cleanup|0|0:0|15|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- import-source-deleted, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "p"⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
invalid command name "p"
    while executing
"p"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_import_source_deleted::consumer" script line 1)
    invoked from within
"namespace eval ::_⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
_tcl_lsp_probe_nh20261012broad27_import_source_deleted::consumer {p}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- import-source-deleted, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- import-force-conflict, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- import-force-conflict, lookup, completion 0, result, chunk 0 of 1:

```text-visible
SOURCE⟦EOF⟧
```

- import-force-conflict, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- import-chain-origin, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- import-chain-origin, lookup, completion 0, result, chunk 0 of 1:

```text-visible
SOURCE ::__tcl_lsp_probe_nh20261012broad27_import_chain_origin::source::p⟦EOF⟧
```

- import-chain-origin, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- import-chain-origin: intended operation reached.

- import-force-conflict: intended operation reached.

- import-source-deleted: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 2

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:57:56 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|setup|0|0:0|2|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:56 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|lookup|1|0:0|2|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520227022
Oct 12 08:57:56 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|lookup|1|0:0|2|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:57:56 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|lookup|1|0:0|2|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:57:56 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|lookup|1|0:0|2|errorinfo|binary-scan-tcl-value|0|2|696e76616c696420636f6d6d616e64206e616d65202270220a202020207768696c6520657865637574696e670a2270220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f736f757263655f64656c657465643a3a636f6e73756d65722220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f
Oct 12 08:57:56 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|lookup|1|0:0|2|errorinfo|binary-scan-tcl-value|1|2|5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f736f757263655f64656c657465643a3a636f6e73756d6572207b707d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:57:56 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|runtime-cleanup|0|0:0|2|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:57:56 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-force-conflict|tmm-traffic-event|setup|0|0:0|2|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:56 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-force-conflict|tmm-traffic-event|lookup|0|0:0|2|result|binary-scan-tcl-value|0|1|534f55524345
Oct 12 08:57:56 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-force-conflict|tmm-traffic-event|runtime-cleanup|0|0:0|2|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:57:56 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-chain-origin|tmm-traffic-event|setup|0|0:0|2|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:56 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-chain-origin|tmm-traffic-event|lookup|0|0:0|2|result|binary-scan-tcl-value|0|1|534f55524345203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f636861696e5f6f726967696e3a3a736f757263653a3a70
Oct 12 08:57:56 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-chain-origin|tmm-traffic-event|runtime-cleanup|0|0:0|2|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- import-source-deleted, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "p"⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
invalid command name "p"
    while executing
"p"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_import_source_deleted::consumer" script line 1)
    invoked from within
"namespace eval ::_⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
_tcl_lsp_probe_nh20261012broad27_import_source_deleted::consumer {p}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- import-source-deleted, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- import-force-conflict, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- import-force-conflict, lookup, completion 0, result, chunk 0 of 1:

```text-visible
SOURCE⟦EOF⟧
```

- import-force-conflict, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- import-chain-origin, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- import-chain-origin, lookup, completion 0, result, chunk 0 of 1:

```text-visible
SOURCE ::__tcl_lsp_probe_nh20261012broad27_import_chain_origin::source::p⟦EOF⟧
```

- import-chain-origin, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- import-chain-origin: intended operation reached.

- import-force-conflict: intended operation reached.

- import-source-deleted: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 3

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:57:56 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|setup|0|0:0|3|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:56 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|lookup|1|0:0|3|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520227022
Oct 12 08:57:56 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|lookup|1|0:0|3|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:57:56 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|lookup|1|0:0|3|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:57:56 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|lookup|1|0:0|3|errorinfo|binary-scan-tcl-value|0|2|696e76616c696420636f6d6d616e64206e616d65202270220a202020207768696c6520657865637574696e670a2270220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f736f757263655f64656c657465643a3a636f6e73756d65722220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f
Oct 12 08:57:56 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|lookup|1|0:0|3|errorinfo|binary-scan-tcl-value|1|2|5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f736f757263655f64656c657465643a3a636f6e73756d6572207b707d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:57:56 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|runtime-cleanup|0|0:0|3|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:57:56 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-force-conflict|tmm-traffic-event|setup|0|0:0|3|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:56 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-force-conflict|tmm-traffic-event|lookup|0|0:0|3|result|binary-scan-tcl-value|0|1|534f55524345
Oct 12 08:57:56 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-force-conflict|tmm-traffic-event|runtime-cleanup|0|0:0|3|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:57:56 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-chain-origin|tmm-traffic-event|setup|0|0:0|3|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:56 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-chain-origin|tmm-traffic-event|lookup|0|0:0|3|result|binary-scan-tcl-value|0|1|534f55524345203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f636861696e5f6f726967696e3a3a736f757263653a3a70
Oct 12 08:57:56 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-chain-origin|tmm-traffic-event|runtime-cleanup|0|0:0|3|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- import-source-deleted, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "p"⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
invalid command name "p"
    while executing
"p"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_import_source_deleted::consumer" script line 1)
    invoked from within
"namespace eval ::_⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
_tcl_lsp_probe_nh20261012broad27_import_source_deleted::consumer {p}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- import-source-deleted, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- import-force-conflict, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- import-force-conflict, lookup, completion 0, result, chunk 0 of 1:

```text-visible
SOURCE⟦EOF⟧
```

- import-force-conflict, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- import-chain-origin, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- import-chain-origin, lookup, completion 0, result, chunk 0 of 1:

```text-visible
SOURCE ::__tcl_lsp_probe_nh20261012broad27_import_chain_origin::source::p⟦EOF⟧
```

- import-chain-origin, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- import-chain-origin: intended operation reached.

- import-force-conflict: intended operation reached.

- import-source-deleted: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 4

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|setup|0|0:0|4|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|lookup|1|0:0|4|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520227022
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|lookup|1|0:0|4|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|lookup|1|0:0|4|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|lookup|1|0:0|4|errorinfo|binary-scan-tcl-value|0|2|696e76616c696420636f6d6d616e64206e616d65202270220a202020207768696c6520657865637574696e670a2270220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f736f757263655f64656c657465643a3a636f6e73756d65722220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|lookup|1|0:0|4|errorinfo|binary-scan-tcl-value|1|2|5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f736f757263655f64656c657465643a3a636f6e73756d6572207b707d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|runtime-cleanup|0|0:0|4|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-force-conflict|tmm-traffic-event|setup|0|0:0|4|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-force-conflict|tmm-traffic-event|lookup|0|0:0|4|result|binary-scan-tcl-value|0|1|534f55524345
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-force-conflict|tmm-traffic-event|runtime-cleanup|0|0:0|4|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-chain-origin|tmm-traffic-event|setup|0|0:0|4|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-chain-origin|tmm-traffic-event|lookup|0|0:0|4|result|binary-scan-tcl-value|0|1|534f55524345203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f636861696e5f6f726967696e3a3a736f757263653a3a70
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-chain-origin|tmm-traffic-event|runtime-cleanup|0|0:0|4|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- import-source-deleted, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "p"⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
invalid command name "p"
    while executing
"p"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_import_source_deleted::consumer" script line 1)
    invoked from within
"namespace eval ::_⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
_tcl_lsp_probe_nh20261012broad27_import_source_deleted::consumer {p}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- import-source-deleted, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- import-force-conflict, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- import-force-conflict, lookup, completion 0, result, chunk 0 of 1:

```text-visible
SOURCE⟦EOF⟧
```

- import-force-conflict, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- import-chain-origin, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- import-chain-origin, lookup, completion 0, result, chunk 0 of 1:

```text-visible
SOURCE ::__tcl_lsp_probe_nh20261012broad27_import_chain_origin::source::p⟦EOF⟧
```

- import-chain-origin, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- import-chain-origin: intended operation reached.

- import-force-conflict: intended operation reached.

- import-source-deleted: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 5

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|setup|0|0:0|5|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|lookup|1|0:0|5|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520227022
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|lookup|1|0:0|5|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|lookup|1|0:0|5|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|lookup|1|0:0|5|errorinfo|binary-scan-tcl-value|0|2|696e76616c696420636f6d6d616e64206e616d65202270220a202020207768696c6520657865637574696e670a2270220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f736f757263655f64656c657465643a3a636f6e73756d65722220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|lookup|1|0:0|5|errorinfo|binary-scan-tcl-value|1|2|5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f736f757263655f64656c657465643a3a636f6e73756d6572207b707d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|runtime-cleanup|0|0:0|5|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-force-conflict|tmm-traffic-event|setup|0|0:0|5|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-force-conflict|tmm-traffic-event|lookup|0|0:0|5|result|binary-scan-tcl-value|0|1|534f55524345
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-force-conflict|tmm-traffic-event|runtime-cleanup|0|0:0|5|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-chain-origin|tmm-traffic-event|setup|0|0:0|5|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-chain-origin|tmm-traffic-event|lookup|0|0:0|5|result|binary-scan-tcl-value|0|1|534f55524345203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f636861696e5f6f726967696e3a3a736f757263653a3a70
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-chain-origin|tmm-traffic-event|runtime-cleanup|0|0:0|5|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- import-source-deleted, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "p"⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
invalid command name "p"
    while executing
"p"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_import_source_deleted::consumer" script line 1)
    invoked from within
"namespace eval ::_⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
_tcl_lsp_probe_nh20261012broad27_import_source_deleted::consumer {p}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- import-source-deleted, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- import-force-conflict, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- import-force-conflict, lookup, completion 0, result, chunk 0 of 1:

```text-visible
SOURCE⟦EOF⟧
```

- import-force-conflict, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- import-chain-origin, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- import-chain-origin, lookup, completion 0, result, chunk 0 of 1:

```text-visible
SOURCE ::__tcl_lsp_probe_nh20261012broad27_import_chain_origin::source::p⟦EOF⟧
```

- import-chain-origin, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- import-chain-origin: intended operation reached.

- import-force-conflict: intended operation reached.

- import-source-deleted: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 6

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|setup|0|0:0|6|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|lookup|1|0:0|6|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520227022
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|lookup|1|0:0|6|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|lookup|1|0:0|6|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|lookup|1|0:0|6|errorinfo|binary-scan-tcl-value|0|2|696e76616c696420636f6d6d616e64206e616d65202270220a202020207768696c6520657865637574696e670a2270220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f736f757263655f64656c657465643a3a636f6e73756d65722220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|lookup|1|0:0|6|errorinfo|binary-scan-tcl-value|1|2|5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f736f757263655f64656c657465643a3a636f6e73756d6572207b707d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|runtime-cleanup|0|0:0|6|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-force-conflict|tmm-traffic-event|setup|0|0:0|6|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-force-conflict|tmm-traffic-event|lookup|0|0:0|6|result|binary-scan-tcl-value|0|1|534f55524345
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-force-conflict|tmm-traffic-event|runtime-cleanup|0|0:0|6|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-chain-origin|tmm-traffic-event|setup|0|0:0|6|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-chain-origin|tmm-traffic-event|lookup|0|0:0|6|result|binary-scan-tcl-value|0|1|534f55524345203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f636861696e5f6f726967696e3a3a736f757263653a3a70
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-chain-origin|tmm-traffic-event|runtime-cleanup|0|0:0|6|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- import-source-deleted, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "p"⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
invalid command name "p"
    while executing
"p"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_import_source_deleted::consumer" script line 1)
    invoked from within
"namespace eval ::_⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
_tcl_lsp_probe_nh20261012broad27_import_source_deleted::consumer {p}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- import-source-deleted, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- import-force-conflict, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- import-force-conflict, lookup, completion 0, result, chunk 0 of 1:

```text-visible
SOURCE⟦EOF⟧
```

- import-force-conflict, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- import-chain-origin, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- import-chain-origin, lookup, completion 0, result, chunk 0 of 1:

```text-visible
SOURCE ::__tcl_lsp_probe_nh20261012broad27_import_chain_origin::source::p⟦EOF⟧
```

- import-chain-origin, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- import-chain-origin: intended operation reached.

- import-force-conflict: intended operation reached.

- import-source-deleted: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 7

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|setup|0|0:0|7|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|lookup|1|0:0|7|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520227022
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|lookup|1|0:0|7|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|lookup|1|0:0|7|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|lookup|1|0:0|7|errorinfo|binary-scan-tcl-value|0|2|696e76616c696420636f6d6d616e64206e616d65202270220a202020207768696c6520657865637574696e670a2270220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f736f757263655f64656c657465643a3a636f6e73756d65722220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|lookup|1|0:0|7|errorinfo|binary-scan-tcl-value|1|2|5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f736f757263655f64656c657465643a3a636f6e73756d6572207b707d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|runtime-cleanup|0|0:0|7|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-force-conflict|tmm-traffic-event|setup|0|0:0|7|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-force-conflict|tmm-traffic-event|lookup|0|0:0|7|result|binary-scan-tcl-value|0|1|534f55524345
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-force-conflict|tmm-traffic-event|runtime-cleanup|0|0:0|7|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-chain-origin|tmm-traffic-event|setup|0|0:0|7|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-chain-origin|tmm-traffic-event|lookup|0|0:0|7|result|binary-scan-tcl-value|0|1|534f55524345203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f636861696e5f6f726967696e3a3a736f757263653a3a70
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-chain-origin|tmm-traffic-event|runtime-cleanup|0|0:0|7|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- import-source-deleted, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "p"⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
invalid command name "p"
    while executing
"p"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_import_source_deleted::consumer" script line 1)
    invoked from within
"namespace eval ::_⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
_tcl_lsp_probe_nh20261012broad27_import_source_deleted::consumer {p}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- import-source-deleted, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- import-force-conflict, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- import-force-conflict, lookup, completion 0, result, chunk 0 of 1:

```text-visible
SOURCE⟦EOF⟧
```

- import-force-conflict, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- import-chain-origin, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- import-chain-origin, lookup, completion 0, result, chunk 0 of 1:

```text-visible
SOURCE ::__tcl_lsp_probe_nh20261012broad27_import_chain_origin::source::p⟦EOF⟧
```

- import-chain-origin, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- import-chain-origin: intended operation reached.

- import-force-conflict: intended operation reached.

- import-source-deleted: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 8

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|setup|0|0:0|8|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|lookup|1|0:0|8|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520227022
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|lookup|1|0:0|8|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|lookup|1|0:0|8|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|lookup|1|0:0|8|errorinfo|binary-scan-tcl-value|0|2|696e76616c696420636f6d6d616e64206e616d65202270220a202020207768696c6520657865637574696e670a2270220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f736f757263655f64656c657465643a3a636f6e73756d65722220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|lookup|1|0:0|8|errorinfo|binary-scan-tcl-value|1|2|5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f736f757263655f64656c657465643a3a636f6e73756d6572207b707d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|runtime-cleanup|0|0:0|8|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-force-conflict|tmm-traffic-event|setup|0|0:0|8|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-force-conflict|tmm-traffic-event|lookup|0|0:0|8|result|binary-scan-tcl-value|0|1|534f55524345
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-force-conflict|tmm-traffic-event|runtime-cleanup|0|0:0|8|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-chain-origin|tmm-traffic-event|setup|0|0:0|8|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-chain-origin|tmm-traffic-event|lookup|0|0:0|8|result|binary-scan-tcl-value|0|1|534f55524345203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f636861696e5f6f726967696e3a3a736f757263653a3a70
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-chain-origin|tmm-traffic-event|runtime-cleanup|0|0:0|8|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- import-source-deleted, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "p"⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
invalid command name "p"
    while executing
"p"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_import_source_deleted::consumer" script line 1)
    invoked from within
"namespace eval ::_⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
_tcl_lsp_probe_nh20261012broad27_import_source_deleted::consumer {p}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- import-source-deleted, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- import-force-conflict, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- import-force-conflict, lookup, completion 0, result, chunk 0 of 1:

```text-visible
SOURCE⟦EOF⟧
```

- import-force-conflict, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- import-chain-origin, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- import-chain-origin, lookup, completion 0, result, chunk 0 of 1:

```text-visible
SOURCE ::__tcl_lsp_probe_nh20261012broad27_import_chain_origin::source::p⟦EOF⟧
```

- import-chain-origin, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- import-chain-origin: intended operation reached.

- import-force-conflict: intended operation reached.

- import-source-deleted: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 9

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|setup|0|0:0|9|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|lookup|1|0:0|9|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520227022
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|lookup|1|0:0|9|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|lookup|1|0:0|9|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|lookup|1|0:0|9|errorinfo|binary-scan-tcl-value|0|2|696e76616c696420636f6d6d616e64206e616d65202270220a202020207768696c6520657865637574696e670a2270220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f736f757263655f64656c657465643a3a636f6e73756d65722220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|lookup|1|0:0|9|errorinfo|binary-scan-tcl-value|1|2|5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f736f757263655f64656c657465643a3a636f6e73756d6572207b707d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-source-deleted|tmm-traffic-event|runtime-cleanup|0|0:0|9|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-force-conflict|tmm-traffic-event|setup|0|0:0|9|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-force-conflict|tmm-traffic-event|lookup|0|0:0|9|result|binary-scan-tcl-value|0|1|534f55524345
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-force-conflict|tmm-traffic-event|runtime-cleanup|0|0:0|9|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-chain-origin|tmm-traffic-event|setup|0|0:0|9|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-chain-origin|tmm-traffic-event|lookup|0|0:0|9|result|binary-scan-tcl-value|0|1|534f55524345203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f696d706f72745f636861696e5f6f726967696e3a3a736f757263653a3a70
Oct 12 08:57:57 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_9 <HTTP_REQUEST>: NHO|nh20261012broad2|import-chain-origin|tmm-traffic-event|runtime-cleanup|0|0:0|9|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- import-source-deleted, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "p"⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
invalid command name "p"
    while executing
"p"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_import_source_deleted::consumer" script line 1)
    invoked from within
"namespace eval ::_⟦EOF⟧
```

- import-source-deleted, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
_tcl_lsp_probe_nh20261012broad27_import_source_deleted::consumer {p}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- import-source-deleted, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- import-force-conflict, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- import-force-conflict, lookup, completion 0, result, chunk 0 of 1:

```text-visible
SOURCE⟦EOF⟧
```

- import-force-conflict, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- import-chain-origin, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- import-chain-origin, lookup, completion 0, result, chunk 0 of 1:

```text-visible
SOURCE ::__tcl_lsp_probe_nh20261012broad27_import_chain_origin::source::p⟦EOF⟧
```

- import-chain-origin, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- import-chain-origin: intended operation reached.

- import-force-conflict: intended operation reached.

- import-source-deleted: intended operation reached.
