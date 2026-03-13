proc helper {x} {
    return [expr {$x + 1}]
}
proc caller {x} {
    tailcall helper $x
}
