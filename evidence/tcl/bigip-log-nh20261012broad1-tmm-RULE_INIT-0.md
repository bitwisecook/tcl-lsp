# BIG-IP Tcl log: nh20261012broad1, tmm-RULE_INIT-0

## Question

The following cases investigate Tcl behavior relevant to name resolution and diagnostics in the LSP, compiler and VM. Each observation is limited to its own case and recorded context.

- **current-before-global:** Bare and relative-qualified command lookup: current, path, global order; qualifier namespace exists without target; no ancestor walk; absolute bypass. Does path exist?

- **namespace-path-relative:** Bare and relative-qualified command lookup: current, path, global order; qualifier namespace exists without target; no ancestor walk; absolute bypass. Does path exist?

- **relative-qualifier-missing-target:** Bare and relative-qualified command lookup: current, path, global order; qualifier namespace exists without target; no ancestor walk; absolute bypass. Does path exist?

## Assumptions

- **current-before-global:** NH01 adapted names; clean global command ::__tcl_lsp_probe_nh20261012broad15_current_before_global_p as well as ::__tcl_lsp_probe_nh20261012broad15_current_before_global.
- **namespace-path-relative:** NH03 adapted names; clean ::__tcl_lsp_probe_nh20261012broad15_namespace_path_relative_path too; path support is a separate control.
- **relative-qualifier-missing-target:** NH02 adapted names; clean ::__tcl_lsp_probe_nh20261012broad15_relative_qualifier_missing_target_target too; target qualifier exists locally.

Other assumptions were not separately recorded.

## Script

