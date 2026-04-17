set result [lmap x {1 2 3 4 5} {
    expr {$x * $x}
}]
