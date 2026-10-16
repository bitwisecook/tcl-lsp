# The message and error code each release gives the errors a command or an
# expression raises as it runs, at top level and in a procedure.
proc show {script} {
    set ::errorCode NONE
    set code [catch {uplevel 1 $script} result]
    puts [string map [list \n { | }] "$code <$result> <$::errorCode>"]
}
set cases {
    {string}
    {string foo x}
    {lindex}
    {expr {08}}
    {expr 08}
    {set x 08; expr {$x + 1}}
    {expr {$}}
    {set x $; string length $x}
    {expr {1/0}}
    {expr {1%0}}
    {expr {1/0.0}}
    {expr {-1/0.0}}
    {expr {inf}}
    {expr {inf + 1}}
    {set x inf; expr {$x + 1}}
    {expr {sqrt(-1)}}
    {expr {log(0)}}
    {expr {1e400}}
    {expr {pow(10, 400)}}
    {expr {abs("x")}}
    {expr {"x" + 1}}
    {set x 1/0; expr $x}
    {expr {0.0/0.0}}
    {expr {"" + 1}}
    {set x 0o8; expr {$x + 1}}
    {set x " -08 "; expr {$x * 2}}
    {expr {exp(1000)}}
    {expr {sqrt("x")}}
    {expr {abs("")}}
    {expr {$ + 1}}
    {expr {"$"}}
    {lindex {a b} 1}
}
foreach case $cases {
    show $case
}
proc p {cases} {
    foreach case $cases {
        show $case
    }
}
p $cases
proc q {} {
    show {expr {08}}
    show {expr {1/0}}
    show {expr {1/0.0}}
    show {expr {inf}}
    show {expr {$}}
    show {lindex}
}
q
