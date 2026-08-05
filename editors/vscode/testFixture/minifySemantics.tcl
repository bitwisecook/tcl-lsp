# Minifier semantic-correctness fixture (issues #1192, #1193, #1194, #1197).
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
