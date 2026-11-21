package require snit
snit::type Engine { method start {} { return vroom } }
snit::type Car {
    component engine
    delegate method start to engine
    constructor {} { set engine [Engine %AUTO%] }
}
Car c
puts [c start]
puts [catch {c stop} m]:$m ;# BAD not delegated
