# Custom integer codes and named codes; invalid codes.
proc f {c} { return -code $c "c=$c" }
foreach c {ok error return break continue 0 1 2 3 4 5 99 -3} { puts "$c: [catch {f $c} r] $r" }
puts "bad: [catch {f bogus} r] $r"
puts "toplevel custom: [catch {return -code 42 x} r] $r"
