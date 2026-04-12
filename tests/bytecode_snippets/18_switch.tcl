set x abc
switch $x {
    abc {set y 1}
    def {set y 2}
    default {set y 0}
}
