# Original ASCII LF source. No binary-produced command names or private headers.
proc emit {label value} {
    binary scan $value H* encoded
    puts "$label\t$encoded"
}
proc source_boom {a} {
    set b 1
    error "bad $a"
}
proc dynamic_boom {a} {
    set command error
    $command "bad $a"
}
foreach name {source_boom dynamic_boom} {
    set code [catch {$name Q} message]
    emit "$name.code" $code
    emit "$name.result" $message
    if {[info exists ::errorInfo]} {emit "$name.errorInfo" $::errorInfo}
    if {[info exists ::errorCode]} {emit "$name.errorCode" $::errorCode}
    set stackCode [catch {info errorstack} stack]
    emit "$name.stackCode" $stackCode
    emit "$name.stack" $stack
}
set code [catch {definitely_missing_command} message]
emit missing.code $code
emit missing.result $message
if {[info exists ::errorCode]} {emit missing.errorCode $::errorCode}
