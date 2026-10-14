puts [list version [info patchlevel]]
set x SCALAR
set {odd name} SPACE
set {cash$name} CASH
set {scalar(open} OPEN
set {a{b}c} NESTED
set {} EMPTY
set a(k) ELEMENT
foreach {key script} {
    scalar {list $x}
    bracedScalar {list ${x}}
    space {list ${odd name}}
    literalDollar {list ${cash$name}}
    unmatchedOpen {list ${scalar(open}}
    nestedBraces {list ${a{b}c}}
    emptyScalar {list ${}}
    bareArray {list $a(k)}
    bracedArray {list ${a(k)}}
    compound {list ${x}suffix}
    dynamicIndex {list $a($x)}
} {
    set code [catch {eval $script} value]
    puts [list $key $code $value]
}
