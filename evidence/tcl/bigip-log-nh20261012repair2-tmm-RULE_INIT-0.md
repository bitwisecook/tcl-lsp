# BIG-IP Tcl log: nh20261012repair2, tmm-RULE_INIT-0

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
        log local0. "NHO|nh20261012repair2|version|tmm-RULE_INIT|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012repair2|version|tmm-RULE_INIT|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012repair2|parent-original-name-inventory|tmm-RULE_INIT|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012repair2|parent-original-name-inventory|tmm-RULE_INIT|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012repair2|parent-tcltest-real-package|tmm-RULE_INIT|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012repair2|parent-tcltest-real-package|tmm-RULE_INIT|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
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
NHO|nh20261012repair2|version|tmm-RULE_INIT|setup|0|0:0|once|result|binary-scan-tcl-value|0|1|
NHO|nh20261012repair2|version|tmm-RULE_INIT|lookup|0|0:0|once|result|binary-scan-tcl-value|0|1|382e342e3620382e34203a3a
NHO|nh20261012repair2|parent-original-name-inventory|tmm-RULE_INIT|setup|0|0:0|once|result|binary-scan-tcl-value|0|1|
NHO|nh20261012repair2|parent-original-name-inventory|tmm-RULE_INIT|lookup|0|0:0|once|result|binary-scan-tcl-value|0|5|7b3a3a666f727761726420636f6d6d616e6473203a3a666f72776172642070726f6373207b7d20626f64792d636f6465203120626f6479207b223a3a666f7277617264222069736e277420612070726f6365647572657d7d207b3a3a6368616e676520636f6d6d616e6473207b7d2070726f6373207b7d20626f64792d636f6465203120626f6479207b223a3a6368616e6765222069736e277420612070726f6365647572657d7d207b3a3a6f7574657220636f6d6d616e6473207b7d2070726f6373207b7d2062
NHO|nh20261012repair2|parent-original-name-inventory|tmm-RULE_INIT|lookup|0|0:0|once|result|binary-scan-tcl-value|1|5|6f64792d636f6465203120626f6479207b223a3a6f75746572222069736e277420612070726f6365647572657d7d207b3a3a656d697420636f6d6d616e6473207b7d2070726f6373207b7d20626f64792d636f6465203120626f6479207b223a3a656d6974222069736e277420612070726f6365647572657d7d207b3a3a72656420636f6d6d616e6473207b7d2070726f6373207b7d20626f64792d636f6465203120626f6479207b223a3a726564222069736e277420612070726f6365647572657d7d207b3a3a
NHO|nh20261012repair2|parent-original-name-inventory|tmm-RULE_INIT|lookup|0|0:0|once|result|binary-scan-tcl-value|2|5|626c756520636f6d6d616e6473207b7d2070726f6373207b7d20626f64792d636f6465203120626f6479207b223a3a626c7565222069736e277420612070726f6365647572657d7d207b3a3a44656275675075747320636f6d6d616e6473207b7d2070726f6373207b7d20626f64792d636f6465203120626f6479207b223a3a446562756750757473222069736e277420612070726f6365647572657d7d207b3a3a74636c746573743a3a44656275675075747320636f6d6d616e6473207b7d2070726f6373207b
NHO|nh20261012repair2|parent-original-name-inventory|tmm-RULE_INIT|lookup|0|0:0|once|result|binary-scan-tcl-value|3|5|7d20626f64792d636f6465203120626f6479207b223a3a74636c746573743a3a446562756750757473222069736e277420612070726f6365647572657d7d207b6672616d652d7661726961626c65206e732065786973747320312076617273206e7320747261636573207b7d7d207b6672616d652d7661726961626c652074617267657420657869737473203120766172732074617267657420747261636573207b7d7d207b6672616d652d7661726961626c65206465736372697074696f6e2065786973747320
NHO|nh20261012repair2|parent-original-name-inventory|tmm-RULE_INIT|lookup|0|0:0|once|result|binary-scan-tcl-value|4|5|312076617273206465736372697074696f6e20747261636573207b7d7d207b6672616d652d7661726961626c652070726566697820657869737473203120766172732070726566697820747261636573207b7d7d
NHO|nh20261012repair2|parent-tcltest-real-package|tmm-RULE_INIT|setup|0|0:0|once|result|binary-scan-tcl-value|0|1|
NHO|nh20261012repair2|parent-tcltest-real-package|tmm-RULE_INIT|lookup|1|0:0|once|result|binary-scan-tcl-value|0|1|63616e27742066696e64207061636b6167652074636c74657374
NHO|nh20261012repair2|parent-tcltest-real-package|tmm-RULE_INIT|lookup|1|0:0|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
NHO|nh20261012repair2|parent-tcltest-real-package|tmm-RULE_INIT|lookup|1|0:0|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
NHO|nh20261012repair2|parent-tcltest-real-package|tmm-RULE_INIT|lookup|1|0:0|once|errorinfo|binary-scan-tcl-value|0|1|63616e27742066696e64207061636b6167652074636c746573740a202020207768696c6520657865637574696e670a227061636b61676520726571756972652074636c74657374220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- version, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- version, lookup, completion 0, result, chunk 0 of 1:

