# BIG-IP Tcl log: nh20261012broad1, tmm-procedure-2

## Question

The following cases investigate Tcl behavior relevant to name resolution and diagnostics in the LSP, compiler and VM. Each observation is limited to its own case and recorded context.

- **duplicate-display-slots:** Colon runs, empty command/tail, lone colon names, namespace segment colons and duplicate display FQNs: which slots are actually reached?

- **empty-command-head:** Colon runs, empty command/tail, lone colon names, namespace segment colons and duplicate display FQNs: which slots are actually reached?

- **lone-colon-command:** Colon runs, empty command/tail, lone colon names, namespace segment colons and duplicate display FQNs: which slots are actually reached?

## Assumptions

- **duplicate-display-slots:** NH10 adapted names; invocation identity and displayed name are distinct observations.
- **empty-command-head:** Computed empty head; distinguish it from namespace-qualified empty tail.
- **lone-colon-command:** Lone-colon command in owned namespace.

Other assumptions were not separately recorded.

## Script

```tcl-visible
set nh_unit "[TMM::cmp_group]:[TMM::cmp_unit]"
set nh_invocation once
set nh_setup [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6475706c69636174655f646973706c61795f736c6f7473207b7d0a6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6475706c69636174655f646973706c61795f736c6f7473207b0a202020206e616d657370616365206576616c20613a207b70726f632070207b7d207b72657475726e2046495253545f534c4f547d7d0a202020206e616d657370616365206576616c2061207b70726f63203a70207b7d207b72657475726e205345434f4e445f534c4f547d7d0a7d0a]
set nh_operation [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6475706c69636174655f646973706c61795f736c6f7473207b0a202020206c697374205b6e616d657370616365206576616c20613a207b707d5d205b6e616d657370616365206576616c2061207b3a707d5d2020202020202020205b6e616d657370616365206576616c20613a207b6e616d65737061636520776869636820707d5d2020202020202020205b6e616d657370616365206576616c2061207b6e616d657370616365207768696368203a707d5d0a7d0a]
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
        log local0. "NHO|nh20261012broad1|duplicate-display-slots|tmm-procedure|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012broad1|duplicate-display-slots|tmm-procedure|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 666f7265616368206e685f6368696c64205b696e7465727020736c617665735d207b6966207b5b737472696e67206d61746368205f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6475706c69636174655f646973706c61795f736c6f74732a20246e685f6368696c645d7d207b696e746572702064656c65746520246e685f6368696c647d7d0a666f7265616368206e685f6e73205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6475706c69636174655f646973706c61795f736c6f74732a5d207b6e616d6573706163652064656c65746520246e685f6e737d0a666f7265616368206e685f636d64205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6475706c69636174655f646973706c61795f736c6f74732a5d207b72656e616d6520246e685f636d64207b7d7d0a666f7265616368206e685f766172205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6475706c69636174655f646973706c61795f736c6f74732a5d207b756e736574202d6e6f636f6d706c61696e20246e685f7661727d0a6c697374205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6475706c69636174655f646973706c61795f736c6f74732a5d205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6475706c69636174655f646973706c61795f736c6f74732a5d205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6475706c69636174655f646973706c61795f736c6f74732a5d]
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
        log local0. "NHO|nh20261012broad1|duplicate-display-slots|tmm-procedure|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
set nh_setup [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6c6f6e655f636f6c6f6e5f636f6d6d616e64207b7d0a6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6c6f6e655f636f6c6f6e5f636f6d6d616e64207b70726f63203a207b7d207b72657475726e204c4f4e455f434f4c4f4e7d7d0a]
set nh_operation [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6c6f6e655f636f6c6f6e5f636f6d6d616e64207b3a7d0a]
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
        log local0. "NHO|nh20261012broad1|lone-colon-command|tmm-procedure|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012broad1|lone-colon-command|tmm-procedure|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 666f7265616368206e685f6368696c64205b696e7465727020736c617665735d207b6966207b5b737472696e67206d61746368205f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6c6f6e655f636f6c6f6e5f636f6d6d616e642a20246e685f6368696c645d7d207b696e746572702064656c65746520246e685f6368696c647d7d0a666f7265616368206e685f6e73205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6c6f6e655f636f6c6f6e5f636f6d6d616e642a5d207b6e616d6573706163652064656c65746520246e685f6e737d0a666f7265616368206e685f636d64205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6c6f6e655f636f6c6f6e5f636f6d6d616e642a5d207b72656e616d6520246e685f636d64207b7d7d0a666f7265616368206e685f766172205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6c6f6e655f636f6c6f6e5f636f6d6d616e642a5d207b756e736574202d6e6f636f6d706c61696e20246e685f7661727d0a6c697374205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6c6f6e655f636f6c6f6e5f636f6d6d616e642a5d205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6c6f6e655f636f6c6f6e5f636f6d6d616e642a5d205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6c6f6e655f636f6c6f6e5f636f6d6d616e642a5d]
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
        log local0. "NHO|nh20261012broad1|lone-colon-command|tmm-procedure|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
set nh_setup [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f656d7074795f636f6d6d616e645f68656164207b7d0a6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f656d7074795f636f6d6d616e645f68656164207b70726f63207b7d207b7d207b72657475726e20454d5054595f484541447d3b207365742068656164207b7d7d0a]
set nh_operation [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f656d7074795f636f6d6d616e645f68656164207b24686561647d0a]
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
        log local0. "NHO|nh20261012broad1|empty-command-head|tmm-procedure|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012broad1|empty-command-head|tmm-procedure|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 666f7265616368206e685f6368696c64205b696e7465727020736c617665735d207b6966207b5b737472696e67206d61746368205f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f656d7074795f636f6d6d616e645f686561642a20246e685f6368696c645d7d207b696e746572702064656c65746520246e685f6368696c647d7d0a666f7265616368206e685f6e73205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f656d7074795f636f6d6d616e645f686561642a5d207b6e616d6573706163652064656c65746520246e685f6e737d0a666f7265616368206e685f636d64205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f656d7074795f636f6d6d616e645f686561642a5d207b72656e616d6520246e685f636d64207b7d7d0a666f7265616368206e685f766172205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f656d7074795f636f6d6d616e645f686561642a5d207b756e736574202d6e6f636f6d706c61696e20246e685f7661727d0a6c697374205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f656d7074795f636f6d6d616e645f686561642a5d205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f656d7074795f636f6d6d616e645f686561642a5d205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f656d7074795f636f6d6d616e645f686561642a5d]
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
        log local0. "NHO|nh20261012broad1|empty-command-head|tmm-procedure|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
Oct 12 08:08:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_2 <RULE_INIT>: NHO|nh20261012broad1|duplicate-display-slots|tmm-procedure|setup|1|0:0|once|result|binary-scan-tcl-value|0|1|63616e2774206372656174652070726f63656475726520223a702220696e206e6f6e2d676c6f62616c206e616d6573706163652077697468206e616d65207374617274696e67207769746820223a22
Oct 12 08:08:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_2 <RULE_INIT>: NHO|nh20261012broad1|duplicate-display-slots|tmm-procedure|setup|1|0:0|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:08:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_2 <RULE_INIT>: NHO|nh20261012broad1|duplicate-display-slots|tmm-procedure|setup|1|0:0|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:08:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_2 <RULE_INIT>: NHO|nh20261012broad1|duplicate-display-slots|tmm-procedure|setup|1|0:0|once|errorinfo|binary-scan-tcl-value|0|4|63616e2774206372656174652070726f63656475726520223a702220696e206e6f6e2d676c6f62616c206e616d6573706163652077697468206e616d65207374617274696e67207769746820223a220a202020207768696c6520657865637574696e670a2270726f63203a70207b7d207b72657475726e205345434f4e445f534c4f547d220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6475706c6963
Oct 12 08:08:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_2 <RULE_INIT>: NHO|nh20261012broad1|duplicate-display-slots|tmm-procedure|setup|1|0:0|once|errorinfo|binary-scan-tcl-value|1|4|6174655f646973706c61795f736c6f74733a3a612220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c2061207b70726f63203a70207b7d207b72657475726e205345434f4e445f534c4f547d7d220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6475706c69636174655f646973706c61795f736c6f7473222073
Oct 12 08:08:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_2 <RULE_INIT>: NHO|nh20261012broad1|duplicate-display-slots|tmm-procedure|setup|1|0:0|once|errorinfo|binary-scan-tcl-value|2|4|6372697074206c696e652033290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6475706c69636174655f646973706c61795f736c6f7473207b0a202020206e616d657370616365206576616c20613a207b70726f632070207b7d207b72657475726e2046495253545f534c4f547d7d0a202020206e616d657370616365206576616c202e2e2e220a202020202822
Oct 12 08:08:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_2 <RULE_INIT>: NHO|nh20261012broad1|duplicate-display-slots|tmm-procedure|setup|1|0:0|once|errorinfo|binary-scan-tcl-value|3|4|6576616c2220626f6479206c696e652032290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f736574757022
Oct 12 08:08:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_2 <RULE_INIT>: NHO|nh20261012broad1|duplicate-display-slots|tmm-procedure|runtime-cleanup|0|0:0|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:08:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_2 <RULE_INIT>: NHO|nh20261012broad1|lone-colon-command|tmm-procedure|setup|1|0:0|once|result|binary-scan-tcl-value|0|1|63616e2774206372656174652070726f63656475726520223a2220696e206e6f6e2d676c6f62616c206e616d6573706163652077697468206e616d65207374617274696e67207769746820223a22
Oct 12 08:08:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_2 <RULE_INIT>: NHO|nh20261012broad1|lone-colon-command|tmm-procedure|setup|1|0:0|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:08:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_2 <RULE_INIT>: NHO|nh20261012broad1|lone-colon-command|tmm-procedure|setup|1|0:0|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:08:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_2 <RULE_INIT>: NHO|nh20261012broad1|lone-colon-command|tmm-procedure|setup|1|0:0|once|errorinfo|binary-scan-tcl-value|0|3|63616e2774206372656174652070726f63656475726520223a2220696e206e6f6e2d676c6f62616c206e616d6573706163652077697468206e616d65207374617274696e67207769746820223a220a202020207768696c6520657865637574696e670a2270726f63203a207b7d207b72657475726e204c4f4e455f434f4c4f4e7d220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6c6f6e655f636f6c6f
Oct 12 08:08:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_2 <RULE_INIT>: NHO|nh20261012broad1|lone-colon-command|tmm-procedure|setup|1|0:0|once|errorinfo|binary-scan-tcl-value|1|3|6e5f636f6d6d616e642220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6c6f6e655f636f6c6f6e5f636f6d6d616e64207b70726f63203a207b7d207b72657475726e204c4f4e455f434f4c4f4e7d7d220a2020202028226576616c2220626f6479206c696e652032290a20202020696e766f6b65642066726f6d2077697468
Oct 12 08:08:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_2 <RULE_INIT>: NHO|nh20261012broad1|lone-colon-command|tmm-procedure|setup|1|0:0|once|errorinfo|binary-scan-tcl-value|2|3|696e0a226576616c20246e685f736574757022
Oct 12 08:08:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_2 <RULE_INIT>: NHO|nh20261012broad1|lone-colon-command|tmm-procedure|runtime-cleanup|0|0:0|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:08:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_2 <RULE_INIT>: NHO|nh20261012broad1|empty-command-head|tmm-procedure|setup|0|0:0|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:08:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_2 <RULE_INIT>: NHO|nh20261012broad1|empty-command-head|tmm-procedure|lookup|0|0:0|once|result|binary-scan-tcl-value|0|1|454d5054595f48454144
Oct 12 08:08:58 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_2 <RULE_INIT>: NHO|nh20261012broad1|empty-command-head|tmm-procedure|runtime-cleanup|0|0:0|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- duplicate-display-slots, setup, completion 1, result, chunk 0 of 1:

```text-visible
can't create procedure ":p" in non-global namespace with name starting with ":"⟦EOF⟧
```

- duplicate-display-slots, setup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- duplicate-display-slots, setup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- duplicate-display-slots, setup, completion 1, errorinfo, chunk 0 of 4:

```text-visible
can't create procedure ":p" in non-global namespace with name starting with ":"
    while executing
