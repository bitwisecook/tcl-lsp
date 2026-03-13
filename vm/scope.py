"""Variable storage, call frames, namespaces, and scoping for the Tcl VM."""

from __future__ import annotations

import fnmatch
import re
from typing import TYPE_CHECKING

from .types import TclError

if TYPE_CHECKING:
    from .commands import CommandHandler
    from .interp import ProcDef, TclInterp

# Matches ``arrayName(elementKey)`` — greedy so nested parens work.
_ARRAY_RE = re.compile(r"^([^(]+)\((.+)\)$", re.DOTALL)


def parse_array_ref(name: str) -> tuple[str, str | None]:
    """Split *name* into ``(array_name, element_key)`` or ``(scalar, None)``."""
    # Fast path: avoid regex for the common case (no parentheses).
    paren = name.find("(")
    if paren < 0 or not name.endswith(")"):
        return name, None
    return name[:paren], name[paren + 1 : -1]


# Namespace


class Namespace:
    """A Tcl namespace — owns variables, commands, and child namespaces.

    The global namespace is ``::`` and is the root of the namespace tree.
    """

    __slots__ = (
        "name",
        "qualname",
        "parent",
        "children",
        "_scalars",
        "_arrays",
        "_commands",
        "_procs",
        "_export_patterns",
        "_path",
        "_unknown_handler",
        "_frame",
    )

    def __init__(self, name: str, parent: Namespace | None = None) -> None:
        self.name = name
        if parent is None:
            self.qualname = "::"
        elif parent.qualname == "::":
            self.qualname = f"::{name}"
        else:
            self.qualname = f"{parent.qualname}::{name}"
        self.parent = parent
        self.children: dict[str, Namespace] = {}
        self._scalars: dict[str, str] = {}
        self._arrays: dict[str, dict[str, str]] = {}
        self._commands: dict[str, CommandHandler] = {}  # name -> handler callable
        self._procs: dict[str, ProcDef] = {}  # name -> ProcDef
        self._export_patterns: list[str] = []
        self._path: list[Namespace] = []
        self._unknown_handler: str | None = None
        self._frame: CallFrame | None = None

    def get_frame(self, interp: TclInterp) -> CallFrame:
        """Return the persistent CallFrame for this namespace.

        For the root namespace ``::`` this is the global frame.  For child
        namespaces a dedicated ``CallFrame`` is created lazily so that
        ``namespace eval`` operates on an isolated variable scope (matching
        real Tcl behaviour).
        """
        if self._frame is not None:
            return self._frame
        # Create a namespace-level frame with parent=global_frame so that
        # ``global`` declarations and ``::`` prefix resolution still reach
        # the global scope.
        frame = CallFrame(
            level=0,
            parent=interp.global_frame,
            namespace=self,
            interp=interp,
        )
        self._frame = frame
        return frame

    # child namespace management

    def find_child(self, name: str) -> Namespace | None:
        """Look up a direct child namespace by name."""
        return self.children.get(name)

    def create_child(self, name: str) -> Namespace:
        """Create (or return existing) child namespace *name*."""
        if name in self.children:
            return self.children[name]
        child = Namespace(name, parent=self)
        self.children[name] = child
        return child

    def delete(self) -> None:
        """Remove this namespace from its parent and clear contents."""
        if self.parent is not None:
            self.parent.children.pop(self.name, None)
        for child in list(self.children.values()):
            child.delete()
        self.children.clear()
        self._scalars.clear()
        self._arrays.clear()
        self._commands.clear()
        self._procs.clear()

    # variable access (namespace-level)

    def ns_get_var(self, name: str) -> str | None:
        """Read a scalar from this namespace; returns None if absent."""
        return self._scalars.get(name)

    def ns_set_var(self, name: str, value: str) -> None:
        self._scalars[name] = value

    def ns_unset_var(self, name: str) -> bool:
        if name in self._scalars:
            del self._scalars[name]
            return True
        if name in self._arrays:
            del self._arrays[name]
            return True
        return False

    def ns_var_exists(self, name: str) -> bool:
        return name in self._scalars or name in self._arrays

    def ns_array_get(self, name: str) -> dict[str, str] | None:
        return self._arrays.get(name)

    def ns_array_set(self, name: str, elem: str, value: str) -> None:
        self._arrays.setdefault(name, {})[elem] = value

    def ns_var_names(self) -> list[str]:
        names = list(self._scalars.keys())
        names.extend(self._arrays.keys())
        return names

    # command access

    def register_command(self, name: str, handler: CommandHandler) -> None:
        self._commands[name] = handler

    def lookup_command(self, name: str) -> CommandHandler | None:
        return self._commands.get(name)

    def remove_command(self, name: str) -> None:
        self._commands.pop(name, None)

    def register_proc(self, name: str, proc_def: ProcDef) -> None:
        self._procs[name] = proc_def

    def lookup_proc(self, name: str) -> ProcDef | None:
        return self._procs.get(name)

    def remove_proc(self, name: str) -> None:
        self._procs.pop(name, None)

    def proc_names(self) -> list[str]:
        """Return names of all procs registered in this namespace."""
        return list(self._procs.keys())

    def command_names(self) -> list[str]:
        """Return names of all commands registered in this namespace."""
        return list(self._commands.keys())

    # export / import

    def add_export(self, pattern: str) -> None:
        if pattern not in self._export_patterns:
            self._export_patterns.append(pattern)

    def exported_commands(self) -> list[str]:
        """Return command names matching export patterns."""
        result: list[str] = []
        all_names = list(self._commands.keys()) + list(self._procs.keys())
        for pat in self._export_patterns:
            for name in all_names:
                if fnmatch.fnmatch(name, pat) and name not in result:
                    result.append(name)
        return result

    def __repr__(self) -> str:
        return f"Namespace({self.qualname!r})"


