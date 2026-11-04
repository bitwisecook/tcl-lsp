# BIG-IP Tcl log: nh20261012broad2, tmm-traffic-event-12

## Question

The following cases investigate Tcl behavior relevant to name resolution and diagnostics in the LSP, compiler and VM. Each observation is limited to its own case and recorded context.

- **alias-deleted-lookup:** `interp alias` target namespace anchoring, late binding, qualified destination parent creation, describe/delete, child interpreters and duplicate prefixes.

- **command-trace-rename:** Rename, command traces, unknown/namespace-unknown: when are lookups repeated, which target namespace is used, which errors/options are returned?

- **rename-procedure-namespace:** Rename, command traces, unknown/namespace-unknown: when are lookups repeated, which target namespace is used, which errors/options are returned?

## Assumptions

- **alias-deleted-lookup:** Alias deletion followed by fresh lookup.
- **command-trace-rename:** Modern trace syntax is an independently checked capability, despite 8.4-compatible source syntax.
- **rename-procedure-namespace:** Procedure namespace after moving owned command.

Other assumptions were not separately recorded.

## Script

```tcl-visible
set nh_options_available unavailable
set nh_return_options {}
set nh_unit "[TMM::cmp_group]:[TMM::cmp_unit]"
set nh_invocation once
set nh_invocation [HTTP::header value X-NH-Invocation]
set nh_setup [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f616c6961735f64656c657465645f6c6f6f6b7570207b7d0a6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f616c6961735f64656c657465645f6c6f6f6b7570207b70726f6320746172676574207b7d207b72657475726e205441524745547d7d0a696e7465727020616c696173207b7d203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f616c6961735f64656c657465645f6c6f6f6b75703a3a616c696173207b7d203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f616c6961735f64656c657465645f6c6f6f6b75703a3a7461726765740a696e7465727020616c696173207b7d203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f616c6961735f64656c657465645f6c6f6f6b75703a3a616c696173207b7d0a]
set nh_operation [binary format H* 3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f616c6961735f64656c657465645f6c6f6f6b75703a3a616c6961730a]
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
        log local0. "NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 666f7265616368206e685f6368696c64205b696e7465727020736c617665735d207b6966207b5b737472696e67206d61746368205f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f616c6961735f64656c657465645f6c6f6f6b75702a20246e685f6368696c645d7d207b696e746572702064656c65746520246e685f6368696c647d7d0a666f7265616368206e685f6e73205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f616c6961735f64656c657465645f6c6f6f6b75702a5d207b6e616d6573706163652064656c65746520246e685f6e737d0a666f7265616368206e685f636d64205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f616c6961735f64656c657465645f6c6f6f6b75702a5d207b72656e616d6520246e685f636d64207b7d7d0a666f7265616368206e685f766172205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f616c6961735f64656c657465645f6c6f6f6b75702a5d207b756e736574202d6e6f636f6d706c61696e20246e685f7661727d0a6c697374205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f616c6961735f64656c657465645f6c6f6f6b75702a5d205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f616c6961735f64656c657465645f6c6f6f6b75702a5d205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f616c6961735f64656c657465645f6c6f6f6b75702a5d]
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
        log local0. "NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
set nh_setup [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f72656e616d655f70726f6365647572655f6e616d657370616365207b7d0a6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f72656e616d655f70726f6365647572655f6e616d6573706163653a3a6f6e65207b7661726961626c652078204f4e453b2070726f6320746172676574207b7d207b7661726961626c6520783b2072657475726e2024787d7d0a6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f72656e616d655f70726f6365647572655f6e616d6573706163653a3a74776f207b7661726961626c6520782054574f7d0a72656e616d65203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f72656e616d655f70726f6365647572655f6e616d6573706163653a3a6f6e653a3a746172676574203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f72656e616d655f70726f6365647572655f6e616d6573706163653a3a74776f3a3a7461726765740a]
set nh_operation [binary format H* 3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f72656e616d655f70726f6365647572655f6e616d6573706163653a3a74776f3a3a7461726765740a]
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
        log local0. "NHO|nh20261012broad2|rename-procedure-namespace|tmm-traffic-event|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012broad2|rename-procedure-namespace|tmm-traffic-event|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 666f7265616368206e685f6368696c64205b696e7465727020736c617665735d207b6966207b5b737472696e67206d61746368205f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f72656e616d655f70726f6365647572655f6e616d6573706163652a20246e685f6368696c645d7d207b696e746572702064656c65746520246e685f6368696c647d7d0a666f7265616368206e685f6e73205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f72656e616d655f70726f6365647572655f6e616d6573706163652a5d207b6e616d6573706163652064656c65746520246e685f6e737d0a666f7265616368206e685f636d64205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f72656e616d655f70726f6365647572655f6e616d6573706163652a5d207b72656e616d6520246e685f636d64207b7d7d0a666f7265616368206e685f766172205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f72656e616d655f70726f6365647572655f6e616d6573706163652a5d207b756e736574202d6e6f636f6d706c61696e20246e685f7661727d0a6c697374205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f72656e616d655f70726f6365647572655f6e616d6573706163652a5d205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f72656e616d655f70726f6365647572655f6e616d6573706163652a5d205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f72656e616d655f70726f6365647572655f6e616d6573706163652a5d]
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
        log local0. "NHO|nh20261012broad2|rename-procedure-namespace|tmm-traffic-event|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
set nh_setup [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f636f6d6d616e645f74726163655f72656e616d65207b7d0a6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f636f6d6d616e645f74726163655f72656e616d65207b0a20202020736574206576656e7473207b7d0a2020202070726f6320746172676574207b7d207b72657475726e205441524745547d0a2020202070726f63206368616e676564207b6f6c64206e6577206f707d207b7661726961626c65206576656e74733b206c617070656e64206576656e7473205b6c69737420246f6c6420246e657720246f705d7d0a7d0a74726163652061646420636f6d6d616e64203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f636f6d6d616e645f74726163655f72656e616d653a3a7461726765742072656e616d65203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f636f6d6d616e645f74726163655f72656e616d653a3a6368616e6765640a]
set nh_operation [binary format H* 72656e616d65203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f636f6d6d616e645f74726163655f72656e616d653a3a746172676574203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f636f6d6d616e645f74726163655f72656e616d653a3a72656e616d65643b20736574203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f636f6d6d616e645f74726163655f72656e616d653a3a6576656e74730a]
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
        log local0. "NHO|nh20261012broad2|command-trace-rename|tmm-traffic-event|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012broad2|command-trace-rename|tmm-traffic-event|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 666f7265616368206e685f6368696c64205b696e7465727020736c617665735d207b6966207b5b737472696e67206d61746368205f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f636f6d6d616e645f74726163655f72656e616d652a20246e685f6368696c645d7d207b696e746572702064656c65746520246e685f6368696c647d7d0a666f7265616368206e685f6e73205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f636f6d6d616e645f74726163655f72656e616d652a5d207b6e616d6573706163652064656c65746520246e685f6e737d0a666f7265616368206e685f636d64205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f636f6d6d616e645f74726163655f72656e616d652a5d207b72656e616d6520246e685f636d64207b7d7d0a666f7265616368206e685f766172205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f636f6d6d616e645f74726163655f72656e616d652a5d207b756e736574202d6e6f636f6d706c61696e20246e685f7661727d0a6c697374205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f636f6d6d616e645f74726163655f72656e616d652a5d205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f636f6d6d616e645f74726163655f72656e616d652a5d205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f636f6d6d616e645f74726163655f72656e616d652a5d]
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
        log local0. "NHO|nh20261012broad2|command-trace-rename|tmm-traffic-event|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|setup|0|0:0|0|result|binary-scan-tcl-value|0|1|
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|lookup|1|0:0|0|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f616c6961735f64656c657465645f6c6f6f6b75703a3a616c69617322
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|lookup|1|0:0|0|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|lookup|1|0:0|0|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|lookup|1|0:0|0|errorinfo|binary-scan-tcl-value|0|2|696e76616c696420636f6d6d616e64206e616d6520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f616c6961735f64656c657465645f6c6f6f6b75703a3a616c696173220a202020207768696c6520657865637574696e670a223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f616c6961735f64656c657465645f6c6f6f6b75703a3a616c696173220a2020202028226576616c2220626f6479206c696e652031290a2020
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|lookup|1|0:0|0|errorinfo|binary-scan-tcl-value|1|2|2020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|runtime-cleanup|0|0:0|0|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|rename-procedure-namespace|tmm-traffic-event|setup|0|0:0|0|result|binary-scan-tcl-value|0|1|
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|rename-procedure-namespace|tmm-traffic-event|lookup|0|0:0|0|result|binary-scan-tcl-value|0|1|54574f
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|rename-procedure-namespace|tmm-traffic-event|runtime-cleanup|0|0:0|0|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|command-trace-rename|tmm-traffic-event|setup|0|0:0|0|result|binary-scan-tcl-value|0|1|
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|command-trace-rename|tmm-traffic-event|lookup|0|0:0|0|result|binary-scan-tcl-value|0|1|7b3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f636f6d6d616e645f74726163655f72656e616d653a3a746172676574203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f636f6d6d616e645f74726163655f72656e616d653a3a72656e616d65642072656e616d657d
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|command-trace-rename|tmm-traffic-event|runtime-cleanup|0|0:0|0|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- alias-deleted-lookup, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "::__tcl_lsp_probe_nh20261012broad27_alias_deleted_lookup::alias"⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
invalid command name "::__tcl_lsp_probe_nh20261012broad27_alias_deleted_lookup::alias"
    while executing
"::__tcl_lsp_probe_nh20261012broad27_alias_deleted_lookup::alias"
    ("eval" body line 1)
⟦SPACE⟧⟦SPACE⟧⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
  invoked from within
"eval $nh_operation"⟦EOF⟧
```