"proc :p {} {return SECOND_SLOT}"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad16_duplic⟦EOF⟧
```

- duplicate-display-slots, setup, completion 1, errorinfo, chunk 1 of 4:

```text-visible
ate_display_slots::a" script line 1)
    invoked from within
"namespace eval a {proc :p {} {return SECOND_SLOT}}"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad16_duplicate_display_slots" s⟦EOF⟧
```

- duplicate-display-slots, setup, completion 1, errorinfo, chunk 2 of 4:

```text-visible
cript line 3)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad16_duplicate_display_slots {
    namespace eval a: {proc p {} {return FIRST_SLOT}}
    namespace eval ..."
    ("⟦EOF⟧
```

- duplicate-display-slots, setup, completion 1, errorinfo, chunk 3 of 4:

```text-visible
eval" body line 2)
    invoked from within
"eval $nh_setup"⟦EOF⟧
```

- duplicate-display-slots, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- lone-colon-command, setup, completion 1, result, chunk 0 of 1:

```text-visible
can't create procedure ":" in non-global namespace with name starting with ":"⟦EOF⟧
```

- lone-colon-command, setup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- lone-colon-command, setup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- lone-colon-command, setup, completion 1, errorinfo, chunk 0 of 3:

```text-visible
can't create procedure ":" in non-global namespace with name starting with ":"
    while executing
"proc : {} {return LONE_COLON}"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad16_lone_colo⟦EOF⟧
```

- lone-colon-command, setup, completion 1, errorinfo, chunk 1 of 3:

```text-visible
n_command" script line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad16_lone_colon_command {proc : {} {return LONE_COLON}}"
    ("eval" body line 2)
    invoked from with⟦EOF⟧
```

- lone-colon-command, setup, completion 1, errorinfo, chunk 2 of 3:

```text-visible
in
"eval $nh_setup"⟦EOF⟧
```

- lone-colon-command, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- empty-command-head, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- empty-command-head, lookup, completion 0, result, chunk 0 of 1:

```text-visible
EMPTY_HEAD⟦EOF⟧
```

- empty-command-head, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- duplicate-display-slots: intended operation not reached.

- empty-command-head: intended operation reached.

- lone-colon-command: intended operation not reached.

### tmm-procedure, unit 0:1, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-procedure.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:08:58 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_2 <RULE_INIT>: NHO|nh20261012broad1|duplicate-display-slots|tmm-procedure|setup|1|0:1|once|result|binary-scan-tcl-value|0|1|63616e2774206372656174652070726f63656475726520223a702220696e206e6f6e2d676c6f62616c206e616d6573706163652077697468206e616d65207374617274696e67207769746820223a22
Oct 12 08:08:58 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_2 <RULE_INIT>: NHO|nh20261012broad1|duplicate-display-slots|tmm-procedure|setup|1|0:1|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:08:58 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_2 <RULE_INIT>: NHO|nh20261012broad1|duplicate-display-slots|tmm-procedure|setup|1|0:1|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:08:58 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_2 <RULE_INIT>: NHO|nh20261012broad1|duplicate-display-slots|tmm-procedure|setup|1|0:1|once|errorinfo|binary-scan-tcl-value|0|4|63616e2774206372656174652070726f63656475726520223a702220696e206e6f6e2d676c6f62616c206e616d6573706163652077697468206e616d65207374617274696e67207769746820223a220a202020207768696c6520657865637574696e670a2270726f63203a70207b7d207b72657475726e205345434f4e445f534c4f547d220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6475706c6963
Oct 12 08:08:58 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_2 <RULE_INIT>: NHO|nh20261012broad1|duplicate-display-slots|tmm-procedure|setup|1|0:1|once|errorinfo|binary-scan-tcl-value|1|4|6174655f646973706c61795f736c6f74733a3a612220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c2061207b70726f63203a70207b7d207b72657475726e205345434f4e445f534c4f547d7d220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6475706c69636174655f646973706c61795f736c6f7473222073
Oct 12 08:08:58 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_2 <RULE_INIT>: NHO|nh20261012broad1|duplicate-display-slots|tmm-procedure|setup|1|0:1|once|errorinfo|binary-scan-tcl-value|2|4|6372697074206c696e652033290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6475706c69636174655f646973706c61795f736c6f7473207b0a202020206e616d657370616365206576616c20613a207b70726f632070207b7d207b72657475726e2046495253545f534c4f547d7d0a202020206e616d657370616365206576616c202e2e2e220a202020202822
Oct 12 08:08:58 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_2 <RULE_INIT>: NHO|nh20261012broad1|duplicate-display-slots|tmm-procedure|setup|1|0:1|once|errorinfo|binary-scan-tcl-value|3|4|6576616c2220626f6479206c696e652032290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f736574757022
Oct 12 08:08:58 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_2 <RULE_INIT>: NHO|nh20261012broad1|duplicate-display-slots|tmm-procedure|runtime-cleanup|0|0:1|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:08:58 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_2 <RULE_INIT>: NHO|nh20261012broad1|lone-colon-command|tmm-procedure|setup|1|0:1|once|result|binary-scan-tcl-value|0|1|63616e2774206372656174652070726f63656475726520223a2220696e206e6f6e2d676c6f62616c206e616d6573706163652077697468206e616d65207374617274696e67207769746820223a22
Oct 12 08:08:58 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_2 <RULE_INIT>: NHO|nh20261012broad1|lone-colon-command|tmm-procedure|setup|1|0:1|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:08:58 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_2 <RULE_INIT>: NHO|nh20261012broad1|lone-colon-command|tmm-procedure|setup|1|0:1|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:08:58 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_2 <RULE_INIT>: NHO|nh20261012broad1|lone-colon-command|tmm-procedure|setup|1|0:1|once|errorinfo|binary-scan-tcl-value|0|3|63616e2774206372656174652070726f63656475726520223a2220696e206e6f6e2d676c6f62616c206e616d6573706163652077697468206e616d65207374617274696e67207769746820223a220a202020207768696c6520657865637574696e670a2270726f63203a207b7d207b72657475726e204c4f4e455f434f4c4f4e7d220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6c6f6e655f636f6c6f
Oct 12 08:08:58 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_2 <RULE_INIT>: NHO|nh20261012broad1|lone-colon-command|tmm-procedure|setup|1|0:1|once|errorinfo|binary-scan-tcl-value|1|3|6e5f636f6d6d616e642220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6c6f6e655f636f6c6f6e5f636f6d6d616e64207b70726f63203a207b7d207b72657475726e204c4f4e455f434f4c4f4e7d7d220a2020202028226576616c2220626f6479206c696e652032290a20202020696e766f6b65642066726f6d2077697468
Oct 12 08:08:58 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_2 <RULE_INIT>: NHO|nh20261012broad1|lone-colon-command|tmm-procedure|setup|1|0:1|once|errorinfo|binary-scan-tcl-value|2|3|696e0a226576616c20246e685f736574757022
Oct 12 08:08:58 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_2 <RULE_INIT>: NHO|nh20261012broad1|lone-colon-command|tmm-procedure|runtime-cleanup|0|0:1|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:08:58 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_2 <RULE_INIT>: NHO|nh20261012broad1|empty-command-head|tmm-procedure|setup|0|0:1|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:08:58 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_2 <RULE_INIT>: NHO|nh20261012broad1|empty-command-head|tmm-procedure|lookup|0|0:1|once|result|binary-scan-tcl-value|0|1|454d5054595f48454144
Oct 12 08:08:58 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_2 <RULE_INIT>: NHO|nh20261012broad1|empty-command-head|tmm-procedure|runtime-cleanup|0|0:1|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- duplicate-display-slots, setup, completion 1, result, chunk 0 of 1:

```text-visible
can't create procedure ":p" in non-global namespace with name starting with ":"⟦EOF⟧
```

- duplicate-display-slots, setup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- duplicate-display-slots, setup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- duplicate-display-slots, setup, completion 1, errorinfo, chunk 0 of 4:

```text-visible
can't create procedure ":p" in non-global namespace with name starting with ":"
    while executing
"proc :p {} {return SECOND_SLOT}"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad16_duplic⟦EOF⟧
```

- duplicate-display-slots, setup, completion 1, errorinfo, chunk 1 of 4:

```text-visible
ate_display_slots::a" script line 1)
    invoked from within
"namespace eval a {proc :p {} {return SECOND_SLOT}}"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad16_duplicate_display_slots" s⟦EOF⟧
```

- duplicate-display-slots, setup, completion 1, errorinfo, chunk 2 of 4:

```text-visible
cript line 3)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad16_duplicate_display_slots {
    namespace eval a: {proc p {} {return FIRST_SLOT}}
    namespace eval ..."
    ("⟦EOF⟧
```

- duplicate-display-slots, setup, completion 1, errorinfo, chunk 3 of 4:

```text-visible
eval" body line 2)
    invoked from within
"eval $nh_setup"⟦EOF⟧
```

- duplicate-display-slots, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- lone-colon-command, setup, completion 1, result, chunk 0 of 1:

```text-visible
can't create procedure ":" in non-global namespace with name starting with ":"⟦EOF⟧
```

- lone-colon-command, setup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- lone-colon-command, setup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- lone-colon-command, setup, completion 1, errorinfo, chunk 0 of 3:

```text-visible
can't create procedure ":" in non-global namespace with name starting with ":"
    while executing
"proc : {} {return LONE_COLON}"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad16_lone_colo⟦EOF⟧
```

- lone-colon-command, setup, completion 1, errorinfo, chunk 1 of 3:

```text-visible
n_command" script line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad16_lone_colon_command {proc : {} {return LONE_COLON}}"
    ("eval" body line 2)
    invoked from with⟦EOF⟧
```

- lone-colon-command, setup, completion 1, errorinfo, chunk 2 of 3:

```text-visible
in
"eval $nh_setup"⟦EOF⟧
```

- lone-colon-command, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- empty-command-head, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- empty-command-head, lookup, completion 0, result, chunk 0 of 1:

```text-visible
EMPTY_HEAD⟦EOF⟧
```

- empty-command-head, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- duplicate-display-slots: intended operation not reached.

- empty-command-head: intended operation reached.

- lone-colon-command: intended operation not reached.

### tmm-procedure, unit 0:2, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-procedure.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:08:58 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_2 <RULE_INIT>: NHO|nh20261012broad1|duplicate-display-slots|tmm-procedure|setup|1|0:2|once|result|binary-scan-tcl-value|0|1|63616e2774206372656174652070726f63656475726520223a702220696e206e6f6e2d676c6f62616c206e616d6573706163652077697468206e616d65207374617274696e67207769746820223a22
Oct 12 08:08:58 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_2 <RULE_INIT>: NHO|nh20261012broad1|duplicate-display-slots|tmm-procedure|setup|1|0:2|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:08:58 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_2 <RULE_INIT>: NHO|nh20261012broad1|duplicate-display-slots|tmm-procedure|setup|1|0:2|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:08:58 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_2 <RULE_INIT>: NHO|nh20261012broad1|duplicate-display-slots|tmm-procedure|setup|1|0:2|once|errorinfo|binary-scan-tcl-value|0|4|63616e2774206372656174652070726f63656475726520223a702220696e206e6f6e2d676c6f62616c206e616d6573706163652077697468206e616d65207374617274696e67207769746820223a220a202020207768696c6520657865637574696e670a2270726f63203a70207b7d207b72657475726e205345434f4e445f534c4f547d220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6475706c6963
Oct 12 08:08:58 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_2 <RULE_INIT>: NHO|nh20261012broad1|duplicate-display-slots|tmm-procedure|setup|1|0:2|once|errorinfo|binary-scan-tcl-value|1|4|6174655f646973706c61795f736c6f74733a3a612220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c2061207b70726f63203a70207b7d207b72657475726e205345434f4e445f534c4f547d7d220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6475706c69636174655f646973706c61795f736c6f7473222073
Oct 12 08:08:58 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_2 <RULE_INIT>: NHO|nh20261012broad1|duplicate-display-slots|tmm-procedure|setup|1|0:2|once|errorinfo|binary-scan-tcl-value|2|4|6372697074206c696e652033290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6475706c69636174655f646973706c61795f736c6f7473207b0a202020206e616d657370616365206576616c20613a207b70726f632070207b7d207b72657475726e2046495253545f534c4f547d7d0a202020206e616d657370616365206576616c202e2e2e220a202020202822
Oct 12 08:08:58 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_2 <RULE_INIT>: NHO|nh20261012broad1|duplicate-display-slots|tmm-procedure|setup|1|0:2|once|errorinfo|binary-scan-tcl-value|3|4|6576616c2220626f6479206c696e652032290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f736574757022
Oct 12 08:08:58 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_2 <RULE_INIT>: NHO|nh20261012broad1|duplicate-display-slots|tmm-procedure|runtime-cleanup|0|0:2|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:08:58 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_2 <RULE_INIT>: NHO|nh20261012broad1|lone-colon-command|tmm-procedure|setup|1|0:2|once|result|binary-scan-tcl-value|0|1|63616e2774206372656174652070726f63656475726520223a2220696e206e6f6e2d676c6f62616c206e616d6573706163652077697468206e616d65207374617274696e67207769746820223a22
Oct 12 08:08:58 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_2 <RULE_INIT>: NHO|nh20261012broad1|lone-colon-command|tmm-procedure|setup|1|0:2|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:08:58 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_2 <RULE_INIT>: NHO|nh20261012broad1|lone-colon-command|tmm-procedure|setup|1|0:2|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:08:58 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_2 <RULE_INIT>: NHO|nh20261012broad1|lone-colon-command|tmm-procedure|setup|1|0:2|once|errorinfo|binary-scan-tcl-value|0|3|63616e2774206372656174652070726f63656475726520223a2220696e206e6f6e2d676c6f62616c206e616d6573706163652077697468206e616d65207374617274696e67207769746820223a220a202020207768696c6520657865637574696e670a2270726f63203a207b7d207b72657475726e204c4f4e455f434f4c4f4e7d220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6c6f6e655f636f6c6f
Oct 12 08:08:58 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_2 <RULE_INIT>: NHO|nh20261012broad1|lone-colon-command|tmm-procedure|setup|1|0:2|once|errorinfo|binary-scan-tcl-value|1|3|6e5f636f6d6d616e642220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6c6f6e655f636f6c6f6e5f636f6d6d616e64207b70726f63203a207b7d207b72657475726e204c4f4e455f434f4c4f4e7d7d220a2020202028226576616c2220626f6479206c696e652032290a20202020696e766f6b65642066726f6d2077697468
Oct 12 08:08:58 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_2 <RULE_INIT>: NHO|nh20261012broad1|lone-colon-command|tmm-procedure|setup|1|0:2|once|errorinfo|binary-scan-tcl-value|2|3|696e0a226576616c20246e685f736574757022
Oct 12 08:08:58 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_2 <RULE_INIT>: NHO|nh20261012broad1|lone-colon-command|tmm-procedure|runtime-cleanup|0|0:2|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:08:58 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_2 <RULE_INIT>: NHO|nh20261012broad1|empty-command-head|tmm-procedure|setup|0|0:2|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:08:58 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_2 <RULE_INIT>: NHO|nh20261012broad1|empty-command-head|tmm-procedure|lookup|0|0:2|once|result|binary-scan-tcl-value|0|1|454d5054595f48454144
Oct 12 08:08:58 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_2 <RULE_INIT>: NHO|nh20261012broad1|empty-command-head|tmm-procedure|runtime-cleanup|0|0:2|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- duplicate-display-slots, setup, completion 1, result, chunk 0 of 1:

```text-visible
can't create procedure ":p" in non-global namespace with name starting with ":"⟦EOF⟧
```

- duplicate-display-slots, setup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- duplicate-display-slots, setup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- duplicate-display-slots, setup, completion 1, errorinfo, chunk 0 of 4:

```text-visible
can't create procedure ":p" in non-global namespace with name starting with ":"
    while executing
"proc :p {} {return SECOND_SLOT}"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad16_duplic⟦EOF⟧
```

- duplicate-display-slots, setup, completion 1, errorinfo, chunk 1 of 4:

```text-visible
ate_display_slots::a" script line 1)
    invoked from within
"namespace eval a {proc :p {} {return SECOND_SLOT}}"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad16_duplicate_display_slots" s⟦EOF⟧
```

- duplicate-display-slots, setup, completion 1, errorinfo, chunk 2 of 4:

```text-visible
cript line 3)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad16_duplicate_display_slots {
    namespace eval a: {proc p {} {return FIRST_SLOT}}
    namespace eval ..."
    ("⟦EOF⟧
```

- duplicate-display-slots, setup, completion 1, errorinfo, chunk 3 of 4:

```text-visible
eval" body line 2)
    invoked from within
"eval $nh_setup"⟦EOF⟧
```

- duplicate-display-slots, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- lone-colon-command, setup, completion 1, result, chunk 0 of 1:

```text-visible
can't create procedure ":" in non-global namespace with name starting with ":"⟦EOF⟧
```

- lone-colon-command, setup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- lone-colon-command, setup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- lone-colon-command, setup, completion 1, errorinfo, chunk 0 of 3:

```text-visible
can't create procedure ":" in non-global namespace with name starting with ":"
    while executing
"proc : {} {return LONE_COLON}"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad16_lone_colo⟦EOF⟧
```

- lone-colon-command, setup, completion 1, errorinfo, chunk 1 of 3:

```text-visible
n_command" script line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad16_lone_colon_command {proc : {} {return LONE_COLON}}"
    ("eval" body line 2)
    invoked from with⟦EOF⟧
```

- lone-colon-command, setup, completion 1, errorinfo, chunk 2 of 3:

```text-visible
in
"eval $nh_setup"⟦EOF⟧
```

- lone-colon-command, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- empty-command-head, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- empty-command-head, lookup, completion 0, result, chunk 0 of 1:

```text-visible
EMPTY_HEAD⟦EOF⟧
```

- empty-command-head, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- duplicate-display-slots: intended operation not reached.

- empty-command-head: intended operation reached.

- lone-colon-command: intended operation not reached.

### tmm-procedure, unit 0:3, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-procedure.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:08:58 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_2 <RULE_INIT>: NHO|nh20261012broad1|duplicate-display-slots|tmm-procedure|setup|1|0:3|once|result|binary-scan-tcl-value|0|1|63616e2774206372656174652070726f63656475726520223a702220696e206e6f6e2d676c6f62616c206e616d6573706163652077697468206e616d65207374617274696e67207769746820223a22
Oct 12 08:08:58 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_2 <RULE_INIT>: NHO|nh20261012broad1|duplicate-display-slots|tmm-procedure|setup|1|0:3|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:08:58 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_2 <RULE_INIT>: NHO|nh20261012broad1|duplicate-display-slots|tmm-procedure|setup|1|0:3|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:08:58 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_2 <RULE_INIT>: NHO|nh20261012broad1|duplicate-display-slots|tmm-procedure|setup|1|0:3|once|errorinfo|binary-scan-tcl-value|0|4|63616e2774206372656174652070726f63656475726520223a702220696e206e6f6e2d676c6f62616c206e616d6573706163652077697468206e616d65207374617274696e67207769746820223a220a202020207768696c6520657865637574696e670a2270726f63203a70207b7d207b72657475726e205345434f4e445f534c4f547d220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6475706c6963
Oct 12 08:08:58 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_2 <RULE_INIT>: NHO|nh20261012broad1|duplicate-display-slots|tmm-procedure|setup|1|0:3|once|errorinfo|binary-scan-tcl-value|1|4|6174655f646973706c61795f736c6f74733a3a612220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c2061207b70726f63203a70207b7d207b72657475726e205345434f4e445f534c4f547d7d220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6475706c69636174655f646973706c61795f736c6f7473222073
Oct 12 08:08:58 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_2 <RULE_INIT>: NHO|nh20261012broad1|duplicate-display-slots|tmm-procedure|setup|1|0:3|once|errorinfo|binary-scan-tcl-value|2|4|6372697074206c696e652033290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6475706c69636174655f646973706c61795f736c6f7473207b0a202020206e616d657370616365206576616c20613a207b70726f632070207b7d207b72657475726e2046495253545f534c4f547d7d0a202020206e616d657370616365206576616c202e2e2e220a202020202822
Oct 12 08:08:58 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_2 <RULE_INIT>: NHO|nh20261012broad1|duplicate-display-slots|tmm-procedure|setup|1|0:3|once|errorinfo|binary-scan-tcl-value|3|4|6576616c2220626f6479206c696e652032290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f736574757022
Oct 12 08:08:58 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_2 <RULE_INIT>: NHO|nh20261012broad1|duplicate-display-slots|tmm-procedure|runtime-cleanup|0|0:3|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:08:58 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_2 <RULE_INIT>: NHO|nh20261012broad1|lone-colon-command|tmm-procedure|setup|1|0:3|once|result|binary-scan-tcl-value|0|1|63616e2774206372656174652070726f63656475726520223a2220696e206e6f6e2d676c6f62616c206e616d6573706163652077697468206e616d65207374617274696e67207769746820223a22
Oct 12 08:08:58 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_2 <RULE_INIT>: NHO|nh20261012broad1|lone-colon-command|tmm-procedure|setup|1|0:3|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:08:58 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_2 <RULE_INIT>: NHO|nh20261012broad1|lone-colon-command|tmm-procedure|setup|1|0:3|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:08:58 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_2 <RULE_INIT>: NHO|nh20261012broad1|lone-colon-command|tmm-procedure|setup|1|0:3|once|errorinfo|binary-scan-tcl-value|0|3|63616e2774206372656174652070726f63656475726520223a2220696e206e6f6e2d676c6f62616c206e616d6573706163652077697468206e616d65207374617274696e67207769746820223a220a202020207768696c6520657865637574696e670a2270726f63203a207b7d207b72657475726e204c4f4e455f434f4c4f4e7d220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6c6f6e655f636f6c6f
Oct 12 08:08:58 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_2 <RULE_INIT>: NHO|nh20261012broad1|lone-colon-command|tmm-procedure|setup|1|0:3|once|errorinfo|binary-scan-tcl-value|1|3|6e5f636f6d6d616e642220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6c6f6e655f636f6c6f6e5f636f6d6d616e64207b70726f63203a207b7d207b72657475726e204c4f4e455f434f4c4f4e7d7d220a2020202028226576616c2220626f6479206c696e652032290a20202020696e766f6b65642066726f6d2077697468
Oct 12 08:08:58 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_2 <RULE_INIT>: NHO|nh20261012broad1|lone-colon-command|tmm-procedure|setup|1|0:3|once|errorinfo|binary-scan-tcl-value|2|3|696e0a226576616c20246e685f736574757022
Oct 12 08:08:58 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_2 <RULE_INIT>: NHO|nh20261012broad1|lone-colon-command|tmm-procedure|runtime-cleanup|0|0:3|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:08:58 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_2 <RULE_INIT>: NHO|nh20261012broad1|empty-command-head|tmm-procedure|setup|0|0:3|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:08:58 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_2 <RULE_INIT>: NHO|nh20261012broad1|empty-command-head|tmm-procedure|lookup|0|0:3|once|result|binary-scan-tcl-value|0|1|454d5054595f48454144
Oct 12 08:08:58 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_2 <RULE_INIT>: NHO|nh20261012broad1|empty-command-head|tmm-procedure|runtime-cleanup|0|0:3|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- duplicate-display-slots, setup, completion 1, result, chunk 0 of 1:

```text-visible
can't create procedure ":p" in non-global namespace with name starting with ":"⟦EOF⟧
```

- duplicate-display-slots, setup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- duplicate-display-slots, setup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- duplicate-display-slots, setup, completion 1, errorinfo, chunk 0 of 4:

```text-visible
can't create procedure ":p" in non-global namespace with name starting with ":"
    while executing
"proc :p {} {return SECOND_SLOT}"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad16_duplic⟦EOF⟧
```

- duplicate-display-slots, setup, completion 1, errorinfo, chunk 1 of 4:

```text-visible
ate_display_slots::a" script line 1)
    invoked from within
"namespace eval a {proc :p {} {return SECOND_SLOT}}"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad16_duplicate_display_slots" s⟦EOF⟧
```

- duplicate-display-slots, setup, completion 1, errorinfo, chunk 2 of 4:

```text-visible
cript line 3)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad16_duplicate_display_slots {
    namespace eval a: {proc p {} {return FIRST_SLOT}}
    namespace eval ..."
    ("⟦EOF⟧
```

- duplicate-display-slots, setup, completion 1, errorinfo, chunk 3 of 4:

```text-visible
eval" body line 2)
    invoked from within
"eval $nh_setup"⟦EOF⟧
```

- duplicate-display-slots, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- lone-colon-command, setup, completion 1, result, chunk 0 of 1:

```text-visible
can't create procedure ":" in non-global namespace with name starting with ":"⟦EOF⟧
```

- lone-colon-command, setup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- lone-colon-command, setup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- lone-colon-command, setup, completion 1, errorinfo, chunk 0 of 3:

```text-visible
can't create procedure ":" in non-global namespace with name starting with ":"
    while executing
"proc : {} {return LONE_COLON}"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad16_lone_colo⟦EOF⟧
```

- lone-colon-command, setup, completion 1, errorinfo, chunk 1 of 3:

```text-visible
n_command" script line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad16_lone_colon_command {proc : {} {return LONE_COLON}}"
    ("eval" body line 2)
    invoked from with⟦EOF⟧
```

- lone-colon-command, setup, completion 1, errorinfo, chunk 2 of 3:

```text-visible
in
"eval $nh_setup"⟦EOF⟧
```

- lone-colon-command, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- empty-command-head, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- empty-command-head, lookup, completion 0, result, chunk 0 of 1:

```text-visible
EMPTY_HEAD⟦EOF⟧
```

- empty-command-head, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- duplicate-display-slots: intended operation not reached.

- empty-command-head: intended operation reached.

- lone-colon-command: intended operation not reached.
