# BIG-IP Tcl log: nh20261012broad1, tmm-procedure-4

## Question

The following cases investigate Tcl behavior relevant to name resolution and diagnostics in the LSP, compiler and VM. Each observation is limited to its own case and recorded context.

- **literal-variable-name-data:** Literal `{}`, `{$n}`, `{{a}}`, Unicode and byte-valued names; array indices with colons/parentheses. Are byte values representable in each context?

- **namespace-exists-global-fallback:** Qualified/relative-qualified variables and namespace-scope missing-read fallback: read/write/create/exists differences.

- **relative-variable-missing-parent:** Qualified/relative-qualified variables and namespace-scope missing-read fallback: read/write/create/exists differences.

## Assumptions

- **literal-variable-name-data:** NH08 adapted namespace; braced spellings remain literal.
- **namespace-exists-global-fallback:** Existence contrast to missing read; clean ::__tcl_lsp_probe_nh20261012broad16_namespace_exists_global_fallback_x.
- **relative-variable-missing-parent:** Creation with nonexistent relative qualifier; expected error is a measurement.

Other assumptions were not separately recorded.

## Script

```tcl-visible
set nh_unit "[TMM::cmp_group]:[TMM::cmp_unit]"
set nh_invocation once
set nh_setup [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6e616d6573706163655f6578697374735f676c6f62616c5f66616c6c6261636b207b7d0a736574203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6e616d6573706163655f6578697374735f676c6f62616c5f66616c6c6261636b5f7820474c4f42414c5f56414c55450a6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6e616d6573706163655f6578697374735f676c6f62616c5f66616c6c6261636b207b7d0a]
set nh_operation [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6e616d6573706163655f6578697374735f676c6f62616c5f66616c6c6261636b207b696e666f20657869737473205f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6e616d6573706163655f6578697374735f676c6f62616c5f66616c6c6261636b5f787d0a]
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
        log local0. "NHO|nh20261012broad1|namespace-exists-global-fallback|tmm-procedure|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012broad1|namespace-exists-global-fallback|tmm-procedure|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 666f7265616368206e685f6368696c64205b696e7465727020736c617665735d207b6966207b5b737472696e67206d61746368205f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6e616d6573706163655f6578697374735f676c6f62616c5f66616c6c6261636b2a20246e685f6368696c645d7d207b696e746572702064656c65746520246e685f6368696c647d7d0a666f7265616368206e685f6e73205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6e616d6573706163655f6578697374735f676c6f62616c5f66616c6c6261636b2a5d207b6e616d6573706163652064656c65746520246e685f6e737d0a666f7265616368206e685f636d64205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6e616d6573706163655f6578697374735f676c6f62616c5f66616c6c6261636b2a5d207b72656e616d6520246e685f636d64207b7d7d0a666f7265616368206e685f766172205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6e616d6573706163655f6578697374735f676c6f62616c5f66616c6c6261636b2a5d207b756e736574202d6e6f636f6d706c61696e20246e685f7661727d0a6c697374205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6e616d6573706163655f6578697374735f676c6f62616c5f66616c6c6261636b2a5d205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6e616d6573706163655f6578697374735f676c6f62616c5f66616c6c6261636b2a5d205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6e616d6573706163655f6578697374735f676c6f62616c5f66616c6c6261636b2a5d]
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
        log local0. "NHO|nh20261012broad1|namespace-exists-global-fallback|tmm-procedure|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
set nh_setup [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f72656c61746976655f7661726961626c655f6d697373696e675f706172656e74207b7d0a6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f72656c61746976655f7661726961626c655f6d697373696e675f706172656e74207b7d0a]
set nh_operation [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f72656c61746976655f7661726961626c655f6d697373696e675f706172656e74207b73657420616273656e743a3a782056414c55457d0a]
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
        log local0. "NHO|nh20261012broad1|relative-variable-missing-parent|tmm-procedure|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012broad1|relative-variable-missing-parent|tmm-procedure|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 666f7265616368206e685f6368696c64205b696e7465727020736c617665735d207b6966207b5b737472696e67206d61746368205f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f72656c61746976655f7661726961626c655f6d697373696e675f706172656e742a20246e685f6368696c645d7d207b696e746572702064656c65746520246e685f6368696c647d7d0a666f7265616368206e685f6e73205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f72656c61746976655f7661726961626c655f6d697373696e675f706172656e742a5d207b6e616d6573706163652064656c65746520246e685f6e737d0a666f7265616368206e685f636d64205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f72656c61746976655f7661726961626c655f6d697373696e675f706172656e742a5d207b72656e616d6520246e685f636d64207b7d7d0a666f7265616368206e685f766172205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f72656c61746976655f7661726961626c655f6d697373696e675f706172656e742a5d207b756e736574202d6e6f636f6d706c61696e20246e685f7661727d0a6c697374205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f72656c61746976655f7661726961626c655f6d697373696e675f706172656e742a5d205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f72656c61746976655f7661726961626c655f6d697373696e675f706172656e742a5d205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f72656c61746976655f7661726961626c655f6d697373696e675f706172656e742a5d]
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
        log local0. "NHO|nh20261012broad1|relative-variable-missing-parent|tmm-procedure|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
set nh_setup [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6c69746572616c5f7661726961626c655f6e616d655f64617461207b7d0a6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6c69746572616c5f7661726961626c655f6e616d655f64617461207b736574207b7d20454d5054593b20736574207b246e7d204c49544552414c5f444f4c4c41523b20736574207b7b617d7d204c49544552414c5f4252414345537d0a]
set nh_operation [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6c69746572616c5f7661726961626c655f6e616d655f64617461207b6c697374205b736574207b7d5d205b736574207b246e7d5d205b736574207b7b617d7d5d205b696e666f20657869737473207b7b617d7d5d7d0a]
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
        log local0. "NHO|nh20261012broad1|literal-variable-name-data|tmm-procedure|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012broad1|literal-variable-name-data|tmm-procedure|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 666f7265616368206e685f6368696c64205b696e7465727020736c617665735d207b6966207b5b737472696e67206d61746368205f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6c69746572616c5f7661726961626c655f6e616d655f646174612a20246e685f6368696c645d7d207b696e746572702064656c65746520246e685f6368696c647d7d0a666f7265616368206e685f6e73205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6c69746572616c5f7661726961626c655f6e616d655f646174612a5d207b6e616d6573706163652064656c65746520246e685f6e737d0a666f7265616368206e685f636d64205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6c69746572616c5f7661726961626c655f6e616d655f646174612a5d207b72656e616d6520246e685f636d64207b7d7d0a666f7265616368206e685f766172205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6c69746572616c5f7661726961626c655f6e616d655f646174612a5d207b756e736574202d6e6f636f6d706c61696e20246e685f7661727d0a6c697374205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6c69746572616c5f7661726961626c655f6e616d655f646174612a5d205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6c69746572616c5f7661726961626c655f6e616d655f646174612a5d205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f6c69746572616c5f7661726961626c655f6e616d655f646174612a5d]
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
        log local0. "NHO|nh20261012broad1|literal-variable-name-data|tmm-procedure|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
Oct 12 08:09:13 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_4 <RULE_INIT>: NHO|nh20261012broad1|namespace-exists-global-fallback|tmm-procedure|setup|0|0:0|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:09:13 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_4 <RULE_INIT>: NHO|nh20261012broad1|namespace-exists-global-fallback|tmm-procedure|lookup|0|0:0|once|result|binary-scan-tcl-value|0|1|31
Oct 12 08:09:13 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_4 <RULE_INIT>: NHO|nh20261012broad1|namespace-exists-global-fallback|tmm-procedure|runtime-cleanup|0|0:0|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:09:13 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_4 <RULE_INIT>: NHO|nh20261012broad1|relative-variable-missing-parent|tmm-procedure|setup|0|0:0|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:09:13 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_4 <RULE_INIT>: NHO|nh20261012broad1|relative-variable-missing-parent|tmm-procedure|lookup|1|0:0|once|result|binary-scan-tcl-value|0|1|63616e2774207365742022616273656e743a3a78223a20706172656e74206e616d65737061636520646f65736e2774206578697374
Oct 12 08:09:13 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_4 <RULE_INIT>: NHO|nh20261012broad1|relative-variable-missing-parent|tmm-procedure|lookup|1|0:0|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:09:13 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_4 <RULE_INIT>: NHO|nh20261012broad1|relative-variable-missing-parent|tmm-procedure|lookup|1|0:0|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:09:13 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_4 <RULE_INIT>: NHO|nh20261012broad1|relative-variable-missing-parent|tmm-procedure|lookup|1|0:0|once|errorinfo|binary-scan-tcl-value|0|3|63616e2774207365742022616273656e743a3a78223a20706172656e74206e616d65737061636520646f65736e27742065786973740a202020207768696c6520657865637574696e670a2273657420616273656e743a3a782056414c5545220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f72656c61746976655f7661726961626c655f6d697373696e675f706172656e742220736372697074206c696e
Oct 12 08:09:13 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_4 <RULE_INIT>: NHO|nh20261012broad1|relative-variable-missing-parent|tmm-procedure|lookup|1|0:0|once|errorinfo|binary-scan-tcl-value|1|3|652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f72656c61746976655f7661726961626c655f6d697373696e675f706172656e74207b73657420616273656e743a3a782056414c55457d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f706572
Oct 12 08:09:13 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_4 <RULE_INIT>: NHO|nh20261012broad1|relative-variable-missing-parent|tmm-procedure|lookup|1|0:0|once|errorinfo|binary-scan-tcl-value|2|3|6174696f6e22
Oct 12 08:09:13 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_4 <RULE_INIT>: NHO|nh20261012broad1|relative-variable-missing-parent|tmm-procedure|runtime-cleanup|0|0:0|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:09:13 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_4 <RULE_INIT>: NHO|nh20261012broad1|literal-variable-name-data|tmm-procedure|setup|0|0:0|once|result|binary-scan-tcl-value|0|1|4c49544552414c5f425241434553
Oct 12 08:09:13 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_4 <RULE_INIT>: NHO|nh20261012broad1|literal-variable-name-data|tmm-procedure|lookup|0|0:0|once|result|binary-scan-tcl-value|0|1|454d505459204c49544552414c5f444f4c4c4152204c49544552414c5f4252414345532031
Oct 12 08:09:13 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_4 <RULE_INIT>: NHO|nh20261012broad1|literal-variable-name-data|tmm-procedure|runtime-cleanup|0|0:0|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- namespace-exists-global-fallback, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- namespace-exists-global-fallback, lookup, completion 0, result, chunk 0 of 1:

```text-visible
1⟦EOF⟧
```

- namespace-exists-global-fallback, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- relative-variable-missing-parent, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- relative-variable-missing-parent, lookup, completion 1, result, chunk 0 of 1:

```text-visible
can't set "absent::x": parent namespace doesn't exist⟦EOF⟧
```

- relative-variable-missing-parent, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- relative-variable-missing-parent, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- relative-variable-missing-parent, lookup, completion 1, errorinfo, chunk 0 of 3:

```text-visible
can't set "absent::x": parent namespace doesn't exist
    while executing