```tcl-visible
set nh_unit "[TMM::cmp_group]:[TMM::cmp_unit]"
set nh_invocation once
set nh_setup [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f63757272656e745f6265666f72655f676c6f62616c207b7d0a70726f63203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f63757272656e745f6265666f72655f676c6f62616c5f70207b7d207b72657475726e20474c4f42414c7d0a6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f63757272656e745f6265666f72655f676c6f62616c207b70726f63205f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f63757272656e745f6265666f72655f676c6f62616c5f70207b7d207b72657475726e2043555252454e547d7d0a]
set nh_operation [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f63757272656e745f6265666f72655f676c6f62616c207b5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f63757272656e745f6265666f72655f676c6f62616c5f707d0a]
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
        log local0. "NHO|nh20261012broad1|current-before-global|tmm-RULE_INIT|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012broad1|current-before-global|tmm-RULE_INIT|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 666f7265616368206e685f6368696c64205b696e7465727020736c617665735d207b6966207b5b737472696e67206d61746368205f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f63757272656e745f6265666f72655f676c6f62616c2a20246e685f6368696c645d7d207b696e746572702064656c65746520246e685f6368696c647d7d0a666f7265616368206e685f6e73205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f63757272656e745f6265666f72655f676c6f62616c2a5d207b6e616d6573706163652064656c65746520246e685f6e737d0a666f7265616368206e685f636d64205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f63757272656e745f6265666f72655f676c6f62616c2a5d207b72656e616d6520246e685f636d64207b7d7d0a666f7265616368206e685f766172205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f63757272656e745f6265666f72655f676c6f62616c2a5d207b756e736574202d6e6f636f6d706c61696e20246e685f7661727d0a6c697374205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f63757272656e745f6265666f72655f676c6f62616c2a5d205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f63757272656e745f6265666f72655f676c6f62616c2a5d205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f63757272656e745f6265666f72655f676c6f62616c2a5d]
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
        log local0. "NHO|nh20261012broad1|current-before-global|tmm-RULE_INIT|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
set nh_setup [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f72656c61746976655f7175616c69666965725f6d697373696e675f746172676574207b7d0a6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f72656c61746976655f7175616c69666965725f6d697373696e675f7461726765745f746172676574207b70726f632070207b7d207b72657475726e20474c4f42414c5f5441524745547d7d0a6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f72656c61746976655f7175616c69666965725f6d697373696e675f746172676574207b6e616d657370616365206576616c205f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f72656c61746976655f7175616c69666965725f6d697373696e675f7461726765745f746172676574207b7d7d0a]
set nh_operation [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f72656c61746976655f7175616c69666965725f6d697373696e675f746172676574207b5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f72656c61746976655f7175616c69666965725f6d697373696e675f7461726765745f7461726765743a3a707d0a]
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
        log local0. "NHO|nh20261012broad1|relative-qualifier-missing-target|tmm-RULE_INIT|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012broad1|relative-qualifier-missing-target|tmm-RULE_INIT|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 666f7265616368206e685f6368696c64205b696e7465727020736c617665735d207b6966207b5b737472696e67206d61746368205f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f72656c61746976655f7175616c69666965725f6d697373696e675f7461726765742a20246e685f6368696c645d7d207b696e746572702064656c65746520246e685f6368696c647d7d0a666f7265616368206e685f6e73205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f72656c61746976655f7175616c69666965725f6d697373696e675f7461726765742a5d207b6e616d6573706163652064656c65746520246e685f6e737d0a666f7265616368206e685f636d64205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f72656c61746976655f7175616c69666965725f6d697373696e675f7461726765742a5d207b72656e616d6520246e685f636d64207b7d7d0a666f7265616368206e685f766172205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f72656c61746976655f7175616c69666965725f6d697373696e675f7461726765742a5d207b756e736574202d6e6f636f6d706c61696e20246e685f7661727d0a6c697374205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f72656c61746976655f7175616c69666965725f6d697373696e675f7461726765742a5d205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f72656c61746976655f7175616c69666965725f6d697373696e675f7461726765742a5d205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f72656c61746976655f7175616c69666965725f6d697373696e675f7461726765742a5d]
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
        log local0. "NHO|nh20261012broad1|relative-qualifier-missing-target|tmm-RULE_INIT|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
set nh_setup [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6e616d6573706163655f706174685f72656c6174697665207b7d0a6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6e616d6573706163655f706174685f72656c61746976655f706174683a3a696e6e6572207b70726f632070207b7d207b72657475726e20504154485f5441524745547d7d0a6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6e616d6573706163655f706174685f72656c6174697665207b6e616d6573706163652070617468203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6e616d6573706163655f706174685f72656c61746976655f706174687d0a]
set nh_operation [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6e616d6573706163655f706174685f72656c6174697665207b696e6e65723a3a707d0a]
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
        log local0. "NHO|nh20261012broad1|namespace-path-relative|tmm-RULE_INIT|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012broad1|namespace-path-relative|tmm-RULE_INIT|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 666f7265616368206e685f6368696c64205b696e7465727020736c617665735d207b6966207b5b737472696e67206d61746368205f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6e616d6573706163655f706174685f72656c61746976652a20246e685f6368696c645d7d207b696e746572702064656c65746520246e685f6368696c647d7d0a666f7265616368206e685f6e73205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6e616d6573706163655f706174685f72656c61746976652a5d207b6e616d6573706163652064656c65746520246e685f6e737d0a666f7265616368206e685f636d64205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6e616d6573706163655f706174685f72656c61746976652a5d207b72656e616d6520246e685f636d64207b7d7d0a666f7265616368206e685f766172205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6e616d6573706163655f706174685f72656c61746976652a5d207b756e736574202d6e6f636f6d706c61696e20246e685f7661727d0a6c697374205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6e616d6573706163655f706174685f72656c61746976652a5d205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6e616d6573706163655f706174685f72656c61746976652a5d205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6e616d6573706163655f706174685f72656c61746976652a5d]
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
        log local0. "NHO|nh20261012broad1|namespace-path-relative|tmm-RULE_INIT|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
Oct 12 08:06:04 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_0 <RULE_INIT>: NHO|nh20261012broad1|current-before-global|tmm-RULE_INIT|setup|0|0:0|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:06:04 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_0 <RULE_INIT>: NHO|nh20261012broad1|current-before-global|tmm-RULE_INIT|lookup|0|0:0|once|result|binary-scan-tcl-value|0|1|43555252454e54
Oct 12 08:06:04 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_0 <RULE_INIT>: NHO|nh20261012broad1|current-before-global|tmm-RULE_INIT|runtime-cleanup|0|0:0|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:06:04 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_0 <RULE_INIT>: NHO|nh20261012broad1|relative-qualifier-missing-target|tmm-RULE_INIT|setup|0|0:0|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:06:04 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_0 <RULE_INIT>: NHO|nh20261012broad1|relative-qualifier-missing-target|tmm-RULE_INIT|lookup|0|0:0|once|result|binary-scan-tcl-value|0|1|474c4f42414c5f544152474554
Oct 12 08:06:04 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_0 <RULE_INIT>: NHO|nh20261012broad1|relative-qualifier-missing-target|tmm-RULE_INIT|runtime-cleanup|0|0:0|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:06:04 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_0 <RULE_INIT>: NHO|nh20261012broad1|namespace-path-relative|tmm-RULE_INIT|setup|1|0:0|once|result|binary-scan-tcl-value|0|1|626164206f7074696f6e202270617468223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f72207768696368
Oct 12 08:06:04 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_0 <RULE_INIT>: NHO|nh20261012broad1|namespace-path-relative|tmm-RULE_INIT|setup|1|0:0|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:06:04 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_0 <RULE_INIT>: NHO|nh20261012broad1|namespace-path-relative|tmm-RULE_INIT|setup|1|0:0|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:06:04 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_0 <RULE_INIT>: NHO|nh20261012broad1|namespace-path-relative|tmm-RULE_INIT|setup|1|0:0|once|errorinfo|binary-scan-tcl-value|0|3|626164206f7074696f6e202270617468223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f722077686963680a202020207768696c6520657865637574696e670a226e616d6573706163652070617468203a3a5f5f74636c5f6c73705f7072
Oct 12 08:06:04 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_0 <RULE_INIT>: NHO|nh20261012broad1|namespace-path-relative|tmm-RULE_INIT|setup|1|0:0|once|errorinfo|binary-scan-tcl-value|1|3|6f62655f6e68323032363130313262726f616431355f6e616d6573706163655f706174685f72656c61746976655f70617468220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6e616d6573706163655f706174685f72656c61746976652220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f5f74636c5f
Oct 12 08:06:04 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_0 <RULE_INIT>: NHO|nh20261012broad1|namespace-path-relative|tmm-RULE_INIT|setup|1|0:0|once|errorinfo|binary-scan-tcl-value|2|3|6c73705f70726f62655f6e68323032363130313262726f616431355f6e616d6573706163655f706174685f72656c6174697665207b6e616d6573706163652070617468203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6e616d6573706163655f706174685f72656c61746976652e2e2e220a2020202028226576616c2220626f6479206c696e652033290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f736574757022
Oct 12 08:06:04 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_0 <RULE_INIT>: NHO|nh20261012broad1|namespace-path-relative|tmm-RULE_INIT|runtime-cleanup|0|0:0|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- current-before-global, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- current-before-global, lookup, completion 0, result, chunk 0 of 1:

```text-visible
CURRENT⟦EOF⟧
```

- current-before-global, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- relative-qualifier-missing-target, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- relative-qualifier-missing-target, lookup, completion 0, result, chunk 0 of 1:

```text-visible
GLOBAL_TARGET⟦EOF⟧
```

- relative-qualifier-missing-target, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- namespace-path-relative, setup, completion 1, result, chunk 0 of 1:

```text-visible
bad option "path": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which⟦EOF⟧
```

- namespace-path-relative, setup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- namespace-path-relative, setup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- namespace-path-relative, setup, completion 1, errorinfo, chunk 0 of 3:

```text-visible
bad option "path": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which
    while executing
