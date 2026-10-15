set literal redirected
set cases [list plain key space {two words} sigil {$literal} bracket {[list redirected]} backslash {\x41} newline "a\nb" empty {} open {(} close {)} nested {a(b)c} balanced {a{b}c} unmatched "a\{b"]
set arr(redirected) REDIRECTED
set arr(A) DECODED
foreach {label index} $cases {
    set expected VALUE_$label
    set name [format {arr(%s)} $index]
    set $name $expected
    foreach form [list direct braced] {
        if {[string equal $form direct]} {
            set reference [format {$arr(%s)} $index]
        } else {
            set reference [format {${arr(%s)}} $index]
        }
        set script [format {list %s} $reference]
        set code [catch $script value]
        puts [list $label form $form code $code exact [expr {$code == 0 && [string equal $value $expected]}] reference $reference value $value]
    }
}
