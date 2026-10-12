# Borrowed compiled-local slots

These fixed controls compare fresh original string scripts entered through `eval`
and `uplevel` with dynamic original-object variable lookup. Names include raw NUL,
modified NUL, invalid bytes, opaque array spelling, and qualification on either side
of NUL. Procedure declarations retain one or two original formal objects.

Each expected row contains the original definition/invocation completion code and
exact result bytes captured before observer lookup. The six pinned interpreters
complete 240 references. The manifest retains source, header, library, executable
and raw-log hashes. These rows do not certify cache headers or physical object
reference counts. Single-command list evaluation has a separate native dispatch
path and does not supply borrowed-slot compilation evidence.