- alias-deleted-lookup, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- rename-procedure-namespace, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- rename-procedure-namespace, lookup, completion 0, result, chunk 0 of 1:

```text-visible
TWO⟦EOF⟧
```

- rename-procedure-namespace, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- command-trace-rename, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- command-trace-rename, lookup, completion 0, result, chunk 0 of 1:

```text-visible
{::__tcl_lsp_probe_nh20261012broad27_command_trace_rename::target ::__tcl_lsp_probe_nh20261012broad27_command_trace_rename::renamed rename}⟦EOF⟧
```

- command-trace-rename, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- alias-deleted-lookup: intended operation reached.

- command-trace-rename: intended operation reached.

- rename-procedure-namespace: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 1

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|setup|0|0:0|1|result|binary-scan-tcl-value|0|1|
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|lookup|1|0:0|1|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f616c6961735f64656c657465645f6c6f6f6b75703a3a616c69617322
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|lookup|1|0:0|1|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|lookup|1|0:0|1|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|lookup|1|0:0|1|errorinfo|binary-scan-tcl-value|0|2|696e76616c696420636f6d6d616e64206e616d6520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f616c6961735f64656c657465645f6c6f6f6b75703a3a616c696173220a202020207768696c6520657865637574696e670a223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f616c6961735f64656c657465645f6c6f6f6b75703a3a616c696173220a2020202028226576616c2220626f6479206c696e652031290a2020
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|lookup|1|0:0|1|errorinfo|binary-scan-tcl-value|1|2|2020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|runtime-cleanup|0|0:0|1|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|rename-procedure-namespace|tmm-traffic-event|setup|0|0:0|1|result|binary-scan-tcl-value|0|1|
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|rename-procedure-namespace|tmm-traffic-event|lookup|0|0:0|1|result|binary-scan-tcl-value|0|1|54574f
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|rename-procedure-namespace|tmm-traffic-event|runtime-cleanup|0|0:0|1|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|command-trace-rename|tmm-traffic-event|setup|0|0:0|1|result|binary-scan-tcl-value|0|1|
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|command-trace-rename|tmm-traffic-event|lookup|0|0:0|1|result|binary-scan-tcl-value|0|1|7b3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f636f6d6d616e645f74726163655f72656e616d653a3a746172676574203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f636f6d6d616e645f74726163655f72656e616d653a3a72656e616d65642072656e616d657d
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|command-trace-rename|tmm-traffic-event|runtime-cleanup|0|0:0|1|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- alias-deleted-lookup, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "::__tcl_lsp_probe_nh20261012broad27_alias_deleted_lookup::alias"⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
invalid command name "::__tcl_lsp_probe_nh20261012broad27_alias_deleted_lookup::alias"
    while executing
"::__tcl_lsp_probe_nh20261012broad27_alias_deleted_lookup::alias"
    ("eval" body line 1)
⟦SPACE⟧⟦SPACE⟧⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
  invoked from within
