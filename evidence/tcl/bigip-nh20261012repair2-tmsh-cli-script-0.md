# BIG-IP tmsh Tcl: nh20261012repair2, tmsh-cli-script-0

## Question

This script contains the following cases, which investigate Tcl name handling relevant to the LSP, compiler and VM. Each observation is limited to its own case and this execution context.

- **parent-original-name-inventory:** Which namespace subcommands, interp forms and dynamic definition operations exist in each context? Are unsupported literal commands rejected at load even in `catch`? Compare computed head/eval.

- **parent-tcltest-real-package:** Which namespace subcommands, interp forms and dynamic definition operations exist in each context? Are unsupported literal commands rejected at load even in `catch`? Compare computed head/eval.

- **version:** Which Tcl version and execution namespace are available in this tmsh context, so that observations from this context can be scoped correctly?

## Assumptions

- **parent-original-name-inventory:** Independent capability/ownership diagnostic; no original lookup answer Intended operation: reached.
- **parent-tcltest-real-package:** Independent capability/ownership diagnostic; no original lookup answer Intended operation: reached.
- **version:** Independent capability/ownership diagnostic; no original lookup answer Intended operation: reached.

Other assumptions were not separately recorded.

## Script

```tcl-visible
set nh_options_available unavailable
set nh_return_options {}
set nh_unit "none"
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
        puts "NHO|nh20261012repair2|version|tmsh-cli-script|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        puts "NHO|nh20261012repair2|version|tmsh-cli-script|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        puts "NHO|nh20261012repair2|parent-original-name-inventory|tmsh-cli-script|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        puts "NHO|nh20261012repair2|parent-original-name-inventory|tmsh-cli-script|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        puts "NHO|nh20261012repair2|parent-tcltest-real-package|tmsh-cli-script|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        puts "NHO|nh20261012repair2|parent-tcltest-real-package|tmsh-cli-script|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
```

## Results

### BIG-IP tmsh CLI script

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmsh run cli script.
- Exit status: 0
- Standard output:

```text-visible
NHO|nh20261012repair2|version|tmsh-cli-script|setup|0|none|once|result|utf-8|0|1|
NHO|nh20261012repair2|version|tmsh-cli-script|lookup|0|none|once|result|utf-8|0|1|382e342e3620382e34203a3a736372697074
NHO|nh20261012repair2|parent-original-name-inventory|tmsh-cli-script|setup|0|none|once|result|utf-8|0|1|
NHO|nh20261012repair2|parent-original-name-inventory|tmsh-cli-script|lookup|0|none|once|result|utf-8|0|5|7b3a3a666f727761726420636f6d6d616e6473207b7d2070726f6373207b7d20626f64792d636f6465203120626f6479207b223a3a666f7277617264222069736e277420612070726f6365647572657d7d207b3a3a6368616e676520636f6d6d616e6473207b7d2070726f6373207b7d20626f64792d636f6465203120626f6479207b223a3a6368616e6765222069736e277420612070726f6365647572657d7d207b3a3a6f7574657220636f6d6d616e6473207b7d2070726f6373207b7d20626f64792d636f64
NHO|nh20261012repair2|parent-original-name-inventory|tmsh-cli-script|lookup|0|none|once|result|utf-8|1|5|65203120626f6479207b223a3a6f75746572222069736e277420612070726f6365647572657d7d207b3a3a656d697420636f6d6d616e6473207b7d2070726f6373207b7d20626f64792d636f6465203120626f6479207b223a3a656d6974222069736e277420612070726f6365647572657d7d207b3a3a72656420636f6d6d616e6473207b7d2070726f6373207b7d20626f64792d636f6465203120626f6479207b223a3a726564222069736e277420612070726f6365647572657d7d207b3a3a626c756520636f
NHO|nh20261012repair2|parent-original-name-inventory|tmsh-cli-script|lookup|0|none|once|result|utf-8|2|5|6d6d616e6473207b7d2070726f6373207b7d20626f64792d636f6465203120626f6479207b223a3a626c7565222069736e277420612070726f6365647572657d7d207b3a3a44656275675075747320636f6d6d616e6473207b7d2070726f6373207b7d20626f64792d636f6465203120626f6479207b223a3a446562756750757473222069736e277420612070726f6365647572657d7d207b3a3a74636c746573743a3a44656275675075747320636f6d6d616e6473207b7d2070726f6373207b7d20626f64792d
NHO|nh20261012repair2|parent-original-name-inventory|tmsh-cli-script|lookup|0|none|once|result|utf-8|3|5|636f6465203120626f6479207b223a3a74636c746573743a3a446562756750757473222069736e277420612070726f6365647572657d7d207b6672616d652d7661726961626c65206e732065786973747320302076617273207b7d20747261636573207b7d7d207b6672616d652d7661726961626c65207461726765742065786973747320302076617273207b7d20747261636573207b7d7d207b6672616d652d7661726961626c65206465736372697074696f6e2065786973747320302076617273207b7d2074
NHO|nh20261012repair2|parent-original-name-inventory|tmsh-cli-script|lookup|0|none|once|result|utf-8|4|5|7261636573207b7d7d207b6672616d652d7661726961626c65207072656669782065786973747320302076617273207b7d20747261636573207b7d7d
NHO|nh20261012repair2|parent-tcltest-real-package|tmsh-cli-script|setup|0|none|once|result|utf-8|0|1|
NHO|nh20261012repair2|parent-tcltest-real-package|tmsh-cli-script|lookup|1|none|once|result|utf-8|0|1|63616e2774207265616420226465627567223a206e6f2073756368207661726961626c65
NHO|nh20261012repair2|parent-tcltest-real-package|tmsh-cli-script|lookup|1|none|once|metadata|utf-8|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
NHO|nh20261012repair2|parent-tcltest-real-package|tmsh-cli-script|lookup|1|none|once|errorcode|utf-8|0|1|4e4f4e45
NHO|nh20261012repair2|parent-tcltest-real-package|tmsh-cli-script|lookup|1|none|once|errorinfo|utf-8|0|5|63616e2774207265616420226465627567223a206e6f2073756368207661726961626c650a202020207768696c6520657865637574696e670a226966207b246465627567203e3d20246c6576656c7d207b0a09707574732024737472696e670a202020207d220a202020202870726f636564757265202244656275675075747322206c696e652033290a20202020696e766f6b65642066726f6d2077697468696e0a2244656275675075747320332022656e746572696e6720436f6e73747261696e74496e697469
NHO|nh20261012repair2|parent-tcltest-real-package|tmsh-cli-script|lookup|1|none|once|errorinfo|utf-8|1|5|616c697a65722024636f6e73747261696e74202473637269707422220a202020202870726f6365647572652022436f6e73747261696e74496e697469616c697a657222206c696e652033290a20202020696e766f6b65642066726f6d2077697468696e0a22436f6e73747261696e74496e697469616c697a65722073696e676c6554657374496e74657270207b73696e676c6550726f636573737d220a202020202870726f6365647572652022446566696e65436f6e73747261696e74496e697469616c697a6572
NHO|nh20261012repair2|parent-tcltest-real-package|tmsh-cli-script|lookup|1|none|once|errorinfo|utf-8|2|5|7322206c696e652032290a20202020696e766f6b65642066726f6d2077697468696e0a22446566696e65436f6e73747261696e74496e697469616c697a657273220a2020202028696e206e616d657370616365206576616c20223a3a74636c746573742220736372697074206c696e652033290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c2074636c74657374207b0a202020202320446566696e6520696e697469616c697a65727320666f7220616c6c20
NHO|nh20261012repair2|parent-tcltest-real-package|tmsh-cli-script|lookup|1|none|once|errorinfo|utf-8|3|5|746865206275696c742d696e20636f6e747261696e7420646566696e6974696f6e730a20202020446566696e65436f6e73747261696e74496e697469616c697a6572730a0a2020202023205365742075702074686520636f6e73742e2e2e220a202020202866696c6520222f7573722f73686172652f636f6d7061742d74636c382e342f74636c74657374322e322f74636c746573742e74636c22206c696e652033323732290a20202020696e766f6b65642066726f6d2077697468696e0a22736f75726365202f
NHO|nh20261012repair2|parent-tcltest-real-package|tmsh-cli-script|lookup|1|none|once|errorinfo|utf-8|4|5|7573722f73686172652f636f6d7061742d74636c382e342f74636c74657374322e322f74636c746573742e74636c220a2020202028227061636b6167652069666e65656465642220736372697074290a20202020696e766f6b65642066726f6d2077697468696e0a227061636b61676520726571756972652074636c74657374220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
```

