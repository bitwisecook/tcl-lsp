proc myerror {} {
    return -code error "something went wrong"
}
proc myreturn {} {
    return -code return "early exit"
}