def resolve_namespace(root: Namespace, qualname: str) -> Namespace | None:
    """Resolve a qualified namespace name starting from *root*.

    ``::foo::bar`` resolves from root.  Relative names are resolved
    from root as well (the caller should resolve relative to the
    current namespace before calling this).
    """
    if qualname == "::" or qualname == "":
        return root
    parts = qualname.lstrip(":").split("::")
    parts = [p for p in parts if p]
    ns = root
    for part in parts:
        child = ns.find_child(part)
        if child is None:
            return None
        ns = child
    return ns


def ensure_namespace(root: Namespace, qualname: str) -> Namespace:
    """Create namespace path if needed and return the leaf namespace."""
    if qualname == "::" or qualname == "":
        return root
    parts = qualname.lstrip(":").split("::")
    parts = [p for p in parts if p]
    ns = root
    for part in parts:
        ns = ns.create_child(part)
    return ns


def namespace_qualifiers(name: str) -> str:
    """Return everything before the last ``::`` separator.

    ``::foo::bar::baz`` -> ``::foo::bar``
    ``::foo`` -> ``::``
    ``foo`` -> ``""``
    """
    idx = name.rfind("::")
    if idx < 0:
        return ""
    result = name[:idx]
    return result if result else "::"


def namespace_tail(name: str) -> str:
    """Return the simple name after the last ``::`` separator.

    ``::foo::bar::baz`` -> ``baz``
    ``::foo`` -> ``foo``
    ``foo`` -> ``foo``
    """
    idx = name.rfind("::")
    if idx < 0:
        return name
    return name[idx + 2 :]


