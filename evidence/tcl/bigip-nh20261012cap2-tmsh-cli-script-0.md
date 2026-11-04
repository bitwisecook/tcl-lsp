# BIG-IP tmsh Tcl: nh20261012cap2, tmsh-cli-script-0

## Question

This script contains the following cases, which investigate Tcl name handling relevant to the LSP, compiler and VM. Each observation is limited to its own case and this execution context.

- **call:** Which namespace subcommands, interp forms and dynamic definition operations exist in each context? Are unsupported literal commands rejected at load even in `catch`? Compare computed head/eval.

- **catch.options:** Which namespace subcommands, interp forms and dynamic definition operations exist in each context? Are unsupported literal commands rejected at load even in `catch`? Compare computed head/eval.

- **encoding:** Which namespace subcommands, interp forms and dynamic definition operations exist in each context? Are unsupported literal commands rejected at load even in `catch`? Compare computed head/eval.

- **interp.alias:** Which namespace subcommands, interp forms and dynamic definition operations exist in each context? Are unsupported literal commands rejected at load even in `catch`? Compare computed head/eval.

- **interp.create:** Which namespace subcommands, interp forms and dynamic definition operations exist in each context? Are unsupported literal commands rejected at load even in `catch`? Compare computed head/eval.

- **namespace.code:** Which namespace subcommands, interp forms and dynamic definition operations exist in each context? Are unsupported literal commands rejected at load even in `catch`? Compare computed head/eval.

- **namespace.ensemble:** Which namespace subcommands, interp forms and dynamic definition operations exist in each context? Are unsupported literal commands rejected at load even in `catch`? Compare computed head/eval.

- **namespace.eval:** Which namespace subcommands, interp forms and dynamic definition operations exist in each context? Are unsupported literal commands rejected at load even in `catch`? Compare computed head/eval.

- **namespace.export:** Which namespace subcommands, interp forms and dynamic definition operations exist in each context? Are unsupported literal commands rejected at load even in `catch`? Compare computed head/eval.

- **namespace.import:** Which namespace subcommands, interp forms and dynamic definition operations exist in each context? Are unsupported literal commands rejected at load even in `catch`? Compare computed head/eval.

- **namespace.path:** Which namespace subcommands, interp forms and dynamic definition operations exist in each context? Are unsupported literal commands rejected at load even in `catch`? Compare computed head/eval.

- **namespace.upvar:** Which namespace subcommands, interp forms and dynamic definition operations exist in each context? Are unsupported literal commands rejected at load even in `catch`? Compare computed head/eval.

- **proc.dynamic:** Which namespace subcommands, interp forms and dynamic definition operations exist in each context? Are unsupported literal commands rejected at load even in `catch`? Compare computed head/eval.

- **tcloo:** Which namespace subcommands, interp forms and dynamic definition operations exist in each context? Are unsupported literal commands rejected at load even in `catch`? Compare computed head/eval.

- **tcltest:** Which namespace subcommands, interp forms and dynamic definition operations exist in each context? Are unsupported literal commands rejected at load even in `catch`? Compare computed head/eval.

- **trace.execution:** Which namespace subcommands, interp forms and dynamic definition operations exist in each context? Are unsupported literal commands rejected at load even in `catch`? Compare computed head/eval.

- **version:** Which Tcl version and execution namespace are available in this tmsh context, so that observations from this context can be scoped correctly?

## Assumptions

- **call:** Not recorded Intended operation: reached.
- **catch.options:** Not recorded Intended operation: reached.
- **encoding:** Not recorded Intended operation: reached.
- **interp.alias:** Not recorded Intended operation: reached.
- **interp.create:** Not recorded Intended operation: reached.
- **namespace.code:** Not recorded Intended operation: reached.
- **namespace.ensemble:** Not recorded Intended operation: reached.
- **namespace.eval:** Not recorded Intended operation: reached.
- **namespace.export:** Not recorded Intended operation: reached.
- **namespace.import:** Not recorded Intended operation: reached.
- **namespace.path:** Not recorded Intended operation: reached.
- **namespace.upvar:** Not recorded Intended operation: reached.
- **proc.dynamic:** Not recorded Intended operation: reached.
- **tcloo:** Not recorded Intended operation: reached.
- **tcltest:** Not recorded Intended operation: reached.
- **trace.execution:** Not recorded Intended operation: reached.
- **version:** Not recorded Intended operation: reached.

Other assumptions were not separately recorded.

## Script

