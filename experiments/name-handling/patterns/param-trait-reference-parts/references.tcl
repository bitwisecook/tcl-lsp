proc probe {label script} {
    set status [catch {uplevel 1 $script} result]
    puts [list $label $status $result]
}
set 1abc DIGIT
set {} EMPTY
set {odd name} ODD
set caf ASCII
set café UNICODE
set "a{b" FIRST
set {a{b}c} NESTED
set v ROOT
set k KEY
set arr(KEY) ARRAY
set {arr($k)} LITERALINDEX
probe digit {set result $1abc}
probe empty {set result ${}}
probe odd {set result ${odd name}}
probe unicode {set result $café}
probe nested {set result ${a{b}c}}
probe scalar_then_index {set result ${v}($k)}
probe array_index {set result $arr($k)}
probe braced_array_index {set result ${arr($k)}}
probe escaped {set result \$1abc}
probe literal {set result {$1abc}}
