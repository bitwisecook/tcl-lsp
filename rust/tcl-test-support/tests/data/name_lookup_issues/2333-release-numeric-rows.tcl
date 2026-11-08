set c integer
foreach v {4294967296 2147483648 -4294967295 0x100000000 18446744073709551615} {
    lappend classes [string is $c $v]
}
puts $classes
foreach e {{int(1e20)} {wide(-9.3e18)} {round(1e20)} {int(9.2e18)} {exp(710)} {log(0)}} {
    set code [catch {expr $e} m]
    puts "$e $code $m"
}
