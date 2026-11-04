# Tcl execution evidence

## Question

> `global`, `variable`, `upvar 0/1/#0`, `namespace upvar`, eval/uplevel composition: exact target frame and cell, including namespace-eval callers.

This record addresses only the case described below; it does not answer every part of the broader question. The distinction informs name resolution and diagnostics in the LSP, compiler and VM.

> Related question F5-N06; this record retains one reviewed provider/input/context observation independently of the original C/Jim and exact portable evidence.

> What completion and result did retained variant capability-runtime-control-v1 of case namespace-upvar-link produce in stock-8.4, run nh20261012broad1, unit None, invocation 'once'?

## Assumptions

Assumptions were not separately recorded in the retained capture.

Recorded scope and limits:

> Only retained case namespace-upvar-link, variant capability-runtime-control-v1, run nh20261012broad1, context stock-8.4; event None, unit None, invocation 'once'. This independently scoped input does not replace an exact portable-input answer or establish other contexts, units, versions, recogniser coverage or source mechanisms. Authored measurement scope: Namespace-to-namespace link with explicit owner. Authored/generated source descriptor is the retained case metadata, not an unchanged portable Tcl program. This corpus receipt does not print Tcl patchlevel; see separate cap2 binary-path/version control. Binary identity was not independently hashed.. Interpreter/action facts retained in the measurement: {"interpreter_path": "/usr/bin/tclsh8.4", "interpreter_patchlevel": null, "tcltest_version": null, "execution_namespace": null, "invoking_action": null}. Null means not observed by this receipt.

## Script

```tcl-visible
set nh_unit "none"
set nh_invocation once
set nh_setup [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431305f6e616d6573706163655f75707661725f6c696e6b207b7d0a6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431305f6e616d6573706163655f75707661725f6c696e6b3a3a736f75726365207b7661726961626c652078204f524947494e414c7d0a6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431305f6e616d6573706163655f75707661725f6c696e6b3a3a636f6e73756d6572207b7d0a]
set nh_operation [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431305f6e616d6573706163655f75707661725f6c696e6b3a3a636f6e73756d6572207b6e616d657370616365207570766172203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431305f6e616d6573706163655f75707661725f6c696e6b3a3a736f75726365207820793b207365742079204348414e4745447d3b20736574203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431305f6e616d6573706163655f75707661725f6c696e6b3a3a736f757263653a3a780a]
set nh_setup_code [catch {eval $nh_setup} nh_setup_result]

set nh_ec {}
set nh_ei {}
set nh_ec_present 0
set nh_ei_present 0
if {$nh_setup_code == 1} {
    if {[info exists ::errorCode]} {set nh_ec $::errorCode; set nh_ec_present 1}
    if {[info exists ::errorInfo]} {set nh_ei $::errorInfo; set nh_ei_present 1}
}
foreach {nh_field nh_value} [list result $nh_setup_result metadata [list errorcode-present $nh_ec_present errorinfo-present $nh_ei_present return-options unavailable] errorcode $nh_ec errorinfo $nh_ei] {
    if {$nh_field != "result" && $nh_setup_code != 1} {continue}
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
        puts "NHO|nh20261012broad1|namespace-upvar-link|stock-8.4|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
foreach {nh_field nh_value} [list result $nh_result metadata [list errorcode-present $nh_ec_present errorinfo-present $nh_ei_present return-options unavailable] errorcode $nh_ec errorinfo $nh_ei] {
    if {$nh_field != "result" && $nh_code != 1} {continue}
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
        puts "NHO|nh20261012broad1|namespace-upvar-link|stock-8.4|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 666f7265616368206e685f6368696c64205b696e7465727020736c617665735d207b6966207b5b737472696e67206d61746368205f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431305f6e616d6573706163655f75707661725f6c696e6b2a20246e685f6368696c645d7d207b696e746572702064656c65746520246e685f6368696c647d7d0a666f7265616368206e685f6e73205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431305f6e616d6573706163655f75707661725f6c696e6b2a5d207b6e616d6573706163652064656c65746520246e685f6e737d0a666f7265616368206e685f636d64205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431305f6e616d6573706163655f75707661725f6c696e6b2a5d207b72656e616d6520246e685f636d64207b7d7d0a666f7265616368206e685f766172205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431305f6e616d6573706163655f75707661725f6c696e6b2a5d207b756e736574202d6e6f636f6d706c61696e20246e685f7661727d0a6c697374205b6e616d657370616365206368696c6472656e203a3a203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431305f6e616d6573706163655f75707661725f6c696e6b2a5d205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431305f6e616d6573706163655f75707661725f6c696e6b2a5d205b696e666f2076617273203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431305f6e616d6573706163655f75707661725f6c696e6b2a5d]
set nh_code [catch {eval $nh_cleanup} nh_result]

set nh_ec {}
set nh_ei {}
set nh_ec_present 0
set nh_ei_present 0
if {$nh_code == 1} {
    if {[info exists ::errorCode]} {set nh_ec $::errorCode; set nh_ec_present 1}
    if {[info exists ::errorInfo]} {set nh_ei $::errorInfo; set nh_ei_present 1}
}
foreach {nh_field nh_value} [list result $nh_result metadata [list errorcode-present $nh_ec_present errorinfo-present $nh_ei_present return-options unavailable] errorcode $nh_ec errorinfo $nh_ei] {
    if {$nh_field != "result" && $nh_code != 1} {continue}
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
        puts "NHO|nh20261012broad1|namespace-upvar-link|stock-8.4|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
```

