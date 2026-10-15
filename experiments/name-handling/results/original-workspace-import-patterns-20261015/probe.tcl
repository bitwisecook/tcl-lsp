set setup [catch {
    namespace eval ::Source {
        proc {p*} {} {return STAR}
        proc {p?} {} {return QUESTION}
        proc {p[} {} {return BRACKET}
        proc {p\x} {} {return SLASH}
        namespace export *
    }
    namespace eval ::Target {}
} message]
puts [list SETUP $setup $message]
if {$setup == 0} {
    foreach {name pattern} {{p*} {p\*} {p?} {p\?} {p[} {p\[} {p\x} {p\\x}} {
        set code [catch {namespace eval ::Target [list namespace import ::Source::$pattern]} message]
        puts [list IMPORT $name $pattern $code $message]
        if {$code == 0} {
            set call [list ::Target::$name]
            set code [catch {uplevel #0 $call} result]
            puts [list CALL $name $code $result]
        }
    }
}
