# BIG-IP APL callback: namespace-missing-write-local

## Question

Qualified/relative-qualified variables and namespace-scope missing-read fallback: read/write/create/exists differences.

This callback addresses only the `namespace-missing-write-local` case in that broader question. Its result can inform name resolution and diagnostics in the LSP, compiler and VM, but does not establish the behavior of other cases or environments.

## Assumptions

Recorded case scope: Namespace write/create contrast; clean global variable ::__tcl_lsp_probe_nh20261012aplbroad1_namespace_missing_write_local_x.

Other assumptions were not separately recorded. The APL callback was selected through the browser; this is not an iApp implementation, an iRule, or a stock Tcl measurement. A successful caught cleanup does not prove that every cleanup operation succeeded.

## Script

```tcl-visible
set nh_apl_rows {}
set nh_options_available unavailable
set nh_return_options {}
set nh_unit "none"
set nh_invocation once
set nh_setup [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c62726f6164315f6e616d6573706163655f6d697373696e675f77726974655f6c6f63616c207b7d0a736574203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c62726f6164315f6e616d6573706163655f6d697373696e675f77726974655f6c6f63616c5f7820474c4f42414c5f56414c55450a6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c62726f6164315f6e616d6573706163655f6d697373696e675f77726974655f6c6f63616c207b7d0a]
set nh_operation [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c62726f6164315f6e616d6573706163655f6d697373696e675f77726974655f6c6f63616c207b736574205f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c62726f6164315f6e616d6573706163655f6d697373696e675f77726974655f6c6f63616c5f78204c4f43414c5f56414c55457d3b206c697374205b736574203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c62726f6164315f6e616d6573706163655f6d697373696e675f77726974655f6c6f63616c5f785d205b736574203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c62726f6164315f6e616d6573706163655f6d697373696e675f77726974655f6c6f63616c3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c62726f6164315f6e616d6573706163655f6d697373696e675f77726974655f6c6f63616c5f785d0a]
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
        lappend nh_apl_rows "NHO|nh20261012aplbroad1|namespace-missing-write-local|iapp-apl|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        lappend nh_apl_rows "NHO|nh20261012aplbroad1|namespace-missing-write-local|iapp-apl|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 666f7265616368206e685f6368696c64205b696e7465727020736c617665735d207b6966207b5b737472696e67206d61746368205f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c62726f6164315f6e616d6573706163655f6d697373696e675f77726974655f6c6f63616c2a20246e685f6368696c645d7d207b696e746572702064656c65746520246e685f6368696c647d7d0a666f7265616368206e685f6e73205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c62726f6164315f6e616d6573706163655f6d697373696e675f77726974655f6c6f63616c2a5d207b6e616d6573706163652064656c65746520246e685f6e737d0a666f7265616368206e685f636d64205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c62726f6164315f6e616d6573706163655f6d697373696e675f77726974655f6c6f63616c2a5d207b72656e616d6520246e685f636d64207b7d7d0a666f7265616368206e685f766172205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c62726f6164315f6e616d6573706163655f6d697373696e675f77726974655f6c6f63616c2a5d207b756e736574202d6e6f636f6d706c61696e20246e685f7661727d0a6c697374205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c62726f6164315f6e616d6573706163655f6d697373696e675f77726974655f6c6f63616c2a5d205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c62726f6164315f6e616d6573706163655f6d697373696e675f77726974655f6c6f63616c2a5d205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c62726f6164315f6e616d6573706163655f6d697373696e675f77726974655f6c6f63616c2a5d]
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
        lappend nh_apl_rows "NHO|nh20261012aplbroad1|namespace-missing-write-local|iapp-apl|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}

return [join $nh_apl_rows "\n"]
```

## Results

### BIG-IP iApp APL

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel 8.4.6.
- Recorded input channel: browser Application Services template choice callback.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
NHO|nh20261012aplbroad1|namespace-missing-write-local|iapp-apl|setup|0|none|once|result|utf-8|0|1|
NHO|nh20261012aplbroad1|namespace-missing-write-local|iapp-apl|lookup|1|none|once|result|utf-8|0|1|63616e2774207265616420223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c62726f6164315f6e616d6573706163655f6d697373696e675f77726974655f6c6f63616c3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c62726f6164315f6e616d6573706163655f6d697373696e675f77726974655f6c6f63616c5f78223a206e6f2073756368207661726961626c65
NHO|nh20261012aplbroad1|namespace-missing-write-local|iapp-apl|lookup|1|none|once|metadata|utf-8|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
NHO|nh20261012aplbroad1|namespace-missing-write-local|iapp-apl|lookup|1|none|once|errorcode|utf-8|0|1|4e4f4e45
NHO|nh20261012aplbroad1|namespace-missing-write-local|iapp-apl|lookup|1|none|once|errorinfo|utf-8|0|2|63616e2774207265616420223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c62726f6164315f6e616d6573706163655f6d697373696e675f77726974655f6c6f63616c3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c62726f6164315f6e616d6573706163655f6d697373696e675f77726974655f6c6f63616c5f78223a206e6f2073756368207661726961626c650a202020207768696c6520657865637574696e670a22736574203a3a5f5f74636c
NHO|nh20261012aplbroad1|namespace-missing-write-local|iapp-apl|lookup|1|none|once|errorinfo|utf-8|1|2|5f6c73705f70726f62655f6e68323032363130313261706c62726f6164315f6e616d6573706163655f6d697373696e675f77726974655f6c6f63616c3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c62726f6164315f6e616d6573706163655f6d697373696e675f77726974655f6c6f63616c5f78220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
NHO|nh20261012aplbroad1|namespace-missing-write-local|iapp-apl|runtime-cleanup|0|none|once|result|utf-8|0|1|7b7d207b7d207b7d
```

These are the retained DOM option values projected one row per value, not process stdout. The fields below decode their hexadecimal payloads; completion codes are the callback's caught Tcl codes.

- setup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
⟦EOF⟧
```

- lookup, completion 1, result (utf-8; unit none, invocation once):

```text-visible
can't read "::__tcl_lsp_probe_nh20261012aplbroad1_namespace_missing_write_local::__tcl_lsp_probe_nh20261012aplbroad1_namespace_missing_write_local_x": no such variable⟦EOF⟧
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
can't read "::__tcl_lsp_probe_nh20261012aplbroad1_namespace_missing_write_local::__tcl_lsp_probe_nh20261012aplbroad1_namespace_missing_write_local_x": no such variable
    while executing
"set ::__tcl_lsp_probe_nh20261012aplbroad1_namespace_missing_write_local::__tcl_lsp_probe_nh20261012aplbroad1_namespace_missing_write_local_x"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- runtime-cleanup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
{} {} {}⟦EOF⟧
```
