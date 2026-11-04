# BIG-IP Tcl log: nh20261012exact2, tmm-RULE_INIT-11

## Question

The following cases investigate Tcl behavior relevant to name resolution and diagnostics in the LSP, compiler and VM. Each observation is limited to its own case and recorded context.

- **oo-prefix-deleted-near-miss:** What happens to a command prefix containing an object's self command after that object is destroyed: can it still invoke the method or does command lookup fail?

## Assumptions

- **oo-prefix-deleted-near-miss:** Complete byte-identical original source evaluated in the retained host wrapper's actual namespace/frame. Ownership guard precedes the entire program; cleanup deletes only original exact names proven absent. Whole-program completion does not by itself prove that a particular nested/final lookup was reached.

Other assumptions were not separately recorded.

## Script

```tcl-visible
set nh_options_available unavailable
set nh_return_options {}
set nh_unit "[TMM::cmp_group]:[TMM::cmp_unit]"
set nh_invocation once
set nh_setup [binary format H* 736574206e685f67756172645f73636f7065205b6e616d6573706163652063757272656e745d0a736574206e685f67756172645f70726f6373207b7d0a736574206e685f67756172645f616273656e74207b7d0a666f7265616368206e685f67756172645f6e616d65207b7d207b0a202020206966207b5b6e616d6573706163652065786973747320246e685f67756172645f6e616d655d207c7c205b6c6c656e677468205b696e666f20636f6d6d616e647320246e685f67756172645f6e616d655d5d7d207b6572726f72205b6c697374206f776e6572736869702d636f6c6c6973696f6e206e616d6573706163652d6f722d636f6d6d616e6420246e685f67756172645f6e616d655d7d0a202020206c617070656e64206e685f67756172645f616273656e74205b6c697374206e616d6573706163652d616e642d636f6d6d616e6420246e685f67756172645f6e616d655d0a7d0a666f7265616368206e685f67756172645f7461696c207b7d207b0a20202020736574206e685f67756172645f6e616d65205b737472696e67207472696d726967687420246e685f67756172645f73636f7065203a5d3a3a246e685f67756172645f7461696c0a202020206966207b5b6c6c656e677468205b696e666f20636f6d6d616e647320246e685f67756172645f6e616d655d5d207c7c205b737472696e67206c656e677468205b6e616d657370616365207768696368202d636f6d6d616e6420246e685f67756172645f7461696c5d5d7d207b6572726f72205b6c697374206f776e6572736869702d636f6c6c6973696f6e2070726f63656475726520246e685f67756172645f6e616d65207265736f6c766564205b6e616d657370616365207768696368202d636f6d6d616e6420246e685f67756172645f7461696c5d5d7d0a202020206c617070656e64206e685f67756172645f70726f637320246e685f67756172645f6e616d650a202020206c617070656e64206e685f67756172645f616273656e74205b6c6973742070726f63656475726520246e685f67756172645f6e616d652066616c6c6261636b20246e685f67756172645f7461696c5d0a7d0a666f7265616368206e685f67756172645f6e616d65207b3a3a6e68636c6173737d207b0a202020206966207b5b6e616d6573706163652065786973747320246e685f67756172645f6e616d655d207c7c205b6c6c656e677468205b696e666f20636f6d6d616e647320246e685f67756172645f6e616d655d5d7d207b6572726f72205b6c697374206f776e6572736869702d636f6c6c6973696f6e20636f6d6d616e642d6f722d6e616d65737061636520246e685f67756172645f6e616d655d7d0a202020206c617070656e64206e685f67756172645f70726f637320246e685f67756172645f6e616d650a202020206c617070656e64206e685f67756172645f616273656e74205b6c69737420636f6d6d616e642d616e642d6e616d65737061636520246e685f67756172645f6e616d655d0a7d0a666f7265616368206e685f67756172645f6e616d65207b6f626a6563742070726566697820726573756c747d207b0a202020206966207b5b696e666f2065786973747320246e685f67756172645f6e616d655d207c7c205b6c6c656e677468205b696e666f207661727320246e685f67756172645f6e616d655d5d207c7c205b6c6c656e677468205b74726163652076696e666f20246e685f67756172645f6e616d655d5d7d207b6572726f72205b6c697374206f776e6572736869702d636f6c6c6973696f6e206672616d652d7661726961626c6520246e685f67756172645f6e616d655d7d0a202020206c617070656e64206e685f67756172645f616273656e74205b6c697374206672616d652d7661726961626c6520246e685f67756172645f6e616d655d0a7d0a6c697374206f776e6572736869702d677561726420657865637574696f6e2d6e616d65737061636520246e685f67756172645f73636f706520616273656e7420246e685f67756172645f616273656e740a]
set nh_operation [binary format H* 6f6f3a3a636c61737320637265617465203a3a6e68636c617373207b0a202020206d6574686f6420656d6974207b76616c75657d207b6c697374206f6b202476616c75657d0a202020206d6574686f6420707265666978207b7d207b6c697374205b73656c665d20656d6974207b6120627d7d0a7d0a736574206f626a656374205b3a3a6e68636c617373206e65775d0a73657420707265666978205b246f626a656374207072656669785d0a246f626a6563742064657374726f790a6361746368207b6576616c20247072656669787d20726573756c740a737472696e67206d61746368207b696e76616c696420636f6d6d616e64206e616d65202a7d2024726573756c740a0a]
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
        log local0. "NHO|nh20261012exact2|oo-prefix-deleted-near-miss|tmm-RULE_INIT|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012exact2|oo-prefix-deleted-near-miss|tmm-RULE_INIT|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 6966207b246e685f73657475705f636f6465203d3d20307d207b0a20202020666f7265616368206e685f67756172645f6e616d6520246e685f67756172645f70726f6373207b0a20202020202020206966207b5b6c6c656e677468205b696e666f20636f6d6d616e647320246e685f67756172645f6e616d655d5d7d207b72656e616d6520246e685f67756172645f6e616d65207b7d7d0a202020207d0a20202020666f7265616368206e685f67756172645f6e616d65207b7d207b0a20202020202020206966207b5b6e616d6573706163652065786973747320246e685f67756172645f6e616d655d7d207b6e616d6573706163652064656c65746520246e685f67756172645f6e616d657d0a202020207d0a20202020666f7265616368206e685f67756172645f6e616d65207b6f626a6563742070726566697820726573756c747d207b756e736574202d6e6f636f6d706c61696e20246e685f67756172645f6e616d657d0a20202020736574206e685f67756172645f72656d61696e696e67207b7d0a20202020666f7265616368206e685f67756172645f6e616d6520246e685f67756172645f70726f6373207b0a20202020202020206966207b5b6c6c656e677468205b696e666f20636f6d6d616e647320246e685f67756172645f6e616d655d5d7d207b6c617070656e64206e685f67756172645f72656d61696e696e67205b6c69737420636f6d6d616e6420246e685f67756172645f6e616d655d7d0a202020207d0a20202020666f7265616368206e685f67756172645f6e616d65207b7d207b0a20202020202020206966207b5b6e616d6573706163652065786973747320246e685f67756172645f6e616d655d7d207b6c617070656e64206e685f67756172645f72656d61696e696e67205b6c697374206e616d65737061636520246e685f67756172645f6e616d655d7d0a202020207d0a20202020666f7265616368206e685f67756172645f6e616d65207b6f626a6563742070726566697820726573756c747d207b0a20202020202020206966207b5b696e666f2065786973747320246e685f67756172645f6e616d655d7d207b6c617070656e64206e685f67756172645f72656d61696e696e67205b6c697374206672616d652d7661726961626c6520246e685f67756172645f6e616d655d7d0a202020207d0a202020206c697374206f776e6572736869702d636c65616e757020657865637574696f6e2d6e616d65737061636520246e685f67756172645f73636f70652072656d61696e696e6720246e685f67756172645f72656d61696e696e670a7d20656c7365207b0a202020206c697374206f776e6572736869702d6e6f742d6163717569726564206f726967696e616c2d736f757263652d756e726561636865640a7d0a]
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
        log local0. "NHO|nh20261012exact2|oo-prefix-deleted-near-miss|tmm-RULE_INIT|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
Oct 12 09:25:19 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012exact2_5_11 <RULE_INIT>: NHO|nh20261012exact2|oo-prefix-deleted-near-miss|tmm-RULE_INIT|setup|1|0:0|once|result|binary-scan-tcl-value|0|1|6f776e6572736869702d636f6c6c6973696f6e206672616d652d7661726961626c6520707265666978
Oct 12 09:25:19 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012exact2_5_11 <RULE_INIT>: NHO|nh20261012exact2|oo-prefix-deleted-near-miss|tmm-RULE_INIT|setup|1|0:0|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:25:19 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012exact2_5_11 <RULE_INIT>: NHO|nh20261012exact2|oo-prefix-deleted-near-miss|tmm-RULE_INIT|setup|1|0:0|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:25:19 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012exact2_5_11 <RULE_INIT>: NHO|nh20261012exact2|oo-prefix-deleted-near-miss|tmm-RULE_INIT|setup|1|0:0|once|errorinfo|binary-scan-tcl-value|0|3|6f776e6572736869702d636f6c6c6973696f6e206672616d652d7661726961626c65207072656669780a202020207768696c6520657865637574696e670a226572726f72205b6c697374206f776e6572736869702d636f6c6c6973696f6e206672616d652d7661726961626c6520246e685f67756172645f6e616d655d220a202020202822666f72656163682220626f6479206c696e652032290a20202020696e766f6b65642066726f6d2077697468696e0a22666f7265616368206e685f67756172645f6e616d
Oct 12 09:25:19 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012exact2_5_11 <RULE_INIT>: NHO|nh20261012exact2|oo-prefix-deleted-near-miss|tmm-RULE_INIT|setup|1|0:0|once|errorinfo|binary-scan-tcl-value|1|3|65207b6f626a6563742070726566697820726573756c747d207b0a202020206966207b5b696e666f2065786973747320246e685f67756172645f6e616d655d207c7c205b6c6c656e677468205b696e666f207661727320246e685f67756172645f6e616d655d5d207c7c205b6c6c656e677468205b74726163652076696e666f20242e2e2e220a2020202028226576616c2220626f6479206c696e65203139290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f7365747570
Oct 12 09:25:19 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012exact2_5_11 <RULE_INIT>: NHO|nh20261012exact2|oo-prefix-deleted-near-miss|tmm-RULE_INIT|setup|1|0:0|once|errorinfo|binary-scan-tcl-value|2|3|22
Oct 12 09:25:19 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012exact2_5_11 <RULE_INIT>: NHO|nh20261012exact2|oo-prefix-deleted-near-miss|tmm-RULE_INIT|runtime-cleanup|0|0:0|once|result|binary-scan-tcl-value|0|1|6f776e6572736869702d6e6f742d6163717569726564206f726967696e616c2d736f757263652d756e72656163686564
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- oo-prefix-deleted-near-miss, setup, completion 1, result, chunk 0 of 1:

```text-visible
ownership-collision frame-variable prefix⟦EOF⟧
```

- oo-prefix-deleted-near-miss, setup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- oo-prefix-deleted-near-miss, setup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- oo-prefix-deleted-near-miss, setup, completion 1, errorinfo, chunk 0 of 3:

```text-visible
ownership-collision frame-variable prefix
    while executing
