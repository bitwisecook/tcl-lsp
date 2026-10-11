# Exact ASCII source, Tcl 8.4-compatible except the public workers being measured.
# R2286_IO_PROBE_FILE selects an owned per-provider disposable file.
if {![info exists ::env(R2286_IO_PROBE_FILE)]} {error "owned probe file is required"}
proc hx {value} {binary scan $value H* encoded; return $encoded}
proc report {label script before channel} {
    set ::errorCode {R2286 SEEDED}
    set code [catch {uplevel 1 $script} result]
    set errorCode $::errorCode
    set after $before
    if {$channel ne ""} {set after [tell $channel]}
    puts [list CASE $label CODE $code RESULT_HEX [hx $result] ERRORCODE_HEX [hx $errorCode] BEFORE $before AFTER $after]
}
set path $::env(R2286_IO_PROBE_FILE)
set writer [open $path w]
puts -nonewline $writer "abcdef\n"
close $writer
foreach count {bad 1.5 -1 0 2 18446744073709551616} {
    set channel [open $path r]
    seek $channel 3
    report [list read $count] [list read $channel $count] 3 $channel
    close $channel
}
foreach offset {bad 1.5 -1 0 2 18446744073709551616} {
    foreach origin {omitted start current} {
        set channel [open $path r]
        seek $channel 3
        if {$origin eq "omitted"} {set script [list seek $channel $offset]} else {set script [list seek $channel $offset $origin]}
        report [list seek $offset $origin] $script 3 $channel
        close $channel
    }
}
set channel [open $path r]
seek $channel 3
report {seek bad invalid-origin} [list seek $channel bad invalid-origin] 3 $channel
close $channel
foreach length {bad 1.5 -1 0 2 18446744073709551616} {
    report [list binary-base64-maxlen $length] [list binary encode base64 -maxlen $length abcdef] NA ""
}
foreach flag {bad 1.5 false true 0 1} {
    set child r2286_io_debug
    if {[catch {interp create $child} createResult]} {
        puts [list CASE debug-create CODE 1 RESULT_HEX [hx $createResult]]
        break
    }
    report [list debug-frame $flag] [list interp debug $child -frame $flag] 0 ""
    report [list debug-frame-state $flag] [list interp debug $child -frame] NA ""
    interp delete $child
}
file delete $path