```text-visible
8.4.6 8.4 ::⟦EOF⟧
```

- parent-original-name-inventory, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- parent-original-name-inventory, lookup, completion 0, result, chunk 0 of 5:

```text-visible
{::forward commands ::forward procs {} body-code 1 body {"::forward" isn't a procedure}} {::change commands {} procs {} body-code 1 body {"::change" isn't a procedure}} {::outer commands {} procs {} b⟦EOF⟧
```

- parent-original-name-inventory, lookup, completion 0, result, chunk 1 of 5:

```text-visible
ody-code 1 body {"::outer" isn't a procedure}} {::emit commands {} procs {} body-code 1 body {"::emit" isn't a procedure}} {::red commands {} procs {} body-code 1 body {"::red" isn't a procedure}} {::⟦EOF⟧
```

- parent-original-name-inventory, lookup, completion 0, result, chunk 2 of 5:

```text-visible
blue commands {} procs {} body-code 1 body {"::blue" isn't a procedure}} {::DebugPuts commands {} procs {} body-code 1 body {"::DebugPuts" isn't a procedure}} {::tcltest::DebugPuts commands {} procs {⟦EOF⟧
```

- parent-original-name-inventory, lookup, completion 0, result, chunk 3 of 5:

```text-visible
} body-code 1 body {"::tcltest::DebugPuts" isn't a procedure}} {frame-variable ns exists 1 vars ns traces {}} {frame-variable target exists 1 vars target traces {}} {frame-variable description exists⟦SPACE⟧⟦EOF⟧
```

- parent-original-name-inventory, lookup, completion 0, result, chunk 4 of 5:

```text-visible
1 vars description traces {}} {frame-variable prefix exists 1 vars prefix traces {}}⟦EOF⟧
```

- parent-tcltest-real-package, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- parent-tcltest-real-package, lookup, completion 1, result, chunk 0 of 1:

```text-visible
can't find package tcltest⟦EOF⟧
```

- parent-tcltest-real-package, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- parent-tcltest-real-package, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- parent-tcltest-real-package, lookup, completion 1, errorinfo, chunk 0 of 1:

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