"eval $nh_operation"⟦EOF⟧
```

- alias-deleted-lookup, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- rename-procedure-namespace, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- rename-procedure-namespace, lookup, completion 0, result, chunk 0 of 1:

```text-visible
TWO⟦EOF⟧
```

- rename-procedure-namespace, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- command-trace-rename, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- command-trace-rename, lookup, completion 0, result, chunk 0 of 1:

```text-visible
{::__tcl_lsp_probe_nh20261012broad27_command_trace_rename::target ::__tcl_lsp_probe_nh20261012broad27_command_trace_rename::renamed rename}⟦EOF⟧
```

- command-trace-rename, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- alias-deleted-lookup: intended operation reached.

- command-trace-rename: intended operation reached.

- rename-procedure-namespace: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 10

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|setup|0|0:0|10|result|binary-scan-tcl-value|0|1|
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|lookup|1|0:0|10|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f616c6961735f64656c657465645f6c6f6f6b75703a3a616c69617322
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|lookup|1|0:0|10|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|lookup|1|0:0|10|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|lookup|1|0:0|10|errorinfo|binary-scan-tcl-value|0|2|696e76616c696420636f6d6d616e64206e616d6520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f616c6961735f64656c657465645f6c6f6f6b75703a3a616c696173220a202020207768696c6520657865637574696e670a223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f616c6961735f64656c657465645f6c6f6f6b75703a3a616c696173220a2020202028226576616c2220626f6479206c696e652031290a2020
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|lookup|1|0:0|10|errorinfo|binary-scan-tcl-value|1|2|2020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|runtime-cleanup|0|0:0|10|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|rename-procedure-namespace|tmm-traffic-event|setup|0|0:0|10|result|binary-scan-tcl-value|0|1|
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|rename-procedure-namespace|tmm-traffic-event|lookup|0|0:0|10|result|binary-scan-tcl-value|0|1|54574f
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|rename-procedure-namespace|tmm-traffic-event|runtime-cleanup|0|0:0|10|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|command-trace-rename|tmm-traffic-event|setup|0|0:0|10|result|binary-scan-tcl-value|0|1|
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|command-trace-rename|tmm-traffic-event|lookup|0|0:0|10|result|binary-scan-tcl-value|0|1|7b3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f636f6d6d616e645f74726163655f72656e616d653a3a746172676574203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f636f6d6d616e645f74726163655f72656e616d653a3a72656e616d65642072656e616d657d
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|command-trace-rename|tmm-traffic-event|runtime-cleanup|0|0:0|10|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- alias-deleted-lookup, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "::__tcl_lsp_probe_nh20261012broad27_alias_deleted_lookup::alias"⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
invalid command name "::__tcl_lsp_probe_nh20261012broad27_alias_deleted_lookup::alias"
    while executing
"::__tcl_lsp_probe_nh20261012broad27_alias_deleted_lookup::alias"
    ("eval" body line 1)
⟦SPACE⟧⟦SPACE⟧⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
  invoked from within
"eval $nh_operation"⟦EOF⟧
```

- alias-deleted-lookup, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- rename-procedure-namespace, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- rename-procedure-namespace, lookup, completion 0, result, chunk 0 of 1:

```text-visible
TWO⟦EOF⟧
```

- rename-procedure-namespace, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- command-trace-rename, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- command-trace-rename, lookup, completion 0, result, chunk 0 of 1:

```text-visible
{::__tcl_lsp_probe_nh20261012broad27_command_trace_rename::target ::__tcl_lsp_probe_nh20261012broad27_command_trace_rename::renamed rename}⟦EOF⟧
```

- command-trace-rename, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- alias-deleted-lookup: intended operation reached.

- command-trace-rename: intended operation reached.

- rename-procedure-namespace: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 11

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|setup|0|0:0|11|result|binary-scan-tcl-value|0|1|
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|lookup|1|0:0|11|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f616c6961735f64656c657465645f6c6f6f6b75703a3a616c69617322
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|lookup|1|0:0|11|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|lookup|1|0:0|11|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|lookup|1|0:0|11|errorinfo|binary-scan-tcl-value|0|2|696e76616c696420636f6d6d616e64206e616d6520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f616c6961735f64656c657465645f6c6f6f6b75703a3a616c696173220a202020207768696c6520657865637574696e670a223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f616c6961735f64656c657465645f6c6f6f6b75703a3a616c696173220a2020202028226576616c2220626f6479206c696e652031290a2020
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|lookup|1|0:0|11|errorinfo|binary-scan-tcl-value|1|2|2020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|runtime-cleanup|0|0:0|11|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|rename-procedure-namespace|tmm-traffic-event|setup|0|0:0|11|result|binary-scan-tcl-value|0|1|
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|rename-procedure-namespace|tmm-traffic-event|lookup|0|0:0|11|result|binary-scan-tcl-value|0|1|54574f
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|rename-procedure-namespace|tmm-traffic-event|runtime-cleanup|0|0:0|11|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|command-trace-rename|tmm-traffic-event|setup|0|0:0|11|result|binary-scan-tcl-value|0|1|
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|command-trace-rename|tmm-traffic-event|lookup|0|0:0|11|result|binary-scan-tcl-value|0|1|7b3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f636f6d6d616e645f74726163655f72656e616d653a3a746172676574203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f636f6d6d616e645f74726163655f72656e616d653a3a72656e616d65642072656e616d657d
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|command-trace-rename|tmm-traffic-event|runtime-cleanup|0|0:0|11|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- alias-deleted-lookup, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "::__tcl_lsp_probe_nh20261012broad27_alias_deleted_lookup::alias"⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
invalid command name "::__tcl_lsp_probe_nh20261012broad27_alias_deleted_lookup::alias"
    while executing
"::__tcl_lsp_probe_nh20261012broad27_alias_deleted_lookup::alias"
    ("eval" body line 1)
⟦SPACE⟧⟦SPACE⟧⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
  invoked from within
"eval $nh_operation"⟦EOF⟧
```

- alias-deleted-lookup, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- rename-procedure-namespace, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- rename-procedure-namespace, lookup, completion 0, result, chunk 0 of 1:

```text-visible
TWO⟦EOF⟧
```

- rename-procedure-namespace, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- command-trace-rename, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- command-trace-rename, lookup, completion 0, result, chunk 0 of 1:

```text-visible
{::__tcl_lsp_probe_nh20261012broad27_command_trace_rename::target ::__tcl_lsp_probe_nh20261012broad27_command_trace_rename::renamed rename}⟦EOF⟧
```

- command-trace-rename, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- alias-deleted-lookup: intended operation reached.

- command-trace-rename: intended operation reached.

- rename-procedure-namespace: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 12

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|setup|0|0:0|12|result|binary-scan-tcl-value|0|1|
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|lookup|1|0:0|12|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f616c6961735f64656c657465645f6c6f6f6b75703a3a616c69617322
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|lookup|1|0:0|12|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|lookup|1|0:0|12|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|lookup|1|0:0|12|errorinfo|binary-scan-tcl-value|0|2|696e76616c696420636f6d6d616e64206e616d6520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f616c6961735f64656c657465645f6c6f6f6b75703a3a616c696173220a202020207768696c6520657865637574696e670a223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f616c6961735f64656c657465645f6c6f6f6b75703a3a616c696173220a2020202028226576616c2220626f6479206c696e652031290a2020
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|lookup|1|0:0|12|errorinfo|binary-scan-tcl-value|1|2|2020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|runtime-cleanup|0|0:0|12|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|rename-procedure-namespace|tmm-traffic-event|setup|0|0:0|12|result|binary-scan-tcl-value|0|1|
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|rename-procedure-namespace|tmm-traffic-event|lookup|0|0:0|12|result|binary-scan-tcl-value|0|1|54574f
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|rename-procedure-namespace|tmm-traffic-event|runtime-cleanup|0|0:0|12|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|command-trace-rename|tmm-traffic-event|setup|0|0:0|12|result|binary-scan-tcl-value|0|1|
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|command-trace-rename|tmm-traffic-event|lookup|0|0:0|12|result|binary-scan-tcl-value|0|1|7b3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f636f6d6d616e645f74726163655f72656e616d653a3a746172676574203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f636f6d6d616e645f74726163655f72656e616d653a3a72656e616d65642072656e616d657d
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|command-trace-rename|tmm-traffic-event|runtime-cleanup|0|0:0|12|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- alias-deleted-lookup, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "::__tcl_lsp_probe_nh20261012broad27_alias_deleted_lookup::alias"⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
invalid command name "::__tcl_lsp_probe_nh20261012broad27_alias_deleted_lookup::alias"
    while executing
"::__tcl_lsp_probe_nh20261012broad27_alias_deleted_lookup::alias"
    ("eval" body line 1)
⟦SPACE⟧⟦SPACE⟧⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
  invoked from within
"eval $nh_operation"⟦EOF⟧
```

