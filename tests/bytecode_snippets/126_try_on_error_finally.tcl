set cleanup 0
try {
    expr {1/0}
} on error {msg} {
    set err $msg
} finally {
    set cleanup 1
}
