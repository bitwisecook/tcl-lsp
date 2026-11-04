# BIG-IP Tcl log: nh20261012broad2, tmm-traffic-event-20

## Question

The following cases investigate Tcl behavior relevant to name resolution and diagnostics in the LSP, compiler and VM. Each observation is limited to its own case and recorded context.

- **brace-newline-command:** Do lexical accepted names and runtime evaluated names differ? Compare braced/quoted/substituted/list command prefixes, brace-newline rules and unaddressable-name output round trips.

- **display-name-roundtrip:** Do lexical accepted names and runtime evaluated names differ? Compare braced/quoted/substituted/list command prefixes, brace-newline rules and unaddressable-name output round trips.

- **name-script-prefix-near-miss:** Do lexical accepted names and runtime evaluated names differ? Compare braced/quoted/substituted/list command prefixes, brace-newline rules and unaddressable-name output round trips.

## Assumptions

- **brace-newline-command:** Actual newline in runtime script; keep literal-load variant separate.
- **display-name-roundtrip:** Display-FQN roundtrip after duplicate-slot setup; returned spelling need not identify the original command slot.
- **name-script-prefix-near-miss:** String script reparses spaced head.

Other assumptions were not separately recorded.

## Script

```tcl-visible
set nh_options_available unavailable
set nh_return_options {}
set nh_unit "[TMM::cmp_group]:[TMM::cmp_unit]"
set nh_invocation once
set nh_invocation [HTTP::header value X-NH-Invocation]
set nh_setup [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d655f7363726970745f7072656669785f6e6561725f6d697373207b7d0a6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d655f7363726970745f7072656669785f6e6561725f6d697373207b70726f63207b74776f20776f7264737d2078207b72657475726e2024787d3b207365742068656164207b74776f20776f7264737d7d0a]
set nh_operation [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d655f7363726970745f7072656669785f6e6561725f6d697373207b6576616c20222468656164207b6120627d227d0a]
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
        log local0. "NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 666f7265616368206e685f6368696c64205b696e7465727020736c617665735d207b6966207b5b737472696e67206d61746368205f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d655f7363726970745f7072656669785f6e6561725f6d6973732a20246e685f6368696c645d7d207b696e746572702064656c65746520246e685f6368696c647d7d0a666f7265616368206e685f6e73205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d655f7363726970745f7072656669785f6e6561725f6d6973732a5d207b6e616d6573706163652064656c65746520246e685f6e737d0a666f7265616368206e685f636d64205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d655f7363726970745f7072656669785f6e6561725f6d6973732a5d207b72656e616d6520246e685f636d64207b7d7d0a666f7265616368206e685f766172205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d655f7363726970745f7072656669785f6e6561725f6d6973732a5d207b756e736574202d6e6f636f6d706c61696e20246e685f7661727d0a6c697374205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d655f7363726970745f7072656669785f6e6561725f6d6973732a5d205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d655f7363726970745f7072656669785f6e6561725f6d6973732a5d205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d655f7363726970745f7072656669785f6e6561725f6d6973732a5d]
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
        log local0. "NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
set nh_setup [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f62726163655f6e65776c696e655f636f6d6d616e64207b7d0a6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f62726163655f6e65776c696e655f636f6d6d616e64207b7d0a]
set nh_operation [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f62726163655f6e65776c696e655f636f6d6d616e64207b6c697374206120620a7b637d7d0a]
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
        log local0. "NHO|nh20261012broad2|brace-newline-command|tmm-traffic-event|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012broad2|brace-newline-command|tmm-traffic-event|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 666f7265616368206e685f6368696c64205b696e7465727020736c617665735d207b6966207b5b737472696e67206d61746368205f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f62726163655f6e65776c696e655f636f6d6d616e642a20246e685f6368696c645d7d207b696e746572702064656c65746520246e685f6368696c647d7d0a666f7265616368206e685f6e73205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f62726163655f6e65776c696e655f636f6d6d616e642a5d207b6e616d6573706163652064656c65746520246e685f6e737d0a666f7265616368206e685f636d64205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f62726163655f6e65776c696e655f636f6d6d616e642a5d207b72656e616d6520246e685f636d64207b7d7d0a666f7265616368206e685f766172205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f62726163655f6e65776c696e655f636f6d6d616e642a5d207b756e736574202d6e6f636f6d706c61696e20246e685f7661727d0a6c697374205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f62726163655f6e65776c696e655f636f6d6d616e642a5d205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f62726163655f6e65776c696e655f636f6d6d616e642a5d205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f62726163655f6e65776c696e655f636f6d6d616e642a5d]
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
        log local0. "NHO|nh20261012broad2|brace-newline-command|tmm-traffic-event|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
set nh_setup [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f646973706c61795f6e616d655f726f756e6474726970207b7d0a6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f646973706c61795f6e616d655f726f756e6474726970207b6e616d657370616365206576616c20613a207b70726f632070207b7d207b72657475726e2046495253545f534c4f547d7d3b206e616d657370616365206576616c2061207b70726f63203a70207b7d207b72657475726e205345434f4e445f534c4f547d7d7d0a]
set nh_operation [binary format H* 736574203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f646973706c61795f6e616d655f726f756e64747269703a3a646973706c61796564205b6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f646973706c61795f6e616d655f726f756e64747269703a3a613a207b6e616d65737061636520776869636820707d5d3b206576616c205b6c697374205b736574203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f646973706c61795f6e616d655f726f756e64747269703a3a646973706c617965645d5d0a]
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
        log local0. "NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 666f7265616368206e685f6368696c64205b696e7465727020736c617665735d207b6966207b5b737472696e67206d61746368205f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f646973706c61795f6e616d655f726f756e64747269702a20246e685f6368696c645d7d207b696e746572702064656c65746520246e685f6368696c647d7d0a666f7265616368206e685f6e73205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f646973706c61795f6e616d655f726f756e64747269702a5d207b6e616d6573706163652064656c65746520246e685f6e737d0a666f7265616368206e685f636d64205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f646973706c61795f6e616d655f726f756e64747269702a5d207b72656e616d6520246e685f636d64207b7d7d0a666f7265616368206e685f766172205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f646973706c61795f6e616d655f726f756e64747269702a5d207b756e736574202d6e6f636f6d706c61696e20246e685f7661727d0a6c697374205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f646973706c61795f6e616d655f726f756e64747269702a5d205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f646973706c61795f6e616d655f726f756e64747269702a5d205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f646973706c61795f6e616d655f726f756e64747269702a5d]
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
        log local0. "NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|setup|0|0:0|0|result|binary-scan-tcl-value|0|1|74776f20776f726473
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|0|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d65202274776f22
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|0|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|0|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|0|errorinfo|binary-scan-tcl-value|0|3|696e76616c696420636f6d6d616e64206e616d65202274776f220a202020207768696c6520657865637574696e670a2274776f20776f726473207b6120627d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20222468656164207b6120627d22220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d655f
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|0|errorinfo|binary-scan-tcl-value|1|3|7363726970745f7072656669785f6e6561725f6d6973732220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d655f7363726970745f7072656669785f6e6561725f6d697373207b6576616c20222468656164207b6120627d227d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|0|errorinfo|binary-scan-tcl-value|2|3|6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|runtime-cleanup|0|0:0|0|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|brace-newline-command|tmm-traffic-event|setup|0|0:0|0|result|binary-scan-tcl-value|0|1|
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|brace-newline-command|tmm-traffic-event|lookup|0|0:0|0|result|binary-scan-tcl-value|0|1|6120622063
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|brace-newline-command|tmm-traffic-event|runtime-cleanup|0|0:0|0|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|0|result|binary-scan-tcl-value|0|1|63616e2774206372656174652070726f63656475726520223a702220696e206e6f6e2d676c6f62616c206e616d6573706163652077697468206e616d65207374617274696e67207769746820223a22
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|0|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|0|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|0|errorinfo|binary-scan-tcl-value|0|4|63616e2774206372656174652070726f63656475726520223a702220696e206e6f6e2d676c6f62616c206e616d6573706163652077697468206e616d65207374617274696e67207769746820223a220a202020207768696c6520657865637574696e670a2270726f63203a70207b7d207b72657475726e205345434f4e445f534c4f547d220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f646973706c61
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|0|errorinfo|binary-scan-tcl-value|1|4|795f6e616d655f726f756e64747269703a3a612220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c2061207b70726f63203a70207b7d207b72657475726e205345434f4e445f534c4f547d7d220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f646973706c61795f6e616d655f726f756e64747269702220736372
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|0|errorinfo|binary-scan-tcl-value|2|4|697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f646973706c61795f6e616d655f726f756e6474726970207b6e616d657370616365206576616c20613a207b70726f632070207b7d207b72657475726e2046495253545f534c4f547d7d3b206e616d657370616365206576616c2061207b70726f63203a2e2e2e220a2020202028226576
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|0|errorinfo|binary-scan-tcl-value|3|4|616c2220626f6479206c696e652032290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f736574757022
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|runtime-cleanup|0|0:0|0|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- name-script-prefix-near-miss, setup, completion 0, result, chunk 0 of 1:

```text-visible
two words⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "two"⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, errorinfo, chunk 0 of 3:

```text-visible
invalid command name "two"
    while executing
"two words {a b}"
    ("eval" body line 1)
    invoked from within
"eval "$head {a b}""
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_name_⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, errorinfo, chunk 1 of 3:

```text-visible
script_prefix_near_miss" script line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad27_name_script_prefix_near_miss {eval "$head {a b}"}"
    ("eval" body line 1)
    invo⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, errorinfo, chunk 2 of 3:

```text-visible
ked from within
"eval $nh_operation"⟦EOF⟧
```

- name-script-prefix-near-miss, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- brace-newline-command, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- brace-newline-command, lookup, completion 0, result, chunk 0 of 1:

```text-visible
a b c⟦EOF⟧
```

- brace-newline-command, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, result, chunk 0 of 1:

