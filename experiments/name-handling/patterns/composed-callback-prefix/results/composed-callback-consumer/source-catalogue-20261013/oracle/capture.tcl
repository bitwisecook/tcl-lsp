set code [catch {source [lindex $argv 0]} value]
binary scan $value H* valueHex
puts [list completion $code resultHex $valueHex]
if {$code != 0} {
    puts [list errorCode $::errorCode]
    puts [list errorInfo $::errorInfo]
}