```tcl-visible
set nh_unit "none"
set nh_invocation once
set nh_setup {}
set nh_operation [binary format H* 6c697374205b696e666f2070617463686c6576656c5d205b696e666f2074636c76657273696f6e5d205b6e616d6573706163652063757272656e745d]
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
        puts "NHO|nh20261012cap2|version|tmsh-cli-script|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        puts "NHO|nh20261012cap2|version|tmsh-cli-script|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 6966207b5b6e616d65737061636520657869737473203a3a5f5f6e685f6e683230323631303132636170325f76657273696f6e5d7d207b6e616d6573706163652064656c657465203a3a5f5f6e685f6e683230323631303132636170325f76657273696f6e7d3b20666f7265616368206e685f636d64205b696e666f20636f6d6d616e6473203a3a5f5f6e685f6e683230323631303132636170325f76657273696f6e5f2a5d207b72656e616d6520246e685f636d64207b7d7d3b206c697374205b6e616d65737061636520657869737473203a3a5f5f6e685f6e683230323631303132636170325f76657273696f6e5d205b696e666f20636f6d6d616e6473203a3a5f5f6e685f6e683230323631303132636170325f76657273696f6e5f2a5d]
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
        puts "NHO|nh20261012cap2|version|tmsh-cli-script|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
set nh_setup {}
set nh_operation [binary format H* 62696e617279207363616e205b656e636f64696e6720636f6e76657274746f207574662d38205b666f726d6174202563203934355d5d20482a20783b207365742078]
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
        puts "NHO|nh20261012cap2|encoding|tmsh-cli-script|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        puts "NHO|nh20261012cap2|encoding|tmsh-cli-script|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 6966207b5b6e616d65737061636520657869737473203a3a5f5f6e685f6e683230323631303132636170325f656e636f64696e675d7d207b6e616d6573706163652064656c657465203a3a5f5f6e685f6e683230323631303132636170325f656e636f64696e677d3b20666f7265616368206e685f636d64205b696e666f20636f6d6d616e6473203a3a5f5f6e685f6e683230323631303132636170325f656e636f64696e675f2a5d207b72656e616d6520246e685f636d64207b7d7d3b206c697374205b6e616d65737061636520657869737473203a3a5f5f6e685f6e683230323631303132636170325f656e636f64696e675d205b696e666f20636f6d6d616e6473203a3a5f5f6e685f6e683230323631303132636170325f656e636f64696e675f2a5d]
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
        puts "NHO|nh20261012cap2|encoding|tmsh-cli-script|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
set nh_setup {}
set nh_operation [binary format H* 6e616d657370616365206576616c203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f6576616c207b6e616d6573706163652063757272656e747d]
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
        puts "NHO|nh20261012cap2|namespace.eval|tmsh-cli-script|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        puts "NHO|nh20261012cap2|namespace.eval|tmsh-cli-script|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 6966207b5b6e616d65737061636520657869737473203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f6576616c5d7d207b6e616d6573706163652064656c657465203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f6576616c7d3b20666f7265616368206e685f636d64205b696e666f20636f6d6d616e6473203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f6576616c5f2a5d207b72656e616d6520246e685f636d64207b7d7d3b206c697374205b6e616d65737061636520657869737473203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f6576616c5d205b696e666f20636f6d6d616e6473203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f6576616c5f2a5d]
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
        puts "NHO|nh20261012cap2|namespace.eval|tmsh-cli-script|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
set nh_setup {}
set nh_operation [binary format H* 6e616d657370616365206576616c203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f636f6465207b6e616d65737061636520636f6465207b6c6973742063617074757265647d7d]
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
        puts "NHO|nh20261012cap2|namespace.code|tmsh-cli-script|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        puts "NHO|nh20261012cap2|namespace.code|tmsh-cli-script|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 6966207b5b6e616d65737061636520657869737473203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f636f64655d7d207b6e616d6573706163652064656c657465203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f636f64657d3b20666f7265616368206e685f636d64205b696e666f20636f6d6d616e6473203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f636f64655f2a5d207b72656e616d6520246e685f636d64207b7d7d3b206c697374205b6e616d65737061636520657869737473203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f636f64655d205b696e666f20636f6d6d616e6473203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f636f64655f2a5d]
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
        puts "NHO|nh20261012cap2|namespace.code|tmsh-cli-script|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
set nh_setup {}
set nh_operation [binary format H* 6e616d6573706163652070617468]
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
        puts "NHO|nh20261012cap2|namespace.path|tmsh-cli-script|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        puts "NHO|nh20261012cap2|namespace.path|tmsh-cli-script|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 6966207b5b6e616d65737061636520657869737473203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f706174685d7d207b6e616d6573706163652064656c657465203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f706174687d3b20666f7265616368206e685f636d64205b696e666f20636f6d6d616e6473203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f706174685f2a5d207b72656e616d6520246e685f636d64207b7d7d3b206c697374205b6e616d65737061636520657869737473203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f706174685d205b696e666f20636f6d6d616e6473203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f706174685f2a5d]
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
        puts "NHO|nh20261012cap2|namespace.path|tmsh-cli-script|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
set nh_setup [binary format H* 6e616d657370616365206576616c203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f6578706f7274207b7d]
set nh_operation [binary format H* 6e616d657370616365206576616c203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f6578706f7274207b6e616d657370616365206578706f7274206e685f6e6f6e657d]
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
        puts "NHO|nh20261012cap2|namespace.export|tmsh-cli-script|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        puts "NHO|nh20261012cap2|namespace.export|tmsh-cli-script|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 6966207b5b6e616d65737061636520657869737473203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f6578706f72745d7d207b6e616d6573706163652064656c657465203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f6578706f72747d3b20666f7265616368206e685f636d64205b696e666f20636f6d6d616e6473203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f6578706f72745f2a5d207b72656e616d6520246e685f636d64207b7d7d3b206c697374205b6e616d65737061636520657869737473203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f6578706f72745d205b696e666f20636f6d6d616e6473203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f6578706f72745f2a5d]
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
        puts "NHO|nh20261012cap2|namespace.export|tmsh-cli-script|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
set nh_setup [binary format H* 6e616d657370616365206576616c203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f696d706f7274207b7d]
set nh_operation [binary format H* 6e616d657370616365206576616c203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f696d706f7274207b6e616d65737061636520696d706f7274203a3a5f5f616273656e745f6e685f2a7d]
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
        puts "NHO|nh20261012cap2|namespace.import|tmsh-cli-script|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        puts "NHO|nh20261012cap2|namespace.import|tmsh-cli-script|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 6966207b5b6e616d65737061636520657869737473203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f696d706f72745d7d207b6e616d6573706163652064656c657465203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f696d706f72747d3b20666f7265616368206e685f636d64205b696e666f20636f6d6d616e6473203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f696d706f72745f2a5d207b72656e616d6520246e685f636d64207b7d7d3b206c697374205b6e616d65737061636520657869737473203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f696d706f72745d205b696e666f20636f6d6d616e6473203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f696d706f72745f2a5d]
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
        puts "NHO|nh20261012cap2|namespace.import|tmsh-cli-script|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
set nh_setup [binary format H* 6e616d657370616365206576616c203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f656e73656d626c65207b7d]
set nh_operation [binary format H* 6e616d657370616365206576616c203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f656e73656d626c65207b6e616d65737061636520656e73656d626c6520637265617465202d636f6d6d616e6420666163616465202d6d6170207b7d7d]
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
        puts "NHO|nh20261012cap2|namespace.ensemble|tmsh-cli-script|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        puts "NHO|nh20261012cap2|namespace.ensemble|tmsh-cli-script|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 6966207b5b6e616d65737061636520657869737473203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f656e73656d626c655d7d207b6e616d6573706163652064656c657465203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f656e73656d626c657d3b20666f7265616368206e685f636d64205b696e666f20636f6d6d616e6473203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f656e73656d626c655f2a5d207b72656e616d6520246e685f636d64207b7d7d3b206c697374205b6e616d65737061636520657869737473203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f656e73656d626c655d205b696e666f20636f6d6d616e6473203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f656e73656d626c655f2a5d]
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
        puts "NHO|nh20261012cap2|namespace.ensemble|tmsh-cli-script|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
set nh_setup [binary format H* 6e616d657370616365206576616c203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f7570766172207b7661726961626c6520782056414c55457d]
set nh_operation [binary format H* 6e616d657370616365207570766172203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f75707661722078206e685f6c696e6b3b20736574206e685f6c696e6b]
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
        puts "NHO|nh20261012cap2|namespace.upvar|tmsh-cli-script|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        puts "NHO|nh20261012cap2|namespace.upvar|tmsh-cli-script|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 6966207b5b6e616d65737061636520657869737473203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f75707661725d7d207b6e616d6573706163652064656c657465203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f75707661727d3b20666f7265616368206e685f636d64205b696e666f20636f6d6d616e6473203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f75707661725f2a5d207b72656e616d6520246e685f636d64207b7d7d3b206c697374205b6e616d65737061636520657869737473203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f75707661725d205b696e666f20636f6d6d616e6473203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f75707661725f2a5d]
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
        puts "NHO|nh20261012cap2|namespace.upvar|tmsh-cli-script|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
set nh_setup {}
set nh_operation [binary format H* 736574206e685f6368696c64205b696e74657270206372656174655d3b20736574206e685f726573756c74205b696e74657270206576616c20246e685f6368696c64207b696e666f2070617463686c6576656c7d5d3b20696e746572702064656c65746520246e685f6368696c643b20736574206e685f726573756c74]
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
        puts "NHO|nh20261012cap2|interp.create|tmsh-cli-script|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        puts "NHO|nh20261012cap2|interp.create|tmsh-cli-script|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 6966207b5b6e616d65737061636520657869737473203a3a5f5f6e685f6e683230323631303132636170325f696e746572705f6372656174655d7d207b6e616d6573706163652064656c657465203a3a5f5f6e685f6e683230323631303132636170325f696e746572705f6372656174657d3b20666f7265616368206e685f636d64205b696e666f20636f6d6d616e6473203a3a5f5f6e685f6e683230323631303132636170325f696e746572705f6372656174655f2a5d207b72656e616d6520246e685f636d64207b7d7d3b206c697374205b6e616d65737061636520657869737473203a3a5f5f6e685f6e683230323631303132636170325f696e746572705f6372656174655d205b696e666f20636f6d6d616e6473203a3a5f5f6e685f6e683230323631303132636170325f696e746572705f6372656174655f2a5d]
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
        puts "NHO|nh20261012cap2|interp.create|tmsh-cli-script|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
set nh_setup [binary format H* 70726f63203a3a5f5f6e685f6e683230323631303132636170325f696e746572705f616c6961735f746172676574207b7d207b72657475726e205441524745547d]
set nh_operation [binary format H* 696e7465727020616c696173207b7d203a3a5f5f6e685f6e683230323631303132636170325f696e746572705f616c6961735f616c696173207b7d203a3a5f5f6e685f6e683230323631303132636170325f696e746572705f616c6961735f7461726765743b207365742072205b3a3a5f5f6e685f6e683230323631303132636170325f696e746572705f616c6961735f616c6961735d3b20696e7465727020616c696173207b7d203a3a5f5f6e685f6e683230323631303132636170325f696e746572705f616c6961735f616c696173207b7d3b207365742072]
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
        puts "NHO|nh20261012cap2|interp.alias|tmsh-cli-script|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        puts "NHO|nh20261012cap2|interp.alias|tmsh-cli-script|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 6966207b5b6e616d65737061636520657869737473203a3a5f5f6e685f6e683230323631303132636170325f696e746572705f616c6961735d7d207b6e616d6573706163652064656c657465203a3a5f5f6e685f6e683230323631303132636170325f696e746572705f616c6961737d3b20666f7265616368206e685f636d64205b696e666f20636f6d6d616e6473203a3a5f5f6e685f6e683230323631303132636170325f696e746572705f616c6961735f2a5d207b72656e616d6520246e685f636d64207b7d7d3b206c697374205b6e616d65737061636520657869737473203a3a5f5f6e685f6e683230323631303132636170325f696e746572705f616c6961735d205b696e666f20636f6d6d616e6473203a3a5f5f6e685f6e683230323631303132636170325f696e746572705f616c6961735f2a5d]
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
        puts "NHO|nh20261012cap2|interp.alias|tmsh-cli-script|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
set nh_setup {}
set nh_operation [binary format H* 7061636b61676520726571756972652074636c74657374]
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
        puts "NHO|nh20261012cap2|tcltest|tmsh-cli-script|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        puts "NHO|nh20261012cap2|tcltest|tmsh-cli-script|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 6966207b5b6e616d65737061636520657869737473203a3a5f5f6e685f6e683230323631303132636170325f74636c746573745d7d207b6e616d6573706163652064656c657465203a3a5f5f6e685f6e683230323631303132636170325f74636c746573747d3b20666f7265616368206e685f636d64205b696e666f20636f6d6d616e6473203a3a5f5f6e685f6e683230323631303132636170325f74636c746573745f2a5d207b72656e616d6520246e685f636d64207b7d7d3b206c697374205b6e616d65737061636520657869737473203a3a5f5f6e685f6e683230323631303132636170325f74636c746573745d205b696e666f20636f6d6d616e6473203a3a5f5f6e685f6e683230323631303132636170325f74636c746573745f2a5d]
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
        puts "NHO|nh20261012cap2|tcltest|tmsh-cli-script|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
set nh_setup {}
set nh_operation [binary format H* 6f6f3a3a636c61737320637265617465203a3a5f5f6e685f6e683230323631303132636170325f74636c6f6f5f636c617373207b7d3b203a3a5f5f6e685f6e683230323631303132636170325f74636c6f6f5f636c6173732064657374726f79]
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
        puts "NHO|nh20261012cap2|tcloo|tmsh-cli-script|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        puts "NHO|nh20261012cap2|tcloo|tmsh-cli-script|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 6966207b5b6e616d65737061636520657869737473203a3a5f5f6e685f6e683230323631303132636170325f74636c6f6f5d7d207b6e616d6573706163652064656c657465203a3a5f5f6e685f6e683230323631303132636170325f74636c6f6f7d3b20666f7265616368206e685f636d64205b696e666f20636f6d6d616e6473203a3a5f5f6e685f6e683230323631303132636170325f74636c6f6f5f2a5d207b72656e616d6520246e685f636d64207b7d7d3b206c697374205b6e616d65737061636520657869737473203a3a5f5f6e685f6e683230323631303132636170325f74636c6f6f5d205b696e666f20636f6d6d616e6473203a3a5f5f6e685f6e683230323631303132636170325f74636c6f6f5f2a5d]
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
        puts "NHO|nh20261012cap2|tcloo|tmsh-cli-script|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
set nh_setup {}
set nh_operation [binary format H* 6361746368207b6c6973742076616c75657d206e685f72206e685f6f3b206c69737420246e685f7220246e685f6f]
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
        puts "NHO|nh20261012cap2|catch.options|tmsh-cli-script|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        puts "NHO|nh20261012cap2|catch.options|tmsh-cli-script|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 6966207b5b6e616d65737061636520657869737473203a3a5f5f6e685f6e683230323631303132636170325f63617463685f6f7074696f6e735d7d207b6e616d6573706163652064656c657465203a3a5f5f6e685f6e683230323631303132636170325f63617463685f6f7074696f6e737d3b20666f7265616368206e685f636d64205b696e666f20636f6d6d616e6473203a3a5f5f6e685f6e683230323631303132636170325f63617463685f6f7074696f6e735f2a5d207b72656e616d6520246e685f636d64207b7d7d3b206c697374205b6e616d65737061636520657869737473203a3a5f5f6e685f6e683230323631303132636170325f63617463685f6f7074696f6e735d205b696e666f20636f6d6d616e6473203a3a5f5f6e685f6e683230323631303132636170325f63617463685f6f7074696f6e735f2a5d]
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
        puts "NHO|nh20261012cap2|catch.options|tmsh-cli-script|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
set nh_setup [binary format H* 70726f63203a3a5f5f6e685f6e683230323631303132636170325f74726163655f657865637574696f6e5f746172676574207b7d207b72657475726e205441524745547d3b2070726f63203a3a5f5f6e685f6e683230323631303132636170325f74726163655f657865637574696f6e5f74726163652061726773207b7d]
set nh_operation [binary format H* 74726163652061646420657865637574696f6e203a3a5f5f6e685f6e683230323631303132636170325f74726163655f657865637574696f6e5f746172676574206c65617665203a3a5f5f6e685f6e683230323631303132636170325f74726163655f657865637574696f6e5f74726163653b203a3a5f5f6e685f6e683230323631303132636170325f74726163655f657865637574696f6e5f746172676574]
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
        puts "NHO|nh20261012cap2|trace.execution|tmsh-cli-script|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        puts "NHO|nh20261012cap2|trace.execution|tmsh-cli-script|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 6966207b5b6e616d65737061636520657869737473203a3a5f5f6e685f6e683230323631303132636170325f74726163655f657865637574696f6e5d7d207b6e616d6573706163652064656c657465203a3a5f5f6e685f6e683230323631303132636170325f74726163655f657865637574696f6e7d3b20666f7265616368206e685f636d64205b696e666f20636f6d6d616e6473203a3a5f5f6e685f6e683230323631303132636170325f74726163655f657865637574696f6e5f2a5d207b72656e616d6520246e685f636d64207b7d7d3b206c697374205b6e616d65737061636520657869737473203a3a5f5f6e685f6e683230323631303132636170325f74726163655f657865637574696f6e5d205b696e666f20636f6d6d616e6473203a3a5f5f6e685f6e683230323631303132636170325f74726163655f657865637574696f6e5f2a5d]
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
        puts "NHO|nh20261012cap2|trace.execution|tmsh-cli-script|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
set nh_setup {}
set nh_operation [binary format H* 70726f63203a3a5f5f6e685f6e683230323631303132636170325f70726f635f64796e616d69635f746172676574207b7d207b72657475726e205441524745547d3b203a3a5f5f6e685f6e683230323631303132636170325f70726f635f64796e616d69635f746172676574]
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
        puts "NHO|nh20261012cap2|proc.dynamic|tmsh-cli-script|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        puts "NHO|nh20261012cap2|proc.dynamic|tmsh-cli-script|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 6966207b5b6e616d65737061636520657869737473203a3a5f5f6e685f6e683230323631303132636170325f70726f635f64796e616d69635d7d207b6e616d6573706163652064656c657465203a3a5f5f6e685f6e683230323631303132636170325f70726f635f64796e616d69637d3b20666f7265616368206e685f636d64205b696e666f20636f6d6d616e6473203a3a5f5f6e685f6e683230323631303132636170325f70726f635f64796e616d69635f2a5d207b72656e616d6520246e685f636d64207b7d7d3b206c697374205b6e616d65737061636520657869737473203a3a5f5f6e685f6e683230323631303132636170325f70726f635f64796e616d69635d205b696e666f20636f6d6d616e6473203a3a5f5f6e685f6e683230323631303132636170325f70726f635f64796e616d69635f2a5d]
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
        puts "NHO|nh20261012cap2|proc.dynamic|tmsh-cli-script|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
set nh_setup {}
set nh_operation [binary format H* 63616c6c203a3a5f5f6e685f6e683230323631303132636170325f63616c6c5f616273656e74]
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
        puts "NHO|nh20261012cap2|call|tmsh-cli-script|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        puts "NHO|nh20261012cap2|call|tmsh-cli-script|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 6966207b5b6e616d65737061636520657869737473203a3a5f5f6e685f6e683230323631303132636170325f63616c6c5d7d207b6e616d6573706163652064656c657465203a3a5f5f6e685f6e683230323631303132636170325f63616c6c7d3b20666f7265616368206e685f636d64205b696e666f20636f6d6d616e6473203a3a5f5f6e685f6e683230323631303132636170325f63616c6c5f2a5d207b72656e616d6520246e685f636d64207b7d7d3b206c697374205b6e616d65737061636520657869737473203a3a5f5f6e685f6e683230323631303132636170325f63616c6c5d205b696e666f20636f6d6d616e6473203a3a5f5f6e685f6e683230323631303132636170325f63616c6c5f2a5d]
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
        puts "NHO|nh20261012cap2|call|tmsh-cli-script|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
```