```text-visible
can't create procedure ":p" in non-global namespace with name starting with ":"⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorinfo, chunk 0 of 4:

```text-visible
can't create procedure ":p" in non-global namespace with name starting with ":"
    while executing
"proc :p {} {return SECOND_SLOT}"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_displa⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorinfo, chunk 1 of 4:

```text-visible
y_name_roundtrip::a" script line 1)
    invoked from within
"namespace eval a {proc :p {} {return SECOND_SLOT}}"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_display_name_roundtrip" scr⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorinfo, chunk 2 of 4:

```text-visible
ipt line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad27_display_name_roundtrip {namespace eval a: {proc p {} {return FIRST_SLOT}}; namespace eval a {proc :..."
    ("ev⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorinfo, chunk 3 of 4:

```text-visible
al" body line 2)
    invoked from within
"eval $nh_setup"⟦EOF⟧
```

- display-name-roundtrip, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- brace-newline-command: intended operation reached.

- display-name-roundtrip: intended operation not reached.

- name-script-prefix-near-miss: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 1

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|setup|0|0:0|1|result|binary-scan-tcl-value|0|1|74776f20776f726473
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|1|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d65202274776f22
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|1|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|1|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|1|errorinfo|binary-scan-tcl-value|0|3|696e76616c696420636f6d6d616e64206e616d65202274776f220a202020207768696c6520657865637574696e670a2274776f20776f726473207b6120627d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20222468656164207b6120627d22220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d655f
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|1|errorinfo|binary-scan-tcl-value|1|3|7363726970745f7072656669785f6e6561725f6d6973732220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d655f7363726970745f7072656669785f6e6561725f6d697373207b6576616c20222468656164207b6120627d227d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|1|errorinfo|binary-scan-tcl-value|2|3|6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|runtime-cleanup|0|0:0|1|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|brace-newline-command|tmm-traffic-event|setup|0|0:0|1|result|binary-scan-tcl-value|0|1|
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|brace-newline-command|tmm-traffic-event|lookup|0|0:0|1|result|binary-scan-tcl-value|0|1|6120622063
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|brace-newline-command|tmm-traffic-event|runtime-cleanup|0|0:0|1|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|1|result|binary-scan-tcl-value|0|1|63616e2774206372656174652070726f63656475726520223a702220696e206e6f6e2d676c6f62616c206e616d6573706163652077697468206e616d65207374617274696e67207769746820223a22
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|1|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|1|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|1|errorinfo|binary-scan-tcl-value|0|4|63616e2774206372656174652070726f63656475726520223a702220696e206e6f6e2d676c6f62616c206e616d6573706163652077697468206e616d65207374617274696e67207769746820223a220a202020207768696c6520657865637574696e670a2270726f63203a70207b7d207b72657475726e205345434f4e445f534c4f547d220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f646973706c61
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|1|errorinfo|binary-scan-tcl-value|1|4|795f6e616d655f726f756e64747269703a3a612220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c2061207b70726f63203a70207b7d207b72657475726e205345434f4e445f534c4f547d7d220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f646973706c61795f6e616d655f726f756e64747269702220736372
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|1|errorinfo|binary-scan-tcl-value|2|4|697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f646973706c61795f6e616d655f726f756e6474726970207b6e616d657370616365206576616c20613a207b70726f632070207b7d207b72657475726e2046495253545f534c4f547d7d3b206e616d657370616365206576616c2061207b70726f63203a2e2e2e220a2020202028226576
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|1|errorinfo|binary-scan-tcl-value|3|4|616c2220626f6479206c696e652032290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f736574757022
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|runtime-cleanup|0|0:0|1|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- name-script-prefix-near-miss, setup, completion 0, result, chunk 0 of 1:

```text-visible
two words⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "two"⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, errorinfo, chunk 0 of 3:

```text-visible
invalid command name "two"
    while executing
"two words {a b}"
    ("eval" body line 1)
    invoked from within
"eval "$head {a b}""
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_name_⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, errorinfo, chunk 1 of 3:

```text-visible
script_prefix_near_miss" script line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad27_name_script_prefix_near_miss {eval "$head {a b}"}"
    ("eval" body line 1)
    invo⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, errorinfo, chunk 2 of 3:

```text-visible
ked from within
"eval $nh_operation"⟦EOF⟧
```

- name-script-prefix-near-miss, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- brace-newline-command, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- brace-newline-command, lookup, completion 0, result, chunk 0 of 1:

```text-visible
a b c⟦EOF⟧
```

- brace-newline-command, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, result, chunk 0 of 1:

```text-visible
can't create procedure ":p" in non-global namespace with name starting with ":"⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorinfo, chunk 0 of 4:

```text-visible
can't create procedure ":p" in non-global namespace with name starting with ":"
    while executing
"proc :p {} {return SECOND_SLOT}"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_displa⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorinfo, chunk 1 of 4:

```text-visible
y_name_roundtrip::a" script line 1)
    invoked from within
"namespace eval a {proc :p {} {return SECOND_SLOT}}"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_display_name_roundtrip" scr⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorinfo, chunk 2 of 4:

```text-visible
ipt line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad27_display_name_roundtrip {namespace eval a: {proc p {} {return FIRST_SLOT}}; namespace eval a {proc :..."
    ("ev⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorinfo, chunk 3 of 4:

```text-visible
al" body line 2)
    invoked from within
"eval $nh_setup"⟦EOF⟧
```

- display-name-roundtrip, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- brace-newline-command: intended operation reached.

- display-name-roundtrip: intended operation not reached.

- name-script-prefix-near-miss: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 10

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|setup|0|0:0|10|result|binary-scan-tcl-value|0|1|74776f20776f726473
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|10|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d65202274776f22
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|10|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|10|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|10|errorinfo|binary-scan-tcl-value|0|3|696e76616c696420636f6d6d616e64206e616d65202274776f220a202020207768696c6520657865637574696e670a2274776f20776f726473207b6120627d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20222468656164207b6120627d22220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d655f
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|10|errorinfo|binary-scan-tcl-value|1|3|7363726970745f7072656669785f6e6561725f6d6973732220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d655f7363726970745f7072656669785f6e6561725f6d697373207b6576616c20222468656164207b6120627d227d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|10|errorinfo|binary-scan-tcl-value|2|3|6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|runtime-cleanup|0|0:0|10|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|brace-newline-command|tmm-traffic-event|setup|0|0:0|10|result|binary-scan-tcl-value|0|1|
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|brace-newline-command|tmm-traffic-event|lookup|0|0:0|10|result|binary-scan-tcl-value|0|1|6120622063
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|brace-newline-command|tmm-traffic-event|runtime-cleanup|0|0:0|10|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|10|result|binary-scan-tcl-value|0|1|63616e2774206372656174652070726f63656475726520223a702220696e206e6f6e2d676c6f62616c206e616d6573706163652077697468206e616d65207374617274696e67207769746820223a22
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|10|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|10|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|10|errorinfo|binary-scan-tcl-value|0|4|63616e2774206372656174652070726f63656475726520223a702220696e206e6f6e2d676c6f62616c206e616d6573706163652077697468206e616d65207374617274696e67207769746820223a220a202020207768696c6520657865637574696e670a2270726f63203a70207b7d207b72657475726e205345434f4e445f534c4f547d220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f646973706c61
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|10|errorinfo|binary-scan-tcl-value|1|4|795f6e616d655f726f756e64747269703a3a612220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c2061207b70726f63203a70207b7d207b72657475726e205345434f4e445f534c4f547d7d220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f646973706c61795f6e616d655f726f756e64747269702220736372
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|10|errorinfo|binary-scan-tcl-value|2|4|697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f646973706c61795f6e616d655f726f756e6474726970207b6e616d657370616365206576616c20613a207b70726f632070207b7d207b72657475726e2046495253545f534c4f547d7d3b206e616d657370616365206576616c2061207b70726f63203a2e2e2e220a2020202028226576
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|10|errorinfo|binary-scan-tcl-value|3|4|616c2220626f6479206c696e652032290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f736574757022
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|runtime-cleanup|0|0:0|10|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- name-script-prefix-near-miss, setup, completion 0, result, chunk 0 of 1:

```text-visible
two words⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "two"⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, errorinfo, chunk 0 of 3:

```text-visible
invalid command name "two"
    while executing
"two words {a b}"
    ("eval" body line 1)
    invoked from within
"eval "$head {a b}""
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_name_⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, errorinfo, chunk 1 of 3:

```text-visible
script_prefix_near_miss" script line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad27_name_script_prefix_near_miss {eval "$head {a b}"}"
    ("eval" body line 1)
    invo⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, errorinfo, chunk 2 of 3:

```text-visible
ked from within
"eval $nh_operation"⟦EOF⟧
```

- name-script-prefix-near-miss, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- brace-newline-command, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- brace-newline-command, lookup, completion 0, result, chunk 0 of 1:

```text-visible
a b c⟦EOF⟧
```

- brace-newline-command, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, result, chunk 0 of 1:

```text-visible
can't create procedure ":p" in non-global namespace with name starting with ":"⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorinfo, chunk 0 of 4:

```text-visible
can't create procedure ":p" in non-global namespace with name starting with ":"
    while executing
"proc :p {} {return SECOND_SLOT}"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_displa⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorinfo, chunk 1 of 4:

```text-visible
y_name_roundtrip::a" script line 1)
    invoked from within
"namespace eval a {proc :p {} {return SECOND_SLOT}}"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_display_name_roundtrip" scr⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorinfo, chunk 2 of 4:

```text-visible
ipt line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad27_display_name_roundtrip {namespace eval a: {proc p {} {return FIRST_SLOT}}; namespace eval a {proc :..."
    ("ev⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorinfo, chunk 3 of 4:

```text-visible
al" body line 2)
    invoked from within
"eval $nh_setup"⟦EOF⟧
```

- display-name-roundtrip, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- brace-newline-command: intended operation reached.

- display-name-roundtrip: intended operation not reached.

- name-script-prefix-near-miss: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 11

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|setup|0|0:0|11|result|binary-scan-tcl-value|0|1|74776f20776f726473
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|11|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d65202274776f22
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|11|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|11|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|11|errorinfo|binary-scan-tcl-value|0|3|696e76616c696420636f6d6d616e64206e616d65202274776f220a202020207768696c6520657865637574696e670a2274776f20776f726473207b6120627d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20222468656164207b6120627d22220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d655f
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|11|errorinfo|binary-scan-tcl-value|1|3|7363726970745f7072656669785f6e6561725f6d6973732220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d655f7363726970745f7072656669785f6e6561725f6d697373207b6576616c20222468656164207b6120627d227d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|11|errorinfo|binary-scan-tcl-value|2|3|6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|runtime-cleanup|0|0:0|11|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|brace-newline-command|tmm-traffic-event|setup|0|0:0|11|result|binary-scan-tcl-value|0|1|
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|brace-newline-command|tmm-traffic-event|lookup|0|0:0|11|result|binary-scan-tcl-value|0|1|6120622063
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|brace-newline-command|tmm-traffic-event|runtime-cleanup|0|0:0|11|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|11|result|binary-scan-tcl-value|0|1|63616e2774206372656174652070726f63656475726520223a702220696e206e6f6e2d676c6f62616c206e616d6573706163652077697468206e616d65207374617274696e67207769746820223a22
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|11|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|11|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|11|errorinfo|binary-scan-tcl-value|0|4|63616e2774206372656174652070726f63656475726520223a702220696e206e6f6e2d676c6f62616c206e616d6573706163652077697468206e616d65207374617274696e67207769746820223a220a202020207768696c6520657865637574696e670a2270726f63203a70207b7d207b72657475726e205345434f4e445f534c4f547d220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f646973706c61
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|11|errorinfo|binary-scan-tcl-value|1|4|795f6e616d655f726f756e64747269703a3a612220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c2061207b70726f63203a70207b7d207b72657475726e205345434f4e445f534c4f547d7d220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f646973706c61795f6e616d655f726f756e64747269702220736372
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|11|errorinfo|binary-scan-tcl-value|2|4|697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f646973706c61795f6e616d655f726f756e6474726970207b6e616d657370616365206576616c20613a207b70726f632070207b7d207b72657475726e2046495253545f534c4f547d7d3b206e616d657370616365206576616c2061207b70726f63203a2e2e2e220a2020202028226576
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|11|errorinfo|binary-scan-tcl-value|3|4|616c2220626f6479206c696e652032290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f736574757022
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|runtime-cleanup|0|0:0|11|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- name-script-prefix-near-miss, setup, completion 0, result, chunk 0 of 1:

```text-visible
two words⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "two"⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, errorinfo, chunk 0 of 3:

```text-visible
invalid command name "two"
    while executing
"two words {a b}"
    ("eval" body line 1)
    invoked from within
"eval "$head {a b}""
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_name_⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, errorinfo, chunk 1 of 3:

```text-visible
script_prefix_near_miss" script line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad27_name_script_prefix_near_miss {eval "$head {a b}"}"
    ("eval" body line 1)
    invo⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, errorinfo, chunk 2 of 3:

```text-visible
ked from within
"eval $nh_operation"⟦EOF⟧
```

- name-script-prefix-near-miss, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- brace-newline-command, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- brace-newline-command, lookup, completion 0, result, chunk 0 of 1:

```text-visible
a b c⟦EOF⟧
```

- brace-newline-command, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, result, chunk 0 of 1:

```text-visible
can't create procedure ":p" in non-global namespace with name starting with ":"⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorinfo, chunk 0 of 4:

```text-visible
can't create procedure ":p" in non-global namespace with name starting with ":"
    while executing
"proc :p {} {return SECOND_SLOT}"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_displa⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorinfo, chunk 1 of 4:

```text-visible
y_name_roundtrip::a" script line 1)
    invoked from within
"namespace eval a {proc :p {} {return SECOND_SLOT}}"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_display_name_roundtrip" scr⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorinfo, chunk 2 of 4:

```text-visible
ipt line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad27_display_name_roundtrip {namespace eval a: {proc p {} {return FIRST_SLOT}}; namespace eval a {proc :..."
    ("ev⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorinfo, chunk 3 of 4:

```text-visible
al" body line 2)
    invoked from within
"eval $nh_setup"⟦EOF⟧
```

- display-name-roundtrip, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- brace-newline-command: intended operation reached.

- display-name-roundtrip: intended operation not reached.

- name-script-prefix-near-miss: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 12

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|setup|0|0:0|12|result|binary-scan-tcl-value|0|1|74776f20776f726473
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|12|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d65202274776f22
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|12|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|12|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|12|errorinfo|binary-scan-tcl-value|0|3|696e76616c696420636f6d6d616e64206e616d65202274776f220a202020207768696c6520657865637574696e670a2274776f20776f726473207b6120627d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20222468656164207b6120627d22220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d655f
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|12|errorinfo|binary-scan-tcl-value|1|3|7363726970745f7072656669785f6e6561725f6d6973732220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d655f7363726970745f7072656669785f6e6561725f6d697373207b6576616c20222468656164207b6120627d227d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|12|errorinfo|binary-scan-tcl-value|2|3|6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|runtime-cleanup|0|0:0|12|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|brace-newline-command|tmm-traffic-event|setup|0|0:0|12|result|binary-scan-tcl-value|0|1|
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|brace-newline-command|tmm-traffic-event|lookup|0|0:0|12|result|binary-scan-tcl-value|0|1|6120622063
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|brace-newline-command|tmm-traffic-event|runtime-cleanup|0|0:0|12|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|12|result|binary-scan-tcl-value|0|1|63616e2774206372656174652070726f63656475726520223a702220696e206e6f6e2d676c6f62616c206e616d6573706163652077697468206e616d65207374617274696e67207769746820223a22
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|12|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|12|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|12|errorinfo|binary-scan-tcl-value|0|4|63616e2774206372656174652070726f63656475726520223a702220696e206e6f6e2d676c6f62616c206e616d6573706163652077697468206e616d65207374617274696e67207769746820223a220a202020207768696c6520657865637574696e670a2270726f63203a70207b7d207b72657475726e205345434f4e445f534c4f547d220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f646973706c61
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|12|errorinfo|binary-scan-tcl-value|1|4|795f6e616d655f726f756e64747269703a3a612220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c2061207b70726f63203a70207b7d207b72657475726e205345434f4e445f534c4f547d7d220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f646973706c61795f6e616d655f726f756e64747269702220736372
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|12|errorinfo|binary-scan-tcl-value|2|4|697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f646973706c61795f6e616d655f726f756e6474726970207b6e616d657370616365206576616c20613a207b70726f632070207b7d207b72657475726e2046495253545f534c4f547d7d3b206e616d657370616365206576616c2061207b70726f63203a2e2e2e220a2020202028226576
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|12|errorinfo|binary-scan-tcl-value|3|4|616c2220626f6479206c696e652032290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f736574757022
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|runtime-cleanup|0|0:0|12|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- name-script-prefix-near-miss, setup, completion 0, result, chunk 0 of 1:

```text-visible
two words⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "two"⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, errorinfo, chunk 0 of 3:

```text-visible
invalid command name "two"
    while executing
"two words {a b}"
    ("eval" body line 1)
    invoked from within
"eval "$head {a b}""
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_name_⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, errorinfo, chunk 1 of 3:

```text-visible
script_prefix_near_miss" script line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad27_name_script_prefix_near_miss {eval "$head {a b}"}"
    ("eval" body line 1)
    invo⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, errorinfo, chunk 2 of 3:

```text-visible
ked from within
"eval $nh_operation"⟦EOF⟧
```

- name-script-prefix-near-miss, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- brace-newline-command, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- brace-newline-command, lookup, completion 0, result, chunk 0 of 1:

```text-visible
a b c⟦EOF⟧
```

- brace-newline-command, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, result, chunk 0 of 1:

```text-visible
can't create procedure ":p" in non-global namespace with name starting with ":"⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorinfo, chunk 0 of 4:

```text-visible
can't create procedure ":p" in non-global namespace with name starting with ":"
    while executing
"proc :p {} {return SECOND_SLOT}"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_displa⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorinfo, chunk 1 of 4:

```text-visible
y_name_roundtrip::a" script line 1)
    invoked from within
"namespace eval a {proc :p {} {return SECOND_SLOT}}"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_display_name_roundtrip" scr⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorinfo, chunk 2 of 4:

```text-visible
ipt line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad27_display_name_roundtrip {namespace eval a: {proc p {} {return FIRST_SLOT}}; namespace eval a {proc :..."
    ("ev⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorinfo, chunk 3 of 4:

```text-visible
al" body line 2)
    invoked from within
"eval $nh_setup"⟦EOF⟧
```

- display-name-roundtrip, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- brace-newline-command: intended operation reached.

- display-name-roundtrip: intended operation not reached.

- name-script-prefix-near-miss: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 13

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 09:00:07 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|setup|0|0:0|13|result|binary-scan-tcl-value|0|1|74776f20776f726473
Oct 12 09:00:07 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|13|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d65202274776f22
Oct 12 09:00:07 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|13|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:00:07 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|13|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:00:07 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|13|errorinfo|binary-scan-tcl-value|0|3|696e76616c696420636f6d6d616e64206e616d65202274776f220a202020207768696c6520657865637574696e670a2274776f20776f726473207b6120627d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20222468656164207b6120627d22220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d655f
Oct 12 09:00:07 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|13|errorinfo|binary-scan-tcl-value|1|3|7363726970745f7072656669785f6e6561725f6d6973732220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d655f7363726970745f7072656669785f6e6561725f6d697373207b6576616c20222468656164207b6120627d227d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f
Oct 12 09:00:07 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|13|errorinfo|binary-scan-tcl-value|2|3|6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 09:00:07 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|runtime-cleanup|0|0:0|13|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 09:00:07 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|brace-newline-command|tmm-traffic-event|setup|0|0:0|13|result|binary-scan-tcl-value|0|1|
Oct 12 09:00:07 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|brace-newline-command|tmm-traffic-event|lookup|0|0:0|13|result|binary-scan-tcl-value|0|1|6120622063
Oct 12 09:00:07 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|brace-newline-command|tmm-traffic-event|runtime-cleanup|0|0:0|13|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 09:00:07 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|13|result|binary-scan-tcl-value|0|1|63616e2774206372656174652070726f63656475726520223a702220696e206e6f6e2d676c6f62616c206e616d6573706163652077697468206e616d65207374617274696e67207769746820223a22
Oct 12 09:00:07 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|13|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:00:07 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|13|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:00:07 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|13|errorinfo|binary-scan-tcl-value|0|4|63616e2774206372656174652070726f63656475726520223a702220696e206e6f6e2d676c6f62616c206e616d6573706163652077697468206e616d65207374617274696e67207769746820223a220a202020207768696c6520657865637574696e670a2270726f63203a70207b7d207b72657475726e205345434f4e445f534c4f547d220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f646973706c61
Oct 12 09:00:07 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|13|errorinfo|binary-scan-tcl-value|1|4|795f6e616d655f726f756e64747269703a3a612220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c2061207b70726f63203a70207b7d207b72657475726e205345434f4e445f534c4f547d7d220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f646973706c61795f6e616d655f726f756e64747269702220736372
Oct 12 09:00:07 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|13|errorinfo|binary-scan-tcl-value|2|4|697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f646973706c61795f6e616d655f726f756e6474726970207b6e616d657370616365206576616c20613a207b70726f632070207b7d207b72657475726e2046495253545f534c4f547d7d3b206e616d657370616365206576616c2061207b70726f63203a2e2e2e220a2020202028226576
Oct 12 09:00:07 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|13|errorinfo|binary-scan-tcl-value|3|4|616c2220626f6479206c696e652032290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f736574757022
Oct 12 09:00:07 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|runtime-cleanup|0|0:0|13|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- name-script-prefix-near-miss, setup, completion 0, result, chunk 0 of 1:

```text-visible
two words⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "two"⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, errorinfo, chunk 0 of 3:

```text-visible
invalid command name "two"
    while executing
"two words {a b}"
    ("eval" body line 1)
    invoked from within
"eval "$head {a b}""
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_name_⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, errorinfo, chunk 1 of 3:

```text-visible
script_prefix_near_miss" script line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad27_name_script_prefix_near_miss {eval "$head {a b}"}"
    ("eval" body line 1)
    invo⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, errorinfo, chunk 2 of 3:

```text-visible
ked from within
"eval $nh_operation"⟦EOF⟧
```

- name-script-prefix-near-miss, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- brace-newline-command, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- brace-newline-command, lookup, completion 0, result, chunk 0 of 1:

```text-visible
a b c⟦EOF⟧
```

- brace-newline-command, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, result, chunk 0 of 1:

```text-visible
can't create procedure ":p" in non-global namespace with name starting with ":"⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorinfo, chunk 0 of 4:

```text-visible
can't create procedure ":p" in non-global namespace with name starting with ":"
    while executing
"proc :p {} {return SECOND_SLOT}"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_displa⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorinfo, chunk 1 of 4:

```text-visible
y_name_roundtrip::a" script line 1)
    invoked from within
"namespace eval a {proc :p {} {return SECOND_SLOT}}"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_display_name_roundtrip" scr⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorinfo, chunk 2 of 4:

```text-visible
ipt line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad27_display_name_roundtrip {namespace eval a: {proc p {} {return FIRST_SLOT}}; namespace eval a {proc :..."
    ("ev⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorinfo, chunk 3 of 4:

```text-visible
al" body line 2)
    invoked from within
"eval $nh_setup"⟦EOF⟧
```

- display-name-roundtrip, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- brace-newline-command: intended operation reached.

- display-name-roundtrip: intended operation not reached.

- name-script-prefix-near-miss: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 14

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 09:00:07 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|setup|0|0:0|14|result|binary-scan-tcl-value|0|1|74776f20776f726473
Oct 12 09:00:07 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|14|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d65202274776f22
Oct 12 09:00:07 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|14|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:00:07 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|14|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:00:07 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|14|errorinfo|binary-scan-tcl-value|0|3|696e76616c696420636f6d6d616e64206e616d65202274776f220a202020207768696c6520657865637574696e670a2274776f20776f726473207b6120627d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20222468656164207b6120627d22220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d655f
Oct 12 09:00:07 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|14|errorinfo|binary-scan-tcl-value|1|3|7363726970745f7072656669785f6e6561725f6d6973732220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d655f7363726970745f7072656669785f6e6561725f6d697373207b6576616c20222468656164207b6120627d227d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f
Oct 12 09:00:07 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|14|errorinfo|binary-scan-tcl-value|2|3|6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 09:00:07 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|runtime-cleanup|0|0:0|14|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 09:00:07 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|brace-newline-command|tmm-traffic-event|setup|0|0:0|14|result|binary-scan-tcl-value|0|1|
Oct 12 09:00:07 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|brace-newline-command|tmm-traffic-event|lookup|0|0:0|14|result|binary-scan-tcl-value|0|1|6120622063
Oct 12 09:00:07 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|brace-newline-command|tmm-traffic-event|runtime-cleanup|0|0:0|14|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 09:00:07 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|14|result|binary-scan-tcl-value|0|1|63616e2774206372656174652070726f63656475726520223a702220696e206e6f6e2d676c6f62616c206e616d6573706163652077697468206e616d65207374617274696e67207769746820223a22
Oct 12 09:00:07 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|14|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:00:07 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|14|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:00:07 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|14|errorinfo|binary-scan-tcl-value|0|4|63616e2774206372656174652070726f63656475726520223a702220696e206e6f6e2d676c6f62616c206e616d6573706163652077697468206e616d65207374617274696e67207769746820223a220a202020207768696c6520657865637574696e670a2270726f63203a70207b7d207b72657475726e205345434f4e445f534c4f547d220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f646973706c61
Oct 12 09:00:07 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|14|errorinfo|binary-scan-tcl-value|1|4|795f6e616d655f726f756e64747269703a3a612220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c2061207b70726f63203a70207b7d207b72657475726e205345434f4e445f534c4f547d7d220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f646973706c61795f6e616d655f726f756e64747269702220736372
Oct 12 09:00:07 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|14|errorinfo|binary-scan-tcl-value|2|4|697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f646973706c61795f6e616d655f726f756e6474726970207b6e616d657370616365206576616c20613a207b70726f632070207b7d207b72657475726e2046495253545f534c4f547d7d3b206e616d657370616365206576616c2061207b70726f63203a2e2e2e220a2020202028226576
Oct 12 09:00:07 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|14|errorinfo|binary-scan-tcl-value|3|4|616c2220626f6479206c696e652032290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f736574757022
Oct 12 09:00:07 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|runtime-cleanup|0|0:0|14|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- name-script-prefix-near-miss, setup, completion 0, result, chunk 0 of 1:

```text-visible
two words⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "two"⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, errorinfo, chunk 0 of 3:

```text-visible
invalid command name "two"
    while executing
"two words {a b}"
    ("eval" body line 1)
    invoked from within
"eval "$head {a b}""
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_name_⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, errorinfo, chunk 1 of 3:

```text-visible
script_prefix_near_miss" script line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad27_name_script_prefix_near_miss {eval "$head {a b}"}"
    ("eval" body line 1)
    invo⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, errorinfo, chunk 2 of 3:

```text-visible
ked from within
"eval $nh_operation"⟦EOF⟧
```

- name-script-prefix-near-miss, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- brace-newline-command, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- brace-newline-command, lookup, completion 0, result, chunk 0 of 1:

```text-visible
a b c⟦EOF⟧
```

- brace-newline-command, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, result, chunk 0 of 1:

```text-visible
can't create procedure ":p" in non-global namespace with name starting with ":"⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorinfo, chunk 0 of 4:

```text-visible
can't create procedure ":p" in non-global namespace with name starting with ":"
    while executing
"proc :p {} {return SECOND_SLOT}"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_displa⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorinfo, chunk 1 of 4:

```text-visible
y_name_roundtrip::a" script line 1)
    invoked from within
"namespace eval a {proc :p {} {return SECOND_SLOT}}"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_display_name_roundtrip" scr⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorinfo, chunk 2 of 4:

```text-visible
ipt line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad27_display_name_roundtrip {namespace eval a: {proc p {} {return FIRST_SLOT}}; namespace eval a {proc :..."
    ("ev⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorinfo, chunk 3 of 4:

```text-visible
al" body line 2)
    invoked from within
"eval $nh_setup"⟦EOF⟧
```

- display-name-roundtrip, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- brace-newline-command: intended operation reached.

- display-name-roundtrip: intended operation not reached.

- name-script-prefix-near-miss: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 15

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 09:00:07 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|setup|0|0:0|15|result|binary-scan-tcl-value|0|1|74776f20776f726473
Oct 12 09:00:07 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|15|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d65202274776f22
Oct 12 09:00:07 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|15|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:00:07 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|15|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:00:07 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|15|errorinfo|binary-scan-tcl-value|0|3|696e76616c696420636f6d6d616e64206e616d65202274776f220a202020207768696c6520657865637574696e670a2274776f20776f726473207b6120627d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20222468656164207b6120627d22220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d655f
Oct 12 09:00:07 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|15|errorinfo|binary-scan-tcl-value|1|3|7363726970745f7072656669785f6e6561725f6d6973732220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d655f7363726970745f7072656669785f6e6561725f6d697373207b6576616c20222468656164207b6120627d227d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f
Oct 12 09:00:07 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|15|errorinfo|binary-scan-tcl-value|2|3|6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 09:00:07 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|runtime-cleanup|0|0:0|15|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 09:00:07 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|brace-newline-command|tmm-traffic-event|setup|0|0:0|15|result|binary-scan-tcl-value|0|1|
Oct 12 09:00:07 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|brace-newline-command|tmm-traffic-event|lookup|0|0:0|15|result|binary-scan-tcl-value|0|1|6120622063
Oct 12 09:00:07 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|brace-newline-command|tmm-traffic-event|runtime-cleanup|0|0:0|15|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 09:00:07 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|15|result|binary-scan-tcl-value|0|1|63616e2774206372656174652070726f63656475726520223a702220696e206e6f6e2d676c6f62616c206e616d6573706163652077697468206e616d65207374617274696e67207769746820223a22
Oct 12 09:00:07 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|15|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:00:07 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|15|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:00:07 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|15|errorinfo|binary-scan-tcl-value|0|4|63616e2774206372656174652070726f63656475726520223a702220696e206e6f6e2d676c6f62616c206e616d6573706163652077697468206e616d65207374617274696e67207769746820223a220a202020207768696c6520657865637574696e670a2270726f63203a70207b7d207b72657475726e205345434f4e445f534c4f547d220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f646973706c61
Oct 12 09:00:07 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|15|errorinfo|binary-scan-tcl-value|1|4|795f6e616d655f726f756e64747269703a3a612220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c2061207b70726f63203a70207b7d207b72657475726e205345434f4e445f534c4f547d7d220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f646973706c61795f6e616d655f726f756e64747269702220736372
Oct 12 09:00:07 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|15|errorinfo|binary-scan-tcl-value|2|4|697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f646973706c61795f6e616d655f726f756e6474726970207b6e616d657370616365206576616c20613a207b70726f632070207b7d207b72657475726e2046495253545f534c4f547d7d3b206e616d657370616365206576616c2061207b70726f63203a2e2e2e220a2020202028226576
Oct 12 09:00:07 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|15|errorinfo|binary-scan-tcl-value|3|4|616c2220626f6479206c696e652032290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f736574757022
Oct 12 09:00:07 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|runtime-cleanup|0|0:0|15|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- name-script-prefix-near-miss, setup, completion 0, result, chunk 0 of 1:

```text-visible
two words⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "two"⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, errorinfo, chunk 0 of 3:

```text-visible
invalid command name "two"
    while executing
"two words {a b}"
    ("eval" body line 1)
    invoked from within
"eval "$head {a b}""
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_name_⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, errorinfo, chunk 1 of 3:

```text-visible
script_prefix_near_miss" script line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad27_name_script_prefix_near_miss {eval "$head {a b}"}"
    ("eval" body line 1)
    invo⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, errorinfo, chunk 2 of 3:

```text-visible
ked from within
"eval $nh_operation"⟦EOF⟧
```

- name-script-prefix-near-miss, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- brace-newline-command, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- brace-newline-command, lookup, completion 0, result, chunk 0 of 1:

```text-visible
a b c⟦EOF⟧
```

- brace-newline-command, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, result, chunk 0 of 1:

```text-visible
can't create procedure ":p" in non-global namespace with name starting with ":"⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorinfo, chunk 0 of 4:

```text-visible
can't create procedure ":p" in non-global namespace with name starting with ":"
    while executing
"proc :p {} {return SECOND_SLOT}"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_displa⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorinfo, chunk 1 of 4:

```text-visible
y_name_roundtrip::a" script line 1)
    invoked from within
"namespace eval a {proc :p {} {return SECOND_SLOT}}"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_display_name_roundtrip" scr⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorinfo, chunk 2 of 4:

```text-visible
ipt line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad27_display_name_roundtrip {namespace eval a: {proc p {} {return FIRST_SLOT}}; namespace eval a {proc :..."
    ("ev⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorinfo, chunk 3 of 4:

```text-visible
al" body line 2)
    invoked from within
"eval $nh_setup"⟦EOF⟧
```

- display-name-roundtrip, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- brace-newline-command: intended operation reached.

- display-name-roundtrip: intended operation not reached.

- name-script-prefix-near-miss: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 2

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|setup|0|0:0|2|result|binary-scan-tcl-value|0|1|74776f20776f726473
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|2|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d65202274776f22
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|2|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|2|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|2|errorinfo|binary-scan-tcl-value|0|3|696e76616c696420636f6d6d616e64206e616d65202274776f220a202020207768696c6520657865637574696e670a2274776f20776f726473207b6120627d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20222468656164207b6120627d22220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d655f
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|2|errorinfo|binary-scan-tcl-value|1|3|7363726970745f7072656669785f6e6561725f6d6973732220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d655f7363726970745f7072656669785f6e6561725f6d697373207b6576616c20222468656164207b6120627d227d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|2|errorinfo|binary-scan-tcl-value|2|3|6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|runtime-cleanup|0|0:0|2|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|brace-newline-command|tmm-traffic-event|setup|0|0:0|2|result|binary-scan-tcl-value|0|1|
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|brace-newline-command|tmm-traffic-event|lookup|0|0:0|2|result|binary-scan-tcl-value|0|1|6120622063
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|brace-newline-command|tmm-traffic-event|runtime-cleanup|0|0:0|2|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|2|result|binary-scan-tcl-value|0|1|63616e2774206372656174652070726f63656475726520223a702220696e206e6f6e2d676c6f62616c206e616d6573706163652077697468206e616d65207374617274696e67207769746820223a22
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|2|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|2|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|2|errorinfo|binary-scan-tcl-value|0|4|63616e2774206372656174652070726f63656475726520223a702220696e206e6f6e2d676c6f62616c206e616d6573706163652077697468206e616d65207374617274696e67207769746820223a220a202020207768696c6520657865637574696e670a2270726f63203a70207b7d207b72657475726e205345434f4e445f534c4f547d220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f646973706c61
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|2|errorinfo|binary-scan-tcl-value|1|4|795f6e616d655f726f756e64747269703a3a612220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c2061207b70726f63203a70207b7d207b72657475726e205345434f4e445f534c4f547d7d220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f646973706c61795f6e616d655f726f756e64747269702220736372
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|2|errorinfo|binary-scan-tcl-value|2|4|697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f646973706c61795f6e616d655f726f756e6474726970207b6e616d657370616365206576616c20613a207b70726f632070207b7d207b72657475726e2046495253545f534c4f547d7d3b206e616d657370616365206576616c2061207b70726f63203a2e2e2e220a2020202028226576
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|2|errorinfo|binary-scan-tcl-value|3|4|616c2220626f6479206c696e652032290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f736574757022
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|runtime-cleanup|0|0:0|2|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- name-script-prefix-near-miss, setup, completion 0, result, chunk 0 of 1:

```text-visible
two words⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "two"⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, errorinfo, chunk 0 of 3:

```text-visible
invalid command name "two"
    while executing
"two words {a b}"
    ("eval" body line 1)
    invoked from within
"eval "$head {a b}""
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_name_⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, errorinfo, chunk 1 of 3:

```text-visible
script_prefix_near_miss" script line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad27_name_script_prefix_near_miss {eval "$head {a b}"}"
    ("eval" body line 1)
    invo⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, errorinfo, chunk 2 of 3:

```text-visible
ked from within
"eval $nh_operation"⟦EOF⟧
```

- name-script-prefix-near-miss, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- brace-newline-command, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- brace-newline-command, lookup, completion 0, result, chunk 0 of 1:

```text-visible
a b c⟦EOF⟧
```

- brace-newline-command, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, result, chunk 0 of 1:

```text-visible
can't create procedure ":p" in non-global namespace with name starting with ":"⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorinfo, chunk 0 of 4:

```text-visible
can't create procedure ":p" in non-global namespace with name starting with ":"
    while executing
"proc :p {} {return SECOND_SLOT}"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_displa⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorinfo, chunk 1 of 4:

```text-visible
y_name_roundtrip::a" script line 1)
    invoked from within
"namespace eval a {proc :p {} {return SECOND_SLOT}}"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_display_name_roundtrip" scr⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorinfo, chunk 2 of 4:

```text-visible
ipt line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad27_display_name_roundtrip {namespace eval a: {proc p {} {return FIRST_SLOT}}; namespace eval a {proc :..."
    ("ev⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorinfo, chunk 3 of 4:

```text-visible
al" body line 2)
    invoked from within
"eval $nh_setup"⟦EOF⟧
```

- display-name-roundtrip, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- brace-newline-command: intended operation reached.

- display-name-roundtrip: intended operation not reached.

- name-script-prefix-near-miss: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 3

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|setup|0|0:0|3|result|binary-scan-tcl-value|0|1|74776f20776f726473
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|3|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d65202274776f22
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|3|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|3|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|3|errorinfo|binary-scan-tcl-value|0|3|696e76616c696420636f6d6d616e64206e616d65202274776f220a202020207768696c6520657865637574696e670a2274776f20776f726473207b6120627d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20222468656164207b6120627d22220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d655f
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|3|errorinfo|binary-scan-tcl-value|1|3|7363726970745f7072656669785f6e6561725f6d6973732220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d655f7363726970745f7072656669785f6e6561725f6d697373207b6576616c20222468656164207b6120627d227d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|3|errorinfo|binary-scan-tcl-value|2|3|6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|runtime-cleanup|0|0:0|3|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|brace-newline-command|tmm-traffic-event|setup|0|0:0|3|result|binary-scan-tcl-value|0|1|
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|brace-newline-command|tmm-traffic-event|lookup|0|0:0|3|result|binary-scan-tcl-value|0|1|6120622063
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|brace-newline-command|tmm-traffic-event|runtime-cleanup|0|0:0|3|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|3|result|binary-scan-tcl-value|0|1|63616e2774206372656174652070726f63656475726520223a702220696e206e6f6e2d676c6f62616c206e616d6573706163652077697468206e616d65207374617274696e67207769746820223a22
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|3|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|3|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|3|errorinfo|binary-scan-tcl-value|0|4|63616e2774206372656174652070726f63656475726520223a702220696e206e6f6e2d676c6f62616c206e616d6573706163652077697468206e616d65207374617274696e67207769746820223a220a202020207768696c6520657865637574696e670a2270726f63203a70207b7d207b72657475726e205345434f4e445f534c4f547d220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f646973706c61
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|3|errorinfo|binary-scan-tcl-value|1|4|795f6e616d655f726f756e64747269703a3a612220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c2061207b70726f63203a70207b7d207b72657475726e205345434f4e445f534c4f547d7d220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f646973706c61795f6e616d655f726f756e64747269702220736372
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|3|errorinfo|binary-scan-tcl-value|2|4|697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f646973706c61795f6e616d655f726f756e6474726970207b6e616d657370616365206576616c20613a207b70726f632070207b7d207b72657475726e2046495253545f534c4f547d7d3b206e616d657370616365206576616c2061207b70726f63203a2e2e2e220a2020202028226576
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|3|errorinfo|binary-scan-tcl-value|3|4|616c2220626f6479206c696e652032290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f736574757022
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|runtime-cleanup|0|0:0|3|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- name-script-prefix-near-miss, setup, completion 0, result, chunk 0 of 1:

```text-visible
two words⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "two"⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, errorinfo, chunk 0 of 3:

```text-visible
invalid command name "two"
    while executing
"two words {a b}"
    ("eval" body line 1)
    invoked from within
"eval "$head {a b}""
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_name_⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, errorinfo, chunk 1 of 3:

```text-visible
script_prefix_near_miss" script line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad27_name_script_prefix_near_miss {eval "$head {a b}"}"
    ("eval" body line 1)
    invo⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, errorinfo, chunk 2 of 3:

```text-visible
ked from within
"eval $nh_operation"⟦EOF⟧
```

- name-script-prefix-near-miss, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- brace-newline-command, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- brace-newline-command, lookup, completion 0, result, chunk 0 of 1:

```text-visible
a b c⟦EOF⟧
```

- brace-newline-command, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, result, chunk 0 of 1:

```text-visible
can't create procedure ":p" in non-global namespace with name starting with ":"⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorinfo, chunk 0 of 4:

```text-visible
can't create procedure ":p" in non-global namespace with name starting with ":"
    while executing
"proc :p {} {return SECOND_SLOT}"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_displa⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorinfo, chunk 1 of 4:

```text-visible
y_name_roundtrip::a" script line 1)
    invoked from within
"namespace eval a {proc :p {} {return SECOND_SLOT}}"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_display_name_roundtrip" scr⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorinfo, chunk 2 of 4:

```text-visible
ipt line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad27_display_name_roundtrip {namespace eval a: {proc p {} {return FIRST_SLOT}}; namespace eval a {proc :..."
    ("ev⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorinfo, chunk 3 of 4:

```text-visible
al" body line 2)
    invoked from within
"eval $nh_setup"⟦EOF⟧
```

- display-name-roundtrip, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- brace-newline-command: intended operation reached.

- display-name-roundtrip: intended operation not reached.

- name-script-prefix-near-miss: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 4

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|setup|0|0:0|4|result|binary-scan-tcl-value|0|1|74776f20776f726473
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|4|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d65202274776f22
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|4|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|4|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|4|errorinfo|binary-scan-tcl-value|0|3|696e76616c696420636f6d6d616e64206e616d65202274776f220a202020207768696c6520657865637574696e670a2274776f20776f726473207b6120627d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20222468656164207b6120627d22220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d655f
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|4|errorinfo|binary-scan-tcl-value|1|3|7363726970745f7072656669785f6e6561725f6d6973732220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d655f7363726970745f7072656669785f6e6561725f6d697373207b6576616c20222468656164207b6120627d227d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|4|errorinfo|binary-scan-tcl-value|2|3|6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|runtime-cleanup|0|0:0|4|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|brace-newline-command|tmm-traffic-event|setup|0|0:0|4|result|binary-scan-tcl-value|0|1|
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|brace-newline-command|tmm-traffic-event|lookup|0|0:0|4|result|binary-scan-tcl-value|0|1|6120622063
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|brace-newline-command|tmm-traffic-event|runtime-cleanup|0|0:0|4|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|4|result|binary-scan-tcl-value|0|1|63616e2774206372656174652070726f63656475726520223a702220696e206e6f6e2d676c6f62616c206e616d6573706163652077697468206e616d65207374617274696e67207769746820223a22
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|4|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|4|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|4|errorinfo|binary-scan-tcl-value|0|4|63616e2774206372656174652070726f63656475726520223a702220696e206e6f6e2d676c6f62616c206e616d6573706163652077697468206e616d65207374617274696e67207769746820223a220a202020207768696c6520657865637574696e670a2270726f63203a70207b7d207b72657475726e205345434f4e445f534c4f547d220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f646973706c61
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|4|errorinfo|binary-scan-tcl-value|1|4|795f6e616d655f726f756e64747269703a3a612220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c2061207b70726f63203a70207b7d207b72657475726e205345434f4e445f534c4f547d7d220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f646973706c61795f6e616d655f726f756e64747269702220736372
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|4|errorinfo|binary-scan-tcl-value|2|4|697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f646973706c61795f6e616d655f726f756e6474726970207b6e616d657370616365206576616c20613a207b70726f632070207b7d207b72657475726e2046495253545f534c4f547d7d3b206e616d657370616365206576616c2061207b70726f63203a2e2e2e220a2020202028226576
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|4|errorinfo|binary-scan-tcl-value|3|4|616c2220626f6479206c696e652032290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f736574757022
Oct 12 09:00:05 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|runtime-cleanup|0|0:0|4|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- name-script-prefix-near-miss, setup, completion 0, result, chunk 0 of 1:

```text-visible
two words⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "two"⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, errorinfo, chunk 0 of 3:

```text-visible
invalid command name "two"
    while executing
"two words {a b}"
    ("eval" body line 1)
    invoked from within
"eval "$head {a b}""
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_name_⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, errorinfo, chunk 1 of 3:

```text-visible
script_prefix_near_miss" script line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad27_name_script_prefix_near_miss {eval "$head {a b}"}"
    ("eval" body line 1)
    invo⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, errorinfo, chunk 2 of 3:

```text-visible
ked from within
"eval $nh_operation"⟦EOF⟧
```

- name-script-prefix-near-miss, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- brace-newline-command, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- brace-newline-command, lookup, completion 0, result, chunk 0 of 1:

```text-visible
a b c⟦EOF⟧
```

- brace-newline-command, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, result, chunk 0 of 1:

```text-visible
can't create procedure ":p" in non-global namespace with name starting with ":"⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorinfo, chunk 0 of 4:

```text-visible
can't create procedure ":p" in non-global namespace with name starting with ":"
    while executing
"proc :p {} {return SECOND_SLOT}"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_displa⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorinfo, chunk 1 of 4:

```text-visible
y_name_roundtrip::a" script line 1)
    invoked from within
"namespace eval a {proc :p {} {return SECOND_SLOT}}"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_display_name_roundtrip" scr⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorinfo, chunk 2 of 4:

```text-visible
ipt line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad27_display_name_roundtrip {namespace eval a: {proc p {} {return FIRST_SLOT}}; namespace eval a {proc :..."
    ("ev⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorinfo, chunk 3 of 4:

```text-visible
al" body line 2)
    invoked from within
"eval $nh_setup"⟦EOF⟧
```

- display-name-roundtrip, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- brace-newline-command: intended operation reached.

- display-name-roundtrip: intended operation not reached.

- name-script-prefix-near-miss: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 5

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|setup|0|0:0|5|result|binary-scan-tcl-value|0|1|74776f20776f726473
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|5|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d65202274776f22
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|5|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|5|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|5|errorinfo|binary-scan-tcl-value|0|3|696e76616c696420636f6d6d616e64206e616d65202274776f220a202020207768696c6520657865637574696e670a2274776f20776f726473207b6120627d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20222468656164207b6120627d22220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d655f
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|5|errorinfo|binary-scan-tcl-value|1|3|7363726970745f7072656669785f6e6561725f6d6973732220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d655f7363726970745f7072656669785f6e6561725f6d697373207b6576616c20222468656164207b6120627d227d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|5|errorinfo|binary-scan-tcl-value|2|3|6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|runtime-cleanup|0|0:0|5|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|brace-newline-command|tmm-traffic-event|setup|0|0:0|5|result|binary-scan-tcl-value|0|1|
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|brace-newline-command|tmm-traffic-event|lookup|0|0:0|5|result|binary-scan-tcl-value|0|1|6120622063
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|brace-newline-command|tmm-traffic-event|runtime-cleanup|0|0:0|5|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|5|result|binary-scan-tcl-value|0|1|63616e2774206372656174652070726f63656475726520223a702220696e206e6f6e2d676c6f62616c206e616d6573706163652077697468206e616d65207374617274696e67207769746820223a22
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|5|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|5|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|5|errorinfo|binary-scan-tcl-value|0|4|63616e2774206372656174652070726f63656475726520223a702220696e206e6f6e2d676c6f62616c206e616d6573706163652077697468206e616d65207374617274696e67207769746820223a220a202020207768696c6520657865637574696e670a2270726f63203a70207b7d207b72657475726e205345434f4e445f534c4f547d220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f646973706c61
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|5|errorinfo|binary-scan-tcl-value|1|4|795f6e616d655f726f756e64747269703a3a612220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c2061207b70726f63203a70207b7d207b72657475726e205345434f4e445f534c4f547d7d220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f646973706c61795f6e616d655f726f756e64747269702220736372
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|5|errorinfo|binary-scan-tcl-value|2|4|697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f646973706c61795f6e616d655f726f756e6474726970207b6e616d657370616365206576616c20613a207b70726f632070207b7d207b72657475726e2046495253545f534c4f547d7d3b206e616d657370616365206576616c2061207b70726f63203a2e2e2e220a2020202028226576
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|5|errorinfo|binary-scan-tcl-value|3|4|616c2220626f6479206c696e652032290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f736574757022
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|runtime-cleanup|0|0:0|5|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- name-script-prefix-near-miss, setup, completion 0, result, chunk 0 of 1:

```text-visible
two words⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "two"⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, errorinfo, chunk 0 of 3:

```text-visible
invalid command name "two"
    while executing
"two words {a b}"
    ("eval" body line 1)
    invoked from within
"eval "$head {a b}""
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_name_⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, errorinfo, chunk 1 of 3:

```text-visible
script_prefix_near_miss" script line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad27_name_script_prefix_near_miss {eval "$head {a b}"}"
    ("eval" body line 1)
    invo⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, errorinfo, chunk 2 of 3:

```text-visible
ked from within
"eval $nh_operation"⟦EOF⟧
```

- name-script-prefix-near-miss, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- brace-newline-command, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- brace-newline-command, lookup, completion 0, result, chunk 0 of 1:

```text-visible
a b c⟦EOF⟧
```

- brace-newline-command, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, result, chunk 0 of 1:

```text-visible
can't create procedure ":p" in non-global namespace with name starting with ":"⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorinfo, chunk 0 of 4:

```text-visible
can't create procedure ":p" in non-global namespace with name starting with ":"
    while executing
"proc :p {} {return SECOND_SLOT}"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_displa⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorinfo, chunk 1 of 4:

```text-visible
y_name_roundtrip::a" script line 1)
    invoked from within
"namespace eval a {proc :p {} {return SECOND_SLOT}}"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_display_name_roundtrip" scr⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorinfo, chunk 2 of 4:

```text-visible
ipt line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad27_display_name_roundtrip {namespace eval a: {proc p {} {return FIRST_SLOT}}; namespace eval a {proc :..."
    ("ev⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorinfo, chunk 3 of 4:

```text-visible
al" body line 2)
    invoked from within
"eval $nh_setup"⟦EOF⟧
```

- display-name-roundtrip, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- brace-newline-command: intended operation reached.

- display-name-roundtrip: intended operation not reached.

- name-script-prefix-near-miss: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 6

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|setup|0|0:0|6|result|binary-scan-tcl-value|0|1|74776f20776f726473
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|6|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d65202274776f22
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|6|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|6|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|6|errorinfo|binary-scan-tcl-value|0|3|696e76616c696420636f6d6d616e64206e616d65202274776f220a202020207768696c6520657865637574696e670a2274776f20776f726473207b6120627d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20222468656164207b6120627d22220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d655f
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|6|errorinfo|binary-scan-tcl-value|1|3|7363726970745f7072656669785f6e6561725f6d6973732220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d655f7363726970745f7072656669785f6e6561725f6d697373207b6576616c20222468656164207b6120627d227d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|6|errorinfo|binary-scan-tcl-value|2|3|6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|runtime-cleanup|0|0:0|6|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|brace-newline-command|tmm-traffic-event|setup|0|0:0|6|result|binary-scan-tcl-value|0|1|
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|brace-newline-command|tmm-traffic-event|lookup|0|0:0|6|result|binary-scan-tcl-value|0|1|6120622063
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|brace-newline-command|tmm-traffic-event|runtime-cleanup|0|0:0|6|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|6|result|binary-scan-tcl-value|0|1|63616e2774206372656174652070726f63656475726520223a702220696e206e6f6e2d676c6f62616c206e616d6573706163652077697468206e616d65207374617274696e67207769746820223a22
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|6|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|6|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|6|errorinfo|binary-scan-tcl-value|0|4|63616e2774206372656174652070726f63656475726520223a702220696e206e6f6e2d676c6f62616c206e616d6573706163652077697468206e616d65207374617274696e67207769746820223a220a202020207768696c6520657865637574696e670a2270726f63203a70207b7d207b72657475726e205345434f4e445f534c4f547d220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f646973706c61
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|6|errorinfo|binary-scan-tcl-value|1|4|795f6e616d655f726f756e64747269703a3a612220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c2061207b70726f63203a70207b7d207b72657475726e205345434f4e445f534c4f547d7d220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f646973706c61795f6e616d655f726f756e64747269702220736372
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|6|errorinfo|binary-scan-tcl-value|2|4|697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f646973706c61795f6e616d655f726f756e6474726970207b6e616d657370616365206576616c20613a207b70726f632070207b7d207b72657475726e2046495253545f534c4f547d7d3b206e616d657370616365206576616c2061207b70726f63203a2e2e2e220a2020202028226576
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|6|errorinfo|binary-scan-tcl-value|3|4|616c2220626f6479206c696e652032290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f736574757022
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|runtime-cleanup|0|0:0|6|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- name-script-prefix-near-miss, setup, completion 0, result, chunk 0 of 1:

```text-visible
two words⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "two"⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, errorinfo, chunk 0 of 3:

```text-visible
invalid command name "two"
    while executing
"two words {a b}"
    ("eval" body line 1)
    invoked from within
"eval "$head {a b}""
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_name_⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, errorinfo, chunk 1 of 3:

```text-visible
script_prefix_near_miss" script line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad27_name_script_prefix_near_miss {eval "$head {a b}"}"
    ("eval" body line 1)
    invo⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, errorinfo, chunk 2 of 3:

```text-visible
ked from within
"eval $nh_operation"⟦EOF⟧
```

- name-script-prefix-near-miss, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- brace-newline-command, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- brace-newline-command, lookup, completion 0, result, chunk 0 of 1:

```text-visible
a b c⟦EOF⟧
```

- brace-newline-command, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, result, chunk 0 of 1:

```text-visible
can't create procedure ":p" in non-global namespace with name starting with ":"⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorinfo, chunk 0 of 4:

```text-visible
can't create procedure ":p" in non-global namespace with name starting with ":"
    while executing
"proc :p {} {return SECOND_SLOT}"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_displa⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorinfo, chunk 1 of 4:

```text-visible
y_name_roundtrip::a" script line 1)
    invoked from within
"namespace eval a {proc :p {} {return SECOND_SLOT}}"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_display_name_roundtrip" scr⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorinfo, chunk 2 of 4:

```text-visible
ipt line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad27_display_name_roundtrip {namespace eval a: {proc p {} {return FIRST_SLOT}}; namespace eval a {proc :..."
    ("ev⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorinfo, chunk 3 of 4:

```text-visible
al" body line 2)
    invoked from within
"eval $nh_setup"⟦EOF⟧
```

- display-name-roundtrip, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- brace-newline-command: intended operation reached.

- display-name-roundtrip: intended operation not reached.

- name-script-prefix-near-miss: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 7

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|setup|0|0:0|7|result|binary-scan-tcl-value|0|1|74776f20776f726473
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|7|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d65202274776f22
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|7|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|7|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|7|errorinfo|binary-scan-tcl-value|0|3|696e76616c696420636f6d6d616e64206e616d65202274776f220a202020207768696c6520657865637574696e670a2274776f20776f726473207b6120627d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20222468656164207b6120627d22220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d655f
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|7|errorinfo|binary-scan-tcl-value|1|3|7363726970745f7072656669785f6e6561725f6d6973732220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d655f7363726970745f7072656669785f6e6561725f6d697373207b6576616c20222468656164207b6120627d227d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|7|errorinfo|binary-scan-tcl-value|2|3|6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|runtime-cleanup|0|0:0|7|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|brace-newline-command|tmm-traffic-event|setup|0|0:0|7|result|binary-scan-tcl-value|0|1|
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|brace-newline-command|tmm-traffic-event|lookup|0|0:0|7|result|binary-scan-tcl-value|0|1|6120622063
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|brace-newline-command|tmm-traffic-event|runtime-cleanup|0|0:0|7|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|7|result|binary-scan-tcl-value|0|1|63616e2774206372656174652070726f63656475726520223a702220696e206e6f6e2d676c6f62616c206e616d6573706163652077697468206e616d65207374617274696e67207769746820223a22
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|7|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|7|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|7|errorinfo|binary-scan-tcl-value|0|4|63616e2774206372656174652070726f63656475726520223a702220696e206e6f6e2d676c6f62616c206e616d6573706163652077697468206e616d65207374617274696e67207769746820223a220a202020207768696c6520657865637574696e670a2270726f63203a70207b7d207b72657475726e205345434f4e445f534c4f547d220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f646973706c61
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|7|errorinfo|binary-scan-tcl-value|1|4|795f6e616d655f726f756e64747269703a3a612220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c2061207b70726f63203a70207b7d207b72657475726e205345434f4e445f534c4f547d7d220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f646973706c61795f6e616d655f726f756e64747269702220736372
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|7|errorinfo|binary-scan-tcl-value|2|4|697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f646973706c61795f6e616d655f726f756e6474726970207b6e616d657370616365206576616c20613a207b70726f632070207b7d207b72657475726e2046495253545f534c4f547d7d3b206e616d657370616365206576616c2061207b70726f63203a2e2e2e220a2020202028226576
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|7|errorinfo|binary-scan-tcl-value|3|4|616c2220626f6479206c696e652032290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f736574757022
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|runtime-cleanup|0|0:0|7|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- name-script-prefix-near-miss, setup, completion 0, result, chunk 0 of 1:

```text-visible
two words⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "two"⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, errorinfo, chunk 0 of 3:

```text-visible
invalid command name "two"
    while executing
"two words {a b}"
    ("eval" body line 1)
    invoked from within
"eval "$head {a b}""
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_name_⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, errorinfo, chunk 1 of 3:

```text-visible
script_prefix_near_miss" script line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad27_name_script_prefix_near_miss {eval "$head {a b}"}"
    ("eval" body line 1)
    invo⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, errorinfo, chunk 2 of 3:

```text-visible
ked from within
"eval $nh_operation"⟦EOF⟧
```

- name-script-prefix-near-miss, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- brace-newline-command, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- brace-newline-command, lookup, completion 0, result, chunk 0 of 1:

```text-visible
a b c⟦EOF⟧
```

- brace-newline-command, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, result, chunk 0 of 1:

```text-visible
can't create procedure ":p" in non-global namespace with name starting with ":"⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorinfo, chunk 0 of 4:

```text-visible
can't create procedure ":p" in non-global namespace with name starting with ":"
    while executing
"proc :p {} {return SECOND_SLOT}"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_displa⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorinfo, chunk 1 of 4:

```text-visible
y_name_roundtrip::a" script line 1)
    invoked from within
"namespace eval a {proc :p {} {return SECOND_SLOT}}"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_display_name_roundtrip" scr⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorinfo, chunk 2 of 4:

```text-visible
ipt line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad27_display_name_roundtrip {namespace eval a: {proc p {} {return FIRST_SLOT}}; namespace eval a {proc :..."
    ("ev⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorinfo, chunk 3 of 4:

```text-visible
al" body line 2)
    invoked from within
"eval $nh_setup"⟦EOF⟧
```

- display-name-roundtrip, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- brace-newline-command: intended operation reached.

- display-name-roundtrip: intended operation not reached.

- name-script-prefix-near-miss: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 8

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|setup|0|0:0|8|result|binary-scan-tcl-value|0|1|74776f20776f726473
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|8|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d65202274776f22
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|8|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|8|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|8|errorinfo|binary-scan-tcl-value|0|3|696e76616c696420636f6d6d616e64206e616d65202274776f220a202020207768696c6520657865637574696e670a2274776f20776f726473207b6120627d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20222468656164207b6120627d22220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d655f
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|8|errorinfo|binary-scan-tcl-value|1|3|7363726970745f7072656669785f6e6561725f6d6973732220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d655f7363726970745f7072656669785f6e6561725f6d697373207b6576616c20222468656164207b6120627d227d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|8|errorinfo|binary-scan-tcl-value|2|3|6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|runtime-cleanup|0|0:0|8|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|brace-newline-command|tmm-traffic-event|setup|0|0:0|8|result|binary-scan-tcl-value|0|1|
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|brace-newline-command|tmm-traffic-event|lookup|0|0:0|8|result|binary-scan-tcl-value|0|1|6120622063
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|brace-newline-command|tmm-traffic-event|runtime-cleanup|0|0:0|8|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|8|result|binary-scan-tcl-value|0|1|63616e2774206372656174652070726f63656475726520223a702220696e206e6f6e2d676c6f62616c206e616d6573706163652077697468206e616d65207374617274696e67207769746820223a22
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|8|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|8|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|8|errorinfo|binary-scan-tcl-value|0|4|63616e2774206372656174652070726f63656475726520223a702220696e206e6f6e2d676c6f62616c206e616d6573706163652077697468206e616d65207374617274696e67207769746820223a220a202020207768696c6520657865637574696e670a2270726f63203a70207b7d207b72657475726e205345434f4e445f534c4f547d220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f646973706c61
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|8|errorinfo|binary-scan-tcl-value|1|4|795f6e616d655f726f756e64747269703a3a612220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c2061207b70726f63203a70207b7d207b72657475726e205345434f4e445f534c4f547d7d220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f646973706c61795f6e616d655f726f756e64747269702220736372
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|8|errorinfo|binary-scan-tcl-value|2|4|697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f646973706c61795f6e616d655f726f756e6474726970207b6e616d657370616365206576616c20613a207b70726f632070207b7d207b72657475726e2046495253545f534c4f547d7d3b206e616d657370616365206576616c2061207b70726f63203a2e2e2e220a2020202028226576
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|8|errorinfo|binary-scan-tcl-value|3|4|616c2220626f6479206c696e652032290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f736574757022
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|runtime-cleanup|0|0:0|8|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- name-script-prefix-near-miss, setup, completion 0, result, chunk 0 of 1:

```text-visible
two words⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "two"⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, errorinfo, chunk 0 of 3:

```text-visible
invalid command name "two"
    while executing
"two words {a b}"
    ("eval" body line 1)
    invoked from within
"eval "$head {a b}""
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_name_⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, errorinfo, chunk 1 of 3:

```text-visible
script_prefix_near_miss" script line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad27_name_script_prefix_near_miss {eval "$head {a b}"}"
    ("eval" body line 1)
    invo⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, errorinfo, chunk 2 of 3:

```text-visible
ked from within
"eval $nh_operation"⟦EOF⟧
```

- name-script-prefix-near-miss, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- brace-newline-command, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- brace-newline-command, lookup, completion 0, result, chunk 0 of 1:

```text-visible
a b c⟦EOF⟧
```

- brace-newline-command, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, result, chunk 0 of 1:

```text-visible
can't create procedure ":p" in non-global namespace with name starting with ":"⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorinfo, chunk 0 of 4:

```text-visible
can't create procedure ":p" in non-global namespace with name starting with ":"
    while executing
"proc :p {} {return SECOND_SLOT}"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_displa⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorinfo, chunk 1 of 4:

```text-visible
y_name_roundtrip::a" script line 1)
    invoked from within
"namespace eval a {proc :p {} {return SECOND_SLOT}}"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_display_name_roundtrip" scr⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorinfo, chunk 2 of 4:

```text-visible
ipt line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad27_display_name_roundtrip {namespace eval a: {proc p {} {return FIRST_SLOT}}; namespace eval a {proc :..."
    ("ev⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorinfo, chunk 3 of 4:

```text-visible
al" body line 2)
    invoked from within
"eval $nh_setup"⟦EOF⟧
```

- display-name-roundtrip, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- brace-newline-command: intended operation reached.

- display-name-roundtrip: intended operation not reached.

- name-script-prefix-near-miss: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 9

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|setup|0|0:0|9|result|binary-scan-tcl-value|0|1|74776f20776f726473
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|9|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d65202274776f22
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|9|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|9|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|9|errorinfo|binary-scan-tcl-value|0|3|696e76616c696420636f6d6d616e64206e616d65202274776f220a202020207768696c6520657865637574696e670a2274776f20776f726473207b6120627d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20222468656164207b6120627d22220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d655f
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|9|errorinfo|binary-scan-tcl-value|1|3|7363726970745f7072656669785f6e6561725f6d6973732220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f6e616d655f7363726970745f7072656669785f6e6561725f6d697373207b6576616c20222468656164207b6120627d227d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|lookup|1|0:0|9|errorinfo|binary-scan-tcl-value|2|3|6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|name-script-prefix-near-miss|tmm-traffic-event|runtime-cleanup|0|0:0|9|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|brace-newline-command|tmm-traffic-event|setup|0|0:0|9|result|binary-scan-tcl-value|0|1|
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|brace-newline-command|tmm-traffic-event|lookup|0|0:0|9|result|binary-scan-tcl-value|0|1|6120622063
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|brace-newline-command|tmm-traffic-event|runtime-cleanup|0|0:0|9|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|9|result|binary-scan-tcl-value|0|1|63616e2774206372656174652070726f63656475726520223a702220696e206e6f6e2d676c6f62616c206e616d6573706163652077697468206e616d65207374617274696e67207769746820223a22
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|9|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|9|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|9|errorinfo|binary-scan-tcl-value|0|4|63616e2774206372656174652070726f63656475726520223a702220696e206e6f6e2d676c6f62616c206e616d6573706163652077697468206e616d65207374617274696e67207769746820223a220a202020207768696c6520657865637574696e670a2270726f63203a70207b7d207b72657475726e205345434f4e445f534c4f547d220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f646973706c61
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|9|errorinfo|binary-scan-tcl-value|1|4|795f6e616d655f726f756e64747269703a3a612220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c2061207b70726f63203a70207b7d207b72657475726e205345434f4e445f534c4f547d7d220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f646973706c61795f6e616d655f726f756e64747269702220736372
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|9|errorinfo|binary-scan-tcl-value|2|4|697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f646973706c61795f6e616d655f726f756e6474726970207b6e616d657370616365206576616c20613a207b70726f632070207b7d207b72657475726e2046495253545f534c4f547d7d3b206e616d657370616365206576616c2061207b70726f63203a2e2e2e220a2020202028226576
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|setup|1|0:0|9|errorinfo|binary-scan-tcl-value|3|4|616c2220626f6479206c696e652032290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f736574757022
Oct 12 09:00:06 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_20 <HTTP_REQUEST>: NHO|nh20261012broad2|display-name-roundtrip|tmm-traffic-event|runtime-cleanup|0|0:0|9|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- name-script-prefix-near-miss, setup, completion 0, result, chunk 0 of 1:

```text-visible
two words⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "two"⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, errorinfo, chunk 0 of 3:

```text-visible
invalid command name "two"
    while executing
"two words {a b}"
    ("eval" body line 1)
    invoked from within
"eval "$head {a b}""
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_name_⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, errorinfo, chunk 1 of 3:

```text-visible
script_prefix_near_miss" script line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad27_name_script_prefix_near_miss {eval "$head {a b}"}"
    ("eval" body line 1)
    invo⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, errorinfo, chunk 2 of 3:

```text-visible
ked from within
"eval $nh_operation"⟦EOF⟧
```

- name-script-prefix-near-miss, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- brace-newline-command, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- brace-newline-command, lookup, completion 0, result, chunk 0 of 1:

```text-visible
a b c⟦EOF⟧
```

- brace-newline-command, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, result, chunk 0 of 1:

```text-visible
can't create procedure ":p" in non-global namespace with name starting with ":"⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorinfo, chunk 0 of 4:

```text-visible
can't create procedure ":p" in non-global namespace with name starting with ":"
    while executing
"proc :p {} {return SECOND_SLOT}"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_displa⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorinfo, chunk 1 of 4:

```text-visible
y_name_roundtrip::a" script line 1)
    invoked from within
"namespace eval a {proc :p {} {return SECOND_SLOT}}"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_display_name_roundtrip" scr⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorinfo, chunk 2 of 4:

```text-visible
ipt line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad27_display_name_roundtrip {namespace eval a: {proc p {} {return FIRST_SLOT}}; namespace eval a {proc :..."
    ("ev⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorinfo, chunk 3 of 4:

```text-visible
al" body line 2)
    invoked from within
"eval $nh_setup"⟦EOF⟧
```

- display-name-roundtrip, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- brace-newline-command: intended operation reached.

- display-name-roundtrip: intended operation not reached.

- name-script-prefix-near-miss: intended operation reached.
