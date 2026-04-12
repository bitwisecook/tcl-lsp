# Inheritance with next dispatch (oo-7.3 equivalent)
oo::class create superClass {
    method doit x {
        lappend ::result |$x|
    }
}
oo::class create subClass {
    superclass superClass
    method doit x {lappend ::result -$x-; next [incr x]}
}
set result {}
[subClass new] doit 1
set result
