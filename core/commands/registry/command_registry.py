"""Central command registry for completion and hover metadata."""

from __future__ import annotations

from dataclasses import dataclass, field, replace
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from ...compiler.side_effects import SideEffect
    from .models import CodegenHook, CommandHandler, LoweringHook, SubcommandHandler

from .expect import expect_command_specs
from .iapps import iapps_command_specs
from .irules import irules_command_specs
from .models import (
    ArgumentValueSpec,
    CommandLegality,
    CommandSpec,
    DialectStatus,
    EventCommandSet,
    OptionSpec,
    OptionTerminatorSpec,
    ValidationSpec,
)
from .stdlib import stdlib_command_specs
from .taint_sink_info import _EMPTY_TAINT_SINK_INFO, TaintSinkInfo
from .tcl import tcl_command_specs
from .tcllib import tcllib_command_specs
from .tk import tk_command_specs

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
)


def _all_command_specs() -> tuple:
    """Return command specs from all built-in command catalogs.

    Class-per-command Tcl specs first, then stdlib (package-gated),
    then tcllib library commands, then package-based packs (Tk),
    then dialect packs (irules, iapps).  Later entries override
    earlier ones for the same command name within a dialect.
    """
    return (
        tcl_command_specs()
        + stdlib_command_specs()
        + tcllib_command_specs()
        + tk_command_specs()
        + irules_command_specs()
        + iapps_command_specs()
        + expect_command_specs()
    )


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

    @classmethod
    def build_default(cls) -> "CommandRegistry":
        specs = _all_command_specs()
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
        hook: CodegenHook,
        *,
        subcommand: str | None = None,
    ) -> None:
        """Back-register a codegen hook for *name*."""
        specs = self.specs_by_name.get(name)
        if specs is None:
            return
        for spec in specs:
            if subcommand is not None:
                sub = spec.subcommands.get(subcommand)
                if sub is not None:
                    sub.codegen = hook
            else:
                spec.codegen = hook

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
        """
        specs = self.specs_by_name.get(name)
        if specs is None:
            return None
        return specs[-1]  # prefer latest (most curated) spec

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
                for o in sub_obj.options:
                    if o.name == option:
                        opt = o
                        break

        # Fall back to command-level options.
        if opt is None:
            opt = spec.option(option)

        if opt is None:
            return DialectStatus.NOT_EXISTS

        # Options inherit dialect from their parent.
        if sub is not None:
            sub_obj = spec.subcommands.get(sub)
            if sub_obj is not None and not sub_obj.supports_dialect(dialect, spec.dialects):
                return DialectStatus.DISALLOWED
        elif not spec.supports_dialect(dialect):
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
        return spec.switch_names()

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
        return spec.option(option_name)

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
        """Return True if any spec for *name* has a truthy value for *attr*."""
        trait_names = self._trait_indexes.get(attr)
        if trait_names is not None:
            return name in trait_names
        specs = self.specs_by_name.get(name)
        if specs is None:
            return False
        return any(getattr(spec, attr, False) for spec in specs)

    def is_dynamic_barrier(self, name: str, dialect: str | None = None) -> bool:
        """Check if the command creates a dynamic barrier (eval, uplevel, etc.)."""
        return self._any_spec_has(name, "creates_dynamic_barrier")

    def has_loop_body(self, name: str, dialect: str | None = None) -> bool:
        """Check if the command has a loop body (for, while, foreach)."""
        return self._any_spec_has(name, "has_loop_body")

    def never_inline_body(self, name: str, dialect: str | None = None) -> bool:
        """Check if the command's body should never be formatted inline."""
        return self._any_spec_has(name, "never_inline_body")

    def option_terminator_profiles(
        self,
        name: str,
        dialect: str | None = None,
    ) -> tuple[OptionTerminatorSpec, ...]:
        """Return option-terminator profiles for the command.

        Merges profiles from all specs for the command.  Prefers
        SubCommand-derived option terminators when the ``subcommands``
        dict is populated, falling back to the legacy
        ``option_terminator_profiles`` field.
        """
        specs = self.specs_by_name.get(name)
        if specs is None:
            return ()
        profiles: list[OptionTerminatorSpec] = []
        seen: set[str | None] = set()
        for spec in reversed(specs):
            # Prefer SubCommand-derived option terminators
            if spec.subcommands:
                for sub_name, sub in spec.subcommands.items():
                    if sub.option_terminator and sub_name not in seen:
                        seen.add(sub_name)
                        ot = sub.option_terminator
                        # Ensure the profile carries the subcommand name
                        if ot.subcommand != sub_name:
                            from dataclasses import replace

                            ot = replace(ot, subcommand=sub_name)
                        profiles.append(ot)
            # Fall back to legacy profiles
            for p in spec.option_terminator_profiles:
                if p.subcommand not in seen:
                    seen.add(p.subcommand)
                    profiles.append(p)
        return tuple(profiles)

    def dynamic_barrier_commands(self, dialect: str | None = None) -> frozenset[str]:
        """Return all commands that create dynamic barriers."""
        return self._trait_names("creates_dynamic_barrier")

    def loop_body_commands(self, dialect: str | None = None) -> frozenset[str]:
        """Return all commands that have loop bodies."""
        return self._trait_names("has_loop_body")

    def never_inline_body_commands(self, dialect: str | None = None) -> frozenset[str]:
        """Return all commands whose bodies should never be formatted inline."""
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

        Built on first access, invalidated only if registry changes
        (which doesn't happen after startup).
        """
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
