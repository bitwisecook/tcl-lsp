# Minifier semantic-correctness fixture.
proc longprocedure {} {
    set arr(longmember) 1
    return [array get arr]
}
puts [info procs longprocedure]
puts [longprocedure]
switch # {
    # {puts matched}
    default {puts default}
}
puts [set a]
