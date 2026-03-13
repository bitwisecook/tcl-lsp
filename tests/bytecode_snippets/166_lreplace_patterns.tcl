set lst {a b c d e}
set a [lreplace $lst 1 2 X Y Z]
set b [lreplace $lst 0 0]
set c [lreplace $lst end end W]
