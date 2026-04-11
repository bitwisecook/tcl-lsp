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
from ..ir import (
    IRCall,
    IRCatch,
    IRFor,
    IRForeach,
    IRIf,
    IRModule,
    IRScript,
    IRStatement,
    IRSwitch,
    IRTry,
    IRWhile,
)
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

    def _source_file_arg(args: tuple[str, ...]) -> str | None:
        """Extract the filename from ``source ?-encoding enc? filename``."""
        remaining = list(args)
        while remaining and remaining[0].startswith("-"):
            flag = remaining.pop(0)
            if flag == "-encoding" and remaining:
                remaining.pop(0)  # skip the encoding value
        if not remaining:
            return None
        target = remaining[0]
        if target.startswith("$") or target.startswith("["):
            return None
        return target

    def _scan_stmt(stmt: IRStatement) -> None:
        if isinstance(stmt, IRCall) and stmt.command == "source" and stmt.args:
            target = _source_file_arg(stmt.args)
            if target is not None:
                targets.append(target)

    def _scan_script(script: IRScript) -> None:
        for stmt in script.statements:
            _scan_stmt(stmt)
            # Recurse into nested control-flow bodies
            if isinstance(stmt, IRIf):
                for clause in stmt.clauses:
                    _scan_script(clause.body)
                if stmt.else_body:
                    _scan_script(stmt.else_body)
            elif isinstance(stmt, IRFor):
                _scan_script(stmt.init)
                _scan_script(stmt.body)
                _scan_script(stmt.next)
            elif isinstance(stmt, IRWhile):
                _scan_script(stmt.body)
            elif isinstance(stmt, IRForeach):
                _scan_script(stmt.body)
            elif isinstance(stmt, IRCatch):
                _scan_script(stmt.body)
            elif isinstance(stmt, IRTry):
                _scan_script(stmt.body)
                if stmt.finally_body:
                    _scan_script(stmt.finally_body)
            elif isinstance(stmt, IRSwitch):
                for arm in stmt.arms:
                    if arm.body:
                        _scan_script(arm.body)
                if stmt.default_body:
                    _scan_script(stmt.default_body)

    _scan_script(ir_module.top_level)
    for proc in ir_module.procedures.values():
        _scan_script(proc.body)
    return targets


def _extract_package_requires(ir_module: IRModule) -> list[str]:
    """Scan IR for ``package require`` commands, returning package names."""
    packages: list[str] = []

    def _scan_stmt(stmt: IRStatement) -> None:
        if isinstance(stmt, IRCall) and stmt.command == "package" and stmt.args:
            if stmt.args[0] == "require":
                # package require ?-exact? name ?version?
                args = stmt.args[1:]
                if args and args[0] == "-exact":
                    args = args[1:]
                if args:
                    pkg_name = args[0]
                    if pkg_name != "Tcl" and pkg_name != "tcl":
                        packages.append(pkg_name)

    def _scan_script(script: IRScript) -> None:
        for stmt in script.statements:
            _scan_stmt(stmt)
            if isinstance(stmt, IRIf):
                for clause in stmt.clauses:
                    _scan_script(clause.body)
                if stmt.else_body:
                    _scan_script(stmt.else_body)
            elif isinstance(stmt, IRFor):
                _scan_script(stmt.init)
                _scan_script(stmt.body)
                _scan_script(stmt.next)
            elif isinstance(stmt, IRWhile):
                _scan_script(stmt.body)
            elif isinstance(stmt, IRForeach):
                _scan_script(stmt.body)
            elif isinstance(stmt, IRCatch):
                _scan_script(stmt.body)
            elif isinstance(stmt, IRTry):
                _scan_script(stmt.body)
                if stmt.finally_body:
                    _scan_script(stmt.finally_body)

    _scan_script(ir_module.top_level)
    return packages


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
) -> list[Path]:
    """Resolve a ``package require`` to source files.

    Searches for the package by:
    1. Looking for ``<name>/<name>.tcl`` or ``<name>/<name>.tm`` in search paths
    2. Parsing ``pkgIndex.tcl`` files for ``package ifneeded`` scripts that
       contain ``source`` commands pointing to the actual source files
    3. Checking for Tcl Modules (``<name>-<version>.tm``) in search paths

    Returns a list of source file paths (may be empty).
    """
    results: list[Path] = []
    for sp in search_paths:
        # Direct package directory with matching source file
        pkg_dir = sp / package_name
        if pkg_dir.is_dir():
            for suffix in (".tcl", ".tm"):
                candidate = pkg_dir / f"{package_name}{suffix}"
                if candidate.is_file():
                    results.append(candidate)
                    return results
            # Parse pkgIndex.tcl for source targets
            idx = pkg_dir / "pkgIndex.tcl"
            if idx.is_file():
                sources = _parse_pkg_index(idx, package_name)
                if sources:
                    results.extend(sources)
                    return results
        # Tcl Module files: <name>-<version>.tm
        if sp.is_dir():
            for f in sp.iterdir():
                if f.name.startswith(f"{package_name}-") and f.suffix == ".tm":
                    results.append(f)
                    return results
        # pkgIndex.tcl at the search path root
        idx = sp / "pkgIndex.tcl"
        if idx.is_file():
            sources = _parse_pkg_index(idx, package_name)
            if sources:
                results.extend(sources)
                return results
    return results


def _parse_pkg_index(pkg_index: Path, package_name: str) -> list[Path]:
    """Parse a ``pkgIndex.tcl`` file for ``package ifneeded`` scripts.

    Looks for lines matching::

        package ifneeded <name> <version> [list source <file>]
        package ifneeded <name> <version> "source <file>"

    Returns a list of resolved source file paths.
    """
    import re

    results: list[Path] = []
    try:
        text = pkg_index.read_text(encoding="utf-8", errors="replace")
    except OSError:
        return results

    base_dir = pkg_index.parent
    # Match: package ifneeded <name> <version> <script>
    pattern = re.compile(
        r"package\s+ifneeded\s+" + re.escape(package_name) + r"\s+\S+\s+(.*)",
    )
    for line in text.splitlines():
        line = line.strip()
        m = pattern.match(line)
        if not m:
            continue
        script = m.group(1)
        # Extract source file from: [list source $dir/<file>]
        # or: "source $dir/<file>"
        source_match = re.search(
            r'source\s+(?:\[file\s+join\s+\$dir\s+)?["\']?([^\s\]"\']+)', script
        )
        if source_match:
            rel_path = source_match.group(1)
            # Replace $dir with the pkgIndex.tcl directory
            rel_path = rel_path.replace("$dir/", "").replace("$dir\\", "")
            candidate = base_dir / rel_path
            if candidate.is_file():
                results.append(candidate)
    return results


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

        base_dir = path.parent

        # Resolve source commands
        targets = _extract_source_targets(ir_module)
        for target in targets:
            target_path = _resolve_file(target, base_dir, resolved_search)
            if target_path is not None:
                _resolve_recursive(target_path.resolve(), depth + 1)

        # Resolve package require dependencies
        packages = _extract_package_requires(ir_module)
        for pkg_name in packages:
            pkg_files = _resolve_package(pkg_name, resolved_search + (base_dir,))
            for pkg_path in pkg_files:
                _resolve_recursive(pkg_path.resolve(), depth + 1)

    _resolve_recursive(main_path, 0)

    merged = merge_ir_modules(*modules)
    cfg = build_cfg(merged)
    return wasm_codegen_module(cfg, merged, optimise=optimise)