## Results

### BIG-IP tmsh CLI script

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmsh run cli script.
- Exit status: 0
- Standard output:

```text-visible
NHO|nh20261012cap2|version|tmsh-cli-script|setup|0|none|once|result|utf-8|0|1|
NHO|nh20261012cap2|version|tmsh-cli-script|lookup|0|none|once|result|utf-8|0|1|382e342e3620382e34203a3a736372697074
NHO|nh20261012cap2|version|tmsh-cli-script|runtime-cleanup|0|none|once|result|utf-8|0|1|30207b7d
NHO|nh20261012cap2|encoding|tmsh-cli-script|setup|0|none|once|result|utf-8|0|1|
NHO|nh20261012cap2|encoding|tmsh-cli-script|lookup|0|none|once|result|utf-8|0|1|63656231
NHO|nh20261012cap2|encoding|tmsh-cli-script|runtime-cleanup|0|none|once|result|utf-8|0|1|30207b7d
NHO|nh20261012cap2|namespace.eval|tmsh-cli-script|setup|0|none|once|result|utf-8|0|1|
NHO|nh20261012cap2|namespace.eval|tmsh-cli-script|lookup|0|none|once|result|utf-8|0|1|3a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f6576616c
NHO|nh20261012cap2|namespace.eval|tmsh-cli-script|runtime-cleanup|0|none|once|result|utf-8|0|1|30207b7d
NHO|nh20261012cap2|namespace.code|tmsh-cli-script|setup|0|none|once|result|utf-8|0|1|
NHO|nh20261012cap2|namespace.code|tmsh-cli-script|lookup|0|none|once|result|utf-8|0|1|3a3a6e616d65737061636520696e73636f7065203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f636f6465207b6c6973742063617074757265647d
NHO|nh20261012cap2|namespace.code|tmsh-cli-script|runtime-cleanup|0|none|once|result|utf-8|0|1|30207b7d
NHO|nh20261012cap2|namespace.path|tmsh-cli-script|setup|0|none|once|result|utf-8|0|1|
NHO|nh20261012cap2|namespace.path|tmsh-cli-script|lookup|1|none|once|result|utf-8|0|1|626164206f7074696f6e202270617468223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f72207768696368
NHO|nh20261012cap2|namespace.path|tmsh-cli-script|lookup|1|none|once|metadata|utf-8|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
NHO|nh20261012cap2|namespace.path|tmsh-cli-script|lookup|1|none|once|errorcode|utf-8|0|1|4e4f4e45
NHO|nh20261012cap2|namespace.path|tmsh-cli-script|lookup|1|none|once|errorinfo|utf-8|0|2|626164206f7074696f6e202270617468223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f722077686963680a202020207768696c6520657865637574696e670a226e616d6573706163652070617468220a2020202028226576616c222062
NHO|nh20261012cap2|namespace.path|tmsh-cli-script|lookup|1|none|once|errorinfo|utf-8|1|2|6f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
NHO|nh20261012cap2|namespace.path|tmsh-cli-script|runtime-cleanup|0|none|once|result|utf-8|0|1|30207b7d
NHO|nh20261012cap2|namespace.export|tmsh-cli-script|setup|0|none|once|result|utf-8|0|1|
NHO|nh20261012cap2|namespace.export|tmsh-cli-script|lookup|0|none|once|result|utf-8|0|1|
NHO|nh20261012cap2|namespace.export|tmsh-cli-script|runtime-cleanup|0|none|once|result|utf-8|0|1|30207b7d
NHO|nh20261012cap2|namespace.import|tmsh-cli-script|setup|0|none|once|result|utf-8|0|1|
NHO|nh20261012cap2|namespace.import|tmsh-cli-script|lookup|0|none|once|result|utf-8|0|1|
NHO|nh20261012cap2|namespace.import|tmsh-cli-script|runtime-cleanup|0|none|once|result|utf-8|0|1|30207b7d
NHO|nh20261012cap2|namespace.ensemble|tmsh-cli-script|setup|0|none|once|result|utf-8|0|1|
NHO|nh20261012cap2|namespace.ensemble|tmsh-cli-script|lookup|1|none|once|result|utf-8|0|1|626164206f7074696f6e2022656e73656d626c65223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f72207768696368
NHO|nh20261012cap2|namespace.ensemble|tmsh-cli-script|lookup|1|none|once|metadata|utf-8|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
NHO|nh20261012cap2|namespace.ensemble|tmsh-cli-script|lookup|1|none|once|errorcode|utf-8|0|1|4e4f4e45
NHO|nh20261012cap2|namespace.ensemble|tmsh-cli-script|lookup|1|none|once|errorinfo|utf-8|0|3|626164206f7074696f6e2022656e73656d626c65223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f722077686963680a202020207768696c6520657865637574696e670a226e616d65737061636520656e73656d626c6520637265617465
NHO|nh20261012cap2|namespace.ensemble|tmsh-cli-script|lookup|1|none|once|errorinfo|utf-8|1|3|202d636f6d6d616e6420666163616465202d6d6170207b7d220a2020202028696e206e616d657370616365206576616c20223a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f656e73656d626c652220736372697074206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c203a3a5f5f6e685f6e683230323631303132636170325f6e616d6573706163655f656e73656d626c65207b6e616d6573706163652065
NHO|nh20261012cap2|namespace.ensemble|tmsh-cli-script|lookup|1|none|once|errorinfo|utf-8|2|3|6e73656d626c6520637265617465202d636f6d6d616e6420666163616465202d6d6170207b7d7d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
NHO|nh20261012cap2|namespace.ensemble|tmsh-cli-script|runtime-cleanup|0|none|once|result|utf-8|0|1|30207b7d
NHO|nh20261012cap2|namespace.upvar|tmsh-cli-script|setup|0|none|once|result|utf-8|0|1|
NHO|nh20261012cap2|namespace.upvar|tmsh-cli-script|lookup|1|none|once|result|utf-8|0|1|626164206f7074696f6e20227570766172223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f72207768696368
NHO|nh20261012cap2|namespace.upvar|tmsh-cli-script|lookup|1|none|once|metadata|utf-8|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
NHO|nh20261012cap2|namespace.upvar|tmsh-cli-script|lookup|1|none|once|errorcode|utf-8|0|1|4e4f4e45
NHO|nh20261012cap2|namespace.upvar|tmsh-cli-script|lookup|1|none|once|errorinfo|utf-8|0|2|626164206f7074696f6e20227570766172223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f722077686963680a202020207768696c6520657865637574696e670a226e616d657370616365207570766172203a3a5f5f6e685f6e68323032
NHO|nh20261012cap2|namespace.upvar|tmsh-cli-script|lookup|1|none|once|errorinfo|utf-8|1|2|3631303132636170325f6e616d6573706163655f75707661722078206e685f6c696e6b220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
NHO|nh20261012cap2|namespace.upvar|tmsh-cli-script|runtime-cleanup|0|none|once|result|utf-8|0|1|30207b7d
NHO|nh20261012cap2|interp.create|tmsh-cli-script|setup|0|none|once|result|utf-8|0|1|
NHO|nh20261012cap2|interp.create|tmsh-cli-script|lookup|0|none|once|result|utf-8|0|1|382e342e36
NHO|nh20261012cap2|interp.create|tmsh-cli-script|runtime-cleanup|0|none|once|result|utf-8|0|1|30207b7d
NHO|nh20261012cap2|interp.alias|tmsh-cli-script|setup|0|none|once|result|utf-8|0|1|
NHO|nh20261012cap2|interp.alias|tmsh-cli-script|lookup|0|none|once|result|utf-8|0|1|544152474554
NHO|nh20261012cap2|interp.alias|tmsh-cli-script|runtime-cleanup|0|none|once|result|utf-8|0|1|30207b7d
NHO|nh20261012cap2|tcltest|tmsh-cli-script|setup|0|none|once|result|utf-8|0|1|
NHO|nh20261012cap2|tcltest|tmsh-cli-script|lookup|1|none|once|result|utf-8|0|1|63616e2774207265616420226465627567223a206e6f2073756368207661726961626c65
NHO|nh20261012cap2|tcltest|tmsh-cli-script|lookup|1|none|once|metadata|utf-8|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
NHO|nh20261012cap2|tcltest|tmsh-cli-script|lookup|1|none|once|errorcode|utf-8|0|1|4e4f4e45
NHO|nh20261012cap2|tcltest|tmsh-cli-script|lookup|1|none|once|errorinfo|utf-8|0|5|63616e2774207265616420226465627567223a206e6f2073756368207661726961626c650a202020207768696c6520657865637574696e670a226966207b246465627567203e3d20246c6576656c7d207b0a09707574732024737472696e670a202020207d220a202020202870726f636564757265202244656275675075747322206c696e652033290a20202020696e766f6b65642066726f6d2077697468696e0a2244656275675075747320332022656e746572696e6720436f6e73747261696e74496e697469
NHO|nh20261012cap2|tcltest|tmsh-cli-script|lookup|1|none|once|errorinfo|utf-8|1|5|616c697a65722024636f6e73747261696e74202473637269707422220a202020202870726f6365647572652022436f6e73747261696e74496e697469616c697a657222206c696e652033290a20202020696e766f6b65642066726f6d2077697468696e0a22436f6e73747261696e74496e697469616c697a65722073696e676c6554657374496e74657270207b73696e676c6550726f636573737d220a202020202870726f6365647572652022446566696e65436f6e73747261696e74496e697469616c697a6572
NHO|nh20261012cap2|tcltest|tmsh-cli-script|lookup|1|none|once|errorinfo|utf-8|2|5|7322206c696e652032290a20202020696e766f6b65642066726f6d2077697468696e0a22446566696e65436f6e73747261696e74496e697469616c697a657273220a2020202028696e206e616d657370616365206576616c20223a3a74636c746573742220736372697074206c696e652033290a20202020696e766f6b65642066726f6d2077697468696e0a226e616d657370616365206576616c2074636c74657374207b0a202020202320446566696e6520696e697469616c697a65727320666f7220616c6c20
NHO|nh20261012cap2|tcltest|tmsh-cli-script|lookup|1|none|once|errorinfo|utf-8|3|5|746865206275696c742d696e20636f6e747261696e7420646566696e6974696f6e730a20202020446566696e65436f6e73747261696e74496e697469616c697a6572730a0a2020202023205365742075702074686520636f6e73742e2e2e220a202020202866696c6520222f7573722f73686172652f636f6d7061742d74636c382e342f74636c74657374322e322f74636c746573742e74636c22206c696e652033323732290a20202020696e766f6b65642066726f6d2077697468696e0a22736f75726365202f
NHO|nh20261012cap2|tcltest|tmsh-cli-script|lookup|1|none|once|errorinfo|utf-8|4|5|7573722f73686172652f636f6d7061742d74636c382e342f74636c74657374322e322f74636c746573742e74636c220a2020202028227061636b6167652069666e65656465642220736372697074290a20202020696e766f6b65642066726f6d2077697468696e0a227061636b61676520726571756972652074636c74657374220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
NHO|nh20261012cap2|tcltest|tmsh-cli-script|runtime-cleanup|0|none|once|result|utf-8|0|1|30207b7d
NHO|nh20261012cap2|tcloo|tmsh-cli-script|setup|0|none|once|result|utf-8|0|1|
NHO|nh20261012cap2|tcloo|tmsh-cli-script|lookup|1|none|once|result|utf-8|0|1|696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c61737322
NHO|nh20261012cap2|tcloo|tmsh-cli-script|lookup|1|none|once|metadata|utf-8|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
NHO|nh20261012cap2|tcloo|tmsh-cli-script|lookup|1|none|once|errorcode|utf-8|0|1|4e4f4e45
NHO|nh20261012cap2|tcloo|tmsh-cli-script|lookup|1|none|once|errorinfo|utf-8|0|1|696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c617373220a202020207768696c6520657865637574696e670a226f6f3a3a636c61737320637265617465203a3a5f5f6e685f6e683230323631303132636170325f74636c6f6f5f636c617373207b7d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
NHO|nh20261012cap2|tcloo|tmsh-cli-script|runtime-cleanup|0|none|once|result|utf-8|0|1|30207b7d
NHO|nh20261012cap2|catch.options|tmsh-cli-script|setup|0|none|once|result|utf-8|0|1|
NHO|nh20261012cap2|catch.options|tmsh-cli-script|lookup|1|none|once|result|utf-8|0|1|77726f6e67202320617267733a2073686f756c642062652022636174636820636f6d6d616e64203f7661724e616d653f22
NHO|nh20261012cap2|catch.options|tmsh-cli-script|lookup|1|none|once|metadata|utf-8|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
NHO|nh20261012cap2|catch.options|tmsh-cli-script|lookup|1|none|once|errorcode|utf-8|0|1|4e4f4e45
NHO|nh20261012cap2|catch.options|tmsh-cli-script|lookup|1|none|once|errorinfo|utf-8|0|1|77726f6e67202320617267733a2073686f756c642062652022636174636820636f6d6d616e64203f7661724e616d653f220a202020207768696c6520636f6d70696c696e670a226361746368207b6c6973742076616c75657d206e685f72206e685f6f220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
NHO|nh20261012cap2|catch.options|tmsh-cli-script|runtime-cleanup|0|none|once|result|utf-8|0|1|30207b7d
NHO|nh20261012cap2|trace.execution|tmsh-cli-script|setup|0|none|once|result|utf-8|0|1|
NHO|nh20261012cap2|trace.execution|tmsh-cli-script|lookup|0|none|once|result|utf-8|0|1|544152474554
NHO|nh20261012cap2|trace.execution|tmsh-cli-script|runtime-cleanup|0|none|once|result|utf-8|0|1|30207b7d
NHO|nh20261012cap2|proc.dynamic|tmsh-cli-script|setup|0|none|once|result|utf-8|0|1|
NHO|nh20261012cap2|proc.dynamic|tmsh-cli-script|lookup|0|none|once|result|utf-8|0|1|544152474554
NHO|nh20261012cap2|proc.dynamic|tmsh-cli-script|runtime-cleanup|0|none|once|result|utf-8|0|1|30207b7d
NHO|nh20261012cap2|call|tmsh-cli-script|setup|0|none|once|result|utf-8|0|1|
NHO|nh20261012cap2|call|tmsh-cli-script|lookup|1|none|once|result|utf-8|0|1|696e76616c696420636f6d6d616e64206e616d65202263616c6c22
NHO|nh20261012cap2|call|tmsh-cli-script|lookup|1|none|once|metadata|utf-8|0|1|6572726f72636f64652d70726573656e742031206572726f72696e666f2d70726573656e7420312072657475726e2d6f7074696f6e7320756e617661696c61626c65
NHO|nh20261012cap2|call|tmsh-cli-script|lookup|1|none|once|errorcode|utf-8|0|1|4e4f4e45
NHO|nh20261012cap2|call|tmsh-cli-script|lookup|1|none|once|errorinfo|utf-8|0|1|696e76616c696420636f6d6d616e64206e616d65202263616c6c220a202020207768696c6520657865637574696e670a2263616c6c203a3a5f5f6e685f6e683230323631303132636170325f63616c6c5f616273656e74220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c20246e685f6f7065726174696f6e22
NHO|nh20261012cap2|call|tmsh-cli-script|runtime-cleanup|0|none|once|result|utf-8|0|1|30207b7d
```

