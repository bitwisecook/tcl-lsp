# `string`'s subcommands, usage and errors as each release has them, and a
# unique prefix compiled as the subcommand it selects.
proc show {script} {
    set ::errorCode NONE
    set code [catch {uplevel 1 $script} result]
    puts [string map [list \n { | }] "$code <$result> <$::errorCode>"]
}
show {string}
show {string foo x}
show {string t abc}
show {string tr abc}
show {string bytelength}
show {string bytelength abcé\0}
show {string byte abc}
show {string first}
show {string last a b c d}
show {string is}
show {string is a b c d e f}
show {string is foo x}
show {string is integer -foo x}
show {string is wideinteger 12}
show {string is entier 12}
show {string is dict {a b}}
show {string ran abcdef 0 end-2147483649}
show {set x [string ran abcdef 0 end-2147483649]}
show {::string range abcdef 0 end-2147483649}
show {{*}{string range} abcdef 0 end-2147483649}
show {set x [{*}{string range} abcdef 0 end-2147483649]}
show {set x [string to abc]}
show {set x [string r abc]}
proc p {} {
    show {string ran abcdef 0 end-2147483649}
    show {set x [string ran abcdef 0 end-2147483649]}
    show {::string range abcdef 0 end-2147483649}
    show {{*}{string range} abcdef 0 end-2147483649}
    show {set x [string to abc]}
    show {set x [string wordend abc 1]}
    show {set x [string words abc 1]}
    show {set x [string bytel abc]}
}
proc q {} {
    list [string ran abcdef 0 end-2147483649] [::string range abcdef 0 end-2147483649] \
        [{*}{string range} abcdef 0 end-2147483649] [string wordend abc 1] \
        [string wordst abc 1]
}
p
show q
