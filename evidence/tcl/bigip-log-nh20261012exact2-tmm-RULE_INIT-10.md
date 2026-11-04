# BIG-IP Tcl log: nh20261012exact2, tmm-RULE_INIT-10

## Question

The following cases investigate Tcl behavior relevant to name resolution and diagnostics in the LSP, compiler and VM. Each observation is limited to its own case and recorded context.

- **namespace-captured-prefix:** Does a namespace code prefix retain the defining namespace when evaluated elsewhere, including after a same-named global command is defined, and preserve its argument elements?

## Assumptions

- **namespace-captured-prefix:** The byte-identical original program was evaluated sequentially. Complete recorded binary-scan errorInfo bytes contain reversible ASCII quoted-command and eval-body-line markers locating the authored final operation. Reaching that source line establishes that preceding authored top-level commands completed without propagating a nonzero completion. Code1/result belongs to evaluating the final operation, including substitutions; an argument-substitution failure does not imply outer command dispatch. ASCII is an explicit reversible byte projection for locating stack markers only. The original result/error encodings remain binary-scan-tcl-value; no UTF8 or Tcl Unicode projection is claimed. Successful setup means prior source evaluation completed, not that every expected binding exists. Ownership collisions and earlier program errors remain unpromoted; this review applies only to this exact identity.

Other assumptions were not separately recorded.

## Script