- Standard error: empty.

The exit status and streams are from the recorded tmsh invocation. The entries below decode complete NHO payloads from its standard output.

#### call

- setup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
⟦EOF⟧
```

- lookup, completion 1, result (utf-8; unit none, invocation once):

```text-visible
invalid command name "call"⟦EOF⟧
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
invalid command name "call"
    while executing
"call ::__nh_nh20261012cap2_call_absent"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- runtime-cleanup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
0 {}⟦EOF⟧
```

#### catch.options

- setup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
⟦EOF⟧
```

- lookup, completion 1, result (utf-8; unit none, invocation once):

```text-visible
wrong # args: should be "catch command ?varName?"⟦EOF⟧
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
wrong # args: should be "catch command ?varName?"
    while compiling
"catch {list value} nh_r nh_o"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- runtime-cleanup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
0 {}⟦EOF⟧
```

#### encoding

- setup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
⟦EOF⟧
```

- lookup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
ceb1⟦EOF⟧
```

- runtime-cleanup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
0 {}⟦EOF⟧
```

#### interp.alias

- setup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
⟦EOF⟧
```

- lookup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
TARGET⟦EOF⟧
```

- runtime-cleanup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
0 {}⟦EOF⟧
```

#### interp.create

- setup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
⟦EOF⟧
```

- lookup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
8.4.6⟦EOF⟧
```

