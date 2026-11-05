# BIG-IP Tcl log: nh20261012repair2, tmm-procedure-0

## Question

The following cases investigate Tcl behavior relevant to name resolution and diagnostics in the LSP, compiler and VM. Each observation is limited to its own case and recorded context.

- **parent-original-name-inventory:** Which namespace subcommands, interp forms and dynamic definition operations exist in each context? Are unsupported literal commands rejected at load even in `catch`? Compare computed head/eval.

- **parent-tcltest-real-package:** Which namespace subcommands, interp forms and dynamic definition operations exist in each context? Are unsupported literal commands rejected at load even in `catch`? Compare computed head/eval.

- **version:** Which Tcl version and execution namespace are available in this tmsh context, so that observations from this context can be scoped correctly?

## Assumptions

- **parent-original-name-inventory:** Independent capability/ownership diagnostic; no original lookup answer
- **parent-tcltest-real-package:** Independent capability/ownership diagnostic; no original lookup answer
- **version:** Independent capability/ownership diagnostic; no original lookup answer

Other assumptions were not separately recorded.

## Script

```tcl-visible
set nh_options_available unavailable
set nh_return_options {}
set nh_unit "[TMM::cmp_group]:[TMM::cmp_unit]"
set nh_invocation once
set nh_setup {}
set nh_operation [binary format H* 6c697374205b696e666f2070617463686c6576656c5d205b696e666f2074636c76657273696f6e5d205b6e616d6573706163652063757272656e745d]
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
        log local0. "NHO|nh20261012repair2|version|tmm-procedure|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012repair2|version|tmm-procedure|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_setup {}
set nh_operation [binary format H* 736574206e685f696e76656e746f7279207b7d3b20666f7265616368206e685f6e616d65207b3a3a666f7277617264203a3a6368616e6765203a3a6f75746572203a3a656d6974203a3a726564203a3a626c7565203a3a446562756750757473203a3a74636c746573743a3a4465627567507574737d207b736574206e685f626f6479207b7d3b20736574206e685f626f64795f636f6465205b6361746368207b696e666f20626f647920246e685f6e616d657d206e685f626f64795d3b206c617070656e64206e685f696e76656e746f7279205b6c69737420246e685f6e616d6520636f6d6d616e6473205b696e666f20636f6d6d616e647320246e685f6e616d655d2070726f6373205b696e666f2070726f637320246e685f6e616d655d20626f64792d636f646520246e685f626f64795f636f646520626f647920246e685f626f64795d7d3b20666f7265616368206e685f6e616d65207b6e7320746172676574206465736372697074696f6e207072656669787d207b6c617070656e64206e685f696e76656e746f7279205b6c697374206672616d652d7661726961626c6520246e685f6e616d6520657869737473205b696e666f2065786973747320246e685f6e616d655d2076617273205b696e666f207661727320246e685f6e616d655d20747261636573205b74726163652076696e666f20246e685f6e616d655d5d7d3b20736574206e685f696e76656e746f7279]
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
        log local0. "NHO|nh20261012repair2|parent-original-name-inventory|tmm-procedure|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012repair2|parent-original-name-inventory|tmm-procedure|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
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
        log local0. "NHO|nh20261012repair2|parent-tcltest-real-package|tmm-procedure|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012repair2|parent-tcltest-real-package|tmm-procedure|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
```

## Results

### tmm-procedure, unit 0:0, invocation once; tmm-procedure, unit 0:1, invocation once; tmm-procedure, unit 0:2, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-procedure.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- version, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- version, lookup, completion 0, result:

```text-visible
8.4.6 8.4 ::⟦EOF⟧
```

- parent-original-name-inventory, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- parent-original-name-inventory, lookup, completion 0, result:

```text-visible
{::forward commands ::forward procs {} body-code 1 body {"::forward" isn't a procedure}} {::change commands {} procs {} body-code 1 body {"::change" isn't a procedure}} {::outer commands {} procs {} body-code 1 body {"::outer" isn't a procedure}} {::emit commands {} procs {} body-code 1 body {"::emit" isn't a procedure}} {::red commands {} procs {} body-code 1 body {"::red" isn't a procedure}} {::blue commands {} procs {} body-code 1 body {"::blue" isn't a procedure}} {::DebugPuts commands {} procs {} body-code 1 body {"::DebugPuts" isn't a procedure}} {::tcltest::DebugPuts commands {} procs {} body-code 1 body {"::tcltest::DebugPuts" isn't a procedure}} {frame-variable ns exists 0 vars {} traces {}} {frame-variable target exists 0 vars {} traces {}} {frame-variable description exists 0 vars {} traces {}} {frame-variable prefix exists 0 vars {} traces {}}⟦EOF⟧
```

- parent-tcltest-real-package, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- parent-tcltest-real-package, lookup, completion 1, result:

```text-visible
can't find package tcltest⟦EOF⟧
```

- parent-tcltest-real-package, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- parent-tcltest-real-package, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- parent-tcltest-real-package, lookup, completion 1, errorinfo:

```text-visible
can't find package tcltest
    while executing
"package require tcltest"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- parent-original-name-inventory: intended operation reached.

- parent-tcltest-real-package: intended operation reached.

- version: intended operation reached.

### tmm-procedure, unit 0:3, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-procedure.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- version, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- version, lookup, completion 0, result:

```text-visible
8.4.6 8.4 ::⟦EOF⟧
```

- parent-original-name-inventory, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- parent-original-name-inventory, lookup, completion 0, result:

```text-visible
{::forward commands ::forward procs {} body-code 1 body {"::forward" isn't a procedure}} {::change commands {} procs {} body-code 1 body {"::change" isn't a procedure}} {::outer commands {} procs {} body-code 1 body {"::outer" isn't a procedure}} {::emit commands {} procs {} body-code 1 body {"::emit" isn't a procedure}} {::red commands {} procs {} body-code 1 body {"::red" isn't a procedure}} {::blue commands {} procs {} body-code 1 body {"::blue" isn't a procedure}} {::DebugPuts commands {} procs {} body-code 1 body {"::DebugPuts" isn't a procedure}} {::tcltest::DebugPuts commands {} procs {} body-code 1 body {"::tcltest::DebugPuts" isn't a procedure}} {frame-variable ns exists 0 vars {} traces {}} {frame-variable target exists 0 vars {} traces {}} {frame-variable description exists 0 vars {} traces {}} {frame-variable prefix exists 0 vars {} traces {}}⟦EOF⟧
```

- parent-tcltest-real-package, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- parent-tcltest-real-package, lookup, completion 1, result:

```text-visible
can't find package tcltest⟦EOF⟧
```

- parent-tcltest-real-package, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- parent-tcltest-real-package, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- parent-tcltest-real-package, lookup, completion 1, errorinfo:

```text-visible
can't find package tcltest
    while executing
"package require tcltest"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- parent-original-name-inventory: intended operation reached.

- parent-tcltest-real-package: intended operation reached.

- version: intended operation reached.
