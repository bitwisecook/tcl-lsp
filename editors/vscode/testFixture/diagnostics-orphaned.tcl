# W125 - orphaned control-flow keywords (misplaced newline)

# Orphaned else — should produce W125
if {1} {
    puts yes
}
else {
    puts no
}

# Orphaned elseif — should produce W125
if {1} {
    puts a
}
elseif {0} {
    puts b
}

# Correct else — should NOT produce W125
if {1} {
    puts yes
} else {
    puts no
}
