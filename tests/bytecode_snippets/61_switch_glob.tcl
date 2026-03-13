set x "hello"
switch -glob $x {
    h* {set y 1}
    *lo {set y 2}
    default {set y 0}
}
