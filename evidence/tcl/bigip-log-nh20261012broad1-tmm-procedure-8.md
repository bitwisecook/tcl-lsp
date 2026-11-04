# BIG-IP Tcl log: nh20261012broad1, tmm-procedure-8

## Question

The following cases investigate Tcl behavior relevant to name resolution and diagnostics in the LSP, compiler and VM. Each observation is limited to its own case and recorded context.

- **import-source-redefined:** Import/export/force/forget/origin support and lifecycle; global-origin imports, source rename/redefinition/deletion, chains and conflicts.

- **import-source-renamed:** Import/export/force/forget/origin support and lifecycle; global-origin imports, source rename/redefinition/deletion, chains and conflicts.

- **namespace-upvar-link:** `global`, `variable`, `upvar 0/1/#0`, `namespace upvar`, eval/uplevel composition: exact target frame and cell, including namespace-eval callers.

## Assumptions

- **import-source-redefined:** NH12 adapted names.
- **import-source-renamed:** NH11 adapted names; import invocation and origin text retained separately.
- **namespace-upvar-link:** Namespace-to-namespace link with explicit owner.

Other assumptions were not separately recorded.

## Script

```tcl-visible
set nh_unit "[TMM::cmp_group]:[TMM::cmp_unit]"
set nh_invocation once
set nh_setup [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6e616d6573706163655f75707661725f6c696e6b207b7d0a6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6e616d6573706163655f75707661725f6c696e6b3a3a736f75726365207b7661726961626c652078204f524947494e414c7d0a6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6e616d6573706163655f75707661725f6c696e6b3a3a636f6e73756d6572207b7d0a]
set nh_operation [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6e616d6573706163655f75707661725f6c696e6b3a3a636f6e73756d6572207b6e616d657370616365207570766172203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6e616d6573706163655f75707661725f6c696e6b3a3a736f75726365207820793b207365742079204348414e4745447d3b20736574203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6e616d6573706163655f75707661725f6c696e6b3a3a736f757263653a3a780a]
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
        log local0. "NHO|nh20261012broad1|namespace-upvar-link|tmm-procedure|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012broad1|namespace-upvar-link|tmm-procedure|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 666f7265616368206e685f6368696c64205b696e7465727020736c617665735d207b6966207b5b737472696e67206d61746368205f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6e616d6573706163655f75707661725f6c696e6b2a20246e685f6368696c645d7d207b696e746572702064656c65746520246e685f6368696c647d7d0a666f7265616368206e685f6e73205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6e616d6573706163655f75707661725f6c696e6b2a5d207b6e616d6573706163652064656c65746520246e685f6e737d0a666f7265616368206e685f636d64205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6e616d6573706163655f75707661725f6c696e6b2a5d207b72656e616d6520246e685f636d64207b7d7d0a666f7265616368206e685f766172205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6e616d6573706163655f75707661725f6c696e6b2a5d207b756e736574202d6e6f636f6d706c61696e20246e685f7661727d0a6c697374205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6e616d6573706163655f75707661725f6c696e6b2a5d205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6e616d6573706163655f75707661725f6c696e6b2a5d205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6e616d6573706163655f75707661725f6c696e6b2a5d]
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
        log local0. "NHO|nh20261012broad1|namespace-upvar-link|tmm-procedure|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
set nh_setup [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f696d706f72745f736f757263655f72656e616d6564207b7d0a6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f696d706f72745f736f757263655f72656e616d65643a3a736f75726365207b70726f632070207b7d207b72657475726e204f524947494e414c7d3b206e616d657370616365206578706f727420707d0a6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f696d706f72745f736f757263655f72656e616d65643a3a636f6e73756d6572207b6e616d65737061636520696d706f7274203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f696d706f72745f736f757263655f72656e616d65643a3a736f757263653a3a707d0a72656e616d65203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f696d706f72745f736f757263655f72656e616d65643a3a736f757263653a3a70203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f696d706f72745f736f757263655f72656e616d65643a3a736f757263653a3a70700a]
set nh_operation [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f696d706f72745f736f757263655f72656e616d65643a3a636f6e73756d6572207b6c697374205b705d205b6e616d657370616365206f726967696e20705d7d0a]
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
        log local0. "NHO|nh20261012broad1|import-source-renamed|tmm-procedure|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012broad1|import-source-renamed|tmm-procedure|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 666f7265616368206e685f6368696c64205b696e7465727020736c617665735d207b6966207b5b737472696e67206d61746368205f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f696d706f72745f736f757263655f72656e616d65642a20246e685f6368696c645d7d207b696e746572702064656c65746520246e685f6368696c647d7d0a666f7265616368206e685f6e73205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f696d706f72745f736f757263655f72656e616d65642a5d207b6e616d6573706163652064656c65746520246e685f6e737d0a666f7265616368206e685f636d64205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f696d706f72745f736f757263655f72656e616d65642a5d207b72656e616d6520246e685f636d64207b7d7d0a666f7265616368206e685f766172205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f696d706f72745f736f757263655f72656e616d65642a5d207b756e736574202d6e6f636f6d706c61696e20246e685f7661727d0a6c697374205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f696d706f72745f736f757263655f72656e616d65642a5d205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f696d706f72745f736f757263655f72656e616d65642a5d205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f696d706f72745f736f757263655f72656e616d65642a5d]
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
        log local0. "NHO|nh20261012broad1|import-source-renamed|tmm-procedure|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
set nh_setup [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f696d706f72745f736f757263655f7265646566696e6564207b7d0a6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f696d706f72745f736f757263655f7265646566696e65643a3a736f75726365207b70726f632070207b7d207b72657475726e204f4c447d3b206e616d657370616365206578706f727420707d0a6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f696d706f72745f736f757263655f7265646566696e65643a3a636f6e73756d6572207b6e616d65737061636520696d706f7274203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f696d706f72745f736f757263655f7265646566696e65643a3a736f757263653a3a707d0a70726f63203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f696d706f72745f736f757263655f7265646566696e65643a3a736f757263653a3a70207b7d207b72657475726e204e45577d0a]
set nh_operation [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f696d706f72745f736f757263655f7265646566696e65643a3a636f6e73756d6572207b707d0a]
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
        log local0. "NHO|nh20261012broad1|import-source-redefined|tmm-procedure|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012broad1|import-source-redefined|tmm-procedure|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 666f7265616368206e685f6368696c64205b696e7465727020736c617665735d207b6966207b5b737472696e67206d61746368205f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f696d706f72745f736f757263655f7265646566696e65642a20246e685f6368696c645d7d207b696e746572702064656c65746520246e685f6368696c647d7d0a666f7265616368206e685f6e73205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f696d706f72745f736f757263655f7265646566696e65642a5d207b6e616d6573706163652064656c65746520246e685f6e737d0a666f7265616368206e685f636d64205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f696d706f72745f736f757263655f7265646566696e65642a5d207b72656e616d6520246e685f636d64207b7d7d0a666f7265616368206e685f766172205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f696d706f72745f736f757263655f7265646566696e65642a5d207b756e736574202d6e6f636f6d706c61696e20246e685f7661727d0a6c697374205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f696d706f72745f736f757263655f7265646566696e65642a5d205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f696d706f72745f736f757263655f7265646566696e65642a5d205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f696d706f72745f736f757263655f7265646566696e65642a5d]
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
        log local0. "NHO|nh20261012broad1|import-source-redefined|tmm-procedure|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
```

