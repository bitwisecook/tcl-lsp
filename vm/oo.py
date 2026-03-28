"""TclOO object model for the VM runtime.

Implements class and object storage matching Tcl 8.6/9.0's OO system.
Method dispatch uses the same two-pass DFS + late-placement algorithm
as ``tclOOCall.c``.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import TYPE_CHECKING

from core.analysis.mro import MROError, tcloo_linearise

from .types import TclError, TclResult, TclReturn

if TYPE_CHECKING:
    from core.compiler.codegen import FunctionAsm

    from .interp import TclInterp


def _format_method_list(names: list[str]) -> str:
    """Format a list of method names for Tcl error messages.

    Tcl uses ``"a, b or c"`` (no Oxford comma).
    """
    if len(names) == 0:
        return ""
    if len(names) == 1:
        return names[0]
    if len(names) == 2:
        return f"{names[0]} or {names[1]}"
    return ", ".join(names[:-1]) + " or " + names[-1]


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
    forward_target: list[str] | None = None  # for forward methods: [cmdName, ?arg ...?]


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
            result = tcloo_linearise(self.qualified_name, supers_map, mixins_map=mixins_map)
        except MROError:
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
    instance_mixins: list[str] = field(default_factory=list)
    instance_filters: list[str] = field(default_factory=list)
    instance_variables: list[str] = field(default_factory=list)
    exported_methods: set[str] = field(default_factory=set)
    unexported_methods: set[str] = field(default_factory=set)
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
        # oo::object is an object whose class is oo::class
        self.objects["::oo::object"] = TclOOObject(
            name="::oo::object",
            class_name="::oo::class",
            namespace="::oo::object",
        )

        # Register oo::class as the metaclass
        metaclass = TclOOClass(
            name="class",
            qualified_name="::oo::class",
            superclasses=["::oo::object"],
        )
        self.classes["::oo::class"] = metaclass
        self.objects["::oo::class"] = TclOOObject(
            name="::oo::class",
            class_name="::oo::class",
            namespace="::oo::class",
        )

    def register_class(self, cls: TclOOClass) -> None:
        """Register a class in the runtime.

        Every class is also an object in TclOO, so we register a
        corresponding ``TclOOObject`` entry as well.  This allows
        ``info object`` introspection to work on class names.
        """
        self.classes[cls.qualified_name] = cls
        # Register the class as an object (classes are objects in TclOO)
        if cls.qualified_name not in self.objects:
            # Determine the class-of-a-class (metaclass)
            metaclass = "::oo::class"
            self.objects[cls.qualified_name] = TclOOObject(
                name=cls.qualified_name,
                class_name=metaclass,
                namespace=cls.qualified_name,
            )
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

        self._next_obj_id += 1
        if obj_name is None:
            obj_name = f"::oo::Obj{self._next_obj_id}"
        else:
            # Check for duplicate object name (only named objects, not auto-generated)
            if obj_name in self.objects:
                raise TclError(
                    f'can\'t create object "{obj_name}": command already exists with that name'
                )
            # Also check short name for qualified names
            if obj_name.startswith("::"):
                short = obj_name.rsplit("::", 1)[-1]
                if short and short in self.objects:
                    raise TclError(
                        f'can\'t create object "{obj_name}": command already exists with that name'
                    )

        # Each object gets a unique internal namespace like C Tcl's ::oo::ObjN
        ns = f"::oo::Obj{self._next_obj_id}"
        obj = TclOOObject(
            name=obj_name,
            class_name=class_name,
            namespace=ns,
        )
        self.objects[obj_name] = obj

        # Register the object command before running the constructor
        # so the object is usable from within the constructor body.
        self._register_object_command(interp, obj)

        # Run constructor — walk MRO to find the first constructor.
        # If the constructor throws, clean up the partially-constructed object.
        ctor, ctor_class = self._resolve_constructor(cls)
        if ctor is not None:
            try:
                self._invoke_method(interp, obj, ctor, args or [], defining_class=ctor_class)
            except TclError:
                # Constructor failed — destroy the partial object
                interp._runtime_commands.pop(obj_name, None)
                if obj_name.startswith("::"):
                    short = obj_name.rsplit("::", 1)[-1]
                    if short:
                        interp._runtime_commands.pop(short, None)
                self.objects.pop(obj_name, None)
                raise

        return obj_name

    def _resolve_constructor(self, cls: TclOOClass) -> tuple[TclOOMethod | None, str | None]:
        """Find the constructor by walking the MRO chain.

        Returns ``(constructor, defining_class_qname)`` so that ``next``
        inside the constructor resolves from the correct MRO position.
        The MRO includes the class itself as the first element.
        """
        for class_qname in cls.mro(self.classes):
            ancestor = self.classes.get(class_qname)
            if ancestor and ancestor.constructor is not None:
                return ancestor.constructor, class_qname
        return None, None

    def _resolve_destructor(self, cls: TclOOClass) -> tuple[TclOOMethod | None, str | None]:
        """Find the destructor by walking the MRO chain.

        Returns ``(destructor, defining_class_qname)`` so that ``next``
        inside the destructor resolves from the correct MRO position.
        The MRO includes the class itself as the first element.
        """
        for class_qname in cls.mro(self.classes):
            ancestor = self.classes.get(class_qname)
            if ancestor and ancestor.destructor is not None:
                return ancestor.destructor, class_qname
        return None, None

    def _effective_mro(self, obj: TclOOObject) -> list[str]:
        """Compute the full effective MRO for an object, including instance mixins.

        Uses ``tcloo_linearise`` with instance mixins prepended to the class's
        mixin list so the result matches C Tcl's call chain ordering.
        """
        cls = self.classes.get(obj.class_name)
        if cls is None:
            return []

        if not obj.instance_mixins:
            return cls.mro(self.classes)

        # Build supers/mixins maps for the lineariser
        def _normalise(name: str) -> str:
            if name.startswith("::"):
                return name
            if f"::{name}" in self.classes:
                return f"::{name}"
            return name

        supers_map: dict[str, list[str]] = {}
        mixins_map: dict[str, list[str]] = {}
        for qn, c in self.classes.items():
            supers_map[qn] = [_normalise(s) for s in c.superclasses]
            if c.mixins:
                mixins_map[qn] = [_normalise(m) for m in c.mixins]

        # Prepend instance mixins to the class's mixin list
        inst_mixins = [_normalise(m) for m in obj.instance_mixins]
        existing_mixins = mixins_map.get(cls.qualified_name, [])
        mixins_map[cls.qualified_name] = inst_mixins + existing_mixins

        try:
            return tcloo_linearise(cls.qualified_name, supers_map, mixins_map=mixins_map)
        except MROError:
            return cls.mro(self.classes)

    def _collect_filters(self, obj: TclOOObject) -> list[str]:
        """Collect all applicable filter names for an object.

        Order: class filters from effective MRO first, then instance filters.
        Duplicates are removed (first occurrence wins).
        """
        seen: set[str] = set()
        result: list[str] = []

        # Class filters from effective MRO (includes instance mixins)
        for class_qname in self._effective_mro(obj):
            ancestor = self.classes.get(class_qname)
            if ancestor:
                for f in ancestor.filters:
                    if f not in seen:
                        seen.add(f)
                        result.append(f)

        # Instance filters last
        for f in obj.instance_filters:
            if f not in seen:
                seen.add(f)
                result.append(f)

        return result

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
                # Try unknown method handler
                unknown, unknown_class = self.resolve_method(obj, "unknown")
                if unknown is not None:
                    return self._invoke_method(
                        interp,
                        obj,
                        unknown,
                        [method_name] + method_args,
                        defining_class=unknown_class,
                    )
                raise TclError(
                    f'unknown method "{method_name}": must be '
                    + _format_method_list(self._available_methods(obj))
                )

            # Check visibility — unexported methods are not callable from
            # outside the object (only via ``my``).  Private methods
            # (TIP 500) are not callable via external dispatch at all.
            # Object-level export/unexport overrides class-level visibility.
            effective_vis = method.visibility
            if method_name in obj.exported_methods:
                effective_vis = "public"
            elif method_name in obj.unexported_methods:
                effective_vis = "unexported"
            if effective_vis in ("unexported", "private"):
                frame = interp.current_frame
                caller_self = getattr(frame, "_oo_self", None)
                if caller_self != obj.name:
                    raise TclError(
                        f'unknown method "{method_name}": must be '
                        + _format_method_list(self._available_methods(obj))
                    )

            # Check for filters — filters intercept all method calls except
            # destroy and the filter methods themselves.
            filters = self._collect_filters(obj)
            if filters:
                # Build filter chain: resolve each filter name to a method
                chain: list[tuple[TclOOMethod, str]] = []
                for fname in filters:
                    fmethod, fclass = self.resolve_method(obj, fname)
                    if fmethod is not None:
                        chain.append((fmethod, fclass or obj.class_name))
                if chain:
                    return self._invoke_with_filters(
                        interp,
                        obj,
                        chain,
                        0,
                        method_name,
                        method_args,
                        method,
                        defining_class,
                    )

            return self._invoke_method(
                interp, obj, method, method_args, defining_class=defining_class
            )

        interp._runtime_commands[obj.name] = _obj_dispatch
        # Also register the short name if different
        if obj.name.startswith("::"):
            short = obj.name.rsplit("::", 1)[-1]
            if short and short != obj.name:
                interp._runtime_commands[short] = _obj_dispatch

    def resolve_method(
        self, obj: TclOOObject, method_name: str
    ) -> tuple[TclOOMethod | None, str | None]:
        """Resolve a method on an object using MRO.

        Returns ``(method, defining_class_qname)`` so that ``next`` can
        find the correct position in the MRO chain.
        """
        # Instance methods first — use the object name as a sentinel
        # "defining class" so that ``next`` starts at the beginning of
        # the real MRO (instance methods sit before the MRO).
        if method_name in obj.instance_methods:
            return obj.instance_methods[method_name], f"__instance__{obj.name}"

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
        methods: set[str] = set()
        # Instance methods — respect visibility and export overrides
        for m, md in obj.instance_methods.items():
            if m in obj.exported_methods:
                methods.add(m)
            elif m in obj.unexported_methods:
                continue
            elif md.visibility == "public":
                methods.add(m)
        methods.add("destroy")
        cls = self.classes.get(obj.class_name)
        if cls:
            for class_qname in self._effective_mro(obj):
                ancestor = self.classes.get(class_qname)
                if ancestor:
                    for m, md in ancestor.methods.items():
                        # Object-level export overrides class visibility
                        if m in obj.exported_methods:
                            methods.add(m)
                        elif m in obj.unexported_methods:
                            continue
                        elif md.visibility == "public":
                            methods.add(m)
        return sorted(methods)

    def _invoke_with_filters(
        self,
        interp: TclInterp,
        obj: TclOOObject,
        filter_chain: list[tuple[TclOOMethod, str]],
        filter_index: int,
        method_name: str,
        method_args: list[str],
        target_method: TclOOMethod,
        target_class: str | None,
    ) -> TclResult:
        """Invoke a filter in the chain, setting up next to proceed."""
        fmethod, fclass = filter_chain[filter_index]
        # Filters receive [methodName, ?arg ...?]
        filter_args = [method_name] + method_args
        return self._invoke_method(
            interp,
            obj,
            fmethod,
            filter_args,
            defining_class=fclass,
            filter_chain=filter_chain,
            filter_index=filter_index,
            filter_target=(target_method, target_class),
            filter_method_name=method_name,
            filter_method_args=method_args,
        )

    def _invoke_method(
        self,
        interp: TclInterp,
        obj: TclOOObject,
        method: TclOOMethod,
        args: list[str],
        defining_class: str | None = None,
        filter_chain: list[tuple[TclOOMethod, str]] | None = None,
        filter_index: int = 0,
        filter_target: tuple[TclOOMethod, str | None] | None = None,
        filter_method_name: str | None = None,
        filter_method_args: list[str] | None = None,
    ) -> TclResult:
        """Invoke a method on an object, setting up the instance context.

        *defining_class* is the qualified name of the class that owns
        *method*.  This is stored on the call frame so that ``next``
        can find the correct position in the MRO chain.
        """
        # Forward methods bypass the normal method invocation and directly
        # call the target command in the caller's scope.
        if method.forward_target is not None:
            cmd_parts = list(method.forward_target) + list(args)
            return interp.invoke(cmd_parts[0], cmd_parts[1:])

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
            from .machine import _list_escape

            remaining = args[len(all_params) :]
            frame.set_var("args", " ".join(_list_escape(a) for a in remaining))

        # Store context for `self`, `my`, `next` resolution
        frame._oo_self = obj.name
        frame._oo_class = defining_class or obj.class_name
        frame._oo_method = method.name

        # Store filter chain info so `next` can advance through filters
        if filter_chain is not None:
            frame._oo_filter_chain = filter_chain
            frame._oo_filter_index = filter_index
            frame._oo_filter_target = filter_target
            frame._oo_filter_method_name = filter_method_name
            frame._oo_filter_method_args = filter_method_args

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
                except (TclError, KeyError):
                    pass  # variable was not set in this method invocation
            interp.current_frame = old_frame
        return result

    def _destroy_object(self, interp: TclInterp, obj: TclOOObject) -> TclResult:
        """Destroy an object, running its destructor if present.

        If the object is a class, all its instances are destroyed first.
        """
        from .types import TclResult

        # If this object is a class, destroy subclasses and instances first
        cls_def = self.classes.get(obj.name)
        if cls_def is not None:
            # Destroy subclasses first (they are also classes whose superclass is this class)
            qn = cls_def.qualified_name
            short = qn.lstrip(":")
            subclasses_to_destroy = [
                c
                for c in list(self.classes.values())
                if c.qualified_name != qn
                and any(s == qn or s == short or f"::{s}" == qn for s in c.superclasses)
            ]
            for sub_cls in subclasses_to_destroy:
                sub_obj = self.objects.get(sub_cls.qualified_name)
                if sub_obj and sub_obj.name in self.objects:
                    self._destroy_object(interp, sub_obj)

            # Collect direct instances to destroy (snapshot to avoid mutation during iteration)
            instances_to_destroy = [
                o
                for o in list(self.objects.values())
                if o.class_name == cls_def.qualified_name and o.name != obj.name
            ]
            for inst in instances_to_destroy:
                if inst.name in self.objects:
                    self._destroy_object(interp, inst)
            # Remove class from registry
            self.classes.pop(cls_def.qualified_name, None)
            self.invalidate_all_mro()

        cls = self.classes.get(obj.class_name)
        if cls:
            dtor, dtor_class = self._resolve_destructor(cls)
            if dtor is not None:
                self._invoke_method(interp, obj, dtor, [], defining_class=dtor_class)

        # Remove object command and storage
        interp._runtime_commands.pop(obj.name, None)
        if obj.name.startswith("::"):
            short = obj.name.rsplit("::", 1)[-1]
            if short:
                interp._runtime_commands.pop(short, None)
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

        # TIP 500: private methods via ``my`` are only accessible from
        # within the defining class itself.
        if method.visibility == "private" and defining_class:
            caller_class = getattr(frame, "_oo_class", None)
            if caller_class != defining_class:
                raise TclError(f'unknown method "{method_name}"')

        return self._invoke_method(interp, obj, method, args[1:], defining_class=defining_class)

    def next_dispatch(self, interp: TclInterp, args: list[str]) -> TclResult:
        """Dispatch `next` — call next method in MRO chain.

        If the current frame is executing a filter, ``next`` advances
        through the filter chain and then to the target method.
        Otherwise it walks the MRO to find the next class that
        implements the same method name.
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

        # --- Filter chain support ---
        filter_chain = getattr(frame, "_oo_filter_chain", None)
        if filter_chain is not None:
            filter_index = getattr(frame, "_oo_filter_index", 0)
            filter_target = getattr(frame, "_oo_filter_target", None)
            filter_method_name = getattr(frame, "_oo_filter_method_name", None)
            filter_method_args = getattr(frame, "_oo_filter_method_args", [])

            next_idx = filter_index + 1

            # When next is called from a filter, the args passed to next
            # typically include [methodName, ...originalArgs] because the
            # filter was invoked with that shape.  We always use the
            # original method args for the target and for subsequent
            # filters, so the method name prefix is handled correctly.
            if next_idx < len(filter_chain):
                # Advance to next filter — pass original method name + args
                return self._invoke_with_filters(
                    interp,
                    obj,
                    filter_chain,
                    next_idx,
                    filter_method_name,
                    filter_method_args,
                    filter_target[0],
                    filter_target[1],
                )
            else:
                # All filters exhausted — invoke the target method
                # with the original method args (no method name prefix)
                target_method, target_class = filter_target
                return self._invoke_method(
                    interp,
                    obj,
                    target_method,
                    filter_method_args,
                    defining_class=target_class,
                )

        # --- Normal MRO walk ---
        inst_cls = self.classes.get(obj.class_name)
        if inst_cls is None:
            raise TclError(f'class "{obj.class_name}" not found')

        mro = inst_cls.mro(self.classes)

        # Helper to find a method/constructor/destructor on an ancestor class
        def _find_on_ancestor(ancestor: TclOOClass) -> TclOOMethod | None:
            if method_name == "<constructor>":
                return ancestor.constructor
            if method_name == "<destructor>":
                return ancestor.destructor
            return ancestor.methods.get(method_name)

        # If the current method is an instance method (sentinel marker),
        # start searching from the beginning of the MRO.
        if defining_class.startswith("__instance__"):
            for class_qname in mro:
                ancestor = self.classes.get(class_qname)
                if ancestor:
                    m = _find_on_ancestor(ancestor)
                    if m is not None:
                        return self._invoke_method(
                            interp,
                            obj,
                            m,
                            args,
                            defining_class=class_qname,
                        )
        else:
            # Find the defining class in the MRO, then look for the next
            # class that defines the same method/constructor/destructor
            found_defining = False
            for class_qname in mro:
                if class_qname == defining_class:
                    found_defining = True
                    continue
                if found_defining:
                    ancestor = self.classes.get(class_qname)
                    if ancestor:
                        m = _find_on_ancestor(ancestor)
                        if m is not None:
                            return self._invoke_method(
                                interp,
                                obj,
                                m,
                                args,
                                defining_class=class_qname,
                            )

        # In Tcl, next silently does nothing when there is no next
        # implementation for constructors/destructors.
        if method_name in ("<constructor>", "<destructor>"):
            return TclResult()

        raise TclError("no next method implementation")

    def nextto_dispatch(self, interp: TclInterp, target_class: str, args: list[str]) -> TclResult:
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
            raise TclError(f'method "{method_name}" is not defined on class "{target_class}"')

        return self._invoke_method(
            interp,
            obj,
            ancestor.methods[method_name],
            args,
            defining_class=target_class,
        )

    def build_object_call_chain(
        self, obj: TclOOObject, method_name: str
    ) -> list[tuple[str, str, str, str]]:
        """Build the call chain for a method on an object.

        Returns a list of ``(call_type, method_name, class_name, impl_type)``
        tuples matching what ``info object call`` returns.
        """
        chain: list[tuple[str, str, str, str]] = []
        effective_mro = self._effective_mro(obj)

        # Collect filter names (deduped)
        filter_names = self._collect_filters(obj)
        for fname in filter_names:
            if fname in obj.instance_methods:
                m = obj.instance_methods[fname]
                impl = "forward" if m.forward_target else "method"
                chain.append(("filter", fname, "object", impl))
            for class_qname in effective_mro:
                ancestor = self.classes.get(class_qname)
                if ancestor and fname in ancestor.methods:
                    m = ancestor.methods[fname]
                    impl = "forward" if m.forward_target else "method"
                    chain.append(("filter", fname, class_qname, impl))

        # Walk effective MRO for method entries
        # Instance mixin methods come first, then instance method, then class methods
        # Split: classes from instance mixins vs class MRO
        instance_mixin_classes: set[str] = set()
        for mixin_name in obj.instance_mixins:
            qn = mixin_name if mixin_name.startswith("::") else f"::{mixin_name}"
            mixin_cls = self.classes.get(qn)
            if mixin_cls:
                for cqn in mixin_cls.mro(self.classes):
                    instance_mixin_classes.add(cqn)

        # Instance mixin method entries
        for class_qname in effective_mro:
            if class_qname not in instance_mixin_classes:
                continue
            ancestor = self.classes.get(class_qname)
            if ancestor and method_name in ancestor.methods:
                m = ancestor.methods[method_name]
                impl = "forward" if m.forward_target else "method"
                chain.append(("method", method_name, class_qname, impl))

        # Class-level mixin method entries (from effective MRO, before instance method)
        cls = self.classes.get(obj.class_name)
        class_own_mro: list[str] = []
        class_mixin_classes: set[str] = set()
        if cls:
            class_own_mro = cls.mro(self.classes)
            # Identify which classes are from class-level mixins vs the plain hierarchy
            plain_hierarchy = self._plain_hierarchy(cls)
            class_mixin_classes = set(class_own_mro) - plain_hierarchy

        for class_qname in effective_mro:
            if class_qname in instance_mixin_classes:
                continue
            if class_qname not in class_mixin_classes:
                continue
            ancestor = self.classes.get(class_qname)
            if ancestor and method_name in ancestor.methods:
                m = ancestor.methods[method_name]
                impl = "forward" if m.forward_target else "method"
                chain.append(("method", method_name, class_qname, impl))

        # Instance method
        if method_name in obj.instance_methods:
            m = obj.instance_methods[method_name]
            impl = "forward" if m.forward_target else "method"
            chain.append(("method", method_name, "object", impl))

        # Class hierarchy methods (non-mixin)
        for class_qname in effective_mro:
            if class_qname in instance_mixin_classes:
                continue
            if class_qname in class_mixin_classes:
                continue
            ancestor = self.classes.get(class_qname)
            if ancestor and method_name in ancestor.methods:
                m = ancestor.methods[method_name]
                impl = "forward" if m.forward_target else "method"
                chain.append(("method", method_name, class_qname, impl))

        # If no method found, add unknown handlers
        if not any(ct == "method" for ct, _, _, _ in chain):
            if "unknown" in obj.instance_methods:
                m = obj.instance_methods["unknown"]
                impl = "forward" if m.forward_target else "method"
                chain.append(("unknown", "unknown", "object", impl))
            for class_qname in effective_mro:
                ancestor = self.classes.get(class_qname)
                if ancestor and "unknown" in ancestor.methods:
                    m = ancestor.methods["unknown"]
                    impl = "forward" if m.forward_target else "method"
                    chain.append(("unknown", "unknown", class_qname, impl))
            chain.append(("unknown", "unknown", "::oo::object", '{core method: "unknown"}'))

        return chain

    def _plain_hierarchy(self, cls: TclOOClass) -> set[str]:
        """Get the plain inheritance hierarchy (without mixins) for a class."""
        result: set[str] = set()
        visited: set[str] = set()
        stack = [cls.qualified_name]
        while stack:
            qn = stack.pop()
            if qn in visited:
                continue
            visited.add(qn)
            result.add(qn)
            c = self.classes.get(qn)
            if c:
                for s in c.superclasses:
                    sq = s if s.startswith("::") else f"::{s}"
                    stack.append(sq)
        return result

    def build_class_call_chain(
        self, class_name: str, method_name: str
    ) -> list[tuple[str, str, str, str]]:
        """Build the call chain for a method on a class (no instance methods).

        Returns a list of ``(call_type, method_name, class_name, impl_type)``
        tuples matching what ``info class call`` returns.
        """
        cls = self.classes.get(class_name)
        if cls is None:
            return []

        chain: list[tuple[str, str, str, str]] = []

        # Collect filter names (deduped, ordered by MRO)
        seen_filters: set[str] = set()
        filter_names: list[str] = []
        for class_qname in cls.mro(self.classes):
            ancestor = self.classes.get(class_qname)
            if ancestor:
                for fname in ancestor.filters:
                    if fname not in seen_filters:
                        seen_filters.add(fname)
                        filter_names.append(fname)

        # For each filter, list every class in MRO that defines the filter method
        for fname in filter_names:
            for class_qname in cls.mro(self.classes):
                ancestor = self.classes.get(class_qname)
                if ancestor and fname in ancestor.methods:
                    fm = ancestor.methods[fname]
                    impl = "forward" if fm.forward_target else "method"
                    chain.append(("filter", fname, class_qname, impl))

        # Walk MRO for the method
        found = False
        for class_qname in cls.mro(self.classes):
            ancestor = self.classes.get(class_qname)
            if ancestor and method_name in ancestor.methods:
                m = ancestor.methods[method_name]
                impl = "forward" if m.forward_target else "method"
                chain.append(("method", method_name, class_qname, impl))
                found = True

        if not found:
            # User-defined unknown methods from MRO
            for class_qname in cls.mro(self.classes):
                ancestor = self.classes.get(class_qname)
                if ancestor and "unknown" in ancestor.methods:
                    m = ancestor.methods["unknown"]
                    impl = "forward" if m.forward_target else "method"
                    chain.append(("unknown", "unknown", class_qname, impl))
            # Core unknown handler
            chain.append(("unknown", "unknown", "::oo::object", '{core method: "unknown"}'))

        return chain
