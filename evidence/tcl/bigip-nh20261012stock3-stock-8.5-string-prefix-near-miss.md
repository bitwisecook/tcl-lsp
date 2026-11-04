# BIG-IP stock Tcl: string-prefix-near-miss

## Question

What happens when a command name containing spaces is interpolated into an eval script string rather than preserved as a list command-prefix element?

This case informs name resolution and diagnostics in the LSP, compiler and VM; it does not establish behavior in other environments.

## Assumptions

Recorded case scope: Original authored names and concatenated source bytes; setup/final command split in a fresh stock process. Process destruction supplies interpreter isolation; no in-process cleanup. This is a wrapper/frame/catch-boundary adaptation, not an exact portable-input answer.

Other assumptions were not separately recorded. Connection identifiers were redacted in the retained invocation.

## Script

```tcl-visible
set nh_options_available unavailable
set nh_return_options {}
set nh_unit "none"
set nh_invocation once
set nh_setup [binary format H* 6e616d657370616365206576616c203a3a6e68207b7d0a70726f63207b3a3a6e683a3a74776f20776f7264737d207b66697865642076616c75657d207b6c69737420246669786564202476616c75657d0a73657420746172676574207b3a3a6e683a3a74776f20776f7264737d0a]
set nh_operation [binary format H* 6576616c202224746172676574207b6120627d207b6320647d220a0a]
set nh_setup_code [catch {eval $nh_setup} nh_setup_result nh_return_options]
set nh_options_available captured

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
        puts "NHO|nh20261012stock3|string-prefix-near-miss|stock-8.5|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
if {$nh_setup_code == 0} {
set nh_code [catch {eval $nh_operation} nh_result nh_return_options]

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
        puts "NHO|nh20261012stock3|string-prefix-near-miss|stock-8.5|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
NHO|nh20261012stock3|string-prefix-near-miss|stock-8.5|setup|0|none|once|result|utf-8|0|1|3a3a6e683a3a74776f20776f726473
NHO|nh20261012stock3|string-prefix-near-miss|stock-8.5|setup|0|none|once|returnoptions|utf-8|0|1|2d636f64652030202d6c6576656c2030
NHO|nh20261012stock3|string-prefix-near-miss|stock-8.5|lookup|1|none|once|result|utf-8|0|1|696e76616c696420636f6d6d616e64206e616d6520223a3a6e683a3a74776f22
NHO|nh20261012stock3|string-prefix-near-miss|stock-8.5|lookup|1|none|once|metadata|utf-8|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e73206361707475726564
NHO|nh20261012stock3|string-prefix-near-miss|stock-8.5|lookup|1|none|once|returnoptions|utf-8|0|2|2d636f64652031202d6c6576656c2030202d6572726f72636f6465204e4f4e45202d6572726f72696e666f207b696e76616c696420636f6d6d616e64206e616d6520223a3a6e683a3a74776f220a202020207768696c6520657865637574696e670a223a3a6e683a3a74776f20776f726473207b6120627d207b6320647d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c202224746172676574207b6120627d207b63
NHO|nh20261012stock3|string-prefix-near-miss|stock-8.5|lookup|1|none|once|returnoptions|utf-8|1|2|20647d22220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e227d202d6572726f726c696e652031
NHO|nh20261012stock3|string-prefix-near-miss|stock-8.5|lookup|1|none|once|errorcode|utf-8|0|1|4e4f4e45
NHO|nh20261012stock3|string-prefix-near-miss|stock-8.5|lookup|1|none|once|errorinfo|utf-8|0|2|696e76616c696420636f6d6d616e64206e616d6520223a3a6e683a3a74776f220a202020207768696c6520657865637574696e670a223a3a6e683a3a74776f20776f726473207b6120627d207b6320647d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c202224746172676574207b6120627d207b6320647d22220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b6564206672
NHO|nh20261012stock3|string-prefix-near-miss|stock-8.5|lookup|1|none|once|errorinfo|utf-8|1|2|6f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
```

- Standard error: empty.

The exit status and streams are from the SSH invocation; standard error can include SSH transport messages as well as remote errors. The entries below decode complete NHO result payloads from standard output.

- setup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
::nh::two words⟦EOF⟧
```

- setup, completion 0, returnoptions (utf-8; unit none, invocation once):

```text-visible
-code 0 -level 0⟦EOF⟧
```

- lookup, completion 1, result (utf-8; unit none, invocation once):

```text-visible
invalid command name "::nh::two"⟦EOF⟧
```

- lookup, completion 1, metadata (utf-8; unit none, invocation once):

```text-visible
errorcode-present 1 errorinfo-present 1 return-options captured⟦EOF⟧
```

- lookup, completion 1, returnoptions (utf-8; unit none, invocation once):

```text-visible
-code 1 -level 0 -errorcode NONE -errorinfo {invalid command name "::nh::two"
    while executing
"::nh::two words {a b} {c d}"
    ("eval" body line 1)
    invoked from within
"eval "$target {a b} {c d}""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"} -errorline 1⟦EOF⟧
```

- lookup, completion 1, errorcode (utf-8; unit none, invocation once):

```text-visible
NONE⟦EOF⟧
```

- lookup, completion 1, errorinfo (utf-8; unit none, invocation once):

```text-visible
invalid command name "::nh::two"
    while executing
"::nh::two words {a b} {c d}"
    ("eval" body line 1)
    invoked from within
"eval "$target {a b} {c d}""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```
