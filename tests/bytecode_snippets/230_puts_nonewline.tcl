# ``puts -nonewline <string>`` must not append a trailing newline.
# Previously the option token was silently ignored; every ``puts``
# call — regardless of ``-nonewline`` — emitted the string plus
# ``\n``, so building up a line incrementally was impossible.
# The actual output is verified via the harness's captured stdout;
# here we just need to make sure the command compiles cleanly and
# the dispatch path selects the new helper (runtime side trap
# otherwise).
puts -nonewline abc
puts def
puts -nonewline ""
puts line
return ok