"set absent::x VALUE"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad16_relative_variable_missing_parent" script lin⟦EOF⟧
```

- relative-variable-missing-parent, lookup, completion 1, errorinfo, chunk 1 of 3:

```text-visible
e 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad16_relative_variable_missing_parent {set absent::x VALUE}"
    ("eval" body line 1)
    invoked from within
"eval $nh_oper⟦EOF⟧
```

- relative-variable-missing-parent, lookup, completion 1, errorinfo, chunk 2 of 3:

```text-visible
ation"⟦EOF⟧
```

- relative-variable-missing-parent, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- literal-variable-name-data, setup, completion 0, result, chunk 0 of 1:

```text-visible
LITERAL_BRACES⟦EOF⟧
```

- literal-variable-name-data, lookup, completion 0, result, chunk 0 of 1:

```text-visible
EMPTY LITERAL_DOLLAR LITERAL_BRACES 1⟦EOF⟧
```

- literal-variable-name-data, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- literal-variable-name-data: intended operation reached.

- namespace-exists-global-fallback: intended operation reached.

- relative-variable-missing-parent: intended operation reached.

### tmm-procedure, unit 0:1, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-procedure.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:09:13 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_4 <RULE_INIT>: NHO|nh20261012broad1|namespace-exists-global-fallback|tmm-procedure|setup|0|0:1|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:09:13 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_4 <RULE_INIT>: NHO|nh20261012broad1|namespace-exists-global-fallback|tmm-procedure|lookup|0|0:1|once|result|binary-scan-tcl-value|0|1|31
Oct 12 08:09:13 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_4 <RULE_INIT>: NHO|nh20261012broad1|namespace-exists-global-fallback|tmm-procedure|runtime-cleanup|0|0:1|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:09:13 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_4 <RULE_INIT>: NHO|nh20261012broad1|relative-variable-missing-parent|tmm-procedure|setup|0|0:1|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:09:13 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_4 <RULE_INIT>: NHO|nh20261012broad1|relative-variable-missing-parent|tmm-procedure|lookup|1|0:1|once|result|binary-scan-tcl-value|0|1|63616e2774207365742022616273656e743a3a78223a20706172656e74206e616d65737061636520646f65736e2774206578697374
Oct 12 08:09:13 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_4 <RULE_INIT>: NHO|nh20261012broad1|relative-variable-missing-parent|tmm-procedure|lookup|1|0:1|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:09:13 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_4 <RULE_INIT>: NHO|nh20261012broad1|relative-variable-missing-parent|tmm-procedure|lookup|1|0:1|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:09:13 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_4 <RULE_INIT>: NHO|nh20261012broad1|relative-variable-missing-parent|tmm-procedure|lookup|1|0:1|once|errorinfo|binary-scan-tcl-value|0|3|63616e2774207365742022616273656e743a3a78223a20706172656e74206e616d65737061636520646f65736e27742065786973740a202020207768696c6520657865637574696e670a2273657420616273656e743a3a782056414c5545220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f72656c61746976655f7661726961626c655f6d697373696e675f706172656e742220736372697074206c696e
Oct 12 08:09:13 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_4 <RULE_INIT>: NHO|nh20261012broad1|relative-variable-missing-parent|tmm-procedure|lookup|1|0:1|once|errorinfo|binary-scan-tcl-value|1|3|652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f72656c61746976655f7661726961626c655f6d697373696e675f706172656e74207b73657420616273656e743a3a782056414c55457d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f706572
Oct 12 08:09:13 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_4 <RULE_INIT>: NHO|nh20261012broad1|relative-variable-missing-parent|tmm-procedure|lookup|1|0:1|once|errorinfo|binary-scan-tcl-value|2|3|6174696f6e22
Oct 12 08:09:13 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_4 <RULE_INIT>: NHO|nh20261012broad1|relative-variable-missing-parent|tmm-procedure|runtime-cleanup|0|0:1|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:09:13 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_4 <RULE_INIT>: NHO|nh20261012broad1|literal-variable-name-data|tmm-procedure|setup|0|0:1|once|result|binary-scan-tcl-value|0|1|4c49544552414c5f425241434553
Oct 12 08:09:13 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_4 <RULE_INIT>: NHO|nh20261012broad1|literal-variable-name-data|tmm-procedure|lookup|0|0:1|once|result|binary-scan-tcl-value|0|1|454d505459204c49544552414c5f444f4c4c4152204c49544552414c5f4252414345532031
Oct 12 08:09:13 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_4 <RULE_INIT>: NHO|nh20261012broad1|literal-variable-name-data|tmm-procedure|runtime-cleanup|0|0:1|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- namespace-exists-global-fallback, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- namespace-exists-global-fallback, lookup, completion 0, result, chunk 0 of 1:

```text-visible
1⟦EOF⟧
```

- namespace-exists-global-fallback, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- relative-variable-missing-parent, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- relative-variable-missing-parent, lookup, completion 1, result, chunk 0 of 1:

```text-visible
can't set "absent::x": parent namespace doesn't exist⟦EOF⟧
```

- relative-variable-missing-parent, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- relative-variable-missing-parent, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- relative-variable-missing-parent, lookup, completion 1, errorinfo, chunk 0 of 3:

```text-visible
can't set "absent::x": parent namespace doesn't exist
    while executing
