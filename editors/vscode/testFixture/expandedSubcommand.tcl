# FP-STY-18 fixture: a `{*}`-expanded subcommand word splices its list
# elements into the argument list, so the literal source text must never be
# compared against the subcommand set. The unexpanded genuine typo below
# must still fire W001 (the marker that analysis ran).
dict {*}{create a b}
dict bogus a b
