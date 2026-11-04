# BIG-IP Tcl log: nh20261012broad1, tmm-RULE_INIT-18

## Question

The following cases investigate Tcl behavior relevant to name resolution and diagnostics in the LSP, compiler and VM. Each observation is limited to its own case and recorded context.

- **error-import-conflict:** Error conformance: missing command/variable/namespace, forbidden link and rename/import conflict: result, completion, `-errorcode`, `-errorinfo`, options.

- **error-rename-conflict:** Error conformance: missing command/variable/namespace, forbidden link and rename/import conflict: result, completion, `-errorcode`, `-errorinfo`, options.

- **name-braced-head:** Do lexical accepted names and runtime evaluated names differ? Compare braced/quoted/substituted/list command prefixes, brace-newline rules and unaddressable-name output round trips.

## Assumptions

- **error-import-conflict:** Non-force import conflict between owned commands.
- **error-rename-conflict:** Conflict between owned commands only.
- **name-braced-head:** Braced spaced command spelling.

Other assumptions were not separately recorded.

## Script

```tcl-visible
set nh_unit "[TMM::cmp_group]:[TMM::cmp_unit]"
set nh_invocation once
set nh_setup [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f72656e616d655f636f6e666c696374207b7d0a6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f72656e616d655f636f6e666c696374207b70726f6320736f75726365207b7d207b72657475726e20534f555243457d3b2070726f632064657374696e6174696f6e207b7d207b72657475726e2044455354494e4154494f4e7d7d0a]
set nh_operation [binary format H* 72656e616d65203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f72656e616d655f636f6e666c6963743a3a736f75726365203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f72656e616d655f636f6e666c6963743a3a64657374696e6174696f6e0a]
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
        log local0. "NHO|nh20261012broad1|error-rename-conflict|tmm-RULE_INIT|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012broad1|error-rename-conflict|tmm-RULE_INIT|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 666f7265616368206e685f6368696c64205b696e7465727020736c617665735d207b6966207b5b737472696e67206d61746368205f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f72656e616d655f636f6e666c6963742a20246e685f6368696c645d7d207b696e746572702064656c65746520246e685f6368696c647d7d0a666f7265616368206e685f6e73205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f72656e616d655f636f6e666c6963742a5d207b6e616d6573706163652064656c65746520246e685f6e737d0a666f7265616368206e685f636d64205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f72656e616d655f636f6e666c6963742a5d207b72656e616d6520246e685f636d64207b7d7d0a666f7265616368206e685f766172205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f72656e616d655f636f6e666c6963742a5d207b756e736574202d6e6f636f6d706c61696e20246e685f7661727d0a6c697374205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f72656e616d655f636f6e666c6963742a5d205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f72656e616d655f636f6e666c6963742a5d205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f72656e616d655f636f6e666c6963742a5d]
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
        log local0. "NHO|nh20261012broad1|error-rename-conflict|tmm-RULE_INIT|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
set nh_setup [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f696d706f72745f636f6e666c696374207b7d0a6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f696d706f72745f636f6e666c6963743a3a736f75726365207b70726f632070207b7d207b72657475726e20534f555243457d3b206e616d657370616365206578706f727420707d0a6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f696d706f72745f636f6e666c6963743a3a636f6e73756d6572207b70726f632070207b7d207b72657475726e204c4f43414c7d7d0a]
set nh_operation [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f696d706f72745f636f6e666c6963743a3a636f6e73756d6572207b6e616d65737061636520696d706f7274203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f696d706f72745f636f6e666c6963743a3a736f757263653a3a707d0a]
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
        log local0. "NHO|nh20261012broad1|error-import-conflict|tmm-RULE_INIT|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012broad1|error-import-conflict|tmm-RULE_INIT|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 666f7265616368206e685f6368696c64205b696e7465727020736c617665735d207b6966207b5b737472696e67206d61746368205f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f696d706f72745f636f6e666c6963742a20246e685f6368696c645d7d207b696e746572702064656c65746520246e685f6368696c647d7d0a666f7265616368206e685f6e73205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f696d706f72745f636f6e666c6963742a5d207b6e616d6573706163652064656c65746520246e685f6e737d0a666f7265616368206e685f636d64205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f696d706f72745f636f6e666c6963742a5d207b72656e616d6520246e685f636d64207b7d7d0a666f7265616368206e685f766172205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f696d706f72745f636f6e666c6963742a5d207b756e736574202d6e6f636f6d706c61696e20246e685f7661727d0a6c697374205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f696d706f72745f636f6e666c6963742a5d205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f696d706f72745f636f6e666c6963742a5d205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f696d706f72745f636f6e666c6963742a5d]
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
        log local0. "NHO|nh20261012broad1|error-import-conflict|tmm-RULE_INIT|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
set nh_setup [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6e616d655f6272616365645f68656164207b7d0a6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6e616d655f6272616365645f68656164207b70726f63207b74776f20776f7264737d2078207b72657475726e2024787d7d0a]
set nh_operation [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6e616d655f6272616365645f68656164207b7b74776f20776f7264737d207b6120627d7d0a]
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
        log local0. "NHO|nh20261012broad1|name-braced-head|tmm-RULE_INIT|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012broad1|name-braced-head|tmm-RULE_INIT|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 666f7265616368206e685f6368696c64205b696e7465727020736c617665735d207b6966207b5b737472696e67206d61746368205f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6e616d655f6272616365645f686561642a20246e685f6368696c645d7d207b696e746572702064656c65746520246e685f6368696c647d7d0a666f7265616368206e685f6e73205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6e616d655f6272616365645f686561642a5d207b6e616d6573706163652064656c65746520246e685f6e737d0a666f7265616368206e685f636d64205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6e616d655f6272616365645f686561642a5d207b72656e616d6520246e685f636d64207b7d7d0a666f7265616368206e685f766172205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6e616d655f6272616365645f686561642a5d207b756e736574202d6e6f636f6d706c61696e20246e685f7661727d0a6c697374205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6e616d655f6272616365645f686561642a5d205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6e616d655f6272616365645f686561642a5d205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6e616d655f6272616365645f686561642a5d]
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
        log local0. "NHO|nh20261012broad1|name-braced-head|tmm-RULE_INIT|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
Oct 12 08:08:20 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_18 <RULE_INIT>: NHO|nh20261012broad1|error-rename-conflict|tmm-RULE_INIT|setup|0|0:0|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:08:20 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_18 <RULE_INIT>: NHO|nh20261012broad1|error-rename-conflict|tmm-RULE_INIT|lookup|1|0:0|once|result|binary-scan-tcl-value|0|1|63616e27742072656e616d6520746f20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f72656e616d655f636f6e666c6963743a3a64657374696e6174696f6e223a20636f6d6d616e6420616c726561647920657869737473
Oct 12 08:08:20 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_18 <RULE_INIT>: NHO|nh20261012broad1|error-rename-conflict|tmm-RULE_INIT|lookup|1|0:0|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:08:20 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_18 <RULE_INIT>: NHO|nh20261012broad1|error-rename-conflict|tmm-RULE_INIT|lookup|1|0:0|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:08:20 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_18 <RULE_INIT>: NHO|nh20261012broad1|error-rename-conflict|tmm-RULE_INIT|lookup|1|0:0|once|errorinfo|binary-scan-tcl-value|0|2|63616e27742072656e616d6520746f20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f72656e616d655f636f6e666c6963743a3a64657374696e6174696f6e223a20636f6d6d616e6420616c7265616479206578697374730a202020207768696c6520657865637574696e670a2272656e616d65203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f72656e616d655f636f6e666c6963743a3a
Oct 12 08:08:20 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_18 <RULE_INIT>: NHO|nh20261012broad1|error-rename-conflict|tmm-RULE_INIT|lookup|1|0:0|once|errorinfo|binary-scan-tcl-value|1|2|736f75726365203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f72656e616d655f636f6e666c6963743a3a64657374696e6174696f6e220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:08:20 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_18 <RULE_INIT>: NHO|nh20261012broad1|error-rename-conflict|tmm-RULE_INIT|runtime-cleanup|0|0:0|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:08:20 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_18 <RULE_INIT>: NHO|nh20261012broad1|error-import-conflict|tmm-RULE_INIT|setup|0|0:0|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:08:20 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_18 <RULE_INIT>: NHO|nh20261012broad1|error-import-conflict|tmm-RULE_INIT|lookup|1|0:0|once|result|binary-scan-tcl-value|0|1|63616e277420696d706f727420636f6d6d616e64202270223a20616c726561647920657869737473
Oct 12 08:08:20 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_18 <RULE_INIT>: NHO|nh20261012broad1|error-import-conflict|tmm-RULE_INIT|lookup|1|0:0|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:08:20 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_18 <RULE_INIT>: NHO|nh20261012broad1|error-import-conflict|tmm-RULE_INIT|lookup|1|0:0|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:08:20 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_18 <RULE_INIT>: NHO|nh20261012broad1|error-import-conflict|tmm-RULE_INIT|lookup|1|0:0|once|errorinfo|binary-scan-tcl-value|0|3|63616e277420696d706f727420636f6d6d616e64202270223a20616c7265616479206578697374730a202020207768696c6520657865637574696e670a226e616d65737061636520696d706f7274203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f696d706f72745f636f6e666c6963743a3a736f757263653a3a70220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e6832303236313031
Oct 12 08:08:20 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_18 <RULE_INIT>: NHO|nh20261012broad1|error-import-conflict|tmm-RULE_INIT|lookup|1|0:0|once|errorinfo|binary-scan-tcl-value|1|3|3262726f616431355f6572726f725f696d706f72745f636f6e666c6963743a3a636f6e73756d65722220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f696d706f72745f636f6e666c6963743a3a636f6e73756d6572207b6e616d65737061636520696d706f7274203a3a5f5f74636c5f6c73705f70726f6265
Oct 12 08:08:20 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_18 <RULE_INIT>: NHO|nh20261012broad1|error-import-conflict|tmm-RULE_INIT|lookup|1|0:0|once|errorinfo|binary-scan-tcl-value|2|3|5f6e68323032363130313262726f616431355f6572726f725f696d706f72745f2e2e2e220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:08:20 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_18 <RULE_INIT>: NHO|nh20261012broad1|error-import-conflict|tmm-RULE_INIT|runtime-cleanup|0|0:0|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:08:20 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_18 <RULE_INIT>: NHO|nh20261012broad1|name-braced-head|tmm-RULE_INIT|setup|0|0:0|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:08:20 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_18 <RULE_INIT>: NHO|nh20261012broad1|name-braced-head|tmm-RULE_INIT|lookup|0|0:0|once|result|binary-scan-tcl-value|0|1|612062
Oct 12 08:08:20 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_18 <RULE_INIT>: NHO|nh20261012broad1|name-braced-head|tmm-RULE_INIT|runtime-cleanup|0|0:0|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- error-rename-conflict, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- error-rename-conflict, lookup, completion 1, result, chunk 0 of 1:

```text-visible
can't rename to "::__tcl_lsp_probe_nh20261012broad15_error_rename_conflict::destination": command already exists⟦EOF⟧
```

- error-rename-conflict, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- error-rename-conflict, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- error-rename-conflict, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
can't rename to "::__tcl_lsp_probe_nh20261012broad15_error_rename_conflict::destination": command already exists
    while executing
"rename ::__tcl_lsp_probe_nh20261012broad15_error_rename_conflict::⟦EOF⟧
```

