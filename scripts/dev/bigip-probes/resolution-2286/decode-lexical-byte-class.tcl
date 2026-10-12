#!/usr/bin/env tclsh
# Decode one raw LBC1 export envelope without interpreting result byte fields.

if {$argc != 3} {
    puts stderr "usage: decode-lexical-byte-class.tcl INPUT MATRIX_TSV SUMMARY"
    exit 2
}

proc hex {value} {
    binary scan $value H* result
    return $result
}

set input [lindex $argv 0]
set matrix_path [lindex $argv 1]
set summary_path [lindex $argv 2]
set channel [open $input rb]
fconfigure $channel -translation binary
set envelope [read $channel]
close $channel
set envelope [string trimright $envelope "\r\n"]

if {[llength $envelope] != 4 || [lindex $envelope 0] ne "LBC1"} {
    error "invalid LBC1 envelope"
}
set export_rc [lindex $envelope 1]
set export_result [lindex $envelope 2]
if {$export_rc != 0} {
    error "transcript export failed: $export_result"
}
set transcript [binary format H* [lindex $envelope 3]]
if {[llength $transcript] != 5 || [lindex $transcript 0] ne "LBC1"} {
    error "invalid decoded LBC1 transcript"
}
set context [lindex $transcript 1]
set identity [lindex $transcript 2]
set declared_count [lindex $transcript 3]
set rows [lindex $transcript 4]
if {$declared_count != 1044 || [llength $rows] != 1044} {
    error "expected 1044 rows, declared $declared_count and decoded [llength $rows]"
}

set matrix [open $matrix_path wb]
fconfigure $matrix -translation lf -encoding ascii
puts $matrix "context\tidentity_hex\tcase\tsource_kind\tform\tfactory_hex\tconstructor_hex\tpayload_scan_rc\tpayload_hex\tsource_build_rc\tsource_build_result_hex\tsource_scan_rc\tsource_hex\teval_code\tresult_scan_rc\tresult_hex\treads_count\treads_hex\tsetup_hex\ttrace_add_hex\tutf8_diagnostic_hex\ttrace_remove_hex\tcleanup_hex\traw_row_hex"
array set seen {}
foreach item $rows {
    array unset row
    array set row $item
    set key "$row(case)/$row(source_kind)/$row(form)"
    if {[info exists seen($key)]} {
        error "duplicate key $key"
    }
    set seen($key) 1
    set columns [list \
        $context [hex $identity] $row(case) $row(source_kind) $row(form) \
        [hex $row(factory)] $row(constructor_hex) [lindex $row(payload_scan) 0] \
        [lindex $row(payload_scan) 1] [lindex $row(source_build) 0] \
        [hex [lindex $row(source_build) 1]] [lindex $row(source_scan) 0] \
        [lindex $row(source_scan) 1] $row(eval_code) [lindex $row(result_scan) 0] \
        [lindex $row(result_scan) 1] [llength $row(reads)] [hex $row(reads)] [hex $row(setup)] \
        [hex $row(trace_add)] [hex $row(utf8_diagnostic)] \
        [hex $row(trace_remove)] [hex $row(cleanup)] [hex $item]]
    puts $matrix [join $columns "\t"]
}
close $matrix
if {[array size seen] != 1044} {
    error "expected 1044 unique keys, got [array size seen]"
}

set summary [open $summary_path wb]
fconfigure $summary -translation lf -encoding ascii
puts $summary "input=$input"
puts $summary "envelope_sha256_external=RECORD_WITH_SHA256SUM"
puts $summary "export_rc=$export_rc"
puts $summary "export_result_hex=[hex $export_result]"
puts $summary "context=$context"
puts $summary "identity_hex=[hex $identity]"
puts $summary "declared_rows=$declared_count"
puts $summary "decoded_rows=[llength $rows]"
puts $summary "unique_keys=[array size seen]"
puts $summary "decoded_transcript_bytes=[string length $transcript]"
close $summary
