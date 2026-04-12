proc greet {{name "World"} {greeting "Hello"}} {
    return "$greeting, $name!"
}
proc compute {{a 1} {b 2} {c 3}} {
    expr {$a + $b + $c}
}
