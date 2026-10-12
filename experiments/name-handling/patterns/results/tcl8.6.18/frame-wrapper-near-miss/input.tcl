
set setup [binary format H* 70726f6320666f7277617264207b7072656669787d207b6576616c20247072656669787d0a70726f63206368616e6765207b6e616d657d207b7570766172203120246e616d652063656c6c3b207365742063656c6c206368616e6765647d0a70726f63206f75746572207b7d207b0a20202020736574207b6120627d206f726967696e616c0a20202020666f7277617264205b6c697374206368616e6765207b6120627d5d0a20202020736574207b6120627d0a7d]
set operation [binary format H* 6f75746572]
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
