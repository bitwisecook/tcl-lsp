# BIG-IP Tcl log: nh20261012collision1, tmm-RULE_INIT-6

## Question

The following cases investigate Tcl behavior relevant to name resolution and diagnostics in the LSP, compiler and VM. Each observation is limited to its own case and recorded context.

- **string-prefix-near-miss:** What happens when a command name containing spaces is interpolated into an eval script string rather than preserved as a list command-prefix element?

## Assumptions

- **string-prefix-near-miss:** Complete separately adapted program in actual host. Intended original final lookup is not inferred from whole-program completion. Enumerated spelling substitutions avoid unowned native commands/frame variables.

Other assumptions were not separately recorded.

## Script

```tcl-visible
set nh_options_available unavailable
set nh_return_options {}
set nh_unit "[TMM::cmp_group]:[TMM::cmp_unit]"
set nh_invocation once
set nh_setup [binary format H* 736574206e685f67756172645f73636f7065205b6e616d6573706163652063757272656e745d0a736574206e685f67756172645f70726f6373207b7d0a736574206e685f67756172645f616273656e74207b7d0a666f7265616368206e685f67756172645f6e616d65207b3a3a6e687d207b0a202020206966207b5b6e616d6573706163652065786973747320246e685f67756172645f6e616d655d207c7c205b6c6c656e677468205b696e666f20636f6d6d616e647320246e685f67756172645f6e616d655d5d7d207b6572726f72205b6c697374206f776e6572736869702d636f6c6c6973696f6e206e616d6573706163652d6f722d636f6d6d616e6420246e685f67756172645f6e616d655d7d0a202020206c617070656e64206e685f67756172645f616273656e74205b6c697374206e616d6573706163652d616e642d636f6d6d616e6420246e685f67756172645f6e616d655d0a7d0a666f7265616368206e685f67756172645f7461696c207b7d207b0a20202020736574206e685f67756172645f6e616d65205b737472696e67207472696d726967687420246e685f67756172645f73636f7065203a5d3a3a246e685f67756172645f7461696c0a202020206966207b5b6c6c656e677468205b696e666f20636f6d6d616e647320246e685f67756172645f6e616d655d5d207c7c205b737472696e67206c656e677468205b6e616d657370616365207768696368202d636f6d6d616e6420246e685f67756172645f7461696c5d5d7d207b6572726f72205b6c697374206f776e6572736869702d636f6c6c6973696f6e2070726f63656475726520246e685f67756172645f6e616d65207265736f6c766564205b6e616d657370616365207768696368202d636f6d6d616e6420246e685f67756172645f7461696c5d5d7d0a202020206c617070656e64206e685f67756172645f70726f637320246e685f67756172645f6e616d650a202020206c617070656e64206e685f67756172645f616273656e74205b6c6973742070726f63656475726520246e685f67756172645f6e616d652066616c6c6261636b20246e685f67756172645f7461696c5d0a7d0a666f7265616368206e685f67756172645f6e616d65207b7d207b0a202020206966207b5b6e616d6573706163652065786973747320246e685f67756172645f6e616d655d207c7c205b6c6c656e677468205b696e666f20636f6d6d616e647320246e685f67756172645f6e616d655d5d7d207b6572726f72205b6c697374206f776e6572736869702d636f6c6c6973696f6e20636f6d6d616e642d6f722d6e616d65737061636520246e685f67756172645f6e616d655d7d0a202020206c617070656e64206e685f67756172645f70726f637320246e685f67756172645f6e616d650a202020206c617070656e64206e685f67756172645f616273656e74205b6c69737420636f6d6d616e642d616e642d6e616d65737061636520246e685f67756172645f6e616d655d0a7d0a666f7265616368206e685f67756172645f6e616d65207b6e685f6f776e65645f6e683230323631303132636f6c6c6973696f6e315f365f7461726765747d207b0a202020206966207b5b696e666f2065786973747320246e685f67756172645f6e616d655d207c7c205b6c6c656e677468205b696e666f207661727320246e685f67756172645f6e616d655d5d207c7c205b6c6c656e677468205b74726163652076696e666f20246e685f67756172645f6e616d655d5d7d207b6572726f72205b6c697374206f776e6572736869702d636f6c6c6973696f6e206672616d652d7661726961626c6520246e685f67756172645f6e616d655d7d0a202020206c617070656e64206e685f67756172645f616273656e74205b6c697374206672616d652d7661726961626c6520246e685f67756172645f6e616d655d0a7d0a6c697374206f776e6572736869702d677561726420657865637574696f6e2d6e616d65737061636520246e685f67756172645f73636f706520616273656e7420246e685f67756172645f616273656e740a]
set nh_operation [binary format H* 6e616d657370616365206576616c203a3a6e68207b7d0a70726f63207b3a3a6e683a3a74776f20776f7264737d207b66697865642076616c75657d207b6c69737420246669786564202476616c75657d0a736574206e685f6f776e65645f6e683230323631303132636f6c6c6973696f6e315f365f746172676574207b3a3a6e683a3a74776f20776f7264737d0a6576616c2022246e685f6f776e65645f6e683230323631303132636f6c6c6973696f6e315f365f746172676574207b6120627d207b6320647d220a0a]
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
        log local0. "NHO|nh20261012collision1|string-prefix-near-miss|tmm-RULE_INIT|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012collision1|string-prefix-near-miss|tmm-RULE_INIT|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 6966207b246e685f73657475705f636f6465203d3d20307d207b0a20202020666f7265616368206e685f67756172645f6e616d6520246e685f67756172645f70726f6373207b0a20202020202020206966207b5b6c6c656e677468205b696e666f20636f6d6d616e647320246e685f67756172645f6e616d655d5d7d207b72656e616d6520246e685f67756172645f6e616d65207b7d7d0a202020207d0a20202020666f7265616368206e685f67756172645f6e616d65207b3a3a6e687d207b0a20202020202020206966207b5b6e616d6573706163652065786973747320246e685f67756172645f6e616d655d7d207b6e616d6573706163652064656c65746520246e685f67756172645f6e616d657d0a202020207d0a20202020666f7265616368206e685f67756172645f6e616d65207b6e685f6f776e65645f6e683230323631303132636f6c6c6973696f6e315f365f7461726765747d207b756e736574202d6e6f636f6d706c61696e20246e685f67756172645f6e616d657d0a20202020666f7265616368206e685f67756172645f66616c6c6261636b207b7d207b6966207b5b6c6c656e677468205b696e666f20636f6d6d616e647320246e685f67756172645f66616c6c6261636b5d5d7d207b72656e616d6520246e685f67756172645f66616c6c6261636b207b7d7d7d0a20202020736574206e685f67756172645f72656d61696e696e67207b7d0a20202020666f7265616368206e685f67756172645f6e616d6520246e685f67756172645f70726f6373207b0a20202020202020206966207b5b6c6c656e677468205b696e666f20636f6d6d616e647320246e685f67756172645f6e616d655d5d7d207b6c617070656e64206e685f67756172645f72656d61696e696e67205b6c69737420636f6d6d616e6420246e685f67756172645f6e616d655d7d0a202020207d0a20202020666f7265616368206e685f67756172645f6e616d65207b3a3a6e687d207b0a20202020202020206966207b5b6e616d6573706163652065786973747320246e685f67756172645f6e616d655d7d207b6c617070656e64206e685f67756172645f72656d61696e696e67205b6c697374206e616d65737061636520246e685f67756172645f6e616d655d7d0a202020207d0a20202020666f7265616368206e685f67756172645f6e616d65207b6e685f6f776e65645f6e683230323631303132636f6c6c6973696f6e315f365f7461726765747d207b0a20202020202020206966207b5b696e666f2065786973747320246e685f67756172645f6e616d655d7d207b6c617070656e64206e685f67756172645f72656d61696e696e67205b6c697374206672616d652d7661726961626c6520246e685f67756172645f6e616d655d7d0a202020207d0a202020206c697374206f776e6572736869702d636c65616e757020657865637574696f6e2d6e616d65737061636520246e685f67756172645f73636f70652072656d61696e696e6720246e685f67756172645f72656d61696e696e670a7d20656c7365207b0a202020206c697374206f776e6572736869702d6e6f742d6163717569726564206f726967696e616c2d736f757263652d756e726561636865640a7d0a]
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
        log local0. "NHO|nh20261012collision1|string-prefix-near-miss|tmm-RULE_INIT|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
NHO|nh20261012collision1|string-prefix-near-miss|tmm-RULE_INIT|setup|0|0:0|once|result|binary-scan-tcl-value|0|1|6f776e6572736869702d677561726420657865637574696f6e2d6e616d657370616365203a3a20616273656e74207b7b6e616d6573706163652d616e642d636f6d6d616e64203a3a6e687d207b6672616d652d7661726961626c65206e685f6f776e65645f6e683230323631303132636f6c6c6973696f6e315f365f7461726765747d7d
NHO|nh20261012collision1|string-prefix-near-miss|tmm-RULE_INIT|lookup|1|0:0|once|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520223a3a6e683a3a74776f22
NHO|nh20261012collision1|string-prefix-near-miss|tmm-RULE_INIT|lookup|1|0:0|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
NHO|nh20261012collision1|string-prefix-near-miss|tmm-RULE_INIT|lookup|1|0:0|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
NHO|nh20261012collision1|string-prefix-near-miss|tmm-RULE_INIT|lookup|1|0:0|once|errorinfo|binary-scan-tcl-value|0|2|696e76616c696420636f6d6d616e64206e616d6520223a3a6e683a3a74776f220a202020207768696c6520657865637574696e670a223a3a6e683a3a74776f20776f726473207b6120627d207b6320647d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c2022246e685f6f776e65645f6e683230323631303132636f6c6c6973696f6e315f365f746172676574207b6120627d207b6320647d22220a20202020282265
NHO|nh20261012collision1|string-prefix-near-miss|tmm-RULE_INIT|lookup|1|0:0|once|errorinfo|binary-scan-tcl-value|1|2|76616c2220626f6479206c696e652034290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
NHO|nh20261012collision1|string-prefix-near-miss|tmm-RULE_INIT|runtime-cleanup|0|0:0|once|result|binary-scan-tcl-value|0|1|6f776e6572736869702d636c65616e757020657865637574696f6e2d6e616d657370616365203a3a2072656d61696e696e67207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- string-prefix-near-miss, setup, completion 0, result, chunk 0 of 1:

```text-visible
ownership-guard execution-namespace :: absent {{namespace-and-command ::nh} {frame-variable nh_owned_nh20261012collision1_6_target}}⟦EOF⟧
```

- string-prefix-near-miss, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "::nh::two"⟦EOF⟧
```