## Results

### tmm-procedure, unit 0:0, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-procedure.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:09:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_8 <RULE_INIT>: NHO|nh20261012broad1|namespace-upvar-link|tmm-procedure|setup|0|0:0|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:09:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_8 <RULE_INIT>: NHO|nh20261012broad1|namespace-upvar-link|tmm-procedure|lookup|1|0:0|once|result|binary-scan-tcl-value|0|1|626164206f7074696f6e20227570766172223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f72207768696368
Oct 12 08:09:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_8 <RULE_INIT>: NHO|nh20261012broad1|namespace-upvar-link|tmm-procedure|lookup|1|0:0|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:09:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_8 <RULE_INIT>: NHO|nh20261012broad1|namespace-upvar-link|tmm-procedure|lookup|1|0:0|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:09:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_8 <RULE_INIT>: NHO|nh20261012broad1|namespace-upvar-link|tmm-procedure|lookup|1|0:0|once|errorinfo|binary-scan-tcl-value|0|4|626164206f7074696f6e20227570766172223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f722077686963680a202020207768696c6520657865637574696e670a226e616d657370616365207570766172203a3a5f5f74636c5f6c73705f
Oct 12 08:09:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_8 <RULE_INIT>: NHO|nh20261012broad1|namespace-upvar-link|tmm-procedure|lookup|1|0:0|once|errorinfo|binary-scan-tcl-value|1|4|70726f62655f6e68323032363130313262726f616431365f6e616d6573706163655f75707661725f6c696e6b3a3a736f7572636520782079220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6e616d6573706163655f75707661725f6c696e6b3a3a636f6e73756d65722220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d65737061636520
Oct 12 08:09:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_8 <RULE_INIT>: NHO|nh20261012broad1|namespace-upvar-link|tmm-procedure|lookup|1|0:0|once|errorinfo|binary-scan-tcl-value|2|4|6576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6e616d6573706163655f75707661725f6c696e6b3a3a636f6e73756d6572207b6e616d657370616365207570766172203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6e616d6573706163655f75707661722e2e2e220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20
Oct 12 08:09:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_8 <RULE_INIT>: NHO|nh20261012broad1|namespace-upvar-link|tmm-procedure|lookup|1|0:0|once|errorinfo|binary-scan-tcl-value|3|4|246e685f6f7065726174696f6e22
Oct 12 08:09:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_8 <RULE_INIT>: NHO|nh20261012broad1|namespace-upvar-link|tmm-procedure|runtime-cleanup|0|0:0|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:09:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_8 <RULE_INIT>: NHO|nh20261012broad1|import-source-renamed|tmm-procedure|setup|0|0:0|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:09:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_8 <RULE_INIT>: NHO|nh20261012broad1|import-source-renamed|tmm-procedure|lookup|0|0:0|once|result|binary-scan-tcl-value|0|1|4f524947494e414c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f696d706f72745f736f757263655f72656e616d65643a3a736f757263653a3a7070
Oct 12 08:09:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_8 <RULE_INIT>: NHO|nh20261012broad1|import-source-renamed|tmm-procedure|runtime-cleanup|0|0:0|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:09:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_8 <RULE_INIT>: NHO|nh20261012broad1|import-source-redefined|tmm-procedure|setup|0|0:0|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:09:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_8 <RULE_INIT>: NHO|nh20261012broad1|import-source-redefined|tmm-procedure|lookup|0|0:0|once|result|binary-scan-tcl-value|0|1|4e4557
Oct 12 08:09:43 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_8 <RULE_INIT>: NHO|nh20261012broad1|import-source-redefined|tmm-procedure|runtime-cleanup|0|0:0|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- namespace-upvar-link, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- namespace-upvar-link, lookup, completion 1, result, chunk 0 of 1:

```text-visible
bad option "upvar": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which⟦EOF⟧
```

- namespace-upvar-link, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- namespace-upvar-link, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- namespace-upvar-link, lookup, completion 1, errorinfo, chunk 0 of 4:

```text-visible
bad option "upvar": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which
    while executing
"namespace upvar ::__tcl_lsp_⟦EOF⟧
```

- namespace-upvar-link, lookup, completion 1, errorinfo, chunk 1 of 4:

```text-visible
probe_nh20261012broad16_namespace_upvar_link::source x y"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad16_namespace_upvar_link::consumer" script line 1)
    invoked from within
"namespace⟦SPACE⟧⟦EOF⟧
```

- namespace-upvar-link, lookup, completion 1, errorinfo, chunk 2 of 4:

```text-visible
eval ::__tcl_lsp_probe_nh20261012broad16_namespace_upvar_link::consumer {namespace upvar ::__tcl_lsp_probe_nh20261012broad16_namespace_upvar..."
    ("eval" body line 1)
    invoked from within
"eval⟦SPACE⟧⟦EOF⟧
```

- namespace-upvar-link, lookup, completion 1, errorinfo, chunk 3 of 4:

```text-visible
$nh_operation"⟦EOF⟧
```

- namespace-upvar-link, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- import-source-renamed, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- import-source-renamed, lookup, completion 0, result, chunk 0 of 1:

```text-visible
ORIGINAL ::__tcl_lsp_probe_nh20261012broad16_import_source_renamed::source::pp⟦EOF⟧
```

- import-source-renamed, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- import-source-redefined, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- import-source-redefined, lookup, completion 0, result, chunk 0 of 1:

```text-visible
NEW⟦EOF⟧
```

- import-source-redefined, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- import-source-redefined: intended operation reached.

- import-source-renamed: intended operation reached.

- namespace-upvar-link: intended operation reached.

### tmm-procedure, unit 0:1, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-procedure.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:09:43 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_8 <RULE_INIT>: NHO|nh20261012broad1|namespace-upvar-link|tmm-procedure|setup|0|0:1|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:09:43 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_8 <RULE_INIT>: NHO|nh20261012broad1|namespace-upvar-link|tmm-procedure|lookup|1|0:1|once|result|binary-scan-tcl-value|0|1|626164206f7074696f6e20227570766172223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f72207768696368
Oct 12 08:09:43 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_8 <RULE_INIT>: NHO|nh20261012broad1|namespace-upvar-link|tmm-procedure|lookup|1|0:1|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:09:43 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_8 <RULE_INIT>: NHO|nh20261012broad1|namespace-upvar-link|tmm-procedure|lookup|1|0:1|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:09:43 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_8 <RULE_INIT>: NHO|nh20261012broad1|namespace-upvar-link|tmm-procedure|lookup|1|0:1|once|errorinfo|binary-scan-tcl-value|0|4|626164206f7074696f6e20227570766172223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f722077686963680a202020207768696c6520657865637574696e670a226e616d657370616365207570766172203a3a5f5f74636c5f6c73705f
Oct 12 08:09:43 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_8 <RULE_INIT>: NHO|nh20261012broad1|namespace-upvar-link|tmm-procedure|lookup|1|0:1|once|errorinfo|binary-scan-tcl-value|1|4|70726f62655f6e68323032363130313262726f616431365f6e616d6573706163655f75707661725f6c696e6b3a3a736f7572636520782079220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6e616d6573706163655f75707661725f6c696e6b3a3a636f6e73756d65722220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d65737061636520
Oct 12 08:09:43 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_8 <RULE_INIT>: NHO|nh20261012broad1|namespace-upvar-link|tmm-procedure|lookup|1|0:1|once|errorinfo|binary-scan-tcl-value|2|4|6576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6e616d6573706163655f75707661725f6c696e6b3a3a636f6e73756d6572207b6e616d657370616365207570766172203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6e616d6573706163655f75707661722e2e2e220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20
Oct 12 08:09:43 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_8 <RULE_INIT>: NHO|nh20261012broad1|namespace-upvar-link|tmm-procedure|lookup|1|0:1|once|errorinfo|binary-scan-tcl-value|3|4|246e685f6f7065726174696f6e22
Oct 12 08:09:43 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_8 <RULE_INIT>: NHO|nh20261012broad1|namespace-upvar-link|tmm-procedure|runtime-cleanup|0|0:1|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:09:43 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_8 <RULE_INIT>: NHO|nh20261012broad1|import-source-renamed|tmm-procedure|setup|0|0:1|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:09:43 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_8 <RULE_INIT>: NHO|nh20261012broad1|import-source-renamed|tmm-procedure|lookup|0|0:1|once|result|binary-scan-tcl-value|0|1|4f524947494e414c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f696d706f72745f736f757263655f72656e616d65643a3a736f757263653a3a7070
Oct 12 08:09:43 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_8 <RULE_INIT>: NHO|nh20261012broad1|import-source-renamed|tmm-procedure|runtime-cleanup|0|0:1|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:09:43 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_8 <RULE_INIT>: NHO|nh20261012broad1|import-source-redefined|tmm-procedure|setup|0|0:1|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:09:43 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_8 <RULE_INIT>: NHO|nh20261012broad1|import-source-redefined|tmm-procedure|lookup|0|0:1|once|result|binary-scan-tcl-value|0|1|4e4557
Oct 12 08:09:43 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_8 <RULE_INIT>: NHO|nh20261012broad1|import-source-redefined|tmm-procedure|runtime-cleanup|0|0:1|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- namespace-upvar-link, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- namespace-upvar-link, lookup, completion 1, result, chunk 0 of 1:

```text-visible
bad option "upvar": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which⟦EOF⟧
```

- namespace-upvar-link, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- namespace-upvar-link, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- namespace-upvar-link, lookup, completion 1, errorinfo, chunk 0 of 4:

```text-visible
bad option "upvar": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which
    while executing
"namespace upvar ::__tcl_lsp_⟦EOF⟧
```