```tcl-visible
set nh_options_available unavailable
set nh_return_options {}
set nh_unit "[TMM::cmp_group]:[TMM::cmp_unit]"
set nh_invocation once
set nh_setup [binary format H* 736574206e685f67756172645f73636f7065205b6e616d6573706163652063757272656e745d0a736574206e685f67756172645f70726f6373207b7d0a736574206e685f67756172645f616273656e74207b7d0a666f7265616368206e685f67756172645f6e616d65207b3a3a6e687d207b0a202020206966207b5b6e616d6573706163652065786973747320246e685f67756172645f6e616d655d207c7c205b6c6c656e677468205b696e666f20636f6d6d616e647320246e685f67756172645f6e616d655d5d7d207b6572726f72205b6c697374206f776e6572736869702d636f6c6c6973696f6e206e616d6573706163652d6f722d636f6d6d616e6420246e685f67756172645f6e616d655d7d0a202020206c617070656e64206e685f67756172645f616273656e74205b6c697374206e616d6573706163652d616e642d636f6d6d616e6420246e685f67756172645f6e616d655d0a7d0a666f7265616368206e685f67756172645f7461696c207b656d69747d207b0a20202020736574206e685f67756172645f6e616d65205b737472696e67207472696d726967687420246e685f67756172645f73636f7065203a5d3a3a246e685f67756172645f7461696c0a202020206966207b5b6c6c656e677468205b696e666f20636f6d6d616e647320246e685f67756172645f6e616d655d5d207c7c205b737472696e67206c656e677468205b6e616d657370616365207768696368202d636f6d6d616e6420246e685f67756172645f7461696c5d5d7d207b6572726f72205b6c697374206f776e6572736869702d636f6c6c6973696f6e2070726f63656475726520246e685f67756172645f6e616d65207265736f6c766564205b6e616d657370616365207768696368202d636f6d6d616e6420246e685f67756172645f7461696c5d5d7d0a202020206c617070656e64206e685f67756172645f70726f637320246e685f67756172645f6e616d650a202020206c617070656e64206e685f67756172645f616273656e74205b6c6973742070726f63656475726520246e685f67756172645f6e616d652066616c6c6261636b20246e685f67756172645f7461696c5d0a7d0a666f7265616368206e685f67756172645f6e616d65207b7d207b0a202020206966207b5b6e616d6573706163652065786973747320246e685f67756172645f6e616d655d207c7c205b6c6c656e677468205b696e666f20636f6d6d616e647320246e685f67756172645f6e616d655d5d7d207b6572726f72205b6c697374206f776e6572736869702d636f6c6c6973696f6e20636f6d6d616e642d6f722d6e616d65737061636520246e685f67756172645f6e616d655d7d0a202020206c617070656e64206e685f67756172645f70726f637320246e685f67756172645f6e616d650a202020206c617070656e64206e685f67756172645f616273656e74205b6c69737420636f6d6d616e642d616e642d6e616d65737061636520246e685f67756172645f6e616d655d0a7d0a666f7265616368206e685f67756172645f6e616d65207b7d207b0a202020206966207b5b696e666f2065786973747320246e685f67756172645f6e616d655d207c7c205b6c6c656e677468205b696e666f207661727320246e685f67756172645f6e616d655d5d207c7c205b6c6c656e677468205b74726163652076696e666f20246e685f67756172645f6e616d655d5d7d207b6572726f72205b6c697374206f776e6572736869702d636f6c6c6973696f6e206672616d652d7661726961626c6520246e685f67756172645f6e616d655d7d0a202020206c617070656e64206e685f67756172645f616273656e74205b6c697374206672616d652d7661726961626c6520246e685f67756172645f6e616d655d0a7d0a6c697374206f776e6572736869702d677561726420657865637574696f6e2d6e616d65737061636520246e685f67756172645f73636f706520616273656e7420246e685f67756172645f616273656e740a]
set nh_operation [binary format H* 6e616d657370616365206576616c203a3a6e68207b0a2020202070726f6320656d6974207b76616c756520617267737d207b6c697374206c6f63616c202476616c75652024617267737d0a2020202073657420707265666978205b6e616d65737061636520636f6465205b6c69737420656d6974207b6120627d5d5d0a7d0a70726f6320656d6974207b617267737d207b72657475726e20676c6f62616c7d0a6576616c205b6c696e7365727420243a3a6e683a3a70726566697820656e64207b6320647d5d0a0a]
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
        log local0. "NHO|nh20261012exact2|namespace-captured-prefix|tmm-RULE_INIT|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012exact2|namespace-captured-prefix|tmm-RULE_INIT|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012exact2|namespace-captured-prefix|tmm-RULE_INIT|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
Oct 12 09:25:12 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012exact2_5_10 <RULE_INIT>: NHO|nh20261012exact2|namespace-captured-prefix|tmm-RULE_INIT|setup|0|0:0|once|result|binary-scan-tcl-value|0|1|6f776e6572736869702d677561726420657865637574696f6e2d6e616d657370616365203a3a20616273656e74207b7b6e616d6573706163652d616e642d636f6d6d616e64203a3a6e687d207b70726f636564757265203a3a656d69742066616c6c6261636b20656d69747d7d
Oct 12 09:25:12 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012exact2_5_10 <RULE_INIT>: NHO|nh20261012exact2|namespace-captured-prefix|tmm-RULE_INIT|lookup|1|0:0|once|result|binary-scan-tcl-value|0|1|63616e2774207265616420223a3a6e683a3a707265666978223a206e6f2073756368207661726961626c65
Oct 12 09:25:12 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012exact2_5_10 <RULE_INIT>: NHO|nh20261012exact2|namespace-captured-prefix|tmm-RULE_INIT|lookup|1|0:0|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:25:12 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012exact2_5_10 <RULE_INIT>: NHO|nh20261012exact2|namespace-captured-prefix|tmm-RULE_INIT|lookup|1|0:0|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:25:12 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012exact2_5_10 <RULE_INIT>: NHO|nh20261012exact2|namespace-captured-prefix|tmm-RULE_INIT|lookup|1|0:0|once|errorinfo|binary-scan-tcl-value|0|1|63616e2774207265616420223a3a6e683a3a707265666978223a206e6f2073756368207661726961626c650a202020207768696c6520657865637574696e670a226c696e7365727420243a3a6e683a3a70726566697820656e64207b6320647d220a2020202028226576616c2220626f6479206c696e652036290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 09:25:12 bigip.bitwisecook.org info tmm[16500]: Rule /Common/__tcl_lsp_probe_nh20261012exact2_5_10 <RULE_INIT>: NHO|nh20261012exact2|namespace-captured-prefix|tmm-RULE_INIT|runtime-cleanup|0|0:0|once|result|binary-scan-tcl-value|0|1|6f776e6572736869702d636c65616e757020657865637574696f6e2d6e616d657370616365203a3a2072656d61696e696e67207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- namespace-captured-prefix, setup, completion 0, result, chunk 0 of 1:

```text-visible
ownership-guard execution-namespace :: absent {{namespace-and-command ::nh} {procedure ::emit fallback emit}}⟦EOF⟧
```

- namespace-captured-prefix, lookup, completion 1, result, chunk 0 of 1:

```text-visible
can't read "::nh::prefix": no such variable⟦EOF⟧
```

- namespace-captured-prefix, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- namespace-captured-prefix, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- namespace-captured-prefix, lookup, completion 1, errorinfo, chunk 0 of 1:

```text-visible
can't read "::nh::prefix": no such variable
    while executing
"linsert $::nh::prefix end {c d}"
    ("eval" body line 6)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- namespace-captured-prefix, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
