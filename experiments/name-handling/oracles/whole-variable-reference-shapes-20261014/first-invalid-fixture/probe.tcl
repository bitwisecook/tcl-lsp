set a A
set {a:b} AB
set caf PREFIX
set café UNICODE
set {} EMPTY
set {scalar(open} OPEN
set arr(key) ELEMENT
set {a{b} FIRST
set {a{b}c} NESTED
foreach {label script} [list isolated_colon {list $a:b} unicode {list $café} empty {list ${}} open_scalar {list ${scalar(open}} combined_element {list ${arr(key)}} nested_brace {list ${a{b}c}} separate_element {list $arr(key)}] {
    set code [catch $script value]
    puts [list $label $code $value]
}
