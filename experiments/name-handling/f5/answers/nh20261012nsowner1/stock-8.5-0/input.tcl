set nh_options_available unavailable
set nh_return_options {}
set nh_unit "none"
set nh_invocation once
set nh_setup [binary format H* 736574206e685f6f776e65725f6e206e616d6573706163650a736574206e685f6f776e65725f73636f7065205b246e685f6f776e65725f6e2063757272656e745d0a736574206e685f6f776e65725f63616e64696461746573205b6c697374203a3a5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e736f776e6572315f6e616d6573706163653a3a5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e736f776e6572315f746172676574205b737472696e67207472696d726967687420246e685f6f776e65725f73636f7065203a5d3a3a5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e736f776e6572315f746172676574203a3a5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e736f776e6572315f7461726765745d0a6966207b5b246e685f6f776e65725f6e20657869737473203a3a5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e736f776e6572315f6e616d6573706163655d207c7c205b6c6c656e677468205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e736f776e6572315f6e616d6573706163655d5d7d207b6572726f72207b6e616d657370616365206f776e65727368697020636f6c6c6973696f6e7d7d0a666f7265616368206e685f6f776e65725f6865616420246e685f6f776e65725f63616e64696461746573207b6966207b5b6c6c656e677468205b696e666f20636f6d6d616e647320246e685f6f776e65725f686561645d5d207c7c205b6c6c656e677468205b696e666f2070726f637320246e685f6f776e65725f686561645d5d7d207b6572726f72205b6c69737420636f6d6d616e64206f776e65727368697020636f6c6c6973696f6e20246e685f6f776e65725f686561645d7d7d0a6c69737420616273656e7420657865637574696f6e2d6e616d65737061636520246e685f6f776e65725f73636f70652063616e6469646174657320246e685f6f776e65725f63616e646964617465730a]
set nh_operation [binary format H* 736574206e685f6f776e65725f696e73696465205b6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e736f776e6572315f6e616d657370616365207b0a70726f63205f5f74636c5f6c73705f70726f62655f6e6832303236313031326e736f776e6572315f746172676574207b7d207b72657475726e205441524745547d0a6e616d6573706163652063757272656e740a7d5d0a736574206e685f6f776e65725f726f7773207b7d0a666f7265616368206e685f6f776e65725f6865616420246e685f6f776e65725f63616e64696461746573207b0a736574206e685f6f776e65725f636f6465205b6361746368207b6576616c205b6c69737420246e685f6f776e65725f686561645d7d206e685f6f776e65725f76616c75655d0a736574206e685f6f776e65725f626f64795f636f6465205b6361746368207b696e666f20626f647920246e685f6f776e65725f686561647d206e685f6f776e65725f626f64795d0a6c617070656e64206e685f6f776e65725f726f7773205b6c697374206865616420246e685f6f776e65725f6865616420636f6d6d616e6473205b696e666f20636f6d6d616e647320246e685f6f776e65725f686561645d2070726f6373205b696e666f2070726f637320246e685f6f776e65725f686561645d20696e766f636174696f6e20246e685f6f776e65725f636f646520246e685f6f776e65725f76616c756520626f647920246e685f6f776e65725f626f64795f636f646520246e685f6f776e65725f626f64795d0a7d0a6c69737420657865637574696f6e2d6e616d65737061636520246e685f6f776e65725f73636f706520696e736964652d6e616d65737061636520246e685f6f776e65725f696e7369646520646566696e6974696f6e7320246e685f6f776e65725f726f77730a]
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
        puts "NHO|nh20261012nsowner1|namespace-definition-owner|stock-8.5|setup|$nh_setup_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
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
        puts "NHO|nh20261012nsowner1|namespace-definition-owner|stock-8.5|lookup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
}
set nh_cleanup [binary format H* 6966207b246e685f73657475705f636f646520213d20307d207b72657475726e207b6f776e6572736869702d6e6f742d61637175697265647d7d0a666f7265616368206e685f6f776e65725f6865616420246e685f6f776e65725f63616e64696461746573207b6966207b5b6c6c656e677468205b696e666f20636f6d6d616e647320246e685f6f776e65725f686561645d5d7d207b72656e616d6520246e685f6f776e65725f68656164207b7d7d7d0a6966207b5b6e616d65737061636520657869737473203a3a5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e736f776e6572315f6e616d6573706163655d7d207b6e616d6573706163652064656c657465203a3a5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e736f776e6572315f6e616d6573706163657d0a736574206e685f6f776e65725f72656d61696e696e67207b7d0a666f7265616368206e685f6f776e65725f6865616420246e685f6f776e65725f63616e64696461746573207b0a736574206e685f6f776e65725f61667465725f636f6465205b6361746368207b6576616c205b6c69737420246e685f6f776e65725f686561645d7d206e685f6f776e65725f61667465725f76616c75655d0a6c617070656e64206e685f6f776e65725f72656d61696e696e67205b6c697374206865616420246e685f6f776e65725f6865616420636f6d6d616e6473205b696e666f20636f6d6d616e647320246e685f6f776e65725f686561645d2070726f6373205b696e666f2070726f637320246e685f6f776e65725f686561645d20696e766f636174696f6e20246e685f6f776e65725f61667465725f636f646520246e685f6f776e65725f61667465725f76616c75655d0a7d0a6c697374206e616d657370616365205b6e616d65737061636520657869737473203a3a5f5f74636c5f6c73705f70726f62655f6e6832303236313031326e736f776e6572315f6e616d6573706163655d2063616e6469646174657320246e685f6f776e65725f72656d61696e696e670a]
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
        puts "NHO|nh20261012nsowner1|namespace-definition-owner|stock-8.5|runtime-cleanup|$nh_code|$nh_unit|$nh_invocation|$nh_field|$nh_encoding|$nh_i|$nh_chunks|$nh_part"
    }
}