- alias-deleted-lookup, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- rename-procedure-namespace, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- rename-procedure-namespace, lookup, completion 0, result, chunk 0 of 1:

```text-visible
TWO⟦EOF⟧
```

- rename-procedure-namespace, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- command-trace-rename, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- command-trace-rename, lookup, completion 0, result, chunk 0 of 1:

```text-visible
{::__tcl_lsp_probe_nh20261012broad27_command_trace_rename::target ::__tcl_lsp_probe_nh20261012broad27_command_trace_rename::renamed rename}⟦EOF⟧
```

- command-trace-rename, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- alias-deleted-lookup: intended operation reached.

- command-trace-rename: intended operation reached.

- rename-procedure-namespace: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 13

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|setup|0|0:0|13|result|binary-scan-tcl-value|0|1|
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|lookup|1|0:0|13|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f616c6961735f64656c657465645f6c6f6f6b75703a3a616c69617322
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|lookup|1|0:0|13|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|lookup|1|0:0|13|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|lookup|1|0:0|13|errorinfo|binary-scan-tcl-value|0|2|696e76616c696420636f6d6d616e64206e616d6520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f616c6961735f64656c657465645f6c6f6f6b75703a3a616c696173220a202020207768696c6520657865637574696e670a223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f616c6961735f64656c657465645f6c6f6f6b75703a3a616c696173220a2020202028226576616c2220626f6479206c696e652031290a2020
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|lookup|1|0:0|13|errorinfo|binary-scan-tcl-value|1|2|2020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|runtime-cleanup|0|0:0|13|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|rename-procedure-namespace|tmm-traffic-event|setup|0|0:0|13|result|binary-scan-tcl-value|0|1|
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|rename-procedure-namespace|tmm-traffic-event|lookup|0|0:0|13|result|binary-scan-tcl-value|0|1|54574f
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|rename-procedure-namespace|tmm-traffic-event|runtime-cleanup|0|0:0|13|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|command-trace-rename|tmm-traffic-event|setup|0|0:0|13|result|binary-scan-tcl-value|0|1|
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|command-trace-rename|tmm-traffic-event|lookup|0|0:0|13|result|binary-scan-tcl-value|0|1|7b3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f636f6d6d616e645f74726163655f72656e616d653a3a746172676574203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f636f6d6d616e645f74726163655f72656e616d653a3a72656e616d65642072656e616d657d
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|command-trace-rename|tmm-traffic-event|runtime-cleanup|0|0:0|13|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- alias-deleted-lookup, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "::__tcl_lsp_probe_nh20261012broad27_alias_deleted_lookup::alias"⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
invalid command name "::__tcl_lsp_probe_nh20261012broad27_alias_deleted_lookup::alias"
    while executing
"::__tcl_lsp_probe_nh20261012broad27_alias_deleted_lookup::alias"
    ("eval" body line 1)
⟦SPACE⟧⟦SPACE⟧⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
  invoked from within
"eval $nh_operation"⟦EOF⟧
```

- alias-deleted-lookup, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- rename-procedure-namespace, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- rename-procedure-namespace, lookup, completion 0, result, chunk 0 of 1:

```text-visible
TWO⟦EOF⟧
```

- rename-procedure-namespace, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- command-trace-rename, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- command-trace-rename, lookup, completion 0, result, chunk 0 of 1:

```text-visible
{::__tcl_lsp_probe_nh20261012broad27_command_trace_rename::target ::__tcl_lsp_probe_nh20261012broad27_command_trace_rename::renamed rename}⟦EOF⟧
```

- command-trace-rename, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- alias-deleted-lookup: intended operation reached.

- command-trace-rename: intended operation reached.

- rename-procedure-namespace: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 14

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|setup|0|0:0|14|result|binary-scan-tcl-value|0|1|
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|lookup|1|0:0|14|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f616c6961735f64656c657465645f6c6f6f6b75703a3a616c69617322
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|lookup|1|0:0|14|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|lookup|1|0:0|14|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|lookup|1|0:0|14|errorinfo|binary-scan-tcl-value|0|2|696e76616c696420636f6d6d616e64206e616d6520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f616c6961735f64656c657465645f6c6f6f6b75703a3a616c696173220a202020207768696c6520657865637574696e670a223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f616c6961735f64656c657465645f6c6f6f6b75703a3a616c696173220a2020202028226576616c2220626f6479206c696e652031290a2020
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|lookup|1|0:0|14|errorinfo|binary-scan-tcl-value|1|2|2020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|runtime-cleanup|0|0:0|14|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|rename-procedure-namespace|tmm-traffic-event|setup|0|0:0|14|result|binary-scan-tcl-value|0|1|
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|rename-procedure-namespace|tmm-traffic-event|lookup|0|0:0|14|result|binary-scan-tcl-value|0|1|54574f
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|rename-procedure-namespace|tmm-traffic-event|runtime-cleanup|0|0:0|14|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|command-trace-rename|tmm-traffic-event|setup|0|0:0|14|result|binary-scan-tcl-value|0|1|
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|command-trace-rename|tmm-traffic-event|lookup|0|0:0|14|result|binary-scan-tcl-value|0|1|7b3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f636f6d6d616e645f74726163655f72656e616d653a3a746172676574203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f636f6d6d616e645f74726163655f72656e616d653a3a72656e616d65642072656e616d657d
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|command-trace-rename|tmm-traffic-event|runtime-cleanup|0|0:0|14|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- alias-deleted-lookup, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "::__tcl_lsp_probe_nh20261012broad27_alias_deleted_lookup::alias"⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
invalid command name "::__tcl_lsp_probe_nh20261012broad27_alias_deleted_lookup::alias"
    while executing
"::__tcl_lsp_probe_nh20261012broad27_alias_deleted_lookup::alias"
    ("eval" body line 1)
⟦SPACE⟧⟦SPACE⟧⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
  invoked from within
"eval $nh_operation"⟦EOF⟧
```

- alias-deleted-lookup, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- rename-procedure-namespace, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- rename-procedure-namespace, lookup, completion 0, result, chunk 0 of 1:

```text-visible
TWO⟦EOF⟧
```

- rename-procedure-namespace, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- command-trace-rename, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- command-trace-rename, lookup, completion 0, result, chunk 0 of 1:

```text-visible
{::__tcl_lsp_probe_nh20261012broad27_command_trace_rename::target ::__tcl_lsp_probe_nh20261012broad27_command_trace_rename::renamed rename}⟦EOF⟧
```

- command-trace-rename, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- alias-deleted-lookup: intended operation reached.

- command-trace-rename: intended operation reached.

- rename-procedure-namespace: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 15

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:58:33 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|setup|0|0:0|15|result|binary-scan-tcl-value|0|1|
Oct 12 08:58:33 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|lookup|1|0:0|15|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f616c6961735f64656c657465645f6c6f6f6b75703a3a616c69617322
Oct 12 08:58:33 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|lookup|1|0:0|15|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:58:33 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|lookup|1|0:0|15|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:58:33 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|lookup|1|0:0|15|errorinfo|binary-scan-tcl-value|0|2|696e76616c696420636f6d6d616e64206e616d6520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f616c6961735f64656c657465645f6c6f6f6b75703a3a616c696173220a202020207768696c6520657865637574696e670a223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f616c6961735f64656c657465645f6c6f6f6b75703a3a616c696173220a2020202028226576616c2220626f6479206c696e652031290a2020
Oct 12 08:58:33 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|lookup|1|0:0|15|errorinfo|binary-scan-tcl-value|1|2|2020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:58:33 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|runtime-cleanup|0|0:0|15|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:58:33 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|rename-procedure-namespace|tmm-traffic-event|setup|0|0:0|15|result|binary-scan-tcl-value|0|1|
Oct 12 08:58:33 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|rename-procedure-namespace|tmm-traffic-event|lookup|0|0:0|15|result|binary-scan-tcl-value|0|1|54574f
Oct 12 08:58:33 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|rename-procedure-namespace|tmm-traffic-event|runtime-cleanup|0|0:0|15|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:58:33 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|command-trace-rename|tmm-traffic-event|setup|0|0:0|15|result|binary-scan-tcl-value|0|1|
Oct 12 08:58:33 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|command-trace-rename|tmm-traffic-event|lookup|0|0:0|15|result|binary-scan-tcl-value|0|1|7b3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f636f6d6d616e645f74726163655f72656e616d653a3a746172676574203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f636f6d6d616e645f74726163655f72656e616d653a3a72656e616d65642072656e616d657d
Oct 12 08:58:33 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|command-trace-rename|tmm-traffic-event|runtime-cleanup|0|0:0|15|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- alias-deleted-lookup, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "::__tcl_lsp_probe_nh20261012broad27_alias_deleted_lookup::alias"⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
invalid command name "::__tcl_lsp_probe_nh20261012broad27_alias_deleted_lookup::alias"
    while executing
"::__tcl_lsp_probe_nh20261012broad27_alias_deleted_lookup::alias"
    ("eval" body line 1)
⟦SPACE⟧⟦SPACE⟧⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
  invoked from within
"eval $nh_operation"⟦EOF⟧
```

- alias-deleted-lookup, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- rename-procedure-namespace, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- rename-procedure-namespace, lookup, completion 0, result, chunk 0 of 1:

```text-visible
TWO⟦EOF⟧
```

- rename-procedure-namespace, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- command-trace-rename, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- command-trace-rename, lookup, completion 0, result, chunk 0 of 1:

```text-visible
{::__tcl_lsp_probe_nh20261012broad27_command_trace_rename::target ::__tcl_lsp_probe_nh20261012broad27_command_trace_rename::renamed rename}⟦EOF⟧
```

- command-trace-rename, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- alias-deleted-lookup: intended operation reached.

- command-trace-rename: intended operation reached.

- rename-procedure-namespace: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 2

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|setup|0|0:0|2|result|binary-scan-tcl-value|0|1|
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|lookup|1|0:0|2|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f616c6961735f64656c657465645f6c6f6f6b75703a3a616c69617322
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|lookup|1|0:0|2|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|lookup|1|0:0|2|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|lookup|1|0:0|2|errorinfo|binary-scan-tcl-value|0|2|696e76616c696420636f6d6d616e64206e616d6520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f616c6961735f64656c657465645f6c6f6f6b75703a3a616c696173220a202020207768696c6520657865637574696e670a223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f616c6961735f64656c657465645f6c6f6f6b75703a3a616c696173220a2020202028226576616c2220626f6479206c696e652031290a2020
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|lookup|1|0:0|2|errorinfo|binary-scan-tcl-value|1|2|2020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|runtime-cleanup|0|0:0|2|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|rename-procedure-namespace|tmm-traffic-event|setup|0|0:0|2|result|binary-scan-tcl-value|0|1|
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|rename-procedure-namespace|tmm-traffic-event|lookup|0|0:0|2|result|binary-scan-tcl-value|0|1|54574f
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|rename-procedure-namespace|tmm-traffic-event|runtime-cleanup|0|0:0|2|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|command-trace-rename|tmm-traffic-event|setup|0|0:0|2|result|binary-scan-tcl-value|0|1|
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|command-trace-rename|tmm-traffic-event|lookup|0|0:0|2|result|binary-scan-tcl-value|0|1|7b3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f636f6d6d616e645f74726163655f72656e616d653a3a746172676574203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f636f6d6d616e645f74726163655f72656e616d653a3a72656e616d65642072656e616d657d
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|command-trace-rename|tmm-traffic-event|runtime-cleanup|0|0:0|2|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- alias-deleted-lookup, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "::__tcl_lsp_probe_nh20261012broad27_alias_deleted_lookup::alias"⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
invalid command name "::__tcl_lsp_probe_nh20261012broad27_alias_deleted_lookup::alias"
    while executing
"::__tcl_lsp_probe_nh20261012broad27_alias_deleted_lookup::alias"
    ("eval" body line 1)
⟦SPACE⟧⟦SPACE⟧⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
  invoked from within
"eval $nh_operation"⟦EOF⟧
```

- alias-deleted-lookup, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- rename-procedure-namespace, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- rename-procedure-namespace, lookup, completion 0, result, chunk 0 of 1:

```text-visible
TWO⟦EOF⟧
```

- rename-procedure-namespace, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- command-trace-rename, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- command-trace-rename, lookup, completion 0, result, chunk 0 of 1:

```text-visible
{::__tcl_lsp_probe_nh20261012broad27_command_trace_rename::target ::__tcl_lsp_probe_nh20261012broad27_command_trace_rename::renamed rename}⟦EOF⟧
```

- command-trace-rename, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- alias-deleted-lookup: intended operation reached.

- command-trace-rename: intended operation reached.

