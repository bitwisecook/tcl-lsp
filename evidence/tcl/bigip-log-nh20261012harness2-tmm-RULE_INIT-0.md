# BIG-IP Tcl log: nh20261012harness2, tmm-RULE_INIT-0

## Question

The following cases investigate Tcl behavior relevant to name resolution and diagnostics in the LSP, compiler and VM. Each observation is limited to its own case and recorded context.

- **real-harness-explicit-source-child:** Which namespace subcommands, interp forms and dynamic definition operations exist in each context? Are unsupported literal commands rejected at load even in `catch`? Compare computed head/eval.

## Assumptions

- **real-harness-explicit-source-child:** Independent child-interpreter real-package capability. Does not substitute for native-parent package require or original suite.

Other assumptions were not separately recorded.

## Script

```tcl-visible
set nh_options_available unavailable
set nh_return_options {}
set nh_unit "[TMM::cmp_group]:[TMM::cmp_unit]"
set nh_invocation once
set nh_setup [binary format H* 736574206e685f6368696c64205b696e74657270206372656174655d3b20736574206e685f6d6f64756c65207b7d3b20617070656e64206e685f6d6f64756c65205b63616c6c202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326861726e657373325f305f6d6f64756c65303a3a62797465735d205b63616c6c202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326861726e657373325f305f6d6f64756c65313a3a62797465735d205b63616c6c202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326861726e657373325f305f6d6f64756c65323a3a62797465735d205b63616c6c202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326861726e657373325f305f6d6f64756c65333a3a62797465735d205b63616c6c202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326861726e657373325f305f6d6f64756c65343a3a62797465735d205b63616c6c202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326861726e657373325f305f6d6f64756c65353a3a62797465735d205b63616c6c202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326861726e657373325f305f6d6f64756c65363a3a62797465735d205b63616c6c202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326861726e657373325f305f6d6f64756c65373a3a62797465735d205b63616c6c202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326861726e657373325f305f6d6f64756c65383a3a62797465735d3b206c697374206368696c642d70726f7669646572205b696e74657270206576616c20246e685f6368696c64207b6c697374205b696e666f2070617463686c6576656c5d205b696e666f2074636c76657273696f6e5d205b6e616d6573706163652063757272656e745d7d5d206d6f64756c652d6279746573205b737472696e67206c656e67746820246e685f6d6f64756c655d]
set nh_operation [binary format H* 696e74657270206576616c20246e685f6368696c6420246e685f6d6f64756c65]
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
        log local0. "NHO|nh20261012harness2|real-harness-explicit-source-child|tmm-RULE_INIT|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012harness2|real-harness-explicit-source-child|tmm-RULE_INIT|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 696e746572702064656c65746520246e685f6368696c64]
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
        log local0. "NHO|nh20261012harness2|real-harness-explicit-source-child|tmm-RULE_INIT|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
NHO|nh20261012harness2|real-harness-explicit-source-child|tmm-RULE_INIT|setup|0|0:0|once|result|binary-scan-tcl-value|0|1|6368696c642d70726f7669646572207b382e342e3620382e34203a3a7d206d6f64756c652d6279746573203938373633
NHO|nh20261012harness2|real-harness-explicit-source-child|tmm-RULE_INIT|lookup|1|0:0|once|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d65202270776422
NHO|nh20261012harness2|real-harness-explicit-source-child|tmm-RULE_INIT|lookup|1|0:0|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
NHO|nh20261012harness2|real-harness-explicit-source-child|tmm-RULE_INIT|lookup|1|0:0|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
NHO|nh20261012harness2|real-harness-explicit-source-child|tmm-RULE_INIT|lookup|1|0:0|once|errorinfo|binary-scan-tcl-value|0|3|696e76616c696420636f6d6d616e64206e616d652022707764220a202020207768696c6520657865637574696e670a22707764220a2020202028696e206e616d657370616365206576616c20223a3a74636c746573742220736372697074206c696e6520323435290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c2074636c74657374207b0a0a2020202023205768656e207468652076657273696f6e206e756d626572206368616e6765732c206265207375
NHO|nh20261012harness2|real-harness-explicit-source-child|tmm-RULE_INIT|lookup|1|0:0|once|errorinfo|binary-scan-tcl-value|1|3|726520746f207570646174652074686520706b67496e6465782e74636c2066696c652c0a202020202320616e642074686520696e7374616c6c206469726563746f727920696e20746865204d616b65662e2e2e220a20202020696e766f6b65642066726f6d2077697468696e0a22696e74657270206576616c20246e685f6368696c6420246e685f6d6f64756c65220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c2024
NHO|nh20261012harness2|real-harness-explicit-source-child|tmm-RULE_INIT|lookup|1|0:0|once|errorinfo|binary-scan-tcl-value|2|3|6e685f6f7065726174696f6e22
NHO|nh20261012harness2|real-harness-explicit-source-child|tmm-RULE_INIT|runtime-cleanup|0|0:0|once|result|binary-scan-tcl-value|0|1|
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- real-harness-explicit-source-child, setup, completion 0, result, chunk 0 of 1:

```text-visible
child-provider {8.4.6 8.4 ::} module-bytes 98763⟦EOF⟧
```

- real-harness-explicit-source-child, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "pwd"⟦EOF⟧
```