"namespace path ::__tcl_lsp_pr⟦EOF⟧
```

- namespace-path-relative, setup, completion 1, errorinfo, chunk 1 of 3:

```text-visible
obe_nh20261012broad15_namespace_path_relative_path"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad15_namespace_path_relative" script line 1)
    invoked from within
"namespace eval ::__tcl_⟦EOF⟧
```

- namespace-path-relative, setup, completion 1, errorinfo, chunk 2 of 3:

```text-visible
lsp_probe_nh20261012broad15_namespace_path_relative {namespace path ::__tcl_lsp_probe_nh20261012broad15_namespace_path_relative..."
    ("eval" body line 3)
    invoked from within
"eval $nh_setup"⟦EOF⟧
```

- namespace-path-relative, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- current-before-global: intended operation reached.

- namespace-path-relative: intended operation not reached.

- relative-qualifier-missing-target: intended operation reached.

### tmm-RULE_INIT, unit 0:1, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:06:04 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_0 <RULE_INIT>: NHO|nh20261012broad1|current-before-global|tmm-RULE_INIT|setup|0|0:1|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:06:04 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_0 <RULE_INIT>: NHO|nh20261012broad1|current-before-global|tmm-RULE_INIT|lookup|0|0:1|once|result|binary-scan-tcl-value|0|1|43555252454e54
Oct 12 08:06:04 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_0 <RULE_INIT>: NHO|nh20261012broad1|current-before-global|tmm-RULE_INIT|runtime-cleanup|0|0:1|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:06:04 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_0 <RULE_INIT>: NHO|nh20261012broad1|relative-qualifier-missing-target|tmm-RULE_INIT|setup|0|0:1|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:06:04 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_0 <RULE_INIT>: NHO|nh20261012broad1|relative-qualifier-missing-target|tmm-RULE_INIT|lookup|0|0:1|once|result|binary-scan-tcl-value|0|1|474c4f42414c5f544152474554
Oct 12 08:06:04 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_0 <RULE_INIT>: NHO|nh20261012broad1|relative-qualifier-missing-target|tmm-RULE_INIT|runtime-cleanup|0|0:1|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:06:04 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_0 <RULE_INIT>: NHO|nh20261012broad1|namespace-path-relative|tmm-RULE_INIT|setup|1|0:1|once|result|binary-scan-tcl-value|0|1|626164206f7074696f6e202270617468223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f72207768696368
Oct 12 08:06:04 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_0 <RULE_INIT>: NHO|nh20261012broad1|namespace-path-relative|tmm-RULE_INIT|setup|1|0:1|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:06:04 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_0 <RULE_INIT>: NHO|nh20261012broad1|namespace-path-relative|tmm-RULE_INIT|setup|1|0:1|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:06:04 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_0 <RULE_INIT>: NHO|nh20261012broad1|namespace-path-relative|tmm-RULE_INIT|setup|1|0:1|once|errorinfo|binary-scan-tcl-value|0|3|626164206f7074696f6e202270617468223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f722077686963680a202020207768696c6520657865637574696e670a226e616d6573706163652070617468203a3a5f5f74636c5f6c73705f7072
Oct 12 08:06:04 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_0 <RULE_INIT>: NHO|nh20261012broad1|namespace-path-relative|tmm-RULE_INIT|setup|1|0:1|once|errorinfo|binary-scan-tcl-value|1|3|6f62655f6e68323032363130313262726f616431355f6e616d6573706163655f706174685f72656c61746976655f70617468220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6e616d6573706163655f706174685f72656c61746976652220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f5f74636c5f
Oct 12 08:06:04 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_0 <RULE_INIT>: NHO|nh20261012broad1|namespace-path-relative|tmm-RULE_INIT|setup|1|0:1|once|errorinfo|binary-scan-tcl-value|2|3|6c73705f70726f62655f6e68323032363130313262726f616431355f6e616d6573706163655f706174685f72656c6174697665207b6e616d6573706163652070617468203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6e616d6573706163655f706174685f72656c61746976652e2e2e220a2020202028226576616c2220626f6479206c696e652033290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f736574757022
Oct 12 08:06:04 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_0 <RULE_INIT>: NHO|nh20261012broad1|namespace-path-relative|tmm-RULE_INIT|runtime-cleanup|0|0:1|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- current-before-global, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- current-before-global, lookup, completion 0, result, chunk 0 of 1:

```text-visible
CURRENT⟦EOF⟧
```

- current-before-global, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- relative-qualifier-missing-target, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- relative-qualifier-missing-target, lookup, completion 0, result, chunk 0 of 1:

```text-visible
GLOBAL_TARGET⟦EOF⟧
```

- relative-qualifier-missing-target, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- namespace-path-relative, setup, completion 1, result, chunk 0 of 1:

```text-visible
bad option "path": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which⟦EOF⟧
```

- namespace-path-relative, setup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- namespace-path-relative, setup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- namespace-path-relative, setup, completion 1, errorinfo, chunk 0 of 3:

```text-visible
bad option "path": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which
    while executing
"namespace path ::__tcl_lsp_pr⟦EOF⟧
```

- namespace-path-relative, setup, completion 1, errorinfo, chunk 1 of 3:

```text-visible
obe_nh20261012broad15_namespace_path_relative_path"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad15_namespace_path_relative" script line 1)
    invoked from within