- rename-procedure-namespace: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 3

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|setup|0|0:0|3|result|binary-scan-tcl-value|0|1|
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|lookup|1|0:0|3|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f616c6961735f64656c657465645f6c6f6f6b75703a3a616c69617322
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|lookup|1|0:0|3|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|lookup|1|0:0|3|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|lookup|1|0:0|3|errorinfo|binary-scan-tcl-value|0|2|696e76616c696420636f6d6d616e64206e616d6520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f616c6961735f64656c657465645f6c6f6f6b75703a3a616c696173220a202020207768696c6520657865637574696e670a223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f616c6961735f64656c657465645f6c6f6f6b75703a3a616c696173220a2020202028226576616c2220626f6479206c696e652031290a2020
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|lookup|1|0:0|3|errorinfo|binary-scan-tcl-value|1|2|2020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|runtime-cleanup|0|0:0|3|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|rename-procedure-namespace|tmm-traffic-event|setup|0|0:0|3|result|binary-scan-tcl-value|0|1|
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|rename-procedure-namespace|tmm-traffic-event|lookup|0|0:0|3|result|binary-scan-tcl-value|0|1|54574f
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|rename-procedure-namespace|tmm-traffic-event|runtime-cleanup|0|0:0|3|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|command-trace-rename|tmm-traffic-event|setup|0|0:0|3|result|binary-scan-tcl-value|0|1|
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|command-trace-rename|tmm-traffic-event|lookup|0|0:0|3|result|binary-scan-tcl-value|0|1|7b3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f636f6d6d616e645f74726163655f72656e616d653a3a746172676574203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f636f6d6d616e645f74726163655f72656e616d653a3a72656e616d65642072656e616d657d
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|command-trace-rename|tmm-traffic-event|runtime-cleanup|0|0:0|3|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- alias-deleted-lookup, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "::__tcl_lsp_probe_nh20261012broad27_alias_deleted_lookup::alias"⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
invalid command name "::__tcl_lsp_probe_nh20261012broad27_alias_deleted_lookup::alias"
    while executing
"::__tcl_lsp_probe_nh20261012broad27_alias_deleted_lookup::alias"
    ("eval" body line 1)
⟦SPACE⟧⟦SPACE⟧⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
  invoked from within
"eval $nh_operation"⟦EOF⟧
```

- alias-deleted-lookup, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- rename-procedure-namespace, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- rename-procedure-namespace, lookup, completion 0, result, chunk 0 of 1:

```text-visible
TWO⟦EOF⟧
```

- rename-procedure-namespace, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- command-trace-rename, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- command-trace-rename, lookup, completion 0, result, chunk 0 of 1:

```text-visible
{::__tcl_lsp_probe_nh20261012broad27_command_trace_rename::target ::__tcl_lsp_probe_nh20261012broad27_command_trace_rename::renamed rename}⟦EOF⟧
```

- command-trace-rename, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- alias-deleted-lookup: intended operation reached.

- command-trace-rename: intended operation reached.

- rename-procedure-namespace: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 4

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|setup|0|0:0|4|result|binary-scan-tcl-value|0|1|
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|lookup|1|0:0|4|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f616c6961735f64656c657465645f6c6f6f6b75703a3a616c69617322
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|lookup|1|0:0|4|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|lookup|1|0:0|4|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|lookup|1|0:0|4|errorinfo|binary-scan-tcl-value|0|2|696e76616c696420636f6d6d616e64206e616d6520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f616c6961735f64656c657465645f6c6f6f6b75703a3a616c696173220a202020207768696c6520657865637574696e670a223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f616c6961735f64656c657465645f6c6f6f6b75703a3a616c696173220a2020202028226576616c2220626f6479206c696e652031290a2020
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|lookup|1|0:0|4|errorinfo|binary-scan-tcl-value|1|2|2020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|runtime-cleanup|0|0:0|4|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|rename-procedure-namespace|tmm-traffic-event|setup|0|0:0|4|result|binary-scan-tcl-value|0|1|
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|rename-procedure-namespace|tmm-traffic-event|lookup|0|0:0|4|result|binary-scan-tcl-value|0|1|54574f
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|rename-procedure-namespace|tmm-traffic-event|runtime-cleanup|0|0:0|4|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|command-trace-rename|tmm-traffic-event|setup|0|0:0|4|result|binary-scan-tcl-value|0|1|
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|command-trace-rename|tmm-traffic-event|lookup|0|0:0|4|result|binary-scan-tcl-value|0|1|7b3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f636f6d6d616e645f74726163655f72656e616d653a3a746172676574203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f636f6d6d616e645f74726163655f72656e616d653a3a72656e616d65642072656e616d657d
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|command-trace-rename|tmm-traffic-event|runtime-cleanup|0|0:0|4|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- alias-deleted-lookup, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "::__tcl_lsp_probe_nh20261012broad27_alias_deleted_lookup::alias"⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
invalid command name "::__tcl_lsp_probe_nh20261012broad27_alias_deleted_lookup::alias"
    while executing
"::__tcl_lsp_probe_nh20261012broad27_alias_deleted_lookup::alias"
    ("eval" body line 1)
⟦SPACE⟧⟦SPACE⟧⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
  invoked from within
"eval $nh_operation"⟦EOF⟧
```

- alias-deleted-lookup, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- rename-procedure-namespace, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- rename-procedure-namespace, lookup, completion 0, result, chunk 0 of 1:

```text-visible
TWO⟦EOF⟧
```

- rename-procedure-namespace, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- command-trace-rename, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- command-trace-rename, lookup, completion 0, result, chunk 0 of 1:

```text-visible
{::__tcl_lsp_probe_nh20261012broad27_command_trace_rename::target ::__tcl_lsp_probe_nh20261012broad27_command_trace_rename::renamed rename}⟦EOF⟧
```

- command-trace-rename, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- alias-deleted-lookup: intended operation reached.

- command-trace-rename: intended operation reached.

- rename-procedure-namespace: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 5

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|setup|0|0:0|5|result|binary-scan-tcl-value|0|1|
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|lookup|1|0:0|5|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f616c6961735f64656c657465645f6c6f6f6b75703a3a616c69617322
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|lookup|1|0:0|5|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|lookup|1|0:0|5|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|lookup|1|0:0|5|errorinfo|binary-scan-tcl-value|0|2|696e76616c696420636f6d6d616e64206e616d6520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f616c6961735f64656c657465645f6c6f6f6b75703a3a616c696173220a202020207768696c6520657865637574696e670a223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f616c6961735f64656c657465645f6c6f6f6b75703a3a616c696173220a2020202028226576616c2220626f6479206c696e652031290a2020
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|lookup|1|0:0|5|errorinfo|binary-scan-tcl-value|1|2|2020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|runtime-cleanup|0|0:0|5|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|rename-procedure-namespace|tmm-traffic-event|setup|0|0:0|5|result|binary-scan-tcl-value|0|1|
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|rename-procedure-namespace|tmm-traffic-event|lookup|0|0:0|5|result|binary-scan-tcl-value|0|1|54574f
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|rename-procedure-namespace|tmm-traffic-event|runtime-cleanup|0|0:0|5|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|command-trace-rename|tmm-traffic-event|setup|0|0:0|5|result|binary-scan-tcl-value|0|1|
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|command-trace-rename|tmm-traffic-event|lookup|0|0:0|5|result|binary-scan-tcl-value|0|1|7b3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f636f6d6d616e645f74726163655f72656e616d653a3a746172676574203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f636f6d6d616e645f74726163655f72656e616d653a3a72656e616d65642072656e616d657d
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|command-trace-rename|tmm-traffic-event|runtime-cleanup|0|0:0|5|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- alias-deleted-lookup, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "::__tcl_lsp_probe_nh20261012broad27_alias_deleted_lookup::alias"⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
invalid command name "::__tcl_lsp_probe_nh20261012broad27_alias_deleted_lookup::alias"
    while executing
