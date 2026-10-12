foreach {label script} {
    VERSION {info patchlevel}
    DECLARATION_ARGS {info args {namespace info}}
    DECLARATION_BODY {info body {namespace info}}
    GUARD_COMMANDS {info commands ::tcl::mathop::+}
} {
    set code [catch $script result]
    binary scan $result H* hex
    puts "$label $code $hex"
}
puts CLOSED_METADATA