- real-harness-explicit-source-child, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- real-harness-explicit-source-child, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- real-harness-explicit-source-child, lookup, completion 1, errorinfo, chunk 0 of 3:

```text-visible
invalid command name "pwd"
    while executing
"pwd"
    (in namespace eval "::tcltest" script line 245)
    invoked from within
"namespace eval tcltest {

    # When the version number changes, be su⟦EOF⟧
```

- real-harness-explicit-source-child, lookup, completion 1, errorinfo, chunk 1 of 3:

```text-visible
re to update the pkgIndex.tcl file,
    # and the install directory in the Makef..."
    invoked from within
"interp eval $nh_child $nh_module"
    ("eval" body line 1)
    invoked from within
"eval $⟦EOF⟧
```

- real-harness-explicit-source-child, lookup, completion 1, errorinfo, chunk 2 of 3:

```text-visible
nh_operation"⟦EOF⟧
```

- real-harness-explicit-source-child, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- real-harness-explicit-source-child: intended operation reached.

### tmm-RULE_INIT, unit 0:1, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
NHO|nh20261012harness2|real-harness-explicit-source-child|tmm-RULE_INIT|setup|0|0:1|once|result|binary-scan-tcl-value|0|1|6368696c642d70726f7669646572207b382e342e3620382e34203a3a7d206d6f64756c652d6279746573203938373633
NHO|nh20261012harness2|real-harness-explicit-source-child|tmm-RULE_INIT|lookup|1|0:1|once|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d65202270776422
NHO|nh20261012harness2|real-harness-explicit-source-child|tmm-RULE_INIT|lookup|1|0:1|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
NHO|nh20261012harness2|real-harness-explicit-source-child|tmm-RULE_INIT|lookup|1|0:1|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
NHO|nh20261012harness2|real-harness-explicit-source-child|tmm-RULE_INIT|lookup|1|0:1|once|errorinfo|binary-scan-tcl-value|0|3|696e76616c696420636f6d6d616e64206e616d652022707764220a202020207768696c6520657865637574696e670a22707764220a2020202028696e206e616d657370616365206576616c20223a3a74636c746573742220736372697074206c696e6520323435290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c2074636c74657374207b0a0a2020202023205768656e207468652076657273696f6e206e756d626572206368616e6765732c206265207375
NHO|nh20261012harness2|real-harness-explicit-source-child|tmm-RULE_INIT|lookup|1|0:1|once|errorinfo|binary-scan-tcl-value|1|3|726520746f207570646174652074686520706b67496e6465782e74636c2066696c652c0a202020202320616e642074686520696e7374616c6c206469726563746f727920696e20746865204d616b65662e2e2e220a20202020696e766f6b65642066726f6d2077697468696e0a22696e74657270206576616c20246e685f6368696c6420246e685f6d6f64756c65220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c2024
NHO|nh20261012harness2|real-harness-explicit-source-child|tmm-RULE_INIT|lookup|1|0:1|once|errorinfo|binary-scan-tcl-value|2|3|6e685f6f7065726174696f6e22
NHO|nh20261012harness2|real-harness-explicit-source-child|tmm-RULE_INIT|runtime-cleanup|0|0:1|once|result|binary-scan-tcl-value|0|1|
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- real-harness-explicit-source-child, setup, completion 0, result, chunk 0 of 1:

```text-visible
child-provider {8.4.6 8.4 ::} module-bytes 98763⟦EOF⟧
```

- real-harness-explicit-source-child, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "pwd"⟦EOF⟧
```

- real-harness-explicit-source-child, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- real-harness-explicit-source-child, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- real-harness-explicit-source-child, lookup, completion 1, errorinfo, chunk 0 of 3:

```text-visible
invalid command name "pwd"
    while executing
"pwd"
    (in namespace eval "::tcltest" script line 245)
    invoked from within
"namespace eval tcltest {

    # When the version number changes, be su⟦EOF⟧
```

- real-harness-explicit-source-child, lookup, completion 1, errorinfo, chunk 1 of 3:

```text-visible
re to update the pkgIndex.tcl file,
    # and the install directory in the Makef..."
    invoked from within
"interp eval $nh_child $nh_module"
    ("eval" body line 1)
    invoked from within
"eval $⟦EOF⟧
```

- real-harness-explicit-source-child, lookup, completion 1, errorinfo, chunk 2 of 3:

```text-visible
nh_operation"⟦EOF⟧
```

- real-harness-explicit-source-child, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- real-harness-explicit-source-child: intended operation reached.

### tmm-RULE_INIT, unit 0:2, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
NHO|nh20261012harness2|real-harness-explicit-source-child|tmm-RULE_INIT|setup|0|0:2|once|result|binary-scan-tcl-value|0|1|6368696c642d70726f7669646572207b382e342e3620382e34203a3a7d206d6f64756c652d6279746573203938373633
NHO|nh20261012harness2|real-harness-explicit-source-child|tmm-RULE_INIT|lookup|1|0:2|once|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d65202270776422
NHO|nh20261012harness2|real-harness-explicit-source-child|tmm-RULE_INIT|lookup|1|0:2|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
NHO|nh20261012harness2|real-harness-explicit-source-child|tmm-RULE_INIT|lookup|1|0:2|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
NHO|nh20261012harness2|real-harness-explicit-source-child|tmm-RULE_INIT|lookup|1|0:2|once|errorinfo|binary-scan-tcl-value|0|3|696e76616c696420636f6d6d616e64206e616d652022707764220a202020207768696c6520657865637574696e670a22707764220a2020202028696e206e616d657370616365206576616c20223a3a74636c746573742220736372697074206c696e6520323435290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c2074636c74657374207b0a0a2020202023205768656e207468652076657273696f6e206e756d626572206368616e6765732c206265207375
NHO|nh20261012harness2|real-harness-explicit-source-child|tmm-RULE_INIT|lookup|1|0:2|once|errorinfo|binary-scan-tcl-value|1|3|726520746f207570646174652074686520706b67496e6465782e74636c2066696c652c0a202020202320616e642074686520696e7374616c6c206469726563746f727920696e20746865204d616b65662e2e2e220a20202020696e766f6b65642066726f6d2077697468696e0a22696e74657270206576616c20246e685f6368696c6420246e685f6d6f64756c65220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c2024
NHO|nh20261012harness2|real-harness-explicit-source-child|tmm-RULE_INIT|lookup|1|0:2|once|errorinfo|binary-scan-tcl-value|2|3|6e685f6f7065726174696f6e22
NHO|nh20261012harness2|real-harness-explicit-source-child|tmm-RULE_INIT|runtime-cleanup|0|0:2|once|result|binary-scan-tcl-value|0|1|
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- real-harness-explicit-source-child, setup, completion 0, result, chunk 0 of 1:

```text-visible
child-provider {8.4.6 8.4 ::} module-bytes 98763⟦EOF⟧
```

- real-harness-explicit-source-child, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "pwd"⟦EOF⟧
```

- real-harness-explicit-source-child, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- real-harness-explicit-source-child, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- real-harness-explicit-source-child, lookup, completion 1, errorinfo, chunk 0 of 3:

```text-visible
invalid command name "pwd"
    while executing
"pwd"
    (in namespace eval "::tcltest" script line 245)
    invoked from within
"namespace eval tcltest {

    # When the version number changes, be su⟦EOF⟧
```

