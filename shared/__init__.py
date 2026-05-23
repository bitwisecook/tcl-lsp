"""Cross-cutting utilities shared by every concern.

`shared/` is a leaf package: it must not import from any sibling concern
(compiler, analyser, server, tooling, ai, dialects). It exposes the small
amount of infrastructure that those concerns all rely on — diagnostic codes,
source positions, ranges, document buffers, the WASM-runtime locator, the
tclsh discoverer, KCS help data, and the wire-format sentinels that the
compiler and the VM agree on.

Imports from this package are always safe.
"""
