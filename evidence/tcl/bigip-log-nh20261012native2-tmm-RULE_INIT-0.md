# BIG-IP Tcl log: nh20261012native2, tmm-RULE_INIT-0

## Question

The following cases investigate Tcl behavior relevant to name resolution and diagnostics in the LSP, compiler and VM. Each observation is limited to its own case and recorded context.

- **owned-native-state-absence:** Which local/static::/qualified cells do rule-procedure calls read or write across caller and provider rules?

## Assumptions

- **owned-native-state-absence:** New exact native materialisation. Actual context/event/unit and administrative phase retained. Nested completions are list data. Fresh observer activation is distinct from continuing traffic caller; no suspended-frame claim.

Other assumptions were not separately recorded.

## Script

```tcl-visible
set nh_options_available unavailable
set nh_return_options {}
set nh_unit "[TMM::cmp_group]:[TMM::cmp_unit]"
set nh_invocation once
set nh_setup {}
set nh_operation [binary format H* 736574206e206e616d6573706163650a6966207b5b246e20657869737473203a3a5f5f6e685f6e6832303236313031326e6174697665325d7d207b6572726f72207b4f776e6564206e616d65737061636520636f6c6c6973696f6e7d7d0a666f72656163682063656c6c205b6c697374203a3a5f5f6e685f6e6832303236313031326e6174697665325f676c6f62616c207374617469633a3a5f5f6e685f6e6832303236313031326e6174697665325f737461746963207374617469633a3a5f5f6e685f6e6832303236313031326e6174697665325f6f776e65725d207b6966207b5b696e666f20657869737473202463656c6c5d207c7c205b6c6c656e677468205b74726163652076696e666f202463656c6c5d5d7d207b6572726f72205b6c697374204f776e65642d63656c6c2d636f6c6c6973696f6e202463656c6c5d7d7d0a666f726561636820746172676574205b6c697374202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665325f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665325f746172676574202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665325f6f746865723a3a5f5f6e685f6e6832303236313031326e6174697665325f746172676574202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665325f666f6c6465722f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665325f666f6c6465643a3a5f5f6e685f6e6832303236313031326e6174697665325f746172676574202f4e485f6e6832303236313031326e6174697665322f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665325f666f726569676e3a3a5f5f6e685f6e6832303236313031326e6174697665325f7461726765745d207b0a207365742063205b6361746368207b6576616c205b6c6973742063616c6c2024746172676574205f5f67756172645f5f205f5f67756172645f5f5d7d20765d0a206966207b246320213d2031207c7c20215b737472696e67206d61746368207b2a6e6f7420666f756e642a7d2024765d7d207b6572726f72205b6c697374204e61746976652d616273656e63652d6e6f742d70726f76656420247461726765742024632024765d7d0a7d0a6c69737420616273656e74203a3a5f5f6e685f6e6832303236313031326e617469766532203a3a5f5f6e685f6e6832303236313031326e6174697665325f676c6f62616c207374617469633a3a5f5f6e685f6e6832303236313031326e6174697665325f737461746963207374617469633a3a5f5f6e685f6e6832303236313031326e6174697665325f6f776e6572202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665325f70726f76696465723a3a5f5f6e685f6e6832303236313031326e6174697665325f746172676574202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665325f6f746865723a3a5f5f6e685f6e6832303236313031326e6174697665325f746172676574202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665325f666f6c6465722f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665325f666f6c6465643a3a5f5f6e685f6e6832303236313031326e6174697665325f746172676574202f4e485f6e6832303236313031326e6174697665322f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665325f666f726569676e3a3a5f5f6e685f6e6832303236313031326e6174697665325f7461726765740a]
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
        log local0. "NHO|nh20261012native2|owned-native-state-absence|tmm-RULE_INIT|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012native2|owned-native-state-absence|tmm-RULE_INIT|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
NHO|nh20261012native2|owned-native-state-absence|tmm-RULE_INIT|setup|0|0:0|once|result|binary-scan-tcl-value|0|1|
NHO|nh20261012native2|owned-native-state-absence|tmm-RULE_INIT|lookup|0|0:0|once|result|binary-scan-tcl-value|0|3|616273656e74203a3a5f5f6e685f6e6832303236313031326e617469766532203a3a5f5f6e685f6e6832303236313031326e6174697665325f676c6f62616c207374617469633a3a5f5f6e685f6e6832303236313031326e6174697665325f737461746963207374617469633a3a5f5f6e685f6e6832303236313031326e6174697665325f6f776e6572202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665325f70726f76696465723a3a5f5f6e685f6e683230
NHO|nh20261012native2|owned-native-state-absence|tmm-RULE_INIT|lookup|0|0:0|once|result|binary-scan-tcl-value|1|3|3236313031326e6174697665325f746172676574202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665325f6f746865723a3a5f5f6e685f6e6832303236313031326e6174697665325f746172676574202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665325f666f6c6465722f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665325f666f6c6465643a3a5f5f6e685f6e683230
NHO|nh20261012native2|owned-native-state-absence|tmm-RULE_INIT|lookup|0|0:0|once|result|binary-scan-tcl-value|2|3|3236313031326e6174697665325f746172676574202f4e485f6e6832303236313031326e6174697665322f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665325f666f726569676e3a3a5f5f6e685f6e6832303236313031326e6174697665325f746172676574
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- owned-native-state-absence, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- owned-native-state-absence, lookup, completion 0, result, chunk 0 of 3:

```text-visible
absent ::__nh_nh20261012native2 ::__nh_nh20261012native2_global static::__nh_nh20261012native2_static static::__nh_nh20261012native2_owner /Common/__tcl_lsp_probe_nh20261012native2_provider::__nh_nh20⟦EOF⟧
```

- owned-native-state-absence, lookup, completion 0, result, chunk 1 of 3:

```text-visible
261012native2_target /Common/__tcl_lsp_probe_nh20261012native2_other::__nh_nh20261012native2_target /Common/__tcl_lsp_probe_nh20261012native2_folder/__tcl_lsp_probe_nh20261012native2_folded::__nh_nh20⟦EOF⟧
```

- owned-native-state-absence, lookup, completion 0, result, chunk 2 of 3:

```text-visible
261012native2_target /NH_nh20261012native2/__tcl_lsp_probe_nh20261012native2_foreign::__nh_nh20261012native2_target⟦EOF⟧
```

- owned-native-state-absence: intended operation reached.

### tmm-RULE_INIT, unit 0:1, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
NHO|nh20261012native2|owned-native-state-absence|tmm-RULE_INIT|setup|0|0:1|once|result|binary-scan-tcl-value|0|1|
NHO|nh20261012native2|owned-native-state-absence|tmm-RULE_INIT|lookup|0|0:1|once|result|binary-scan-tcl-value|0|3|616273656e74203a3a5f5f6e685f6e6832303236313031326e617469766532203a3a5f5f6e685f6e6832303236313031326e6174697665325f676c6f62616c207374617469633a3a5f5f6e685f6e6832303236313031326e6174697665325f737461746963207374617469633a3a5f5f6e685f6e6832303236313031326e6174697665325f6f776e6572202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665325f70726f76696465723a3a5f5f6e685f6e683230
NHO|nh20261012native2|owned-native-state-absence|tmm-RULE_INIT|lookup|0|0:1|once|result|binary-scan-tcl-value|1|3|3236313031326e6174697665325f746172676574202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665325f6f746865723a3a5f5f6e685f6e6832303236313031326e6174697665325f746172676574202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665325f666f6c6465722f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665325f666f6c6465643a3a5f5f6e685f6e683230
NHO|nh20261012native2|owned-native-state-absence|tmm-RULE_INIT|lookup|0|0:1|once|result|binary-scan-tcl-value|2|3|3236313031326e6174697665325f746172676574202f4e485f6e6832303236313031326e6174697665322f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665325f666f726569676e3a3a5f5f6e685f6e6832303236313031326e6174697665325f746172676574
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- owned-native-state-absence, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- owned-native-state-absence, lookup, completion 0, result, chunk 0 of 3:

```text-visible
absent ::__nh_nh20261012native2 ::__nh_nh20261012native2_global static::__nh_nh20261012native2_static static::__nh_nh20261012native2_owner /Common/__tcl_lsp_probe_nh20261012native2_provider::__nh_nh20⟦EOF⟧
```

- owned-native-state-absence, lookup, completion 0, result, chunk 1 of 3:

```text-visible
261012native2_target /Common/__tcl_lsp_probe_nh20261012native2_other::__nh_nh20261012native2_target /Common/__tcl_lsp_probe_nh20261012native2_folder/__tcl_lsp_probe_nh20261012native2_folded::__nh_nh20⟦EOF⟧
```

- owned-native-state-absence, lookup, completion 0, result, chunk 2 of 3:

```text-visible
261012native2_target /NH_nh20261012native2/__tcl_lsp_probe_nh20261012native2_foreign::__nh_nh20261012native2_target⟦EOF⟧
```

- owned-native-state-absence: intended operation reached.

### tmm-RULE_INIT, unit 0:2, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
NHO|nh20261012native2|owned-native-state-absence|tmm-RULE_INIT|setup|0|0:2|once|result|binary-scan-tcl-value|0|1|
NHO|nh20261012native2|owned-native-state-absence|tmm-RULE_INIT|lookup|0|0:2|once|result|binary-scan-tcl-value|0|3|616273656e74203a3a5f5f6e685f6e6832303236313031326e617469766532203a3a5f5f6e685f6e6832303236313031326e6174697665325f676c6f62616c207374617469633a3a5f5f6e685f6e6832303236313031326e6174697665325f737461746963207374617469633a3a5f5f6e685f6e6832303236313031326e6174697665325f6f776e6572202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665325f70726f76696465723a3a5f5f6e685f6e683230
NHO|nh20261012native2|owned-native-state-absence|tmm-RULE_INIT|lookup|0|0:2|once|result|binary-scan-tcl-value|1|3|3236313031326e6174697665325f746172676574202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665325f6f746865723a3a5f5f6e685f6e6832303236313031326e6174697665325f746172676574202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665325f666f6c6465722f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665325f666f6c6465643a3a5f5f6e685f6e683230
NHO|nh20261012native2|owned-native-state-absence|tmm-RULE_INIT|lookup|0|0:2|once|result|binary-scan-tcl-value|2|3|3236313031326e6174697665325f746172676574202f4e485f6e6832303236313031326e6174697665322f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665325f666f726569676e3a3a5f5f6e685f6e6832303236313031326e6174697665325f746172676574
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- owned-native-state-absence, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- owned-native-state-absence, lookup, completion 0, result, chunk 0 of 3:

```text-visible
absent ::__nh_nh20261012native2 ::__nh_nh20261012native2_global static::__nh_nh20261012native2_static static::__nh_nh20261012native2_owner /Common/__tcl_lsp_probe_nh20261012native2_provider::__nh_nh20⟦EOF⟧
```

- owned-native-state-absence, lookup, completion 0, result, chunk 1 of 3:

```text-visible
261012native2_target /Common/__tcl_lsp_probe_nh20261012native2_other::__nh_nh20261012native2_target /Common/__tcl_lsp_probe_nh20261012native2_folder/__tcl_lsp_probe_nh20261012native2_folded::__nh_nh20⟦EOF⟧
```

- owned-native-state-absence, lookup, completion 0, result, chunk 2 of 3:

```text-visible
261012native2_target /NH_nh20261012native2/__tcl_lsp_probe_nh20261012native2_foreign::__nh_nh20261012native2_target⟦EOF⟧
```

- owned-native-state-absence: intended operation reached.

### tmm-RULE_INIT, unit 0:3, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
NHO|nh20261012native2|owned-native-state-absence|tmm-RULE_INIT|setup|0|0:3|once|result|binary-scan-tcl-value|0|1|
NHO|nh20261012native2|owned-native-state-absence|tmm-RULE_INIT|lookup|0|0:3|once|result|binary-scan-tcl-value|0|3|616273656e74203a3a5f5f6e685f6e6832303236313031326e617469766532203a3a5f5f6e685f6e6832303236313031326e6174697665325f676c6f62616c207374617469633a3a5f5f6e685f6e6832303236313031326e6174697665325f737461746963207374617469633a3a5f5f6e685f6e6832303236313031326e6174697665325f6f776e6572202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665325f70726f76696465723a3a5f5f6e685f6e683230
NHO|nh20261012native2|owned-native-state-absence|tmm-RULE_INIT|lookup|0|0:3|once|result|binary-scan-tcl-value|1|3|3236313031326e6174697665325f746172676574202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665325f6f746865723a3a5f5f6e685f6e6832303236313031326e6174697665325f746172676574202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665325f666f6c6465722f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665325f666f6c6465643a3a5f5f6e685f6e683230
NHO|nh20261012native2|owned-native-state-absence|tmm-RULE_INIT|lookup|0|0:3|once|result|binary-scan-tcl-value|2|3|3236313031326e6174697665325f746172676574202f4e485f6e6832303236313031326e6174697665322f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665325f666f726569676e3a3a5f5f6e685f6e6832303236313031326e6174697665325f746172676574
```

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- owned-native-state-absence, setup, completion 0, result, chunk 0 of 1:

```text-visible
⟦EOF⟧
```

- owned-native-state-absence, lookup, completion 0, result, chunk 0 of 3:

```text-visible
absent ::__nh_nh20261012native2 ::__nh_nh20261012native2_global static::__nh_nh20261012native2_static static::__nh_nh20261012native2_owner /Common/__tcl_lsp_probe_nh20261012native2_provider::__nh_nh20⟦EOF⟧
```

- owned-native-state-absence, lookup, completion 0, result, chunk 1 of 3:

```text-visible
261012native2_target /Common/__tcl_lsp_probe_nh20261012native2_other::__nh_nh20261012native2_target /Common/__tcl_lsp_probe_nh20261012native2_folder/__tcl_lsp_probe_nh20261012native2_folded::__nh_nh20⟦EOF⟧
```

- owned-native-state-absence, lookup, completion 0, result, chunk 2 of 3:

```text-visible
261012native2_target /NH_nh20261012native2/__tcl_lsp_probe_nh20261012native2_foreign::__nh_nh20261012native2_target⟦EOF⟧
```

- owned-native-state-absence: intended operation reached.