### tmm-RULE_INIT, unit 0:1, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
NHO|nh20261012repair2|version|tmm-RULE_INIT|setup|0|0:1|once|result|binary-scan-tcl-value|0|1|
NHO|nh20261012repair2|version|tmm-RULE_INIT|lookup|0|0:1|once|result|binary-scan-tcl-value|0|1|382e342e3620382e34203a3a
NHO|nh20261012repair2|parent-original-name-inventory|tmm-RULE_INIT|setup|0|0:1|once|result|binary-scan-tcl-value|0|1|
NHO|nh20261012repair2|parent-original-name-inventory|tmm-RULE_INIT|lookup|0|0:1|once|result|binary-scan-tcl-value|0|5|7b3a3a666f727761726420636f6d6d616e6473203a3a666f72776172642070726f6373207b7d20626f64792d636f6465203120626f6479207b223a3a666f7277617264222069736e277420612070726f6365647572657d7d207b3a3a6368616e676520636f6d6d616e6473207b7d2070726f6373207b7d20626f64792d636f6465203120626f6479207b223a3a6368616e6765222069736e277420612070726f6365647572657d7d207b3a3a6f7574657220636f6d6d616e6473207b7d2070726f6373207b7d2062
NHO|nh20261012repair2|parent-original-name-inventory|tmm-RULE_INIT|lookup|0|0:1|once|result|binary-scan-tcl-value|1|5|6f64792d636f6465203120626f6479207b223a3a6f75746572222069736e277420612070726f6365647572657d7d207b3a3a656d697420636f6d6d616e6473207b7d2070726f6373207b7d20626f64792d636f6465203120626f6479207b223a3a656d6974222069736e277420612070726f6365647572657d7d207b3a3a72656420636f6d6d616e6473207b7d2070726f6373207b7d20626f64792d636f6465203120626f6479207b223a3a726564222069736e277420612070726f6365647572657d7d207b3a3a
NHO|nh20261012repair2|parent-original-name-inventory|tmm-RULE_INIT|lookup|0|0:1|once|result|binary-scan-tcl-value|2|5|626c756520636f6d6d616e6473207b7d2070726f6373207b7d20626f64792d636f6465203120626f6479207b223a3a626c7565222069736e277420612070726f6365647572657d7d207b3a3a44656275675075747320636f6d6d616e6473207b7d2070726f6373207b7d20626f64792d636f6465203120626f6479207b223a3a446562756750757473222069736e277420612070726f6365647572657d7d207b3a3a74636c746573743a3a44656275675075747320636f6d6d616e6473207b7d2070726f6373207b
NHO|nh20261012repair2|parent-original-name-inventory|tmm-RULE_INIT|lookup|0|0:1|once|result|binary-scan-tcl-value|3|5|7d20626f64792d636f6465203120626f6479207b223a3a74636c746573743a3a446562756750757473222069736e277420612070726f6365647572657d7d207b6672616d652d7661726961626c65206e732065786973747320312076617273206e7320747261636573207b7d7d207b6672616d652d7661726961626c652074617267657420657869737473203120766172732074617267657420747261636573207b7d7d207b6672616d652d7661726961626c65206465736372697074696f6e2065786973747320
NHO|nh20261012repair2|parent-original-name-inventory|tmm-RULE_INIT|lookup|0|0:1|once|result|binary-scan-tcl-value|4|5|312076617273206465736372697074696f6e20747261636573207b7d7d207b6672616d652d7661726961626c652070726566697820657869737473203120766172732070726566697820747261636573207b7d7d
NHO|nh20261012repair2|parent-tcltest-real-package|tmm-RULE_INIT|setup|0|0:1|once|result|binary-scan-tcl-value|0|1|
NHO|nh20261012repair2|parent-tcltest-real-package|tmm-RULE_INIT|lookup|1|0:1|once|result|binary-scan-tcl-value|0|1|63616e27742066696e64207061636b6167652074636c74657374
NHO|nh20261012repair2|parent-tcltest-real-package|tmm-RULE_INIT|lookup|1|0:1|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
NHO|nh20261012repair2|parent-tcltest-real-package|tmm-RULE_INIT|lookup|1|0:1|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
NHO|nh20261012repair2|parent-tcltest-real-package|tmm-RULE_INIT|lookup|1|0:1|once|errorinfo|binary-scan-tcl-value|0|1|63616e27742066696e64207061636b6167652074636c746573740a202020207768696c6520657865637574696e670a227061636b61676520726571756972652074636c74657374220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- version, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- version, lookup, completion 0, result, chunk 0 of 1:

