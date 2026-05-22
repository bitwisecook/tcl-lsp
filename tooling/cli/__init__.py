"""Shared CLI framework: compile pipeline + result serialisation.

This package is the common machinery the tcl-lsp CLI tools build on —
the compile pipeline (`pipeline.py`), the result serialiser
(`serialise.py`), output formatters (`formatters.py`), and
shell-completion support (`_argcomplete_support.py`).  The per-tool
verb registries live with their tools (`tooling.tcl.verbs`,
`tooling.f5.verbs`) and the F5 remote REST/UCS client lives in
`tooling.f5.f5_remote`.

It is consumed by the per-tool CLI trees — `tooling.tcl`, `tooling.f5`,
`tooling.wasm` — and by the compiler explorer (`tooling.explorer.cli`
for its CLI face, `tooling.explorer.web` for its web face).  It is NOT
itself a command; it has no `__main__`.
"""
