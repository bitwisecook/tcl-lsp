set x "hello"
switch -exact -- $x {
    hello {set y 1}
    world {set y 2}
    default {set y 0}
}
