# BIG-IP Tcl log: nh20261012cap3, tmm-traffic-event-3

## Question

The following cases investigate Tcl behavior relevant to name resolution and diagnostics in the LSP, compiler and VM. Each observation is limited to its own case and recorded context.

- **interp.alias:** Which namespace subcommands, interp forms and dynamic definition operations exist in each context? Are unsupported literal commands rejected at load even in `catch`? Compare computed head/eval.

- **interp.create:** Which namespace subcommands, interp forms and dynamic definition operations exist in each context? Are unsupported literal commands rejected at load even in `catch`? Compare computed head/eval.

- **tcltest:** Which namespace subcommands, interp forms and dynamic definition operations exist in each context? Are unsupported literal commands rejected at load even in `catch`? Compare computed head/eval.

## Assumptions

- **interp.alias:** Not recorded
- **interp.create:** Not recorded
- **tcltest:** Not recorded

Other assumptions were not separately recorded.

## Script

```tcl-visible
set nh_options_available unavailable
set nh_return_options {}
set nh_unit "[TMM::cmp_group]:[TMM::cmp_unit]"
set nh_invocation once
set nh_invocation [HTTP::header value X-NH-Invocation]
set nh_setup {}
set nh_operation [binary format H* 736574206e685f6368696c64205b696e74657270206372656174655d3b20736574206e685f726573756c74205b696e74657270206576616c20246e685f6368696c64207b696e666f2070617463686c6576656c7d5d3b20696e746572702064656c65746520246e685f6368696c643b20736574206e685f726573756c74]
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
        log local0. "NHO|nh20261012cap3|interp.create|tmm-traffic-event|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012cap3|interp.create|tmm-traffic-event|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 6966207b5b6e616d65737061636520657869737473203a3a5f5f6e685f6e683230323631303132636170335f696e746572705f6372656174655d7d207b6e616d6573706163652064656c657465203a3a5f5f6e685f6e683230323631303132636170335f696e746572705f6372656174657d3b20666f7265616368206e685f636d64205b696e666f20636f6d6d616e6473203a3a5f5f6e685f6e683230323631303132636170335f696e746572705f6372656174655f2a5d207b72656e616d6520246e685f636d64207b7d7d3b206c697374205b6e616d65737061636520657869737473203a3a5f5f6e685f6e683230323631303132636170335f696e746572705f6372656174655d205b696e666f20636f6d6d616e6473203a3a5f5f6e685f6e683230323631303132636170335f696e746572705f6372656174655f2a5d]
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
        log local0. "NHO|nh20261012cap3|interp.create|tmm-traffic-event|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
set nh_setup [binary format H* 70726f63203a3a5f5f6e685f6e683230323631303132636170335f696e746572705f616c6961735f746172676574207b7d207b72657475726e205441524745547d]
set nh_operation [binary format H* 696e7465727020616c696173207b7d203a3a5f5f6e685f6e683230323631303132636170335f696e746572705f616c6961735f616c696173207b7d203a3a5f5f6e685f6e683230323631303132636170335f696e746572705f616c6961735f7461726765743b207365742072205b3a3a5f5f6e685f6e683230323631303132636170335f696e746572705f616c6961735f616c6961735d3b20696e7465727020616c696173207b7d203a3a5f5f6e685f6e683230323631303132636170335f696e746572705f616c6961735f616c696173207b7d3b207365742072]
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
        log local0. "NHO|nh20261012cap3|interp.alias|tmm-traffic-event|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012cap3|interp.alias|tmm-traffic-event|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 6966207b5b6e616d65737061636520657869737473203a3a5f5f6e685f6e683230323631303132636170335f696e746572705f616c6961735d7d207b6e616d6573706163652064656c657465203a3a5f5f6e685f6e683230323631303132636170335f696e746572705f616c6961737d3b20666f7265616368206e685f636d64205b696e666f20636f6d6d616e6473203a3a5f5f6e685f6e683230323631303132636170335f696e746572705f616c6961735f2a5d207b72656e616d6520246e685f636d64207b7d7d3b206c697374205b6e616d65737061636520657869737473203a3a5f5f6e685f6e683230323631303132636170335f696e746572705f616c6961735d205b696e666f20636f6d6d616e6473203a3a5f5f6e685f6e683230323631303132636170335f696e746572705f616c6961735f2a5d]
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
        log local0. "NHO|nh20261012cap3|interp.alias|tmm-traffic-event|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
set nh_setup {}
set nh_operation [binary format H* 7061636b61676520726571756972652074636c74657374]
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
        log local0. "NHO|nh20261012cap3|tcltest|tmm-traffic-event|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012cap3|tcltest|tmm-traffic-event|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 6966207b5b6e616d65737061636520657869737473203a3a5f5f6e685f6e683230323631303132636170335f74636c746573745d7d207b6e616d6573706163652064656c657465203a3a5f5f6e685f6e683230323631303132636170335f74636c746573747d3b20666f7265616368206e685f636d64205b696e666f20636f6d6d616e6473203a3a5f5f6e685f6e683230323631303132636170335f74636c746573745f2a5d207b72656e616d6520246e685f636d64207b7d7d3b206c697374205b6e616d65737061636520657869737473203a3a5f5f6e685f6e683230323631303132636170335f74636c746573745d205b696e666f20636f6d6d616e6473203a3a5f5f6e685f6e683230323631303132636170335f74636c746573745f2a5d]
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
        log local0. "NHO|nh20261012cap3|tcltest|tmm-traffic-event|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
```