## Results

### bigip

- Recorded version: 21.1.0.1
- Recorded input channel: Only retained case namespace-upvar-link, variant capability-runtime-control-v1, run nh20261012broad1, context stock-8.4; event None, unit None, invocation 'once'. This independently scoped input does not replace an exact portable-input answer or establish other contexts, units, versions, recogniser coverage or source mechanisms. Authored measurement scope: Namespace-to-namespace link with explicit owner. Authored/generated source descriptor is the retained case metadata, not an unchanged portable Tcl program. This corpus receipt does not print Tcl patchlevel; see separate cap2 binary-path/version control. Binary identity was not independently hashed.. Interpreter/action facts retained in the measurement: {"interpreter_path": "/usr/bin/tclsh8.4", "interpreter_patchlevel": null, "tcltest_version": null, "execution_namespace": null, "invoking_action": null}. Null means not observed by this receipt.
- Exit status: Not recorded
- Standard output:

```text-visible
NHO|nh20261012broad1|namespace-upvar-link|stock-8.4|setup|0|none|once|result|utf-8|0|1|
NHO|nh20261012broad1|namespace-upvar-link|stock-8.4|lookup|1|none|once|result|utf-8|0|1|626164206f7074696f6e20227570766172223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f72207768696368
NHO|nh20261012broad1|namespace-upvar-link|stock-8.4|lookup|1|none|once|metadata|utf-8|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
NHO|nh20261012broad1|namespace-upvar-link|stock-8.4|lookup|1|none|once|errorcode|utf-8|0|1|4e4f4e45
NHO|nh20261012broad1|namespace-upvar-link|stock-8.4|lookup|1|none|once|errorinfo|utf-8|0|4|626164206f7074696f6e20227570766172223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f722077686963680a202020207768696c6520657865637574696e670a226e616d657370616365207570766172203a3a5f5f74636c5f6c73705f
NHO|nh20261012broad1|namespace-upvar-link|stock-8.4|lookup|1|none|once|errorinfo|utf-8|1|4|70726f62655f6e68323032363130313262726f616431305f6e616d6573706163655f75707661725f6c696e6b3a3a736f7572636520782079220a2020202028696e206e616d657370616365206576616c20223a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431305f6e616d6573706163655f75707661725f6c696e6b3a3a636f6e73756d65722220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d65737061636520
NHO|nh20261012broad1|namespace-upvar-link|stock-8.4|lookup|1|none|once|errorinfo|utf-8|2|4|6576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431305f6e616d6573706163655f75707661725f6c696e6b3a3a636f6e73756d6572207b6e616d657370616365207570766172203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313262726f616431305f6e616d6573706163655f75707661722e2e2e220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20
NHO|nh20261012broad1|namespace-upvar-link|stock-8.4|lookup|1|none|once|errorinfo|utf-8|3|4|246e685f6f7065726174696f6e22
NHO|nh20261012broad1|namespace-upvar-link|stock-8.4|runtime-cleanup|0|none|once|result|utf-8|0|1|7b7d207b7d207b7d
```

- Standard error:

```text-visible
** WARNING: connection is not using a post-quantum key exchange algorithm.⟦CRLF⟧
** This session may be vulnerable to "store now, decrypt later" attacks.⟦CRLF⟧
** The server may need to be upgraded. See https://openssh.com/pq.html⟦CRLF⟧
```
