"""Central command registry for completion and hover metadata."""

from __future__ import annotations

import importlib
import threading
from collections.abc import Callable
from dataclasses import dataclass, field, replace
from typing import TYPE_CHECKING, NamedTuple

if TYPE_CHECKING:
    from compiler.side_effects import SideEffect

    from .models import CodegenHook, CommandHandler, LoweringHook, SubcommandHandler, WasmEmitHook

from dialects.stdlib import stdlib_command_specs
from dialects.tcl import tcl_command_specs
from dialects.tcllib import tcllib_command_specs

from .models import (
    ArgumentValueSpec,
    CommandLegality,
    CommandSpec,
    DialectStatus,
    EventCommandSet,
    OptionSpec,
    ValidationSpec,
)
from .taint_sink_info import _EMPTY_TAINT_SINK_INFO, TaintSinkInfo


class ResolvedTerminator(NamedTuple):
    """Resolved option-terminator info for a command invocation.

    Returned by ``CommandRegistry.resolve_option_terminator()`` after
    matching the invocation's subcommand against the registry.  Consumers
    use the fields directly — no need for the old ``OptionTerminatorSpec``.
    """

    scan_start: int
    """Arg index (0-based after command name) where option scanning begins."""
    subcommand: str | None
    """Matched subcommand word, or ``None`` for top-level profiles."""
    options_with_values: frozenset[str]
    """Option names that consume a value argument (derived from OptionSpec)."""
    warn_without_terminator: bool
    """Whether to warn even for non-dynamic positional values."""


_BOOLEAN_TRAITS: tuple[str, ...] = (
    "creates_dynamic_barrier",
    "has_loop_body",
    "never_inline_body",
    "pure",
    "cse_candidate",
    "unsafe",
    "diagram_action",
    "is_control_flow",
    "needs_start_cmd",
    "defines_procedure",
    # Analysis check dispatch traits.
    "evaluates_code",
    "performs_substitution",
    "opens_channel",
    "sources_file",
    "has_switch_body",
    "has_string_list_confusion_risk",
    "configures_channel",
    "has_interp_eval",
    "has_destructive_ops",
    "is_irules_event_handler",
    "is_unnormalized_http_getter",
    # Rendered-value and semantic traits.
    "returns_path",
    "is_unescape_command",
    "pure_evaluation",
    "destroys_variable",
    "is_language_keyword",
    "reads_variable_before_write",
    "has_boolean_condition",
    "produces_canonical_list",
    "is_side_switch",
    "irules_top_level_only",
    "is_oo_metaclass",
    "terminates_block",
    # Command-classification traits replacing consumer-local name lists.
    "byte_compiled",
    "not_proc_factory",
    "frameless_runtime",
)


def _core_command_specs() -> tuple[CommandSpec, ...]:
    """Return command specs needed by every dialect.

    Tcl core, stdlib (package-gated), and tcllib (namespace-scoped).
    Dialect-specific packs (Tk, iRules, iApps, EDA, Expect) are loaded
    on demand via :meth:`CommandRegistry.load_dialect_specs`.
    """
    return tcl_command_specs() + stdlib_command_specs() + tcllib_command_specs()


# Lazy dialect spec loaders.  Each key maps to a (module_name, func_name)
# pair resolved relative to this package via importlib.
_DIALECT_LOADER_SPECS: dict[str, tuple[str, str]] = {
    "tk": ("dialects.tk.specs", "tk_command_specs"),
    "f5-irules": ("dialects.f5.irules", "irules_command_specs"),
    "f5-iapps": ("dialects.f5.iapps", "iapps_command_specs"),
    "sdc-base": ("dialects.eda.sdc_base", "sdc_base_command_specs"),
    "synopsys-eda-tcl": ("dialects.eda.synopsys", "synopsys_command_specs"),
    "cadence-eda-tcl": ("dialects.eda.cadence", "cadence_command_specs"),
    "xilinx-eda-tcl": ("dialects.eda.xilinx", "xilinx_command_specs"),
    "intel-quartus-eda-tcl": ("dialects.eda.quartus", "quartus_command_specs"),
    "mentor-eda-tcl": ("dialects.eda.mentor", "mentor_command_specs"),
    "expect": ("dialects.expect", "expect_command_specs"),
}


def _load_dialect_pack(key: str) -> tuple[CommandSpec, ...]:
    """Import and call the spec factory for *key*."""
    mod_name, func_name = _DIALECT_LOADER_SPECS[key]
    mod = importlib.import_module(mod_name)
    return getattr(mod, func_name)()


# Dialect -> loader keys needed.  Core Tcl specs are always loaded.
_DIALECT_TO_LOADERS: dict[str, tuple[str, ...]] = {
    "tcl8.4": ("tk",),
    "tcl8.5": ("tk",),
    "tcl8.6": ("tk",),
    "tcl9.0": ("tk",),
    "f5-irules": ("f5-irules",),
    "f5-iapps": ("f5-iapps",),
    "f5-tmsh": ("f5-iapps",),
    "f5-bigip": ("f5-iapps",),
    "synopsys-eda-tcl": ("tk", "sdc-base", "synopsys-eda-tcl"),
    "cadence-eda-tcl": ("tk", "sdc-base", "cadence-eda-tcl"),
    "xilinx-eda-tcl": ("tk", "sdc-base", "xilinx-eda-tcl"),
    "intel-quartus-eda-tcl": ("tk", "sdc-base", "intel-quartus-eda-tcl"),
    "mentor-eda-tcl": ("tk", "sdc-base", "mentor-eda-tcl"),
    "expect": ("tk", "expect"),
}

# Callback set by runtime.py to invalidate derived caches after spec loading.
_on_specs_loaded: Callable[[list[str]], None] | None = None