- runtime-cleanup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
0 {}⟦EOF⟧
```

#### namespace.code

- setup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
⟦EOF⟧
```

- lookup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
::namespace inscope ::__nh_nh20261012cap2_namespace_code {list captured}⟦EOF⟧
```

- runtime-cleanup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
0 {}⟦EOF⟧
```

#### namespace.ensemble

- setup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
⟦EOF⟧
```

- lookup, completion 1, result (utf-8; unit none, invocation once):

```text-visible
bad option "ensemble": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which⟦EOF⟧
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
bad option "ensemble": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which
    while executing
"namespace ensemble create -command facade -map {}"
    (in namespace eval "::__nh_nh20261012cap2_namespace_ensemble" script line 1)
    invoked from within
"namespace eval ::__nh_nh20261012cap2_namespace_ensemble {namespace ensemble create -command facade -map {}}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- runtime-cleanup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
0 {}⟦EOF⟧
```

#### namespace.eval

- setup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
⟦EOF⟧
```

- lookup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
::__nh_nh20261012cap2_namespace_eval⟦EOF⟧
```

- runtime-cleanup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
0 {}⟦EOF⟧
```

#### namespace.export

- setup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
⟦EOF⟧
```

- lookup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
⟦EOF⟧
```

- runtime-cleanup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
0 {}⟦EOF⟧
```

#### namespace.import

- setup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
⟦EOF⟧
```

