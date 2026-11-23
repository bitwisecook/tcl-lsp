proc p {} {
    defer {puts deferred}
    puts body
}
p