- error-rename-conflict, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
source ::__tcl_lsp_probe_nh20261012broad15_error_rename_conflict::destination"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- error-rename-conflict, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- error-import-conflict, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- error-import-conflict, lookup, completion 1, result, chunk 0 of 1:

```text-visible
can't import command "p": already exists⟦EOF⟧
```

- error-import-conflict, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- error-import-conflict, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- error-import-conflict, lookup, completion 1, errorinfo, chunk 0 of 3:

```text-visible
can't import command "p": already exists
    while executing
"namespace import ::__tcl_lsp_probe_nh20261012broad15_error_import_conflict::source::p"
    (in namespace eval "::__tcl_lsp_probe_nh2026101⟦EOF⟧
```

- error-import-conflict, lookup, completion 1, errorinfo, chunk 1 of 3:

```text-visible
2broad15_error_import_conflict::consumer" script line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad15_error_import_conflict::consumer {namespace import ::__tcl_lsp_probe⟦EOF⟧
```

- error-import-conflict, lookup, completion 1, errorinfo, chunk 2 of 3:

```text-visible
_nh20261012broad15_error_import_..."
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- error-import-conflict, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- name-braced-head, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- name-braced-head, lookup, completion 0, result, chunk 0 of 1:

```text-visible
a b⟦EOF⟧
```

- name-braced-head, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- error-import-conflict: intended operation reached.

- error-rename-conflict: intended operation reached.

- name-braced-head: intended operation reached.

### tmm-RULE_INIT, unit 0:1, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:08:20 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_18 <RULE_INIT>: NHO|nh20261012broad1|error-rename-conflict|tmm-RULE_INIT|setup|0|0:1|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:08:20 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_18 <RULE_INIT>: NHO|nh20261012broad1|error-rename-conflict|tmm-RULE_INIT|lookup|1|0:1|once|result|binary-scan-tcl-value|0|1|63616e27742072656e616d6520746f20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f72656e616d655f636f6e666c6963743a3a64657374696e6174696f6e223a20636f6d6d616e6420616c726561647920657869737473
Oct 12 08:08:20 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_18 <RULE_INIT>: NHO|nh20261012broad1|error-rename-conflict|tmm-RULE_INIT|lookup|1|0:1|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:08:20 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_18 <RULE_INIT>: NHO|nh20261012broad1|error-rename-conflict|tmm-RULE_INIT|lookup|1|0:1|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:08:20 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_18 <RULE_INIT>: NHO|nh20261012broad1|error-rename-conflict|tmm-RULE_INIT|lookup|1|0:1|once|errorinfo|binary-scan-tcl-value|0|2|63616e27742072656e616d6520746f20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f72656e616d655f636f6e666c6963743a3a64657374696e6174696f6e223a20636f6d6d616e6420616c7265616479206578697374730a202020207768696c6520657865637574696e670a2272656e616d65203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f72656e616d655f636f6e666c6963743a3a
Oct 12 08:08:20 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_18 <RULE_INIT>: NHO|nh20261012broad1|error-rename-conflict|tmm-RULE_INIT|lookup|1|0:1|once|errorinfo|binary-scan-tcl-value|1|2|736f75726365203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f72656e616d655f636f6e666c6963743a3a64657374696e6174696f6e220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:08:20 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_18 <RULE_INIT>: NHO|nh20261012broad1|error-rename-conflict|tmm-RULE_INIT|runtime-cleanup|0|0:1|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:08:20 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_18 <RULE_INIT>: NHO|nh20261012broad1|error-import-conflict|tmm-RULE_INIT|setup|0|0:1|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:08:20 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_18 <RULE_INIT>: NHO|nh20261012broad1|error-import-conflict|tmm-RULE_INIT|lookup|1|0:1|once|result|binary-scan-tcl-value|0|1|63616e277420696d706f727420636f6d6d616e64202270223a20616c726561647920657869737473
Oct 12 08:08:20 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_18 <RULE_INIT>: NHO|nh20261012broad1|error-import-conflict|tmm-RULE_INIT|lookup|1|0:1|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:08:20 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_18 <RULE_INIT>: NHO|nh20261012broad1|error-import-conflict|tmm-RULE_INIT|lookup|1|0:1|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:08:20 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_18 <RULE_INIT>: NHO|nh20261012broad1|error-import-conflict|tmm-RULE_INIT|lookup|1|0:1|once|errorinfo|binary-scan-tcl-value|0|3|63616e277420696d706f727420636f6d6d616e64202270223a20616c7265616479206578697374730a202020207768696c6520657865637574696e670a226e616d65737061636520696d706f7274203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f696d706f72745f636f6e666c6963743a3a736f757263653a3a70220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e6832303236313031
Oct 12 08:08:20 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_18 <RULE_INIT>: NHO|nh20261012broad1|error-import-conflict|tmm-RULE_INIT|lookup|1|0:1|once|errorinfo|binary-scan-tcl-value|1|3|3262726f616431355f6572726f725f696d706f72745f636f6e666c6963743a3a636f6e73756d65722220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f696d706f72745f636f6e666c6963743a3a636f6e73756d6572207b6e616d65737061636520696d706f7274203a3a5f5f74636c5f6c73705f70726f6265
Oct 12 08:08:20 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_18 <RULE_INIT>: NHO|nh20261012broad1|error-import-conflict|tmm-RULE_INIT|lookup|1|0:1|once|errorinfo|binary-scan-tcl-value|2|3|5f6e68323032363130313262726f616431355f6572726f725f696d706f72745f2e2e2e220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:08:20 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_18 <RULE_INIT>: NHO|nh20261012broad1|error-import-conflict|tmm-RULE_INIT|runtime-cleanup|0|0:1|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:08:20 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_18 <RULE_INIT>: NHO|nh20261012broad1|name-braced-head|tmm-RULE_INIT|setup|0|0:1|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:08:20 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_18 <RULE_INIT>: NHO|nh20261012broad1|name-braced-head|tmm-RULE_INIT|lookup|0|0:1|once|result|binary-scan-tcl-value|0|1|612062
Oct 12 08:08:20 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_18 <RULE_INIT>: NHO|nh20261012broad1|name-braced-head|tmm-RULE_INIT|runtime-cleanup|0|0:1|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- error-rename-conflict, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- error-rename-conflict, lookup, completion 1, result, chunk 0 of 1:

```text-visible
can't rename to "::__tcl_lsp_probe_nh20261012broad15_error_rename_conflict::destination": command already exists⟦EOF⟧
```

- error-rename-conflict, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- error-rename-conflict, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- error-rename-conflict, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
can't rename to "::__tcl_lsp_probe_nh20261012broad15_error_rename_conflict::destination": command already exists
    while executing
"rename ::__tcl_lsp_probe_nh20261012broad15_error_rename_conflict::⟦EOF⟧
```