- real-harness-explicit-source-child, lookup, completion 1, errorinfo, chunk 1 of 3:

```text-visible
re to update the pkgIndex.tcl file,
    # and the install directory in the Makef..."
    invoked from within
"interp eval $nh_child $nh_module"
    ("eval" body line 1)
    invoked from within
"eval $⟦EOF⟧
```

- real-harness-explicit-source-child, lookup, completion 1, errorinfo, chunk 2 of 3:

```text-visible
nh_operation"⟦EOF⟧
```

- real-harness-explicit-source-child, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- real-harness-explicit-source-child: intended operation reached.

### tmm-RULE_INIT, unit 0:3, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
NHO|nh20261012harness2|real-harness-explicit-source-child|tmm-RULE_INIT|setup|0|0:3|once|result|binary-scan-tcl-value|0|1|6368696c642d70726f7669646572207b382e342e3620382e34203a3a7d206d6f64756c652d6279746573203938373633
NHO|nh20261012harness2|real-harness-explicit-source-child|tmm-RULE_INIT|lookup|1|0:3|once|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d65202270776422
NHO|nh20261012harness2|real-harness-explicit-source-child|tmm-RULE_INIT|lookup|1|0:3|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
NHO|nh20261012harness2|real-harness-explicit-source-child|tmm-RULE_INIT|lookup|1|0:3|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
NHO|nh20261012harness2|real-harness-explicit-source-child|tmm-RULE_INIT|lookup|1|0:3|once|errorinfo|binary-scan-tcl-value|0|3|696e76616c696420636f6d6d616e64206e616d652022707764220a202020207768696c6520657865637574696e670a22707764220a2020202028696e206e616d657370616365206576616c20223a3a74636c746573742220736372697074206c696e6520323435290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c2074636c74657374207b0a0a2020202023205768656e207468652076657273696f6e206e756d626572206368616e6765732c206265207375
NHO|nh20261012harness2|real-harness-explicit-source-child|tmm-RULE_INIT|lookup|1|0:3|once|errorinfo|binary-scan-tcl-value|1|3|726520746f207570646174652074686520706b67496e6465782e74636c2066696c652c0a202020202320616e642074686520696e7374616c6c206469726563746f727920696e20746865204d616b65662e2e2e220a20202020696e766f6b65642066726f6d2077697468696e0a22696e74657270206576616c20246e685f6368696c6420246e685f6d6f64756c65220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c2024
NHO|nh20261012harness2|real-harness-explicit-source-child|tmm-RULE_INIT|lookup|1|0:3|once|errorinfo|binary-scan-tcl-value|2|3|6e685f6f7065726174696f6e22
NHO|nh20261012harness2|real-harness-explicit-source-child|tmm-RULE_INIT|runtime-cleanup|0|0:3|once|result|binary-scan-tcl-value|0|1|
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- real-harness-explicit-source-child, setup, completion 0, result, chunk 0 of 1:

```text-visible
child-provider {8.4.6 8.4 ::} module-bytes 98763⟦EOF⟧
```

- real-harness-explicit-source-child, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "pwd"⟦EOF⟧
```

- real-harness-explicit-source-child, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- real-harness-explicit-source-child, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- real-harness-explicit-source-child, lookup, completion 1, errorinfo, chunk 0 of 3:

```text-visible
invalid command name "pwd"
    while executing
"pwd"
    (in namespace eval "::tcltest" script line 245)
    invoked from within
"namespace eval tcltest {

    # When the version number changes, be su⟦EOF⟧
```

- real-harness-explicit-source-child, lookup, completion 1, errorinfo, chunk 1 of 3:

```text-visible
re to update the pkgIndex.tcl file,
    # and the install directory in the Makef..."
    invoked from within
"interp eval $nh_child $nh_module"
    ("eval" body line 1)
    invoked from within
"eval $⟦EOF⟧
```

- real-harness-explicit-source-child, lookup, completion 1, errorinfo, chunk 2 of 3:

```text-visible
nh_operation"⟦EOF⟧
```

- real-harness-explicit-source-child, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- real-harness-explicit-source-child: intended operation reached.