"set absent::x VALUE"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad16_relative_variable_missing_parent" script lin⟦EOF⟧
```

- relative-variable-missing-parent, lookup, completion 1, errorinfo, chunk 1 of 3:

```text-visible
e 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad16_relative_variable_missing_parent {set absent::x VALUE}"
    ("eval" body line 1)
    invoked from within
"eval $nh_oper⟦EOF⟧
```

- relative-variable-missing-parent, lookup, completion 1, errorinfo, chunk 2 of 3:

```text-visible
ation"⟦EOF⟧
```

- relative-variable-missing-parent, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- literal-variable-name-data, setup, completion 0, result, chunk 0 of 1:

```text-visible
LITERAL_BRACES⟦EOF⟧
```

- literal-variable-name-data, lookup, completion 0, result, chunk 0 of 1:

```text-visible
EMPTY LITERAL_DOLLAR LITERAL_BRACES 1⟦EOF⟧
```

- literal-variable-name-data, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- literal-variable-name-data: intended operation reached.

- namespace-exists-global-fallback: intended operation reached.

- relative-variable-missing-parent: intended operation reached.

### tmm-procedure, unit 0:2, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-procedure.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:09:13 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_4 <RULE_INIT>: NHO|nh20261012broad1|namespace-exists-global-fallback|tmm-procedure|setup|0|0:2|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:09:13 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_4 <RULE_INIT>: NHO|nh20261012broad1|namespace-exists-global-fallback|tmm-procedure|lookup|0|0:2|once|result|binary-scan-tcl-value|0|1|31
Oct 12 08:09:13 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_4 <RULE_INIT>: NHO|nh20261012broad1|namespace-exists-global-fallback|tmm-procedure|runtime-cleanup|0|0:2|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:09:13 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_4 <RULE_INIT>: NHO|nh20261012broad1|relative-variable-missing-parent|tmm-procedure|setup|0|0:2|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:09:13 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_4 <RULE_INIT>: NHO|nh20261012broad1|relative-variable-missing-parent|tmm-procedure|lookup|1|0:2|once|result|binary-scan-tcl-value|0|1|63616e2774207365742022616273656e743a3a78223a20706172656e74206e616d65737061636520646f65736e2774206578697374
Oct 12 08:09:13 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_4 <RULE_INIT>: NHO|nh20261012broad1|relative-variable-missing-parent|tmm-procedure|lookup|1|0:2|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:09:13 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_4 <RULE_INIT>: NHO|nh20261012broad1|relative-variable-missing-parent|tmm-procedure|lookup|1|0:2|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:09:13 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_4 <RULE_INIT>: NHO|nh20261012broad1|relative-variable-missing-parent|tmm-procedure|lookup|1|0:2|once|errorinfo|binary-scan-tcl-value|0|3|63616e2774207365742022616273656e743a3a78223a20706172656e74206e616d65737061636520646f65736e27742065786973740a202020207768696c6520657865637574696e670a2273657420616273656e743a3a782056414c5545220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f72656c61746976655f7661726961626c655f6d697373696e675f706172656e742220736372697074206c696e
Oct 12 08:09:13 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_4 <RULE_INIT>: NHO|nh20261012broad1|relative-variable-missing-parent|tmm-procedure|lookup|1|0:2|once|errorinfo|binary-scan-tcl-value|1|3|652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f72656c61746976655f7661726961626c655f6d697373696e675f706172656e74207b73657420616273656e743a3a782056414c55457d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f706572
Oct 12 08:09:13 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_4 <RULE_INIT>: NHO|nh20261012broad1|relative-variable-missing-parent|tmm-procedure|lookup|1|0:2|once|errorinfo|binary-scan-tcl-value|2|3|6174696f6e22
Oct 12 08:09:13 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_4 <RULE_INIT>: NHO|nh20261012broad1|relative-variable-missing-parent|tmm-procedure|runtime-cleanup|0|0:2|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:09:13 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_4 <RULE_INIT>: NHO|nh20261012broad1|literal-variable-name-data|tmm-procedure|setup|0|0:2|once|result|binary-scan-tcl-value|0|1|4c49544552414c5f425241434553
Oct 12 08:09:13 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_4 <RULE_INIT>: NHO|nh20261012broad1|literal-variable-name-data|tmm-procedure|lookup|0|0:2|once|result|binary-scan-tcl-value|0|1|454d505459204c49544552414c5f444f4c4c4152204c49544552414c5f4252414345532031
Oct 12 08:09:13 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_4 <RULE_INIT>: NHO|nh20261012broad1|literal-variable-name-data|tmm-procedure|runtime-cleanup|0|0:2|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- namespace-exists-global-fallback, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- namespace-exists-global-fallback, lookup, completion 0, result, chunk 0 of 1:

```text-visible
1⟦EOF⟧
```

- namespace-exists-global-fallback, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- relative-variable-missing-parent, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- relative-variable-missing-parent, lookup, completion 1, result, chunk 0 of 1:

```text-visible
can't set "absent::x": parent namespace doesn't exist⟦EOF⟧
```

- relative-variable-missing-parent, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- relative-variable-missing-parent, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- relative-variable-missing-parent, lookup, completion 1, errorinfo, chunk 0 of 3:

```text-visible
can't set "absent::x": parent namespace doesn't exist
    while executing