"namespace eval ::__tcl_⟦EOF⟧
```

- namespace-path-relative, setup, completion 1, errorinfo, chunk 2 of 3:

```text-visible
lsp_probe_nh20261012broad15_namespace_path_relative {namespace path ::__tcl_lsp_probe_nh20261012broad15_namespace_path_relative..."
    ("eval" body line 3)
    invoked from within
"eval $nh_setup"⟦EOF⟧
```

- namespace-path-relative, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- current-before-global: intended operation reached.

- namespace-path-relative: intended operation not reached.

- relative-qualifier-missing-target: intended operation reached.

### tmm-RULE_INIT, unit 0:2, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:06:04 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_0 <RULE_INIT>: NHO|nh20261012broad1|current-before-global|tmm-RULE_INIT|setup|0|0:2|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:06:04 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_0 <RULE_INIT>: NHO|nh20261012broad1|current-before-global|tmm-RULE_INIT|lookup|0|0:2|once|result|binary-scan-tcl-value|0|1|43555252454e54
Oct 12 08:06:04 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_0 <RULE_INIT>: NHO|nh20261012broad1|current-before-global|tmm-RULE_INIT|runtime-cleanup|0|0:2|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:06:04 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_0 <RULE_INIT>: NHO|nh20261012broad1|relative-qualifier-missing-target|tmm-RULE_INIT|setup|0|0:2|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:06:04 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_0 <RULE_INIT>: NHO|nh20261012broad1|relative-qualifier-missing-target|tmm-RULE_INIT|lookup|0|0:2|once|result|binary-scan-tcl-value|0|1|474c4f42414c5f544152474554
Oct 12 08:06:04 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_0 <RULE_INIT>: NHO|nh20261012broad1|relative-qualifier-missing-target|tmm-RULE_INIT|runtime-cleanup|0|0:2|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:06:04 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_0 <RULE_INIT>: NHO|nh20261012broad1|namespace-path-relative|tmm-RULE_INIT|setup|1|0:2|once|result|binary-scan-tcl-value|0|1|626164206f7074696f6e202270617468223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f72207768696368
Oct 12 08:06:04 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_0 <RULE_INIT>: NHO|nh20261012broad1|namespace-path-relative|tmm-RULE_INIT|setup|1|0:2|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:06:04 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_0 <RULE_INIT>: NHO|nh20261012broad1|namespace-path-relative|tmm-RULE_INIT|setup|1|0:2|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:06:04 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_0 <RULE_INIT>: NHO|nh20261012broad1|namespace-path-relative|tmm-RULE_INIT|setup|1|0:2|once|errorinfo|binary-scan-tcl-value|0|3|626164206f7074696f6e202270617468223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f722077686963680a202020207768696c6520657865637574696e670a226e616d6573706163652070617468203a3a5f5f74636c5f6c73705f7072
Oct 12 08:06:04 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_0 <RULE_INIT>: NHO|nh20261012broad1|namespace-path-relative|tmm-RULE_INIT|setup|1|0:2|once|errorinfo|binary-scan-tcl-value|1|3|6f62655f6e68323032363130313262726f616431355f6e616d6573706163655f706174685f72656c61746976655f70617468220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6e616d6573706163655f706174685f72656c61746976652220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f5f74636c5f
Oct 12 08:06:04 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_0 <RULE_INIT>: NHO|nh20261012broad1|namespace-path-relative|tmm-RULE_INIT|setup|1|0:2|once|errorinfo|binary-scan-tcl-value|2|3|6c73705f70726f62655f6e68323032363130313262726f616431355f6e616d6573706163655f706174685f72656c6174697665207b6e616d6573706163652070617468203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6e616d6573706163655f706174685f72656c61746976652e2e2e220a2020202028226576616c2220626f6479206c696e652033290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f736574757022
Oct 12 08:06:04 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_0 <RULE_INIT>: NHO|nh20261012broad1|namespace-path-relative|tmm-RULE_INIT|runtime-cleanup|0|0:2|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- current-before-global, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- current-before-global, lookup, completion 0, result, chunk 0 of 1:

```text-visible
CURRENT⟦EOF⟧
```

- current-before-global, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- relative-qualifier-missing-target, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- relative-qualifier-missing-target, lookup, completion 0, result, chunk 0 of 1:

```text-visible
GLOBAL_TARGET⟦EOF⟧
```

- relative-qualifier-missing-target, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- namespace-path-relative, setup, completion 1, result, chunk 0 of 1:

```text-visible
bad option "path": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which⟦EOF⟧
```

- namespace-path-relative, setup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- namespace-path-relative, setup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- namespace-path-relative, setup, completion 1, errorinfo, chunk 0 of 3:

```text-visible
bad option "path": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which
    while executing
"namespace path ::__tcl_lsp_pr⟦EOF⟧
```

- namespace-path-relative, setup, completion 1, errorinfo, chunk 1 of 3:

```text-visible
obe_nh20261012broad15_namespace_path_relative_path"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad15_namespace_path_relative" script line 1)
    invoked from within
