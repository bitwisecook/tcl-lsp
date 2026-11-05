# BIG-IP stock Tcl: wiki-autoload-1

## Question

Can lookup of a missing command load its implementation from auto_index and then invoke it?

This case informs name resolution and diagnostics in the LSP, compiler and VM; it does not establish behavior in other environments.

## Assumptions

Recorded case scope: Fresh child/process; evaluate complete unchanged suite using real tcltest and actual isolated helper/constraints. A constraint skip is not unsupported proof.

Other assumptions were not separately recorded. Connection identifiers were redacted in the retained invocation.

## Script

Fixture: [bigip-stock-8.5-2](fixtures/bigip-stock-8.5-2.md)

- run: `nh20261012stock2`
- case: `wiki-autoload-1`

### nh_setup

```tcl-visible
package require tcltest
::tcltest::configure -match wiki-autoload-1
proc ::nh_body_return {cmd code value op} {set nh_invocation once;⟦SPACE⟧
set nh_ec {}
set nh_ei {}
set nh_ec_present 0
set nh_ei_present 0
if {$code == 1} {
    if {[info exists ::errorCode]} {set nh_ec $::errorCode; set nh_ec_present 1}
    if {[info exists ::errorInfo]} {set nh_ei $::errorInfo; set nh_ei_present 1}
}
foreach {nh_field nh_value} [list result $value metadata [list errorcode-present $nh_ec_present errorinfo-present $nh_ei_present return-options unavailable] errorcode $nh_ec errorinfo $nh_ei] {
    if {$nh_field != "result" && $code != 1} {continue}
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
        puts "NHO|nh20261012stock2|wiki-autoload-1|stock-8.5|body-return|$code|none|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
proc ::nh_attach {cmd code value op} {if {$code == 0 && [lindex $cmd 1] == "isolated"} {trace add execution ::isolated leave ::nh_body_return}}
trace add execution ::proc leave ::nh_attach
```

## Results

### BIG-IP stock-8.5

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel 8.5.13.
- Recorded input channel: SSH to /usr/bin/tclsh8.5, script on standard input.
- Exit status: 0
- Standard output:

```text-visible
NHO|nh20261012stock2|wiki-autoload-1|stock-8.5|setup|0|none|once|result|utf-8|0|1|
oracle 8.5.13 library /usr/share/tcl8.5 tcltest 2.3.4
NHO|nh20261012stock2|wiki-autoload-1|stock-8.5|body-return|0|none|once|result|utf-8|0|1|7b7d206c6f61646564203a3a6c617465
++++ wiki-autoload-1 PASSED
:⟦TAB⟧Total⟦TAB⟧30⟦TAB⟧Passed⟦TAB⟧1⟦TAB⟧Skipped⟦TAB⟧29⟦TAB⟧Failed⟦TAB⟧0
NHO|nh20261012stock2|wiki-autoload-1|stock-8.5|lookup|0|none|once|result|utf-8|0|1|
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

- body-return, completion 0, result (utf-8; unit none, invocation once):

```text-visible
{} loaded ::late⟦EOF⟧
```

- lookup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
⟦EOF⟧
```
