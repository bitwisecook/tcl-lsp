"""TclOO method resolution order.

Implements the same MRO algorithm as Tcl 8.6/9.0's ``tclOOCall.c``:

1. Two-pass DFS matching ``TclOOGetCallContext()``:
   - Pass 1 (``BUILDING_MIXINS``): traverse the hierarchy but only
     collect classes reached via a mixin edge (``TRAVERSED_MIXIN``).
   - Pass 2: traverse again but only collect classes reached via
     non-mixin edges (superclass relationships).
   This guarantees all mixin-path classes precede all non-mixin
   classes in the final chain.

2. Within each pass, the traversal matches
   ``AddSimpleClassChainToCallContext()``: process class-level
   mixins first, then add the class itself, then recurse into
   superclasses.

3. Late-placement deduplication from ``AddMethodToCallChain()``:
   when a class is encountered again it is *moved to the end* of
   the list (not skipped).  The comment in tclOOCall.c says
   "methods come as *late* in the call chain as possible".

The algorithm is **identical** between Tcl 8.6 and 9.0; the only
9.0 change is TIP 500 private method support which does not affect
traversal order.

Reference: ``AddSimpleClassChainToCallContext()`` and
``TclOOGetCallContext()`` in ``generic/tclOOCall.c``.
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
    is_mixin_path: bool,
    building_mixins: bool,
) -> None:
    """Recursive DFS matching TclOO's AddSimpleClassChainToCallContext.

    For each class:
    1. Recurse into class-level mixins (setting mixin-path flag)
    2. Add the class itself if MIXIN_CONSISTENT allows it
    3. Recurse into superclasses (inheriting current mixin-path status)

    The *is_mixin_path* flag corresponds to Tcl's ``TRAVERSED_MIXIN``:
    True when the class was reached via a mixin edge.

    The *building_mixins* flag corresponds to Tcl's ``BUILDING_MIXINS``:
    when set, only mixin-path classes are added; when clear, only
    non-mixin-path classes are added.
    """
    if cls in visiting:
        return  # cycle guard (Tcl lacks this but would infinite-loop on cycles)
    visiting.add(cls)
    try:
        # 1. Process class-level mixins (enter mixin path)
        for mixin in mixins_map.get(cls, []):
            _tcloo_dfs(
                mixin,
                mixins_map,
                supers_map,
                result,
                visiting,
                is_mixin_path=True,
                building_mixins=building_mixins,
            )

        # 2. Add own class with MIXIN_CONSISTENT gate:
        #    Pass 1 (building_mixins=True):  only add if on mixin path
        #    Pass 2 (building_mixins=False): only add if NOT on mixin path
        if building_mixins == is_mixin_path:
            if cls in result:
                result.remove(cls)
            result.append(cls)

        # 3. Process superclasses (inherit mixin-path status)
        for parent in supers_map.get(cls, []):
            _tcloo_dfs(
                parent,
                mixins_map,
                supers_map,
                result,
                visiting,
                is_mixin_path=is_mixin_path,
                building_mixins=building_mixins,
            )
    finally:
        visiting.discard(cls)


def c3_linearise(
    class_name: str,
    superclasses_map: dict[str, list[str]],
    mixins_map: dict[str, list[str]] | None = None,
) -> list[str]:
    """Return the method resolution order for *class_name*.

    Uses TclOO's two-pass DFS + late-placement algorithm (NOT C3
    linearisation).  The function name is kept for backwards
    compatibility.

    *superclasses_map* maps class name -> direct superclasses.
    *mixins_map* maps class name -> mixin classes (processed before supers).

    Returns a list with ancestors in resolution order: mixin-path
    classes first, then non-mixin classes.

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

    # Pass 1: BUILDING_MIXINS — only collect mixin-path classes
    _tcloo_dfs(
        class_name,
        mixins_map,
        superclasses_map,
        result,
        set(),
        is_mixin_path=False,
        building_mixins=True,
    )

    # Pass 2: collect non-mixin-path classes (shares result list for dedup)
    _tcloo_dfs(
        class_name,
        mixins_map,
        superclasses_map,
        result,
        set(),
        is_mixin_path=False,
        building_mixins=False,
    )

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