```text-visible
8.4.6 8.4 ::⟦EOF⟧
```

- parent-original-name-inventory, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- parent-original-name-inventory, lookup, completion 0, result, chunk 0 of 5:

```text-visible
{::forward commands ::forward procs {} body-code 1 body {"::forward" isn't a procedure}} {::change commands {} procs {} body-code 1 body {"::change" isn't a procedure}} {::outer commands {} procs {} b⟦EOF⟧
```

- parent-original-name-inventory, lookup, completion 0, result, chunk 1 of 5:

```text-visible
ody-code 1 body {"::outer" isn't a procedure}} {::emit commands {} procs {} body-code 1 body {"::emit" isn't a procedure}} {::red commands {} procs {} body-code 1 body {"::red" isn't a procedure}} {::⟦EOF⟧
```

- parent-original-name-inventory, lookup, completion 0, result, chunk 2 of 5:

```text-visible
blue commands {} procs {} body-code 1 body {"::blue" isn't a procedure}} {::DebugPuts commands {} procs {} body-code 1 body {"::DebugPuts" isn't a procedure}} {::tcltest::DebugPuts commands {} procs {⟦EOF⟧
```

- parent-original-name-inventory, lookup, completion 0, result, chunk 3 of 5:

```text-visible
} body-code 1 body {"::tcltest::DebugPuts" isn't a procedure}} {frame-variable ns exists 1 vars ns traces {}} {frame-variable target exists 1 vars target traces {}} {frame-variable description exists⟦SPACE⟧⟦EOF⟧
```

- parent-original-name-inventory, lookup, completion 0, result, chunk 4 of 5:

```text-visible
1 vars description traces {}} {frame-variable prefix exists 1 vars prefix traces {}}⟦EOF⟧
```

- parent-tcltest-real-package, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- parent-tcltest-real-package, lookup, completion 1, result, chunk 0 of 1:

```text-visible
can't find package tcltest⟦EOF⟧
```

- parent-tcltest-real-package, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- parent-tcltest-real-package, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- parent-tcltest-real-package, lookup, completion 1, errorinfo, chunk 0 of 1:

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

