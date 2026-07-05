proc greet {name age} {
    return "$name is $age"
}
apply {{alpha beta} {
    return [expr {$alpha + $beta}]
}} 1 2
dict map {mk mv} $d {
    set mk $mv
}
