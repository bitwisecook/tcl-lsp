"""TclOO object model for the VM runtime.

Implements class and object storage matching Tcl 8.6/9.0's OO system.
Method dispatch uses the same two-pass DFS + late-placement algorithm
as ``tclOOCall.c``.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import TYPE_CHECKING

from core.analysis.mro import MROError, tcloo_linearise

from .types import TclError, TclResult, TclReturn, TclTailcall

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
    exported_methods: set[str] = field(default_factory=set)
    unexported_methods: set[str] = field(default_factory=set)
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
    creation_id: int = 0
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
        # Built-in unexported methods inherited by all objects
        for builtin_name in ("eval", "unknown", "variable", "varname"):
            root.methods[builtin_name] = TclOOMethod(
                name=builtin_name,
                params=[("args", None)] if builtin_name != "varname" else [("varName", None)],
                param_names=["args"] if builtin_name != "varname" else ["varName"],
                body=f"__builtin_{builtin_name}__",
                has_args=builtin_name != "varname",
                visibility="unexported",
            )
        # <cloned> is a private built-in (no-op by default, overridable)
        root.methods["<cloned>"] = TclOOMethod(
            name="<cloned>",
            params=[("originObject", None)],
            param_names=["originObject"],
            body="__builtin_cloned__",
            has_args=False,
            visibility="private",
        )
        self.classes["::oo::object"] = root
        # oo::object is an object whose class is oo::class
        self.objects["::oo::object"] = TclOOObject(
            name="::oo::object",
            class_name="::oo::class",
            namespace="::oo::object",
        )
        # TIP 524: definition namespace configuration for root classes
        root.instance_definition_namespace = "::oo::objdefine"

        # Register oo::class as the metaclass
        metaclass = TclOOClass(
            name="class",
            qualified_name="::oo::class",
            superclasses=["::oo::object"],
        )
        metaclass.definition_namespace = "::oo::define"
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

    def _is_metaclass(self, cls: TclOOClass) -> bool:
        """Check if *cls* is a metaclass (has ``::oo::class`` in its MRO)."""
        mro = cls.mro(self.classes)
        return "::oo::class" in mro

    def create_object(
        self,
        interp: TclInterp,
        class_name: str,
        obj_name: str | None = None,
        args: list[str] | None = None,
        display_name: str | None = None,
    ) -> str:
        """Create a new object instance of the given class.

        If *class_name* is a metaclass (has ``::oo::class`` in its MRO),
        the new object is itself a class and gets registered as such with
        a class command.

        *display_name* is the original user-provided name used in error
        messages (C Tcl uses the unqualified form).
        """
        cls = self.classes.get(class_name)
        if cls is None:
            raise TclError(f'unknown class "{class_name}"')

        err_name = display_name or obj_name

        self._next_obj_id += 1
        if obj_name is None:
            obj_name = f"::oo::Obj{self._next_obj_id}"
        else:
            # Check for duplicate object name (only named objects, not auto-generated)
            if obj_name in self.objects:
                raise TclError(
                    f'can\'t create object "{err_name}": command already exists with that name'
                )
            # Also check short name for qualified names
            if obj_name.startswith("::"):
                short = obj_name.rsplit("::", 1)[-1]
                if short and short in self.objects:
                    raise TclError(
                        f'can\'t create object "{err_name}": command already exists with that name'
                    )

        # If the creating class is a metaclass, the new object is itself a class.
        if self._is_metaclass(cls):
            from .commands.oo_cmds import _cmd_oo_class

            # Delegate to the class creation machinery so that the new
            # object gets a class record, a class command, etc.  We build
            # args for ``oo::class create name ?body?``.
            create_args = ["create", obj_name]
            if args:
                # The remaining args become the constructor args, but
                # for metaclass-created classes C Tcl passes them to the
                # constructor rather than treating them as a body.
                pass
            result = _cmd_oo_class(interp, create_args)
            new_name = result.value

            # Run constructor if provided via args (metaclass may have a
            # constructor that initialises the new class).
            new_cls = self.classes.get(new_name)
            new_obj = self.objects.get(new_name)
            if new_cls is not None and new_obj is not None:
                # Set the metaclass as the new class's class_name
                new_obj.class_name = class_name
                ctor, ctor_class = self._resolve_constructor(cls)
                if ctor is not None and args:
                    self._invoke_method(interp, new_obj, ctor, args, defining_class=ctor_class)
            return new_name

        # Each object gets a unique internal namespace like C Tcl's ::oo::ObjN
        ns = f"::oo::Obj{self._next_obj_id}"
        obj = TclOOObject(
            name=obj_name,
            class_name=class_name,
            namespace=ns,
            creation_id=self._next_obj_id,
        )
        self.objects[obj_name] = obj

        # Register the object command before running the constructor
        # so the object is usable from within the constructor body.
        self._register_object_command(interp, obj)

        # Run constructor — walk MRO to find the first constructor.
        # If the constructor throws, run destructor then clean up.
        ctor, ctor_class = self._resolve_constructor(cls)
        if ctor is not None:
            try:
                self._invoke_method(interp, obj, ctor, args or [], defining_class=ctor_class)
            except TclError as ctor_err:
                # Constructor failed — run destructor then destroy the object
                try:
                    self._destroy_object(interp, obj)
                except TclError:
                    pass  # ignore destructor errors during cleanup
                raise ctor_err

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
            # Use short (unqualified) name in error messages, like C Tcl
            short_name = obj.name
            if short_name.startswith("::"):
                short_name = short_name[2:]

            # Track invocation name for forward error rewriting.
            # If invoked as "::foo", use "::foo"; if as "foo", use "foo".
            invoked_name = getattr(interp, "_last_cmd_name", None) or short_name
            obj._invoked_name = invoked_name

            if not args:
                raise TclError(f'wrong # args: should be "{short_name} method ?arg ...?"')
            method_name = args[0]
            method_args = args[1:]

            method, defining_class = self.resolve_method(obj, method_name)

            # destroy is special: always available, but goes through filters
            is_destroy = method_name == "destroy"

            if method is None and not is_destroy:
                # Try unknown method handler (skip the built-in one on oo::object)
                unknown, unknown_class = self.resolve_method(obj, "unknown")
                if unknown is not None and not unknown.body.startswith("__builtin_"):
                    return self._invoke_method(
                        interp,
                        obj,
                        unknown,
                        [method_name] + method_args,
                        defining_class=unknown_class,
                    )
                avail = self._available_methods(obj)
                if avail:
                    raise TclError(
                        f'unknown method "{method_name}": must be '
                        + _format_method_list(avail)
                    )
                raise TclError(
                    f'object "{obj.name}" has no visible methods'
                )

            if not is_destroy:
                # Check visibility — unexported methods are not callable from
                # outside the object (only via ``my``).  Private methods
                # (TIP 500) are not callable via external dispatch at all.
                # Object-level export/unexport overrides class-level visibility,
                # and class-level export/unexport overrides method-level visibility.
                effective_vis = method.visibility
                # Check class-level export/unexport from MRO
                cls_check = self.classes.get(obj.class_name)
                if cls_check:
                    for class_qname in self._effective_mro(obj):
                        ancestor = self.classes.get(class_qname)
                        if ancestor:
                            if method_name in ancestor.exported_methods:
                                effective_vis = "public"
                                break
                            if method_name in ancestor.unexported_methods:
                                effective_vis = "unexported"
                                break
                # Object-level overrides class-level
                if method_name in obj.exported_methods:
                    effective_vis = "public"
                elif method_name in obj.unexported_methods:
                    effective_vis = "unexported"
                if effective_vis in ("unexported", "private"):
                    frame = interp.current_frame
                    caller_self = getattr(frame, "_oo_self", None)
                    if caller_self != obj.name:
                        avail = self._available_methods(obj)
                    if avail:
                        raise TclError(
                            f'unknown method "{method_name}": must be '
                            + _format_method_list(avail)
                        )
                    raise TclError(
                        f'object "{obj.name}" has no visible methods'
                    )

            # Check for filters — filters intercept all method calls
            # including destroy (but not the filter methods themselves).
            filters = self._collect_filters(obj)
            if filters and method_name not in [f for f in filters]:
                # Build filter chain: resolve each filter name to a method
                chain: list[tuple[TclOOMethod, str]] = []
                for fname in filters:
                    fmethod, fclass = self.resolve_method(obj, fname)
                    if fmethod is not None:
                        chain.append((fmethod, fclass or obj.class_name))
                if chain:
                    if is_destroy:
                        # For destroy, create a synthetic method that triggers destruction
                        destroy_method = TclOOMethod(
                            name="destroy",
                            params=[],
                            param_names=[],
                            body="__builtin_destroy__",
                            has_args=False,
                            visibility="public",
                        )
                        result = self._invoke_with_filters(
                            interp,
                            obj,
                            chain,
                            0,
                            method_name,
                            method_args,
                            destroy_method,
                            "::oo::object",
                        )
                        # After filters complete, actually destroy
                        if obj.name in self.objects:
                            self._destroy_object(interp, obj)
                        return result
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

            if is_destroy:
                return self._destroy_object(interp, obj)

            try:
                return self._invoke_method(
                    interp, obj, method, method_args, defining_class=defining_class
                )
            except TclError as e:
                # Add top-level "invoked from within" frame to errorInfo
                info = list(e.error_info) if e.error_info else [e.message]
                info.append(f'    invoked from within\n"{short_name} {method_name}"')
                raise TclError(e.message, error_info=info) from None

        interp._runtime_commands[obj.name] = _obj_dispatch
        # Also register the short name if different
        if obj.name.startswith("::"):
            short = obj.name.rsplit("::", 1)[-1]
            if short and short != obj.name:
                interp._runtime_commands[short] = _obj_dispatch

        # Register per-object `my` in the object's namespace, like C Tcl.
        # This allows `[info object namespace $obj]::my methodName` to work.
        oo_self = self

        def _ns_my(interp: TclInterp, args: list[str]) -> TclResult:
            if not args:
                raise TclError('wrong # args: should be "my method ?arg ...?"')
            method_name = args[0]
            # For class objects, route create/new/destroy to the class command
            if obj.name in oo_self.classes and method_name in ("create", "new", "destroy"):
                cls_cmd = interp._runtime_commands.get(obj.name)
                if cls_cmd is not None:
                    saved = obj.unexported_methods.copy()
                    obj.unexported_methods -= {"create", "new", "destroy"}
                    try:
                        return cls_cmd(interp, args)
                    finally:
                        obj.unexported_methods = saved
            method, defining_class = oo_self.resolve_method(obj, method_name)
            if method is None:
                avail = oo_self._available_methods(obj, include_all=True)
                if avail:
                    raise TclError(
                        f'unknown method "{method_name}": must be '
                        + _format_method_list(avail)
                    )
                raise TclError(f'unknown method "{method_name}"')
            # TIP 500: private methods are only accessible from the
            # defining class itself, matching the check in my_dispatch.
            if method.visibility == "private" and defining_class:
                frame = interp.current_frame
                caller_class = getattr(frame, "_oo_class", None)
                if caller_class != defining_class:
                    avail = oo_self._available_methods(obj, include_all=True)
                    if avail:
                        raise TclError(
                            f'unknown method "{method_name}": must be '
                            + _format_method_list(avail)
                        )
                    raise TclError(f'unknown method "{method_name}"')
            # Check for filters — my calls go through filters unless
            # we're already inside a filter chain for this object.
            in_filter = getattr(obj, "_in_filter", False)
            if not in_filter:
                filters = oo_self._collect_filters(obj)
                if filters and method_name not in filters:
                    chain: list[tuple[TclOOMethod, str]] = []
                    for fname in filters:
                        fmethod, fclass = oo_self.resolve_method(obj, fname)
                        if fmethod is not None:
                            chain.append((fmethod, fclass or obj.class_name))
                    if chain:
                        return oo_self._invoke_with_filters(
                            interp, obj, chain, 0, method_name, args[1:],
                            method, defining_class,
                        )
            return oo_self._invoke_method(interp, obj, method, args[1:], defining_class=defining_class)

        ns_my_name = f"{obj.namespace}::my"
        interp._runtime_commands[ns_my_name] = _ns_my

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

        # Walk effective MRO (includes instance mixins)
        for class_qname in self._effective_mro(obj):
            ancestor = self.classes.get(class_qname)
            if ancestor and method_name in ancestor.methods:
                return ancestor.methods[method_name], class_qname

        return None, None

    def _available_methods(
        self,
        obj: TclOOObject,
        *,
        include_all: bool = False,
        caller_class: str | None = None,
    ) -> list[str]:
        """Return list of available method names for error messages.

        When *include_all* is True, include unexported methods and built-in
        private methods.  User-defined private methods are only included if
        *caller_class* matches the class that defines them.
        """
        methods: set[str] = set()
        # Collect class-level export/unexport sets from MRO
        cls_exports: set[str] = set()
        cls_unexports: set[str] = set()
        cls = self.classes.get(obj.class_name)
        if cls:
            for class_qname in self._effective_mro(obj):
                ancestor = self.classes.get(class_qname)
                if ancestor:
                    for m in ancestor.exported_methods:
                        if m not in cls_unexports:
                            cls_exports.add(m)
                    for m in ancestor.unexported_methods:
                        if m not in cls_exports:
                            cls_unexports.add(m)

        def _should_include(md: TclOOMethod, defining: str | None = None) -> bool:
            """Check if a private method should be included based on caller context."""
            if md.visibility != "private":
                return True
            # Built-in private methods are always visible in include_all mode
            if md.body and md.body.startswith("__builtin_"):
                return True
            # User-defined private: only visible if caller_class matches
            return caller_class is not None and defining == caller_class

        # Instance methods — respect visibility and export overrides
        for m, md in obj.instance_methods.items():
            if include_all:
                if _should_include(md):
                    methods.add(m)
            elif m in obj.exported_methods:
                methods.add(m)
            elif m in obj.unexported_methods:
                continue
            elif m in cls_exports:
                methods.add(m)
            elif m in cls_unexports:
                continue
            elif md.visibility == "public":
                methods.add(m)
        if "destroy" not in obj.unexported_methods and "destroy" not in cls_unexports:
            methods.add("destroy")
        elif include_all:
            methods.add("destroy")
        if cls:
            for class_qname in self._effective_mro(obj):
                ancestor = self.classes.get(class_qname)
                if ancestor:
                    for m, md in ancestor.methods.items():
                        if include_all:
                            if _should_include(md, class_qname):
                                methods.add(m)
                        else:
                            # Object-level export overrides class visibility
                            if m in obj.exported_methods:
                                methods.add(m)
                            elif m in obj.unexported_methods:
                                continue
                            elif m in cls_exports:
                                methods.add(m)
                            elif m in cls_unexports:
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
        # Mark that this object is inside a filter chain so nested
        # my calls don't re-trigger filters (preventing infinite loops).
        was_in_filter = getattr(obj, "_in_filter", False)
        obj._in_filter = True
        try:
            return self._invoke_method(
                interp,
                obj,
                fmethod,
                method_args,
                defining_class=fclass,
                filter_chain=filter_chain,
                filter_index=filter_index,
                filter_target=(target_method, target_class),
                filter_method_name=method_name,
                filter_method_args=method_args,
            )
        finally:
            obj._in_filter = was_in_filter

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
        via_next: bool = False,
    ) -> TclResult:
        """Invoke a method on an object, setting up the instance context.

        *defining_class* is the qualified name of the class that owns
        *method*.  This is stored on the call frame so that ``next``
        can find the correct position in the MRO chain.
        """
        # Forward methods bypass the normal method invocation and directly
        # call the target command in the caller's scope, but with OO
        # context set so that `my` and `self` work.
        # Non-qualified forward targets are resolved in the object's
        # namespace so that per-object procs (created inside the
        # constructor) are found correctly.
        if method.forward_target is not None:
            cmd_parts = list(method.forward_target) + list(args)
            frame = interp.current_frame
            saved_self = getattr(frame, "_oo_self", None)
            saved_class = getattr(frame, "_oo_class", None)
            saved_method = getattr(frame, "_oo_method", None)
            frame._oo_self = obj.name
            frame._oo_class = defining_class
            frame._oo_method = method.name
            # Resolve non-qualified commands in the object's namespace
            from .scope import ensure_namespace
            obj_ns = ensure_namespace(interp.root_namespace, obj.namespace)
            saved_ns = interp.current_namespace
            interp.current_namespace = obj_ns
            try:
                return interp.invoke(cmd_parts[0], cmd_parts[1:])
            except TclError as e:
                # Rewrite "wrong # args" errors to show the forward
                # method name and object instead of the internal target.
                msg = str(e)
                if msg.startswith("wrong # args: should be "):
                    import re

                    m = re.match(
                        r'wrong # args: should be "([^"]*)"', msg
                    )
                    if m:
                        original = m.group(1).split()
                        prefix_count = len(method.forward_target)
                        remaining_params = original[prefix_count:]
                        # Use the command name as-invoked for the error message
                        display_name = getattr(obj, "_invoked_name", None) or obj.name
                        new_params = [display_name, method.name] + remaining_params
                        raise TclError(
                            f'wrong # args: should be "{" ".join(new_params)}"'
                        ) from None
                raise
            finally:
                interp.current_namespace = saved_ns
                frame._oo_self = saved_self
                frame._oo_class = saved_class
                frame._oo_method = saved_method

        from .scope import CallFrame, ensure_namespace

        cls = self.classes.get(obj.class_name)
        # Method bodies execute in the object's per-instance namespace
        # so that commands like ``proc`` create definitions scoped to
        # the object rather than polluting the global namespace.
        proc_ns = ensure_namespace(interp.root_namespace, obj.namespace)

        # When invoked via ``next``, the new frame shares the same
        # parent and level as the calling method frame so that
        # ``upvar 1`` reaches the original caller — matching real Tcl
        # semantics where chained methods are at the same call depth.
        if via_next:
            parent_frame = interp.current_frame.parent or interp.current_frame
            frame_level = interp.current_frame.level
        else:
            parent_frame = interp.current_frame
            frame_level = interp.current_frame.level + 1

        frame = CallFrame(
            level=frame_level,
            proc_name=f"{obj.name} {method.name}" if method.name else obj.name,
            parent=parent_frame,
            namespace=proc_ns,
            interp=interp,
            call_args=args,
        )

        # Bind instance variables into the frame — collect from all
        # classes in the MRO that declare variables.
        # We store the obj._vars reference so reads/writes go directly
        # to the shared instance variable store, avoiding stale copies
        # when nested method calls (via my/next) modify the same vars.
        all_vars: list[str] = []
        if cls:
            for class_qname in cls.mro(self.classes):
                ancestor = self.classes.get(class_qname)
                if ancestor:
                    for v in ancestor.variables:
                        if v not in all_vars:
                            all_vars.append(v)
        # Store reference to shared instance variable store.
        # The frame's get_var/set_var will proxy reads/writes for these
        # names directly to obj._vars, avoiding stale copies when nested
        # method calls (via my/next) modify the same variables.
        if all_vars:
            all_vars_set = set(all_vars)
            frame._oo_instance_vars = (obj._vars, all_vars_set)
            # Seed local scalars for info vars visibility
            for var_name in all_vars:
                if var_name in obj._vars:
                    frame._scalars[var_name] = obj._vars[var_name]

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

        # Execute method body in the object's namespace context
        old_frame = interp.current_frame
        old_ns = interp.current_namespace
        interp.current_frame = frame
        interp.current_namespace = proc_ns
        tailcall_pending: tuple[str, list[str]] | None = None
        try:
            # Handle built-in methods (eval, variable, varname, unknown)
            if method.body.startswith("__builtin_"):
                result = self._exec_builtin(interp, obj, method, args, frame)
            else:
                result = interp.eval(method.body)
        except TclReturn as ret:
            result = TclResult(value=ret.value)
        except TclTailcall as tc:
            # tailcall: unwind this method frame, execute in caller
            tailcall_pending = (tc.cmd, tc.args)
            result = TclResult()
        except TclError as e:
            # Augment errorInfo with OO method context frame, matching
            # C Tcl's format: (class "::X" method "foo" line N) or
            # (object "::x" method "foo" line N)
            dc = defining_class or obj.class_name
            is_class_method = dc and dc in self.classes
            if is_class_method:
                ctx = f'    (class "{dc}" method "{method.name}" line 1)'
            else:
                ctx = f'    (object "{obj.name}" method "{method.name}" line 1)'
            info = list(e.error_info) if e.error_info else [e.message]
            info.append(ctx)
            raise TclError(e.message, error_info=info) from None
        finally:
            # When _oo_instance_vars proxy is active, reads/writes go
            # directly through obj._vars so no write-back is needed.
            # Without the proxy (via_next calls that share parent frame),
            # we must sync frame locals back to obj._vars.
            if frame._oo_instance_vars is None:
                for var_name in all_vars:
                    val = frame._scalars.get(var_name)
                    if val is not None:
                        obj._vars[var_name] = val
            interp.current_frame = old_frame
            interp.current_namespace = old_ns
        if tailcall_pending is not None:
            return interp.invoke(tailcall_pending[0], tailcall_pending[1])
        return result

    def _exec_builtin(
        self,
        interp: TclInterp,
        obj: TclOOObject,
        method: TclOOMethod,
        args: list[str],
        frame: object,
    ) -> TclResult:
        """Execute a built-in oo::object method (eval, variable, varname, unknown)."""
        name = method.name
        if name == "eval":
            # Evaluate script in the object's namespace context
            from .machine import _list_escape
            from .scope import ensure_namespace

            script = " ".join(_list_escape(a) for a in args) if len(args) > 1 else (args[0] if args else "")
            # Set frame AND interp namespace to the object's namespace
            obj_ns = ensure_namespace(interp.root_namespace, obj.namespace)
            old_ns = frame.namespace
            old_interp_ns = interp.current_namespace
            frame.namespace = obj_ns
            interp.current_namespace = obj_ns
            try:
                result = interp.eval(script)
            finally:
                frame.namespace = old_ns
                interp.current_namespace = old_interp_ns
                # Write back namespace variables to obj._vars so they persist
                if hasattr(obj_ns, '_frame') and obj_ns._frame is not None:
                    ns_frame = obj_ns._frame
                    for vname in list(ns_frame._scalars.keys()):
                        obj._vars[vname] = ns_frame._scalars[vname]
            return result
        elif name == "variable":
            # Import object variables into the caller's local scope.
            # This links the caller's local variable to obj._vars so that
            # changes persist across method calls.
            # When called via "my variable", this runs in its own frame but
            # needs to affect the *caller's* frame (the method that called my).
            caller = frame.parent if frame.parent is not None else frame
            for var_name in args:
                if "::" in var_name:
                    raise TclError(
                        f'variable name "{var_name}" illegal: must not contain namespace separator'
                    )
                if "(" in var_name and var_name.endswith(")"):
                    raise TclError(
                        f'can\'t define "{var_name}": name refers to an element in an array'
                    )
                # Add to the instance variable proxy on the caller's frame,
                # so that reads/writes go directly through obj._vars.
                proxy = getattr(caller, '_oo_instance_vars', None)
                if proxy is not None:
                    vars_dict, vars_set = proxy
                    vars_set.add(var_name)
                else:
                    caller._oo_instance_vars = (obj._vars, {var_name})
                if var_name in obj._vars:
                    caller._scalars[var_name] = obj._vars[var_name]
                else:
                    obj._vars[var_name] = ""
                    caller._scalars[var_name] = ""
            return TclResult()
        elif name == "varname":
            # Return the fully-qualified variable name
            if not args:
                raise TclError('wrong # args: should be "varname varName"')
            var_name = args[0]
            return TclResult(value=f"{obj.namespace}::{var_name}")
        elif name == "unknown":
            # Default unknown handler — error with method list
            if args:
                method_name = args[0]
                avail = self._available_methods(obj)
                if avail:
                    raise TclError(
                        f'unknown method "{method_name}": must be '
                        + _format_method_list(avail)
                    )
                raise TclError(f'unknown method "{method_name}"')
            raise TclError("no method name given")
        elif name == "destroy":
            # Called from filter chain's next → actual destroy
            return TclResult()
        elif name == "<cloned>":
            # Default <cloned> handler — copy variables from origin object
            if args:
                from .scope import ensure_namespace
                origin_name = args[0]
                origin = self.objects.get(origin_name)
                if origin is None and not origin_name.startswith("::"):
                    origin = self.objects.get(f"::{origin_name}")
                if origin is not None:
                    obj._vars.update(origin._vars)
                    obj._arrays = {k: dict(v) for k, v in origin._arrays.items()}
                    # Also copy namespace-level variables (created by Tcl's
                    # `variable` command inside methods/constructors).
                    origin_ns = ensure_namespace(interp.root_namespace, origin.namespace)
                    obj_ns = ensure_namespace(interp.root_namespace, obj.namespace)
                    if hasattr(origin_ns, '_frame') and origin_ns._frame is not None:
                        if not hasattr(obj_ns, '_frame') or obj_ns._frame is None:
                            from .scope import CallFrame
                            obj_ns._frame = CallFrame(namespace=obj_ns)
                        for vn, vv in origin_ns._frame._scalars.items():
                            obj_ns._frame._scalars[vn] = vv
                        for vn, vv in origin_ns._frame._arrays.items():
                            obj_ns._frame._arrays[vn] = dict(vv)
            return TclResult()
        raise TclError(f'unknown builtin method "{name}"')

    def _destroy_object(self, interp: TclInterp, obj: TclOOObject) -> TclResult:
        """Destroy an object, running its destructor if present.

        If the object is a class, all its instances are destroyed first.
        """
        from .types import TclResult

        # Re-entrancy guard: prevents infinite recursion when ``[self]
        # destroy`` is called inside a destructor (Bug 2944404).  The
        # flag only suppresses the destructor call — cleanup always runs.
        already_destroying = getattr(obj, "_destroying", False)
        obj._destroying = True

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

        # Run destructor (unless we're re-entering from inside the destructor)
        cls = self.classes.get(obj.class_name)
        dtor_error: TclError | None = None
        if cls and not already_destroying:
            dtor, dtor_class = self._resolve_destructor(cls)
            if dtor is not None:
                try:
                    self._invoke_method(interp, obj, dtor, [], defining_class=dtor_class)
                except TclError as e:
                    dtor_error = e

        # Fire command traces before removing the command
        from .commands.trace_cmds import fire_command_traces

        fire_command_traces(interp, obj.name, obj.name, "")
        if obj.name.startswith("::"):
            short = obj.name.rsplit("::", 1)[-1]
            if short:
                fire_command_traces(interp, short, obj.name, "")

        # Always remove object command and storage, even if destructor errored
        interp._runtime_commands.pop(obj.name, None)
        if obj.name.startswith("::"):
            short = obj.name.rsplit("::", 1)[-1]
            if short:
                interp._runtime_commands.pop(short, None)
        self.objects.pop(obj.name, None)

        if dtor_error is not None:
            raise dtor_error
        return TclResult()

    def self_name(self, interp: TclInterp) -> str:
        """Return the name of the current object (for `self` command).

        Also works inside ``oo::define`` / ``oo::objdefine`` bodies
        (TIP #470) where the class/object being defined is returned.
        """
        frame = interp.current_frame
        obj_name = getattr(frame, "_oo_self", None)
        if obj_name is not None:
            return obj_name
        # TIP #470: inside oo::define / oo::objdefine body, self returns
        # the class or object being defined.
        defining_cls = getattr(interp, "_defining_class", None)
        if defining_cls is not None:
            return defining_cls.qualified_name
        defining_obj = getattr(interp, "_defining_object", None)
        if defining_obj is not None:
            return defining_obj.name
        raise TclError('"self" may only be invoked from within a method')

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

        # For class objects, `my create`/`my new`/`my destroy` dispatch
        # directly to the class command logic, bypassing export checks
        # (since `my` is an internal-access mechanism).
        if obj_name in self.classes and method_name in ("create", "new", "destroy"):
            cls_cmd = interp._runtime_commands.get(obj_name)
            if cls_cmd is not None:
                # Temporarily clear unexport marks so the class command
                # allows the built-in method through.
                saved = obj.unexported_methods.copy()
                obj.unexported_methods -= {"create", "new", "destroy"}
                try:
                    return cls_cmd(interp, args)
                finally:
                    obj.unexported_methods = saved

        method, defining_class = self.resolve_method(obj, method_name)
        my_caller_class = getattr(frame, "_oo_class", None)
        if method is None:
            avail = self._available_methods(obj, include_all=True, caller_class=my_caller_class)
            if avail:
                raise TclError(
                    f'unknown method "{method_name}": must be '
                    + _format_method_list(avail)
                )
            raise TclError(f'unknown method "{method_name}"')

        # TIP 500: private methods via ``my`` are only accessible from
        # within the defining class itself.
        if method.visibility == "private" and defining_class:
            if my_caller_class != defining_class:
                avail = self._available_methods(obj, include_all=True, caller_class=my_caller_class)
                if avail:
                    raise TclError(
                        f'unknown method "{method_name}": must be '
                        + _format_method_list(avail)
                    )
                raise TclError(f'unknown method "{method_name}"')

        # Check for filters — my calls go through filters unless
        # we're already inside a filter chain for this object.
        in_filter = getattr(obj, "_in_filter", False)
        if not in_filter:
            filters = self._collect_filters(obj)
            if filters and method_name not in filters:
                chain: list[tuple[TclOOMethod, str]] = []
                for fname in filters:
                    fmethod, fclass = self.resolve_method(obj, fname)
                    if fmethod is not None:
                        chain.append((fmethod, fclass or obj.class_name))
                if chain:
                    return self._invoke_with_filters(
                        interp, obj, chain, 0, method_name, args[1:],
                        method, defining_class,
                    )

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
                # with the original method args (no method name prefix).
                # Clear _in_filter so my calls from the target method
                # can re-trigger filters (matching C Tcl behaviour).
                target_method, target_class = filter_target
                was_in_filter = getattr(obj, "_in_filter", False)
                obj._in_filter = False
                try:
                    return self._invoke_method(
                        interp,
                        obj,
                        target_method,
                        filter_method_args,
                        defining_class=target_class,
                        via_next=True,
                    )
                finally:
                    obj._in_filter = was_in_filter

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
        def _next_invoke(class_qname: str, m: TclOOMethod) -> TclResult:
            """Invoke method and augment errorInfo with 'next' context."""
            try:
                return self._invoke_method(
                    interp, obj, m, args,
                    defining_class=class_qname, via_next=True,
                )
            except TclError as e:
                from .machine import _list_escape
                next_cmd = "next" + (" " + " ".join(_list_escape(a) for a in args) if args else " ")
                info = list(e.error_info) if e.error_info else [e.message]
                info.append(f'    invoked from within\n"{next_cmd}"')
                raise TclError(e.message, error_info=info) from None

        if defining_class.startswith("__instance__"):
            for class_qname in mro:
                ancestor = self.classes.get(class_qname)
                if ancestor:
                    m = _find_on_ancestor(ancestor)
                    if m is not None:
                        return _next_invoke(class_qname, m)
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
                            return _next_invoke(class_qname, m)

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
            raise TclError(f'"{target_class}" is not a class')

        # Validate that the target class is reachable from the current
        # position in the MRO.  The caller's class must come before the
        # target class in the chain.
        mro = self._effective_mro(obj)
        caller_class = getattr(frame, "_oo_class", None)
        if target_class not in mro:
            short_name = target_class.lstrip(":")
            raise TclError(
                f'method has no non-filter implementation by "{short_name}"'
            )
        if caller_class and caller_class in mro:
            caller_idx = mro.index(caller_class)
            target_idx = mro.index(target_class)
            if target_idx <= caller_idx:
                short_name = target_class.split("::")[-1] if "::" in target_class else target_class
                raise TclError(
                    f'method implementation by "{short_name}" not reachable from here'
                )

        # Handle constructors and destructors specially
        if method_name == "<constructor>":
            if ancestor.constructor is None:
                short_name = target_class.lstrip(":")
                raise TclError(
                    f'method has no non-filter implementation by "{short_name}"'
                )
            return self._invoke_method(
                interp, obj, ancestor.constructor, args,
                defining_class=target_class,
            )
        elif method_name == "<destructor>":
            if ancestor.destructor is None:
                short_name = target_class.lstrip(":")
                raise TclError(
                    f'method has no non-filter implementation by "{short_name}"'
                )
            return self._invoke_method(
                interp, obj, ancestor.destructor, args,
                defining_class=target_class,
            )

        if method_name not in ancestor.methods:
            short_name = target_class.lstrip(":")
            raise TclError(
                f'method has no non-filter implementation by "{short_name}"'
            )

        target_method = ancestor.methods[method_name]
        try:
            return self._invoke_method(
                interp,
                obj,
                target_method,
                args,
                defining_class=target_class,
            )
        except TclError as e:
            # Rewrite wrong # args to use "nextto ClassName" form
            if e.message.startswith("wrong # args"):
                tc_short = target_class.split("::")[-1] if "::" in target_class else target_class
                param_list = " ".join(p for p, _ in target_method.params)
                raise TclError(
                    f'wrong # args: should be "nextto {tc_short} {param_list}"'
                ) from None
            raise

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
                if m.body.startswith("__builtin_"):
                    continue
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
                if m.body.startswith("__builtin_"):
                    continue
                impl = "forward" if m.forward_target else "method"
                chain.append(("method", method_name, class_qname, impl))

        # Handle special methods: <constructor>, <destructor>
        if method_name in ("<constructor>", "<destructor>"):
            is_ctor = method_name == "<constructor>"
            for class_qname in effective_mro:
                ancestor = self.classes.get(class_qname)
                if ancestor:
                    target = ancestor.constructor if is_ctor else ancestor.destructor
                    if target is not None:
                        chain.append(("method", method_name, class_qname, "method"))
            return chain

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
                if m.body.startswith("__builtin_"):
                    continue
                impl = "forward" if m.forward_target else "method"
                chain.append(("method", method_name, class_qname, impl))

        # If method_name is "destroy", add as core method on oo::object
        if method_name == "destroy":
            if not any(ct == "method" for ct, _, _, _ in chain):
                chain.append(("method", "destroy", "::oo::object", '{core method: "destroy"}'))
            return chain

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
                    # Skip builtin methods — they're represented as core methods
                    if m.body.startswith("__builtin_"):
                        continue
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
        # Handle "destroy" as a core method
        if method_name == "destroy":
            # Walk MRO for user-defined destroy methods first
            found_user = False
            for class_qname in cls.mro(self.classes):
                ancestor = self.classes.get(class_qname)
                if ancestor and "destroy" in ancestor.methods:
                    m = ancestor.methods["destroy"]
                    if not m.body.startswith("__builtin_"):
                        impl = "forward" if m.forward_target else "method"
                        chain.append(("method", "destroy", class_qname, impl))
                        found_user = True
            chain.append(("method", "destroy", "::oo::object", '{core method: "destroy"}'))
            return chain

        found = False
        for class_qname in cls.mro(self.classes):
            ancestor = self.classes.get(class_qname)
            if ancestor and method_name in ancestor.methods:
                m = ancestor.methods[method_name]
                if m.body.startswith("__builtin_"):
                    continue
                impl = "forward" if m.forward_target else "method"
                chain.append(("method", method_name, class_qname, impl))
                found = True

        if not found:
            # User-defined unknown methods from MRO
            for class_qname in cls.mro(self.classes):
                ancestor = self.classes.get(class_qname)
                if ancestor and "unknown" in ancestor.methods:
                    m = ancestor.methods["unknown"]
                    # Skip builtin methods — they're represented as core methods
                    if m.body.startswith("__builtin_"):
                        continue
                    impl = "forward" if m.forward_target else "method"
                    chain.append(("unknown", "unknown", class_qname, impl))
            # Core unknown handler
            chain.append(("unknown", "unknown", "::oo::object", '{core method: "unknown"}'))

        return chain
