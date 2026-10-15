set cases [list plain alpha space {two words} sigil {$literal} bracket {[literal]} backslash {a\b} newline "a\nb" open "a\{b" balanced "a\{b\}c" close "a_\}_closebrace" rooted ::selected_root empty {} array {selected_array(key)}]
foreach {label name} $cases {
    set expected VALUE_$label
    set $name $expected
    set reference [format {${%s}} $name]
    set script [format {list %s} $reference]
    set code [catch $script value]
    puts [list $label code $code exact [expr {$code == 0 && [string equal $value $expected]}] reference $reference value $value]
}