@dataclass(slots=True)
class CommandRegistry:
    """Lookup facade over command specs."""

    specs_by_name: dict[str, tuple[CommandSpec, ...]]

    # Tcllib package index: package name -> frozenset of command names.
    _tcllib_packages: dict[str, frozenset[str]]
    # Reverse lookup: command name -> tcllib package name.
    _tcllib_command_to_package: dict[str, str]

    # General package index: command name -> required_package value.
    # Covers all package-gated commands (stdlib, tcllib, Tk).
    _command_to_required_package: dict[str, str]

    # VM handlers for commands without specs (e.g. tcl::mathfunc::*).
    _standalone_handlers: dict[str, "CommandHandler"]

    # Cached command name snapshots keyed by (dialect, active_packages).
    _command_names_cache: dict[
        tuple[str | None, frozenset[str] | None],
        tuple[str, ...],
    ] = field(default_factory=dict, init=False, repr=False)

    # Precomputed boolean trait indexes: trait name -> command names.
    _trait_indexes: dict[str, frozenset[str]] = field(default_factory=dict, init=False, repr=False)

    # Cached EventCommandSet keyed by (dialect, event).
    _event_command_cache: dict[
        tuple[str, str | None],
        EventCommandSet,
    ] = field(default_factory=dict, init=False, repr=False)

    # Cached CommandLegality keyed by dialect.
    _legality_cache: dict[str, CommandLegality] = field(
        default_factory=dict,
        init=False,
        repr=False,
    )

    # Cached filtered registry snapshots keyed by (dialect, active_packages).
    _filtered_cache: dict[
        tuple[str | None, frozenset[str] | None],
        "CommandRegistry",
    ] = field(default_factory=dict, init=False, repr=False)

    # Loader keys that have already been applied to this registry.
    _loaded_loaders: set[str] = field(default_factory=set, init=False, repr=False)

    # Serialises dialect loading so concurrent threads cannot double-merge.
    _load_lock: threading.Lock = field(default_factory=threading.Lock, init=False, repr=False)

    def __post_init__(self) -> None:
        self._trait_indexes = self._build_trait_indexes()

    @staticmethod
    def _normalise_package_set(
        active_packages: frozenset[str] | set[str] | None,
    ) -> frozenset[str] | None:
        if active_packages is None:
            return None
        if isinstance(active_packages, frozenset):
            return active_packages
        return frozenset(active_packages)

    def _build_trait_indexes(self) -> dict[str, frozenset[str]]:
        index: dict[str, set[str]] = {trait: set() for trait in _BOOLEAN_TRAITS}
        for name, specs in self.specs_by_name.items():
            for trait in _BOOLEAN_TRAITS:
                if any(getattr(spec, trait, False) for spec in specs):
                    index[trait].add(name)
        return {trait: frozenset(names) for trait, names in index.items()}

    def _trait_names(self, trait: str) -> frozenset[str]:
        names = self._trait_indexes.get(trait)
        if names is not None:
            return names
        return frozenset()

    def _ensure_dialect_loaded(self, dialect: str | None) -> None:
        """Auto-load specs for *dialect* if not already loaded."""
        if dialect is not None and dialect in _DIALECT_TO_LOADERS:
            self.load_dialect_specs(dialect)

    @classmethod
    def build_default(cls) -> "CommandRegistry":
        specs = _core_command_specs()
        by_name: dict[str, list[CommandSpec]] = {}
        for spec in specs:
            if spec.name not in by_name:
                by_name[spec.name] = []
            by_name[spec.name].append(spec)
        frozen = {name: tuple(spec_list) for name, spec_list in by_name.items()}

        # Build tcllib package index from specs that have tcllib_package set.
        pkg_to_cmds: dict[str, set[str]] = {}
        cmd_to_pkg: dict[str, str] = {}
        for spec in specs:
            if spec.tcllib_package:
                pkg_to_cmds.setdefault(spec.tcllib_package, set()).add(spec.name)
                cmd_to_pkg[spec.name] = spec.tcllib_package
        tcllib_pkgs = {pkg: frozenset(cmds) for pkg, cmds in pkg_to_cmds.items()}

        # Build general required_package index from all package-gated specs.
        cmd_to_req_pkg: dict[str, str] = {}
        for spec in specs:
            if spec.required_package:
                cmd_to_req_pkg[spec.name] = spec.required_package
        return cls(
            specs_by_name=frozen,
            _tcllib_packages=tcllib_pkgs,
            _tcllib_command_to_package=cmd_to_pkg,
            _command_to_required_package=cmd_to_req_pkg,
            _standalone_handlers={},
        )

    def load_dialect_specs(self, dialect: str) -> bool:
        """Load command specs for *dialect* if not already loaded.

        Returns ``True`` if new specs were added to the registry.
        Thread-safe: concurrent callers are serialised by ``_load_lock``.
        """
        loader_keys = _DIALECT_TO_LOADERS.get(dialect, ())
        # Fast path: avoid acquiring the lock when nothing needs loading.
        if not any(k not in self._loaded_loaders for k in loader_keys):
            return False

        with self._load_lock:
            # Re-check under lock (another thread may have loaded in the meantime).
            needed = [k for k in loader_keys if k not in self._loaded_loaders]
            if not needed:
                return False

            new_specs: list[CommandSpec] = []
            for key in needed:
                if key in _DIALECT_LOADER_SPECS:
                    new_specs.extend(_load_dialect_pack(key))
                self._loaded_loaders.add(key)

            if not new_specs:
                return False

            # Merge new specs into specs_by_name (mutable dict of tuples).
            by_name: dict[str, list[CommandSpec]] = {}
            for spec in new_specs:
                by_name.setdefault(spec.name, []).append(spec)

            for name, spec_list in by_name.items():
                existing = self.specs_by_name.get(name)
                if existing is not None:
                    self.specs_by_name[name] = existing + tuple(spec_list)
                else:
                    self.specs_by_name[name] = tuple(spec_list)

            # Update package indexes for newly loaded specs.
            for spec in new_specs:
                if spec.tcllib_package:
                    pkg = spec.tcllib_package
                    existing_cmds = self._tcllib_packages.get(pkg, frozenset())
                    self._tcllib_packages[pkg] = existing_cmds | {spec.name}
                    self._tcllib_command_to_package[spec.name] = pkg
                if spec.required_package:
                    self._command_to_required_package[spec.name] = spec.required_package

            # Invalidate all derived caches.
            self._trait_indexes = self._build_trait_indexes()
            self._command_names_cache.clear()
            self._event_command_cache.clear()
            self._legality_cache.clear()
            self._filtered_cache.clear()

            # Notify runtime.py to rebuild its derived data (only for the
            # global singleton, not for test-local registry copies).
            if _on_specs_loaded is not None and self is REGISTRY:
                _on_specs_loaded(needed)

            return True

    # Handler back-registration

    def register_handler(
        self,
        name: str,
        handler: SubcommandHandler | CommandHandler,
        *,
        subcommand: str | None = None,
    ) -> None:
        """Back-register a VM execution handler for *name*.

        When *subcommand* is given the handler is set on the matching
        ``SubCommand.handler``; otherwise on ``CommandSpec.handler``.
        """
        specs = self.specs_by_name.get(name)
        if specs is None:
            # No spec — store as a standalone handler (e.g. tcl::mathfunc::*).
            if subcommand is None:
                self._standalone_handlers[name] = handler
            return
        for spec in specs:
            if subcommand is not None:
                sub = spec.subcommands.get(subcommand)
                if sub is not None:
                    sub.handler = handler
            else:
                spec.handler = handler

    def register_codegen(
        self,
        name: str,
        hook: "CodegenHook",
        *,
        subcommand: str | None = None,
        target: str = "vm",
    ) -> None:
        """Back-register a codegen hook for *name* under *target*.

        *target* defaults to ``"vm"`` (Python bytecode) for backwards
        compatibility with call sites that predate the multi-backend
        ``codegens`` dict.  Phase E.7 unified the old singular
        ``spec.codegen`` field and the ``spec.codegens["wasm"]`` dict
        into one target-keyed dict; each registration names the backend
        it targets.
        """
        specs = self.specs_by_name.get(name)
        if specs is None:
            return
        for spec in specs:
            if subcommand is not None:
                sub = spec.subcommands.get(subcommand)
                if sub is not None:
                    sub.codegens[target] = hook
            else:
                spec.codegens[target] = hook

    def register_lowering(
        self,
        name: str,
        hook: LoweringHook,
        *,
        subcommand: str | None = None,
    ) -> None:
        """Back-register a lowering hook for *name*."""
        specs = self.specs_by_name.get(name)
        if specs is None:
            return
        for spec in specs:
            if subcommand is not None:
                sub = spec.subcommands.get(subcommand)
                if sub is not None:
                    sub.lowering = hook
            else:
                spec.lowering = hook

    def register_wasm_emitter(
        self,
        name: str,
        hook: "CodegenHook",
        *,
        target: str = "wasm",
        subcommand: str | None = None,
    ) -> None:
        """Back-register a WASM emit hook for *name* under *target*.

        Thin wrapper around :meth:`register_codegen` that defaults the
        target to ``"wasm"``.  Multiple WASM-like targets (e.g. future
        ``"wasm64"``) can coexist under different keys.
        """
        self.register_codegen(name, hook, target=target, subcommand=subcommand)

    def register_vm_emitter(
        self,
        name: str,
        hook: "CodegenHook",
        *,
        subcommand: str | None = None,
    ) -> None:
        """Back-register a VM bytecode hook for *name*.

        Thin wrapper around :meth:`register_codegen` with
        ``target="vm"``.  Provided for symmetry with
        :meth:`register_wasm_emitter`.
        """
        self.register_codegen(name, hook, target="vm", subcommand=subcommand)

    def get_wasm_hook(
        self,
        name: str,
        target: str = "wasm",
    ) -> "WasmEmitHook | None":
        """Return the first WASM emit hook registered for *name* under *target*.

        Searches all specs (not just the last/latest) because hooks are set at
        emitter import time.  Dialect packs loaded after that point add new
        specs with empty ``codegens`` dicts; the hook is still present on the
        earlier spec and this method finds it.

        Accepts both bare (``upvar``) and canonical (``::upvar``,
        ``::HTTP::respond``) command forms — hooks are registered on
        bare-name specs at emitter import time, so the canonical form
        strips its leading ``::`` to recover the bare name.  See issue #246.
        """
        specs = self.specs_by_name.get(name)
        if specs is None and name.startswith("::"):
            specs = self.specs_by_name.get(name[2:])
        if specs is None:
            return None
        for spec in specs:
            hook = spec.codegens.get(target)
            if hook is not None:
                return hook
        return None

    def lookup_handler(self, name: str) -> "CommandHandler | None":
        """Return the VM handler for *name*, or ``None``."""
        specs = self.specs_by_name.get(name)
        if specs is not None:
            for spec in reversed(specs):
                if spec.handler is not None:
                    return spec.handler
        return self._standalone_handlers.get(name)

    # Snapshots

    def filtered(
        self,
        dialect: str | None = None,
        active_packages: frozenset[str] | None = None,
    ) -> "CommandRegistry":
        """Return a snapshot containing only specs matching *dialect*/*active_packages*.

        Useful for passing to the segmenter or other components that
        need a pre-filtered view without importing the global REGISTRY.

        Results are cached — the static registry never changes within a
        session, so context changes (package edits) produce a new key.
        """
        self._ensure_dialect_loaded(dialect)
        key = (dialect, active_packages or None)
        cached = self._filtered_cache.get(key)
        if cached is not None:
            return cached
        filtered_by_name: dict[str, list[CommandSpec]] = {}
        for name, specs in self.specs_by_name.items():
            matching = [
                s
                for s in specs
                if s.supports_dialect(dialect) and s.supports_packages(active_packages)
            ]
            if matching:
                filtered_by_name[name] = matching
        frozen = {name: tuple(spec_list) for name, spec_list in filtered_by_name.items()}
        result = CommandRegistry(
            specs_by_name=frozen,
            _tcllib_packages=self._tcllib_packages,
            _tcllib_command_to_package=self._tcllib_command_to_package,
            _command_to_required_package=self._command_to_required_package,
            _standalone_handlers=self._standalone_handlers,
        )
        self._filtered_cache[key] = result
        return result

    # Lookup

    def get(
        self,
        name: str,
        dialect: str | None = None,
        active_packages: frozenset[str] | None = None,
    ) -> CommandSpec | None:
        self._ensure_dialect_loaded(dialect)
        specs = self.specs_by_name.get(name)
        if specs is None:
            return None

        # Prefer later specs so curated overrides win within the same dialect.
        for spec in reversed(specs):
            if spec.supports_dialect(dialect) and spec.supports_packages(active_packages):
                return spec
        return None

    def get_any(self, name: str) -> CommandSpec | None:
        """Return any spec for *name*, ignoring dialect and package filters.

        Useful for checking whether a command exists at all (in any dialect)
        before performing a dialect-filtered lookup.

        Qualified names (``::cmd``, ``::HTTP::respond``) for which only
        a bare-name spec exists also resolve here — e.g. ``::unset``
        returns the ``unset`` builtin spec, ``::HTTP::respond`` returns
        the registered ``HTTP::respond`` iRules spec.  This mirrors
        Tcl's command resolution: the global namespace form is just
        an explicitly-qualified spelling of the same builtin.  See
        issue #246.

        The leading ``::`` is stripped only when no exact match exists,
        so user-defined ``::ns::userproc`` does not accidentally pick
        up a same-name builtin (the lookup falls through to ``None``).
        """
        specs = self.specs_by_name.get(name)
        if specs is None and name.startswith("::"):
            specs = self.specs_by_name.get(name[2:])
        if specs is None:
            return None
        return specs[-1]  # prefer latest (most curated) spec

    def is_safe_on_uninit(
        self,
        command: str,
        subcommand: str | None = None,
        dialect: str | None = None,
    ) -> bool:
        """Return True if *command* (optionally with *subcommand*) safely
        initialises an uninitialised variable in *dialect*.

        Checks ``safe_on_uninit`` on the ``SubCommand`` first (if
        *subcommand* is given), then falls back to the top-level
        ``CommandSpec``.  An empty frozenset means "safe in all dialects";
        a non-empty frozenset restricts to the listed dialects.
        """
        spec = self.get(command, dialect)
        if spec is None:
            return False

        # Check subcommand first.
        if subcommand is not None:
            sub = spec.subcommands.get(subcommand)
            if sub is not None and sub.safe_on_uninit is not None:
                return not sub.safe_on_uninit or (
                    dialect is not None and dialect in sub.safe_on_uninit
                )
        # Fall back to top-level spec.
        if spec.safe_on_uninit is None:
            return False
        return not spec.safe_on_uninit or (dialect is not None and dialect in spec.safe_on_uninit)

    def command_status(
        self,
        name: str,
        dialect: str | None,
        active_packages: frozenset[str] | None = None,
    ) -> DialectStatus:
        """Lookup status of a command in *dialect* with *active_packages*.

        Returns a tri-state result: EXISTS, DEPRECATED, DISALLOWED, or
        NOT_EXISTS.  Replaces scattered boolean checks
        (``is_command_disabled``, ``REGISTRY.get() is None``,
        ``disabled_commands_for_active_profile()``, etc.).
        """
        spec = self.get(name, dialect, active_packages)
        if spec is not None:
            if spec.deprecated_replacement is not None:
                return DialectStatus.DEPRECATED
            return DialectStatus.EXISTS
        # Not in this dialect/package set — does it exist in ANY?
        if self.get_any(name) is not None:
            return DialectStatus.DISALLOWED
        return DialectStatus.NOT_EXISTS

    def subcommand_status(
        self,
        cmd: str,
        sub: str,
        dialect: str | None,
        active_packages: frozenset[str] | None = None,
    ) -> DialectStatus:
        """Lookup status of a subcommand in *dialect* with *active_packages*."""
        spec = self.get(cmd, dialect, active_packages)
        if spec is None:
            # Fall back: does the command exist at all?
            if self.get_any(cmd) is not None:
                return DialectStatus.DISALLOWED
            return DialectStatus.NOT_EXISTS

        sub_obj = spec.subcommands.get(sub)
        if sub_obj is not None:
            if not sub_obj.supports_dialect(dialect, spec.dialects):
                return DialectStatus.DISALLOWED
            if sub_obj.deprecated_replacement is not None:
                return DialectStatus.DEPRECATED
            return DialectStatus.EXISTS

        return DialectStatus.NOT_EXISTS

    def option_status(
        self,
        cmd: str,
        sub: str | None,
        option: str,
        dialect: str | None,
        active_packages: frozenset[str] | None = None,
    ) -> DialectStatus:
        """Lookup status of an option in *dialect* with *active_packages*.

        If *sub* is ``None``, checks command-level options.
        Otherwise checks subcommand-level options first, then falls back
        to command-level.
        """
        spec = self.get(cmd, dialect, active_packages)
        if spec is None:
            if self.get_any(cmd) is not None:
                return DialectStatus.DISALLOWED
            return DialectStatus.NOT_EXISTS

        opt: OptionSpec | None = None

        if sub is not None:
            # Check subcommand-level options (new subcommands dict).
            sub_obj = spec.subcommands.get(sub)
            if sub_obj is not None:
                if not sub_obj.supports_dialect(dialect, spec.dialects):
                    return DialectStatus.DISALLOWED
                # Inherited dialect set for option gating: the sub's own
                # dialects if set, else the parent's.
                parent_dialects = sub_obj.dialects or spec.dialects
                for o in sub_obj.options:
                    if o.name == option:
                        if not o.supports_dialect(dialect, parent_dialects):
                            return DialectStatus.DISALLOWED
                        opt = o
                        break

        # Fall back to command-level options.
        if opt is None:
            opt = spec.option(option, dialect)
            if opt is None and spec.option(option) is not None:
                # Option exists at command level but not in this dialect.
                return DialectStatus.DISALLOWED

        if opt is None:
            return DialectStatus.NOT_EXISTS

        # Parent dialect (command or sub) was already verified above.
        if sub is None and not spec.supports_dialect(dialect):
            return DialectStatus.DISALLOWED

        return DialectStatus.EXISTS

    def switches(
        self,
        name: str,
        dialect: str | None = None,
        active_packages: frozenset[str] | None = None,
    ) -> tuple[str, ...]:
        spec = self.get(name, dialect, active_packages)
        if spec is None:
            return ()
        return spec.switch_names(dialect)

    def option(
        self,
        name: str,
        option_name: str,
        dialect: str | None = None,
        active_packages: frozenset[str] | None = None,
    ) -> OptionSpec | None:
        spec = self.get(name, dialect, active_packages)
        if spec is None:
            return None
        return spec.option(option_name, dialect)

    def argument_values(
        self,
        name: str,
        arg_index: int,
        dialect: str | None = None,
        active_packages: frozenset[str] | None = None,
    ) -> tuple[ArgumentValueSpec, ...]:
        spec = self.get(name, dialect, active_packages)
        if spec is None:
            return ()
        return spec.argument_values(arg_index)

    def argument_value(
        self,
        name: str,
        arg_index: int,
        value: str,
        dialect: str | None = None,
        active_packages: frozenset[str] | None = None,
    ) -> ArgumentValueSpec | None:
        spec = self.get(name, dialect, active_packages)
        if spec is None:
            return None
        return spec.argument_value(arg_index, value)

    def subcommand_argument_values(
        self,
        name: str,
        subcommand: str,
        arg_index: int,
        dialect: str | None = None,
        active_packages: frozenset[str] | None = None,
    ) -> tuple[ArgumentValueSpec, ...]:
        spec = self.get(name, dialect, active_packages)
        if spec is None:
            return ()
        return spec.subcommand_argument_values(subcommand, arg_index)

    def subcommand_argument_value(
        self,
        name: str,
        subcommand: str,
        arg_index: int,
        value: str,
        dialect: str | None = None,
        active_packages: frozenset[str] | None = None,
    ) -> ArgumentValueSpec | None:
        spec = self.get(name, dialect, active_packages)
        if spec is None:
            return None
        return spec.subcommand_argument_value(subcommand, arg_index, value)

    def validation(
        self,
        name: str,
        dialect: str | None = None,
        active_packages: frozenset[str] | None = None,
    ) -> ValidationSpec | None:
        self._ensure_dialect_loaded(dialect)
        specs = self.specs_by_name.get(name)
        if specs is None:
            return None
        for spec in reversed(specs):
            if not spec.supports_dialect(dialect):
                continue
            if not spec.supports_packages(active_packages):
                continue
            if spec.validation is not None:
                return spec.validation
        # Known command for dialect, but no validation metadata found.
        return None

    def command_names(
        self,
        dialect: str | None = None,
        active_packages: frozenset[str] | None = None,
    ) -> tuple[str, ...]:
        self._ensure_dialect_loaded(dialect)
        packages = self._normalise_package_set(active_packages)
        cache_key = (dialect, packages)
        cached = self._command_names_cache.get(cache_key)
        if cached is not None:
            return cached
        names: list[str] = []
        for name in sorted(self.specs_by_name):
            if self.get(name, dialect, packages) is not None:
                names.append(name)
        result = tuple(names)
        self._command_names_cache[cache_key] = result
        return result

    def subcommand_names(
        self,
        cmd_name: str,
        dialect: str | None = None,
        active_packages: frozenset[str] | None = None,
    ) -> tuple[str, ...]:
        """Return sorted subcommand names for *cmd_name* in *dialect*."""
        spec = self.get(cmd_name, dialect, active_packages)
        if spec is None or not spec.subcommands:
            return ()
        return tuple(
            sorted(
                sub_name
                for sub_name, sub in spec.subcommands.items()
                if sub.supports_dialect(dialect, spec.dialects)
            )
        )

    def commands_for_packages(
        self,
        packages: frozenset[str],
        dialect: str | None = None,
    ) -> tuple[str, ...]:
        """Return command names available for the given required packages.

        Commands with ``required_package`` set are only included when that
        package is in *packages*.  Unconditional commands (where
        ``required_package`` is ``None``) are always included.
        """
        return self.command_names(dialect, active_packages=packages)

    def _any_spec_has(self, name: str, attr: str) -> bool:
        """Return True if any spec for *name* has a truthy value for *attr*.

        Accepts both bare (``set``, ``HTTP::respond``) and canonical
        (``::set``, ``::HTTP::respond``) command forms — the canonical
        form is the spelling stamped on ``IRCall.canonical_command`` by
        lowering, so passes that match on canonical can call this helper
        without re-stripping.  Specs themselves are registered under
        bare names (``set``, ``HTTP::respond``); the leading ``::`` is
        stripped to recover the bare form.  See issue #246.
        """
        trait_names = self._trait_indexes.get(attr)
        if trait_names is not None:
            if name in trait_names:
                return True
            if name.startswith("::"):
                return name[2:] in trait_names
            return False
        specs = self.specs_by_name.get(name)
        if specs is None and name.startswith("::"):
            specs = self.specs_by_name.get(name[2:])
        if specs is None:
            return False
        return any(getattr(spec, attr, False) for spec in specs)

    def is_dynamic_barrier(self, name: str, dialect: str | None = None) -> bool:
        """Check if the command creates a dynamic barrier (eval, uplevel, etc.)."""
        self._ensure_dialect_loaded(dialect)
        return self._any_spec_has(name, "creates_dynamic_barrier")

    def has_loop_body(self, name: str, dialect: str | None = None) -> bool:
        """Check if the command has a loop body (for, while, foreach)."""
        self._ensure_dialect_loaded(dialect)
        return self._any_spec_has(name, "has_loop_body")

    def never_inline_body(self, name: str, dialect: str | None = None) -> bool:
        """Check if the command's body should never be formatted inline."""
        self._ensure_dialect_loaded(dialect)
        return self._any_spec_has(name, "never_inline_body")

    def resolve_option_terminator(
        self,
        name: str,
        args: list[str] | tuple[str, ...],
    ) -> ResolvedTerminator | None:
        """Resolve the option-terminator profile for a command invocation.

        Matches the invocation's first argument against SubCommand entries
        that declare ``OptionSpec(name="--")``.  Falls back to form-level
        ``--`` declarations.  Returns ``None`` when the command does not
        support ``--`` at all.
        """
        specs = self.specs_by_name.get(name)
        if specs is None:
            return None

        for spec in reversed(specs):
            # Check subcommand-scoped first.
            if args and spec.subcommands:
                sub = spec.subcommands.get(args[0])
                if sub is not None and any(o.name == "--" for o in sub.options):
                    owv = self.options_with_values(name, args[0])
                    return ResolvedTerminator(
                        scan_start=1,
                        subcommand=args[0],
                        options_with_values=owv,
                        warn_without_terminator=spec.warn_without_terminator,
                    )

            # Check form-level options.
            for form in spec.forms:
                if any(o.name == "--" for o in form.options):
                    owv = self.options_with_values(name)
                    return ResolvedTerminator(
                        scan_start=0,
                        subcommand=None,
                        options_with_values=owv,
                        warn_without_terminator=spec.warn_without_terminator,
                    )

        return None

    def options_with_values(
        self,
        name: str,
        subcommand: str | None = None,
        dialect: str | None = None,
    ) -> frozenset[str]:
        """Derive the set of option names that consume a value argument.

        Returns option names where ``OptionSpec.takes_value`` is ``True``,
        collected from the relevant scope (subcommand options if *subcommand*
        is given, otherwise form-level options).  When *dialect* is given,
        options gated to a different dialect are skipped.
        """
        specs = self.specs_by_name.get(name)
        if specs is None:
            return frozenset()
        result: set[str] = set()
        for spec in specs:
            if subcommand is not None:
                sub = spec.subcommands.get(subcommand)
                if sub is not None:
                    parent_dialects = sub.dialects or spec.dialects
                    for opt in sub.options:
                        if not opt.takes_value:
                            continue
                        if not opt.supports_dialect(dialect, parent_dialects):
                            continue
                        result.add(opt.name)
            for form in spec.forms:
                for opt in form.options:
                    if not opt.takes_value:
                        continue
                    if not opt.supports_dialect(dialect, spec.dialects):
                        continue
                    result.add(opt.name)
        return frozenset(result)

    def dynamic_barrier_commands(self, dialect: str | None = None) -> frozenset[str]:
        """Return all commands that create dynamic barriers."""
        self._ensure_dialect_loaded(dialect)
        return self._trait_names("creates_dynamic_barrier")

    def loop_body_commands(self, dialect: str | None = None) -> frozenset[str]:
        """Return all commands that have loop bodies."""
        self._ensure_dialect_loaded(dialect)
        return self._trait_names("has_loop_body")

    def never_inline_body_commands(self, dialect: str | None = None) -> frozenset[str]:
        """Return all commands whose bodies should never be formatted inline."""
        self._ensure_dialect_loaded(dialect)
        return self._trait_names("never_inline_body")

    # Purity / CSE

    def is_pure(self, name: str) -> bool:
        """Check if the command is side-effect-free and deterministic."""
        return self._any_spec_has(name, "pure")

    def pure_subcommands(self, name: str) -> frozenset[str] | None:
        """Return subcommand names flagged as pure (side-effect-free)."""
        specs = self.specs_by_name.get(name, ())
        result: frozenset[str] = frozenset()
        found = False
        for spec in specs:
            derived = spec.pure_subcommand_names
            if derived:
                result |= derived
                found = True
        return result if found else None

    def is_cse_candidate(self, name: str) -> bool:
        """Check if redundant calls are worth flagging as O105."""
        return self._any_spec_has(name, "cse_candidate")

    def pure_commands(self) -> frozenset[str]:
        """Return all blanket-pure commands."""
        return self._trait_names("pure")

    def cse_candidate_commands(self) -> frozenset[str]:
        """Return all commands worth flagging for CSE."""
        return self._trait_names("cse_candidate")

    def is_unsafe(self, name: str) -> bool:
        """Check if the command is unsafe (allows context escalation in sandboxed dialects)."""
        return self._any_spec_has(name, "unsafe")

    def mutator_subcommands(self, name: str) -> frozenset[str] | None:
        """Return subcommand names flagged as state-mutating."""
        specs = self.specs_by_name.get(name, ())
        result: frozenset[str] = frozenset()
        found = False
        for spec in specs:
            derived = spec.mutator_subcommand_names
            if derived:
                result |= derived
                found = True
        return result if found else None

    def side_effect_hints(
        self,
        name: str,
        subcommand: str | None = None,
        dialect: str | None = None,
    ) -> tuple[SideEffect, ...] | None:
        """Return static side-effect hints for ``name`` and optional ``subcommand``.

        Subcommand hints take precedence over command-level hints. When a
        dialect is supplied, returned effects are normalised so each hint
        explicitly carries the active dialect.
        """
        filter_dialect = "f5-irules" if dialect == "irules" else dialect
        self._ensure_dialect_loaded(filter_dialect)

        specs = self.specs_by_name.get(name, ())
        for spec in reversed(specs):
            if filter_dialect is not None and not spec.supports_dialect(filter_dialect):
                continue

            if subcommand is not None:
                sub = spec.subcommands.get(subcommand)
                if (
                    sub is not None
                    and sub.side_effect_hints is not None
                    and (
                        filter_dialect is None
                        or sub.supports_dialect(filter_dialect, spec.dialects)
                    )
                ):
                    if dialect is None:
                        return sub.side_effect_hints
                    return tuple(
                        replace(effect, dialect=dialect) for effect in sub.side_effect_hints
                    )

            if spec.side_effect_hints is not None:
                if dialect is None:
                    return spec.side_effect_hints
                return tuple(replace(effect, dialect=dialect) for effect in spec.side_effect_hints)

        return None

    def destructive_subcommands(self, name: str) -> frozenset[str]:
        """Return subcommand names flagged as destructive (e.g. file delete)."""
        specs = self.specs_by_name.get(name, ())
        result: frozenset[str] = frozenset()
        for spec in specs:
            result |= frozenset(n for n, s in spec.subcommands.items() if s.destructive)
        return result

    # Security credential queries

    def credential_options(self, name: str) -> frozenset[str] | None:
        """Return option flags that carry secrets for *name* (e.g. {"-headers"})."""
        specs = self.specs_by_name.get(name, ())
        for spec in specs:
            if spec.credential_options is not None:
                return spec.credential_options
        return None

    def subcommand_credential_info(
        self,
        name: str,
        sub: str,
    ) -> tuple[int | None, frozenset[str] | None]:
        """Return ``(credential_arg, sensitive_headers)`` for a subcommand.

        Performs a single ``specs_by_name`` lookup instead of two.
        """
        specs = self.specs_by_name.get(name, ())
        for spec in specs:
            sc = spec.subcommands.get(sub)
            if sc is not None and sc.credential_arg is not None:
                return sc.credential_arg, sc.sensitive_headers
        return None, None

    # Taint sink queries

    def classify_taint_sinks(
        self,
        name: str,
        subcommand: str | None = None,
        dialect: str | None = None,
    ) -> TaintSinkInfo:
        """Classify all taint sink properties of *name* in a single pass.

        Returns a :class:`TaintSinkInfo` with all relevant sink flags set.
        This avoids repeated ``specs_by_name`` lookups and dialect filtering.
        """
        self._ensure_dialect_loaded(dialect)
        specs = self.specs_by_name.get(name, ())
        if not specs:
            return _EMPTY_TAINT_SINK_INFO

        is_code_sink = False
        output_sink: str | None = None
        output_sink_sub_qualified = False
        log_sink: str | None = None
        is_network_sink = False
        interp_eval_subs: frozenset[str] | None = None

        for spec in specs:
            if dialect is not None and not spec.supports_dialect(dialect):
                continue
            if spec.taint_sink:
                is_code_sink = True
            if output_sink is None and spec.taint_output_sink is not None:
                subs = spec.taint_output_sink_subcommands
                if subs is None or (subcommand is not None and subcommand in subs):
                    output_sink = spec.taint_output_sink
                    output_sink_sub_qualified = subs is not None
            if log_sink is None and spec.taint_log_sink is not None:
                log_sink = spec.taint_log_sink
            if not is_network_sink and spec.taint_network_sink_args is not None:
                is_network_sink = True
            if interp_eval_subs is None and spec.taint_interp_eval_subcommands is not None:
                interp_eval_subs = spec.taint_interp_eval_subcommands

        return TaintSinkInfo(
            is_code_sink=is_code_sink,
            output_sink=output_sink,
            output_sink_is_subcommand_qualified=output_sink_sub_qualified,
            log_sink=log_sink,
            is_network_sink=is_network_sink,
            interp_eval_subcommands=interp_eval_subs,
        )

    # Diagram extraction

    def is_diagram_action(self, name: str) -> bool:
        """Check if the command should appear as a notable action in diagrams."""
        return self._any_spec_has(name, "diagram_action")

    def diagram_action_commands(self) -> frozenset[str]:
        """Return all commands that appear as notable actions in diagrams."""
        return self._trait_names("diagram_action")

    # XC translatability

    def is_xc_never_translatable(self, name: str) -> bool:
        """Check if the command is explicitly marked as never translatable to XC."""
        specs = self.specs_by_name.get(name)
        if specs is None:
            return False
        return any(spec.xc_translatable is False for spec in specs)

    def xc_never_translatable_commands(self) -> frozenset[str]:
        """Return commands explicitly marked as never translatable to XC."""
        return frozenset(n for n in self.specs_by_name if self.is_xc_never_translatable(n))

    def is_xc_translatable_override(self, name: str) -> bool:
        """Check if the command is translatable despite namespace prefix."""
        specs = self.specs_by_name.get(name)
        if specs is None:
            return False
        return any(spec.xc_translatable is True for spec in specs)

    def xc_translatable_override_commands(self) -> frozenset[str]:
        """Return commands marked translatable despite namespace prefix."""
        return frozenset(n for n in self.specs_by_name if self.is_xc_translatable_override(n))

    # Tcllib package support

    def tcllib_package_for(self, name: str) -> str | None:
        """Return the tcllib package name for a command, or ``None``."""
        return self._tcllib_command_to_package.get(name)

    def is_tcllib_command(self, name: str) -> bool:
        """Return ``True`` if *name* is a tcllib-provided command."""
        return name in self._tcllib_command_to_package

    def tcllib_command_names(
        self,
        packages: frozenset[str] | set[str] = frozenset(),
    ) -> tuple[str, ...]:
        """Return tcllib command names whose package is in *packages*."""
        if not packages:
            return ()
        names: list[str] = []
        for pkg in sorted(packages):
            cmds = self._tcllib_packages.get(pkg)
            if cmds:
                names.extend(sorted(cmds))
        return tuple(names)

    def known_tcllib_packages(self) -> frozenset[str]:
        """Return the set of all known tcllib package names."""
        return frozenset(self._tcllib_packages)

    # Analysis check dispatch traits

    def check_trait_commands(self, trait: str) -> frozenset[str]:
        """Return command names with the given analysis check trait."""
        return self._trait_names(trait)

    def all_tcllib_command_names(self) -> frozenset[str]:
        """Return all registered tcllib command names."""
        return frozenset(self._tcllib_command_to_package)

    # General package support

    def required_package_for(self, name: str) -> str | None:
        """Return the required package for any package-gated command."""
        return self._command_to_required_package.get(name)

    def has_required_package(self, name: str) -> bool:
        """Return ``True`` if *name* needs a ``package require`` to activate."""
        return name in self._command_to_required_package

    # Control flow classification

    def is_control_flow(self, name: str) -> bool:
        """Return ``True`` if *name* is a control-flow command (if, for, etc.)."""
        return self._any_spec_has(name, "is_control_flow")

    def control_flow_commands(self) -> frozenset[str]:
        """Return all control-flow commands."""
        return self._trait_names("is_control_flow")

    def is_needs_start_cmd(self, name: str) -> bool:
        """Return ``True`` if *name* needs startCommand wrapping in bytecode."""
        return self._any_spec_has(name, "needs_start_cmd")

    def needs_start_cmd_commands(self) -> frozenset[str]:
        """Return all commands that need startCommand wrapping."""
        return self._trait_names("needs_start_cmd")

    def is_defines_procedure(self, name: str) -> bool:
        """Check if this command defines a new procedure."""
        return self._any_spec_has(name, "defines_procedure")

    # Rendered-value property traits

    def is_path_returning(self, name: str) -> bool:
        """Check if the command returns a filesystem path."""
        return self._any_spec_has(name, "returns_path")

    def path_returning_commands(self) -> frozenset[str]:
        """Return all commands that return filesystem paths."""
        return self._trait_names("returns_path")

    def path_returning_subcommands(self, name: str) -> frozenset[str] | None:
        """Return subcommand names flagged as returning filesystem paths."""
        specs = self.specs_by_name.get(name, ())
        result: frozenset[str] = frozenset()
        found = False
        for spec in specs:
            derived = spec.path_returning_subcommand_names
            if derived:
                result |= derived
                found = True
        return result if found else None

    def is_unescape_command(self, name: str) -> bool:
        """Check if the command performs unescaping / decoding."""
        return self._any_spec_has(name, "is_unescape_command")

    def unescape_commands(self) -> frozenset[str]:
        """Return all commands that perform unescaping / decoding."""
        return self._trait_names("is_unescape_command")

    def unescape_subcommands(self, name: str) -> frozenset[str] | None:
        """Return subcommand names flagged as performing unescaping."""
        specs = self.specs_by_name.get(name, ())
        result: frozenset[str] = frozenset()
        found = False
        for spec in specs:
            derived = spec.unescape_subcommand_names
            if derived:
                result |= derived
                found = True
        return result if found else None

    def is_unnormalized_http_getter(self, name: str) -> bool:
        """Check if the command is an unnormalized HTTP getter."""
        return self._any_spec_has(name, "is_unnormalized_http_getter")

    def is_pure_evaluation(self, name: str) -> bool:
        """Check if the command is pure evaluation (expr)."""
        return self._any_spec_has(name, "pure_evaluation")

    def pure_evaluation_commands(self) -> frozenset[str]:
        """Return all pure-evaluation commands."""
        return self._trait_names("pure_evaluation")

    def is_destroys_variable(self, name: str) -> bool:
        """Check if the command destroys/removes a variable (unset)."""
        return self._any_spec_has(name, "destroys_variable")

    def destroys_variable_commands(self) -> frozenset[str]:
        """Return all commands that destroy/remove variables."""
        return self._trait_names("destroys_variable")

    def is_language_keyword(self, name: str) -> bool:
        """Check if the command is a language keyword for semantic tokens."""
        return self._any_spec_has(name, "is_language_keyword")

    def language_keyword_commands(self) -> frozenset[str]:
        """Return all language keyword commands."""
        return self._trait_names("is_language_keyword")

    # Variable read-modify-write

    def is_reads_variable_before_write(self, name: str) -> bool:
        """Check if the command reads the variable before writing (incr, append, lappend)."""
        return self._any_spec_has(name, "reads_variable_before_write")

    def reads_variable_before_write_commands(self) -> frozenset[str]:
        """Return all read-modify-write variable commands."""
        return self._trait_names("reads_variable_before_write")

    # Boolean condition context

    def has_boolean_condition(self, name: str) -> bool:
        """Check if the command's first expression is in boolean context."""
        return self._any_spec_has(name, "has_boolean_condition")

    def boolean_condition_commands(self) -> frozenset[str]:
        """Return all commands with boolean condition expressions."""
        return self._trait_names("has_boolean_condition")

    # Canonical list production

    def is_produces_canonical_list(self, name: str) -> bool:
        """Check if the command produces a canonical Tcl list."""
        return self._any_spec_has(name, "produces_canonical_list")

    def canonical_list_commands(self) -> frozenset[str]:
        """Return all commands that produce canonical lists."""
        return self._trait_names("produces_canonical_list")

    # Side-switching (iRules)

    def is_side_switch(self, name: str) -> bool:
        """Check if the command is a side-switching command (clientside/serverside)."""
        return self._any_spec_has(name, "is_side_switch")

    def side_switch_commands(self) -> frozenset[str]:
        """Return all side-switching commands."""
        return self._trait_names("is_side_switch")

    # iRules top-level-only

    def is_irules_top_level_only(self, name: str) -> bool:
        """Check if the command must appear at top level in iRules."""
        return self._any_spec_has(name, "irules_top_level_only")

    def irules_top_level_only_commands(self) -> frozenset[str]:
        """Return all commands that must appear at top level in iRules."""
        return self._trait_names("irules_top_level_only")

    # Scope alias (upvar-like) queries

    def creates_scope_alias_commands(self) -> frozenset[str]:
        """Return all commands with creates_scope_alias (upvar-like)."""
        result: set[str] = set()
        for name, specs in self.specs_by_name.items():
            for spec in specs:
                if spec.creates_scope_alias:
                    result.add(name)
                    break
                for sub in spec.subcommands.values():
                    if sub.creates_scope_alias:
                        result.add(f"{name} {sub.name}")
                        break
        return frozenset(result)

    def is_scope_alias_command(self, name: str) -> bool:
        """Check if *name* (possibly compound like 'namespace upvar') creates a scope alias."""
        # Handle compound names like "namespace upvar"
        parts = name.split(" ", 1)
        specs = self.specs_by_name.get(parts[0])
        if specs is None:
            return False
        if len(parts) == 1:
            return any(spec.creates_scope_alias for spec in specs)
        sub_name = parts[1]
        for spec in specs:
            sub = spec.subcommands.get(sub_name)
            if sub is not None and sub.creates_scope_alias:
                return True
        return False

    # Command-classification queries (single source of truth = the spec).
    # "Any spec registered under the name" semantics so a dialect-shadowing
    # spec can't hide a core stamp.

    def is_byte_compiled(self, name: str) -> bool:
        """Core builtin the minifier must never rewrite to a ``$var`` alias."""
        return self._any_spec_has(name, "byte_compiled")

    def is_not_proc_factory(self, name: str) -> bool:
        """Registered head matching ``HEAD NAME BRACED BRACED`` but not a proc factory."""
        return self._any_spec_has(name, "not_proc_factory")

    def is_frameless_runtime(self, name: str) -> bool:
        """Command whose codegen always uses a runtime helper (no callee frame)."""
        return self._any_spec_has(name, "frameless_runtime")

    def is_irules_dialect(self, dialect: str | None) -> bool:
        """Whether *dialect* is the F5 iRules dialect (``f5-irules`` / ``irules``)."""
        from compiler.registry.dialects import is_irules_dialect

        return is_irules_dialect(dialect)

    def has_fixed_ensembles(self, dialect: str | None) -> bool:
        """Whether *dialect* guarantees fixed ensembles (no user-added subcommands)."""
        from compiler.registry.dialects import has_fixed_ensembles

        return has_fixed_ensembles(dialect)

    # Event-scoped command sets

    _EVENT_AWARE_DIALECTS = frozenset({"f5-irules"})

    def commands_for_event(
        self,
        dialect: str,
        event: str | None,
    ) -> EventCommandSet:
        """Return cached valid command set for *(dialect, event)*.

        Only meaningful for event-aware dialects (currently ``f5-irules``).
        For non-event dialects, returns all commands with no event
        filtering.  Raises ``ValueError`` if *event* is non-None for
        a dialect that has no event concept.

        Built on first access; the cache is invalidated when
        ``load_dialect_specs`` expands the registry.
        """
        self._ensure_dialect_loaded(dialect)
        if event is not None and dialect not in self._EVENT_AWARE_DIALECTS:
            raise ValueError(
                f"dialect {dialect!r} has no event concept; event={event!r} is not valid"
            )
        key = (dialect, event)
        cached = self._event_command_cache.get(key)
        if cached is not None:
            return cached
        result = self._build_event_set(dialect, event)
        self._event_command_cache[key] = result
        return result

    def _build_event_set(
        self,
        dialect: str,
        event: str | None,
    ) -> EventCommandSet:
        from .namespace_data import event_satisfies, get_event_props

        valid: set[str] = set()
        event_scoped: set[str] = set()
        out_of_event: set[str] = set()
        valid_subs: dict[str, frozenset[str]] = {}

        event_props = get_event_props(event) if event else None
        # Unknown event name: treat commands with event_requires as
        # out-of-event so we don't return an overly permissive set.
        unknown_event = event is not None and event_props is None

        for name, specs in self.specs_by_name.items():
            # Pick the best spec for this dialect.
            # Use reversed() so curated overrides win, matching get().
            spec: CommandSpec | None = None
            for s in reversed(specs):
                if s.supports_dialect(dialect):
                    spec = s
                    break
            if spec is None:
                continue

            # Track whether this command has event-specific metadata.
            has_event_metadata = bool(spec.event_requires or spec.excluded_events)

            # Check excluded_events first.
            if event and spec.excluded_events and event in spec.excluded_events:
                out_of_event.add(name)
                continue

            # Check event_requires.
            if spec.event_requires is not None:
                if unknown_event or (
                    event_props is not None
                    and not event_satisfies(event_props, spec.event_requires, event)
                ):
                    out_of_event.add(name)
                    continue

            valid.add(name)
            if has_event_metadata:
                event_scoped.add(name)
            # Pre-compute valid subcommands for this dialect.
            if spec.subcommands:
                subs = frozenset(
                    sub_name
                    for sub_name, sub in spec.subcommands.items()
                    if sub.supports_dialect(dialect, spec.dialects)
                )
                if subs:
                    valid_subs[name] = subs

        return EventCommandSet(
            valid_commands=frozenset(valid),
            out_of_event_commands=frozenset(out_of_event),
            valid_subcommands=valid_subs,
            event_scoped_commands=frozenset(event_scoped),
        )

    def command_legality(self, dialect: str) -> CommandLegality:
        """Return cached legality matrix for *dialect*.

        Materialises ``(event, command) -> legal`` for every known event,
        allowing O(1) lookup from diagnostics and completions.
        """
        self._ensure_dialect_loaded(dialect)
        cached = self._legality_cache.get(dialect)
        if cached is not None:
            return cached
        result = self._build_legality(dialect)
        self._legality_cache[dialect] = result
        return result

    def _build_legality(self, dialect: str) -> CommandLegality:
        from .namespace_data import EVENT_PROPS

        by_event: dict[str, frozenset[str]] = {}
        out_of_event: dict[str, frozenset[str]] = {}

        for event_name in EVENT_PROPS:
            event_set = self.commands_for_event(dialect, event_name)
            by_event[event_name] = event_set.valid_commands
            out_of_event[event_name] = event_set.out_of_event_commands

        return CommandLegality(
            _by_event=by_event,
            _out_of_event=out_of_event,
        )

    def deprecation_coverage(self) -> dict[str, bool]:
        """Return coverage stats for deprecated commands with fixers.

        Returns a dict mapping deprecated command names to whether they have
        a ``deprecation_fixer`` callable attached.  Used by quality gate tests.
        If a command has multiple deprecated specs (e.g. per dialect), the
        result is ``True`` only if *all* deprecated variants have a fixer.
        """
        result: dict[str, bool] = {}
        for name, specs in self.specs_by_name.items():
            for spec in specs:
                if spec.deprecated_replacement is not None:
                    has_fixer = spec.deprecation_fixer is not None
                    result[name] = result.get(name, True) and has_fixer
                for sub_name, sub in spec.subcommands.items():
                    if sub.deprecated_replacement is not None:
                        key = f"{name} {sub_name}"
                        has_fixer = sub.deprecation_fixer is not None
                        result[key] = result.get(key, True) and has_fixer
        return result


REGISTRY = CommandRegistry.build_default()
