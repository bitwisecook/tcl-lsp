# BIG-IP Tcl log: nh20261012exact1, icall-14

## Question

The following cases investigate Tcl behavior relevant to name resolution and diagnostics in the LSP, compiler and VM. Each observation is limited to its own case and recorded context.

- **tcltest-shared-lifecycle:** Do tcltest setup, body and cleanup share the expected namespace variable state, and is the setup variable absent after cleanup?

## Assumptions

- **tcltest-shared-lifecycle:** Entire-program evaluation stage only; capture ownership-guard completion does not by itself establish completion of the original program's setup prefix. Final intended lookup reachability is recorded separately.

Other assumptions were not separately recorded.

## Script

```tcl-visible
set nh_options_available unavailable
set nh_return_options {}
set nh_unit "none"
set nh_invocation once
set nh_setup [binary format H* 736574206e685f67756172645f73636f7065205b6e616d6573706163652063757272656e745d0a736574206e685f67756172645f70726f6373207b7d0a736574206e685f67756172645f616273656e74207b7d0a666f7265616368206e685f67756172645f6e616d65207b3a3a6e687d207b0a202020206966207b5b6e616d6573706163652065786973747320246e685f67756172645f6e616d655d207c7c205b6c6c656e677468205b696e666f20636f6d6d616e647320246e685f67756172645f6e616d655d5d7d207b6572726f72205b6c697374206f776e6572736869702d636f6c6c6973696f6e206e616d6573706163652d6f722d636f6d6d616e6420246e685f67756172645f6e616d655d7d0a202020206c617070656e64206e685f67756172645f616273656e74205b6c697374206e616d6573706163652d616e642d636f6d6d616e6420246e685f67756172645f6e616d655d0a7d0a666f7265616368206e685f67756172645f7461696c207b7d207b0a20202020736574206e685f67756172645f6e616d65205b737472696e67207472696d726967687420246e685f67756172645f73636f7065203a5d3a3a246e685f67756172645f7461696c0a202020206966207b5b6c6c656e677468205b696e666f20636f6d6d616e647320246e685f67756172645f6e616d655d5d207c7c205b737472696e67206c656e677468205b6e616d657370616365207768696368202d636f6d6d616e6420246e685f67756172645f7461696c5d5d7d207b6572726f72205b6c697374206f776e6572736869702d636f6c6c6973696f6e2070726f63656475726520246e685f67756172645f6e616d65207265736f6c766564205b6e616d657370616365207768696368202d636f6d6d616e6420246e685f67756172645f7461696c5d5d7d0a202020206c617070656e64206e685f67756172645f70726f637320246e685f67756172645f6e616d650a202020206c617070656e64206e685f67756172645f616273656e74205b6c6973742070726f63656475726520246e685f67756172645f6e616d652066616c6c6261636b20246e685f67756172645f7461696c5d0a7d0a666f7265616368206e685f67756172645f6e616d65207b7d207b0a202020206966207b5b6e616d6573706163652065786973747320246e685f67756172645f6e616d655d207c7c205b6c6c656e677468205b696e666f20636f6d6d616e647320246e685f67756172645f6e616d655d5d7d207b6572726f72205b6c697374206f776e6572736869702d636f6c6c6973696f6e20636f6d6d616e642d6f722d6e616d65737061636520246e685f67756172645f6e616d655d7d0a202020206c617070656e64206e685f67756172645f70726f637320246e685f67756172645f6e616d650a202020206c617070656e64206e685f67756172645f616273656e74205b6c69737420636f6d6d616e642d616e642d6e616d65737061636520246e685f67756172645f6e616d655d0a7d0a666f7265616368206e685f67756172645f6e616d65207b7d207b0a202020206966207b5b696e666f2065786973747320246e685f67756172645f6e616d655d207c7c205b6c6c656e677468205b696e666f207661727320246e685f67756172645f6e616d655d5d207c7c205b6c6c656e677468205b74726163652076696e666f20246e685f67756172645f6e616d655d5d7d207b6572726f72205b6c697374206f776e6572736869702d636f6c6c6973696f6e206672616d652d7661726961626c6520246e685f67756172645f6e616d655d7d0a202020206c617070656e64206e685f67756172645f616273656e74205b6c697374206672616d652d7661726961626c6520246e685f67756172645f6e616d655d0a7d0a6966207b5b737472696e67206c656e677468205b7061636b6167652070726f766964652074636c746573745d5d202626205b6c6c656e677468205b696e666f20636f6d6d616e6473203a3a74636c746573743a3a636f6e6669677572655d5d7d207b0a202020206966207b5b3a3a74636c746573743a3a636f6e666967757265202d6d617463685d20213d20222a22207c7c205b6c6c656e677468205b3a3a74636c746573743a3a636f6e666967757265202d736b69705d5d7d207b6572726f72207b6f776e6572736869702d636f6c6c6973696f6e207072652d6578697374696e672074636c746573742073656c656374696f6e7d7d0a7d0a6c697374206f776e6572736869702d677561726420657865637574696f6e2d6e616d65737061636520246e685f67756172645f73636f706520616273656e7420246e685f67756172645f616273656e740a]
set nh_operation [binary format H* 7061636b61676520726571756972652074636c746573740a6e616d657370616365206576616c203a3a6e68207b0a202020206e616d65737061636520696d706f7274203a3a74636c746573743a3a2a0a20202020736574206f62736572766564207b7d0a2020202074657374206e616d652d636f727075732d31207b7d202d7365747570207b0a2020202020202020736574207061796c6f6164207b6120627d0a202020207d202d626f6479207b0a20202020202020206c617070656e64206f62736572766564205b736574207061796c6f61645d0a202020207d202d636c65616e7570207b0a20202020202020206c617070656e64206f62736572766564205b736574207061796c6f61645d0a2020202020202020756e736574207061796c6f61640a202020207d202d726573756c74207b7b6120627d7d0a7d0a6c69737420243a3a6e683a3a6f62736572766564205b696e666f20657869737473203a3a6e683a3a7061796c6f61645d0a0a]
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
        puts "NHO|nh20261012exact1|tcltest-shared-lifecycle|icall|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        puts "NHO|nh20261012exact1|tcltest-shared-lifecycle|icall|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 6966207b246e685f73657475705f636f6465203d3d20307d207b0a20202020666f7265616368206e685f67756172645f6e616d6520246e685f67756172645f70726f6373207b0a20202020202020206966207b5b6c6c656e677468205b696e666f20636f6d6d616e647320246e685f67756172645f6e616d655d5d7d207b72656e616d6520246e685f67756172645f6e616d65207b7d7d0a202020207d0a20202020666f7265616368206e685f67756172645f6e616d65207b3a3a6e687d207b0a20202020202020206966207b5b6e616d6573706163652065786973747320246e685f67756172645f6e616d655d7d207b6e616d6573706163652064656c65746520246e685f67756172645f6e616d657d0a202020207d0a20202020666f7265616368206e685f67756172645f6e616d65207b7d207b756e736574202d6e6f636f6d706c61696e20246e685f67756172645f6e616d657d0a20202020736574206e685f67756172645f72656d61696e696e67207b7d0a20202020666f7265616368206e685f67756172645f6e616d6520246e685f67756172645f70726f6373207b0a20202020202020206966207b5b6c6c656e677468205b696e666f20636f6d6d616e647320246e685f67756172645f6e616d655d5d7d207b6c617070656e64206e685f67756172645f72656d61696e696e67205b6c69737420636f6d6d616e6420246e685f67756172645f6e616d655d7d0a202020207d0a20202020666f7265616368206e685f67756172645f6e616d65207b3a3a6e687d207b0a20202020202020206966207b5b6e616d6573706163652065786973747320246e685f67756172645f6e616d655d7d207b6c617070656e64206e685f67756172645f72656d61696e696e67205b6c697374206e616d65737061636520246e685f67756172645f6e616d655d7d0a202020207d0a20202020666f7265616368206e685f67756172645f6e616d65207b7d207b0a20202020202020206966207b5b696e666f2065786973747320246e685f67756172645f6e616d655d7d207b6c617070656e64206e685f67756172645f72656d61696e696e67205b6c697374206672616d652d7661726961626c6520246e685f67756172645f6e616d655d7d0a202020207d0a202020206c697374206f776e6572736869702d636c65616e757020657865637574696f6e2d6e616d65737061636520246e685f67756172645f73636f70652072656d61696e696e6720246e685f67756172645f72656d61696e696e670a7d20656c7365207b0a202020206c697374206f776e6572736869702d6e6f742d6163717569726564206f726967696e616c2d736f757263652d756e726561636865640a7d0a]
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
        puts "NHO|nh20261012exact1|tcltest-shared-lifecycle|icall|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
```

