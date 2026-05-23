"""Tcl dialect support and command-spec data.

`dialects/` houses every variant of Tcl: the vanilla Tcl 8.4 / 8.5+ /
9.0 stdlib specs, tcllib, expect, EDA tool dialects, F5 BigIP/iRules,
and Tk. It's a data-heavy layer — command specifications, parser
rules, and dialect-specific diagnostics.

Other concerns import dialect-specific helpers (e.g. F5 BigIP parsing
lives under `dialects.f5.bigip`); the registry runtime lives in
`compiler.registry/` and consumes spec data from here through a
uniform loader.

`dialects/` may import from `shared/` and from `compiler.registry`
(registration primitives only); it must not import from `analyser/`,
`server/`, `tooling/`, `ai/`, or from compiler internals like
`compiler.codegen`, `compiler.ssa`, etc.
"""
