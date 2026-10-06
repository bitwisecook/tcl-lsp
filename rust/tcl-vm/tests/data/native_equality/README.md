# Native physical equality

`probe.c` calls C Tcl `TclStringCmp` full case-sensitive equality and Jim
`Jim_StringCompareObj` on the original objects. Fixed byte pairs cross fresh
string, prepared Unicode/string-count, pure byte array and resident byte-array
representations, with same-object and distinct-object inputs.

Each TSV row records case identity, equality, then left/right primary type and
resident-string presence before and after the call. No string observer runs
between those physical observations. C Tcl 8.4 and 8.5 lack this internal
object-comparison entry; their manifest entries record explicit unavailability.