- string-prefix-near-miss, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- string-prefix-near-miss, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- string-prefix-near-miss, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
invalid command name "::nh::two"
    while executing
"::nh::two words {a b} {c d}"
    ("eval" body line 1)
    invoked from within
"eval "$nh_owned_nh20261012collision1_6_target {a b} {c d}""
    ("e⟦EOF⟧
```

- string-prefix-near-miss, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
val" body line 4)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- string-prefix-near-miss, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
ownership-cleanup execution-namespace :: remaining {}⟦EOF⟧
```

- string-prefix-near-miss: intended operation reached.

### tmm-RULE_INIT, unit 0:1, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
NHO|nh20261012collision1|string-prefix-near-miss|tmm-RULE_INIT|setup|0|0:1|once|result|binary-scan-tcl-value|0|1|6f776e6572736869702d677561726420657865637574696f6e2d6e616d657370616365203a3a20616273656e74207b7b6e616d6573706163652d616e642d636f6d6d616e64203a3a6e687d207b6672616d652d7661726961626c65206e685f6f776e65645f6e683230323631303132636f6c6c6973696f6e315f365f7461726765747d7d
NHO|nh20261012collision1|string-prefix-near-miss|tmm-RULE_INIT|lookup|1|0:1|once|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520223a3a6e683a3a74776f22
NHO|nh20261012collision1|string-prefix-near-miss|tmm-RULE_INIT|lookup|1|0:1|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
NHO|nh20261012collision1|string-prefix-near-miss|tmm-RULE_INIT|lookup|1|0:1|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
NHO|nh20261012collision1|string-prefix-near-miss|tmm-RULE_INIT|lookup|1|0:1|once|errorinfo|binary-scan-tcl-value|0|2|696e76616c696420636f6d6d616e64206e616d6520223a3a6e683a3a74776f220a202020207768696c6520657865637574696e670a223a3a6e683a3a74776f20776f726473207b6120627d207b6320647d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c2022246e685f6f776e65645f6e683230323631303132636f6c6c6973696f6e315f365f746172676574207b6120627d207b6320647d22220a20202020282265
NHO|nh20261012collision1|string-prefix-near-miss|tmm-RULE_INIT|lookup|1|0:1|once|errorinfo|binary-scan-tcl-value|1|2|76616c2220626f6479206c696e652034290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
NHO|nh20261012collision1|string-prefix-near-miss|tmm-RULE_INIT|runtime-cleanup|0|0:1|once|result|binary-scan-tcl-value|0|1|6f776e6572736869702d636c65616e757020657865637574696f6e2d6e616d657370616365203a3a2072656d61696e696e67207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- string-prefix-near-miss, setup, completion 0, result, chunk 0 of 1:

```text-visible
ownership-guard execution-namespace :: absent {{namespace-and-command ::nh} {frame-variable nh_owned_nh20261012collision1_6_target}}⟦EOF⟧
```

- string-prefix-near-miss, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "::nh::two"⟦EOF⟧
```

- string-prefix-near-miss, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- string-prefix-near-miss, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- string-prefix-near-miss, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
invalid command name "::nh::two"
    while executing
"::nh::two words {a b} {c d}"
    ("eval" body line 1)
    invoked from within
"eval "$nh_owned_nh20261012collision1_6_target {a b} {c d}""
    ("e⟦EOF⟧
```

- string-prefix-near-miss, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
val" body line 4)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- string-prefix-near-miss, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
ownership-cleanup execution-namespace :: remaining {}⟦EOF⟧
```

- string-prefix-near-miss: intended operation reached.

### tmm-RULE_INIT, unit 0:2, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
NHO|nh20261012collision1|string-prefix-near-miss|tmm-RULE_INIT|setup|0|0:2|once|result|binary-scan-tcl-value|0|1|6f776e6572736869702d677561726420657865637574696f6e2d6e616d657370616365203a3a20616273656e74207b7b6e616d6573706163652d616e642d636f6d6d616e64203a3a6e687d207b6672616d652d7661726961626c65206e685f6f776e65645f6e683230323631303132636f6c6c6973696f6e315f365f7461726765747d7d
NHO|nh20261012collision1|string-prefix-near-miss|tmm-RULE_INIT|lookup|1|0:2|once|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520223a3a6e683a3a74776f22
NHO|nh20261012collision1|string-prefix-near-miss|tmm-RULE_INIT|lookup|1|0:2|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
NHO|nh20261012collision1|string-prefix-near-miss|tmm-RULE_INIT|lookup|1|0:2|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
NHO|nh20261012collision1|string-prefix-near-miss|tmm-RULE_INIT|lookup|1|0:2|once|errorinfo|binary-scan-tcl-value|0|2|696e76616c696420636f6d6d616e64206e616d6520223a3a6e683a3a74776f220a202020207768696c6520657865637574696e670a223a3a6e683a3a74776f20776f726473207b6120627d207b6320647d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c2022246e685f6f776e65645f6e683230323631303132636f6c6c6973696f6e315f365f746172676574207b6120627d207b6320647d22220a20202020282265
NHO|nh20261012collision1|string-prefix-near-miss|tmm-RULE_INIT|lookup|1|0:2|once|errorinfo|binary-scan-tcl-value|1|2|76616c2220626f6479206c696e652034290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
NHO|nh20261012collision1|string-prefix-near-miss|tmm-RULE_INIT|runtime-cleanup|0|0:2|once|result|binary-scan-tcl-value|0|1|6f776e6572736869702d636c65616e757020657865637574696f6e2d6e616d657370616365203a3a2072656d61696e696e67207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- string-prefix-near-miss, setup, completion 0, result, chunk 0 of 1:

```text-visible
ownership-guard execution-namespace :: absent {{namespace-and-command ::nh} {frame-variable nh_owned_nh20261012collision1_6_target}}⟦EOF⟧
```

- string-prefix-near-miss, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "::nh::two"⟦EOF⟧
```

- string-prefix-near-miss, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- string-prefix-near-miss, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- string-prefix-near-miss, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
invalid command name "::nh::two"
    while executing
"::nh::two words {a b} {c d}"
    ("eval" body line 1)
    invoked from within
"eval "$nh_owned_nh20261012collision1_6_target {a b} {c d}""
    ("e⟦EOF⟧
```

- string-prefix-near-miss, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
val" body line 4)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- string-prefix-near-miss, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
ownership-cleanup execution-namespace :: remaining {}⟦EOF⟧
```

- string-prefix-near-miss: intended operation reached.

### tmm-RULE_INIT, unit 0:3, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
NHO|nh20261012collision1|string-prefix-near-miss|tmm-RULE_INIT|setup|0|0:3|once|result|binary-scan-tcl-value|0|1|6f776e6572736869702d677561726420657865637574696f6e2d6e616d657370616365203a3a20616273656e74207b7b6e616d6573706163652d616e642d636f6d6d616e64203a3a6e687d207b6672616d652d7661726961626c65206e685f6f776e65645f6e683230323631303132636f6c6c6973696f6e315f365f7461726765747d7d
NHO|nh20261012collision1|string-prefix-near-miss|tmm-RULE_INIT|lookup|1|0:3|once|result|binary-scan-tcl-value|0|1|696e76616c696420636f6d6d616e64206e616d6520223a3a6e683a3a74776f22
NHO|nh20261012collision1|string-prefix-near-miss|tmm-RULE_INIT|lookup|1|0:3|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
NHO|nh20261012collision1|string-prefix-near-miss|tmm-RULE_INIT|lookup|1|0:3|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
NHO|nh20261012collision1|string-prefix-near-miss|tmm-RULE_INIT|lookup|1|0:3|once|errorinfo|binary-scan-tcl-value|0|2|696e76616c696420636f6d6d616e64206e616d6520223a3a6e683a3a74776f220a202020207768696c6520657865637574696e670a223a3a6e683a3a74776f20776f726473207b6120627d207b6320647d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c2022246e685f6f776e65645f6e683230323631303132636f6c6c6973696f6e315f365f746172676574207b6120627d207b6320647d22220a20202020282265
NHO|nh20261012collision1|string-prefix-near-miss|tmm-RULE_INIT|lookup|1|0:3|once|errorinfo|binary-scan-tcl-value|1|2|76616c2220626f6479206c696e652034290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
NHO|nh20261012collision1|string-prefix-near-miss|tmm-RULE_INIT|runtime-cleanup|0|0:3|once|result|binary-scan-tcl-value|0|1|6f776e6572736869702d636c65616e757020657865637574696f6e2d6e616d657370616365203a3a2072656d61696e696e67207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- string-prefix-near-miss, setup, completion 0, result, chunk 0 of 1:

```text-visible
ownership-guard execution-namespace :: absent {{namespace-and-command ::nh} {frame-variable nh_owned_nh20261012collision1_6_target}}⟦EOF⟧
```

- string-prefix-near-miss, lookup, completion 1, result, chunk 0 of 1:

```text-visible
invalid command name "::nh::two"⟦EOF⟧
```

- string-prefix-near-miss, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- string-prefix-near-miss, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- string-prefix-near-miss, lookup, completion 1, errorinfo, chunk 0 of 2:

```text-visible
invalid command name "::nh::two"
    while executing
"::nh::two words {a b} {c d}"
    ("eval" body line 1)
    invoked from within
"eval "$nh_owned_nh20261012collision1_6_target {a b} {c d}""
    ("e⟦EOF⟧
```

- string-prefix-near-miss, lookup, completion 1, errorinfo, chunk 1 of 2:

```text-visible
val" body line 4)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- string-prefix-near-miss, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
ownership-cleanup execution-namespace :: remaining {}⟦EOF⟧
```

- string-prefix-near-miss: intended operation reached.