class CallFrame:
    """One stack frame — global level or a procedure invocation."""

    __slots__ = (
        "level",
        "proc_name",
        "call_args",
        "parent",
        "namespace",
        "_scalars",
        "_arrays",
        "_aliases",
        "_globals",
        "_declared",
        "_interp",
    )

    def __init__(
        self,
        level: int = 0,
        proc_name: str | None = None,
        parent: CallFrame | None = None,
        namespace: Namespace | None = None,
        interp: TclInterp | None = None,
        call_args: list[str] | None = None,
    ) -> None:
        self.level = level
        self.proc_name = proc_name
        self.call_args = call_args
        self.parent = parent
        self.namespace = namespace
        self._scalars: dict[str, str] = {}
        self._arrays: dict[str, dict[str, str]] = {}
        # upvar aliases: local_name -> (target_frame, target_name)
        self._aliases: dict[str, tuple[CallFrame, str]] = {}
        # Names declared ``global`` in this frame.
        self._globals: set[str] = set()
        # Names declared via ``variable`` but not yet initialised.
        # These appear in ``info vars`` but ``info exists`` returns 0.
        self._declared: set[str] = set()
        # Optional interpreter reference for firing variable traces.
        self._interp = interp

    # alias / global helpers

    def add_alias(self, local_name: str, target_frame: CallFrame, target_name: str) -> None:
        """Create an upvar alias from *local_name* to *target_frame*'s *target_name*."""
        self._aliases[local_name] = (target_frame, target_name)

    def declare_global(self, name: str) -> None:
        """Mark *name* as a global-linked variable in this frame.

        In Tcl, calling ``global`` inside a proc after a local variable
        with the same name already has a value raises
        ``variable "X" already exists``.  At global scope the command
        is a harmless no-op.
        """
        from .types import TclError

        if (
            self.parent is not None
            and name not in self._globals
            and (name in self._scalars or name in self._arrays)
        ):
            raise TclError(f'variable "{name}" already exists')
        self._globals.add(name)

    def _resolve(self, name: str, _seen: set[int] | None = None) -> tuple[CallFrame, str]:
        """Follow aliases and global declarations to the owning frame."""
        # Handle :: prefix — resolve to global frame / target namespace
        if name.startswith("::"):
            stripped = name[2:]
            root = self
            while root.parent is not None:
                root = root.parent
            # Continue resolution on the root frame (may have aliases)
            return root._resolve(stripped)

        # Handle qualified names like ``foo::bar`` — resolve the namespace
        # and look up the variable in that namespace's frame.
        if "::" in name:
            qual = name[: name.rfind("::")]
            tail = name[name.rfind("::") + 2 :]
            # Walk to the root frame to get the interp
            root = self
            while root.parent is not None:
                root = root.parent
            interp = root._interp
            if interp is not None:
                ns = resolve_namespace(interp.root_namespace, f"::{qual}")
                if ns is not None:
                    ns_frame = ns.get_frame(interp)
                    return ns_frame._resolve(tail)
            # Fallback: treat as plain name
            return self, name

        if name in self._aliases:
            target_frame, target_name = self._aliases[name]
            # Guard against infinite alias loops
            if _seen is None:
                _seen = set()
            key = id(target_frame) ^ hash(target_name)
            if key in _seen:
                return target_frame, target_name
            _seen.add(key)
            # Recursively resolve on the target frame
            return target_frame._resolve(target_name, _seen)
        if name in self._globals and self.parent is not None:
            root = self
            while root.parent is not None:
                root = root.parent
            return root._resolve(name)
        return self, name

    # variable location helpers

    def _locate(self, name: str) -> tuple[CallFrame, str, str | None]:
        """Resolve *name* to ``(frame, array_or_scalar, element_or_None)``.

        After following aliases, global declarations and ``::`` prefixes
        the resolved name may itself contain an array reference (e.g. an
        ``upvar`` alias from ``debug`` to ``Option(-debug)``).  This
        helper handles that case so callers don't need to re-parse.
        """
        arr_name, elem = parse_array_ref(name)
        frame, resolved = self._resolve(arr_name if elem is not None else name)

        if elem is not None:
            return frame, resolved, elem

        # The resolved name may itself be an array element.
        r_arr, r_elem = parse_array_ref(resolved)
        if r_elem is not None:
            return frame, r_arr, r_elem
        return frame, resolved, None

    # trace helpers

    def _fire_var_traces(self, interp: TclInterp, name: str, op: str) -> None:
        """Fire traces for *name*, checking both local and resolved names.

        When a variable is accessed through an upvar alias, the trace may
        be registered on the target (resolved) name rather than the local
        alias name.  This helper fires traces for both, passing the local
        alias name to the callback (as Tcl does).
        """
        from .commands.trace_cmds import fire_traces

        arr_name, arr_elem = parse_array_ref(name)
        local_key = arr_name if arr_elem is not None else name

        # Check local name first
        if local_key in interp.variable_traces:
            fire_traces(interp, name, op)
            return

        # Check resolved name (through aliases) — pass the local name
        # for the callback but look up traces via the resolved name
        frame, resolved = self._resolve(local_key)
        if resolved != local_key and resolved in interp.variable_traces:
            fire_traces(interp, name, op, lookup_key=resolved)

    def _has_var_traces(self, interp: TclInterp, name: str) -> bool:
        """Return True if *name* has traces (directly or via alias)."""
        arr_name, arr_elem = parse_array_ref(name)
        local_key = arr_name if arr_elem is not None else name
        if local_key in interp.variable_traces:
            return True
        frame, resolved = self._resolve(local_key)
        return resolved != local_key and resolved in interp.variable_traces

    # scalar variable access

    def get_var(self, name: str, *, default: str | None = None) -> str:
        """Read a scalar or array element."""
        # Fire read traces before reading (the trace callback may
        # initialise the variable — e.g. tcltest's SafeFetch).
        interp = self._interp
        if interp is not None and interp.variable_traces:
            self._fire_var_traces(interp, name, "read")

        frame, resolved, elem = self._locate(name)

        if elem is not None:
            arr = frame._arrays.get(resolved)
            if arr is not None:
                if elem in arr:
                    return arr[elem]
                if default is not None:
                    return default
                raise TclError(f'can\'t read "{name}": no such element in array')
            # Not an array — is it a scalar?
            if resolved in frame._scalars:
                if default is not None:
                    return default
                raise TclError(f"can't read \"{name}\": variable isn't array")
            if default is not None:
                return default
            raise TclError(f'can\'t read "{name}": no such variable')

        # Plain scalar — check for array conflict
        if resolved in frame._arrays:
            if default is not None:
                return default
            raise TclError(f'can\'t read "{name}": variable is array')
        val = frame._scalars.get(resolved)
        if val is not None:
            return val
        if default is not None:
            return default
        raise TclError(f'can\'t read "{name}": no such variable')

    def set_var(self, name: str, value: str) -> str:
        """Write a scalar or array element; returns *value*."""
        frame, resolved, elem = self._locate(name)

        # Save old value for rollback if a write trace errors
        old_scalar: str | None = None
        old_array_elem: str | None = None
        had_old = False

        if elem is not None:
            # Setting array element — check scalar conflict
            if resolved in frame._scalars:
                raise TclError(f"can't set \"{name}\": variable isn't array")
            arr = frame._arrays.get(resolved)
            if arr is not None and elem in arr:
                old_array_elem = arr[elem]
                had_old = True
            else:
                # New element — invalidate active array searches
                from .commands.array_cmds import _invalidate_searches

                _invalidate_searches(resolved)
            frame._arrays.setdefault(resolved, {})[elem] = value
            frame._declared.discard(resolved)
        else:
            # Setting scalar — check array conflict
            if resolved in frame._arrays:
                raise TclError(f'can\'t set "{name}": variable is array')
            old_scalar = frame._scalars.get(resolved)
            had_old = old_scalar is not None
            frame._scalars[resolved] = value
            frame._declared.discard(resolved)

        # Fire write traces if an interpreter is available
        interp = self._interp
        if interp is not None and interp.variable_traces and self._has_var_traces(interp, name):
            try:
                self._fire_var_traces(interp, name, "write")
            except TclError:
                # Restore old value on trace rejection
                if elem is not None:
                    arr = frame._arrays.get(resolved)
                    if arr is not None:
                        if had_old:
                            if old_array_elem is not None:
                                arr[elem] = old_array_elem
                        else:
                            arr.pop(elem, None)
                else:
                    if had_old:
                        if old_scalar is not None:
                            frame._scalars[resolved] = old_scalar
                    else:
                        frame._scalars.pop(resolved, None)
                raise
        return value

    def unset_var(self, name: str, *, nocomplain: bool = False) -> None:
        """Remove a scalar or array element."""
        frame, resolved, elem = self._locate(name)

        if elem is not None:
            arr = frame._arrays.get(resolved)
            if arr is not None:
                if elem in arr:
                    # Removing element — invalidate active array searches
                    from .commands.array_cmds import _invalidate_searches

                    _invalidate_searches(resolved)
                    # Fire unset trace before removing
                    self._fire_unset_trace(name)
                    del arr[elem]
                    return
                if not nocomplain:
                    raise TclError(f'can\'t unset "{name}": no such element in array')
                return
            # Not an array — is it a scalar?
            if resolved in frame._scalars:
                if not nocomplain:
                    raise TclError(f"can't unset \"{name}\": variable isn't array")
                return
        else:
            if resolved in frame._scalars:
                # Fire unset trace before removing
                self._fire_unset_trace(name)
                del frame._scalars[resolved]
                # Clean up variable traces for unset scalars
                self._cleanup_traces(name, resolved)
                return
            if resolved in frame._arrays:
                # Fire unset trace before removing
                self._fire_unset_trace(name)
                del frame._arrays[resolved]
                # Clean up variable traces for unset arrays
                self._cleanup_traces(name, resolved)
                return

        if not nocomplain:
            raise TclError(f'can\'t unset "{name}": no such variable')

    def _fire_unset_trace(self, name: str) -> None:
        """Fire unset traces for *name* if an interpreter is available."""
        interp = self._interp
        if interp is None:
            return
        try:
            self._fire_var_traces(interp, name, "unset")
        except Exception:
            pass  # Unset trace errors are silently ignored

    def _cleanup_traces(self, name: str, resolved: str) -> None:
        """Remove variable traces when a variable is unset."""
        interp = self._interp
        if interp is None:
            return
        # Remove traces registered under either the original or resolved name
        interp.variable_traces.pop(name, None)
        if resolved != name:
            interp.variable_traces.pop(resolved, None)

    def exists(self, name: str) -> bool:
        """Return True if *name* exists as a scalar or array element."""
        frame, resolved, elem = self._locate(name)

        if elem is not None:
            arr = frame._arrays.get(resolved)
            return arr is not None and elem in arr
        return resolved in frame._scalars or resolved in frame._arrays

    def var_names(self) -> list[str]:
        """Return all variable names in this frame (scalars + arrays + aliases + declared)."""
        names_set = set(self._scalars.keys())
        names_set.update(self._arrays.keys())
        # Include upvar aliases — they are visible as local variables
        names_set.update(self._aliases.keys())
        # Include variables declared via ``variable`` but not yet initialised
        names_set.update(self._declared)
        return list(names_set)

    # array-level access

    def array_get(self, name: str) -> dict[str, str]:
        frame, resolved = self._resolve(name)
        return dict(frame._arrays.get(resolved, {}))

    def array_set(self, name: str, mapping: dict[str, str]) -> None:
        frame, resolved = self._resolve(name)
        if resolved in frame._scalars:
            if mapping:
                raise TclError(f"can't set \"{name}({next(iter(mapping))})\": variable isn't array")
            raise TclError(f"can't array set \"{name}\": variable isn't array")
        frame._arrays.setdefault(resolved, {}).update(mapping)

    def array_names(self, name: str) -> list[str]:
        frame, resolved = self._resolve(name)
        arr = frame._arrays.get(resolved)
        return list(arr.keys()) if arr else []

    def array_exists(self, name: str) -> bool:
        frame, resolved = self._resolve(name)
        return resolved in frame._arrays

    def array_size(self, name: str) -> int:
        frame, resolved = self._resolve(name)
        arr = frame._arrays.get(resolved)
        return len(arr) if arr else 0
