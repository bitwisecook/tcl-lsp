
set setup [binary format H* 6e616d657370616365206576616c203a3a6e68207b70726f6320656d6974207b66697865642076616c75657d207b6c69737420246669786564202476616c75657d7d0a696e7465727020616c696173207b7d203a3a6e683a3a6c6f63616c207b7d203a3a6e683a3a656d6974207b6120627d0a72656e616d65203a3a6e683a3a6c6f63616c203a3a6e683a3a72656e616d65640a736574206465736372697074696f6e205b696e7465727020616c696173207b7d203a3a6e683a3a6c6f63616c5d]
set operation [binary format H* 6c69737420246465736372697074696f6e205b3a3a6e683a3a72656e616d6564207b6320647d5d]
set catch_command catch
set modern [expr {[package vcompare [info patchlevel] 8.5] >= 0}]
set setup_options {}
if {$modern} {
    set setup_code [$catch_command $setup setup_result setup_options]
} else {
    set setup_code [$catch_command $setup setup_result]
}
set setup_error_code {}
set setup_error_info {}
if {$setup_code && [info exists ::errorCode]} {set setup_error_code $::errorCode; set setup_error_info $::errorInfo}
if {$setup_code} {
    set lookup_code not-run
    set lookup_result {}
    set lookup_options {}
} elseif {$modern} {
    set lookup_code [$catch_command $operation lookup_result lookup_options]
} else {
    set lookup_code [$catch_command $operation lookup_result]
    set lookup_options {}
}
set lookup_error_code {}
set lookup_error_info {}
if {$lookup_code eq "1" && [info exists ::errorCode]} {set lookup_error_code $::errorCode; set lookup_error_info $::errorInfo}
puts [list OBSERVATION setup_code $setup_code lookup_code $lookup_code]
foreach field {setup_result setup_options setup_error_code setup_error_info lookup_result lookup_options lookup_error_code lookup_error_info} {
    binary scan [set $field] H* bytes
    puts [list FIELD $field $bytes]
}
puts [list TCLTEST [expr {[info exists ::tcltest::numTests] ? [array get ::tcltest::numTests] : {}}]]
