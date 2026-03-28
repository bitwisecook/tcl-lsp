"""TclOO object model for the VM runtime.

Implements class and object storage matching Tcl 8.6/9.0's OO system.
Method dispatch uses the same two-pass DFS + late-placement algorithm
as ``tclOOCall.c``.
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
        """Return the MRO for this class using TclOO's two-pass DFS algorithm."""
        if self._mro_cache is not None:
            return self._mro_cache

        supers_map: dict[str, list[str]] = {}
        mixins_map: dict[str, list[str]] = {}

        def _normalise(names: list[str]) -> list[str]:
            normalised: list[str] = []
            for p in names:
                if p.startswith("::"):
                    normalised.append(p)
                elif f"::{p}" in class_registry:
                    normalised.append(f"::{p}")
                else:
                    normalised.append(p)
            return normalised

        for qn, cls in class_registry.items():
            supers_map[qn] = _normalise(list(cls.superclasses))
            if cls.mixins:
                mixins_map[qn] = _normalise(list(cls.mixins))

        try:
            result = c3_linearise(self.qualified_name, supers_map, mixins_map=mixins_map)
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

    def invalidate_all_mro(self) -> None:
        """Invalidate MRO cache for all classes."""
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

        # Run constructor — walk MRO to find the first constructor
        ctor = self._resolve_constructor(cls)
        if ctor is not None:
            self._invoke_method(interp, obj, ctor, args or [], defining_class=class_name)

        # Register the object as a command
        self._register_object_command(interp, obj)

        return obj_name

    def _resolve_constructor(self, cls: TclOOClass) -> TclOOMethod | None:
        """Find the constructor by walking the MRO chain."""
        if cls.constructor is not None:
            return cls.constructor
        for class_qname in cls.mro(self.classes):
            ancestor = self.classes.get(class_qname)
            if ancestor and ancestor.constructor is not None:
                return ancestor.constructor
        return None

    def _resolve_destructor(self, cls: TclOOClass) -> TclOOMethod | None:
        """Find the destructor by walking the MRO chain."""
        if cls.destructor is not None:
            return cls.destructor
        for class_qname in cls.mro(self.classes):
            ancestor = self.classes.get(class_qname)
            if ancestor and ancestor.destructor is not None:
                return ancestor.destructor
        return None

    def _register_object_command(self, interp: TclInterp, obj: TclOOObject) -> None:
        """Register the object as a command that dispatches method calls."""

        def _obj_dispatch(interp: TclInterp, args: list[str]) -> TclResult:

            if not args:
                raise TclError(f'wrong # args: should be "{obj.name} method ?arg ...?"')
            method_name = args[0]
            method_args = args[1:]

            if method_name == "destroy":
                return self._destroy_object(interp, obj)

            method, defining_class = self.resolve_method(obj, method_name)
            if method is None:
                raise TclError(
                    f'unknown method "{method_name}": must be '
                    + ", ".join(self._available_methods(obj))
                )
            return self._invoke_method(
                interp, obj, method, method_args, defining_class=defining_class
            )

        interp._runtime_commands[obj.name] = _obj_dispatch

    def resolve_method(
        self, obj: TclOOObject, method_name: str
    ) -> tuple[TclOOMethod | None, str | None]:
        """Resolve a method on an object using MRO.

        Returns ``(method, defining_class_qname)`` so that ``next`` can
        find the correct position in the MRO chain.
        """
        # Instance methods first
        if method_name in obj.instance_methods:
            return obj.instance_methods[method_name], obj.class_name

        # Walk MRO
        cls = self.classes.get(obj.class_name)
        if cls is None:
            return None, None

        for class_qname in cls.mro(self.classes):
            ancestor = self.classes.get(class_qname)
            if ancestor and method_name in ancestor.methods:
                return ancestor.methods[method_name], class_qname

        return None, None

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
        defining_class: str | None = None,
    ) -> TclResult:
        """Invoke a method on an object, setting up the instance context.

        *defining_class* is the qualified name of the class that owns
        *method*.  This is stored on the call frame so that ``next``
        can find the correct position in the MRO chain.
        """
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

        # Bind instance variables into the frame — collect from all
        # classes in the MRO that declare variables
        all_vars: list[str] = []
        if cls:
            for class_qname in cls.mro(self.classes):
                ancestor = self.classes.get(class_qname)
                if ancestor:
                    for v in ancestor.variables:
                        if v not in all_vars:
                            all_vars.append(v)
        for var_name in all_vars:
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

        # Store context for `self`, `my`, `next` resolution
        frame._oo_self = obj.name
        frame._oo_class = defining_class or obj.class_name
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
            for var_name in all_vars:
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
        if cls:
            dtor = self._resolve_destructor(cls)
            if dtor is not None:
                self._invoke_method(interp, obj, dtor, [], defining_class=obj.class_name)

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
        method, defining_class = self.resolve_method(obj, method_name)
        if method is None:
            raise TclError(f'unknown method "{method_name}"')
        return self._invoke_method(
            interp, obj, method, args[1:], defining_class=defining_class
        )

    def next_dispatch(self, interp: TclInterp, args: list[str]) -> TclResult:
        """Dispatch `next` — call next method in MRO chain.

        Uses ``_oo_class`` on the call frame to identify the *defining*
        class of the currently executing method, then walks the MRO to
        find the next class that implements the same method name.
        """
        frame = interp.current_frame
        obj_name = getattr(frame, "_oo_self", None)
        defining_class = getattr(frame, "_oo_class", None)
        method_name = getattr(frame, "_oo_method", None)
        if obj_name is None or defining_class is None or method_name is None:
            raise TclError('"next" may only be invoked from within a method')

        obj = self.objects.get(obj_name)
        if obj is None:
            raise TclError(f'object "{obj_name}" has been destroyed')

        # Get MRO from the object's instantiating class (not the defining class)
        inst_cls = self.classes.get(obj.class_name)
        if inst_cls is None:
            raise TclError(f'class "{obj.class_name}" not found')

        mro = inst_cls.mro(self.classes)

        # Find the defining class in the MRO, then look for the next
        # class that defines the same method name
        found_defining = False
        for class_qname in mro:
            if class_qname == defining_class:
                found_defining = True
                continue
            if found_defining:
                ancestor = self.classes.get(class_qname)
                if ancestor and method_name in ancestor.methods:
                    return self._invoke_method(
                        interp, obj, ancestor.methods[method_name], args,
                        defining_class=class_qname,
                    )

        # Tcl raises an error when next is called with no more implementations
        raise TclError("no next method implementation")

    def nextto_dispatch(
        self, interp: TclInterp, target_class: str, args: list[str]
    ) -> TclResult:
        """Dispatch `nextto className` — jump to a specific class in the MRO chain."""
        frame = interp.current_frame
        obj_name = getattr(frame, "_oo_self", None)
        method_name = getattr(frame, "_oo_method", None)
        if obj_name is None or method_name is None:
            raise TclError('"nextto" may only be invoked from within a method')

        obj = self.objects.get(obj_name)
        if obj is None:
            raise TclError(f'object "{obj_name}" has been destroyed')

        # Normalise target class name
        if not target_class.startswith("::"):
            if f"::{target_class}" in self.classes:
                target_class = f"::{target_class}"

        ancestor = self.classes.get(target_class)
        if ancestor is None:
            raise TclError(f'unknown class "{target_class}"')

        if method_name not in ancestor.methods:
            raise TclError(
                f'method "{method_name}" is not defined on class "{target_class}"'
            )

        return self._invoke_method(
            interp, obj, ancestor.methods[method_name], args,
            defining_class=target_class,
        )
