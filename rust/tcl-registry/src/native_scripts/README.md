# Native Jim dictionary procedure bodies

These files contain the exact bodies of the pinned Jim 0.84 standard-library
procedures, including the leading/trailing newline and indentation returned
by `info body`. The adjacent `LICENSE.jim` retains Jim's original notices.
Do not add source comments inside these bodies: procedure introspection
observes their complete spelling.

`dictionary_scope::stock_scripted_wrappers` supplies the native formals and
selects this roster for the audited Jim engine. Bootstrap creates real
procedures in the retained root namespace. The separate dispatch recipe
retains the original selector spelling and argument schedule; invocation
resolves the current procedure binding rather than treating this source as
proof that an unchanged stock implementation is still installed.

Native source, body/formal captures and execution controls are retained in
`rust/tcl-vm/tests/data/native_jim_dictionary`.
