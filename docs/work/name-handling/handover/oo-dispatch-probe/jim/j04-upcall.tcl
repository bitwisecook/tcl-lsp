# Jim upcall: call the command that a local proc replaced
proc greet {} {return orig}
proc wrap {} {
    local proc greet {} {return "new+[upcall greet]"}
    return [greet]
}
puts [wrap]
puts [greet]
