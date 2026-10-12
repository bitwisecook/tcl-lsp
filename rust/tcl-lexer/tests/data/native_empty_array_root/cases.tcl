puts "VERSION|[info patchlevel]"
proc observe {label script} {
    set code [catch {uplevel 1 $script} result]
    binary scan $result H* hex
    puts "CASE|$label|$code|$hex"
}
observe EMPTY_LEXICAL {set {(k)} VALUE; set observed $(k)}
observe EMPTY_BRACED {set {(k)} VALUE; set observed ${(k)}}
observe EMPTY_SCALAR {set {(k)} VALUE; set observed ${}}
observe EMPTY_LITERAL {set {(k)} VALUE; set observed {$(k)}}
observe NAMED_INDEX {set k k; set a(k) ARRAY; set observed $a($k)}