### tmm-RULE_INIT, unit 0:2, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
NHO|nh20261012repair2|version|tmm-RULE_INIT|setup|0|0:2|once|result|binary-scan-tcl-value|0|1|
NHO|nh20261012repair2|version|tmm-RULE_INIT|lookup|0|0:2|once|result|binary-scan-tcl-value|0|1|382e342e3620382e34203a3a
NHO|nh20261012repair2|parent-original-name-inventory|tmm-RULE_INIT|setup|0|0:2|once|result|binary-scan-tcl-value|0|1|
NHO|nh20261012repair2|parent-original-name-inventory|tmm-RULE_INIT|lookup|0|0:2|once|result|binary-scan-tcl-value|0|5|7b3a3a666f727761726420636f6d6d616e6473203a3a666f72776172642070726f6373207b7d20626f64792d636f6465203120626f6479207b223a3a666f7277617264222069736e277420612070726f6365647572657d7d207b3a3a6368616e676520636f6d6d616e6473207b7d2070726f6373207b7d20626f64792d636f6465203120626f6479207b223a3a6368616e6765222069736e277420612070726f6365647572657d7d207b3a3a6f7574657220636f6d6d616e6473207b7d2070726f6373207b7d2062
NHO|nh20261012repair2|parent-original-name-inventory|tmm-RULE_INIT|lookup|0|0:2|once|result|binary-scan-tcl-value|1|5|6f64792d636f6465203120626f6479207b223a3a6f75746572222069736e277420612070726f6365647572657d7d207b3a3a656d697420636f6d6d616e6473207b7d2070726f6373207b7d20626f64792d636f6465203120626f6479207b223a3a656d6974222069736e277420612070726f6365647572657d7d207b3a3a72656420636f6d6d616e6473207b7d2070726f6373207b7d20626f64792d636f6465203120626f6479207b223a3a726564222069736e277420612070726f6365647572657d7d207b3a3a
NHO|nh20261012repair2|parent-original-name-inventory|tmm-RULE_INIT|lookup|0|0:2|once|result|binary-scan-tcl-value|2|5|626c756520636f6d6d616e6473207b7d2070726f6373207b7d20626f64792d636f6465203120626f6479207b223a3a626c7565222069736e277420612070726f6365647572657d7d207b3a3a44656275675075747320636f6d6d616e6473207b7d2070726f6373207b7d20626f64792d636f6465203120626f6479207b223a3a446562756750757473222069736e277420612070726f6365647572657d7d207b3a3a74636c746573743a3a44656275675075747320636f6d6d616e6473207b7d2070726f6373207b
NHO|nh20261012repair2|parent-original-name-inventory|tmm-RULE_INIT|lookup|0|0:2|once|result|binary-scan-tcl-value|3|5|7d20626f64792d636f6465203120626f6479207b223a3a74636c746573743a3a446562756750757473222069736e277420612070726f6365647572657d7d207b6672616d652d7661726961626c65206e732065786973747320312076617273206e7320747261636573207b7d7d207b6672616d652d7661726961626c652074617267657420657869737473203120766172732074617267657420747261636573207b7d7d207b6672616d652d7661726961626c65206465736372697074696f6e2065786973747320
NHO|nh20261012repair2|parent-original-name-inventory|tmm-RULE_INIT|lookup|0|0:2|once|result|binary-scan-tcl-value|4|5|312076617273206465736372697074696f6e20747261636573207b7d7d207b6672616d652d7661726961626c652070726566697820657869737473203120766172732070726566697820747261636573207b7d7d
NHO|nh20261012repair2|parent-tcltest-real-package|tmm-RULE_INIT|setup|0|0:2|once|result|binary-scan-tcl-value|0|1|
NHO|nh20261012repair2|parent-tcltest-real-package|tmm-RULE_INIT|lookup|1|0:2|once|result|binary-scan-tcl-value|0|1|63616e27742066696e64207061636b6167652074636c74657374
NHO|nh20261012repair2|parent-tcltest-real-package|tmm-RULE_INIT|lookup|1|0:2|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
NHO|nh20261012repair2|parent-tcltest-real-package|tmm-RULE_INIT|lookup|1|0:2|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
NHO|nh20261012repair2|parent-tcltest-real-package|tmm-RULE_INIT|lookup|1|0:2|once|errorinfo|binary-scan-tcl-value|0|1|63616e27742066696e64207061636b6167652074636c746573740a202020207768696c6520657865637574696e670a227061636b61676520726571756972652074636c74657374220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- version, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- version, lookup, completion 0, result, chunk 0 of 1:

```text-visible
8.4.6 8.4 ::⟦EOF⟧
```

- parent-original-name-inventory, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- parent-original-name-inventory, lookup, completion 0, result, chunk 0 of 5:

```text-visible
{::forward commands ::forward procs {} body-code 1 body {"::forward" isn't a procedure}} {::change commands {} procs {} body-code 1 body {"::change" isn't a procedure}} {::outer commands {} procs {} b⟦EOF⟧
```

- parent-original-name-inventory, lookup, completion 0, result, chunk 1 of 5:

```text-visible
ody-code 1 body {"::outer" isn't a procedure}} {::emit commands {} procs {} body-code 1 body {"::emit" isn't a procedure}} {::red commands {} procs {} body-code 1 body {"::red" isn't a procedure}} {::⟦EOF⟧
```

- parent-original-name-inventory, lookup, completion 0, result, chunk 2 of 5:

