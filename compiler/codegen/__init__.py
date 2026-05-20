"""Tcl code generation — two backends behind a shared front-end.

The compiler front-end (parse → IR → CFG → SSA → lowering) is shared;
this package holds the two back-ends that consume a lowered
:class:`CFGModule` and emit executable output:

- :mod:`compiler.codegen.bytecode` — Tcl VM bytecode assembly text,
  matching ``tcl::unsupported::disassemble`` (driven by the internal
  Python VM in ``tooling/vm``).
- :mod:`compiler.codegen.wasm` — WebAssembly binary modules, plus the
  whole-program linker in :mod:`compiler.codegen.wasm.link`.

There is deliberately no parent-level re-export: a caller picks a
backend explicitly (``from compiler.codegen.bytecode import …`` /
``from compiler.codegen.wasm import …``) so the two stay symmetric and
neither is privileged as "the" codegen API.
"""

from __future__ import annotations
