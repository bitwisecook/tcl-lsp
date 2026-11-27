# Native Jim namespace helper bodies

These files contain the exact bodies of the pinned Jim 0.84 namespace
helper procedures and the generated static extension sources that
`native_jim_lookup` selects, including the leading/trailing newline and
indentation returned by `info body`. The adjacent `LICENSE.jim` retains
Jim's original notices. Do not add source comments inside these bodies:
procedure introspection observes their complete spelling.

The dictionary and ensemble bodies, and the scripted-distribution roster
that also installs `namespace info` and `namespace ensemble`, are in
`rust/tcl-registry/src/native_scripts`.