- Standard error: empty.

The exit status and streams are from the recorded tmsh invocation. The entries below decode complete NHO payloads from its standard output.

#### parent-original-name-inventory

- setup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
⟦EOF⟧
```

- lookup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
{::forward commands {} procs {} body-code 1 body {"::forward" isn't a procedure}} {::change commands {} procs {} body-code 1 body {"::change" isn't a procedure}} {::outer commands {} procs {} body-code 1 body {"::outer" isn't a procedure}} {::emit commands {} procs {} body-code 1 body {"::emit" isn't a procedure}} {::red commands {} procs {} body-code 1 body {"::red" isn't a procedure}} {::blue commands {} procs {} body-code 1 body {"::blue" isn't a procedure}} {::DebugPuts commands {} procs {} body-code 1 body {"::DebugPuts" isn't a procedure}} {::tcltest::DebugPuts commands {} procs {} body-code 1 body {"::tcltest::DebugPuts" isn't a procedure}} {frame-variable ns exists 0 vars {} traces {}} {frame-variable target exists 0 vars {} traces {}} {frame-variable description exists 0 vars {} traces {}} {frame-variable prefix exists 0 vars {} traces {}}⟦EOF⟧
```

#### parent-tcltest-real-package

- setup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
⟦EOF⟧
```

- lookup, completion 1, result (utf-8; unit none, invocation once):

```text-visible
can't read "debug": no such variable⟦EOF⟧
```

- lookup, completion 1, metadata (utf-8; unit none, invocation once):

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- lookup, completion 1, errorcode (utf-8; unit none, invocation once):

```text-visible
NONE⟦EOF⟧
```

- lookup, completion 1, errorinfo (utf-8; unit none, invocation once):

```text-visible
can't read "debug": no such variable
    while executing
"if {$debug >= $level} {
⟦TAB⟧puts $string
    }"
    (procedure "DebugPuts" line 3)
    invoked from within
"DebugPuts 3 "entering ConstraintInitializer $constraint $script""
    (procedure "ConstraintInitializer" line 3)
    invoked from within
"ConstraintInitializer singleTestInterp {singleProcess}"
    (procedure "DefineConstraintInitializers" line 2)
    invoked from within
"DefineConstraintInitializers"
    (in namespace eval "::tcltest" script line 3)
    invoked from within
"namespace eval tcltest {
    # Define initializers for all the built-in contraint definitions
    DefineConstraintInitializers

    # Set up the const..."
    (file "/usr/share/compat-tcl8.4/tcltest2.2/tcltest.tcl" line 3272)
    invoked from within
"source /usr/share/compat-tcl8.4/tcltest2.2/tcltest.tcl"
    ("package ifneeded" script)
    invoked from within
"package require tcltest"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

#### version

- setup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
⟦EOF⟧
```

- lookup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
8.4.6 8.4 ::script⟦EOF⟧
```