- error-rename-conflict, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
source ::__tcl_lsp_probe_nh20261012broad15_error_rename_conflict::destination"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- error-rename-conflict, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- error-import-conflict, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- error-import-conflict, lookup, completion 1, result, chunk 0 of 1:

```text-visible
can't import command "p": already exists⟦EOF⟧
```

- error-import-conflict, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- error-import-conflict, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- error-import-conflict, lookup, completion 1, errorinfo, chunk 0 of 3:

```text-visible
can't import command "p": already exists
    while executing
"namespace import ::__tcl_lsp_probe_nh20261012broad15_error_import_conflict::source::p"
    (in namespace eval "::__tcl_lsp_probe_nh2026101⟦EOF⟧
```

- error-import-conflict, lookup, completion 1, errorinfo, chunk 1 of 3:

```text-visible
2broad15_error_import_conflict::consumer" script line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad15_error_import_conflict::consumer {namespace import ::__tcl_lsp_probe⟦EOF⟧
```

- error-import-conflict, lookup, completion 1, errorinfo, chunk 2 of 3:

```text-visible
_nh20261012broad15_error_import_..."
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- error-import-conflict, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- name-braced-head, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- name-braced-head, lookup, completion 0, result, chunk 0 of 1:

```text-visible
a b⟦EOF⟧
```

- name-braced-head, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- error-import-conflict: intended operation reached.

- error-rename-conflict: intended operation reached.

- name-braced-head: intended operation reached.

### tmm-RULE_INIT, unit 0:2, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:08:20 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_18 <RULE_INIT>: NHO|nh20261012broad1|error-rename-conflict|tmm-RULE_INIT|setup|0|0:2|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:08:20 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_18 <RULE_INIT>: NHO|nh20261012broad1|error-rename-conflict|tmm-RULE_INIT|lookup|1|0:2|once|result|binary-scan-tcl-value|0|1|63616e27742072656e616d6520746f20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f72656e616d655f636f6e666c6963743a3a64657374696e6174696f6e223a20636f6d6d616e6420616c726561647920657869737473
Oct 12 08:08:20 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_18 <RULE_INIT>: NHO|nh20261012broad1|error-rename-conflict|tmm-RULE_INIT|lookup|1|0:2|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:08:20 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_18 <RULE_INIT>: NHO|nh20261012broad1|error-rename-conflict|tmm-RULE_INIT|lookup|1|0:2|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:08:20 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_18 <RULE_INIT>: NHO|nh20261012broad1|error-rename-conflict|tmm-RULE_INIT|lookup|1|0:2|once|errorinfo|binary-scan-tcl-value|0|2|63616e27742072656e616d6520746f20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f72656e616d655f636f6e666c6963743a3a64657374696e6174696f6e223a20636f6d6d616e6420616c7265616479206578697374730a202020207768696c6520657865637574696e670a2272656e616d65203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f72656e616d655f636f6e666c6963743a3a
Oct 12 08:08:20 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_18 <RULE_INIT>: NHO|nh20261012broad1|error-rename-conflict|tmm-RULE_INIT|lookup|1|0:2|once|errorinfo|binary-scan-tcl-value|1|2|736f75726365203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f72656e616d655f636f6e666c6963743a3a64657374696e6174696f6e220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:08:20 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_18 <RULE_INIT>: NHO|nh20261012broad1|error-rename-conflict|tmm-RULE_INIT|runtime-cleanup|0|0:2|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:08:20 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_18 <RULE_INIT>: NHO|nh20261012broad1|error-import-conflict|tmm-RULE_INIT|setup|0|0:2|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:08:20 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_18 <RULE_INIT>: NHO|nh20261012broad1|error-import-conflict|tmm-RULE_INIT|lookup|1|0:2|once|result|binary-scan-tcl-value|0|1|63616e277420696d706f727420636f6d6d616e64202270223a20616c726561647920657869737473
Oct 12 08:08:20 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_18 <RULE_INIT>: NHO|nh20261012broad1|error-import-conflict|tmm-RULE_INIT|lookup|1|0:2|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:08:20 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_18 <RULE_INIT>: NHO|nh20261012broad1|error-import-conflict|tmm-RULE_INIT|lookup|1|0:2|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:08:20 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_18 <RULE_INIT>: NHO|nh20261012broad1|error-import-conflict|tmm-RULE_INIT|lookup|1|0:2|once|errorinfo|binary-scan-tcl-value|0|3|63616e277420696d706f727420636f6d6d616e64202270223a20616c7265616479206578697374730a202020207768696c6520657865637574696e670a226e616d65737061636520696d706f7274203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f696d706f72745f636f6e666c6963743a3a736f757263653a3a70220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e6832303236313031
Oct 12 08:08:20 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_18 <RULE_INIT>: NHO|nh20261012broad1|error-import-conflict|tmm-RULE_INIT|lookup|1|0:2|once|errorinfo|binary-scan-tcl-value|1|3|3262726f616431355f6572726f725f696d706f72745f636f6e666c6963743a3a636f6e73756d65722220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f696d706f72745f636f6e666c6963743a3a636f6e73756d6572207b6e616d65737061636520696d706f7274203a3a5f5f74636c5f6c73705f70726f6265
Oct 12 08:08:20 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_18 <RULE_INIT>: NHO|nh20261012broad1|error-import-conflict|tmm-RULE_INIT|lookup|1|0:2|once|errorinfo|binary-scan-tcl-value|2|3|5f6e68323032363130313262726f616431355f6572726f725f696d706f72745f2e2e2e220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:08:20 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_18 <RULE_INIT>: NHO|nh20261012broad1|error-import-conflict|tmm-RULE_INIT|runtime-cleanup|0|0:2|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:08:20 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_18 <RULE_INIT>: NHO|nh20261012broad1|name-braced-head|tmm-RULE_INIT|setup|0|0:2|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:08:20 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_18 <RULE_INIT>: NHO|nh20261012broad1|name-braced-head|tmm-RULE_INIT|lookup|0|0:2|once|result|binary-scan-tcl-value|0|1|612062
Oct 12 08:08:20 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_18 <RULE_INIT>: NHO|nh20261012broad1|name-braced-head|tmm-RULE_INIT|runtime-cleanup|0|0:2|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- error-rename-conflict, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- error-rename-conflict, lookup, completion 1, result, chunk 0 of 1:

```text-visible
can't rename to "::__tcl_lsp_probe_nh20261012broad15_error_rename_conflict::destination": command already exists⟦EOF⟧
```

- error-rename-conflict, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- error-rename-conflict, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- error-rename-conflict, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
can't rename to "::__tcl_lsp_probe_nh20261012broad15_error_rename_conflict::destination": command already exists
    while executing
"rename ::__tcl_lsp_probe_nh20261012broad15_error_rename_conflict::⟦EOF⟧
```

