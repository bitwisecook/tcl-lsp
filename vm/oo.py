"""TclOO object model for the VM runtime.

Implements class and object storage matching Tcl 9.0's OO system.
Method dispatch uses C3 linearisation identical to TclOO.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import TYPE_CHECKING

from core.analysis.mro import C3Error, c3_linearise

from .types import TclError, TclResult, TclReturn

if TYPE_CHECKING:
    from core.compiler.codegen import FunctionAsm

    from .interp import TclInterp


@dataclass
class TclOOMethod:
    """A single method on a class or object."""

    name: str
    params: list[tuple[str, str | None]]  # (name, default_or_None)
    param_names: list[str]
    body: str
    has_args: bool = False
    visibility: str = "public"  # public | private | unexported
    compiled_asm: FunctionAsm | None = None


@dataclass
class TclOOClass:
    """A TclOO class definition."""

    name: str
    qualified_name: str
    superclasses: list[str] = field(default_factory=list)
    mixins: list[str] = field(default_factory=list)
    methods: dict[str, TclOOMethod] = field(default_factory=dict)
    class_methods: dict[str, TclOOMethod] = field(default_factory=dict)
    constructor: TclOOMethod | None = None
    destructor: TclOOMethod | None = None
    variables: list[str] = field(default_factory=list)
    filters: list[str] = field(default_factory=list)
    _mro_cache: list[str] | None = field(default=None, repr=False)

    def mro(self, class_registry: dict[str, TclOOClass]) -> list[str]:
        """Return the C3 linearised MRO for this class."""
        if self._mro_cache is not None:
            return self._mro_cache
        supers_map: dict[str, list[str]] = {}
        for qn, cls in class_registry.items():
            parents = list(cls.mixins) + list(cls.superclasses)
            # Normalise parent names to qualified form
            normalised: list[str] = []
            for p in parents:
                if p.startswith("::"):
                    normalised.append(p)
                elif f"::{p}" in class_registry:
                    normalised.append(f"::{p}")
                else:
                    normalised.append(p)
            supers_map[qn] = normalised
        try:
            result = c3_linearise(self.qualified_name, supers_map)
        except C3Error:
            result = [self.qualified_name]
        self._mro_cache = result
        return result

    def invalidate_mro(self) -> None:
        self._mro_cache = None


@dataclass
class TclOOObject:
    """A TclOO object instance."""

    name: str
    class_name: str
    namespace: str  # per-instance namespace
    instance_methods: dict[str, TclOOMethod] = field(default_factory=dict)
    _vars: dict[str, str] = field(default_factory=dict)
    _arrays: dict[str, dict[str, str]] = field(default_factory=dict)


class OORuntime:
    """OO runtime state for the interpreter."""

    def __init__(self) -> None:
        self.classes: dict[str, TclOOClass] = {}
        self.objects: dict[str, TclOOObject] = {}
        self._next_obj_id = 0

        # Register oo::object as the root class
        root = TclOOClass(name="object", qualified_name="::oo::object")
        self.classes["::oo::object"] = root

    def register_class(self, cls: TclOOClass) -> None:
        """Register a class in the runtime."""
        self.classes[cls.qualified_name] = cls
        # Invalidate MRO cache for all classes (a new class may affect subclasses)
        for c in self.classes.values():
            c.invalidate_mro()

    def create_object(
        self,
        interp: TclInterp,
        class_name: str,
        obj_name: str | None = None,
        args: list[str] | None = None,
    ) -> str:
        """Create a new object instance of the given class."""
        cls = self.classes.get(class_name)
        if cls is None:
            raise TclError(f'unknown class "{class_name}"')

        if obj_name is None:
            self._next_obj_id += 1
            obj_name = f"::oo::Obj{self._next_obj_id}"

        ns = f"{obj_name}"
        obj = TclOOObject(
            name=obj_name,
            class_name=class_name,
            namespace=ns,
        )
        self.objects[obj_name] = obj

        # Run constructor if present
        if cls.constructor is not None:
            self._invoke_method(interp, obj, cls.constructor, args or [])

        # Register the object as a command
        self._register_object_command(interp, obj)

        return obj_name

    def _register_object_command(self, interp: TclInterp, obj: TclOOObject) -> None:
        """Register the object as a command that dispatches method calls."""

        def _obj_dispatch(interp: TclInterp, args: list[str]) -> TclResult:

            if not args:
                raise TclError(f'wrong # args: should be "{obj.name} method ?arg ...?"')
            method_name = args[0]
            method_args = args[1:]

            if method_name == "destroy":
                return self._destroy_object(interp, obj)

            method = self.resolve_method(obj, method_name)
            if method is None:
                raise TclError(
                    f'unknown method "{method_name}": must be '
                    + ", ".join(self._available_methods(obj))
                )
            return self._invoke_method(interp, obj, method, method_args)

        interp._runtime_commands[obj.name] = _obj_dispatch

    def resolve_method(self, obj: TclOOObject, method_name: str) -> TclOOMethod | None:
        """Resolve a method on an object using MRO."""
        # Instance methods first
        if method_name in obj.instance_methods:
            return obj.instance_methods[method_name]

        # Walk MRO
        cls = self.classes.get(obj.class_name)
        if cls is None:
            return None

        for class_qname in cls.mro(self.classes):
            ancestor = self.classes.get(class_qname)
            if ancestor and method_name in ancestor.methods:
                return ancestor.methods[method_name]

        return None

    def _available_methods(self, obj: TclOOObject) -> list[str]:
        """Return list of available method names for error messages."""
        methods = set(obj.instance_methods.keys())
        methods.add("destroy")
        cls = self.classes.get(obj.class_name)
        if cls:
            for class_qname in cls.mro(self.classes):
                ancestor = self.classes.get(class_qname)
                if ancestor:
                    methods.update(
                        m for m, md in ancestor.methods.items() if md.visibility == "public"
                    )
        return sorted(methods)

    def _invoke_method(
        self,
        interp: TclInterp,
        obj: TclOOObject,
        method: TclOOMethod,
        args: list[str],
    ) -> TclResult:
        """Invoke a method on an object, setting up the instance context."""
        from .scope import CallFrame

        cls = self.classes.get(obj.class_name)
        proc_ns = interp.root_namespace

        frame = CallFrame(
            level=interp.current_frame.level + 1,
            proc_name=f"{obj.name} {method.name}" if method.name else obj.name,
            parent=interp.current_frame,
            namespace=proc_ns,
            interp=interp,
            call_args=args,
        )

        # Bind instance variables into the frame
        if cls:
            for var_name in cls.variables:
                if var_name in obj._vars:
                    frame.set_var(var_name, obj._vars[var_name])

        # Bind parameters
        all_params = [(n, d) for n, d in method.params if n != "args"]
        required = [(n, d) for n, d in all_params if d is None]

        if not method.has_args and len(args) > len(all_params):
            raise TclError(
                f'wrong # args: should be "{obj.name} {method.name}'
                + (" " + " ".join(n for n, _ in method.params) if method.params else "")
                + '"'
            )
        if len(args) < len(required):
            raise TclError(
                f'wrong # args: should be "{obj.name} {method.name}'
                + (" " + " ".join(n for n, _ in method.params) if method.params else "")
                + '"'
            )

        for i, (pname, default) in enumerate(all_params):
            if i < len(args):
                frame.set_var(pname, args[i])
            elif default is not None:
                frame.set_var(pname, default)

        if method.has_args:
            remaining = args[len(all_params) :]
            frame.set_var("args", " ".join(remaining))

        # Store context for `self` and `my` resolution
        frame._oo_self = obj.name
        frame._oo_class = obj.class_name
        frame._oo_method = method.name

        # Execute method body
        old_frame = interp.current_frame
        interp.current_frame = frame
        try:
            result = interp.eval(method.body)
        except TclReturn as ret:
            result = TclResult(value=ret.value)
        finally:
            # Write back instance variables before restoring frame
            if cls:
                for var_name in cls.variables:
                    try:
                        obj._vars[var_name] = frame.get_var(var_name)
                    except Exception:
                        pass
            interp.current_frame = old_frame
        return result

    def _destroy_object(self, interp: TclInterp, obj: TclOOObject) -> TclResult:
        """Destroy an object, running its destructor if present."""
        from .types import TclResult

        cls = self.classes.get(obj.class_name)
        if cls and cls.destructor:
            self._invoke_method(interp, obj, cls.destructor, [])

        # Remove object command and storage
        interp._runtime_commands.pop(obj.name, None)
        self.objects.pop(obj.name, None)
        return TclResult()

    def self_name(self, interp: TclInterp) -> str:
        """Return the name of the current object (for `self` command)."""
        frame = interp.current_frame
        obj_name = getattr(frame, "_oo_self", None)
        if obj_name is None:
            raise TclError('"self" may only be invoked from within a method')
        return obj_name

    def my_dispatch(self, interp: TclInterp, args: list[str]) -> TclResult:
        """Dispatch `my method args...` from within a method body."""
        if not args:
            raise TclError('wrong # args: should be "my method ?arg ...?"')

        frame = interp.current_frame
        obj_name = getattr(frame, "_oo_self", None)
        if obj_name is None:
            raise TclError('"my" may only be invoked from within a method')

        obj = self.objects.get(obj_name)
        if obj is None:
            raise TclError(f'object "{obj_name}" has been destroyed')

        method_name = args[0]
        method = self.resolve_method(obj, method_name)
        if method is None:
            raise TclError(f'unknown method "{method_name}"')
        return self._invoke_method(interp, obj, method, args[1:])

    def next_dispatch(self, interp: TclInterp, args: list[str]) -> TclResult:
        """Dispatch `next` — call next method in MRO chain."""
        frame = interp.current_frame
        obj_name = getattr(frame, "_oo_self", None)
        class_name = getattr(frame, "_oo_class", None)
        method_name = getattr(frame, "_oo_method", None)
        if obj_name is None or class_name is None or method_name is None:
            raise TclError('"next" may only be invoked from within a method')

        obj = self.objects.get(obj_name)
        if obj is None:
            raise TclError(f'object "{obj_name}" has been destroyed')

        cls = self.classes.get(class_name)
        if cls is None:
            raise TclError(f'class "{class_name}" not found')

        mro = cls.mro(self.classes)
        # Find current class in MRO and get next
        found_current = False
        for class_qname in mro:
            if found_current:
                ancestor = self.classes.get(class_qname)
                if ancestor and method_name in ancestor.methods:
                    return self._invoke_method(interp, obj, ancestor.methods[method_name], args)
            ancestor = self.classes.get(class_qname)
            if ancestor and method_name in ancestor.methods:
                found_current = True

        from .types import TclResult

        return TclResult()