- namespace-upvar-link, lookup, completion 1, errorinfo, chunk 1 of 4:

```text-visible
probe_nh20261012broad16_namespace_upvar_link::source x y"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad16_namespace_upvar_link::consumer" script line 1)
    invoked from within
"namespace⟦SPACE⟧⟦EOF⟧
```

- namespace-upvar-link, lookup, completion 1, errorinfo, chunk 2 of 4:

```text-visible
eval ::__tcl_lsp_probe_nh20261012broad16_namespace_upvar_link::consumer {namespace upvar ::__tcl_lsp_probe_nh20261012broad16_namespace_upvar..."
    ("eval" body line 1)
    invoked from within
"eval⟦SPACE⟧⟦EOF⟧
```

- namespace-upvar-link, lookup, completion 1, errorinfo, chunk 3 of 4:

```text-visible
$nh_operation"⟦EOF⟧
```

- namespace-upvar-link, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- import-source-renamed, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- import-source-renamed, lookup, completion 0, result, chunk 0 of 1:

```text-visible
ORIGINAL ::__tcl_lsp_probe_nh20261012broad16_import_source_renamed::source::pp⟦EOF⟧
```

- import-source-renamed, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- import-source-redefined, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- import-source-redefined, lookup, completion 0, result, chunk 0 of 1:

```text-visible
NEW⟦EOF⟧
```

- import-source-redefined, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- import-source-redefined: intended operation reached.

- import-source-renamed: intended operation reached.

- namespace-upvar-link: intended operation reached.

### tmm-procedure, unit 0:2, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-procedure.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:09:43 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_8 <RULE_INIT>: NHO|nh20261012broad1|namespace-upvar-link|tmm-procedure|setup|0|0:2|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:09:43 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_8 <RULE_INIT>: NHO|nh20261012broad1|namespace-upvar-link|tmm-procedure|lookup|1|0:2|once|result|binary-scan-tcl-value|0|1|626164206f7074696f6e20227570766172223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f72207768696368
Oct 12 08:09:43 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_8 <RULE_INIT>: NHO|nh20261012broad1|namespace-upvar-link|tmm-procedure|lookup|1|0:2|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:09:43 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_8 <RULE_INIT>: NHO|nh20261012broad1|namespace-upvar-link|tmm-procedure|lookup|1|0:2|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:09:43 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_8 <RULE_INIT>: NHO|nh20261012broad1|namespace-upvar-link|tmm-procedure|lookup|1|0:2|once|errorinfo|binary-scan-tcl-value|0|4|626164206f7074696f6e20227570766172223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f722077686963680a202020207768696c6520657865637574696e670a226e616d657370616365207570766172203a3a5f5f74636c5f6c73705f
Oct 12 08:09:43 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_8 <RULE_INIT>: NHO|nh20261012broad1|namespace-upvar-link|tmm-procedure|lookup|1|0:2|once|errorinfo|binary-scan-tcl-value|1|4|70726f62655f6e68323032363130313262726f616431365f6e616d6573706163655f75707661725f6c696e6b3a3a736f7572636520782079220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6e616d6573706163655f75707661725f6c696e6b3a3a636f6e73756d65722220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d65737061636520
Oct 12 08:09:43 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_8 <RULE_INIT>: NHO|nh20261012broad1|namespace-upvar-link|tmm-procedure|lookup|1|0:2|once|errorinfo|binary-scan-tcl-value|2|4|6576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6e616d6573706163655f75707661725f6c696e6b3a3a636f6e73756d6572207b6e616d657370616365207570766172203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6e616d6573706163655f75707661722e2e2e220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20
Oct 12 08:09:43 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_8 <RULE_INIT>: NHO|nh20261012broad1|namespace-upvar-link|tmm-procedure|lookup|1|0:2|once|errorinfo|binary-scan-tcl-value|3|4|246e685f6f7065726174696f6e22
Oct 12 08:09:43 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_8 <RULE_INIT>: NHO|nh20261012broad1|namespace-upvar-link|tmm-procedure|runtime-cleanup|0|0:2|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:09:43 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_8 <RULE_INIT>: NHO|nh20261012broad1|import-source-renamed|tmm-procedure|setup|0|0:2|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:09:43 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_8 <RULE_INIT>: NHO|nh20261012broad1|import-source-renamed|tmm-procedure|lookup|0|0:2|once|result|binary-scan-tcl-value|0|1|4f524947494e414c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f696d706f72745f736f757263655f72656e616d65643a3a736f757263653a3a7070
Oct 12 08:09:43 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_8 <RULE_INIT>: NHO|nh20261012broad1|import-source-renamed|tmm-procedure|runtime-cleanup|0|0:2|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:09:43 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_8 <RULE_INIT>: NHO|nh20261012broad1|import-source-redefined|tmm-procedure|setup|0|0:2|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:09:43 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_8 <RULE_INIT>: NHO|nh20261012broad1|import-source-redefined|tmm-procedure|lookup|0|0:2|once|result|binary-scan-tcl-value|0|1|4e4557
Oct 12 08:09:43 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_8 <RULE_INIT>: NHO|nh20261012broad1|import-source-redefined|tmm-procedure|runtime-cleanup|0|0:2|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- namespace-upvar-link, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- namespace-upvar-link, lookup, completion 1, result, chunk 0 of 1:

```text-visible
bad option "upvar": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which⟦EOF⟧
```

- namespace-upvar-link, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- namespace-upvar-link, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- namespace-upvar-link, lookup, completion 1, errorinfo, chunk 0 of 4:

```text-visible
bad option "upvar": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which
    while executing
"namespace upvar ::__tcl_lsp_⟦EOF⟧
```

- namespace-upvar-link, lookup, completion 1, errorinfo, chunk 1 of 4:

```text-visible
probe_nh20261012broad16_namespace_upvar_link::source x y"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad16_namespace_upvar_link::consumer" script line 1)
    invoked from within
"namespace⟦SPACE⟧⟦EOF⟧
```

- namespace-upvar-link, lookup, completion 1, errorinfo, chunk 2 of 4:

```text-visible
eval ::__tcl_lsp_probe_nh20261012broad16_namespace_upvar_link::consumer {namespace upvar ::__tcl_lsp_probe_nh20261012broad16_namespace_upvar..."
    ("eval" body line 1)
    invoked from within
"eval⟦SPACE⟧⟦EOF⟧
```

- namespace-upvar-link, lookup, completion 1, errorinfo, chunk 3 of 4:

```text-visible
$nh_operation"⟦EOF⟧
```

- namespace-upvar-link, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- import-source-renamed, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- import-source-renamed, lookup, completion 0, result, chunk 0 of 1:

```text-visible
ORIGINAL ::__tcl_lsp_probe_nh20261012broad16_import_source_renamed::source::pp⟦EOF⟧
```

- import-source-renamed, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- import-source-redefined, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- import-source-redefined, lookup, completion 0, result, chunk 0 of 1:

```text-visible
NEW⟦EOF⟧
```

- import-source-redefined, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- import-source-redefined: intended operation reached.

- import-source-renamed: intended operation reached.

- namespace-upvar-link: intended operation reached.

