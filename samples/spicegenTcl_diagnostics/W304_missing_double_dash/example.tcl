# W304: Missing '--' option terminator before substituted input
# Source: SpiceGenTcl/src/generalClasses.tcl (lsearch, regexp, switch)
#         SpiceGenTcl/src/ngspice/netlistParserClassNgspice.tcl
#
# When a variable is passed where a command could interpret it as an
# option (starting with '-'), the '--' terminator prevents misparse.
#
# Expected: tclsh runs — demonstrates the option-injection issue.

# lsearch without '--': if $needle starts with '-', it's parsed as a flag
proc find_item {haystack needle} {
    # Risky: if needle is "-exact", lsearch treats it as an option
    set idx [lsearch $haystack $needle]
    return $idx
}

# With '--': safe regardless of needle content
proc find_item_safe {haystack needle} {
    set idx [lsearch -- $haystack $needle]
    return $idx
}

set items {apple -exact banana cherry}

puts "find '-exact' without --: [find_item $items -exact]"
puts "find '-exact' with --:    [find_item_safe $items -exact]"

# regexp without '--'
set pattern {^hello}
set input "-nocase world"
# Without '--', regexp might interpret $input as a flag
catch {regexp $pattern $input} err
puts "regexp without --: [expr {$err eq 0 ? {no match} : $err}]"
regexp -- $pattern "hello world" match
puts "regexp with --: matched=$match"

# switch without '--'
set val "-glob"
switch -- $val {
    -glob   { puts "switch matched: -glob (as value, not option)" }
    default { puts "switch: default" }
}

puts "W304 test complete"