"::__tcl_lsp_probe_nh20261012broad27_alias_deleted_lookup::alias"
    ("eval" body line 1)
⟦SPACE⟧⟦SPACE⟧⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
  invoked from within
"eval $nh_operation"⟦EOF⟧
```

- alias-deleted-lookup, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- rename-procedure-namespace, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- rename-procedure-namespace, lookup, completion 0, result, chunk 0 of 1:

```text-visible
TWO⟦EOF⟧
```

- rename-procedure-namespace, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- command-trace-rename, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- command-trace-rename, lookup, completion 0, result, chunk 0 of 1:

```text-visible
{::__tcl_lsp_probe_nh20261012broad27_command_trace_rename::target ::__tcl_lsp_probe_nh20261012broad27_command_trace_rename::renamed rename}⟦EOF⟧
```

- command-trace-rename, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- alias-deleted-lookup: intended operation reached.

- command-trace-rename: intended operation reached.

- rename-procedure-namespace: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 6

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|setup|0|0:0|6|result|binary-scan-tcl-value|0|1|
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|lookup|1|0:0|6|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f616c6961735f64656c657465645f6c6f6f6b75703a3a616c69617322
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|lookup|1|0:0|6|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|lookup|1|0:0|6|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|lookup|1|0:0|6|errorinfo|binary-scan-tcl-value|0|2|696e76616c696420636f6d6d616e64206e616d6520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f616c6961735f64656c657465645f6c6f6f6b75703a3a616c696173220a202020207768696c6520657865637574696e670a223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f616c6961735f64656c657465645f6c6f6f6b75703a3a616c696173220a2020202028226576616c2220626f6479206c696e652031290a2020
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|lookup|1|0:0|6|errorinfo|binary-scan-tcl-value|1|2|2020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|runtime-cleanup|0|0:0|6|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|rename-procedure-namespace|tmm-traffic-event|setup|0|0:0|6|result|binary-scan-tcl-value|0|1|
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|rename-procedure-namespace|tmm-traffic-event|lookup|0|0:0|6|result|binary-scan-tcl-value|0|1|54574f
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|rename-procedure-namespace|tmm-traffic-event|runtime-cleanup|0|0:0|6|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|command-trace-rename|tmm-traffic-event|setup|0|0:0|6|result|binary-scan-tcl-value|0|1|
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|command-trace-rename|tmm-traffic-event|lookup|0|0:0|6|result|binary-scan-tcl-value|0|1|7b3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f636f6d6d616e645f74726163655f72656e616d653a3a746172676574203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f636f6d6d616e645f74726163655f72656e616d653a3a72656e616d65642072656e616d657d
Oct 12 08:58:31 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|command-trace-rename|tmm-traffic-event|runtime-cleanup|0|0:0|6|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- alias-deleted-lookup, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "::__tcl_lsp_probe_nh20261012broad27_alias_deleted_lookup::alias"⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
invalid command name "::__tcl_lsp_probe_nh20261012broad27_alias_deleted_lookup::alias"
    while executing
"::__tcl_lsp_probe_nh20261012broad27_alias_deleted_lookup::alias"
    ("eval" body line 1)
⟦SPACE⟧⟦SPACE⟧⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
  invoked from within
"eval $nh_operation"⟦EOF⟧
```

- alias-deleted-lookup, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- rename-procedure-namespace, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- rename-procedure-namespace, lookup, completion 0, result, chunk 0 of 1:

```text-visible
TWO⟦EOF⟧
```

- rename-procedure-namespace, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- command-trace-rename, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- command-trace-rename, lookup, completion 0, result, chunk 0 of 1:

```text-visible
{::__tcl_lsp_probe_nh20261012broad27_command_trace_rename::target ::__tcl_lsp_probe_nh20261012broad27_command_trace_rename::renamed rename}⟦EOF⟧
```

- command-trace-rename, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- alias-deleted-lookup: intended operation reached.

- command-trace-rename: intended operation reached.

- rename-procedure-namespace: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 7

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|setup|0|0:0|7|result|binary-scan-tcl-value|0|1|
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|lookup|1|0:0|7|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f616c6961735f64656c657465645f6c6f6f6b75703a3a616c69617322
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|lookup|1|0:0|7|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|lookup|1|0:0|7|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|lookup|1|0:0|7|errorinfo|binary-scan-tcl-value|0|2|696e76616c696420636f6d6d616e64206e616d6520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f616c6961735f64656c657465645f6c6f6f6b75703a3a616c696173220a202020207768696c6520657865637574696e670a223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f616c6961735f64656c657465645f6c6f6f6b75703a3a616c696173220a2020202028226576616c2220626f6479206c696e652031290a2020
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|lookup|1|0:0|7|errorinfo|binary-scan-tcl-value|1|2|2020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|runtime-cleanup|0|0:0|7|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|rename-procedure-namespace|tmm-traffic-event|setup|0|0:0|7|result|binary-scan-tcl-value|0|1|
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|rename-procedure-namespace|tmm-traffic-event|lookup|0|0:0|7|result|binary-scan-tcl-value|0|1|54574f
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|rename-procedure-namespace|tmm-traffic-event|runtime-cleanup|0|0:0|7|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|command-trace-rename|tmm-traffic-event|setup|0|0:0|7|result|binary-scan-tcl-value|0|1|
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|command-trace-rename|tmm-traffic-event|lookup|0|0:0|7|result|binary-scan-tcl-value|0|1|7b3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f636f6d6d616e645f74726163655f72656e616d653a3a746172676574203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f636f6d6d616e645f74726163655f72656e616d653a3a72656e616d65642072656e616d657d
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|command-trace-rename|tmm-traffic-event|runtime-cleanup|0|0:0|7|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- alias-deleted-lookup, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "::__tcl_lsp_probe_nh20261012broad27_alias_deleted_lookup::alias"⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
invalid command name "::__tcl_lsp_probe_nh20261012broad27_alias_deleted_lookup::alias"
    while executing
"::__tcl_lsp_probe_nh20261012broad27_alias_deleted_lookup::alias"
    ("eval" body line 1)
