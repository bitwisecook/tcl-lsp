proc literal_word {} {set x 42; return {$x}}
proc compound_word {} {set a 42; set a:b 99; return $a:b}
proc array_word {} {set arr 42; return ${arr(k)}}
foreach name {literal_word compound_word array_word} {
    set code [catch $name value]
    puts [list $name $code $value]
}
