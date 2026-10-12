
set setup [binary format H* 7061636b61676520726571756972652074636c746573740a6e616d657370616365206576616c203a3a6e68207b0a202020206e616d65737061636520696d706f7274203a3a74636c746573743a3a2a0a20202020736574206f62736572766564207b7d0a2020202074657374206e616d652d636f727075732d31207b7d202d7365747570207b0a2020202020202020736574207061796c6f6164207b6120627d0a202020207d202d626f6479207b0a20202020202020206c617070656e64206f62736572766564205b736574207061796c6f61645d0a202020207d202d636c65616e7570207b0a20202020202020206c617070656e64206f62736572766564205b736574207061796c6f61645d0a2020202020202020756e736574207061796c6f61640a202020207d202d726573756c74207b7b6120627d7d0a7d]
set operation [binary format H* 6c69737420243a3a6e683a3a6f62736572766564205b696e666f20657869737473203a3a6e683a3a7061796c6f61645d]
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
