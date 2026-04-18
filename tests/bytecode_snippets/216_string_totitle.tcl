# Tcl ``string totitle`` uppercases the first alphabetic character and
# lowercases every other alphabetic character.  Regression guard for
# the bug where the subcommand fell through the string-dispatch table
# (no entry in _STRING_SUBCMD_IMPORT) and returned empty.
if {[string totitle hello] ne "Hello"} { error "hello" }
if {[string totitle HELLO] ne "Hello"} { error "HELLO" }
if {[string totitle {hello world}] ne "Hello world"} { error "multi-word" }
# Non-alpha prefix doesn't consume the "title" slot.
if {[string totitle 123abc] ne "123Abc"} { error "digits-prefix" }
if {[string totitle {}] ne ""} { error "empty" }
return ok
