# BIG-IP Tcl log: nh20261012broad1, tmm-RULE_INIT-17

## Question

The following cases investigate Tcl behavior relevant to name resolution and diagnostics in the LSP, compiler and VM. Each observation is limited to its own case and recorded context.

- **error-link-array-element:** Error conformance: missing command/variable/namespace, forbidden link and rename/import conflict: result, completion, `-errorcode`, `-errorinfo`, options.

- **error-missing-namespace:** Error conformance: missing command/variable/namespace, forbidden link and rename/import conflict: result, completion, `-errorcode`, `-errorinfo`, options.

- **error-missing-variable:** Error conformance: missing command/variable/namespace, forbidden link and rename/import conflict: result, completion, `-errorcode`, `-errorinfo`, options.

## Assumptions

- **error-link-array-element:** Link targets an array element then attempts array access through scalar link.
- **error-missing-namespace:** Direct missing-namespace error after capability control.
- **error-missing-variable:** Direct missing-variable error, no inner catch.

Other assumptions were not separately recorded.

## Script

```tcl-visible
set nh_unit "[TMM::cmp_group]:[TMM::cmp_unit]"
set nh_invocation once
set nh_setup [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6d697373696e675f7661726961626c65207b7d0a6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6d697373696e675f7661726961626c65207b7d0a]
set nh_operation [binary format H* 736574203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6d697373696e675f7661726961626c653a3a616273656e740a]
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
        log local0. "NHO|nh20261012broad1|error-missing-variable|tmm-RULE_INIT|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012broad1|error-missing-variable|tmm-RULE_INIT|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 666f7265616368206e685f6368696c64205b696e7465727020736c617665735d207b6966207b5b737472696e67206d61746368205f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6d697373696e675f7661726961626c652a20246e685f6368696c645d7d207b696e746572702064656c65746520246e685f6368696c647d7d0a666f7265616368206e685f6e73205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6d697373696e675f7661726961626c652a5d207b6e616d6573706163652064656c65746520246e685f6e737d0a666f7265616368206e685f636d64205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6d697373696e675f7661726961626c652a5d207b72656e616d6520246e685f636d64207b7d7d0a666f7265616368206e685f766172205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6d697373696e675f7661726961626c652a5d207b756e736574202d6e6f636f6d706c61696e20246e685f7661727d0a6c697374205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6d697373696e675f7661726961626c652a5d205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6d697373696e675f7661726961626c652a5d205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6d697373696e675f7661726961626c652a5d]
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
        log local0. "NHO|nh20261012broad1|error-missing-variable|tmm-RULE_INIT|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
set nh_setup [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6d697373696e675f6e616d657370616365207b7d0a6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6d697373696e675f6e616d657370616365207b7d0a]
set nh_operation [binary format H* 6e616d65737061636520696e73636f7065203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6d697373696e675f6e616d6573706163653a3a616273656e74207b73657420787d0a]
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
        log local0. "NHO|nh20261012broad1|error-missing-namespace|tmm-RULE_INIT|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012broad1|error-missing-namespace|tmm-RULE_INIT|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 666f7265616368206e685f6368696c64205b696e7465727020736c617665735d207b6966207b5b737472696e67206d61746368205f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6d697373696e675f6e616d6573706163652a20246e685f6368696c645d7d207b696e746572702064656c65746520246e685f6368696c647d7d0a666f7265616368206e685f6e73205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6d697373696e675f6e616d6573706163652a5d207b6e616d6573706163652064656c65746520246e685f6e737d0a666f7265616368206e685f636d64205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6d697373696e675f6e616d6573706163652a5d207b72656e616d6520246e685f636d64207b7d7d0a666f7265616368206e685f766172205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6d697373696e675f6e616d6573706163652a5d207b756e736574202d6e6f636f6d706c61696e20246e685f7661727d0a6c697374205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6d697373696e675f6e616d6573706163652a5d205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6d697373696e675f6e616d6573706163652a5d205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6d697373696e675f6e616d6573706163652a5d]
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
        log local0. "NHO|nh20261012broad1|error-missing-namespace|tmm-RULE_INIT|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
set nh_setup [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6c696e6b5f61727261795f656c656d656e74207b7d0a6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6c696e6b5f61727261795f656c656d656e74207b73657420617272286b2920454c454d454e543b2070726f6320746172676574207b7d207b7570766172202330203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6c696e6b5f61727261795f656c656d656e743a3a617272286b29206c696e6b3b20736574206c696e6b286b292056414c55457d7d0a]
set nh_operation [binary format H* 3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6c696e6b5f61727261795f656c656d656e743a3a7461726765740a]
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
        log local0. "NHO|nh20261012broad1|error-link-array-element|tmm-RULE_INIT|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012broad1|error-link-array-element|tmm-RULE_INIT|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 666f7265616368206e685f6368696c64205b696e7465727020736c617665735d207b6966207b5b737472696e67206d61746368205f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6c696e6b5f61727261795f656c656d656e742a20246e685f6368696c645d7d207b696e746572702064656c65746520246e685f6368696c647d7d0a666f7265616368206e685f6e73205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6c696e6b5f61727261795f656c656d656e742a5d207b6e616d6573706163652064656c65746520246e685f6e737d0a666f7265616368206e685f636d64205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6c696e6b5f61727261795f656c656d656e742a5d207b72656e616d6520246e685f636d64207b7d7d0a666f7265616368206e685f766172205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6c696e6b5f61727261795f656c656d656e742a5d207b756e736574202d6e6f636f6d706c61696e20246e685f7661727d0a6c697374205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6c696e6b5f61727261795f656c656d656e742a5d205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6c696e6b5f61727261795f656c656d656e742a5d205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6c696e6b5f61727261795f656c656d656e742a5d]
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
        log local0. "NHO|nh20261012broad1|error-link-array-element|tmm-RULE_INIT|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
Oct 12 08:08:13 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-missing-variable|tmm-RULE_INIT|setup|0|0:0|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:08:13 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-missing-variable|tmm-RULE_INIT|lookup|1|0:0|once|result|binary-scan-tcl-value|0|1|63616e2774207265616420223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6d697373696e675f7661726961626c653a3a616273656e74223a206e6f2073756368207661726961626c65
Oct 12 08:08:13 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-missing-variable|tmm-RULE_INIT|lookup|1|0:0|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:08:13 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-missing-variable|tmm-RULE_INIT|lookup|1|0:0|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:08:13 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-missing-variable|tmm-RULE_INIT|lookup|1|0:0|once|errorinfo|binary-scan-tcl-value|0|2|63616e2774207265616420223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6d697373696e675f7661726961626c653a3a616273656e74223a206e6f2073756368207661726961626c650a202020207768696c6520657865637574696e670a22736574203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6d697373696e675f7661726961626c653a3a616273656e74220a202020202822657661
Oct 12 08:08:13 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-missing-variable|tmm-RULE_INIT|lookup|1|0:0|once|errorinfo|binary-scan-tcl-value|1|2|6c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:08:13 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-missing-variable|tmm-RULE_INIT|runtime-cleanup|0|0:0|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:08:13 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-missing-namespace|tmm-RULE_INIT|setup|0|0:0|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:08:13 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-missing-namespace|tmm-RULE_INIT|lookup|1|0:0|once|result|binary-scan-tcl-value|0|1|756e6b6e6f776e206e616d65737061636520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6d697373696e675f6e616d6573706163653a3a616273656e742220696e20696e73636f7065206e616d65737061636520636f6d6d616e64
Oct 12 08:08:13 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-missing-namespace|tmm-RULE_INIT|lookup|1|0:0|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:08:13 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-missing-namespace|tmm-RULE_INIT|lookup|1|0:0|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:08:13 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-missing-namespace|tmm-RULE_INIT|lookup|1|0:0|once|errorinfo|binary-scan-tcl-value|0|2|756e6b6e6f776e206e616d65737061636520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6d697373696e675f6e616d6573706163653a3a616273656e742220696e20696e73636f7065206e616d65737061636520636f6d6d616e640a202020207768696c6520657865637574696e670a226e616d65737061636520696e73636f7065203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6d69
Oct 12 08:08:13 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-missing-namespace|tmm-RULE_INIT|lookup|1|0:0|once|errorinfo|binary-scan-tcl-value|1|2|7373696e675f6e616d6573706163653a3a616273656e74207b73657420787d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:08:13 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-missing-namespace|tmm-RULE_INIT|runtime-cleanup|0|0:0|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:08:13 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-link-array-element|tmm-RULE_INIT|setup|0|0:0|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:08:13 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-link-array-element|tmm-RULE_INIT|lookup|1|0:0|once|result|binary-scan-tcl-value|0|1|63616e27742073657420226c696e6b286b29223a207661726961626c652069736e2774206172726179
Oct 12 08:08:13 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-link-array-element|tmm-RULE_INIT|lookup|1|0:0|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:08:13 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-link-array-element|tmm-RULE_INIT|lookup|1|0:0|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:08:13 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-link-array-element|tmm-RULE_INIT|lookup|1|0:0|once|errorinfo|binary-scan-tcl-value|0|2|63616e27742073657420226c696e6b286b29223a207661726961626c652069736e27742061727261790a202020207768696c6520657865637574696e670a22736574206c696e6b286b292056414c5545220a202020202870726f63656475726520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6c696e6b5f61727261795f656c656d656e742e2e2e22206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a223a3a5f5f
Oct 12 08:08:13 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-link-array-element|tmm-RULE_INIT|lookup|1|0:0|once|errorinfo|binary-scan-tcl-value|1|2|74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6c696e6b5f61727261795f656c656d656e743a3a746172676574220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:08:13 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-link-array-element|tmm-RULE_INIT|runtime-cleanup|0|0:0|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- error-missing-variable, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- error-missing-variable, lookup, completion 1, result, chunk 0 of 1:

```text-visible
can't read "::__tcl_lsp_probe_nh20261012broad15_error_missing_variable::absent": no such variable⟦EOF⟧
```

- error-missing-variable, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- error-missing-variable, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- error-missing-variable, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
can't read "::__tcl_lsp_probe_nh20261012broad15_error_missing_variable::absent": no such variable
    while executing
"set ::__tcl_lsp_probe_nh20261012broad15_error_missing_variable::absent"
    ("eva⟦EOF⟧
```

