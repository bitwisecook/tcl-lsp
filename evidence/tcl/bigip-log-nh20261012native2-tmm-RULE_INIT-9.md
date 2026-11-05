# BIG-IP Tcl log: nh20261012native2, tmm-RULE_INIT-9

## Question

The following cases investigate Tcl behavior relevant to name resolution and diagnostics in the LSP, compiler and VM. Each observation is limited to its own case and recorded context.

- **rejected-provider-local-upvar-write:** Which local/static::/qualified cells do rule-procedure calls read or write across caller and provider rules?

## Assumptions

- **rejected-provider-local-upvar-write:** New uniquely named native2 control, identical native operation family to native1; singleton RULE_INIT load on actual measured units. Explicit provider writes; OLD/NEW/failed replacement phases. No native-proc HTTP substitution or suspended-frame claim.

Other assumptions were not separately recorded.

## Script

```tcl-visible
set nh_options_available unavailable
set nh_return_options {}
set nh_unit "[TMM::cmp_group]:[TMM::cmp_unit]"
set nh_invocation rejected-provider-local-upvar-write
set nh_setup [binary format H* 736574206e685f70686173652072656a65637465643b20736574206e685f616374696f6e2070726f76696465722d7772697465]
set nh_operation [binary format H* 736574205f5f6e685f6e6832303236313031326e6174697665325f6c6f63616c2043414c4c45525f4245464f52453b207365742076205b63616c6c202f436f6d6d6f6e2f5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e6174697665325f70726f76696465723a3a6c696e6b205f5f6e685f6e6832303236313031326e6174697665325f6c6f63616c20246e685f696e766f636174696f6e5d3b206c6973742070726f76696465722024762063616c6c6572205b736574205f5f6e685f6e6832303236313031326e6174697665325f6c6f63616c5d]
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
        log local0. "NHO|nh20261012native2|rejected-provider-local-upvar-write|tmm-RULE_INIT|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        log local0. "NHO|nh20261012native2|rejected-provider-local-upvar-write|tmm-RULE_INIT|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
```

## Results

### tmm-RULE_INIT, unit 0:0, invocation rejected-provider-local-upvar-write; tmm-RULE_INIT, unit 0:1, invocation rejected-provider-local-upvar-write; tmm-RULE_INIT, unit 0:2, invocation rejected-provider-local-upvar-write

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- rejected-provider-local-upvar-write, setup, completion 0, result:

```text-visible
provider-write⟦EOF⟧
```

- rejected-provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL rejected-provider-local-upvar-write} before CALLER_BEFORE after {CALLER_WRITE rejected-provider-local-upvar-write} level 1} caller {CALLER_WRITE rejected-provider-local-upvar-write}⟦EOF⟧
```

- rejected-provider-local-upvar-write: intended operation reached.

### tmm-RULE_INIT, unit 0:3, invocation rejected-provider-local-upvar-write

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- rejected-provider-local-upvar-write, setup, completion 0, result:

```text-visible
provider-write⟦EOF⟧
```

- rejected-provider-local-upvar-write, lookup, completion 0, result:

```text-visible
provider {local {PROVIDER_LOCAL rejected-provider-local-upvar-write} before CALLER_BEFORE after {CALLER_WRITE rejected-provider-local-upvar-write} level 1} caller {CALLER_WRITE rejected-provider-local-upvar-write}⟦EOF⟧
```

- rejected-provider-local-upvar-write: intended operation reached.
