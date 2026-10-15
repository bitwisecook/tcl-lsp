# Selected POSIX shell literal-source observations

`original/` retains the executed driver, complete shell source files, exact original Unicode inputs and inherited environment bytes, argv/context/channel/completion records and actual `/bin/sh` resolved binary identity/hash before and after. No binary is included. Original checksum seal: `213b21d19cee65082d388bdb1c3072856d7b67dc83919bd3d0be5bb7acec6e09`.

Run `python3 verify.py` to verify both closures without executing a shell, provider, Rust implementation or network request. The executed driver's absolute paths describe the actual capture; they are historical provenance, not portable discovery instructions. Fresh execution requires a separately selected shell and fresh source/input/identity/context receipts.

Twelve invocations completed with twelve exact checks: eight original literal argv values, including quotes, dollars, backticks, brackets, newlines and Unicode; four quoted environment-suffix observations, including absent, empty and original opaque OS bytes. Actual shell identity/hash remained unchanged. Raw channels remain byte-valued. These are source-recipe observations for the captured `/bin/sh` only; no Rust control, Tcl interpreter, Fish or Docker grammar ran. They grant no installed command identity, Native currentness, arbitrary external encoding or filesystem selection.
