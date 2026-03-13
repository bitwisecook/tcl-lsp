set cleanup 0
try {
    set x 42
} finally {
    set cleanup 1
}