- lookup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
⟦EOF⟧
```

- runtime-cleanup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
0 {}⟦EOF⟧
```

#### namespace.path

- setup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
⟦EOF⟧
```

- lookup, completion 1, result (utf-8; unit none, invocation once):

```text-visible
bad option "path": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which⟦EOF⟧
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
bad option "path": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which
    while executing
"namespace path"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- runtime-cleanup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
0 {}⟦EOF⟧
```

#### namespace.upvar

- setup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
⟦EOF⟧
```

- lookup, completion 1, result (utf-8; unit none, invocation once):

```text-visible
bad option "upvar": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which⟦EOF⟧
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
bad option "upvar": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which
    while executing
"namespace upvar ::__nh_nh20261012cap2_namespace_upvar x nh_link"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- runtime-cleanup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
0 {}⟦EOF⟧
```

#### proc.dynamic

- setup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
⟦EOF⟧
```

- lookup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
TARGET⟦EOF⟧
```

- runtime-cleanup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
0 {}⟦EOF⟧
```

#### tcloo

- setup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
⟦EOF⟧
```

- lookup, completion 1, result (utf-8; unit none, invocation once):

```text-visible
invalid command name "oo::class"⟦EOF⟧
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
invalid command name "oo::class"
    while executing
"oo::class create ::__nh_nh20261012cap2_tcloo_class {}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- runtime-cleanup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
0 {}⟦EOF⟧
```