"set absent::x VALUE"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad16_relative_variable_missing_parent" script lin⟦EOF⟧
```

- relative-variable-missing-parent, lookup, completion 1, errorinfo, chunk 1 of 3:

```text-visible
e 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad16_relative_variable_missing_parent {set absent::x VALUE}"
    ("eval" body line 1)
    invoked from within
"eval $nh_oper⟦EOF⟧
```

- relative-variable-missing-parent, lookup, completion 1, errorinfo, chunk 2 of 3:

```text-visible
ation"⟦EOF⟧
```

- relative-variable-missing-parent, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- literal-variable-name-data, setup, completion 0, result, chunk 0 of 1:

```text-visible
LITERAL_BRACES⟦EOF⟧
```

- literal-variable-name-data, lookup, completion 0, result, chunk 0 of 1:

```text-visible
EMPTY LITERAL_DOLLAR LITERAL_BRACES 1⟦EOF⟧
```

- literal-variable-name-data, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- literal-variable-name-data: intended operation reached.

- namespace-exists-global-fallback: intended operation reached.

- relative-variable-missing-parent: intended operation reached.

### tmm-procedure, unit 0:3, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-procedure.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:09:13 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_4 <RULE_INIT>: NHO|nh20261012broad1|namespace-exists-global-fallback|tmm-procedure|setup|0|0:3|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:09:13 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_4 <RULE_INIT>: NHO|nh20261012broad1|namespace-exists-global-fallback|tmm-procedure|lookup|0|0:3|once|result|binary-scan-tcl-value|0|1|31
Oct 12 08:09:13 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_4 <RULE_INIT>: NHO|nh20261012broad1|namespace-exists-global-fallback|tmm-procedure|runtime-cleanup|0|0:3|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:09:13 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_4 <RULE_INIT>: NHO|nh20261012broad1|relative-variable-missing-parent|tmm-procedure|setup|0|0:3|once|result|binary-scan-tcl-value|0|1|
Oct 12 08:09:13 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_4 <RULE_INIT>: NHO|nh20261012broad1|relative-variable-missing-parent|tmm-procedure|lookup|1|0:3|once|result|binary-scan-tcl-value|0|1|63616e2774207365742022616273656e743a3a78223a20706172656e74206e616d65737061636520646f65736e2774206578697374
Oct 12 08:09:13 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_4 <RULE_INIT>: NHO|nh20261012broad1|relative-variable-missing-parent|tmm-procedure|lookup|1|0:3|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 08:09:13 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_4 <RULE_INIT>: NHO|nh20261012broad1|relative-variable-missing-parent|tmm-procedure|lookup|1|0:3|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 08:09:13 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_4 <RULE_INIT>: NHO|nh20261012broad1|relative-variable-missing-parent|tmm-procedure|lookup|1|0:3|once|errorinfo|binary-scan-tcl-value|0|3|63616e2774207365742022616273656e743a3a78223a20706172656e74206e616d65737061636520646f65736e27742065786973740a202020207768696c6520657865637574696e670a2273657420616273656e743a3a782056414c5545220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f72656c61746976655f7661726961626c655f6d697373696e675f706172656e742220736372697074206c696e
Oct 12 08:09:13 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_4 <RULE_INIT>: NHO|nh20261012broad1|relative-variable-missing-parent|tmm-procedure|lookup|1|0:3|once|errorinfo|binary-scan-tcl-value|1|3|652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431365f72656c61746976655f7661726961626c655f6d697373696e675f706172656e74207b73657420616273656e743a3a782056414c55457d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f706572
Oct 12 08:09:13 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_4 <RULE_INIT>: NHO|nh20261012broad1|relative-variable-missing-parent|tmm-procedure|lookup|1|0:3|once|errorinfo|binary-scan-tcl-value|2|3|6174696f6e22
Oct 12 08:09:13 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_4 <RULE_INIT>: NHO|nh20261012broad1|relative-variable-missing-parent|tmm-procedure|runtime-cleanup|0|0:3|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:09:13 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_4 <RULE_INIT>: NHO|nh20261012broad1|literal-variable-name-data|tmm-procedure|setup|0|0:3|once|result|binary-scan-tcl-value|0|1|4c49544552414c5f425241434553
Oct 12 08:09:13 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_4 <RULE_INIT>: NHO|nh20261012broad1|literal-variable-name-data|tmm-procedure|lookup|0|0:3|once|result|binary-scan-tcl-value|0|1|454d505459204c49544552414c5f444f4c4c4152204c49544552414c5f4252414345532031
Oct 12 08:09:13 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad1_6_4 <RULE_INIT>: NHO|nh20261012broad1|literal-variable-name-data|tmm-procedure|runtime-cleanup|0|0:3|once|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- namespace-exists-global-fallback, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- namespace-exists-global-fallback, lookup, completion 0, result, chunk 0 of 1:

```text-visible
1⟦EOF⟧
```

- namespace-exists-global-fallback, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- relative-variable-missing-parent, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- relative-variable-missing-parent, lookup, completion 1, result, chunk 0 of 1:

```text-visible
can't set "absent::x": parent namespace doesn't exist⟦EOF⟧
```

- relative-variable-missing-parent, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- relative-variable-missing-parent, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- relative-variable-missing-parent, lookup, completion 1, errorinfo, chunk 0 of 3:

```text-visible
can't set "absent::x": parent namespace doesn't exist
    while executing
"set absent::x VALUE"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad16_relative_variable_missing_parent" script lin⟦EOF⟧
```

- relative-variable-missing-parent, lookup, completion 1, errorinfo, chunk 1 of 3:

```text-visible
e 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad16_relative_variable_missing_parent {set absent::x VALUE}"
    ("eval" body line 1)
    invoked from within
"eval $nh_oper⟦EOF⟧
```

- relative-variable-missing-parent, lookup, completion 1, errorinfo, chunk 2 of 3:

```text-visible
ation"⟦EOF⟧
```

- relative-variable-missing-parent, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- literal-variable-name-data, setup, completion 0, result, chunk 0 of 1:

```text-visible
LITERAL_BRACES⟦EOF⟧
```

- literal-variable-name-data, lookup, completion 0, result, chunk 0 of 1:

```text-visible
EMPTY LITERAL_DOLLAR LITERAL_BRACES 1⟦EOF⟧
```

- literal-variable-name-data, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- literal-variable-name-data: intended operation reached.

- namespace-exists-global-fallback: intended operation reached.

- relative-variable-missing-parent: intended operation reached.