- error-missing-variable, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
l" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- error-missing-variable, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- error-missing-namespace, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- error-missing-namespace, lookup, completion 1, result, chunk 0 of 1:

```text-visible
unknown namespace "::__tcl_lsp_probe_nh20261012broad15_error_missing_namespace::absent" in inscope namespace command⟦EOF⟧
```

- error-missing-namespace, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- error-missing-namespace, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- error-missing-namespace, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
unknown namespace "::__tcl_lsp_probe_nh20261012broad15_error_missing_namespace::absent" in inscope namespace command
    while executing
"namespace inscope ::__tcl_lsp_probe_nh20261012broad15_error_mi⟦EOF⟧
```

- error-missing-namespace, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
ssing_namespace::absent {set x}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- error-missing-namespace, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- error-link-array-element, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- error-link-array-element, lookup, completion 1, result, chunk 0 of 1:

```text-visible
can't set "link(k)": variable isn't array⟦EOF⟧
```

- error-link-array-element, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- error-link-array-element, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- error-link-array-element, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
can't set "link(k)": variable isn't array
    while executing
"set link(k) VALUE"
    (procedure "::__tcl_lsp_probe_nh20261012broad15_error_link_array_element..." line 1)
    invoked from within
"::__⟦EOF⟧
```

- error-link-array-element, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
tcl_lsp_probe_nh20261012broad15_error_link_array_element::target"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- error-link-array-element, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- error-link-array-element: intended operation reached.

