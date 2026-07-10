proc renderReport {body} {
    puts $body
}

proc foo {x} {
    switch $x
    a {
        return 1
    }
    renderReport {
        Some unrelated braced-argument call, not a switch case.
    }
}
