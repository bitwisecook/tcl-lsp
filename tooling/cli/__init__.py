"""Shared CLI framework: verb registry + compile pipeline + result serialisation.

This package is the common machinery every tcl-lsp CLI tool builds on —
the verb registry (`verbs/`), the compile pipeline (`pipeline.py`), the
result serialiser (`serialise.py`), output formatters (`formatters.py`),
shell-completion support (`_argcomplete_support.py`), and the F5 remote
REST/UCS client (`f5_remote/`).

It is consumed by the per-tool CLI trees — `tooling.tcl`, `tooling.f5`,
`tooling.wasm` — and by the compiler explorer (`tooling.explorer.cli`
for its CLI face, `tooling.explorer.web` for its web face).  It is NOT
itself a command; it has no `__main__`.
"""
