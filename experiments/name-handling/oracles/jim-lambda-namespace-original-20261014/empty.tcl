puts [list VERSION [info patchlevel]]
namespace eval ::Caller {
    set lambda [list {} {namespace current} ""]
    set code [catch {apply $lambda} value]
    binary scan $value H* bytes
    puts [list RESULT $code $bytes]
}
