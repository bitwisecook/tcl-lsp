"""C3 linearisation for TclOO class hierarchies.

Implements the same C3 method resolution order algorithm used by TclOO
(and Python). Given a class hierarchy graph, produces the linearised MRO
for each class.

Reference: "A Monotonic Superclass Linearization for Dylan"
(Barrett et al., 1996, OOPSLA).
"""

from __future__ import annotations


class C3Error(Exception):
    """Raised when C3 linearisation fails due to an inconsistent hierarchy."""


def c3_linearise(
    class_name: str,
    superclasses_map: dict[str, list[str]],
) -> list[str]:
    """Return the method resolution order for *class_name* using C3 linearisation.

    *superclasses_map* maps each class name to its direct superclasses
    (in declaration order).  Classes not present in the map are assumed
    to have no superclasses (leaf classes or ``oo::object``).

    Returns a list starting with *class_name* itself, followed by its
    linearised ancestors.

    Raises ``C3Error`` if the hierarchy is inconsistent (no valid
    linearisation exists).
    """
    cache: dict[str, list[str]] = {}
    return _c3(class_name, superclasses_map, cache, set())


def _c3(
    cls: str,
    supers_map: dict[str, list[str]],
    cache: dict[str, list[str]],
    in_progress: set[str],
) -> list[str]:
    """Recursive C3 with cycle detection and memoisation."""
    if cls in cache:
        return cache[cls]

    if cls in in_progress:
        raise C3Error(f"cycle detected in class hierarchy involving '{cls}'")

    in_progress.add(cls)
    try:
        parents = supers_map.get(cls, [])
        if not parents:
            result = [cls]
            cache[cls] = result
            return result

        # Compute linearisations of each parent
        parent_linears = [_c3(p, supers_map, cache, in_progress) for p in parents]
        # Merge: parent linearisations + list of parents
        result = [cls] + _merge(parent_linears + [list(parents)], cls)
        cache[cls] = result
        return result
    finally:
        in_progress.discard(cls)


def _merge(lists: list[list[str]], cls: str) -> list[str]:
    """Merge step of C3 linearisation.

    Takes a list of linearisation lists and merges them according to
    the C3 algorithm: at each step, pick the first head that does not
    appear in the tail of any other list.
    """
    result: list[str] = []
    # Work on copies so we can mutate
    seqs = [lst[:] for lst in lists if lst]

    while seqs:
        # Find a good head: one that doesn't appear in any other tail
        candidate = None
        for seq in seqs:
            head = seq[0]
            # Check if head appears in the tail of any other sequence
            in_tail = False
            for other in seqs:
                if head in other[1:]:
                    in_tail = True
                    break
            if not in_tail:
                candidate = head
                break

        if candidate is None:
            remaining = [s[0] for s in seqs if s]
            raise C3Error(
                f"inconsistent hierarchy for '{cls}': "
                f"cannot linearise {remaining}"
            )

        result.append(candidate)
        # Remove candidate from the head of all sequences
        seqs = [s[1:] if s[0] == candidate else s for s in seqs]
        # Remove empty sequences
        seqs = [s for s in seqs if s]

    return result


def build_mro_map(
    superclasses_map: dict[str, list[str]],
) -> tuple[dict[str, list[str]], list[str]]:
    """Compute MRO for all classes in the hierarchy.

    Returns ``(mro_map, errors)`` where *mro_map* maps each class name
    to its linearised MRO, and *errors* is a list of error messages for
    classes whose hierarchy is inconsistent.
    """
    mro_map: dict[str, list[str]] = {}
    errors: list[str] = []

    for cls in superclasses_map:
        if cls in mro_map:
            continue
        try:
            mro = c3_linearise(cls, superclasses_map)
            mro_map[cls] = mro
        except C3Error as e:
            errors.append(str(e))

    return mro_map, errors