## Results

### tmm-traffic-event, unit 0:0, invocation 0; tmm-traffic-event, unit 0:0, invocation 1; tmm-traffic-event, unit 0:0, invocation 10; tmm-traffic-event, unit 0:0, invocation 11; tmm-traffic-event, unit 0:0, invocation 12; tmm-traffic-event, unit 0:0, invocation 13; tmm-traffic-event, unit 0:0, invocation 14; tmm-traffic-event, unit 0:0, invocation 15; tmm-traffic-event, unit 0:0, invocation 2; tmm-traffic-event, unit 0:0, invocation 3; tmm-traffic-event, unit 0:0, invocation 4; tmm-traffic-event, unit 0:0, invocation 5; tmm-traffic-event, unit 0:0, invocation 6; tmm-traffic-event, unit 0:0, invocation 7; tmm-traffic-event, unit 0:0, invocation 8

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- interp.create, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- interp.create, lookup, completion 0, result:

```text-visible
8.4.6⟦EOF⟧
```

- interp.create, runtime-cleanup, completion 0, result:

```text-visible
0 {}⟦EOF⟧
```

- interp.alias, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- interp.alias, lookup, completion 0, result:

```text-visible
TARGET⟦EOF⟧
```

- interp.alias, runtime-cleanup, completion 0, result:

```text-visible
0 {}⟦EOF⟧
```

- tcltest, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- tcltest, lookup, completion 1, result:

```text-visible
can't find package tcltest⟦EOF⟧
```

- tcltest, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- tcltest, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- tcltest, lookup, completion 1, errorinfo:

```text-visible
can't find package tcltest
    while executing
"package require tcltest"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- tcltest, runtime-cleanup, completion 0, result:

```text-visible
0 {}⟦EOF⟧
```

- interp.alias: intended operation reached.

- interp.create: intended operation reached.

- tcltest: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 9

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- interp.create, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- interp.create, lookup, completion 0, result:

```text-visible
8.4.6⟦EOF⟧
```

- interp.create, runtime-cleanup, completion 0, result:

```text-visible
0 {}⟦EOF⟧
```

- interp.alias, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- interp.alias, lookup, completion 0, result:

```text-visible
TARGET⟦EOF⟧
```

- interp.alias, runtime-cleanup, completion 0, result:

```text-visible
0 {}⟦EOF⟧
```

- tcltest, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- tcltest, lookup, completion 1, result:

```text-visible
can't find package tcltest⟦EOF⟧
```

- tcltest, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- tcltest, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- tcltest, lookup, completion 1, errorinfo:

```text-visible
can't find package tcltest
    while executing
"package require tcltest"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- tcltest, runtime-cleanup, completion 0, result:

```text-visible
0 {}⟦EOF⟧
```

- interp.alias: intended operation reached.

- interp.create: intended operation reached.

- tcltest: intended operation reached.