### tmm-procedure, unit 0:3, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-procedure.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:09:43 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_8 <RULE_INIT>: NHO|nh20261012broad1|namespace-upvar-link|tmm-procedure|setup|0|0:3|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:09:43 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_8 <RULE_INIT>: NHO|nh20261012broad1|namespace-upvar-link|tmm-procedure|lookup|1|0:3|once|result|binary-scan-tcl-value|0|1|626164206f7074696f6e20227570766172223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f72207768696368
Oct 12 08:09:43 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_8 <RULE_INIT>: NHO|nh20261012broad1|namespace-upvar-link|tmm-procedure|lookup|1|0:3|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:09:43 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_8 <RULE_INIT>: NHO|nh20261012broad1|namespace-upvar-link|tmm-procedure|lookup|1|0:3|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:09:43 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_8 <RULE_INIT>: NHO|nh20261012broad1|namespace-upvar-link|tmm-procedure|lookup|1|0:3|once|errorinfo|binary-scan-tcl-value|0|4|626164206f7074696f6e20227570766172223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f722077686963680a202020207768696c6520657865637574696e670a226e616d657370616365207570766172203a3a5f5f74636c5f6c73705f
Oct 12 08:09:43 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_8 <RULE_INIT>: NHO|nh20261012broad1|namespace-upvar-link|tmm-procedure|lookup|1|0:3|once|errorinfo|binary-scan-tcl-value|1|4|70726f62655f6e68323032363130313262726f616431365f6e616d6573706163655f75707661725f6c696e6b3a3a736f7572636520782079220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6e616d6573706163655f75707661725f6c696e6b3a3a636f6e73756d65722220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d65737061636520
Oct 12 08:09:43 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_8 <RULE_INIT>: NHO|nh20261012broad1|namespace-upvar-link|tmm-procedure|lookup|1|0:3|once|errorinfo|binary-scan-tcl-value|2|4|6576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6e616d6573706163655f75707661725f6c696e6b3a3a636f6e73756d6572207b6e616d657370616365207570766172203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6e616d6573706163655f75707661722e2e2e220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20
Oct 12 08:09:43 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_8 <RULE_INIT>: NHO|nh20261012broad1|namespace-upvar-link|tmm-procedure|lookup|1|0:3|once|errorinfo|binary-scan-tcl-value|3|4|246e685f6f7065726174696f6e22
Oct 12 08:09:43 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_8 <RULE_INIT>: NHO|nh20261012broad1|namespace-upvar-link|tmm-procedure|runtime-cleanup|0|0:3|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:09:43 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_8 <RULE_INIT>: NHO|nh20261012broad1|import-source-renamed|tmm-procedure|setup|0|0:3|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:09:43 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_8 <RULE_INIT>: NHO|nh20261012broad1|import-source-renamed|tmm-procedure|lookup|0|0:3|once|result|binary-scan-tcl-value|0|1|4f524947494e414c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f696d706f72745f736f757263655f72656e616d65643a3a736f757263653a3a7070
Oct 12 08:09:43 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_8 <RULE_INIT>: NHO|nh20261012broad1|import-source-renamed|tmm-procedure|runtime-cleanup|0|0:3|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:09:43 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_8 <RULE_INIT>: NHO|nh20261012broad1|import-source-redefined|tmm-procedure|setup|0|0:3|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:09:43 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_8 <RULE_INIT>: NHO|nh20261012broad1|import-source-redefined|tmm-procedure|lookup|0|0:3|once|result|binary-scan-tcl-value|0|1|4e4557
Oct 12 08:09:43 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_8 <RULE_INIT>: NHO|nh20261012broad1|import-source-redefined|tmm-procedure|runtime-cleanup|0|0:3|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- namespace-upvar-link, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- namespace-upvar-link, lookup, completion 1, result, chunk 0 of 1:

```text-visible
bad option "upvar": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which⟦EOF⟧
```

- namespace-upvar-link, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- namespace-upvar-link, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- namespace-upvar-link, lookup, completion 1, errorinfo, chunk 0 of 4:

```text-visible
bad option "upvar": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which
    while executing
"namespace upvar ::__tcl_lsp_⟦EOF⟧
```

- namespace-upvar-link, lookup, completion 1, errorinfo, chunk 1 of 4:

```text-visible
probe_nh20261012broad16_namespace_upvar_link::source x y"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad16_namespace_upvar_link::consumer" script line 1)
    invoked from within
"namespace⟦SPACE⟧⟦EOF⟧
```

- namespace-upvar-link, lookup, completion 1, errorinfo, chunk 2 of 4:

```text-visible
eval ::__tcl_lsp_probe_nh20261012broad16_namespace_upvar_link::consumer {namespace upvar ::__tcl_lsp_probe_nh20261012broad16_namespace_upvar..."
    ("eval" body line 1)
    invoked from within
"eval⟦SPACE⟧⟦EOF⟧
```

- namespace-upvar-link, lookup, completion 1, errorinfo, chunk 3 of 4:

```text-visible
$nh_operation"⟦EOF⟧
```

- namespace-upvar-link, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- import-source-renamed, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- import-source-renamed, lookup, completion 0, result, chunk 0 of 1:

```text-visible
ORIGINAL ::__tcl_lsp_probe_nh20261012broad16_import_source_renamed::source::pp⟦EOF⟧
```

- import-source-renamed, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- import-source-redefined, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- import-source-redefined, lookup, completion 0, result, chunk 0 of 1:

```text-visible
NEW⟦EOF⟧
```

- import-source-redefined, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- import-source-redefined: intended operation reached.

- import-source-renamed: intended operation reached.

- namespace-upvar-link: intended operation reached.
