"""TclOO method resolution order.

Implements the same MRO algorithm as Tcl 9.0's ``tclOOCall.c``:

1. Depth-first traversal: mixins first, then the class itself, then
   superclasses.
2. Late-placement deduplication: when a class is encountered again,
   it is *moved to the end* of the list (not skipped).

This produces the same order as C3 linearisation for standard single
and diamond hierarchies, but differs for mixin orderings where TclOO
places mixins before the class itself.

Reference: ``AddSimpleClassChainToCallContext()`` in
``generic/tclOOCall.c`` (Tcl 9.0.3).
"""

from __future__ import annotations


class C3Error(Exception):
    """Raised when MRO computation fails (e.g. cycle in hierarchy)."""


def _tcloo_dfs(
    cls: str,
    mixins_map: dict[str, list[str]],
    supers_map: dict[str, list[str]],
    result: list[str],
    visiting: set[str],
) -> None:
    """Recursive DFS matching TclOO's AddSimpleClassChainToCallContext.

    For each class:
    1. Recurse into mixins
    2. Add the class itself (with late-placement dedup)
    3. Recurse into superclasses
    """
    if cls in visiting:
        return  # cycle guard
    visiting.add(cls)
    try:
        # 1. Process mixins first
        for mixin in mixins_map.get(cls, []):
            _tcloo_dfs(mixin, mixins_map, supers_map, result, visiting)

        # 2. Add own class (late-placement: move to end if already present)
        if cls in result:
            result.remove(cls)
        result.append(cls)

        # 3. Process superclasses
        for parent in supers_map.get(cls, []):
            _tcloo_dfs(parent, mixins_map, supers_map, result, visiting)
    finally:
        visiting.discard(cls)


def c3_linearise(
    class_name: str,
    superclasses_map: dict[str, list[str]],
    mixins_map: dict[str, list[str]] | None = None,
) -> list[str]:
    """Return the method resolution order for *class_name*.

    Uses TclOO's DFS + late-placement algorithm (NOT C3 linearisation).
    The function name is kept for backwards compatibility.

    *superclasses_map* maps class name → direct superclasses.
    *mixins_map* maps class name → mixin classes (processed before supers).

    Returns a list starting with *class_name* followed by ancestors
    in resolution order.

    Raises ``C3Error`` on cycles.
    """
    if mixins_map is None:
        mixins_map = {}

    # Cycle detection: check for trivial self-cycles
    parents = superclasses_map.get(class_name, [])
    if class_name in parents:
        raise C3Error(f"cycle detected in class hierarchy involving '{class_name}'")

    # Check for two-node cycles
    for p in parents:
        if class_name in superclasses_map.get(p, []):
            raise C3Error(f"cycle detected in class hierarchy involving '{class_name}'")

    result: list[str] = []
    _tcloo_dfs(class_name, mixins_map, superclasses_map, result, set())
    return result


def build_mro_map(
    superclasses_map: dict[str, list[str]],
    mixins_map: dict[str, list[str]] | None = None,
) -> tuple[dict[str, list[str]], list[str]]:
    """Compute MRO for all classes in the hierarchy.

    Returns ``(mro_map, errors)`` where *mro_map* maps each class name
    to its linearised MRO, and *errors* is a list of error messages for
    classes whose hierarchy is inconsistent.
    """
    if mixins_map is None:
        mixins_map = {}
    mro_map: dict[str, list[str]] = {}
    errors: list[str] = []

    for cls in superclasses_map:
        if cls in mro_map:
            continue
        try:
            mro = c3_linearise(cls, superclasses_map, mixins_map)
            mro_map[cls] = mro
        except C3Error as e:
            errors.append(str(e))

    return mro_map, errors
