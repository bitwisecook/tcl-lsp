# Evidence gating for the `package require` suggestion (issue #1191).
#
# Only line 12 is an executable command head. The three lines before it
# mention the same identifier as data — a comment, a quoted string, and an
# argument word — and offering to load a package (which runs the package's
# initialisation code) on any of them is a false positive.
#
# Line numbers are load-bearing: the test drives the lightbulb at each.
# see http::fetchall for details
set example "http::fetchall"
dict set docs command http::fetchall
http::fetchall $url
