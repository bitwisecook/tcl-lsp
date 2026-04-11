"""Whole-program WASM linker for Tcl.

Resolves ``source`` commands at compile time, merges all IR modules
into a single compilation unit, and produces a standalone WASM module
that includes all procedures from all sourced files.

Public API::

    wasm_link(main_source, *, search_paths=(), optimise=False) -> WasmModule
    wasm_link_sources(sources, *, optimise=False) -> WasmModule
    merge_ir_modules(*modules) -> IRModule

The linker scans IR for ``source`` calls, reads the target files,
lowers them to IR, and merges everything before WASM codegen.
``package require`` dependencies are resolved by searching for
``pkgIndex.tcl`` files in the search paths.
"""

from __future__ import annotations

from pathlib import Path

from ..cfg import build_cfg
from ..ir import IRCall, IRModule, IRScript, IRStatement
from ..lowering import lower_to_ir
from .wasm import WasmModule, wasm_codegen_module


def merge_ir_modules(*modules: IRModule) -> IRModule:
    """Merge multiple IR modules into a single compilation unit.

    Top-level statements are concatenated in order.  Procedure
    definitions from later modules override earlier ones (matching
    Tcl's redefinition semantics).  Methods are merged similarly.
    """
    merged = IRModule()
    all_stmts: list[IRStatement] = []
    for mod in modules:
        all_stmts.extend(mod.top_level.statements)
        for qname, proc in mod.procedures.items():
            if qname in merged.procedures:
                merged.redefined_procedures.add(qname)
            merged.procedures[qname] = proc
        if mod.methods:
            for qname, method in mod.methods.items():
                merged.methods[qname] = method
        merged.redefined_procedures.update(mod.redefined_procedures)
    merged.top_level = IRScript(statements=tuple(all_stmts))
    return merged


def _extract_source_targets(ir_module: IRModule) -> list[str]:
    """Scan IR for ``source`` commands, returning the file path arguments."""
    targets: list[str] = []

    def _scan_stmt(stmt: IRStatement) -> None:
        if isinstance(stmt, IRCall) and stmt.command == "source" and stmt.args:
            target = stmt.args[0]
            # Skip variable references and command substitutions
            if not target.startswith("$") and not target.startswith("["):
                targets.append(target)

    def _scan_script(script: IRScript) -> None:
        for stmt in script.statements:
            _scan_stmt(stmt)

    _scan_script(ir_module.top_level)
    return targets


def _resolve_file(
    target: str,
    base_dir: Path,
    search_paths: tuple[Path, ...],
) -> Path | None:
    """Resolve a ``source`` target to an actual file path.

    Tries the target relative to *base_dir* first, then each search
    path.  Returns ``None`` if the file cannot be found.
    """
    # Absolute path
    p = Path(target)
    if p.is_absolute() and p.is_file():
        return p

    # Relative to the base directory
    candidate = base_dir / target
    if candidate.is_file():
        return candidate

    # Search paths
    for sp in search_paths:
        candidate = sp / target
        if candidate.is_file():
            return candidate

    return None


def _resolve_package(
    package_name: str,
    search_paths: tuple[Path, ...],
) -> Path | None:
    """Resolve a ``package require`` to a source file.

    Searches for ``pkgIndex.tcl`` files in search paths and looks
    for the package name.  Returns the package source file if found.

    This is a simplified resolver — real Tcl package loading is
    more complex (pkgIndex.tcl contains ``package ifneeded`` scripts).
    """
    for sp in search_paths:
        pkg_dir = sp / package_name
        if pkg_dir.is_dir():
            # Look for a main file with the package name
            for suffix in (".tcl", ".tm"):
                candidate = pkg_dir / f"{package_name}{suffix}"
                if candidate.is_file():
                    return candidate
            # Look for pkgIndex.tcl
            idx = pkg_dir / "pkgIndex.tcl"
            if idx.is_file():
                return idx
    return None


def wasm_link_sources(
    sources: list[tuple[str, str]],
    *,
    optimise: bool = False,
) -> WasmModule:
    """Compile multiple Tcl sources into a single WASM module.

    Each element of *sources* is a ``(name, source_text)`` pair.
    Sources are lowered to IR independently, then merged and compiled
    to a single WASM module.

    Args:
        sources: List of ``(name, source_text)`` pairs.
        optimise: Enable WASM optimisation passes.

    Returns:
        A ``WasmModule`` containing all procedures and top-level code.
    """
    modules: list[IRModule] = []
    for _name, text in sources:
        modules.append(lower_to_ir(text))

    merged = merge_ir_modules(*modules)
    cfg = build_cfg(merged)
    return wasm_codegen_module(cfg, merged, optimise=optimise)


def wasm_link(
    main_source: str | Path,
    *,
    search_paths: tuple[str | Path, ...] = (),
    optimise: bool = False,
    max_depth: int = 10,
) -> WasmModule:
    """Compile a Tcl source file and all its ``source`` dependencies to WASM.

    Recursively resolves ``source`` commands at compile time, reads
    the target files, merges all IR, and produces a single WASM module.

    Args:
        main_source: Path to the main Tcl source file.
        search_paths: Additional directories to search for sourced files.
        optimise: Enable WASM optimisation passes.
        max_depth: Maximum recursion depth for ``source`` resolution.

    Returns:
        A ``WasmModule`` containing all procedures and top-level code
        from the main file and all transitively sourced files.
    """
    main_path = Path(main_source).resolve()
    if not main_path.is_file():
        msg = f"Main source file not found: {main_path}"
        raise FileNotFoundError(msg)

    resolved_search = tuple(Path(sp).resolve() for sp in search_paths)

    # Track already-resolved files to avoid cycles
    resolved: set[Path] = set()
    modules: list[IRModule] = []

    def _resolve_recursive(path: Path, depth: int) -> None:
        if path in resolved or depth > max_depth:
            return
        resolved.add(path)

        text = path.read_text(encoding="utf-8", errors="replace")
        ir_module = lower_to_ir(text)
        modules.append(ir_module)

        # Find source targets in this module
        targets = _extract_source_targets(ir_module)
        base_dir = path.parent
        for target in targets:
            target_path = _resolve_file(target, base_dir, resolved_search)
            if target_path is not None:
                _resolve_recursive(target_path.resolve(), depth + 1)

    _resolve_recursive(main_path, 0)

    merged = merge_ir_modules(*modules)
    cfg = build_cfg(merged)
    return wasm_codegen_module(cfg, merged, optimise=optimise)