- error-rename-conflict, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
source ::__tcl_lsp_probe_nh20261012broad15_error_rename_conflict::destination"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- error-rename-conflict, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- error-import-conflict, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- error-import-conflict, lookup, completion 1, result, chunk 0 of 1:

```text-visible
can't import command "p": already exists⟦EOF⟧
```

- error-import-conflict, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- error-import-conflict, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- error-import-conflict, lookup, completion 1, errorinfo, chunk 0 of 3:

```text-visible
can't import command "p": already exists
    while executing
"namespace import ::__tcl_lsp_probe_nh20261012broad15_error_import_conflict::source::p"
    (in namespace eval "::__tcl_lsp_probe_nh2026101⟦EOF⟧
```

- error-import-conflict, lookup, completion 1, errorinfo, chunk 1 of 3:

```text-visible
2broad15_error_import_conflict::consumer" script line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad15_error_import_conflict::consumer {namespace import ::__tcl_lsp_probe⟦EOF⟧
```

- error-import-conflict, lookup, completion 1, errorinfo, chunk 2 of 3:

```text-visible
_nh20261012broad15_error_import_..."
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- error-import-conflict, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- name-braced-head, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- name-braced-head, lookup, completion 0, result, chunk 0 of 1:

```text-visible
a b⟦EOF⟧
```

- name-braced-head, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- error-import-conflict: intended operation reached.

- error-rename-conflict: intended operation reached.

- name-braced-head: intended operation reached.

### tmm-RULE_INIT, unit 0:3, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:08:20 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_18 <RULE_INIT>: NHO|nh20261012broad1|error-rename-conflict|tmm-RULE_INIT|setup|0|0:3|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:08:20 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_18 <RULE_INIT>: NHO|nh20261012broad1|error-rename-conflict|tmm-RULE_INIT|lookup|1|0:3|once|result|binary-scan-tcl-value|0|1|63616e27742072656e616d6520746f20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f72656e616d655f636f6e666c6963743a3a64657374696e6174696f6e223a20636f6d6d616e6420616c726561647920657869737473
Oct 12 08:08:20 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_18 <RULE_INIT>: NHO|nh20261012broad1|error-rename-conflict|tmm-RULE_INIT|lookup|1|0:3|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:08:20 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_18 <RULE_INIT>: NHO|nh20261012broad1|error-rename-conflict|tmm-RULE_INIT|lookup|1|0:3|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:08:20 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_18 <RULE_INIT>: NHO|nh20261012broad1|error-rename-conflict|tmm-RULE_INIT|lookup|1|0:3|once|errorinfo|binary-scan-tcl-value|0|2|63616e27742072656e616d6520746f20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f72656e616d655f636f6e666c6963743a3a64657374696e6174696f6e223a20636f6d6d616e6420616c7265616479206578697374730a202020207768696c6520657865637574696e670a2272656e616d65203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f72656e616d655f636f6e666c6963743a3a
Oct 12 08:08:20 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_18 <RULE_INIT>: NHO|nh20261012broad1|error-rename-conflict|tmm-RULE_INIT|lookup|1|0:3|once|errorinfo|binary-scan-tcl-value|1|2|736f75726365203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f72656e616d655f636f6e666c6963743a3a64657374696e6174696f6e220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:08:20 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_18 <RULE_INIT>: NHO|nh20261012broad1|error-rename-conflict|tmm-RULE_INIT|runtime-cleanup|0|0:3|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:08:20 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_18 <RULE_INIT>: NHO|nh20261012broad1|error-import-conflict|tmm-RULE_INIT|setup|0|0:3|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:08:20 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_18 <RULE_INIT>: NHO|nh20261012broad1|error-import-conflict|tmm-RULE_INIT|lookup|1|0:3|once|result|binary-scan-tcl-value|0|1|63616e277420696d706f727420636f6d6d616e64202270223a20616c726561647920657869737473
Oct 12 08:08:20 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_18 <RULE_INIT>: NHO|nh20261012broad1|error-import-conflict|tmm-RULE_INIT|lookup|1|0:3|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:08:20 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_18 <RULE_INIT>: NHO|nh20261012broad1|error-import-conflict|tmm-RULE_INIT|lookup|1|0:3|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:08:20 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_18 <RULE_INIT>: NHO|nh20261012broad1|error-import-conflict|tmm-RULE_INIT|lookup|1|0:3|once|errorinfo|binary-scan-tcl-value|0|3|63616e277420696d706f727420636f6d6d616e64202270223a20616c7265616479206578697374730a202020207768696c6520657865637574696e670a226e616d65737061636520696d706f7274203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f696d706f72745f636f6e666c6963743a3a736f757263653a3a70220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e6832303236313031
Oct 12 08:08:20 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_18 <RULE_INIT>: NHO|nh20261012broad1|error-import-conflict|tmm-RULE_INIT|lookup|1|0:3|once|errorinfo|binary-scan-tcl-value|1|3|3262726f616431355f6572726f725f696d706f72745f636f6e666c6963743a3a636f6e73756d65722220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6572726f725f696d706f72745f636f6e666c6963743a3a636f6e73756d6572207b6e616d65737061636520696d706f7274203a3a5f5f74636c5f6c73705f70726f6265
Oct 12 08:08:20 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_18 <RULE_INIT>: NHO|nh20261012broad1|error-import-conflict|tmm-RULE_INIT|lookup|1|0:3|once|errorinfo|binary-scan-tcl-value|2|3|5f6e68323032363130313262726f616431355f6572726f725f696d706f72745f2e2e2e220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 08:08:20 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_18 <RULE_INIT>: NHO|nh20261012broad1|error-import-conflict|tmm-RULE_INIT|runtime-cleanup|0|0:3|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:08:20 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_18 <RULE_INIT>: NHO|nh20261012broad1|name-braced-head|tmm-RULE_INIT|setup|0|0:3|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:08:20 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_18 <RULE_INIT>: NHO|nh20261012broad1|name-braced-head|tmm-RULE_INIT|lookup|0|0:3|once|result|binary-scan-tcl-value|0|1|612062
Oct 12 08:08:20 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_18 <RULE_INIT>: NHO|nh20261012broad1|name-braced-head|tmm-RULE_INIT|runtime-cleanup|0|0:3|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- error-rename-conflict, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- error-rename-conflict, lookup, completion 1, result, chunk 0 of 1:

```text-visible
can't rename to "::__tcl_lsp_probe_nh20261012broad15_error_rename_conflict::destination": command already exists⟦EOF⟧
```

- error-rename-conflict, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- error-rename-conflict, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- error-rename-conflict, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
can't rename to "::__tcl_lsp_probe_nh20261012broad15_error_rename_conflict::destination": command already exists
    while executing
"rename ::__tcl_lsp_probe_nh20261012broad15_error_rename_conflict::⟦EOF⟧
```

