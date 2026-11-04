# BIG-IP Tcl log: nh20261012broad2, tmm-traffic-event-7

## Question

The following cases investigate Tcl behavior relevant to name resolution and diagnostics in the LSP, compiler and VM. Each observation is limited to its own case and recorded context.

- **global-declaration-link:** `global`, `variable`, `upvar 0/1/#0`, `namespace upvar`, eval/uplevel composition: exact target frame and cell, including namespace-eval callers.

- **variable-alternating-arguments:** `global`, `variable`, `upvar 0/1/#0`, `namespace upvar`, eval/uplevel composition: exact target frame and cell, including namespace-eval callers.

- **variable-declaration-link:** `global`, `variable`, `upvar 0/1/#0`, `namespace upvar`, eval/uplevel composition: exact target frame and cell, including namespace-eval callers.

## Assumptions

- **global-declaration-link:** Unique global declaration/link; clean global variable ::__tcl_lsp_probe_nh20261012broad27_global_declaration_link_x.
- **variable-alternating-arguments:** Single variable declaration versus name/value alternation.
- **variable-declaration-link:** Namespace variable declaration in proc.

Other assumptions were not separately recorded.

## Script

```tcl-visible
set nh_options_available unavailable
set nh_return_options {}
set nh_unit "[TMM::cmp_group]:[TMM::cmp_unit]"
set nh_invocation once
set nh_invocation [HTTP::header value X-NH-Invocation]
set nh_setup [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f676c6f62616c5f6465636c61726174696f6e5f6c696e6b207b7d0a736574203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f676c6f62616c5f6465636c61726174696f6e5f6c696e6b5f7820474c4f42414c0a6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f676c6f62616c5f6465636c61726174696f6e5f6c696e6b207b70726f632072756e207b7d207b676c6f62616c205f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f676c6f62616c5f6465636c61726174696f6e5f6c696e6b5f783b20736574205f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f676c6f62616c5f6465636c61726174696f6e5f6c696e6b5f78204348414e4745447d7d0a]
set nh_operation [binary format H* 3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f676c6f62616c5f6465636c61726174696f6e5f6c696e6b3a3a72756e3b20736574203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f676c6f62616c5f6465636c61726174696f6e5f6c696e6b5f780a]
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
        log local0. "NHO|nh20261012broad2|global-declaration-link|tmm-traffic-event|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012broad2|global-declaration-link|tmm-traffic-event|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 666f7265616368206e685f6368696c64205b696e7465727020736c617665735d207b6966207b5b737472696e67206d61746368205f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f676c6f62616c5f6465636c61726174696f6e5f6c696e6b2a20246e685f6368696c645d7d207b696e746572702064656c65746520246e685f6368696c647d7d0a666f7265616368206e685f6e73205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f676c6f62616c5f6465636c61726174696f6e5f6c696e6b2a5d207b6e616d6573706163652064656c65746520246e685f6e737d0a666f7265616368206e685f636d64205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f676c6f62616c5f6465636c61726174696f6e5f6c696e6b2a5d207b72656e616d6520246e685f636d64207b7d7d0a666f7265616368206e685f766172205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f676c6f62616c5f6465636c61726174696f6e5f6c696e6b2a5d207b756e736574202d6e6f636f6d706c61696e20246e685f7661727d0a6c697374205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f676c6f62616c5f6465636c61726174696f6e5f6c696e6b2a5d205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f676c6f62616c5f6465636c61726174696f6e5f6c696e6b2a5d205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f676c6f62616c5f6465636c61726174696f6e5f6c696e6b2a5d]
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
        log local0. "NHO|nh20261012broad2|global-declaration-link|tmm-traffic-event|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
set nh_setup [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f7661726961626c655f6465636c61726174696f6e5f6c696e6b207b7d0a6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f7661726961626c655f6465636c61726174696f6e5f6c696e6b207b7661726961626c652078204e414d4553504143453b2070726f632072756e207b7d207b7661726961626c6520783b207365742078204348414e4745447d7d0a]
set nh_operation [binary format H* 3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f7661726961626c655f6465636c61726174696f6e5f6c696e6b3a3a72756e3b20736574203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f7661726961626c655f6465636c61726174696f6e5f6c696e6b3a3a780a]
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
        log local0. "NHO|nh20261012broad2|variable-declaration-link|tmm-traffic-event|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012broad2|variable-declaration-link|tmm-traffic-event|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 666f7265616368206e685f6368696c64205b696e7465727020736c617665735d207b6966207b5b737472696e67206d61746368205f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f7661726961626c655f6465636c61726174696f6e5f6c696e6b2a20246e685f6368696c645d7d207b696e746572702064656c65746520246e685f6368696c647d7d0a666f7265616368206e685f6e73205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f7661726961626c655f6465636c61726174696f6e5f6c696e6b2a5d207b6e616d6573706163652064656c65746520246e685f6e737d0a666f7265616368206e685f636d64205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f7661726961626c655f6465636c61726174696f6e5f6c696e6b2a5d207b72656e616d6520246e685f636d64207b7d7d0a666f7265616368206e685f766172205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f7661726961626c655f6465636c61726174696f6e5f6c696e6b2a5d207b756e736574202d6e6f636f6d706c61696e20246e685f7661727d0a6c697374205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f7661726961626c655f6465636c61726174696f6e5f6c696e6b2a5d205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f7661726961626c655f6465636c61726174696f6e5f6c696e6b2a5d205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f7661726961626c655f6465636c61726174696f6e5f6c696e6b2a5d]
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
        log local0. "NHO|nh20261012broad2|variable-declaration-link|tmm-traffic-event|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
set nh_setup [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f7661726961626c655f616c7465726e6174696e675f617267756d656e7473207b7d0a6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f7661726961626c655f616c7465726e6174696e675f617267756d656e7473207b7d0a]
set nh_operation [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f7661726961626c655f616c7465726e6174696e675f617267756d656e7473207b7661726961626c65206120623b206c697374202461205b696e666f2065786973747320625d7d0a]
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
        log local0. "NHO|nh20261012broad2|variable-alternating-arguments|tmm-traffic-event|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012broad2|variable-alternating-arguments|tmm-traffic-event|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 666f7265616368206e685f6368696c64205b696e7465727020736c617665735d207b6966207b5b737472696e67206d61746368205f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f7661726961626c655f616c7465726e6174696e675f617267756d656e74732a20246e685f6368696c645d7d207b696e746572702064656c65746520246e685f6368696c647d7d0a666f7265616368206e685f6e73205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f7661726961626c655f616c7465726e6174696e675f617267756d656e74732a5d207b6e616d6573706163652064656c65746520246e685f6e737d0a666f7265616368206e685f636d64205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f7661726961626c655f616c7465726e6174696e675f617267756d656e74732a5d207b72656e616d6520246e685f636d64207b7d7d0a666f7265616368206e685f766172205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f7661726961626c655f616c7465726e6174696e675f617267756d656e74732a5d207b756e736574202d6e6f636f6d706c61696e20246e685f7661727d0a6c697374205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f7661726961626c655f616c7465726e6174696e675f617267756d656e74732a5d205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f7661726961626c655f616c7465726e6174696e675f617267756d656e74732a5d205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616432375f7661726961626c655f616c7465726e6174696e675f617267756d656e74732a5d]
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
        log local0. "NHO|nh20261012broad2|variable-alternating-arguments|tmm-traffic-event|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
Oct 12 08:57:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|global-declaration-link|tmm-traffic-event|setup|0|0:0|0|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|global-declaration-link|tmm-traffic-event|lookup|0|0:0|0|result|binary-scan-tcl-value|0|1|4348414e474544
Oct 12 08:57:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|global-declaration-link|tmm-traffic-event|runtime-cleanup|0|0:0|0|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:57:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-declaration-link|tmm-traffic-event|setup|0|0:0|0|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-declaration-link|tmm-traffic-event|lookup|0|0:0|0|result|binary-scan-tcl-value|0|1|4348414e474544
Oct 12 08:57:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-declaration-link|tmm-traffic-event|runtime-cleanup|0|0:0|0|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:57:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-alternating-arguments|tmm-traffic-event|setup|0|0:0|0|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-alternating-arguments|tmm-traffic-event|lookup|0|0:0|0|result|binary-scan-tcl-value|0|1|622030
Oct 12 08:57:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-alternating-arguments|tmm-traffic-event|runtime-cleanup|0|0:0|0|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- global-declaration-link, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- global-declaration-link, lookup, completion 0, result, chunk 0 of 1:

```text-visible
CHANGED⟦EOF⟧
```

- global-declaration-link, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- variable-declaration-link, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- variable-declaration-link, lookup, completion 0, result, chunk 0 of 1:

```text-visible
CHANGED⟦EOF⟧
```

- variable-declaration-link, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- variable-alternating-arguments, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- variable-alternating-arguments, lookup, completion 0, result, chunk 0 of 1:

```text-visible
b 0⟦EOF⟧
```

- variable-alternating-arguments, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- global-declaration-link: intended operation reached.

- variable-alternating-arguments: intended operation reached.

- variable-declaration-link: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 1

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:57:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|global-declaration-link|tmm-traffic-event|setup|0|0:0|1|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|global-declaration-link|tmm-traffic-event|lookup|0|0:0|1|result|binary-scan-tcl-value|0|1|4348414e474544
Oct 12 08:57:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|global-declaration-link|tmm-traffic-event|runtime-cleanup|0|0:0|1|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:57:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-declaration-link|tmm-traffic-event|setup|0|0:0|1|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-declaration-link|tmm-traffic-event|lookup|0|0:0|1|result|binary-scan-tcl-value|0|1|4348414e474544
Oct 12 08:57:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-declaration-link|tmm-traffic-event|runtime-cleanup|0|0:0|1|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:57:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-alternating-arguments|tmm-traffic-event|setup|0|0:0|1|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-alternating-arguments|tmm-traffic-event|lookup|0|0:0|1|result|binary-scan-tcl-value|0|1|622030
Oct 12 08:57:32 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-alternating-arguments|tmm-traffic-event|runtime-cleanup|0|0:0|1|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- global-declaration-link, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- global-declaration-link, lookup, completion 0, result, chunk 0 of 1:

```text-visible
CHANGED⟦EOF⟧
```

- global-declaration-link, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- variable-declaration-link, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- variable-declaration-link, lookup, completion 0, result, chunk 0 of 1:

```text-visible
CHANGED⟦EOF⟧
```

- variable-declaration-link, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- variable-alternating-arguments, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- variable-alternating-arguments, lookup, completion 0, result, chunk 0 of 1:

```text-visible
b 0⟦EOF⟧
```

- variable-alternating-arguments, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- global-declaration-link: intended operation reached.

- variable-alternating-arguments: intended operation reached.

- variable-declaration-link: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 10

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:57:34 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|global-declaration-link|tmm-traffic-event|setup|0|0:0|10|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:34 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|global-declaration-link|tmm-traffic-event|lookup|0|0:0|10|result|binary-scan-tcl-value|0|1|4348414e474544
Oct 12 08:57:34 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|global-declaration-link|tmm-traffic-event|runtime-cleanup|0|0:0|10|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:57:34 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-declaration-link|tmm-traffic-event|setup|0|0:0|10|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:34 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-declaration-link|tmm-traffic-event|lookup|0|0:0|10|result|binary-scan-tcl-value|0|1|4348414e474544
Oct 12 08:57:34 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-declaration-link|tmm-traffic-event|runtime-cleanup|0|0:0|10|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:57:34 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-alternating-arguments|tmm-traffic-event|setup|0|0:0|10|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:34 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-alternating-arguments|tmm-traffic-event|lookup|0|0:0|10|result|binary-scan-tcl-value|0|1|622030
Oct 12 08:57:34 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-alternating-arguments|tmm-traffic-event|runtime-cleanup|0|0:0|10|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- global-declaration-link, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- global-declaration-link, lookup, completion 0, result, chunk 0 of 1:

```text-visible
CHANGED⟦EOF⟧
```

- global-declaration-link, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- variable-declaration-link, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- variable-declaration-link, lookup, completion 0, result, chunk 0 of 1:

```text-visible
CHANGED⟦EOF⟧
```

- variable-declaration-link, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- variable-alternating-arguments, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- variable-alternating-arguments, lookup, completion 0, result, chunk 0 of 1:

```text-visible
b 0⟦EOF⟧
```

- variable-alternating-arguments, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- global-declaration-link: intended operation reached.

- variable-alternating-arguments: intended operation reached.

- variable-declaration-link: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 11

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:57:34 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|global-declaration-link|tmm-traffic-event|setup|0|0:0|11|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:34 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|global-declaration-link|tmm-traffic-event|lookup|0|0:0|11|result|binary-scan-tcl-value|0|1|4348414e474544
Oct 12 08:57:34 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|global-declaration-link|tmm-traffic-event|runtime-cleanup|0|0:0|11|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:57:34 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-declaration-link|tmm-traffic-event|setup|0|0:0|11|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:34 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-declaration-link|tmm-traffic-event|lookup|0|0:0|11|result|binary-scan-tcl-value|0|1|4348414e474544
Oct 12 08:57:34 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-declaration-link|tmm-traffic-event|runtime-cleanup|0|0:0|11|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:57:34 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-alternating-arguments|tmm-traffic-event|setup|0|0:0|11|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:34 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-alternating-arguments|tmm-traffic-event|lookup|0|0:0|11|result|binary-scan-tcl-value|0|1|622030
Oct 12 08:57:34 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-alternating-arguments|tmm-traffic-event|runtime-cleanup|0|0:0|11|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- global-declaration-link, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- global-declaration-link, lookup, completion 0, result, chunk 0 of 1:

```text-visible
CHANGED⟦EOF⟧
```

- global-declaration-link, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- variable-declaration-link, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- variable-declaration-link, lookup, completion 0, result, chunk 0 of 1:

```text-visible
CHANGED⟦EOF⟧
```

- variable-declaration-link, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- variable-alternating-arguments, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- variable-alternating-arguments, lookup, completion 0, result, chunk 0 of 1:

```text-visible
b 0⟦EOF⟧
```

- variable-alternating-arguments, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- global-declaration-link: intended operation reached.

- variable-alternating-arguments: intended operation reached.

- variable-declaration-link: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 12

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:57:34 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|global-declaration-link|tmm-traffic-event|setup|0|0:0|12|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:34 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|global-declaration-link|tmm-traffic-event|lookup|0|0:0|12|result|binary-scan-tcl-value|0|1|4348414e474544
Oct 12 08:57:34 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|global-declaration-link|tmm-traffic-event|runtime-cleanup|0|0:0|12|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:57:34 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-declaration-link|tmm-traffic-event|setup|0|0:0|12|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:34 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-declaration-link|tmm-traffic-event|lookup|0|0:0|12|result|binary-scan-tcl-value|0|1|4348414e474544
Oct 12 08:57:34 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-declaration-link|tmm-traffic-event|runtime-cleanup|0|0:0|12|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:57:34 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-alternating-arguments|tmm-traffic-event|setup|0|0:0|12|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:34 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-alternating-arguments|tmm-traffic-event|lookup|0|0:0|12|result|binary-scan-tcl-value|0|1|622030
Oct 12 08:57:34 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-alternating-arguments|tmm-traffic-event|runtime-cleanup|0|0:0|12|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- global-declaration-link, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- global-declaration-link, lookup, completion 0, result, chunk 0 of 1:

```text-visible
CHANGED⟦EOF⟧
```

- global-declaration-link, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- variable-declaration-link, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- variable-declaration-link, lookup, completion 0, result, chunk 0 of 1:

```text-visible
CHANGED⟦EOF⟧
```

- variable-declaration-link, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- variable-alternating-arguments, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- variable-alternating-arguments, lookup, completion 0, result, chunk 0 of 1:

```text-visible
b 0⟦EOF⟧
```

- variable-alternating-arguments, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- global-declaration-link: intended operation reached.

- variable-alternating-arguments: intended operation reached.

- variable-declaration-link: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 13

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:57:34 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|global-declaration-link|tmm-traffic-event|setup|0|0:0|13|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:34 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|global-declaration-link|tmm-traffic-event|lookup|0|0:0|13|result|binary-scan-tcl-value|0|1|4348414e474544
Oct 12 08:57:34 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|global-declaration-link|tmm-traffic-event|runtime-cleanup|0|0:0|13|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:57:34 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-declaration-link|tmm-traffic-event|setup|0|0:0|13|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:34 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-declaration-link|tmm-traffic-event|lookup|0|0:0|13|result|binary-scan-tcl-value|0|1|4348414e474544
Oct 12 08:57:34 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-declaration-link|tmm-traffic-event|runtime-cleanup|0|0:0|13|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:57:34 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-alternating-arguments|tmm-traffic-event|setup|0|0:0|13|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:34 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-alternating-arguments|tmm-traffic-event|lookup|0|0:0|13|result|binary-scan-tcl-value|0|1|622030
Oct 12 08:57:34 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-alternating-arguments|tmm-traffic-event|runtime-cleanup|0|0:0|13|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- global-declaration-link, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- global-declaration-link, lookup, completion 0, result, chunk 0 of 1:

```text-visible
CHANGED⟦EOF⟧
```

- global-declaration-link, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- variable-declaration-link, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- variable-declaration-link, lookup, completion 0, result, chunk 0 of 1:

```text-visible
CHANGED⟦EOF⟧
```

- variable-declaration-link, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- variable-alternating-arguments, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- variable-alternating-arguments, lookup, completion 0, result, chunk 0 of 1:

```text-visible
b 0⟦EOF⟧
```

- variable-alternating-arguments, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- global-declaration-link: intended operation reached.

- variable-alternating-arguments: intended operation reached.

- variable-declaration-link: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 14

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:57:34 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|global-declaration-link|tmm-traffic-event|setup|0|0:0|14|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:34 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|global-declaration-link|tmm-traffic-event|lookup|0|0:0|14|result|binary-scan-tcl-value|0|1|4348414e474544
Oct 12 08:57:34 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|global-declaration-link|tmm-traffic-event|runtime-cleanup|0|0:0|14|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:57:34 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-declaration-link|tmm-traffic-event|setup|0|0:0|14|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:34 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-declaration-link|tmm-traffic-event|lookup|0|0:0|14|result|binary-scan-tcl-value|0|1|4348414e474544
Oct 12 08:57:34 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-declaration-link|tmm-traffic-event|runtime-cleanup|0|0:0|14|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:57:34 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-alternating-arguments|tmm-traffic-event|setup|0|0:0|14|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:34 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-alternating-arguments|tmm-traffic-event|lookup|0|0:0|14|result|binary-scan-tcl-value|0|1|622030
Oct 12 08:57:34 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-alternating-arguments|tmm-traffic-event|runtime-cleanup|0|0:0|14|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- global-declaration-link, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- global-declaration-link, lookup, completion 0, result, chunk 0 of 1:

```text-visible
CHANGED⟦EOF⟧
```

- global-declaration-link, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- variable-declaration-link, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- variable-declaration-link, lookup, completion 0, result, chunk 0 of 1:

```text-visible
CHANGED⟦EOF⟧
```

- variable-declaration-link, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- variable-alternating-arguments, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- variable-alternating-arguments, lookup, completion 0, result, chunk 0 of 1:

```text-visible
b 0⟦EOF⟧
```

- variable-alternating-arguments, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- global-declaration-link: intended operation reached.

- variable-alternating-arguments: intended operation reached.

- variable-declaration-link: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 15

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:57:34 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|global-declaration-link|tmm-traffic-event|setup|0|0:0|15|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:34 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|global-declaration-link|tmm-traffic-event|lookup|0|0:0|15|result|binary-scan-tcl-value|0|1|4348414e474544
Oct 12 08:57:34 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|global-declaration-link|tmm-traffic-event|runtime-cleanup|0|0:0|15|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:57:34 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-declaration-link|tmm-traffic-event|setup|0|0:0|15|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:34 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-declaration-link|tmm-traffic-event|lookup|0|0:0|15|result|binary-scan-tcl-value|0|1|4348414e474544
Oct 12 08:57:34 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-declaration-link|tmm-traffic-event|runtime-cleanup|0|0:0|15|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:57:34 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-alternating-arguments|tmm-traffic-event|setup|0|0:0|15|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:34 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-alternating-arguments|tmm-traffic-event|lookup|0|0:0|15|result|binary-scan-tcl-value|0|1|622030
Oct 12 08:57:34 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-alternating-arguments|tmm-traffic-event|runtime-cleanup|0|0:0|15|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- global-declaration-link, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- global-declaration-link, lookup, completion 0, result, chunk 0 of 1:

```text-visible
CHANGED⟦EOF⟧
```

- global-declaration-link, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- variable-declaration-link, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- variable-declaration-link, lookup, completion 0, result, chunk 0 of 1:

```text-visible
CHANGED⟦EOF⟧
```

- variable-declaration-link, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- variable-alternating-arguments, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- variable-alternating-arguments, lookup, completion 0, result, chunk 0 of 1:

```text-visible
b 0⟦EOF⟧
```

- variable-alternating-arguments, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- global-declaration-link: intended operation reached.

- variable-alternating-arguments: intended operation reached.

- variable-declaration-link: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 2

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:57:33 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|global-declaration-link|tmm-traffic-event|setup|0|0:0|2|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:33 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|global-declaration-link|tmm-traffic-event|lookup|0|0:0|2|result|binary-scan-tcl-value|0|1|4348414e474544
Oct 12 08:57:33 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|global-declaration-link|tmm-traffic-event|runtime-cleanup|0|0:0|2|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:57:33 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-declaration-link|tmm-traffic-event|setup|0|0:0|2|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:33 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-declaration-link|tmm-traffic-event|lookup|0|0:0|2|result|binary-scan-tcl-value|0|1|4348414e474544
Oct 12 08:57:33 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-declaration-link|tmm-traffic-event|runtime-cleanup|0|0:0|2|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:57:33 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-alternating-arguments|tmm-traffic-event|setup|0|0:0|2|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:33 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-alternating-arguments|tmm-traffic-event|lookup|0|0:0|2|result|binary-scan-tcl-value|0|1|622030
Oct 12 08:57:33 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-alternating-arguments|tmm-traffic-event|runtime-cleanup|0|0:0|2|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- global-declaration-link, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- global-declaration-link, lookup, completion 0, result, chunk 0 of 1:

```text-visible
CHANGED⟦EOF⟧
```

- global-declaration-link, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- variable-declaration-link, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- variable-declaration-link, lookup, completion 0, result, chunk 0 of 1:

```text-visible
CHANGED⟦EOF⟧
```

- variable-declaration-link, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- variable-alternating-arguments, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- variable-alternating-arguments, lookup, completion 0, result, chunk 0 of 1:

```text-visible
b 0⟦EOF⟧
```

- variable-alternating-arguments, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- global-declaration-link: intended operation reached.

- variable-alternating-arguments: intended operation reached.

- variable-declaration-link: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 3

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:57:33 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|global-declaration-link|tmm-traffic-event|setup|0|0:0|3|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:33 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|global-declaration-link|tmm-traffic-event|lookup|0|0:0|3|result|binary-scan-tcl-value|0|1|4348414e474544
Oct 12 08:57:33 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|global-declaration-link|tmm-traffic-event|runtime-cleanup|0|0:0|3|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:57:33 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-declaration-link|tmm-traffic-event|setup|0|0:0|3|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:33 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-declaration-link|tmm-traffic-event|lookup|0|0:0|3|result|binary-scan-tcl-value|0|1|4348414e474544
Oct 12 08:57:33 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-declaration-link|tmm-traffic-event|runtime-cleanup|0|0:0|3|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:57:33 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-alternating-arguments|tmm-traffic-event|setup|0|0:0|3|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:33 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-alternating-arguments|tmm-traffic-event|lookup|0|0:0|3|result|binary-scan-tcl-value|0|1|622030
Oct 12 08:57:33 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-alternating-arguments|tmm-traffic-event|runtime-cleanup|0|0:0|3|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- global-declaration-link, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- global-declaration-link, lookup, completion 0, result, chunk 0 of 1:

```text-visible
CHANGED⟦EOF⟧
```

- global-declaration-link, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- variable-declaration-link, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- variable-declaration-link, lookup, completion 0, result, chunk 0 of 1:

```text-visible
CHANGED⟦EOF⟧
```

- variable-declaration-link, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- variable-alternating-arguments, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- variable-alternating-arguments, lookup, completion 0, result, chunk 0 of 1:

```text-visible
b 0⟦EOF⟧
```

- variable-alternating-arguments, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- global-declaration-link: intended operation reached.

- variable-alternating-arguments: intended operation reached.

- variable-declaration-link: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 4

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:57:33 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|global-declaration-link|tmm-traffic-event|setup|0|0:0|4|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:33 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|global-declaration-link|tmm-traffic-event|lookup|0|0:0|4|result|binary-scan-tcl-value|0|1|4348414e474544
Oct 12 08:57:33 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|global-declaration-link|tmm-traffic-event|runtime-cleanup|0|0:0|4|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:57:33 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-declaration-link|tmm-traffic-event|setup|0|0:0|4|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:33 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-declaration-link|tmm-traffic-event|lookup|0|0:0|4|result|binary-scan-tcl-value|0|1|4348414e474544
Oct 12 08:57:33 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-declaration-link|tmm-traffic-event|runtime-cleanup|0|0:0|4|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:57:33 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-alternating-arguments|tmm-traffic-event|setup|0|0:0|4|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:33 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-alternating-arguments|tmm-traffic-event|lookup|0|0:0|4|result|binary-scan-tcl-value|0|1|622030
Oct 12 08:57:33 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-alternating-arguments|tmm-traffic-event|runtime-cleanup|0|0:0|4|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- global-declaration-link, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- global-declaration-link, lookup, completion 0, result, chunk 0 of 1:

```text-visible
CHANGED⟦EOF⟧
```

- global-declaration-link, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- variable-declaration-link, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- variable-declaration-link, lookup, completion 0, result, chunk 0 of 1:

```text-visible
CHANGED⟦EOF⟧
```

- variable-declaration-link, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- variable-alternating-arguments, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- variable-alternating-arguments, lookup, completion 0, result, chunk 0 of 1:

```text-visible
b 0⟦EOF⟧
```

- variable-alternating-arguments, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- global-declaration-link: intended operation reached.

- variable-alternating-arguments: intended operation reached.

- variable-declaration-link: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 5

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:57:33 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|global-declaration-link|tmm-traffic-event|setup|0|0:0|5|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:33 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|global-declaration-link|tmm-traffic-event|lookup|0|0:0|5|result|binary-scan-tcl-value|0|1|4348414e474544
Oct 12 08:57:33 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|global-declaration-link|tmm-traffic-event|runtime-cleanup|0|0:0|5|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:57:33 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-declaration-link|tmm-traffic-event|setup|0|0:0|5|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:33 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-declaration-link|tmm-traffic-event|lookup|0|0:0|5|result|binary-scan-tcl-value|0|1|4348414e474544
Oct 12 08:57:33 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-declaration-link|tmm-traffic-event|runtime-cleanup|0|0:0|5|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:57:33 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-alternating-arguments|tmm-traffic-event|setup|0|0:0|5|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:33 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-alternating-arguments|tmm-traffic-event|lookup|0|0:0|5|result|binary-scan-tcl-value|0|1|622030
Oct 12 08:57:33 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-alternating-arguments|tmm-traffic-event|runtime-cleanup|0|0:0|5|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- global-declaration-link, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- global-declaration-link, lookup, completion 0, result, chunk 0 of 1:

```text-visible
CHANGED⟦EOF⟧
```

- global-declaration-link, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- variable-declaration-link, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- variable-declaration-link, lookup, completion 0, result, chunk 0 of 1:

```text-visible
CHANGED⟦EOF⟧
```

- variable-declaration-link, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- variable-alternating-arguments, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- variable-alternating-arguments, lookup, completion 0, result, chunk 0 of 1:

```text-visible
b 0⟦EOF⟧
```

- variable-alternating-arguments, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- global-declaration-link: intended operation reached.

- variable-alternating-arguments: intended operation reached.

- variable-declaration-link: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 6

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:57:33 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|global-declaration-link|tmm-traffic-event|setup|0|0:0|6|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:33 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|global-declaration-link|tmm-traffic-event|lookup|0|0:0|6|result|binary-scan-tcl-value|0|1|4348414e474544
Oct 12 08:57:33 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|global-declaration-link|tmm-traffic-event|runtime-cleanup|0|0:0|6|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:57:33 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-declaration-link|tmm-traffic-event|setup|0|0:0|6|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:33 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-declaration-link|tmm-traffic-event|lookup|0|0:0|6|result|binary-scan-tcl-value|0|1|4348414e474544
Oct 12 08:57:33 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-declaration-link|tmm-traffic-event|runtime-cleanup|0|0:0|6|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:57:33 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-alternating-arguments|tmm-traffic-event|setup|0|0:0|6|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:33 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-alternating-arguments|tmm-traffic-event|lookup|0|0:0|6|result|binary-scan-tcl-value|0|1|622030
Oct 12 08:57:33 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-alternating-arguments|tmm-traffic-event|runtime-cleanup|0|0:0|6|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- global-declaration-link, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- global-declaration-link, lookup, completion 0, result, chunk 0 of 1:

```text-visible
CHANGED⟦EOF⟧
```

- global-declaration-link, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- variable-declaration-link, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- variable-declaration-link, lookup, completion 0, result, chunk 0 of 1:

```text-visible
CHANGED⟦EOF⟧
```

- variable-declaration-link, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- variable-alternating-arguments, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- variable-alternating-arguments, lookup, completion 0, result, chunk 0 of 1:

```text-visible
b 0⟦EOF⟧
```

- variable-alternating-arguments, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- global-declaration-link: intended operation reached.

- variable-alternating-arguments: intended operation reached.

- variable-declaration-link: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 7

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:57:33 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|global-declaration-link|tmm-traffic-event|setup|0|0:0|7|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:33 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|global-declaration-link|tmm-traffic-event|lookup|0|0:0|7|result|binary-scan-tcl-value|0|1|4348414e474544
Oct 12 08:57:33 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|global-declaration-link|tmm-traffic-event|runtime-cleanup|0|0:0|7|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:57:33 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-declaration-link|tmm-traffic-event|setup|0|0:0|7|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:33 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-declaration-link|tmm-traffic-event|lookup|0|0:0|7|result|binary-scan-tcl-value|0|1|4348414e474544
Oct 12 08:57:33 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-declaration-link|tmm-traffic-event|runtime-cleanup|0|0:0|7|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:57:33 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-alternating-arguments|tmm-traffic-event|setup|0|0:0|7|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:33 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-alternating-arguments|tmm-traffic-event|lookup|0|0:0|7|result|binary-scan-tcl-value|0|1|622030
Oct 12 08:57:33 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-alternating-arguments|tmm-traffic-event|runtime-cleanup|0|0:0|7|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- global-declaration-link, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- global-declaration-link, lookup, completion 0, result, chunk 0 of 1:

```text-visible
CHANGED⟦EOF⟧
```

- global-declaration-link, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- variable-declaration-link, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- variable-declaration-link, lookup, completion 0, result, chunk 0 of 1:

```text-visible
CHANGED⟦EOF⟧
```

- variable-declaration-link, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- variable-alternating-arguments, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- variable-alternating-arguments, lookup, completion 0, result, chunk 0 of 1:

```text-visible
b 0⟦EOF⟧
```

- variable-alternating-arguments, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- global-declaration-link: intended operation reached.

- variable-alternating-arguments: intended operation reached.

- variable-declaration-link: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 8

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:57:33 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|global-declaration-link|tmm-traffic-event|setup|0|0:0|8|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:33 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|global-declaration-link|tmm-traffic-event|lookup|0|0:0|8|result|binary-scan-tcl-value|0|1|4348414e474544
Oct 12 08:57:33 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|global-declaration-link|tmm-traffic-event|runtime-cleanup|0|0:0|8|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:57:33 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-declaration-link|tmm-traffic-event|setup|0|0:0|8|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:33 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-declaration-link|tmm-traffic-event|lookup|0|0:0|8|result|binary-scan-tcl-value|0|1|4348414e474544
Oct 12 08:57:33 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-declaration-link|tmm-traffic-event|runtime-cleanup|0|0:0|8|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:57:33 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-alternating-arguments|tmm-traffic-event|setup|0|0:0|8|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:33 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-alternating-arguments|tmm-traffic-event|lookup|0|0:0|8|result|binary-scan-tcl-value|0|1|622030
Oct 12 08:57:33 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-alternating-arguments|tmm-traffic-event|runtime-cleanup|0|0:0|8|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- global-declaration-link, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- global-declaration-link, lookup, completion 0, result, chunk 0 of 1:

```text-visible
CHANGED⟦EOF⟧
```

- global-declaration-link, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- variable-declaration-link, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- variable-declaration-link, lookup, completion 0, result, chunk 0 of 1:

```text-visible
CHANGED⟦EOF⟧
```

- variable-declaration-link, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- variable-alternating-arguments, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- variable-alternating-arguments, lookup, completion 0, result, chunk 0 of 1:

```text-visible
b 0⟦EOF⟧
```

- variable-alternating-arguments, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- global-declaration-link: intended operation reached.

- variable-alternating-arguments: intended operation reached.

- variable-declaration-link: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 9

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 08:57:34 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|global-declaration-link|tmm-traffic-event|setup|0|0:0|9|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:34 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|global-declaration-link|tmm-traffic-event|lookup|0|0:0|9|result|binary-scan-tcl-value|0|1|4348414e474544
Oct 12 08:57:34 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|global-declaration-link|tmm-traffic-event|runtime-cleanup|0|0:0|9|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:57:34 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-declaration-link|tmm-traffic-event|setup|0|0:0|9|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:34 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-declaration-link|tmm-traffic-event|lookup|0|0:0|9|result|binary-scan-tcl-value|0|1|4348414e474544
Oct 12 08:57:34 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-declaration-link|tmm-traffic-event|runtime-cleanup|0|0:0|9|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
Oct 12 08:57:34 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-alternating-arguments|tmm-traffic-event|setup|0|0:0|9|result|binary-scan-tcl-value|0|1|
Oct 12 08:57:34 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-alternating-arguments|tmm-traffic-event|lookup|0|0:0|9|result|binary-scan-tcl-value|0|1|622030
Oct 12 08:57:34 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012broad2_7_7 <HTTP_REQUEST>: NHO|nh20261012broad2|variable-alternating-arguments|tmm-traffic-event|runtime-cleanup|0|0:0|9|result|binary-scan-tcl-value|0|1|7b7d207b7d207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- global-declaration-link, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- global-declaration-link, lookup, completion 0, result, chunk 0 of 1:

```text-visible
CHANGED⟦EOF⟧
```

- global-declaration-link, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- variable-declaration-link, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- variable-declaration-link, lookup, completion 0, result, chunk 0 of 1:

```text-visible
CHANGED⟦EOF⟧
```

- variable-declaration-link, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- variable-alternating-arguments, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- variable-alternating-arguments, lookup, completion 0, result, chunk 0 of 1:

```text-visible
b 0⟦EOF⟧
```

- variable-alternating-arguments, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
{} {} {}⟦EOF⟧
```

- global-declaration-link: intended operation reached.

- variable-alternating-arguments: intended operation reached.

- variable-declaration-link: intended operation reached.
