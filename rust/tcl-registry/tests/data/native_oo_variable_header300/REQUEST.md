# Original TclOO variable declaration header

Problem: a declaration consumer can preserve a counted name but replace its original object header, making byte equality insufficient to establish original-object retention.

Question: after directly invoking `oo::define C variable ORIGINAL`, where ORIGINAL is `Tcl_NewStringObj` with exactly `61 00 7a`, does direct `info class variables C` return that same original object as its sole list member?

Use fresh pinned C Tcl 8.4/8.5/8.6/9.0/9.1 and current Jim builds, recording source/header/library/compiler/build hashes, full version and native configuration. Compile this exact public C observer (USE_JIM only for Jim). The C observer calls full Tcl_Init before the setup. Unsupported stock TclOO is reported as not applicable. Do not add source escapes or use binary format; the original has a counted raw zero byte, not modified UTF-8 C0 80. Keep the original alive until after the query, with pointer equality computed inside the interpreter process and emitted only as a boolean. No address is evidence or required output.

Retain complete stdout/stderr/exit and compile argv/status. DECLARE and QUERY codes are separate. QUERY fields are guest query code, list getter code, list length, same-object boolean and member native string getter bytes. This checks only direct declaration and direct introspection; method resolver frames, execution, caller namespace and arbitrary slot overrides are separate purposes.