"error [list ownership-collision frame-variable $nh_guard_name]"
    ("foreach" body line 2)
    invoked from within
"foreach nh_guard_nam⟦EOF⟧
```

- oo-prefix-deleted-near-miss, setup, completion 1, errorinfo, chunk 1 of 3:

```text-visible
e {object prefix result} {
    if {[info exists $nh_guard_name] || [llength [info vars $nh_guard_name]] || [llength [trace vinfo $..."
    ("eval" body line 19)
    invoked from within
"eval $nh_setup⟦EOF⟧
```

- oo-prefix-deleted-near-miss, setup, completion 1, errorinfo, chunk 2 of 3:

```text-visible
"⟦EOF⟧
```

- oo-prefix-deleted-near-miss, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
ownership-not-acquired original-source-unreached⟦EOF⟧
```

- oo-prefix-deleted-near-miss: intended operation not reached.

### tmm-RULE_INIT, unit 0:1, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 09:25:19 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012exact2_5_11 <RULE_INIT>: NHO|nh20261012exact2|oo-prefix-deleted-near-miss|tmm-RULE_INIT|setup|1|0:1|once|result|binary-scan-tcl-value|0|1|6f776e6572736869702d636f6c6c6973696f6e206672616d652d7661726961626c6520707265666978
Oct 12 09:25:19 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012exact2_5_11 <RULE_INIT>: NHO|nh20261012exact2|oo-prefix-deleted-near-miss|tmm-RULE_INIT|setup|1|0:1|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:25:19 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012exact2_5_11 <RULE_INIT>: NHO|nh20261012exact2|oo-prefix-deleted-near-miss|tmm-RULE_INIT|setup|1|0:1|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:25:19 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012exact2_5_11 <RULE_INIT>: NHO|nh20261012exact2|oo-prefix-deleted-near-miss|tmm-RULE_INIT|setup|1|0:1|once|errorinfo|binary-scan-tcl-value|0|3|6f776e6572736869702d636f6c6c6973696f6e206672616d652d7661726961626c65207072656669780a202020207768696c6520657865637574696e670a226572726f72205b6c697374206f776e6572736869702d636f6c6c6973696f6e206672616d652d7661726961626c6520246e685f67756172645f6e616d655d220a202020202822666f72656163682220626f6479206c696e652032290a20202020696e766f6b65642066726f6d2077697468696e0a22666f7265616368206e685f67756172645f6e616d
Oct 12 09:25:19 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012exact2_5_11 <RULE_INIT>: NHO|nh20261012exact2|oo-prefix-deleted-near-miss|tmm-RULE_INIT|setup|1|0:1|once|errorinfo|binary-scan-tcl-value|1|3|65207b6f626a6563742070726566697820726573756c747d207b0a202020206966207b5b696e666f2065786973747320246e685f67756172645f6e616d655d207c7c205b6c6c656e677468205b696e666f207661727320246e685f67756172645f6e616d655d5d207c7c205b6c6c656e677468205b74726163652076696e666f20242e2e2e220a2020202028226576616c2220626f6479206c696e65203139290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f7365747570
Oct 12 09:25:19 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012exact2_5_11 <RULE_INIT>: NHO|nh20261012exact2|oo-prefix-deleted-near-miss|tmm-RULE_INIT|setup|1|0:1|once|errorinfo|binary-scan-tcl-value|2|3|22
Oct 12 09:25:19 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012exact2_5_11 <RULE_INIT>: NHO|nh20261012exact2|oo-prefix-deleted-near-miss|tmm-RULE_INIT|runtime-cleanup|0|0:1|once|result|binary-scan-tcl-value|0|1|6f776e6572736869702d6e6f742d6163717569726564206f726967696e616c2d736f757263652d756e72656163686564
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- oo-prefix-deleted-near-miss, setup, completion 1, result, chunk 0 of 1:

```text-visible
ownership-collision frame-variable prefix⟦EOF⟧
```

- oo-prefix-deleted-near-miss, setup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- oo-prefix-deleted-near-miss, setup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- oo-prefix-deleted-near-miss, setup, completion 1, errorinfo, chunk 0 of 3:

```text-visible
ownership-collision frame-variable prefix
    while executing
"error [list ownership-collision frame-variable $nh_guard_name]"
    ("foreach" body line 2)
    invoked from within
"foreach nh_guard_nam⟦EOF⟧
```

- oo-prefix-deleted-near-miss, setup, completion 1, errorinfo, chunk 1 of 3:

```text-visible
e {object prefix result} {
    if {[info exists $nh_guard_name] || [llength [info vars $nh_guard_name]] || [llength [trace vinfo $..."
    ("eval" body line 19)
    invoked from within
"eval $nh_setup⟦EOF⟧
```

- oo-prefix-deleted-near-miss, setup, completion 1, errorinfo, chunk 2 of 3:

```text-visible
"⟦EOF⟧
```

- oo-prefix-deleted-near-miss, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
ownership-not-acquired original-source-unreached⟦EOF⟧
```

- oo-prefix-deleted-near-miss: intended operation not reached.

### tmm-RULE_INIT, unit 0:2, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 09:25:19 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012exact2_5_11 <RULE_INIT>: NHO|nh20261012exact2|oo-prefix-deleted-near-miss|tmm-RULE_INIT|setup|1|0:2|once|result|binary-scan-tcl-value|0|1|6f776e6572736869702d636f6c6c6973696f6e206672616d652d7661726961626c6520707265666978
Oct 12 09:25:19 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012exact2_5_11 <RULE_INIT>: NHO|nh20261012exact2|oo-prefix-deleted-near-miss|tmm-RULE_INIT|setup|1|0:2|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:25:19 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012exact2_5_11 <RULE_INIT>: NHO|nh20261012exact2|oo-prefix-deleted-near-miss|tmm-RULE_INIT|setup|1|0:2|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:25:19 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012exact2_5_11 <RULE_INIT>: NHO|nh20261012exact2|oo-prefix-deleted-near-miss|tmm-RULE_INIT|setup|1|0:2|once|errorinfo|binary-scan-tcl-value|0|3|6f776e6572736869702d636f6c6c6973696f6e206672616d652d7661726961626c65207072656669780a202020207768696c6520657865637574696e670a226572726f72205b6c697374206f776e6572736869702d636f6c6c6973696f6e206672616d652d7661726961626c6520246e685f67756172645f6e616d655d220a202020202822666f72656163682220626f6479206c696e652032290a20202020696e766f6b65642066726f6d2077697468696e0a22666f7265616368206e685f67756172645f6e616d
Oct 12 09:25:19 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012exact2_5_11 <RULE_INIT>: NHO|nh20261012exact2|oo-prefix-deleted-near-miss|tmm-RULE_INIT|setup|1|0:2|once|errorinfo|binary-scan-tcl-value|1|3|65207b6f626a6563742070726566697820726573756c747d207b0a202020206966207b5b696e666f2065786973747320246e685f67756172645f6e616d655d207c7c205b6c6c656e677468205b696e666f207661727320246e685f67756172645f6e616d655d5d207c7c205b6c6c656e677468205b74726163652076696e666f20242e2e2e220a2020202028226576616c2220626f6479206c696e65203139290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f7365747570
Oct 12 09:25:19 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012exact2_5_11 <RULE_INIT>: NHO|nh20261012exact2|oo-prefix-deleted-near-miss|tmm-RULE_INIT|setup|1|0:2|once|errorinfo|binary-scan-tcl-value|2|3|22
Oct 12 09:25:19 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012exact2_5_11 <RULE_INIT>: NHO|nh20261012exact2|oo-prefix-deleted-near-miss|tmm-RULE_INIT|runtime-cleanup|0|0:2|once|result|binary-scan-tcl-value|0|1|6f776e6572736869702d6e6f742d6163717569726564206f726967696e616c2d736f757263652d756e72656163686564
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- oo-prefix-deleted-near-miss, setup, completion 1, result, chunk 0 of 1:

```text-visible
ownership-collision frame-variable prefix⟦EOF⟧
```

- oo-prefix-deleted-near-miss, setup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- oo-prefix-deleted-near-miss, setup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- oo-prefix-deleted-near-miss, setup, completion 1, errorinfo, chunk 0 of 3:

```text-visible
ownership-collision frame-variable prefix
    while executing
"error [list ownership-collision frame-variable $nh_guard_name]"
    ("foreach" body line 2)
    invoked from within
"foreach nh_guard_nam⟦EOF⟧
```

- oo-prefix-deleted-near-miss, setup, completion 1, errorinfo, chunk 1 of 3:

```text-visible
e {object prefix result} {
    if {[info exists $nh_guard_name] || [llength [info vars $nh_guard_name]] || [llength [trace vinfo $..."
    ("eval" body line 19)
    invoked from within
"eval $nh_setup⟦EOF⟧
```

- oo-prefix-deleted-near-miss, setup, completion 1, errorinfo, chunk 2 of 3:

```text-visible
"⟦EOF⟧
```

- oo-prefix-deleted-near-miss, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
ownership-not-acquired original-source-unreached⟦EOF⟧
```

- oo-prefix-deleted-near-miss: intended operation not reached.

### tmm-RULE_INIT, unit 0:3, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 09:25:19 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012exact2_5_11 <RULE_INIT>: NHO|nh20261012exact2|oo-prefix-deleted-near-miss|tmm-RULE_INIT|setup|1|0:3|once|result|binary-scan-tcl-value|0|1|6f776e6572736869702d636f6c6c6973696f6e206672616d652d7661726961626c6520707265666978
Oct 12 09:25:19 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012exact2_5_11 <RULE_INIT>: NHO|nh20261012exact2|oo-prefix-deleted-near-miss|tmm-RULE_INIT|setup|1|0:3|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:25:19 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012exact2_5_11 <RULE_INIT>: NHO|nh20261012exact2|oo-prefix-deleted-near-miss|tmm-RULE_INIT|setup|1|0:3|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:25:19 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012exact2_5_11 <RULE_INIT>: NHO|nh20261012exact2|oo-prefix-deleted-near-miss|tmm-RULE_INIT|setup|1|0:3|once|errorinfo|binary-scan-tcl-value|0|3|6f776e6572736869702d636f6c6c6973696f6e206672616d652d7661726961626c65207072656669780a202020207768696c6520657865637574696e670a226572726f72205b6c697374206f776e6572736869702d636f6c6c6973696f6e206672616d652d7661726961626c6520246e685f67756172645f6e616d655d220a202020202822666f72656163682220626f6479206c696e652032290a20202020696e766f6b65642066726f6d2077697468696e0a22666f7265616368206e685f67756172645f6e616d
Oct 12 09:25:19 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012exact2_5_11 <RULE_INIT>: NHO|nh20261012exact2|oo-prefix-deleted-near-miss|tmm-RULE_INIT|setup|1|0:3|once|errorinfo|binary-scan-tcl-value|1|3|65207b6f626a6563742070726566697820726573756c747d207b0a202020206966207b5b696e666f2065786973747320246e685f67756172645f6e616d655d207c7c205b6c6c656e677468205b696e666f207661727320246e685f67756172645f6e616d655d5d207c7c205b6c6c656e677468205b74726163652076696e666f20242e2e2e220a2020202028226576616c2220626f6479206c696e65203139290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f7365747570
Oct 12 09:25:19 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012exact2_5_11 <RULE_INIT>: NHO|nh20261012exact2|oo-prefix-deleted-near-miss|tmm-RULE_INIT|setup|1|0:3|once|errorinfo|binary-scan-tcl-value|2|3|22
Oct 12 09:25:19 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012exact2_5_11 <RULE_INIT>: NHO|nh20261012exact2|oo-prefix-deleted-near-miss|tmm-RULE_INIT|runtime-cleanup|0|0:3|once|result|binary-scan-tcl-value|0|1|6f776e6572736869702d6e6f742d6163717569726564206f726967696e616c2d736f757263652d756e72656163686564
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- oo-prefix-deleted-near-miss, setup, completion 1, result, chunk 0 of 1:

```text-visible
ownership-collision frame-variable prefix⟦EOF⟧
```

- oo-prefix-deleted-near-miss, setup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- oo-prefix-deleted-near-miss, setup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- oo-prefix-deleted-near-miss, setup, completion 1, errorinfo, chunk 0 of 3:

```text-visible
ownership-collision frame-variable prefix
    while executing
"error [list ownership-collision frame-variable $nh_guard_name]"
    ("foreach" body line 2)
    invoked from within
"foreach nh_guard_nam⟦EOF⟧
```

- oo-prefix-deleted-near-miss, setup, completion 1, errorinfo, chunk 1 of 3:

```text-visible
e {object prefix result} {
    if {[info exists $nh_guard_name] || [llength [info vars $nh_guard_name]] || [llength [trace vinfo $..."
    ("eval" body line 19)
    invoked from within
"eval $nh_setup⟦EOF⟧
```

- oo-prefix-deleted-near-miss, setup, completion 1, errorinfo, chunk 2 of 3:

```text-visible
"⟦EOF⟧
```

- oo-prefix-deleted-near-miss, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
ownership-not-acquired original-source-unreached⟦EOF⟧
```

- oo-prefix-deleted-near-miss: intended operation not reached.