- error-rename-conflict, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
source ::__tcl_lsp_probe_nh20261012broad15_error_rename_conflict::destination"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- error-rename-conflict, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- error-import-conflict, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- error-import-conflict, lookup, completion 1, result, chunk 0 of 1:

```text-visible
can't import command "p": already exists⟦EOF⟧
```

- error-import-conflict, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- error-import-conflict, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- error-import-conflict, lookup, completion 1, errorinfo, chunk 0 of 3:

```text-visible
can't import command "p": already exists
    while executing
"namespace import ::__tcl_lsp_probe_nh20261012broad15_error_import_conflict::source::p"
    (in namespace eval "::__tcl_lsp_probe_nh2026101⟦EOF⟧
```

- error-import-conflict, lookup, completion 1, errorinfo, chunk 1 of 3:

```text-visible
2broad15_error_import_conflict::consumer" script line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad15_error_import_conflict::consumer {namespace import ::__tcl_lsp_probe⟦EOF⟧
```

- error-import-conflict, lookup, completion 1, errorinfo, chunk 2 of 3:

```text-visible
_nh20261012broad15_error_import_..."
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- error-import-conflict, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- name-braced-head, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- name-braced-head, lookup, completion 0, result, chunk 0 of 1:

```text-visible
a b⟦EOF⟧
```

- name-braced-head, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- error-import-conflict: intended operation reached.

- error-rename-conflict: intended operation reached.

- name-braced-head: intended operation reached.
