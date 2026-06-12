# FP-STY-15 fixture: `$` before a closing `"` is a literal regex end-anchor.
# The closing quote must terminate the word — no E002/E205, and the end-anchor
# is NOT a substitution so no W306.
regsub "\n$" $msg "" out
string match "abc$" $x
regexp -- "^foo$" $text
# A live `$bar` (b is a name char) IS a substitution and MUST still fire W306:
regexp -- "^foo$bar" $text
