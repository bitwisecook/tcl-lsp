"""Optional WASM extension manifest.

An *extension* adds Tcl commands to the runtime that the user's program
can request via ``package require``.  Today extensions are realised
through **runtime variants**: every extension comes with a sibling Zig
build target that produces a runtime ``.wasm`` with the extension's
``cmd_*.zig`` modules spliced into the static ``BUILTINS`` slice
(``runtime/zig/dispatch/tcl_cmd_table.zig`` ``inline if``s on the
``build_options.with_<extension>`` flag).  The bundler picks whichever
variant covers every required extension.

Trade-off vs. a fully-merged extension WASM file: the variant approach
sidesteps the multi-memory + data-placement problems we hit trying to
merge a separately-built extension with the runtime.  In return, every
extension multiplies the number of pre-built runtime artifacts by 2.
With one extension (Tcltest) we ship two runtimes; an extension matrix
beyond ~3 extensions would push us back toward the merge model.

For the long-term path to 3rd-party C extensions this manifest is
still the right shape: the descriptor's ``runtime_path_factory`` is
the only thing that needs to change to point at a future
fully-merged extension WASM, leaving the rest of the bundler intact.

Adding a new extension is a one-line append to :data:`EXTENSIONS`,
plus a matching build target under ``runtime/zig/<extension>/`` and a
``with_<extension>`` build flag.  See
``docs/design/compiler/wasm-extensions.md``.
"""

from __future__ import annotations

import logging
from collections.abc import Callable
from dataclasses import dataclass
from pathlib import Path

from ...ir import IRModule

_LOGGER = logging.getLogger(__name__)


def _repo_root() -> Path:
    """Locate the repo root by walking up from this file."""
    return Path(__file__).resolve().parents[4]


def _zig_bin() -> Path:
    return _repo_root() / "runtime" / "zig" / "zig-out" / "bin"


def _tcltest_runtime_path() -> Path:
    """Path of the tcltest-enabled runtime variant."""
    return _zig_bin() / "tcl_runtime_with_tcltest.wasm"


def default_runtime_path() -> Path:
    """Path of the lean runtime — used when no extension is requested."""
    return _zig_bin() / "tcl_runtime.wasm"


@dataclass(frozen=True)
class ExtensionDescriptor:
    """Static description of one optional extension.

    *runtime_path_factory* is the central knob: it returns the path
    of the runtime variant to bundle when this extension is required.
    For now every extension maps to a sibling runtime; once the
    merge-with-shared-memory plumbing lands the factory will return
    a separate ``.wasm`` to merge instead.
    """

    name: str
    package_names: tuple[str, ...]
    runtime_path_factory: Callable[[], Path]

    def matches_package(self, package_name: str) -> bool:
        return package_name in self.package_names

    def runtime_path(self) -> Path:
        return self.runtime_path_factory()


EXTENSIONS: tuple[ExtensionDescriptor, ...] = (
    ExtensionDescriptor(
        name="Tcltest",
        package_names=("Tcltest", "tcltest"),
        runtime_path_factory=_tcltest_runtime_path,
    ),
)


def find_required_extensions(ir_module: IRModule) -> list[ExtensionDescriptor]:
    """Return the extensions a compiled IR module needs.

    Scans the IR for ``package require <name>`` calls (delegating to
    :func:`link._extract_package_requires` so the traversal stays
    in one place) and returns each matching extension at most once,
    in :data:`EXTENSIONS` declaration order.
    """
    # Lazy import to avoid a link → wasm.__init__ import cycle.
    from .link import _extract_package_requires  # noqa: PLC0415

    requested = set(_extract_package_requires(ir_module))
    matched: list[ExtensionDescriptor] = []
    for ext in EXTENSIONS:
        if any(name in requested for name in ext.package_names):
            matched.append(ext)
    return matched


def runtime_path_for(ir_module: IRModule) -> Path:
    """Pick the runtime variant covering every required extension.

    Today: returns the matching variant when exactly one extension
    is needed (e.g. Tcltest → ``tcl_runtime_with_tcltest.wasm``).
    The variant-runtime model can't satisfy two extensions
    simultaneously without a combinatorial-explosion of pre-built
    variants, so when more than one is requested we log a warning,
    pick the *first* matching variant (the user's program at least
    gets that extension's commands), and let the bundle proceed —
    the missing extensions will surface to the script as
    ``invalid command name`` at runtime.  Once Stage 2 (separately-
    merged extension WASMs, see
    ``docs/design/compiler/wasm-extensions.md``) lands, this becomes
    a list-of-extensions handover instead of a single-variant pick.
    """
    needed = find_required_extensions(ir_module)
    if len(needed) == 0:
        return default_runtime_path()
    if len(needed) == 1:
        return needed[0].runtime_path()
    requested = [ext.name for ext in needed]
    _LOGGER.warning(
        "Multiple WASM extensions requested (%s) — the variant-runtime "
        "model only covers one at a time. Bundling against %s; commands "
        "from the others will surface as 'invalid command name'.",
        ", ".join(requested),
        needed[0].name,
    )
    return needed[0].runtime_path()
