set measuredSource [lindex $argv 0]
set caseName [lindex $argv 1]
set observe [lindex $argv 2]
set fh [open $measuredSource r]
set source [read $fh]
close $fh
set entered 0
if {$observe} {
    rename regexp __original_regexp
    proc regexp {args} {
        set ::entered 1
        return [uplevel 1 [linsert $args 0 __original_regexp]]
    }
}
set code [catch {eval $source} result]
set measuredErrorCode {}
if {$code && [info exists ::errorCode]} {set measuredErrorCode $::errorCode}
puts [list case $caseName witness $observe code $code result $result entered $entered payloadExists [info exists payload] errorCode $measuredErrorCode]
