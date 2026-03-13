set d [dict create name Alice age 30]
dict update d name n age a {
    set n "Bob"
    incr a
}