- error-missing-namespace: intended operation reached.

- error-missing-variable: intended operation reached.

### tmm-RULE_INIT, unit 0:1, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:08:13 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-missing-variable|tmm-RULE_INIT|setup|0|0:1|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:08:13 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-missing-variable|tmm-RULE_INIT|lookup|1|0:1|once|result|binary-scan-tcl-value|0|1|63616e2774207265616420223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6d697373696e675f7661726961626c653a3a616273656e74223a206e6f2073756368207661726961626c65
Oct 12 08:08:13 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-missing-variable|tmm-RULE_INIT|lookup|1|0:1|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:08:13 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-missing-variable|tmm-RULE_INIT|lookup|1|0:1|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:08:13 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-missing-variable|tmm-RULE_INIT|lookup|1|0:1|once|errorinfo|binary-scan-tcl-value|0|2|63616e2774207265616420223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6d697373696e675f7661726961626c653a3a616273656e74223a206e6f2073756368207661726961626c650a202020207768696c6520657865637574696e670a22736574203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6d697373696e675f7661726961626c653a3a616273656e74220a202020202822657661
Oct 12 08:08:13 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-missing-variable|tmm-RULE_INIT|lookup|1|0:1|once|errorinfo|binary-scan-tcl-value|1|2|6c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:08:13 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-missing-variable|tmm-RULE_INIT|runtime-cleanup|0|0:1|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:08:13 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-missing-namespace|tmm-RULE_INIT|setup|0|0:1|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:08:13 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-missing-namespace|tmm-RULE_INIT|lookup|1|0:1|once|result|binary-scan-tcl-value|0|1|756e6b6e6f776e206e616d65737061636520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6d697373696e675f6e616d6573706163653a3a616273656e742220696e20696e73636f7065206e616d65737061636520636f6d6d616e64
Oct 12 08:08:13 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-missing-namespace|tmm-RULE_INIT|lookup|1|0:1|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:08:13 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-missing-namespace|tmm-RULE_INIT|lookup|1|0:1|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:08:13 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-missing-namespace|tmm-RULE_INIT|lookup|1|0:1|once|errorinfo|binary-scan-tcl-value|0|2|756e6b6e6f776e206e616d65737061636520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6d697373696e675f6e616d6573706163653a3a616273656e742220696e20696e73636f7065206e616d65737061636520636f6d6d616e640a202020207768696c6520657865637574696e670a226e616d65737061636520696e73636f7065203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6d69
Oct 12 08:08:13 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-missing-namespace|tmm-RULE_INIT|lookup|1|0:1|once|errorinfo|binary-scan-tcl-value|1|2|7373696e675f6e616d6573706163653a3a616273656e74207b73657420787d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:08:13 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-missing-namespace|tmm-RULE_INIT|runtime-cleanup|0|0:1|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:08:13 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-link-array-element|tmm-RULE_INIT|setup|0|0:1|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:08:13 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-link-array-element|tmm-RULE_INIT|lookup|1|0:1|once|result|binary-scan-tcl-value|0|1|63616e27742073657420226c696e6b286b29223a207661726961626c652069736e2774206172726179
Oct 12 08:08:13 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-link-array-element|tmm-RULE_INIT|lookup|1|0:1|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:08:13 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-link-array-element|tmm-RULE_INIT|lookup|1|0:1|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:08:13 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-link-array-element|tmm-RULE_INIT|lookup|1|0:1|once|errorinfo|binary-scan-tcl-value|0|2|63616e27742073657420226c696e6b286b29223a207661726961626c652069736e27742061727261790a202020207768696c6520657865637574696e670a22736574206c696e6b286b292056414c5545220a202020202870726f63656475726520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6c696e6b5f61727261795f656c656d656e742e2e2e22206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a223a3a5f5f
Oct 12 08:08:13 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-link-array-element|tmm-RULE_INIT|lookup|1|0:1|once|errorinfo|binary-scan-tcl-value|1|2|74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6c696e6b5f61727261795f656c656d656e743a3a746172676574220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:08:13 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-link-array-element|tmm-RULE_INIT|runtime-cleanup|0|0:1|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- error-missing-variable, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- error-missing-variable, lookup, completion 1, result, chunk 0 of 1:

```text-visible
can't read "::__tcl_lsp_probe_nh20261012broad15_error_missing_variable::absent": no such variable⟦EOF⟧
```

- error-missing-variable, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- error-missing-variable, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- error-missing-variable, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
can't read "::__tcl_lsp_probe_nh20261012broad15_error_missing_variable::absent": no such variable
    while executing
"set ::__tcl_lsp_probe_nh20261012broad15_error_missing_variable::absent"
    ("eva⟦EOF⟧
```

- error-missing-variable, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
l" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- error-missing-variable, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- error-missing-namespace, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- error-missing-namespace, lookup, completion 1, result, chunk 0 of 1:

```text-visible
unknown namespace "::__tcl_lsp_probe_nh20261012broad15_error_missing_namespace::absent" in inscope namespace command⟦EOF⟧
```

- error-missing-namespace, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- error-missing-namespace, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- error-missing-namespace, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
unknown namespace "::__tcl_lsp_probe_nh20261012broad15_error_missing_namespace::absent" in inscope namespace command
    while executing
"namespace inscope ::__tcl_lsp_probe_nh20261012broad15_error_mi⟦EOF⟧
```

- error-missing-namespace, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
ssing_namespace::absent {set x}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- error-missing-namespace, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- error-link-array-element, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- error-link-array-element, lookup, completion 1, result, chunk 0 of 1:

```text-visible
can't set "link(k)": variable isn't array⟦EOF⟧
```

- error-link-array-element, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- error-link-array-element, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- error-link-array-element, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
can't set "link(k)": variable isn't array
    while executing
"set link(k) VALUE"
    (procedure "::__tcl_lsp_probe_nh20261012broad15_error_link_array_element..." line 1)
    invoked from within
"::__⟦EOF⟧
```

- error-link-array-element, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
tcl_lsp_probe_nh20261012broad15_error_link_array_element::target"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- error-link-array-element, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- error-link-array-element: intended operation reached.

- error-missing-namespace: intended operation reached.

- error-missing-variable: intended operation reached.

### tmm-RULE_INIT, unit 0:2, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:08:13 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-missing-variable|tmm-RULE_INIT|setup|0|0:2|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:08:13 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-missing-variable|tmm-RULE_INIT|lookup|1|0:2|once|result|binary-scan-tcl-value|0|1|63616e2774207265616420223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6d697373696e675f7661726961626c653a3a616273656e74223a206e6f2073756368207661726961626c65
Oct 12 08:08:13 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-missing-variable|tmm-RULE_INIT|lookup|1|0:2|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:08:13 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-missing-variable|tmm-RULE_INIT|lookup|1|0:2|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:08:13 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-missing-variable|tmm-RULE_INIT|lookup|1|0:2|once|errorinfo|binary-scan-tcl-value|0|2|63616e2774207265616420223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6d697373696e675f7661726961626c653a3a616273656e74223a206e6f2073756368207661726961626c650a202020207768696c6520657865637574696e670a22736574203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6d697373696e675f7661726961626c653a3a616273656e74220a202020202822657661
Oct 12 08:08:13 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-missing-variable|tmm-RULE_INIT|lookup|1|0:2|once|errorinfo|binary-scan-tcl-value|1|2|6c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:08:13 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-missing-variable|tmm-RULE_INIT|runtime-cleanup|0|0:2|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:08:13 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-missing-namespace|tmm-RULE_INIT|setup|0|0:2|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:08:13 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-missing-namespace|tmm-RULE_INIT|lookup|1|0:2|once|result|binary-scan-tcl-value|0|1|756e6b6e6f776e206e616d65737061636520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6d697373696e675f6e616d6573706163653a3a616273656e742220696e20696e73636f7065206e616d65737061636520636f6d6d616e64
Oct 12 08:08:13 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-missing-namespace|tmm-RULE_INIT|lookup|1|0:2|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:08:13 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-missing-namespace|tmm-RULE_INIT|lookup|1|0:2|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:08:13 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-missing-namespace|tmm-RULE_INIT|lookup|1|0:2|once|errorinfo|binary-scan-tcl-value|0|2|756e6b6e6f776e206e616d65737061636520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6d697373696e675f6e616d6573706163653a3a616273656e742220696e20696e73636f7065206e616d65737061636520636f6d6d616e640a202020207768696c6520657865637574696e670a226e616d65737061636520696e73636f7065203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6d69
Oct 12 08:08:13 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-missing-namespace|tmm-RULE_INIT|lookup|1|0:2|once|errorinfo|binary-scan-tcl-value|1|2|7373696e675f6e616d6573706163653a3a616273656e74207b73657420787d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:08:13 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-missing-namespace|tmm-RULE_INIT|runtime-cleanup|0|0:2|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:08:13 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-link-array-element|tmm-RULE_INIT|setup|0|0:2|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:08:13 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-link-array-element|tmm-RULE_INIT|lookup|1|0:2|once|result|binary-scan-tcl-value|0|1|63616e27742073657420226c696e6b286b29223a207661726961626c652069736e2774206172726179
Oct 12 08:08:13 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-link-array-element|tmm-RULE_INIT|lookup|1|0:2|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:08:13 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-link-array-element|tmm-RULE_INIT|lookup|1|0:2|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:08:13 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-link-array-element|tmm-RULE_INIT|lookup|1|0:2|once|errorinfo|binary-scan-tcl-value|0|2|63616e27742073657420226c696e6b286b29223a207661726961626c652069736e27742061727261790a202020207768696c6520657865637574696e670a22736574206c696e6b286b292056414c5545220a202020202870726f63656475726520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6c696e6b5f61727261795f656c656d656e742e2e2e22206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a223a3a5f5f
Oct 12 08:08:13 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-link-array-element|tmm-RULE_INIT|lookup|1|0:2|once|errorinfo|binary-scan-tcl-value|1|2|74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6c696e6b5f61727261795f656c656d656e743a3a746172676574220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:08:13 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-link-array-element|tmm-RULE_INIT|runtime-cleanup|0|0:2|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- error-missing-variable, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- error-missing-variable, lookup, completion 1, result, chunk 0 of 1:

```text-visible
can't read "::__tcl_lsp_probe_nh20261012broad15_error_missing_variable::absent": no such variable⟦EOF⟧
```

- error-missing-variable, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- error-missing-variable, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- error-missing-variable, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
can't read "::__tcl_lsp_probe_nh20261012broad15_error_missing_variable::absent": no such variable
    while executing
"set ::__tcl_lsp_probe_nh20261012broad15_error_missing_variable::absent"
    ("eva⟦EOF⟧
```

- error-missing-variable, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
l" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- error-missing-variable, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- error-missing-namespace, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- error-missing-namespace, lookup, completion 1, result, chunk 0 of 1:

```text-visible
unknown namespace "::__tcl_lsp_probe_nh20261012broad15_error_missing_namespace::absent" in inscope namespace command⟦EOF⟧
```

- error-missing-namespace, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- error-missing-namespace, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- error-missing-namespace, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
unknown namespace "::__tcl_lsp_probe_nh20261012broad15_error_missing_namespace::absent" in inscope namespace command
    while executing
"namespace inscope ::__tcl_lsp_probe_nh20261012broad15_error_mi⟦EOF⟧
```

- error-missing-namespace, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
ssing_namespace::absent {set x}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- error-missing-namespace, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- error-link-array-element, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- error-link-array-element, lookup, completion 1, result, chunk 0 of 1:

```text-visible
can't set "link(k)": variable isn't array⟦EOF⟧
```

- error-link-array-element, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- error-link-array-element, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- error-link-array-element, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
can't set "link(k)": variable isn't array
    while executing
"set link(k) VALUE"
    (procedure "::__tcl_lsp_probe_nh20261012broad15_error_link_array_element..." line 1)
    invoked from within
"::__⟦EOF⟧
```

- error-link-array-element, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
tcl_lsp_probe_nh20261012broad15_error_link_array_element::target"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- error-link-array-element, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- error-link-array-element: intended operation reached.

- error-missing-namespace: intended operation reached.

- error-missing-variable: intended operation reached.

### tmm-RULE_INIT, unit 0:3, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:08:13 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-missing-variable|tmm-RULE_INIT|setup|0|0:3|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:08:13 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-missing-variable|tmm-RULE_INIT|lookup|1|0:3|once|result|binary-scan-tcl-value|0|1|63616e2774207265616420223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6d697373696e675f7661726961626c653a3a616273656e74223a206e6f2073756368207661726961626c65
Oct 12 08:08:13 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-missing-variable|tmm-RULE_INIT|lookup|1|0:3|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:08:13 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-missing-variable|tmm-RULE_INIT|lookup|1|0:3|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:08:13 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-missing-variable|tmm-RULE_INIT|lookup|1|0:3|once|errorinfo|binary-scan-tcl-value|0|2|63616e2774207265616420223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6d697373696e675f7661726961626c653a3a616273656e74223a206e6f2073756368207661726961626c650a202020207768696c6520657865637574696e670a22736574203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6d697373696e675f7661726961626c653a3a616273656e74220a202020202822657661
Oct 12 08:08:13 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-missing-variable|tmm-RULE_INIT|lookup|1|0:3|once|errorinfo|binary-scan-tcl-value|1|2|6c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:08:13 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-missing-variable|tmm-RULE_INIT|runtime-cleanup|0|0:3|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:08:13 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-missing-namespace|tmm-RULE_INIT|setup|0|0:3|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:08:13 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-missing-namespace|tmm-RULE_INIT|lookup|1|0:3|once|result|binary-scan-tcl-value|0|1|756e6b6e6f776e206e616d65737061636520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6d697373696e675f6e616d6573706163653a3a616273656e742220696e20696e73636f7065206e616d65737061636520636f6d6d616e64
Oct 12 08:08:13 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-missing-namespace|tmm-RULE_INIT|lookup|1|0:3|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:08:13 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-missing-namespace|tmm-RULE_INIT|lookup|1|0:3|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:08:13 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-missing-namespace|tmm-RULE_INIT|lookup|1|0:3|once|errorinfo|binary-scan-tcl-value|0|2|756e6b6e6f776e206e616d65737061636520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6d697373696e675f6e616d6573706163653a3a616273656e742220696e20696e73636f7065206e616d65737061636520636f6d6d616e640a202020207768696c6520657865637574696e670a226e616d65737061636520696e73636f7065203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6d69
Oct 12 08:08:13 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-missing-namespace|tmm-RULE_INIT|lookup|1|0:3|once|errorinfo|binary-scan-tcl-value|1|2|7373696e675f6e616d6573706163653a3a616273656e74207b73657420787d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:08:13 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-missing-namespace|tmm-RULE_INIT|runtime-cleanup|0|0:3|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:08:13 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-link-array-element|tmm-RULE_INIT|setup|0|0:3|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:08:13 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-link-array-element|tmm-RULE_INIT|lookup|1|0:3|once|result|binary-scan-tcl-value|0|1|63616e27742073657420226c696e6b286b29223a207661726961626c652069736e2774206172726179
Oct 12 08:08:13 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-link-array-element|tmm-RULE_INIT|lookup|1|0:3|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:08:13 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-link-array-element|tmm-RULE_INIT|lookup|1|0:3|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:08:13 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-link-array-element|tmm-RULE_INIT|lookup|1|0:3|once|errorinfo|binary-scan-tcl-value|0|2|63616e27742073657420226c696e6b286b29223a207661726961626c652069736e27742061727261790a202020207768696c6520657865637574696e670a22736574206c696e6b286b292056414c5545220a202020202870726f63656475726520223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6c696e6b5f61727261795f656c656d656e742e2e2e22206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a223a3a5f5f
Oct 12 08:08:13 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-link-array-element|tmm-RULE_INIT|lookup|1|0:3|once|errorinfo|binary-scan-tcl-value|1|2|74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f6c696e6b5f61727261795f656c656d656e743a3a746172676574220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:08:13 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_17 <RULE_INIT>: NHO|nh20261012broad1|error-link-array-element|tmm-RULE_INIT|runtime-cleanup|0|0:3|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- error-missing-variable, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- error-missing-variable, lookup, completion 1, result, chunk 0 of 1:

```text-visible
can't read "::__tcl_lsp_probe_nh20261012broad15_error_missing_variable::absent": no such variable⟦EOF⟧
```

- error-missing-variable, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- error-missing-variable, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- error-missing-variable, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
can't read "::__tcl_lsp_probe_nh20261012broad15_error_missing_variable::absent": no such variable
    while executing
"set ::__tcl_lsp_probe_nh20261012broad15_error_missing_variable::absent"
    ("eva⟦EOF⟧
```

- error-missing-variable, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
l" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- error-missing-variable, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- error-missing-namespace, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- error-missing-namespace, lookup, completion 1, result, chunk 0 of 1:

```text-visible
unknown namespace "::__tcl_lsp_probe_nh20261012broad15_error_missing_namespace::absent" in inscope namespace command⟦EOF⟧
```

- error-missing-namespace, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- error-missing-namespace, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- error-missing-namespace, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
unknown namespace "::__tcl_lsp_probe_nh20261012broad15_error_missing_namespace::absent" in inscope namespace command
    while executing
"namespace inscope ::__tcl_lsp_probe_nh20261012broad15_error_mi⟦EOF⟧
```

- error-missing-namespace, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
ssing_namespace::absent {set x}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- error-missing-namespace, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- error-link-array-element, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- error-link-array-element, lookup, completion 1, result, chunk 0 of 1:

```text-visible
can't set "link(k)": variable isn't array⟦EOF⟧
```

- error-link-array-element, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- error-link-array-element, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- error-link-array-element, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
can't set "link(k)": variable isn't array
    while executing
"set link(k) VALUE"
    (procedure "::__tcl_lsp_probe_nh20261012broad15_error_link_array_element..." line 1)
    invoked from within
"::__⟦EOF⟧
```

- error-link-array-element, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
tcl_lsp_probe_nh20261012broad15_error_link_array_element::target"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- error-link-array-element, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- error-link-array-element: intended operation reached.

- error-missing-namespace: intended operation reached.

- error-missing-variable: intended operation reached.
