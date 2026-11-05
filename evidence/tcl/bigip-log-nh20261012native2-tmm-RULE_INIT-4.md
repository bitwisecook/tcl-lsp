# BIG-IP Tcl log: nh20261012native2, tmm-RULE_INIT-4

## Question

The following cases investigate Tcl behavior relevant to name resolution and diagnostics in the LSP, compiler and VM. Each observation is limited to its own case and recorded context.

- **old-provider-shared-write-readback:** Which local/static::/qualified cells do rule-procedure calls read or write across caller and provider rules?

## Assumptions

- **old-provider-shared-write-readback:** New uniquely named native2 control, identical native operation family to native1; singleton RULE_INIT load on actual measured units. Explicit provider writes; OLD/NEW/failed replacement phases. No native-proc HTTP substitution or suspended-frame claim.

Other assumptions were not separately recorded.

## Script

```tcl-visible
set nh_options_available unavailable
set nh_return_options {}
set nh_unit "[TMM::cmp_group]:[TMM::cmp_unit]"
set nh_invocation old-provider-shared-write-readback
set nh_setup [binary format H* 736574206e685f7068617365206f6c643b20736574206e685f616374696f6e2070726f76696465722d7772697465]
set nh_operation [binary format H* 7365742070726f76696465725f76616c7565205b63616c6c202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665325f70726f76696465723a3a63656c6c7320246e685f616374696f6e20246e685f696e766f636174696f6e5d3b2073657420726f7773207b7d3b20666f72656163682063656c6c205b6c697374203a3a5f5f6e685f6e6832303236313031326e6174697665323a3a63656c6c203a3a5f5f6e685f6e6832303236313031326e6174697665325f676c6f62616c207374617469633a3a5f5f6e685f6e6832303236313031326e6174697665325f7374617469635d207b7365742063205b6361746368207b736574202463656c6c7d20765d3b206c617070656e6420726f7773202463656c6c2024632024767d3b206c69737420706861736520246e685f706861736520616374696f6e20246e685f616374696f6e2070726f7669646572202470726f76696465725f76616c75652063616c6c65722024726f7773]
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
        log local0. "NHO|nh20261012native2|old-provider-shared-write-readback|tmm-RULE_INIT|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012native2|old-provider-shared-write-readback|tmm-RULE_INIT|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
```

## Results

### tmm-RULE_INIT, unit 0:0, invocation old-provider-shared-write-readback; tmm-RULE_INIT, unit 0:1, invocation old-provider-shared-write-readback; tmm-RULE_INIT, unit 0:2, invocation old-provider-shared-write-readback

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- old-provider-shared-write-readback, setup, completion 0, result:

```text-visible
provider-write⟦EOF⟧
```

- old-provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase old action provider-write provider {rows {::__nh_nh20261012native2::cell 0 {NAMESPACE_WRITE old-provider-shared-write-readback} ::__nh_nh20261012native2_global 0 {GLOBAL_WRITE old-provider-shared-write-readback} static::__nh_nh20261012native2_static 0 {STATIC_WRITE old-provider-shared-write-readback}} level 1} caller {::__nh_nh20261012native2::cell 0 {NAMESPACE_WRITE old-provider-shared-write-readback} ::__nh_nh20261012native2_global 0 {GLOBAL_WRITE old-provider-shared-write-readback} static::__nh_nh20261012native2_static 0 {STATIC_WRITE old-provider-shared-write-readback}}⟦EOF⟧
```

- old-provider-shared-write-readback: intended operation reached.

### tmm-RULE_INIT, unit 0:3, invocation old-provider-shared-write-readback

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- old-provider-shared-write-readback, setup, completion 0, result:

```text-visible
provider-write⟦EOF⟧
```

- old-provider-shared-write-readback, lookup, completion 0, result:

```text-visible
phase old action provider-write provider {rows {::__nh_nh20261012native2::cell 0 {NAMESPACE_WRITE old-provider-shared-write-readback} ::__nh_nh20261012native2_global 0 {GLOBAL_WRITE old-provider-shared-write-readback} static::__nh_nh20261012native2_static 0 {STATIC_WRITE old-provider-shared-write-readback}} level 1} caller {::__nh_nh20261012native2::cell 0 {NAMESPACE_WRITE old-provider-shared-write-readback} ::__nh_nh20261012native2_global 0 {GLOBAL_WRITE old-provider-shared-write-readback} static::__nh_nh20261012native2_static 0 {STATIC_WRITE old-provider-shared-write-readback}}⟦EOF⟧
```

- old-provider-shared-write-readback: intended operation reached.