ownership-cleanup execution-namespace :: remaining {}⟦EOF⟧
```

- namespace-captured-prefix: intended operation reached.

### tmm-RULE_INIT, unit 0:1, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 09:25:12 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012exact2_5_10 <RULE_INIT>: NHO|nh20261012exact2|namespace-captured-prefix|tmm-RULE_INIT|setup|0|0:1|once|result|binary-scan-tcl-value|0|1|6f776e6572736869702d677561726420657865637574696f6e2d6e616d657370616365203a3a20616273656e74207b7b6e616d6573706163652d616e642d636f6d6d616e64203a3a6e687d207b70726f636564757265203a3a656d69742066616c6c6261636b20656d69747d7d
Oct 12 09:25:12 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012exact2_5_10 <RULE_INIT>: NHO|nh20261012exact2|namespace-captured-prefix|tmm-RULE_INIT|lookup|1|0:1|once|result|binary-scan-tcl-value|0|1|63616e2774207265616420223a3a6e683a3a707265666978223a206e6f2073756368207661726961626c65
Oct 12 09:25:12 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012exact2_5_10 <RULE_INIT>: NHO|nh20261012exact2|namespace-captured-prefix|tmm-RULE_INIT|lookup|1|0:1|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:25:12 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012exact2_5_10 <RULE_INIT>: NHO|nh20261012exact2|namespace-captured-prefix|tmm-RULE_INIT|lookup|1|0:1|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:25:12 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012exact2_5_10 <RULE_INIT>: NHO|nh20261012exact2|namespace-captured-prefix|tmm-RULE_INIT|lookup|1|0:1|once|errorinfo|binary-scan-tcl-value|0|1|63616e2774207265616420223a3a6e683a3a707265666978223a206e6f2073756368207661726961626c650a202020207768696c6520657865637574696e670a226c696e7365727420243a3a6e683a3a70726566697820656e64207b6320647d220a2020202028226576616c2220626f6479206c696e652036290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 09:25:12 bigip.bitwisecook.org info tmm1[16500]: Rule /Common/__tcl_lsp_probe_nh20261012exact2_5_10 <RULE_INIT>: NHO|nh20261012exact2|namespace-captured-prefix|tmm-RULE_INIT|runtime-cleanup|0|0:1|once|result|binary-scan-tcl-value|0|1|6f776e6572736869702d636c65616e757020657865637574696f6e2d6e616d657370616365203a3a2072656d61696e696e67207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- namespace-captured-prefix, setup, completion 0, result, chunk 0 of 1:

```text-visible
ownership-guard execution-namespace :: absent {{namespace-and-command ::nh} {procedure ::emit fallback emit}}⟦EOF⟧
```

- namespace-captured-prefix, lookup, completion 1, result, chunk 0 of 1:

```text-visible
can't read "::nh::prefix": no such variable⟦EOF⟧
```

- namespace-captured-prefix, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- namespace-captured-prefix, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- namespace-captured-prefix, lookup, completion 1, errorinfo, chunk 0 of 1:

```text-visible
can't read "::nh::prefix": no such variable
    while executing
"linsert $::nh::prefix end {c d}"
    ("eval" body line 6)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- namespace-captured-prefix, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
ownership-cleanup execution-namespace :: remaining {}⟦EOF⟧
```

- namespace-captured-prefix: intended operation reached.

### tmm-RULE_INIT, unit 0:2, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 09:25:12 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012exact2_5_10 <RULE_INIT>: NHO|nh20261012exact2|namespace-captured-prefix|tmm-RULE_INIT|setup|0|0:2|once|result|binary-scan-tcl-value|0|1|6f776e6572736869702d677561726420657865637574696f6e2d6e616d657370616365203a3a20616273656e74207b7b6e616d6573706163652d616e642d636f6d6d616e64203a3a6e687d207b70726f636564757265203a3a656d69742066616c6c6261636b20656d69747d7d
Oct 12 09:25:12 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012exact2_5_10 <RULE_INIT>: NHO|nh20261012exact2|namespace-captured-prefix|tmm-RULE_INIT|lookup|1|0:2|once|result|binary-scan-tcl-value|0|1|63616e2774207265616420223a3a6e683a3a707265666978223a206e6f2073756368207661726961626c65
Oct 12 09:25:12 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012exact2_5_10 <RULE_INIT>: NHO|nh20261012exact2|namespace-captured-prefix|tmm-RULE_INIT|lookup|1|0:2|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:25:12 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012exact2_5_10 <RULE_INIT>: NHO|nh20261012exact2|namespace-captured-prefix|tmm-RULE_INIT|lookup|1|0:2|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:25:12 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012exact2_5_10 <RULE_INIT>: NHO|nh20261012exact2|namespace-captured-prefix|tmm-RULE_INIT|lookup|1|0:2|once|errorinfo|binary-scan-tcl-value|0|1|63616e2774207265616420223a3a6e683a3a707265666978223a206e6f2073756368207661726961626c650a202020207768696c6520657865637574696e670a226c696e7365727420243a3a6e683a3a70726566697820656e64207b6320647d220a2020202028226576616c2220626f6479206c696e652036290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 09:25:12 bigip.bitwisecook.org info tmm2[16500]: Rule /Common/__tcl_lsp_probe_nh20261012exact2_5_10 <RULE_INIT>: NHO|nh20261012exact2|namespace-captured-prefix|tmm-RULE_INIT|runtime-cleanup|0|0:2|once|result|binary-scan-tcl-value|0|1|6f776e6572736869702d636c65616e757020657865637574696f6e2d6e616d657370616365203a3a2072656d61696e696e67207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- namespace-captured-prefix, setup, completion 0, result, chunk 0 of 1:

```text-visible
ownership-guard execution-namespace :: absent {{namespace-and-command ::nh} {procedure ::emit fallback emit}}⟦EOF⟧
```

- namespace-captured-prefix, lookup, completion 1, result, chunk 0 of 1:

```text-visible
can't read "::nh::prefix": no such variable⟦EOF⟧
```

- namespace-captured-prefix, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- namespace-captured-prefix, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- namespace-captured-prefix, lookup, completion 1, errorinfo, chunk 0 of 1:

```text-visible
can't read "::nh::prefix": no such variable
    while executing
