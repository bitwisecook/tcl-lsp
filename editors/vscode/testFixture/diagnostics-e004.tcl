# E004 - malformed `if` clause shape

# Condition with no body — E004, "No script following "1" argument"
if {1}

# Dangling elseif with no body — E004
if {1} {
    puts a
} elseif {2}

# Extra words after the recognised final body — E004
if {1} {
    puts a
} {
    puts b
} {
    puts c
}

# Leading `else` as the condition — well-formed (fails at expression
# evaluation instead, a distinct problem), so this must NOT produce E004.
if else {
    puts a
}

# Well-formed elseif/else chain — no E004.
if {1} {
    puts a
} elseif {2} {
    puts b
} else {
    puts c
}