```text-visible
blue commands {} procs {} body-code 1 body {"::blue" isn't a procedure}} {::DebugPuts commands {} procs {} body-code 1 body {"::DebugPuts" isn't a procedure}} {::tcltest::DebugPuts commands {} procs {⟦EOF⟧
```

- parent-original-name-inventory, lookup, completion 0, result, chunk 3 of 5:

```text-visible
} body-code 1 body {"::tcltest::DebugPuts" isn't a procedure}} {frame-variable ns exists 1 vars ns traces {}} {frame-variable target exists 1 vars target traces {}} {frame-variable description exists⟦SPACE⟧⟦EOF⟧
```

- parent-original-name-inventory, lookup, completion 0, result, chunk 4 of 5:

```text-visible
1 vars description traces {}} {frame-variable prefix exists 1 vars prefix traces {}}⟦EOF⟧
```

- parent-tcltest-real-package, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- parent-tcltest-real-package, lookup, completion 1, result, chunk 0 of 1:

```text-visible
can't find package tcltest⟦EOF⟧
```

- parent-tcltest-real-package, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- parent-tcltest-real-package, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- parent-tcltest-real-package, lookup, completion 1, errorinfo, chunk 0 of 1:

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

### tmm-RULE_INIT, unit 0:3, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
NHO|nh20261012repair2|version|tmm-RULE_INIT|setup|0|0:3|once|result|binary-scan-tcl-value|0|1|
NHO|nh20261012repair2|version|tmm-RULE_INIT|lookup|0|0:3|once|result|binary-scan-tcl-value|0|1|382e342e3620382e34203a3a
NHO|nh20261012repair2|parent-original-name-inventory|tmm-RULE_INIT|setup|0|0:3|once|result|binary-scan-tcl-value|0|1|
NHO|nh20261012repair2|parent-original-name-inventory|tmm-RULE_INIT|lookup|0|0:3|once|result|binary-scan-tcl-value|0|5|7b3a3a666f727761726420636f6d6d616e6473203a3a666f72776172642070726f6373207b7d20626f64792d636f6465203120626f6479207b223a3a666f7277617264222069736e277420612070726f6365647572657d7d207b3a3a6368616e676520636f6d6d616e6473207b7d2070726f6373207b7d20626f64792d636f6465203120626f6479207b223a3a6368616e6765222069736e277420612070726f6365647572657d7d207b3a3a6f7574657220636f6d6d616e6473207b7d2070726f6373207b7d2062
NHO|nh20261012repair2|parent-original-name-inventory|tmm-RULE_INIT|lookup|0|0:3|once|result|binary-scan-tcl-value|1|5|6f64792d636f6465203120626f6479207b223a3a6f75746572222069736e277420612070726f6365647572657d7d207b3a3a656d697420636f6d6d616e6473207b7d2070726f6373207b7d20626f64792d636f6465203120626f6479207b223a3a656d6974222069736e277420612070726f6365647572657d7d207b3a3a72656420636f6d6d616e6473207b7d2070726f6373207b7d20626f64792d636f6465203120626f6479207b223a3a726564222069736e277420612070726f6365647572657d7d207b3a3a
NHO|nh20261012repair2|parent-original-name-inventory|tmm-RULE_INIT|lookup|0|0:3|once|result|binary-scan-tcl-value|2|5|626c756520636f6d6d616e6473207b7d2070726f6373207b7d20626f64792d636f6465203120626f6479207b223a3a626c7565222069736e277420612070726f6365647572657d7d207b3a3a44656275675075747320636f6d6d616e6473207b7d2070726f6373207b7d20626f64792d636f6465203120626f6479207b223a3a446562756750757473222069736e277420612070726f6365647572657d7d207b3a3a74636c746573743a3a44656275675075747320636f6d6d616e6473207b7d2070726f6373207b
NHO|nh20261012repair2|parent-original-name-inventory|tmm-RULE_INIT|lookup|0|0:3|once|result|binary-scan-tcl-value|3|5|7d20626f64792d636f6465203120626f6479207b223a3a74636c746573743a3a446562756750757473222069736e277420612070726f6365647572657d7d207b6672616d652d7661726961626c65206e732065786973747320312076617273206e7320747261636573207b7d7d207b6672616d652d7661726961626c652074617267657420657869737473203120766172732074617267657420747261636573207b7d7d207b6672616d652d7661726961626c65206465736372697074696f6e2065786973747320
NHO|nh20261012repair2|parent-original-name-inventory|tmm-RULE_INIT|lookup|0|0:3|once|result|binary-scan-tcl-value|4|5|312076617273206465736372697074696f6e20747261636573207b7d7d207b6672616d652d7661726961626c652070726566697820657869737473203120766172732070726566697820747261636573207b7d7d
NHO|nh20261012repair2|parent-tcltest-real-package|tmm-RULE_INIT|setup|0|0:3|once|result|binary-scan-tcl-value|0|1|
NHO|nh20261012repair2|parent-tcltest-real-package|tmm-RULE_INIT|lookup|1|0:3|once|result|binary-scan-tcl-value|0|1|63616e27742066696e64207061636b6167652074636c74657374
NHO|nh20261012repair2|parent-tcltest-real-package|tmm-RULE_INIT|lookup|1|0:3|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
NHO|nh20261012repair2|parent-tcltest-real-package|tmm-RULE_INIT|lookup|1|0:3|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
NHO|nh20261012repair2|parent-tcltest-real-package|tmm-RULE_INIT|lookup|1|0:3|once|errorinfo|binary-scan-tcl-value|0|1|63616e27742066696e64207061636b6167652074636c746573740a202020207768696c6520657865637574696e670a227061636b61676520726571756972652074636c74657374220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- version, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- version, lookup, completion 0, result, chunk 0 of 1:

```text-visible
8.4.6 8.4 ::⟦EOF⟧
```

- parent-original-name-inventory, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- parent-original-name-inventory, lookup, completion 0, result, chunk 0 of 5:

```text-visible
{::forward commands ::forward procs {} body-code 1 body {"::forward" isn't a procedure}} {::change commands {} procs {} body-code 1 body {"::change" isn't a procedure}} {::outer commands {} procs {} b⟦EOF⟧
```

- parent-original-name-inventory, lookup, completion 0, result, chunk 1 of 5:

```text-visible
ody-code 1 body {"::outer" isn't a procedure}} {::emit commands {} procs {} body-code 1 body {"::emit" isn't a procedure}} {::red commands {} procs {} body-code 1 body {"::red" isn't a procedure}} {::⟦EOF⟧
```

- parent-original-name-inventory, lookup, completion 0, result, chunk 2 of 5:

```text-visible
blue commands {} procs {} body-code 1 body {"::blue" isn't a procedure}} {::DebugPuts commands {} procs {} body-code 1 body {"::DebugPuts" isn't a procedure}} {::tcltest::DebugPuts commands {} procs {⟦EOF⟧
```

- parent-original-name-inventory, lookup, completion 0, result, chunk 3 of 5:

```text-visible
} body-code 1 body {"::tcltest::DebugPuts" isn't a procedure}} {frame-variable ns exists 1 vars ns traces {}} {frame-variable target exists 1 vars target traces {}} {frame-variable description exists⟦SPACE⟧⟦EOF⟧
```

- parent-original-name-inventory, lookup, completion 0, result, chunk 4 of 5:

```text-visible
1 vars description traces {}} {frame-variable prefix exists 1 vars prefix traces {}}⟦EOF⟧
```

- parent-tcltest-real-package, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- parent-tcltest-real-package, lookup, completion 1, result, chunk 0 of 1:

```text-visible
can't find package tcltest⟦EOF⟧
```

- parent-tcltest-real-package, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- parent-tcltest-real-package, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- parent-tcltest-real-package, lookup, completion 1, errorinfo, chunk 0 of 1:

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
