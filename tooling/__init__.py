"""Developer tools: CLIs, the bytecode VM, debugger, fuzzer, compiler explorer, codemods, and the Tcl package manager.

This is an umbrella concern. Its sub-packages are loosely coupled and
share no internal API surface beyond what they import from `compiler/`,
`analyser/`, `dialects/`, and `shared/`. Each sub-package is itself an
entry point: `tooling.tcl`, `tooling.f5`, `tooling.wasm`,
`tooling.explorer`, `tooling.vm`, `tooling.debugger`, `tooling.fuzzing`,
`tooling.tclpkg`, plus the per-codemod helpers (`refactoring/`,
`formatter/`, `minifier/`, `diagram/`, `irule_test/`).
"""