#### tcltest

- setup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
⟦EOF⟧
```

- lookup, completion 1, result (utf-8; unit none, invocation once):

```text-visible
can't read "debug": no such variable⟦EOF⟧
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
can't read "debug": no such variable
    while executing
"if {$debug >= $level} {
⟦TAB⟧puts $string
    }"
    (procedure "DebugPuts" line 3)
    invoked from within
"DebugPuts 3 "entering ConstraintInitializer $constraint $script""
    (procedure "ConstraintInitializer" line 3)
    invoked from within
"ConstraintInitializer singleTestInterp {singleProcess}"
    (procedure "DefineConstraintInitializers" line 2)
    invoked from within
"DefineConstraintInitializers"
    (in namespace eval "::tcltest" script line 3)
    invoked from within
"namespace eval tcltest {
    # Define initializers for all the built-in contraint definitions
    DefineConstraintInitializers

    # Set up the const..."
    (file "/usr/share/compat-tcl8.4/tcltest2.2/tcltest.tcl" line 3272)
    invoked from within
"source /usr/share/compat-tcl8.4/tcltest2.2/tcltest.tcl"
    ("package ifneeded" script)
    invoked from within
"package require tcltest"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- runtime-cleanup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
0 {}⟦EOF⟧
```

#### trace.execution

- setup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
⟦EOF⟧
```

- lookup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
TARGET⟦EOF⟧
```

- runtime-cleanup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
0 {}⟦EOF⟧
```

#### version

- setup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
⟦EOF⟧
```

- lookup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
8.4.6 8.4 ::script⟦EOF⟧
```

- runtime-cleanup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
0 {}⟦EOF⟧
```