⟦SPACE⟧⟦SPACE⟧⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
  invoked from within
"eval $nh_operation"⟦EOF⟧
```

- alias-deleted-lookup, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- rename-procedure-namespace, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- rename-procedure-namespace, lookup, completion 0, result, chunk 0 of 1:

```text-visible
TWO⟦EOF⟧
```

- rename-procedure-namespace, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- command-trace-rename, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- command-trace-rename, lookup, completion 0, result, chunk 0 of 1:

```text-visible
{::__tcl_lsp_probe_nh20261012broad27_command_trace_rename::target ::__tcl_lsp_probe_nh20261012broad27_command_trace_rename::renamed rename}⟦EOF⟧
```

- command-trace-rename, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- alias-deleted-lookup: intended operation reached.

- command-trace-rename: intended operation reached.

- rename-procedure-namespace: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 8

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|setup|0|0:0|8|result|binary-scan-tcl-value|0|1|
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|lookup|1|0:0|8|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f616c6961735f64656c657465645f6c6f6f6b75703a3a616c69617322
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|lookup|1|0:0|8|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|lookup|1|0:0|8|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|lookup|1|0:0|8|errorinfo|binary-scan-tcl-value|0|2|696e76616c696420636f6d6d616e64206e616d6520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f616c6961735f64656c657465645f6c6f6f6b75703a3a616c696173220a202020207768696c6520657865637574696e670a223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f616c6961735f64656c657465645f6c6f6f6b75703a3a616c696173220a2020202028226576616c2220626f6479206c696e652031290a2020
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|lookup|1|0:0|8|errorinfo|binary-scan-tcl-value|1|2|2020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|runtime-cleanup|0|0:0|8|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|rename-procedure-namespace|tmm-traffic-event|setup|0|0:0|8|result|binary-scan-tcl-value|0|1|
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|rename-procedure-namespace|tmm-traffic-event|lookup|0|0:0|8|result|binary-scan-tcl-value|0|1|54574f
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|rename-procedure-namespace|tmm-traffic-event|runtime-cleanup|0|0:0|8|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|command-trace-rename|tmm-traffic-event|setup|0|0:0|8|result|binary-scan-tcl-value|0|1|
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|command-trace-rename|tmm-traffic-event|lookup|0|0:0|8|result|binary-scan-tcl-value|0|1|7b3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f636f6d6d616e645f74726163655f72656e616d653a3a746172676574203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f636f6d6d616e645f74726163655f72656e616d653a3a72656e616d65642072656e616d657d
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|command-trace-rename|tmm-traffic-event|runtime-cleanup|0|0:0|8|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- alias-deleted-lookup, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "::__tcl_lsp_probe_nh20261012broad27_alias_deleted_lookup::alias"⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
invalid command name "::__tcl_lsp_probe_nh20261012broad27_alias_deleted_lookup::alias"
    while executing
"::__tcl_lsp_probe_nh20261012broad27_alias_deleted_lookup::alias"
    ("eval" body line 1)
⟦SPACE⟧⟦SPACE⟧⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
  invoked from within
"eval $nh_operation"⟦EOF⟧
```

- alias-deleted-lookup, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- rename-procedure-namespace, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- rename-procedure-namespace, lookup, completion 0, result, chunk 0 of 1:

```text-visible
TWO⟦EOF⟧
```

- rename-procedure-namespace, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- command-trace-rename, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- command-trace-rename, lookup, completion 0, result, chunk 0 of 1:

```text-visible
{::__tcl_lsp_probe_nh20261012broad27_command_trace_rename::target ::__tcl_lsp_probe_nh20261012broad27_command_trace_rename::renamed rename}⟦EOF⟧
```

- command-trace-rename, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- alias-deleted-lookup: intended operation reached.

- command-trace-rename: intended operation reached.

- rename-procedure-namespace: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 9

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|setup|0|0:0|9|result|binary-scan-tcl-value|0|1|
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|lookup|1|0:0|9|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f616c6961735f64656c657465645f6c6f6f6b75703a3a616c69617322
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|lookup|1|0:0|9|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|lookup|1|0:0|9|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|lookup|1|0:0|9|errorinfo|binary-scan-tcl-value|0|2|696e76616c696420636f6d6d616e64206e616d6520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f616c6961735f64656c657465645f6c6f6f6b75703a3a616c696173220a202020207768696c6520657865637574696e670a223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f616c6961735f64656c657465645f6c6f6f6b75703a3a616c696173220a2020202028226576616c2220626f6479206c696e652031290a2020
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|lookup|1|0:0|9|errorinfo|binary-scan-tcl-value|1|2|2020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|alias-deleted-lookup|tmm-traffic-event|runtime-cleanup|0|0:0|9|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|rename-procedure-namespace|tmm-traffic-event|setup|0|0:0|9|result|binary-scan-tcl-value|0|1|
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|rename-procedure-namespace|tmm-traffic-event|lookup|0|0:0|9|result|binary-scan-tcl-value|0|1|54574f
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|rename-procedure-namespace|tmm-traffic-event|runtime-cleanup|0|0:0|9|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|command-trace-rename|tmm-traffic-event|setup|0|0:0|9|result|binary-scan-tcl-value|0|1|
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|command-trace-rename|tmm-traffic-event|lookup|0|0:0|9|result|binary-scan-tcl-value|0|1|7b3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f636f6d6d616e645f74726163655f72656e616d653a3a746172676574203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f636f6d6d616e645f74726163655f72656e616d653a3a72656e616d65642072656e616d657d
Oct 12 08:58:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_12 <HTTP_REQUEST>: NHO|nh20261012broad2|command-trace-rename|tmm-traffic-event|runtime-cleanup|0|0:0|9|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- alias-deleted-lookup, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "::__tcl_lsp_probe_nh20261012broad27_alias_deleted_lookup::alias"⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
invalid command name "::__tcl_lsp_probe_nh20261012broad27_alias_deleted_lookup::alias"
    while executing
"::__tcl_lsp_probe_nh20261012broad27_alias_deleted_lookup::alias"
    ("eval" body line 1)
⟦SPACE⟧⟦SPACE⟧⟦EOF⟧
```

- alias-deleted-lookup, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
  invoked from within
"eval $nh_operation"⟦EOF⟧
```

- alias-deleted-lookup, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- rename-procedure-namespace, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- rename-procedure-namespace, lookup, completion 0, result, chunk 0 of 1:

```text-visible
TWO⟦EOF⟧
```

- rename-procedure-namespace, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- command-trace-rename, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- command-trace-rename, lookup, completion 0, result, chunk 0 of 1:

```text-visible
{::__tcl_lsp_probe_nh20261012broad27_command_trace_rename::target ::__tcl_lsp_probe_nh20261012broad27_command_trace_rename::renamed rename}⟦EOF⟧
```

- command-trace-rename, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- alias-deleted-lookup: intended operation reached.

- command-trace-rename: intended operation reached.

- rename-procedure-namespace: intended operation reached.
