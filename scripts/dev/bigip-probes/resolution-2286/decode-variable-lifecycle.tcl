#!/usr/bin/env tclsh
# Decode one raw variable-lifecycle control response as tab-separated rows.

if {$argc != 2} {
    puts stderr "usage: decode-variable-lifecycle.tcl INPUT MATRIX_TSV"
    exit 2
}

set input [lindex $argv 0]
set output [lindex $argv 1]
set channel [open $input rb]
fconfigure $channel -translation binary
set response [string trimright [read $channel] "\r\n"]
close $channel
if {[llength $response] != 4 || [lindex $response 0] ne "vl1"} {
    error "invalid vl1 response envelope"
}
set group [lindex $response 1]
set unit [lindex $response 2]
set rows [lindex $response 3]
set channel [open $output wb]
fconfigure $channel -translation lf -encoding ascii
puts $channel "group\tunit\tcase\toperation_rc\tresult_scan_rc\tresult_hex\tresult_scan_result"
foreach row $rows {
    if {[llength $row] != 5} {
        error "invalid lifecycle row: $row"
    }
    puts $channel [join [linsert $row 0 $group $unit] "\t"]
}
close $channel
