# BIG-IP stock Tcl: ensemble-prefix-map

## Question

Does an ensemble subcommand mapping preserve the elements of a list command prefix, including a fixed argument containing a space, when passing a new argument?

This case informs name resolution and diagnostics in the LSP, compiler and VM; it does not establish behavior in other environments.

## Assumptions

Recorded case scope: Complete original source in fresh stock child interpreter/process; source is exact, wrapper/isolation channel must still be retained.

Other assumptions were not separately recorded. Connection identifiers were redacted in the retained invocation.

## Script

```tcl-visible
set nh_unit "none"
set nh_invocation once
set nh_setup {}
set nh_operation [binary format H* 6e616d657370616365206576616c203a3a6e68207b0a2020202070726f6320656d6974207b66697865642076616c75657d207b6c69737420246669786564202476616c75657d0a202020206e616d65737061636520656e73656d626c6520637265617465202d636f6d6d616e6420666163616465202d6d6170205b6c6973742073656e64205b6c697374203a3a6e683a3a656d6974207b6120627d5d5d0a7d0a3a3a6e683a3a6661636164652073656e64207b6320647d0a0a]
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
        puts "NHO|nh20261012pat1|ensemble-prefix-map|stock-8.5|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        puts "NHO|nh20261012pat1|ensemble-prefix-map|stock-8.5|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
```

## Results

### BIG-IP stock-8.5

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: SSH to /usr/bin/tclsh8.5, script on standard input.
- Exit status: 0
- Standard output:

```text-visible
NHO|nh20261012pat1|ensemble-prefix-map|stock-8.5|setup|0|none|once|result|utf-8|0|1|
NHO|nh20261012pat1|ensemble-prefix-map|stock-8.5|lookup|0|none|once|result|utf-8|0|1|7b6120627d207b6320647d
```

- Standard error:

```text-visible
** WARNING: connection is not using a post-quantum key exchange algorithm.⟦CRLF⟧
** This session may be vulnerable to "store now, decrypt later" attacks.⟦CRLF⟧
** The server may need to be upgraded. See https://openssh.com/pq.html⟦CRLF⟧
```


The exit status and streams are from the SSH invocation; standard error can include SSH transport messages as well as remote errors. The entries below decode complete NHO result payloads from standard output.

- setup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
⟦EOF⟧
```

- lookup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
{a b} {c d}⟦EOF⟧
```