"namespace eval ::__tcl_⟦EOF⟧
```

- namespace-path-relative, setup, completion 1, errorinfo, chunk 2 of 3:

```text-visible
lsp_probe_nh20261012broad15_namespace_path_relative {namespace path ::__tcl_lsp_probe_nh20261012broad15_namespace_path_relative..."
    ("eval" body line 3)
    invoked from within
"eval $nh_setup"⟦EOF⟧
```

- namespace-path-relative, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- current-before-global: intended operation reached.

- namespace-path-relative: intended operation not reached.

- relative-qualifier-missing-target: intended operation reached.

### tmm-RULE_INIT, unit 0:3, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:06:04 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_0 <RULE_INIT>: NHO|nh20261012broad1|current-before-global|tmm-RULE_INIT|setup|0|0:3|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:06:04 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_0 <RULE_INIT>: NHO|nh20261012broad1|current-before-global|tmm-RULE_INIT|lookup|0|0:3|once|result|binary-scan-tcl-value|0|1|43555252454e54
Oct 12 08:06:04 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_0 <RULE_INIT>: NHO|nh20261012broad1|current-before-global|tmm-RULE_INIT|runtime-cleanup|0|0:3|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:06:04 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_0 <RULE_INIT>: NHO|nh20261012broad1|relative-qualifier-missing-target|tmm-RULE_INIT|setup|0|0:3|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:06:04 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_0 <RULE_INIT>: NHO|nh20261012broad1|relative-qualifier-missing-target|tmm-RULE_INIT|lookup|0|0:3|once|result|binary-scan-tcl-value|0|1|474c4f42414c5f544152474554
Oct 12 08:06:04 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_0 <RULE_INIT>: NHO|nh20261012broad1|relative-qualifier-missing-target|tmm-RULE_INIT|runtime-cleanup|0|0:3|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:06:04 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_0 <RULE_INIT>: NHO|nh20261012broad1|namespace-path-relative|tmm-RULE_INIT|setup|1|0:3|once|result|binary-scan-tcl-value|0|1|626164206f7074696f6e202270617468223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f72207768696368
Oct 12 08:06:04 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_0 <RULE_INIT>: NHO|nh20261012broad1|namespace-path-relative|tmm-RULE_INIT|setup|1|0:3|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:06:04 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_0 <RULE_INIT>: NHO|nh20261012broad1|namespace-path-relative|tmm-RULE_INIT|setup|1|0:3|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:06:04 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_0 <RULE_INIT>: NHO|nh20261012broad1|namespace-path-relative|tmm-RULE_INIT|setup|1|0:3|once|errorinfo|binary-scan-tcl-value|0|3|626164206f7074696f6e202270617468223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f722077686963680a202020207768696c6520657865637574696e670a226e616d6573706163652070617468203a3a5f5f74636c5f6c73705f7072
Oct 12 08:06:04 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_0 <RULE_INIT>: NHO|nh20261012broad1|namespace-path-relative|tmm-RULE_INIT|setup|1|0:3|once|errorinfo|binary-scan-tcl-value|1|3|6f62655f6e68323032363130313262726f616431355f6e616d6573706163655f706174685f72656c61746976655f70617468220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6e616d6573706163655f706174685f72656c61746976652220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f5f74636c5f
Oct 12 08:06:04 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_0 <RULE_INIT>: NHO|nh20261012broad1|namespace-path-relative|tmm-RULE_INIT|setup|1|0:3|once|errorinfo|binary-scan-tcl-value|2|3|6c73705f70726f62655f6e68323032363130313262726f616431355f6e616d6573706163655f706174685f72656c6174697665207b6e616d6573706163652070617468203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431355f6e616d6573706163655f706174685f72656c61746976652e2e2e220a2020202028226576616c2220626f6479206c696e652033290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f736574757022
Oct 12 08:06:04 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_5_0 <RULE_INIT>: NHO|nh20261012broad1|namespace-path-relative|tmm-RULE_INIT|runtime-cleanup|0|0:3|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- current-before-global, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- current-before-global, lookup, completion 0, result, chunk 0 of 1:

```text-visible
CURRENT⟦EOF⟧
```

- current-before-global, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- relative-qualifier-missing-target, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- relative-qualifier-missing-target, lookup, completion 0, result, chunk 0 of 1:

```text-visible
GLOBAL_TARGET⟦EOF⟧
```

- relative-qualifier-missing-target, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- namespace-path-relative, setup, completion 1, result, chunk 0 of 1:

```text-visible
bad option "path": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which⟦EOF⟧
```

- namespace-path-relative, setup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- namespace-path-relative, setup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- namespace-path-relative, setup, completion 1, errorinfo, chunk 0 of 3:

```text-visible
bad option "path": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which
    while executing
"namespace path ::__tcl_lsp_pr⟦EOF⟧
```

- namespace-path-relative, setup, completion 1, errorinfo, chunk 1 of 3:

```text-visible
obe_nh20261012broad15_namespace_path_relative_path"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad15_namespace_path_relative" script line 1)
    invoked from within
"namespace eval ::__tcl_⟦EOF⟧
```

- namespace-path-relative, setup, completion 1, errorinfo, chunk 2 of 3:

```text-visible
lsp_probe_nh20261012broad15_namespace_path_relative {namespace path ::__tcl_lsp_probe_nh20261012broad15_namespace_path_relative..."
    ("eval" body line 3)
    invoked from within
"eval $nh_setup"⟦EOF⟧
```

- namespace-path-relative, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- current-before-global: intended operation reached.

- namespace-path-relative: intended operation not reached.

- relative-qualifier-missing-target: intended operation reached.
