# A quote or brace opens a word only at its start, in a command substitution
# as anywhere else (Tcl_ParseCommand).
proc show {script} {
    set code [catch {uplevel 1 $script} result]
    puts "$code <$result>"
}
show {expr {[string length a"]}}
show {set x [string length a"]}
show {set x "[string length a"]"}
show {expr {[string length "a]"]}}
show {expr {[string length {a]}]}}
show {set a(x]) 1; expr {[string length $a(x])]}}
show {set x [string length a"b"c]}
show {set x [list a"b c"d]}
proc p {} { expr {[string length a"]} }
show p