"linsert $::nh::prefix end {c d}"
    ("eval" body line 6)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- namespace-captured-prefix, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
ownership-cleanup execution-namespace :: remaining {}⟦EOF⟧
```

- namespace-captured-prefix: intended operation reached.

### tmm-RULE_INIT, unit 0:3, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Oct 12 09:25:12 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012exact2_5_10 <RULE_INIT>: NHO|nh20261012exact2|namespace-captured-prefix|tmm-RULE_INIT|setup|0|0:3|once|result|binary-scan-tcl-value|0|1|6f776e6572736869702d677561726420657865637574696f6e2d6e616d657370616365203a3a20616273656e74207b7b6e616d6573706163652d616e642d636f6d6d616e64203a3a6e687d207b70726f636564757265203a3a656d69742066616c6c6261636b20656d69747d7d
Oct 12 09:25:12 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012exact2_5_10 <RULE_INIT>: NHO|nh20261012exact2|namespace-captured-prefix|tmm-RULE_INIT|lookup|1|0:3|once|result|binary-scan-tcl-value|0|1|63616e2774207265616420223a3a6e683a3a707265666978223a206e6f2073756368207661726961626c65
Oct 12 09:25:12 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012exact2_5_10 <RULE_INIT>: NHO|nh20261012exact2|namespace-captured-prefix|tmm-RULE_INIT|lookup|1|0:3|once|metadata|binary-scan-tcl-value|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
Oct 12 09:25:12 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012exact2_5_10 <RULE_INIT>: NHO|nh20261012exact2|namespace-captured-prefix|tmm-RULE_INIT|lookup|1|0:3|once|errorcode|binary-scan-tcl-value|0|1|4e4f4e45
Oct 12 09:25:12 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012exact2_5_10 <RULE_INIT>: NHO|nh20261012exact2|namespace-captured-prefix|tmm-RULE_INIT|lookup|1|0:3|once|errorinfo|binary-scan-tcl-value|0|1|63616e2774207265616420223a3a6e683a3a707265666978223a206e6f2073756368207661726961626c650a202020207768696c6520657865637574696e670a226c696e7365727420243a3a6e683a3a70726566697820656e64207b6320647d220a2020202028226576616c2220626f6479206c696e652036290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
Oct 12 09:25:12 bigip.bitwisecook.org info tmm3[16500]: Rule /Common/__tcl_lsp_probe_nh20261012exact2_5_10 <RULE_INIT>: NHO|nh20261012exact2|namespace-captured-prefix|tmm-RULE_INIT|runtime-cleanup|0|0:3|once|result|binary-scan-tcl-value|0|1|6f776e6572736869702d636c65616e757020657865637574696f6e2d6e616d657370616365203a3a2072656d61696e696e67207b7d
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- namespace-captured-prefix, setup, completion 0, result, chunk 0 of 1:

```text-visible
ownership-guard execution-namespace :: absent {{namespace-and-command ::nh} {procedure ::emit fallback emit}}⟦EOF⟧
```

- namespace-captured-prefix, lookup, completion 1, result, chunk 0 of 1:

```text-visible
can't read "::nh::prefix": no such variable⟦EOF⟧
```

- namespace-captured-prefix, lookup, completion 1, metadata, chunk 0 of 1:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- namespace-captured-prefix, lookup, completion 1, errorcode, chunk 0 of 1:

```text-visible
NONE⟦EOF⟧
```

- namespace-captured-prefix, lookup, completion 1, errorinfo, chunk 0 of 1:

```text-visible
can't read "::nh::prefix": no such variable
    while executing
"linsert $::nh::prefix end {c d}"
    ("eval" body line 6)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- namespace-captured-prefix, runtime-cleanup, completion 0, result, chunk 0 of 1:

```text-visible
ownership-cleanup execution-namespace :: remaining {}⟦EOF⟧
```

- namespace-captured-prefix: intended operation reached.