## Results

### icall, unit none, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: icall.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
NHO|nh20261012exact1|tcltest-shared-lifecycle|icall|setup|0|none|once|result|utf-8|0|1|6f776e6572736869702d677561726420657865637574696f6e2d6e616d657370616365203a3a20616273656e74207b7b6e616d6573706163652d616e642d636f6d6d616e64203a3a6e687d7d
NHO|nh20261012exact1|tcltest-shared-lifecycle|icall|lookup|1|none|once|result|utf-8|0|1|63616e2774207265616420226465627567223a206e6f2073756368207661726961626c65
NHO|nh20261012exact1|tcltest-shared-lifecycle|icall|lookup|1|none|once|metadata|utf-8|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
NHO|nh20261012exact1|tcltest-shared-lifecycle|icall|lookup|1|none|once|errorcode|utf-8|0|1|4e4f4e45
NHO|nh20261012exact1|tcltest-shared-lifecycle|icall|lookup|1|none|once|errorinfo|utf-8|0|5|63616e2774207265616420226465627567223a206e6f2073756368207661726961626c650a202020207768696c6520657865637574696e670a226966207b246465627567203e3d20246c6576656c7d207b0a09707574732024737472696e670a202020207d220a202020202870726f636564757265202244656275675075747322206c696e652033290a20202020696e766f6b65642066726f6d2077697468696e0a2244656275675075747320332022656e746572696e6720436f6e73747261696e74496e697469
NHO|nh20261012exact1|tcltest-shared-lifecycle|icall|lookup|1|none|once|errorinfo|utf-8|1|5|616c697a65722024636f6e73747261696e74202473637269707422220a202020202870726f6365647572652022436f6e73747261696e74496e697469616c697a657222206c696e652033290a20202020696e766f6b65642066726f6d2077697468696e0a22436f6e73747261696e74496e697469616c697a65722073696e676c6554657374496e74657270207b73696e676c6550726f636573737d220a202020202870726f6365647572652022446566696e65436f6e73747261696e74496e697469616c697a6572
NHO|nh20261012exact1|tcltest-shared-lifecycle|icall|lookup|1|none|once|errorinfo|utf-8|2|5|7322206c696e652032290a20202020696e766f6b65642066726f6d2077697468696e0a22446566696e65436f6e73747261696e74496e697469616c697a657273220a2020202028696e206e616d657370616365206576616c20223a3a74636c746573742220736372697074206c696e652033290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c2074636c74657374207b0a202020202320446566696e6520696e697469616c697a65727320666f7220616c6c20
NHO|nh20261012exact1|tcltest-shared-lifecycle|icall|lookup|1|none|once|errorinfo|utf-8|3|5|746865206275696c742d696e20636f6e747261696e7420646566696e6974696f6e730a20202020446566696e65436f6e73747261696e74496e697469616c697a6572730a0a2020202023205365742075702074686520636f6e73742e2e2e220a202020202866696c6520222f7573722f73686172652f636f6d7061742d74636c382e342f74636c74657374322e322f74636c746573742e74636c22206c696e652033323732290a20202020696e766f6b65642066726f6d2077697468696e0a22736f75726365202f
NHO|nh20261012exact1|tcltest-shared-lifecycle|icall|lookup|1|none|once|errorinfo|utf-8|4|5|7573722f73686172652f636f6d7061742d74636c382e342f74636c74657374322e322f74636c746573742e74636c220a2020202028227061636b6167652069666e65656465642220736372697074290a20202020696e766f6b65642066726f6d2077697468696e0a227061636b61676520726571756972652074636c74657374220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
NHO|nh20261012exact1|tcltest-shared-lifecycle|icall|runtime-cleanup|0|none|once|result|utf-8|0|1|6f776e6572736869702d636c65616e757020657865637574696f6e2d6e616d657370616365203a3a2072656d61696e696e67207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- tcltest-shared-lifecycle, setup, completion 0, result, chunk 0 of 1:

```text-visible
ownership-guard execution-namespace :: absent {{namespace-and-command ::nh}}⟦EOF⟧
```

- tcltest-shared-lifecycle, lookup, completion 1, result, chunk 0 of 1:

```text-visible
can't read "debug": no such variable⟦EOF⟧
```

- tcltest-shared-lifecycle, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- tcltest-shared-lifecycle, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- tcltest-shared-lifecycle, lookup, completion 1, errorinfo, chunk 0 of 5:

```text-visible
can't read "debug": no such variable
    while executing
"if {$debug >= $level} {
⟦TAB⟧puts $string
    }"
    (procedure "DebugPuts" line 3)
    invoked from within
"DebugPuts 3 "entering ConstraintIniti⟦EOF⟧
```

- tcltest-shared-lifecycle, lookup, completion 1, errorinfo, chunk 1 of 5:

```text-visible
alizer $constraint $script""
    (procedure "ConstraintInitializer" line 3)
    invoked from within
"ConstraintInitializer singleTestInterp {singleProcess}"
    (procedure "DefineConstraintInitializer⟦EOF⟧
```

- tcltest-shared-lifecycle, lookup, completion 1, errorinfo, chunk 2 of 5:

```text-visible
s" line 2)
    invoked from within
"DefineConstraintInitializers"
    (in namespace eval "::tcltest" script line 3)
    invoked from within
"namespace eval tcltest {
    # Define initializers for all⟦SPACE⟧⟦EOF⟧
```

- tcltest-shared-lifecycle, lookup, completion 1, errorinfo, chunk 3 of 5:

```text-visible
the built-in contraint definitions
    DefineConstraintInitializers

    # Set up the const..."
    (file "/usr/share/compat-tcl8.4/tcltest2.2/tcltest.tcl" line 3272)
    invoked from within
"source /⟦EOF⟧
```

- tcltest-shared-lifecycle, lookup, completion 1, errorinfo, chunk 4 of 5:

```text-visible
usr/share/compat-tcl8.4/tcltest2.2/tcltest.tcl"
    ("package ifneeded" script)
    invoked from within
"package require tcltest"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- tcltest-shared-lifecycle, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
ownership-cleanup execution-namespace :: remaining {}⟦EOF⟧
```

- tcltest-shared-lifecycle: intended operation not recorded.
