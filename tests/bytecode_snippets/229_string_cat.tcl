# ``string cat ?arg ...?`` concatenates its arguments with no
# separator and no whitespace trimming (unlike ``concat``).  Previously
# there was no codegen entry for ``cat`` and the command returned
# empty.
if {[string cat] ne ""} { error "cat() != empty" }
if {[string cat abc def ghi] ne "abcdefghi"} { error "literal cat wrong" }
if {[string cat single] ne "single"} { error "one-arg cat wrong" }

# Mixed literal / variable args must also concatenate correctly.
set x hello
set y [string cat $x - world]
if {$y ne "hello-world"} { error "mixed cat = '$y'" }

# ``string cat`` does not trim whitespace (unlike concat).
if {[string cat {  a  } {  b  }] ne "  a    b  "} { error "cat trimmed whitespace!" }
return ok
