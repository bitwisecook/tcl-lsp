"""TclOO object model for the VM runtime.

Implements class and object storage matching Tcl 8.6/9.0's OO system.
Method dispatch uses the same two-pass DFS + late-placement algorithm
as ``tclOOCall.c``.
"""

from __future__ import annotations

import re
from dataclasses import dataclass, field
from typing import TYPE_CHECKING

from analyser.mro import MROError, tcloo_linearise

from .types import TclError, TclResult, TclReturn, TclTailcall

if TYPE_CHECKING:
    from compiler.codegen import FunctionAsm

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
    private_variables: list[str] = field(default_factory=list)
    filters: list[str] = field(default_factory=list)
    exported_methods: set[str] = field(default_factory=set)
    unexported_methods: set[str] = field(default_factory=set)
    definition_namespace: str = ""
    instance_definition_namespace: str = ""
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
    private_instance_variables: list[str] = field(default_factory=list)
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

        # Register oo::Slot — the base class for define/objdefine slot commands
        # (filter, mixin, superclass, variable).  Slot provides the standard
        # -set / -append / -clear / -prepend / -remove / -appendifnew protocol.
        slot_cls = TclOOClass(
            name="Slot",
            qualified_name="::oo::Slot",
            superclasses=["::oo::object"],
        )
        # Slot methods use __builtin_slot_*__ bodies handled by the OO runtime
        for slot_op in ("-set", "-append", "-clear", "-prepend", "-remove", "-appendifnew"):
            slot_cls.methods[slot_op] = TclOOMethod(
                name=slot_op,
                params=[("args", None)],
                param_names=["args"],
                body=f"__builtin_slot_{slot_op.lstrip('-')}__",
                has_args=True,
                visibility="public",
            )
        slot_cls.exported_methods = {
            "-set",
            "-append",
            "-clear",
            "-prepend",
            "-remove",
            "-appendifnew",
        }
        # Get and Set are unexported (overridden by subclasses)
        for vmethod in ("Get", "Set"):
            slot_cls.methods[vmethod] = TclOOMethod(
                name=vmethod,
                params=[("args", None)] if vmethod == "Set" else [],
                param_names=["args"] if vmethod == "Set" else [],
                body=f"__builtin_slot_{vmethod}__",
                has_args=vmethod == "Set",
                visibility="unexported",
            )
        slot_cls.unexported_methods = {"Get", "Set"}
        # --default-operation is unexported
        slot_cls.methods["--default-operation"] = TclOOMethod(
            name="--default-operation",
            params=[("args", None)],
            param_names=["args"],
            body="__builtin_slot_default_op__",
            has_args=True,
            visibility="unexported",
        )
        slot_cls.unexported_methods.add("--default-operation")
        # Override unknown to delegate all unknown methods to --default-operation
        # This means "slotObj x y" calls "--default-operation x y" (i.e., -append x y)
        slot_cls.methods["unknown"] = TclOOMethod(
            name="unknown",
            params=[("args", None)],
            param_names=["args"],
            body="__builtin_slot_unknown__",
            has_args=True,
            visibility="unexported",
        )
        # Unexport destroy so external calls go through unknown handler
        slot_cls.unexported_methods.add("destroy")
        self.classes["::oo::Slot"] = slot_cls
        self.objects["::oo::Slot"] = TclOOObject(
            name="::oo::Slot",
            class_name="::oo::class",
            namespace="::oo::Slot",
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
            self._next_obj_id += 1
            self.objects[cls.qualified_name] = TclOOObject(
                name=cls.qualified_name,
                class_name=metaclass,
                namespace=cls.qualified_name,
                creation_id=self._next_obj_id,
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
            # Also check if the command already exists (qualified or short)
            if obj_name in interp._runtime_commands:
                raise TclError(
                    f'can\'t create object "{err_name}": command already exists with that name'
                )

        # If the creating class is a metaclass, the new object is itself a class.
        if self._is_metaclass(cls):
            from .commands.oo_cmds import _cmd_oo_class, _parse_class_body

            # Delegate to the class creation machinery so that the new
            # object gets a class record, a class command, etc.
            create_args = ["create", obj_name]
            result = _cmd_oo_class(interp, create_args)
            new_name = result.value

            new_cls = self.classes.get(new_name)
            new_obj = self.objects.get(new_name)
            if new_cls is not None and new_obj is not None:
                # Set the metaclass as the new class's class_name
                new_obj.class_name = class_name
                ctor, ctor_class = self._resolve_constructor(cls)
                if ctor is not None and args:
                    # Metaclass has an explicit constructor — call it
                    self._invoke_method(interp, new_obj, ctor, args, defining_class=ctor_class)
                elif args:
                    # No explicit constructor — treat the first arg as a
                    # definition body (oo::class default behaviour).
                    _parse_class_body(interp, new_cls, args[0])
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

        # Create the actual Tcl namespace for this object so that
        # `namespace delete` and variable access work correctly.
        from .scope import ensure_namespace

        obj_ns = ensure_namespace(interp.root_namespace, ns)
        # Store a back-reference so namespace delete can trigger destruction
        obj_ns._oo_object_name = obj_name

        # Register the object command before running the constructor
        # so the object is usable from within the constructor body.
        self._register_object_command(interp, obj)

        # Run constructor — walk MRO to find the first constructor.
        # If the constructor throws, run destructor then clean up.
        ctor, ctor_class = self._resolve_constructor(cls)
        if ctor is not None:
            obj._in_constructor = True
            try:
                self._invoke_method(interp, obj, ctor, args or [], defining_class=ctor_class)
            except TclError as ctor_err:
                # Constructor failed — run destructor then destroy the object
                try:
                    self._destroy_object(interp, obj)
                except TclError:
                    pass  # ignore destructor errors during cleanup
                raise ctor_err
            finally:
                obj._in_constructor = False

            # If the object was destroyed during construction, raise error
            if obj.name not in self.objects:
                raise TclError("object deleted in constructor")

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

        In C Tcl, instance filters are inserted just before the object's
        direct class in the MRO (same position as instance methods).
        This means mixin class filters run first, then instance filters,
        then direct class filters.
        Duplicates are removed (first occurrence wins).
        """
        seen: set[str] = set()
        result: list[str] = []

        for class_qname in self._effective_mro(obj):
            # Insert instance filters right before the direct class
            if class_qname == obj.class_name:
                for f in obj.instance_filters:
                    if f not in seen:
                        seen.add(f)
                        result.append(f)
            ancestor = self.classes.get(class_qname)
            if ancestor:
                for f in ancestor.filters:
                    if f not in seen:
                        seen.add(f)
                        result.append(f)

        # Fallback: if direct class not in MRO, add instance filters at end
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
                # No method name — try the unknown handler first (oo-24.3)
                unknown, unknown_class = oo_self.resolve_method(obj, "unknown")
                if unknown is not None and (
                    not unknown.body.startswith("__builtin_")
                    or unknown.body == "__builtin_slot_unknown__"
                ):
                    return oo_self._invoke_method(
                        interp,
                        obj,
                        unknown,
                        [],
                        defining_class=unknown_class,
                    )
                raise TclError(f'wrong # args: should be "{short_name} method ?arg ...?"')
            method_name = args[0]
            method_args = args[1:]

            method, defining_class = oo_self.resolve_method(obj, method_name)

            # destroy is special: always available, but goes through filters
            # Exception: if destroy is explicitly unexported on any class in
            # the MRO, route through unknown (needed for oo::Slot)
            is_destroy = method_name == "destroy"
            if is_destroy and "destroy" not in obj.exported_methods:
                for mro_cls_name in oo_self._effective_mro(obj):
                    mro_cls = oo_self.classes.get(mro_cls_name)
                    if mro_cls is not None and "destroy" in mro_cls.unexported_methods:
                        is_destroy = False
                        method = None
                        break

            if method is None and not is_destroy:
                # Try unknown method handler (skip the built-in one on oo::object,
                # but allow the slot unknown handler)
                unknown, unknown_class = self.resolve_method(obj, "unknown")
                if unknown is not None and (
                    not unknown.body.startswith("__builtin_")
                    or unknown.body == "__builtin_slot_unknown__"
                ):
                    return self._invoke_method(
                        interp,
                        obj,
                        unknown,
                        [method_name] + method_args,
                        defining_class=unknown_class,
                    )
                # Include private methods in error message when caller
                # is in the same class (TIP 500 cross-object access)
                frame = interp.current_frame
                err_caller_class = getattr(frame, "_oo_class", None)
                avail = self._available_methods(obj, caller_class=err_caller_class)
                if avail:
                    raise TclError(
                        f'unknown method "{method_name}": must be ' + _format_method_list(avail)
                    )
                raise TclError(f'object "{obj.name}" has no visible methods')

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
                    allow = False
                    if caller_self == obj.name:
                        # Same object — always allow
                        allow = True
                    elif effective_vis == "private" and defining_class:
                        # TIP 500: private methods accessible from the
                        # same class (cross-object calls)
                        caller_class = getattr(frame, "_oo_class", None)
                        if caller_class and caller_class == defining_class:
                            allow = True
                    if not allow and effective_vis == "private":
                        # Private method not accessible — try to find a
                        # non-private implementation further down the MRO
                        alt_method, alt_class = oo_self.resolve_method(
                            obj,
                            method_name,
                            skip_private_from=defining_class,
                        )
                        if alt_method is not None:
                            method = alt_method
                            defining_class = alt_class
                            allow = True
                            # Re-check visibility of the alternative
                            effective_vis = alt_method.visibility
                            for class_qname in oo_self._effective_mro(obj):
                                ancestor = oo_self.classes.get(class_qname)
                                if ancestor:
                                    if method_name in ancestor.exported_methods:
                                        effective_vis = "public"
                                        break
                                    if method_name in ancestor.unexported_methods:
                                        effective_vis = "unexported"
                                        break
                            if method_name in obj.exported_methods:
                                effective_vis = "public"
                            elif method_name in obj.unexported_methods:
                                effective_vis = "unexported"
                            if effective_vis in ("unexported", "private"):
                                allow = False
                    if not allow:
                        # Try routing through unknown handler before erroring
                        # (oo::Slot's unknown catches unexported calls)
                        unknown, unknown_class = oo_self.resolve_method(obj, "unknown")
                        if unknown is not None and (
                            not unknown.body.startswith("__builtin_")
                            or unknown.body == "__builtin_slot_unknown__"
                        ):
                            return oo_self._invoke_method(
                                interp,
                                obj,
                                unknown,
                                [method_name] + method_args,
                                defining_class=unknown_class,
                            )
                        avail = oo_self._available_methods(obj)
                        if avail:
                            raise TclError(
                                f'unknown method "{method_name}": must be '
                                + _format_method_list(avail)
                            )
                        raise TclError(f'object "{obj.name}" has no visible methods')

            # Check for filters — filters intercept all method calls
            # including destroy (but not the filter methods themselves).
            filters = self._collect_filters(obj)
            if filters and method_name not in [f for f in filters]:
                # Build filter chain: for each filter name, collect ALL
                # implementations in MRO order (like C Tcl's call chain).
                chain: list[tuple[TclOOMethod, str]] = []
                for fname in filters:
                    for cqn, fm in self._resolve_all_methods(obj, fname):
                        chain.append((fm, cqn))
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
            # Check if object has been destroyed
            if obj.name not in oo_self.objects:
                raise TclError('invalid command name "my"')
            method_name = args[0]
            # For class objects, route create/new/destroy to the class command
            if obj.name in oo_self.classes and method_name in ("create", "new", "destroy"):
                cls_cmd = interp._runtime_commands.get(obj.name)
                if cls_cmd is not None:
                    # Temporarily allow class methods through by removing
                    # them from unexported_methods (only the ones that
                    # are currently unexported to be safe for re-entrancy).
                    removed = obj.unexported_methods & {"create", "new", "destroy"}
                    obj.unexported_methods -= removed
                    try:
                        return cls_cmd(interp, args)
                    finally:
                        obj.unexported_methods |= removed
            # Handle destroy via my — destroy is always available
            if method_name == "destroy":
                return oo_self._destroy_object(interp, obj)

            # `my varname` — return fully-qualified variable name
            if method_name == "varname":
                if len(args) != 2:
                    raise TclError('wrong # args: should be "my varname varName"')
                return oo_self._my_varname(interp, obj, args[1])

            # `my variable` — link variables into current frame
            if method_name == "variable":
                return oo_self._my_variable(interp, obj, args[1:])

            # TIP 500: When calling via 'my', check if the caller's
            # defining class has a private method with this name — that
            # takes priority over the normal MRO resolution.
            frame = interp.current_frame
            caller_class = getattr(frame, "_oo_class", None)
            method: TclOOMethod | None = None
            defining_class: str | None = None
            if caller_class and not caller_class.startswith("__instance__"):
                cc = oo_self.classes.get(caller_class)
                if cc and method_name in cc.methods:
                    m = cc.methods[method_name]
                    if m.visibility == "private":
                        method = m
                        defining_class = caller_class
            if method is None:
                method, defining_class = oo_self.resolve_method(obj, method_name)
            if method is None:
                avail = oo_self._available_methods(obj, include_all=True, caller_class=caller_class)
                if avail:
                    raise TclError(
                        f'unknown method "{method_name}": must be ' + _format_method_list(avail)
                    )
                raise TclError(f'unknown method "{method_name}"')
            # TIP 500: private methods are only accessible from the
            # defining class itself, matching the check in my_dispatch.
            # Instance-level private methods are only accessible from
            # other instance-level methods on the same object.
            if method.visibility == "private" and defining_class:
                is_instance_private = defining_class and defining_class.startswith("__instance__")
                caller_is_instance = caller_class and caller_class.startswith("__instance__")
                # Instance private → only accessible from instance methods
                # Class private → only accessible from same class
                allow = False
                if is_instance_private and caller_is_instance:
                    allow = True
                elif not is_instance_private and caller_class == defining_class:
                    allow = True
                if not allow:
                    # Try to find a non-private method with the same name
                    alt_method, alt_class = oo_self.resolve_method(
                        obj, method_name, skip_private_from=defining_class
                    )
                    if alt_method is not None:
                        method = alt_method
                        defining_class = alt_class
                    else:
                        avail = oo_self._available_methods(
                            obj, include_all=True, caller_class=caller_class
                        )
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
                        for cqn, fm in oo_self._resolve_all_methods(obj, fname):
                            chain.append((fm, cqn))
                    if chain:
                        return oo_self._invoke_with_filters(
                            interp,
                            obj,
                            chain,
                            0,
                            method_name,
                            args[1:],
                            method,
                            defining_class,
                        )
            return oo_self._invoke_method(
                interp, obj, method, args[1:], defining_class=defining_class
            )

        ns_my_name = f"{obj.namespace}::my"
        interp._runtime_commands[ns_my_name] = _ns_my

        # TIP 478: myclass command — dispatches to the object's class
        # myclass allows access to unexported methods (like my does)
        def _ns_myclass(interp: TclInterp, args: list[str]) -> TclResult:
            if not args:
                raise TclError('wrong # args: should be "myclass method ?arg ...?"')
            # Check if object has been destroyed
            if obj.name not in oo_self.objects:
                raise TclError('invalid command name "myclass"')
            # Look up the class command and dispatch, allowing unexported access
            cls_cmd = interp._runtime_commands.get(obj.class_name)
            if cls_cmd is not None:
                saved = getattr(interp, "_oo_internal_call", False)
                interp._oo_internal_call = True
                try:
                    return cls_cmd(interp, args)
                finally:
                    interp._oo_internal_call = saved
            raise TclError(f'invalid command name "{obj.class_name}"')

        ns_myclass_name = f"{obj.namespace}::myclass"
        interp._runtime_commands[ns_myclass_name] = _ns_myclass

    def resolve_method(
        self,
        obj: TclOOObject,
        method_name: str,
        *,
        skip_private_from: str | None = None,
    ) -> tuple[TclOOMethod | None, str | None]:
        """Resolve a method on an object using MRO.

        Returns ``(method, defining_class_qname)`` so that ``next`` can
        find the correct position in the MRO chain.

        If *skip_private_from* is set, skip private methods defined in
        that class (used to fall back to non-private alternatives).

        In C Tcl the call chain order is:
          mixin methods → instance methods → class hierarchy methods
        Instance methods are inserted just before the object's direct class.
        """
        mro = self._effective_mro(obj)
        inst_dc = f"__instance__{obj.name}"

        for class_qname in mro:
            # Insert instance method check right before the direct class
            if class_qname == obj.class_name and method_name in obj.instance_methods:
                m = obj.instance_methods[method_name]
                if not (
                    skip_private_from and inst_dc == skip_private_from and m.visibility == "private"
                ):
                    return m, inst_dc

            ancestor = self.classes.get(class_qname)
            if ancestor and method_name in ancestor.methods:
                m = ancestor.methods[method_name]
                if (
                    skip_private_from
                    and class_qname == skip_private_from
                    and m.visibility == "private"
                ):
                    continue
                return m, class_qname

        # Fallback: if direct class not in MRO, still check instance methods
        if method_name in obj.instance_methods:
            m = obj.instance_methods[method_name]
            if not (
                skip_private_from and inst_dc == skip_private_from and m.visibility == "private"
            ):
                return m, inst_dc

        return None, None

    def _resolve_all_methods(
        self,
        obj: TclOOObject,
        method_name: str,
    ) -> list[tuple[str, TclOOMethod]]:
        """Resolve ALL implementations of a method in MRO order.

        Returns a list of ``(defining_class, method)`` tuples representing
        every implementation in the call chain.  Instance methods are
        inserted before the direct class (matching C Tcl ordering).
        """
        result: list[tuple[str, TclOOMethod]] = []
        mro = self._effective_mro(obj)
        inst_dc = f"__instance__{obj.name}"

        for class_qname in mro:
            if class_qname == obj.class_name and method_name in obj.instance_methods:
                result.append((inst_dc, obj.instance_methods[method_name]))
            ancestor = self.classes.get(class_qname)
            if ancestor and method_name in ancestor.methods:
                result.append((class_qname, ancestor.methods[method_name]))

        # Fallback: if direct class not in MRO, still check instance methods
        if not any(dc == inst_dc for dc, _ in result) and method_name in obj.instance_methods:
            result.insert(0, (inst_dc, obj.instance_methods[method_name]))

        return result

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
                            elif (
                                md.visibility == "private"
                                and caller_class is not None
                                and class_qname == caller_class
                            ):
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
                    m = re.match(r'wrong # args: should be "([^"]*)"', msg)
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

        # Method bodies execute in the object's per-instance namespace
        # so that commands like ``proc`` create definitions scoped to
        # the object rather than polluting the global namespace.
        proc_ns = ensure_namespace(interp.root_namespace, obj.namespace)

        # When invoked via ``next``, chained methods share the same
        # ``level`` as the calling method frame so that ``upvar 1``
        # reaches the original caller.  The frame's ``parent`` is set
        # to the calling method's parent (for upvar resolution), but
        # ``_info_parent`` tracks the true call chain for ``info frame``.
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
        # For `next` chains, track the true call parent for `info frame` depth.
        # `parent` is used for upvar resolution (same as calling method's parent),
        # `_info_parent` tracks the actual call chain.
        if via_next:
            frame._info_parent = interp.current_frame

        # Bind instance variables into the frame.  In C Tcl, each method
        # only sees variables declared by its *defining* class (not the
        # full MRO).  Instance methods see only the per-object instance
        # variables from oo::objdefine.
        all_vars: list[str] = []
        private_var_map: dict[str, str] = {}
        def_cls = self.classes.get(defining_class) if defining_class else None
        if def_cls:
            # Class method — link only the defining class's declared variables
            for v in def_cls.variables:
                if v not in all_vars:
                    all_vars.append(v)
            # TIP 500: private variables get mangled names
            if def_cls.private_variables:
                cls_obj = self.objects.get(defining_class)
                cid = cls_obj.creation_id if cls_obj else 0
                for v in def_cls.private_variables:
                    if v not in all_vars:
                        all_vars.append(v)
                        private_var_map[v] = f"{cid} : {v}"
        else:
            # Instance method (or no defining class) — link per-object
            # instance variables only
            for v in obj.instance_variables:
                if v not in all_vars:
                    all_vars.append(v)
            # TIP 500: private instance variables
            if obj.private_instance_variables:
                cid = obj.creation_id
                for v in obj.private_instance_variables:
                    if v not in all_vars:
                        all_vars.append(v)
                        private_var_map[v] = f"{cid} : {v}"
        # Store reference to shared instance variable store.
        # The frame's get_var/set_var will proxy reads/writes for these
        # names directly to obj._vars, avoiding stale copies when nested
        # method calls (via my/next) modify the same variables.
        if all_vars:
            all_vars_set = set(all_vars)
            frame._oo_instance_vars = (obj._vars, all_vars_set)
            if private_var_map:
                frame._oo_private_var_map = private_var_map
            # Seed local scalars for info vars visibility
            for var_name in all_vars:
                storage = private_var_map.get(var_name, var_name)
                if storage in obj._vars:
                    frame._scalars[var_name] = obj._vars[storage]

        # Bind parameters — only strip the last "args" if has_args is True
        if method.has_args:
            all_params = method.params[:-1]  # last param is variadic "args"
        else:
            all_params = list(method.params)
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
            info = list(e.error_info) if e.error_info else [e.message]
            if method.body.startswith("__builtin_") and method.name == "eval":
                # Built-in eval: use (in "my eval" script line N) format
                # when invoked via my, or (in "::obj eval" script line N)
                # when invoked externally.
                caller_frame = frame.parent
                via_my = (
                    getattr(caller_frame, "_oo_self", None) == obj.name if caller_frame else False
                )
                if via_my:
                    eval_ctx = '    (in "my eval" script line 1)'
                    eval_inv = f'    invoked from within\n"my eval {{{args[0] if args else ""}}}"'
                else:
                    eval_ctx = f'    (in "{obj.name} eval" script line 1)'
                    eval_inv = (
                        f'    invoked from within\n"{obj.name} eval {{{args[0] if args else ""}}}"'
                    )
                info.append(eval_ctx)
                info.append(eval_inv)
            else:
                dc = defining_class or obj.class_name
                is_class_method = dc and dc in self.classes
                if is_class_method:
                    ctx = f'    (class "{dc}" method "{method.name}" line 1)'
                else:
                    ctx = f'    (object "{obj.name}" method "{method.name}" line 1)'
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
            # Uses namespace eval semantics so the namespace's variable
            # table is accessible (like C Tcl).
            from .machine import _list_escape
            from .scope import ensure_namespace

            script = (
                " ".join(_list_escape(a) for a in args)
                if len(args) > 1
                else (args[0] if args else "")
            )
            obj_ns = ensure_namespace(interp.root_namespace, obj.namespace)
            ns_frame = obj_ns.get_frame(interp)

            # Sync obj._vars into the namespace frame so variables are
            # accessible during eval
            for k, v in obj._vars.items():
                ns_frame._scalars[k] = v

            # Push the namespace frame as current and evaluate.
            # Propagate OO context so self/my work inside eval.
            saved_frame = interp.current_frame
            saved_ns = interp.current_namespace
            saved_oo_self = getattr(ns_frame, "_oo_self", None)
            saved_oo_class = getattr(ns_frame, "_oo_class", None)
            saved_oo_method = getattr(ns_frame, "_oo_method", None)
            ns_frame._oo_self = obj.name
            ns_frame._oo_class = getattr(frame, "_oo_class", None)
            ns_frame._oo_method = getattr(frame, "_oo_method", None)
            interp.current_frame = ns_frame
            interp.current_namespace = obj_ns
            try:
                result = interp.eval(script)
            finally:
                interp.current_frame = saved_frame
                interp.current_namespace = saved_ns
                ns_frame._oo_self = saved_oo_self
                ns_frame._oo_class = saved_oo_class
                ns_frame._oo_method = saved_oo_method
                # Write back namespace variables to obj._vars
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
                proxy = getattr(caller, "_oo_instance_vars", None)
                if proxy is not None:
                    vars_dict, vars_set = proxy
                    vars_set.add(var_name)
                else:
                    caller._oo_instance_vars = (obj._vars, {var_name})
                if var_name in obj._vars:
                    caller._scalars[var_name] = obj._vars[var_name]
            return TclResult()
        elif name == "varname":
            # Return the fully-qualified variable name.
            # Ensure the namespace frame actually has the variable so
            # external `set ::oo::ObjN::varName` can read/write it.
            if not args:
                raise TclError('wrong # args: should be "varname varName"')
            var_name = args[0]
            # Check if the variable has an upvar alias in the caller's
            # frame or the object's namespace frame (Bug 2da1cb0c80)
            from .scope import ensure_namespace

            caller = interp.current_frame
            # First check caller's frame (for `my variable` context)
            check_frame = None
            if var_name in caller._aliases:
                check_frame = caller
            else:
                obj_ns_check = ensure_namespace(interp.root_namespace, obj.namespace)
                ns_frame_check = obj_ns_check.get_frame(interp)
                if var_name in ns_frame_check._aliases:
                    check_frame = ns_frame_check
            if check_frame is not None and var_name in check_frame._aliases:
                target_frame, target_name = check_frame._aliases[var_name]
                # Walk to find the fully-qualified target name
                seen: set[int] = set()
                while target_name in target_frame._aliases:
                    key = id(target_frame) ^ hash(target_name)
                    if key in seen:
                        break
                    seen.add(key)
                    target_frame, target_name = target_frame._aliases[target_name]
                # If the target is in a namespace frame, return qualified
                ns_ref = getattr(target_frame, "namespace", None)
                if ns_ref is not None and hasattr(ns_ref, "qualname"):
                    qn_ref = ns_ref.qualname
                    return TclResult(
                        value=f"{qn_ref}::{target_name}" if qn_ref != "::" else f"::{target_name}"
                    )
                # For global vars, return ::name
                if target_frame.parent is None:
                    return TclResult(value=f"::{target_name}")
            # Ensure variable is accessible in the object's namespace frame
            from .scope import ensure_namespace

            obj_ns = ensure_namespace(interp.root_namespace, obj.namespace)
            ns_frame = obj_ns.get_frame(interp)
            # Sync: if the variable exists in obj._vars, make it visible
            # in the namespace frame
            if var_name in obj._vars:
                ns_frame._scalars[var_name] = obj._vars[var_name]
            # Set up a proxy so future reads/writes go through obj._vars
            existing = getattr(ns_frame, "_oo_instance_vars", None)
            if existing is not None:
                existing[1].add(var_name)
            else:
                ns_frame._oo_instance_vars = (obj._vars, {var_name})
            return TclResult(value=f"{obj.namespace}::{var_name}")
        elif name == "unknown" and method.body == "__builtin_slot_unknown__":
            # oo::Slot unknown handler: dispatch all unknown methods to
            # --default-operation (which defaults to -append).
            # Methods starting with "-" are slot operations and should
            # NOT be caught by the unknown handler — they should error.
            called_method = args[0] if args else ""
            if called_method.startswith("-"):
                # Fall through to the default unknown handler for - methods
                avail = self._available_methods(obj)
                if avail:
                    raise TclError(
                        f'unknown method "{called_method}": must be ' + _format_method_list(avail)
                    )
                raise TclError(f'unknown method "{called_method}"')
            all_args = list(args)
            # Dispatch through normal method resolution to honor overrides
            # (e.g., oo::objdefine $s forward --default-operation my -set)
            defop_method = self._find_method(obj, "--default-operation")
            # Pop frame so the sub-call runs at the correct level
            saved_frame = interp.current_frame
            parent = getattr(frame, "parent", None)
            if parent is not None:
                interp.current_frame = parent
            try:
                return self._invoke_method(interp, obj, defop_method, all_args, defining_class=None)
            finally:
                interp.current_frame = saved_frame
        elif name == "unknown":
            # Default unknown handler — error with method list
            if args:
                method_name = args[0]
                avail = self._available_methods(obj)
                if avail:
                    raise TclError(
                        f'unknown method "{method_name}": must be ' + _format_method_list(avail)
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
                    if hasattr(origin_ns, "_frame") and origin_ns._frame is not None:
                        if not hasattr(obj_ns, "_frame") or obj_ns._frame is None:
                            from .scope import CallFrame

                            obj_ns._frame = CallFrame(namespace=obj_ns)
                        for vn, vv in origin_ns._frame._scalars.items():
                            obj_ns._frame._scalars[vn] = vv
                        for vn, vv in origin_ns._frame._arrays.items():
                            obj_ns._frame._arrays[vn] = dict(vv)
            return TclResult()
        # oo::Slot builtin operations
        if name in (
            "-set",
            "-append",
            "-clear",
            "-prepend",
            "-remove",
            "-appendifnew",
            "--default-operation",
            "Get",
            "Set",
        ):
            return self._exec_slot_builtin(interp, obj, name, args, frame)

        # Define-namespace slot builtins (Get/Set/Resolve for filter/mixin/etc.)
        if method.body.startswith("__builtin_define_slot_"):
            return self._exec_define_slot_builtin(interp, obj, method, args, frame)

        raise TclError(f'unknown builtin method "{name}"')

    def _exec_slot_builtin(
        self,
        interp: TclInterp,
        obj: TclOOObject,
        op: str,
        args: list[str],
        frame: object,
    ) -> TclResult:
        """Execute a built-in oo::Slot method.

        In C Tcl, slot builtins are C code that calls the Tcl-level
        Get/Set/Resolve methods.  The C code itself doesn't add a Tcl
        frame, so the virtual methods run one level above the caller.
        We replicate this by temporarily popping back to the parent
        frame before dispatching sub-method calls.
        """
        from .machine import _list_escape, _split_list

        if op == "Get":
            return TclResult(value="")
        elif op == "Set":
            return TclResult()
        elif op == "--default-operation":
            return self._exec_slot_builtin(interp, obj, "-append", args, frame)

        # For the actual slot operations, temporarily restore the parent
        # frame so sub-method calls (Get/Set/Resolve) execute at the
        # same level as the slot command invocation.
        saved_frame = interp.current_frame
        parent = getattr(frame, "parent", None)
        if parent is not None:
            interp.current_frame = parent

        def _call(method_name: str, margs: list[str]) -> TclResult:
            return self._invoke_method(
                interp,
                obj,
                self._find_method(obj, method_name),
                margs,
                defining_class=None,
            )

        def _resolve_args(raw_args: list[str]) -> list[str]:
            resolved = []
            for a in raw_args:
                try:
                    r = _call("Resolve", [a])
                    resolved.append(r.value)
                except TclError:
                    resolved.append(a)
            return resolved

        def _to_list(items: list[str]) -> str:
            return " ".join(_list_escape(r) for r in items)

        try:
            if op == "-clear":
                _call("Set", [""])
            elif op == "-set":
                resolved = _resolve_args(args)
                _call("Set", [_to_list(resolved)])
            elif op == "-append":
                resolved = _resolve_args(args)
                current = _call("Get", [])
                current_list = _split_list(current.value) if current.value else []
                _call("Set", [_to_list(current_list + resolved)])
            elif op == "-prepend":
                resolved = _resolve_args(args)
                current = _call("Get", [])
                current_list = _split_list(current.value) if current.value else []
                _call("Set", [_to_list(resolved + current_list)])
            elif op == "-remove":
                resolved = _resolve_args(args)
                current = _call("Get", [])
                current_list = _split_list(current.value) if current.value else []
                for r in resolved:
                    if r in current_list:
                        current_list.remove(r)
                _call("Set", [_to_list(current_list)])
            elif op == "-appendifnew":
                resolved = _resolve_args(args)
                current = _call("Get", [])
                current_list = _split_list(current.value) if current.value else []
                for r in resolved:
                    if r not in current_list:
                        current_list.append(r)
                _call("Set", [_to_list(current_list)])
        finally:
            interp.current_frame = saved_frame
        return TclResult()

    def _exec_define_slot_builtin(
        self,
        interp: TclInterp,
        obj: TclOOObject,
        method: TclOOMethod,
        args: list[str],
        frame: object,
    ) -> TclResult:
        """Execute a define-namespace slot builtin (Get/Set/Resolve for filter etc.)."""
        from .machine import _list_escape, _split_list

        body = method.body
        # Extract the operation and slot name from the body string
        # Format: __builtin_define_slot_{Op}_{slot_name}__
        # e.g.: __builtin_define_slot_Get_::oo::define::filter__
        inner = body[len("__builtin_define_slot_") : -2]  # strip prefix and trailing __
        # Split at first underscore after Op name
        for op_name in ("Get", "Set", "Resolve", "default"):
            if inner.startswith(op_name + "_"):
                slot_name = inner[len(op_name) + 1 :]
                break
        else:
            raise TclError(f"unknown define slot builtin: {body}")

        # Determine if this is a define or objdefine slot
        is_objdefine = "::oo::objdefine::" in slot_name
        # Extract the property name (filter, mixin, superclass, variable)
        prop = slot_name.rsplit("::", 1)[-1]

        # Get the target class/object
        if is_objdefine:
            target_obj = getattr(interp, "_defining_object", None)
            if target_obj is None:
                raise TclError("this command may only be used within objdefine")
            if op_name == "Get":
                if prop == "filter":
                    return TclResult(
                        value=" ".join(_list_escape(f) for f in target_obj.instance_filters)
                    )
                elif prop == "mixin":
                    return TclResult(
                        value=" ".join(_list_escape(m) for m in target_obj.instance_mixins)
                    )
                elif prop == "variable":
                    return TclResult(
                        value=" ".join(_list_escape(v) for v in target_obj.instance_variables)
                    )
            elif op_name == "Set":
                lst = _split_list(args[0]) if args and args[0] else []
                if prop == "filter":
                    target_obj.instance_filters = lst
                elif prop == "mixin":
                    target_obj.instance_mixins = lst
                    self.invalidate_all_mro()
                elif prop == "variable":
                    target_obj.instance_variables = lst
                return TclResult()
            elif op_name == "Resolve":
                if prop in ("mixin",):
                    name = args[0] if args else ""
                    qn = name if name.startswith("::") else f"::{name}"
                    if qn in self.classes:
                        return TclResult(value=qn)
                    if name in self.classes:
                        return TclResult(value=name)
                    # Try current namespace
                    ns = interp.current_namespace.qualname
                    ns_qn = f"{ns}::{name}" if ns != "::" else f"::{name}"
                    if ns_qn in self.classes:
                        return TclResult(value=ns_qn)
                return TclResult(value=args[0] if args else "")
            elif op_name == "default":
                return self._exec_slot_builtin(interp, obj, "-append", args, frame)
        else:
            target_cls = getattr(interp, "_defining_class", None)
            if target_cls is None:
                raise TclError("this command may only be used within oo::define")
            if op_name == "Get":
                if prop == "filter":
                    return TclResult(value=" ".join(_list_escape(f) for f in target_cls.filters))
                elif prop == "mixin":
                    return TclResult(value=" ".join(_list_escape(m) for m in target_cls.mixins))
                elif prop == "superclass":
                    supers = target_cls.superclasses
                    # Normalize to qualified names
                    normalized = []
                    for s in supers:
                        qn = s if s.startswith("::") else f"::{s}"
                        if qn in self.classes:
                            normalized.append(qn)
                        else:
                            normalized.append(s)
                    return TclResult(value=" ".join(_list_escape(s) for s in normalized))
                elif prop == "variable":
                    return TclResult(value=" ".join(_list_escape(v) for v in target_cls.variables))
            elif op_name == "Set":
                lst = _split_list(args[0]) if args and args[0] else []
                if prop == "filter":
                    target_cls.filters = lst
                elif prop == "mixin":
                    target_cls.mixins = lst
                    self.invalidate_all_mro()
                elif prop == "superclass":
                    target_cls.superclasses = lst
                    self.invalidate_all_mro()
                elif prop == "variable":
                    target_cls.variables = lst
                return TclResult()
            elif op_name == "Resolve":
                if prop in ("mixin", "superclass"):
                    name = args[0] if args else ""
                    qn = name if name.startswith("::") else f"::{name}"
                    if qn in self.classes:
                        return TclResult(value=qn)
                    if name in self.classes:
                        return TclResult(value=name)
                    # Try current namespace
                    ns = interp.current_namespace.qualname
                    ns_qn = f"{ns}::{name}" if ns != "::" else f"::{name}"
                    if ns_qn in self.classes:
                        return TclResult(value=ns_qn)
                    if prop == "superclass":
                        raise TclError(f'unknown class "{name}"')
                    raise TclError(f'unknown class "{name}"')
                return TclResult(value=args[0] if args else "")
            elif op_name == "default":
                return self._exec_slot_builtin(interp, obj, "-append", args, frame)

        return TclResult()

    def _find_method(self, obj: TclOOObject, method_name: str) -> TclOOMethod:
        """Find a method on an object (checking instance methods then class methods)."""
        md = obj.instance_methods.get(method_name)
        if md is not None:
            return md
        for class_qn in self._effective_mro(obj):
            cls = self.classes.get(class_qn)
            if cls and method_name in cls.methods:
                return cls.methods[method_name]
        raise TclError(f'unknown method "{method_name}"')

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
            qn = cls_def.qualified_name
            short = qn.lstrip(":")

            # Destroy objects that have this class as an object mixin
            def _matches_class(mixin_name: str) -> bool:
                """Check if a mixin name matches the class being destroyed."""
                if mixin_name == qn or mixin_name == short:
                    return True
                if not mixin_name.startswith("::"):
                    return f"::{mixin_name}" == qn
                return False

            mixin_objs = [
                o
                for o in list(self.objects.values())
                if o.name != obj.name and any(_matches_class(m) for m in o.instance_mixins)
            ]
            for mo in mixin_objs:
                if mo.name in self.objects:
                    self._destroy_object(interp, mo)

            # Destroy classes that have this class as a class mixin
            mixin_classes = [
                c
                for c in list(self.classes.values())
                if c.qualified_name != qn and any(_matches_class(m) for m in c.mixins)
            ]
            for mc in mixin_classes:
                mc_obj = self.objects.get(mc.qualified_name)
                if mc_obj and mc_obj.name in self.objects:
                    self._destroy_object(interp, mc_obj)

            # Destroy subclasses (classes whose superclass is this class)
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

        # Destroy namespace-owned children (objects whose commands live in
        # this object's namespace).  This handles the nested ownership case
        # where [self class] create xyz creates inside the current namespace.
        # Check both name prefix and namespace prefix since object names
        # may be registered under either.
        prefixes = {obj.name + "::"}
        if obj.namespace != obj.name:
            prefixes.add(obj.namespace + "::")
        ns_children = [
            o
            for o in list(self.objects.values())
            if o.name in self.objects and any(o.name.startswith(p) for p in prefixes)
        ]
        for child in ns_children:
            if child.name in self.objects:
                self._destroy_object(interp, child)

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

        # Delete the object's namespace (like C Tcl).  Skip the default
        # ``::oo::Obj*`` namespace when it matches the object name to avoid
        # deleting namespaces that are still in use by subclasses.
        from .scope import resolve_namespace

        obj_ns = resolve_namespace(interp.root_namespace, obj.namespace)
        if obj_ns is not None and obj_ns.qualname != "::":
            # Only delete if no children remain (avoid wiping class namespaces
            # that contain subclass definitions).
            try:
                interp.eval(f"namespace delete {obj.namespace}")
            except TclError:
                pass  # best-effort cleanup

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

    def _my_varname(self, interp: TclInterp, obj: TclOOObject, var_name: str) -> TclResult:
        """Implement `my varname varName` — return fully-qualified variable name."""
        frame = interp.current_frame
        caller_class = getattr(frame, "_oo_class", None)
        caller_cls = self.classes.get(caller_class) if caller_class else None
        cls_obj = self.objects.get(caller_class) if caller_class else None
        if caller_cls and var_name in caller_cls.private_variables and cls_obj:
            mangled = f"{cls_obj.creation_id} : {var_name}"
        elif var_name in obj.private_instance_variables:
            mangled = f"{obj.creation_id} : {var_name}"
        else:
            mangled = var_name
        ns = obj.namespace
        fq = f"{ns}::{mangled}" if ns != "::" else f"::{mangled}"
        return TclResult(value=fq)

    def _my_variable(self, interp: TclInterp, obj: TclOOObject, var_names: list[str]) -> TclResult:
        """Implement `my variable ?name ...?` — link object vars into frame."""
        if not var_names:
            return TclResult(value="")
        for vn in var_names:
            if "(" in vn:
                raise TclError(f'can\'t define "{vn}": name refers to an element in an array')
            if "::" in vn:
                raise TclError(
                    f'variable name "{vn}" illegal: must not contain namespace separator'
                )
        frame = interp.current_frame
        caller_class = getattr(frame, "_oo_class", None)
        caller_cls = self.classes.get(caller_class) if caller_class else None
        cls_obj = self.objects.get(caller_class) if caller_class else None
        private_set = set(caller_cls.private_variables) if caller_cls else set()
        cid = cls_obj.creation_id if cls_obj else 0
        inst_private = set(obj.private_instance_variables)
        obj_cid = obj.creation_id
        iv = frame._oo_instance_vars
        if iv is None:
            iv = (obj._vars, set())
            frame._oo_instance_vars = iv
        pvm = frame._oo_private_var_map
        if pvm is None:
            pvm = {}
            frame._oo_private_var_map = pvm
        from .scope import ensure_namespace

        obj_ns = ensure_namespace(interp.root_namespace, obj.namespace)
        for vn in var_names:
            iv[1].add(vn)
            storage = vn
            if vn in private_set and cid:
                storage = f"{cid} : {vn}"
                pvm[vn] = storage
            elif vn in inst_private and obj_cid:
                storage = f"{obj_cid} : {vn}"
                pvm[vn] = storage
            # Sync existing obj._vars value to the namespace so that
            # `my varname` + `set` works with qualified variable names.
            if storage in obj._vars and obj_ns is not None:
                ns_frame = getattr(obj_ns, "_frame", None)
                if ns_frame is not None:
                    ns_frame._scalars[storage] = obj._vars[storage]
        return TclResult(value="")

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

        # `my varname` returns the fully-qualified variable name
        if method_name == "varname":
            if len(args) != 2:
                raise TclError('wrong # args: should be "my varname varName"')
            return self._my_varname(interp, obj, args[1])

        # `my variable` links variables into current frame
        if method_name == "variable":
            return self._my_variable(interp, obj, args[1:])

        # For class objects, `my create`/`my new`/`my destroy` dispatch
        # directly to the class command logic, bypassing export checks
        # (since `my` is an internal-access mechanism).
        if obj_name in self.classes and method_name in ("create", "new", "destroy"):
            cls_cmd = interp._runtime_commands.get(obj_name)
            if cls_cmd is not None:
                removed = obj.unexported_methods & {"create", "new", "destroy"}
                obj.unexported_methods -= removed
                try:
                    return cls_cmd(interp, args)
                finally:
                    obj.unexported_methods |= removed

        method, defining_class = self.resolve_method(obj, method_name)
        my_caller_class = getattr(frame, "_oo_class", None)
        if method is None:
            avail = self._available_methods(obj, include_all=True, caller_class=my_caller_class)
            if avail:
                raise TclError(
                    f'unknown method "{method_name}": must be ' + _format_method_list(avail)
                )
            raise TclError(f'unknown method "{method_name}"')

        # TIP 500: private methods via ``my`` are only accessible from
        # within the defining class itself.
        if method.visibility == "private" and defining_class:
            if my_caller_class != defining_class:
                avail = self._available_methods(obj, include_all=True, caller_class=my_caller_class)
                if avail:
                    raise TclError(
                        f'unknown method "{method_name}": must be ' + _format_method_list(avail)
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
                    for cqn, fm in self._resolve_all_methods(obj, fname):
                        chain.append((fm, cqn))
                if chain:
                    return self._invoke_with_filters(
                        interp,
                        obj,
                        chain,
                        0,
                        method_name,
                        args[1:],
                        method,
                        defining_class,
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

        mro = self._effective_mro(obj)

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
                    interp,
                    obj,
                    m,
                    args,
                    defining_class=class_qname,
                    via_next=True,
                )
            except TclError as e:
                from .machine import _list_escape

                next_cmd = "next" + (" " + " ".join(_list_escape(a) for a in args) if args else " ")
                info = list(e.error_info) if e.error_info else [e.message]
                info.append(f'    invoked from within\n"{next_cmd}"')
                raise TclError(e.message, error_info=info) from None

        if defining_class.startswith("__instance__"):
            # Instance method: next goes to the direct class in the MRO
            found_direct = False
            for class_qname in mro:
                if class_qname == obj.class_name:
                    found_direct = True
                if found_direct:
                    ancestor = self.classes.get(class_qname)
                    if ancestor:
                        m = _find_on_ancestor(ancestor)
                        if m is not None:
                            return _next_invoke(class_qname, m)
        else:
            # Find the defining class in the MRO, then look for the next
            # class that defines the same method/constructor/destructor.
            # Instance methods sit between mixins and the direct class.
            found_defining = False
            for class_qname in mro:
                if class_qname == defining_class:
                    found_defining = True
                    continue
                if found_defining:
                    # Insert instance method check before the direct class
                    if class_qname == obj.class_name and method_name in obj.instance_methods:
                        inst_m = obj.instance_methods[method_name]
                        inst_dc = f"__instance__{obj.name}"
                        return _next_invoke(inst_dc, inst_m)
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
            raise TclError(f'method has no non-filter implementation by "{short_name}"')
        if caller_class and caller_class in mro:
            caller_idx = mro.index(caller_class)
            target_idx = mro.index(target_class)
            if target_idx <= caller_idx:
                short_name = target_class.split("::")[-1] if "::" in target_class else target_class
                raise TclError(f'method implementation by "{short_name}" not reachable from here')

        # Handle constructors and destructors specially
        if method_name == "<constructor>":
            if ancestor.constructor is None:
                short_name = target_class.lstrip(":")
                raise TclError(f'method has no non-filter implementation by "{short_name}"')
            return self._invoke_method(
                interp,
                obj,
                ancestor.constructor,
                args,
                defining_class=target_class,
            )
        elif method_name == "<destructor>":
            if ancestor.destructor is None:
                short_name = target_class.lstrip(":")
                raise TclError(f'method has no non-filter implementation by "{short_name}"')
            return self._invoke_method(
                interp,
                obj,
                ancestor.destructor,
                args,
                defining_class=target_class,
            )

        if method_name not in ancestor.methods:
            short_name = target_class.lstrip(":")
            raise TclError(f'method has no non-filter implementation by "{short_name}"')

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
        self,
        obj: TclOOObject,
        method_name: str,
        private_classes: set[str] | None = None,
    ) -> list[tuple[str, str, str, str]]:
        """Build the call chain for a method on an object.

        Returns a list of ``(call_type, method_name, class_name, impl_type)``
        tuples matching what ``info object call`` returns.

        *private_classes* — set of class names whose private methods should
        be included in the chain.  When ``None``, exclude all private methods
        (external dispatch from outside the object).
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

        def _call_type(m: TclOOMethod) -> str:
            return "private" if m.visibility == "private" else "method"

        def _skip_private(m: TclOOMethod, class_qname: str) -> bool:
            """Return True if this private method should be excluded."""
            if m.visibility != "private":
                return False
            if private_classes is None:
                return True
            return class_qname not in private_classes

        # Instance mixin method entries
        for class_qname in effective_mro:
            if class_qname not in instance_mixin_classes:
                continue
            ancestor = self.classes.get(class_qname)
            if ancestor and method_name in ancestor.methods:
                m = ancestor.methods[method_name]
                if m.body.startswith("__builtin_"):
                    continue
                if _skip_private(m, class_qname):
                    continue
                impl = "forward" if m.forward_target else "method"
                chain.append((_call_type(m), method_name, class_qname, impl))

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
                if _skip_private(m, class_qname):
                    continue
                impl = "forward" if m.forward_target else "method"
                chain.append((_call_type(m), method_name, class_qname, impl))

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
            chain.append((_call_type(m), method_name, "object", impl))

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
                if _skip_private(m, class_qname):
                    continue
                impl = "forward" if m.forward_target else "method"
                chain.append((_call_type(m), method_name, class_qname, impl))

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
            for class_qname in cls.mro(self.classes):
                ancestor = self.classes.get(class_qname)
                if ancestor and "destroy" in ancestor.methods:
                    m = ancestor.methods["destroy"]
                    if not m.body.startswith("__builtin_"):
                        impl = "forward" if m.forward_target else "method"
                        chain.append(("method", "destroy", class_qname, impl))
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
