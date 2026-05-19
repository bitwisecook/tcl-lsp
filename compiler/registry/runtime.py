"""Registry runtime: dialect profile + role-aware signature resolution.

Command inventory, hover/completion metadata, and baseline arity come from the
registry command specs. This module owns runtime profile state and arg-role
semantics needed by analysis/formatting/compiler passes.
"""

from __future__ import annotations

import contextlib
import importlib
from collections.abc import Iterable, Iterator
from dataclasses import dataclass
from functools import lru_cache
from typing import TYPE_CHECKING, TypedDict

if TYPE_CHECKING:
    from analyser.semantic_model import StubCommandDef
    from compiler.side_effects import StorageType

from compiler.parsing.tokens import Token
from compiler.types import TclType

from .command_registry import REGISTRY
from .models import CommandSpec, PatternType, ValidationSpec
from .signatures import ArgRole, Arity, BodyKind, CommandSig, SubcommandSig
from .taint_hints import TaintColour, TaintHint
from .type_hints import CommandTypeHint, SubcommandTypeHint


class SignatureProfile(TypedDict):
    dialect: str
    extra_commands: list[str]


# Re-export so existing callers keep working via ``from ...runtime import ...``.
__all__ = [
    "ArgRole",
    "BodyArgument",
    "BodyKind",
    "CommandSig",
    "SubcommandSig",
    "CommandTypeHint",
    "SubcommandTypeHint",
    "SwitchCase",
    "TaintHint",
    "body_kind_for_command",
    "is_switch_case_list_form",
    "iter_switch_case_list",
    "options_with_value",
    "regexp_pattern_index",
    "resolve_arg_role_map",
    "resolve_rewrite_alias",
    "skip_options",
]


def _role_hints_from_registry() -> dict[str, CommandSig | SubcommandSig]:
    """Build role hints from inline arg_roles on CommandSpec/SubCommand.

    This replaces the old tcl_role_hints() aggregator.  The returned dict
    is used by _with_roles() to merge arg_roles onto validation-derived
    signatures for commands that haven't been fully migrated to SubCommand
    dicts (e.g. iRules overrides of Tcl commands).
    """
    hints: dict[str, CommandSig | SubcommandSig] = {}
    for name, specs in REGISTRY.specs_by_name.items():
        for spec in specs:
            if spec.subcommands:
                sub_hints = {}
                for sub_name, sub in spec.subcommands.items():
                    if sub.arg_roles:
                        sub_hints[sub_name] = CommandSig(
                            arity=sub.arity,
                            arg_roles=dict(sub.arg_roles),
                        )
                    else:
                        sub_hints[sub_name] = CommandSig(arity=sub.arity)
                if sub_hints:
                    hints.setdefault(
                        name,
                        SubcommandSig(
                            subcommands=sub_hints,
                            allow_unknown=spec.allow_unknown_subcommands,
                        ),
                    )
            elif spec.arg_roles or spec.arg_role_resolver:
                arity = spec.validation.arity if spec.validation else Arity()
                hints.setdefault(
                    name,
                    CommandSig(
                        arity=arity,
                        arg_roles=dict(spec.arg_roles) if spec.arg_roles else {},
                        arg_role_resolver=spec.arg_role_resolver,
                    ),
                )
    return hints


_ROLE_HINTS: dict[str, CommandSig | SubcommandSig] = _role_hints_from_registry()


def _type_hints_from_registry() -> dict[str, CommandTypeHint | SubcommandTypeHint]:
    """Build TYPE_HINTS from inline return_type/arg_types on CommandSpec/SubCommand."""

    hints: dict[str, CommandTypeHint | SubcommandTypeHint] = {}
    for name, specs in REGISTRY.specs_by_name.items():
        for spec in specs:
            if spec.subcommands:
                sub_hints = {}
                for sub_name, sub in spec.subcommands.items():
                    if sub.return_type is not None or sub.arg_types or sub.arg_type_resolver:
                        sub_hints[sub_name] = CommandTypeHint(
                            return_type=sub.return_type,
                            arg_types=dict(sub.arg_types) if sub.arg_types else {},
                            arg_type_resolver=sub.arg_type_resolver,
                        )
                if sub_hints:
                    hints[name] = SubcommandTypeHint(subcommands=sub_hints)
            elif spec.return_type is not None or spec.arg_types or spec.arg_type_resolver:
                hints[name] = CommandTypeHint(
                    return_type=spec.return_type,
                    arg_types=dict(spec.arg_types) if spec.arg_types else {},
                    arg_type_resolver=spec.arg_type_resolver,
                )
    return hints


# Type hints derived from inline fields on CommandSpec/SubCommand.
TYPE_HINTS: dict[str, CommandTypeHint | SubcommandTypeHint] = _type_hints_from_registry()


# ---------------------------------------------------------------------------
# Constant-fold hints — compile-time evaluators for pure commands
# ---------------------------------------------------------------------------

from .models import ConstFoldFunc  # noqa: E402


def _fold_hints_from_registry() -> tuple[
    dict[str, ConstFoldFunc],
    dict[str, dict[str, ConstFoldFunc]],
]:
    """Build constant-fold callbacks from CommandSpec/SubCommand definitions.

    Returns ``(cmd_folds, subcmd_folds)`` where *cmd_folds* maps command
    names to fold functions and *subcmd_folds* maps command names to dicts
    of ``{subcommand: fold_func}``.
    """
    cmd_folds: dict[str, ConstFoldFunc] = {}
    subcmd_folds: dict[str, dict[str, ConstFoldFunc]] = {}
    for name, specs in REGISTRY.specs_by_name.items():
        for spec in specs:
            if spec.subcommands:
                subs: dict[str, ConstFoldFunc] = {}
                for sub_name, sub in spec.subcommands.items():
                    if sub.const_fold is not None:
                        subs[sub_name] = sub.const_fold
                if subs:
                    subcmd_folds.setdefault(name, {}).update(subs)
            elif spec.const_fold is not None:
                cmd_folds.setdefault(name, spec.const_fold)
    return cmd_folds, subcmd_folds


FOLD_HINTS: dict[str, ConstFoldFunc] = {}
FOLD_SUBCOMMAND_HINTS: dict[str, dict[str, ConstFoldFunc]] = {}


def _rebuild_fold_hints() -> None:
    global FOLD_HINTS, FOLD_SUBCOMMAND_HINTS
    FOLD_HINTS, FOLD_SUBCOMMAND_HINTS = _fold_hints_from_registry()


_rebuild_fold_hints()

# Taint hints from class-per-command definitions.
# Tcl core taint hints are always available; dialect-specific hints are
# merged lazily when the dialect is first loaded.
from dialects.tcl import tcl_taint_hints as _tcl_taint_hints  # noqa: E402

TAINT_HINTS: dict[str, TaintHint] = {**_tcl_taint_hints()}

# Taint hint loaders: (module_name, func_name) for dialects that provide
# taint hints.  Loaded lazily via importlib, mirroring the command spec
# loader pattern in command_registry.py.
_TAINT_HINT_LOADER_SPECS: dict[str, tuple[str, str]] = {
    "f5-irules": ("dialects.f5.irules", "irules_taint_hints"),
}

# Loader keys whose taint hints have already been merged.
_loaded_taint_loaders: set[str] = set()


def _merge_taint_hints_for_loaders(loader_keys: list[str]) -> None:
    """Merge taint hints for newly loaded dialect packs."""
    for key in loader_keys:
        if key in _loaded_taint_loaders:
            continue
        spec = _TAINT_HINT_LOADER_SPECS.get(key)
        if spec is not None:
            mod_name, func_name = spec
            mod = importlib.import_module(mod_name)
            TAINT_HINTS.update(getattr(mod, func_name)())
        _loaded_taint_loaders.add(key)


def _invalidate_runtime_caches(loader_keys: list[str]) -> None:
    """Rebuild derived data after the registry has been expanded."""
    _merge_taint_hints_for_loaders(loader_keys)
    global _ROLE_HINTS
    _ROLE_HINTS = _role_hints_from_registry()
    TYPE_HINTS.clear()
    TYPE_HINTS.update(_type_hints_from_registry())
    _rebuild_fold_hints()
    _rebuild_alias_maps()
    canonical_list_commands.cache_clear()
    taint_transform_map.cache_clear()
    taint_double_encode_map.cache_clear()
    taint_sink_safe_colours.cache_clear()
    regex_pattern_commands.cache_clear()
    storage_type_commands.cache_clear()
    normalized_flag_commands.cache_clear()
    variable_writing_commands.cache_clear()
    loop_list_header_commands.cache_clear()
    scope_alias_commands.cache_clear()
    options_with_value.cache_clear()
    # Drop the per-(dialect, extras) signature cache so it is rebuilt with
    # the freshly-loaded specs on next access.
    signatures_for.cache_clear()

    # Also clear the parser's known-command cache.
    from compiler.parsing.known_commands import known_command_names

    known_command_names.cache_clear()


# Commands that return TclType.LIST but do NOT produce canonical list
# representations.  concat strips one level of grouping from its arguments,
# so its output may contain unquoted specials.
_NON_CANONICAL_LIST_COMMANDS: frozenset[str] = frozenset({"concat"})


@lru_cache(maxsize=1)
def canonical_list_commands() -> frozenset[str]:
    """Return command names whose output is always a canonical Tcl list.

    A canonical list is properly quoted so that re-parsing by ``eval``,
    ``uplevel``, or ``interp eval`` never causes unwanted substitution.

    The set is derived from the command registry: every command (or
    ``"cmd subcmd"`` pair) with ``return_type == TclType.LIST`` is
    included, minus known exceptions in :data:`_NON_CANONICAL_LIST_COMMANDS`.
    """
    names: set[str] = set()
    for name, specs in REGISTRY.specs_by_name.items():
        for spec in specs:
            if spec.subcommands:
                for sub_name, sub in spec.subcommands.items():
                    if sub.return_type is TclType.LIST:
                        full = f"{name} {sub_name}"
                        if full not in _NON_CANONICAL_LIST_COMMANDS:
                            names.add(full)
            elif spec.return_type is TclType.LIST:
                if name not in _NON_CANONICAL_LIST_COMMANDS:
                    names.add(name)
    return frozenset(names)


@lru_cache(maxsize=1)
def taint_transform_map() -> dict[str, TaintColour]:
    """Return ``{command: colour_bits}`` for commands that sanitise tainted data.

    Includes both top-level commands (e.g. ``"URI::encode"``) and
    ``"cmd sub"`` compound keys (e.g. ``"file normalize"``).
    Derived from ``taint_transform`` on :class:`CommandSpec` / :class:`SubCommand`.
    """
    result: dict[str, TaintColour] = {}
    for name, specs in REGISTRY.specs_by_name.items():
        for spec in specs:
            if spec.subcommands:
                for sub_name, sub in spec.subcommands.items():
                    if sub.taint_transform is not None:
                        result[f"{name} {sub_name}"] = sub.taint_transform
            if spec.taint_transform is not None:
                result[name] = spec.taint_transform
    return result


@lru_cache(maxsize=1)
def taint_double_encode_map() -> dict[str, TaintColour]:
    """Return ``{command: colour}`` for T106 double-encoding detection.

    Maps command names to the :class:`TaintColour` that, when already
    present on the input, indicates the data has already been encoded
    in the same way this command would encode it.
    Derived from ``taint_double_encode_colour`` on :class:`CommandSpec` /
    :class:`SubCommand`.
    """
    result: dict[str, TaintColour] = {}
    for name, specs in REGISTRY.specs_by_name.items():
        for spec in specs:
            if spec.subcommands:
                for sub_name, sub in spec.subcommands.items():
                    if sub.taint_double_encode_colour is not None:
                        result[f"{name} {sub_name}"] = sub.taint_double_encode_colour
            if spec.taint_double_encode_colour is not None:
                result[name] = spec.taint_double_encode_colour
    return result


@lru_cache(maxsize=1)
def taint_sink_safe_colours() -> dict[str, TaintColour]:
    """Return ``{command: colour}`` for T100 taint-sink suppression.

    When a tainted value carries the listed colour, the T100 warning
    for this sink command is suppressed (e.g. ``exec`` + ``SHELL_ATOM``).
    Derived from ``taint_sink_safe_colour`` on :class:`CommandSpec`.
    """
    result: dict[str, TaintColour] = {}
    for name, specs in REGISTRY.specs_by_name.items():
        for spec in specs:
            if spec.taint_sink_safe_colour is not None:
                result[name] = spec.taint_sink_safe_colour
    return result


@lru_cache(maxsize=1)
def regex_pattern_commands() -> frozenset[str]:
    """Return command names that take a regex pattern argument.

    Derived from ``pattern_type == PatternType.REGEX`` on :class:`CommandSpec`.
    """
    names: set[str] = set()
    for name, specs in REGISTRY.specs_by_name.items():
        for spec in specs:
            if spec.pattern_type is PatternType.REGEX:
                names.add(name)
    return frozenset(names)


@lru_cache(maxsize=1)
def storage_type_commands() -> dict[str, StorageType]:
    """Return ``{command: StorageType}`` for commands that imply a storage type.

    Used by side-effect analysis to infer whether a variable holds a dict,
    list, or array based on the command that writes to it.
    Derived from ``inferred_storage_type`` on :class:`CommandSpec`.
    """
    result: dict[str, StorageType] = {}
    for name, specs in REGISTRY.specs_by_name.items():
        for spec in specs:
            if spec.inferred_storage_type is not None:
                result[name] = spec.inferred_storage_type
    return result


@lru_cache(maxsize=1)
def normalized_flag_commands() -> frozenset[str]:
    """Return command names that support the ``-normalized`` flag.

    Used by IRULE3102 and side-effect analysis to detect HTTP getters
    that should use ``-normalized`` for consistent matching.
    Derived from the presence of a ``-normalized`` :class:`OptionSpec`
    on the command's forms.
    """
    names: set[str] = set()
    for name, specs in REGISTRY.specs_by_name.items():
        for spec in specs:
            if spec.supports_normalized_flag:
                names.add(name)
    return frozenset(names)


@lru_cache(maxsize=1)
def variable_writing_commands() -> dict[str, int]:
    """Return ``{command: var_arg_index}`` for commands that write to a variable.

    The value is the 0-based argument index (after the command name) where the
    variable name appears.
    Derived from ``assigns_variable_at`` on :class:`CommandSpec`.
    """
    result: dict[str, int] = {}
    for name, specs in REGISTRY.specs_by_name.items():
        for spec in specs:
            if spec.assigns_variable_at is not None:
                result[name] = spec.assigns_variable_at
    return result


@lru_cache(maxsize=1)
def loop_list_header_commands() -> frozenset[str]:
    """Return command names (including ``"cmd sub"`` compounds) whose CFG headers
    carry list-expression args evaluated once before the loop body.

    Derived from ``loop_list_header`` on :class:`CommandSpec` / :class:`SubCommand`.
    """
    names: set[str] = set()
    for name, specs in REGISTRY.specs_by_name.items():
        for spec in specs:
            if spec.loop_list_header:
                names.add(name)
            if spec.subcommands:
                for sub_name, sub in spec.subcommands.items():
                    if sub.loop_list_header:
                        names.add(f"{name} {sub_name}")
    return frozenset(names)


@lru_cache(maxsize=1)
def scope_alias_commands() -> frozenset[str]:
    """Return command names (including ``"cmd sub"`` compounds) that create
    scope aliases (upvar-like variable bindings visible in other scopes).

    Derived from ``creates_scope_alias`` on :class:`CommandSpec` / :class:`SubCommand`.
    """
    names: set[str] = set()
    for name, specs in REGISTRY.specs_by_name.items():
        for spec in specs:
            if spec.creates_scope_alias:
                names.add(name)
            if spec.subcommands:
                for sub_name, sub in spec.subcommands.items():
                    if sub.creates_scope_alias:
                        names.add(f"{name} {sub_name}")
    return frozenset(names)


_EDA_VENDOR_DIALECTS = frozenset(
    {
        "synopsys-eda-tcl",
        "cadence-eda-tcl",
        "xilinx-eda-tcl",
        "intel-quartus-eda-tcl",
        "mentor-eda-tcl",
    }
)

# Map each EDA vendor dialect to its underlying Tcl base version.
# Synopsys DC/PT/ICC2 and Cadence Genus/Innovus/Tempus embed Tcl 8.6.
# Xilinx Vivado, Intel Quartus, and Mentor ModelSim/Questa embed Tcl 8.5.
_EDA_TCL_BASE: dict[str, str] = {
    "synopsys-eda-tcl": "tcl8.6",
    "cadence-eda-tcl": "tcl8.6",
    "xilinx-eda-tcl": "tcl8.5",
    "intel-quartus-eda-tcl": "tcl8.5",
    "mentor-eda-tcl": "tcl8.5",
}

import contextvars  # noqa: E402

from .dialects import KNOWN_DIALECTS as _KNOWN_DIALECTS  # noqa: E402

# Per-request scope variables.  Each LSP request handler is expected to wrap
# its work in ``dialect_scope(dialect, extra_commands=...)`` so that
# ``active_dialect()``, ``SIGNATURES``, and the lexer flags return values
# scoped to the current document's folder rather than a process-wide
# singleton.  See issue #407.
_dialect_var: contextvars.ContextVar[str] = contextvars.ContextVar(
    "tcl_lsp_dialect", default="tcl8.6"
)
_extra_commands_var: contextvars.ContextVar[tuple[str, ...]] = contextvars.ContextVar(
    "tcl_lsp_extra_commands", default=()
)

# Per-analysis overlay for stub command signatures parsed from
# ``# tcl-lsp: stub`` comments and ``.tcl.stubs`` files.  Stubs declared
# in the current source are merged on top of the dialect signature table
# so consumers like ``arg_indices_for_role`` see them as if they were
# first-class registry entries.  Always a frozen mapping; an empty
# default means "no stubs in scope".
_stub_signatures_var: contextvars.ContextVar["dict[str, CommandSig | SubcommandSig]"] = (
    contextvars.ContextVar("tcl_lsp_stub_signatures", default={})
)


class _SignaturesProxy:
    """Mapping-like view of the signature table for the active dialect.

    Resolves on every access via ``signatures_for(dialect, extra)`` which is
    ``lru_cache``-d, so per-dialect lookups are O(1) after warm-up.  Behaves
    like ``dict`` for the read patterns the analyser/compiler/LSP layer
    relies on (``get``, ``__contains__``, ``__getitem__``, iteration,
    ``keys``).  Writes are not supported — the cached per-dialect dicts are
    immutable from the callers' perspective.
    """

    __slots__ = ()

    def _current(self) -> dict[str, "CommandSig | SubcommandSig"]:
        base = signatures_for(_dialect_var.get(), _extra_commands_var.get())
        stubs = _stub_signatures_var.get()
        if not stubs:
            return base
        merged: dict[str, "CommandSig | SubcommandSig"] = dict(base)
        merged.update(stubs)
        return merged

    def __getitem__(self, key: str) -> "CommandSig | SubcommandSig":
        return self._current()[key]

    def __contains__(self, key: object) -> bool:
        return key in self._current()

    def __iter__(self):
        return iter(self._current())

    def __len__(self) -> int:
        return len(self._current())

    def get(self, key, default=None):
        return self._current().get(key, default)

    def keys(self):
        return self._current().keys()

    def values(self):
        return self._current().values()

    def items(self):
        return self._current().items()


SIGNATURES: _SignaturesProxy = _SignaturesProxy()


def _canonical_dialect(dialect: str) -> str | None:
    value = dialect.strip().lower()
    if value in _KNOWN_DIALECTS:
        return value
    return None


def _extra_command_signatures(extra_commands: list[str] | tuple[str, ...]) -> dict[str, CommandSig]:
    extra: dict[str, CommandSig] = {}
    for name in extra_commands:
        cmd = name.strip()
        if not cmd:
            continue
        extra[cmd] = CommandSig()
    return extra


def _with_roles(name: str, sig: CommandSig | SubcommandSig) -> CommandSig | SubcommandSig:
    """Merge role hints onto a validation-derived signature."""
    hint = _ROLE_HINTS.get(name)

    if isinstance(sig, CommandSig):
        if isinstance(hint, CommandSig):
            return CommandSig(
                arity=sig.arity,
                arg_roles=dict(hint.arg_roles),
                arg_role_resolver=hint.arg_role_resolver or sig.arg_role_resolver,
            )
        return sig

    if not isinstance(sig, SubcommandSig):
        return sig

    hint_subs: dict[str, CommandSig] = {}
    if isinstance(hint, SubcommandSig):
        hint_subs = hint.subcommands

    merged_subs: dict[str, CommandSig] = {}
    for sub_name, sub_sig in sig.subcommands.items():
        sub_hint = hint_subs.get(sub_name)
        if isinstance(sub_hint, CommandSig):
            merged_subs[sub_name] = CommandSig(
                arity=sub_sig.arity,
                arg_roles=dict(sub_hint.arg_roles),
                arg_role_resolver=sub_hint.arg_role_resolver or sub_sig.arg_role_resolver,
            )
        else:
            merged_subs[sub_name] = sub_sig

    return SubcommandSig(
        subcommands=merged_subs,
        allow_unknown=sig.allow_unknown,
    )


def _signature_from_validation(validation: ValidationSpec | None) -> CommandSig:
    if validation is None:
        return CommandSig()
    return CommandSig(arity=validation.arity)


def _signature_from_spec(spec: "CommandSpec") -> CommandSig | SubcommandSig:
    """Derive a signature from a CommandSpec, preferring SubCommand data.

    For commands with subcommands: reads arg_roles from SubCommand objects.
    For simple commands: reads arg_roles from CommandSpec.arg_roles,
    falling back to _signature_from_validation when empty.
    """
    # Collect declared option names from all forms for option-aware arity.
    opts = frozenset(spec.switch_names()) if spec.forms else frozenset()

    if spec.subcommands:
        return SubcommandSig(
            subcommands={
                sub_name: CommandSig(
                    arity=sub.arity,
                    arg_roles=dict(sub.arg_roles) if sub.arg_roles else {},
                    arg_role_resolver=sub.arg_role_resolver,
                )
                for sub_name, sub in spec.subcommands.items()
            },
            allow_unknown=spec.allow_unknown_subcommands,
        )
    # Simple command: use inline arg_roles if available.
    arity = spec.validation.arity if spec.validation else Arity()
    if spec.arg_roles or spec.arg_role_resolver:
        return CommandSig(
            arity=arity,
            arg_roles=dict(spec.arg_roles) if spec.arg_roles else {},
            arg_role_resolver=spec.arg_role_resolver,
            leading_options=opts,
        )
    sig = _signature_from_validation(spec.validation)
    if opts:
        return CommandSig(arity=sig.arity, arg_roles=sig.arg_roles, leading_options=opts)
    return sig


def _registry_signatures_for_dialect(dialect: str) -> dict[str, CommandSig | SubcommandSig]:
    signatures: dict[str, CommandSig | SubcommandSig] = {}
    for name in REGISTRY.command_names(dialect):
        # Skip tcllib commands — they are handled separately.
        if REGISTRY.is_tcllib_command(name):
            continue
        spec = REGISTRY.get(name, dialect)
        if spec is not None:
            sig = _signature_from_spec(spec)
        else:
            sig = _signature_from_validation(REGISTRY.validation(name, dialect))
        signatures[name] = _with_roles(name, sig)
    return signatures


def _registry_signatures_for_tcllib() -> dict[str, CommandSig | SubcommandSig]:
    """Build signatures for all tcllib commands from registry specs."""
    signatures: dict[str, CommandSig | SubcommandSig] = {}
    for name in REGISTRY.all_tcllib_command_names():
        spec = REGISTRY.get_any(name)
        if spec is not None:
            sig = _signature_from_spec(spec)
        else:
            sig = _signature_from_validation(REGISTRY.validation(name))
        signatures[name] = _with_roles(name, sig)
    return signatures


def _build_signatures(
    dialect: str,
    *,
    extra_commands: list[str] | tuple[str, ...],
) -> dict[str, CommandSig | SubcommandSig]:
    signatures: dict[str, CommandSig | SubcommandSig]

    match dialect:
        case "tcl8.4" | "tcl8.5" | "tcl8.6" | "tcl9.0":
            signatures = _registry_signatures_for_dialect(dialect)
        case "f5-irules":
            signatures = _registry_signatures_for_dialect("tcl8.6")
            signatures.update(_registry_signatures_for_dialect("f5-irules"))
        case "f5-iapps":
            signatures = _registry_signatures_for_dialect("tcl8.6")
            signatures.update(_registry_signatures_for_dialect("f5-iapps"))
        case "f5-tmsh":
            signatures = _registry_signatures_for_dialect("tcl8.6")
            signatures.update(_registry_signatures_for_dialect("f5-tmsh"))
        case "f5-bigip":
            signatures = _registry_signatures_for_dialect("tcl8.6")
            signatures.update(_registry_signatures_for_dialect("f5-bigip"))
        case d if d in _EDA_VENDOR_DIALECTS:
            signatures = _registry_signatures_for_dialect(_EDA_TCL_BASE[d])
            # EDA vendor commands (SDC base + vendor-specific) are registered
            # as CommandSpecs in the central registry and resolved via the
            # standard dialect-filtering path; overlay them here.
            signatures.update(_registry_signatures_for_dialect(d))
        case "expect":
            signatures = _registry_signatures_for_dialect("tcl8.6")
            signatures.update(_registry_signatures_for_dialect("expect"))
        case _:
            # Guard for defensive callers; configure_signatures should validate.
            return {}

    # Tcllib commands are always included in SIGNATURES because their
    # namespaced names (e.g. json::json2dict) don't collide with core Tcl.
    # Per-document filtering happens in the feature layer (completion,
    # hover, diagnostics) by checking ``package require`` statements.
    signatures.update(_registry_signatures_for_tcllib())

    signatures.update(_extra_command_signatures(extra_commands))
    return signatures


def available_dialects() -> list[str]:
    """Return canonical dialect profile names."""
    return sorted(_KNOWN_DIALECTS)


def active_signature_profile() -> SignatureProfile:
    """Return the currently active command-signature profile."""
    return {
        "dialect": _dialect_var.get(),
        "extra_commands": list(_extra_commands_var.get()),
    }


def is_irules_dialect() -> bool:
    """Return True if the active dialect is iRules."""
    return _dialect_var.get() == "f5-irules"


@lru_cache(maxsize=32)
def signatures_for(
    dialect: str,
    extra_commands: tuple[str, ...] = (),
) -> dict[str, CommandSig | SubcommandSig]:
    """Return the signature table for ``dialect`` (+ ``extra_commands``).

    Cached so per-request lookups via ``SIGNATURES.get(...)`` are O(1) after
    each (dialect, extras) tuple is built once.  The cache is invalidated
    when new specs are loaded via ``_invalidate_runtime_caches``.
    """
    canonical = _canonical_dialect(dialect)
    if canonical is None:
        return {}
    # Ensure dialect-specific command specs are loaded before building.
    REGISTRY.load_dialect_specs(canonical)
    return _build_signatures(canonical, extra_commands=extra_commands)


def configure_signatures(
    *,
    dialect: str | None = None,
    extra_commands: list[str] | tuple[str, ...] | None = None,
) -> bool:
    """Configure the dialect/extra_commands for the current context.

    Returns ``True`` if the effective profile changed.

    This sets the ContextVar values in the *current* context.  For
    per-request scoping (the common case in the LSP), prefer
    :func:`shared.dialect.dialect_scope` which uses a
    ``with``-statement and restores the previous value on exit.
    ``configure_signatures`` remains as the long-lived default-setter
    used at server startup and on workspace-level configuration changes.
    """
    cur_dialect = _dialect_var.get()
    cur_extra = _extra_commands_var.get()

    if dialect is None:
        next_dialect = cur_dialect
    else:
        requested = _canonical_dialect(dialect)
        if requested is None:
            return False
        next_dialect = requested
    if extra_commands is None:
        next_extra = cur_extra
    else:
        next_extra = tuple(
            sorted({name.strip() for name in extra_commands if name and name.strip()})
        )

    if next_dialect == cur_dialect and next_extra == cur_extra:
        return False

    # Eagerly populate the signature cache for the new (dialect, extras)
    # tuple so callers reading ``SIGNATURES`` don't pay the build cost on
    # the first request.  This also ensures dialect-specific command specs
    # are loaded; taint hints merge in via the _on_specs_loaded callback.
    signatures_for(next_dialect, next_extra)

    _dialect_var.set(next_dialect)
    _extra_commands_var.set(next_extra)

    # Lexer flags (``{*}`` word expansion, iRules ``}{`` brace-word
    # separator) are read per-call from ``_dialect_var`` via
    # :func:`compiler.parsing.lexer._expand_syntax_active` /
    # :func:`_irules_brace_separator_active`, so no class-attribute
    # mutation is needed here.

    return True


_STUB_ROLE_MAP: dict[str, ArgRole] = {
    "body": ArgRole.BODY,
    "expr": ArgRole.EXPR,
    "var": ArgRole.VAR_WRITE,
    "var_read": ArgRole.VAR_READ,
    "pattern": ArgRole.PATTERN,
    "channel": ArgRole.CHANNEL,
    "name": ArgRole.NAME,
}


def stub_to_command_sig(stub: "StubCommandDef") -> CommandSig:
    """Build a :class:`CommandSig` from a user-declared stub.

    Argument roles declared on the stub (``body``, ``expr``, ``var``,
    ``var_read``, ``pattern``, ``channel``, ``name``) are translated to
    the matching :class:`ArgRole` so consumers like
    :func:`arg_indices_for_role` and the call-graph scanner see stubbed
    commands as if they were registry entries.

    Optional arguments (``?arg?``) and the variadic ``args`` tail are
    honoured: when a stub has optional args before a role-bearing one
    (e.g. ``{sql ?rowvar? script:body}``), the returned signature has
    a dynamic ``arg_role_resolver`` that shifts the role indices based
    on the actual call's argument count.  This makes positional
    stubs work for commands like ``db eval ?-withoutnulls? sql ?array?
    script`` where the BODY arg's index moves with the optionals.
    """
    # Filter out the variadic ``args`` tail — it accepts zero or more
    # arguments and contributes nothing to arity or roles.
    positional = [a for a in stub.args if a.name != "args"]
    has_variadic = len(positional) != len(stub.args)

    min_required = sum(1 for a in positional if not a.optional)
    max_args: int | None
    if has_variadic:
        max_args = None
    else:
        max_args = len(positional)

    arity = Arity(min_required, max_args) if max_args is not None else Arity(min_required)

    # Static role map — used when there are no optional positional
    # args, so the declared indices match the actual argument indices.
    static_roles: dict[int, frozenset[ArgRole]] = {}
    has_optional = any(a.optional for a in positional)
    for idx, arg in enumerate(positional):
        role = _STUB_ROLE_MAP.get(arg.role)
        if role is not None:
            static_roles[idx] = frozenset({role})

    if not has_optional:
        return CommandSig(arity=arity, arg_roles=static_roles)

    # Build a dynamic resolver that walks the actual argument list and
    # decides which optional slots are filled (left-to-right).
    declared = tuple(positional)
    role_lookup = tuple(_STUB_ROLE_MAP.get(a.role) for a in positional)

    def _resolver(call_args: list[str]) -> dict[int, frozenset[ArgRole]]:
        n = len(call_args)
        n_required = sum(1 for a in declared if not a.optional)
        n_optional = len(declared) - n_required
        # How many optionals are actually present, left-to-right.
        extras = max(0, n - n_required)
        opts_present = min(extras, n_optional)

        result: dict[int, frozenset[ArgRole]] = {}
        actual_idx = 0
        opts_filled = 0
        for decl_idx, arg in enumerate(declared):
            if arg.optional:
                if opts_filled >= opts_present:
                    # This optional is absent — its declared role does
                    # not map to any actual argument position.
                    continue
                opts_filled += 1
            role = role_lookup[decl_idx]
            if role is not None and actual_idx < n:
                result[actual_idx] = frozenset({role})
            actual_idx += 1
        return result

    return CommandSig(arity=arity, arg_roles=static_roles, arg_role_resolver=_resolver)


def signatures_from_stubs(
    stubs: "Iterable[StubCommandDef]",
) -> dict[str, CommandSig | SubcommandSig]:
    """Convert a list of :class:`StubCommandDef` to a signature overlay.

    Stubs with a ``subcommand`` field are folded into a single
    :class:`SubcommandSig` keyed by the command name so an ensemble
    like ``db`` can carry separate signatures for ``db eval``,
    ``db transaction``, ``db function``, etc.  Plain stubs (no
    subcommand) produce a :class:`CommandSig` directly.

    Each declared name is registered under both its qualified
    (``::foo::bar``) and unqualified (``foo::bar``) spellings so
    :func:`_resolve_arg_roles` finds the overlay whether the call
    site writes the bare or fully-qualified form.
    """
    # First pass: group stubs by base command name so we can decide
    # CommandSig vs SubcommandSig per group.
    grouped: dict[str, list[StubCommandDef]] = {}
    for stub in stubs:
        bare = stub.name.lstrip(":")
        grouped.setdefault(bare, []).append(stub)

    overlay: dict[str, CommandSig | SubcommandSig] = {}
    for bare, group in grouped.items():
        subcommand_stubs = [s for s in group if s.subcommand]
        plain_stubs = [s for s in group if not s.subcommand]

        sig: CommandSig | SubcommandSig
        if subcommand_stubs:
            sub_sigs: dict[str, CommandSig] = {}
            for sub_stub in subcommand_stubs:
                sub_sigs[sub_stub.subcommand or ""] = stub_to_command_sig(sub_stub)
            sig = SubcommandSig(subcommands=sub_sigs, allow_unknown=bool(plain_stubs))
            # A plain stub for the same name is taken as the fallback
            # for unknown subcommands — represent it by leaving
            # ``allow_unknown=True`` so the analyser doesn't fire
            # an "unknown subcommand" diagnostic; the fallback sig
            # isn't attached because :class:`SubcommandSig` doesn't
            # model a default arm.  Users wanting a fully untyped
            # fallback should drop the subcommand stubs.
        else:
            # Either one plain stub or multiple plain stubs (the last
            # wins; duplicate plain stubs for the same name are
            # already a user authoring error).
            sig = stub_to_command_sig(plain_stubs[-1])

        overlay[bare] = sig
        # Also store the fully-qualified spelling.
        qualified = bare if bare.startswith("::") else f"::{bare}"
        overlay[qualified] = sig
    return overlay


@contextlib.contextmanager
def stub_signature_scope(
    stubs: "Iterable[StubCommandDef]",
):
    """Context manager that overlays *stubs* on the active signature table.

    All ``SIGNATURES.get(...)`` reads (and therefore
    :func:`arg_indices_for_role`, :func:`resolve_arg_role_map`,
    :func:`body_arg_indices`, and friends) see the stubs for the
    duration of the ``with`` block.  Restores the previous overlay on
    exit — safe to nest.
    """
    overlay = signatures_from_stubs(stubs)
    if not overlay:
        # Fast path: nothing to scope.
        yield
        return
    previous = _stub_signatures_var.get()
    merged = dict(previous) if previous else {}
    merged.update(overlay)
    token = _stub_signatures_var.set(merged)
    try:
        yield
    finally:
        _stub_signatures_var.reset(token)


def _oo_definition_body_indices(command: str, args: list[str]) -> set[int]:
    """Return BODY argument indices for TclOO definition-script commands."""
    if command == "constructor" and len(args) >= 2:
        return {1}
    if command == "destructor" and len(args) >= 1:
        return {0}
    if command == "method" and len(args) >= 3:
        return {len(args) - 1}
    if command == "classmethod" and len(args) >= 3:
        return {len(args) - 1}
    if command in ("initialise", "initialize") and len(args) >= 1:
        return {0}  # initialise script
    if command == "private" and len(args) >= 1:
        return {0}  # private script (block form)
    if command == "self" and args:
        subcommand = args[0]
        if subcommand == "constructor" and len(args) >= 3:
            return {2}
        if subcommand == "destructor" and len(args) >= 2:
            return {1}
        if subcommand == "method" and len(args) >= 4:
            return {len(args) - 1}
        if subcommand == "classmethod" and len(args) >= 4:
            return {len(args) - 1}
    if command == "property":
        result: set[int] = set()
        for i in range(len(args) - 1):
            if args[i] in ("-set", "-get"):
                result.add(i + 1)
        return result
    return set()


def _skip_switch_options(args: list[str]) -> int:
    """Skip option flags and the switch value arg, return next index."""
    value_opts = options_with_value("switch")
    i = skip_options(args, value_opts)
    if i < len(args):
        i += 1
    return i


def is_switch_case_list_form(args: list[str]) -> bool:
    """Return True when *args* (after command name) use the braced case-list form.

    In the braced form, all patterns and bodies are packed inside a single
    trailing argument.  In the non-braced form they appear as separate args.
    """
    i = _skip_switch_options(args)
    return i < len(args) and i == len(args) - 1


@dataclass(frozen=True, slots=True)
class SwitchCase:
    """A pattern/body pair from a ``switch`` braced case list."""

    pattern: str
    """The match pattern text (e.g. ``*.gif``, ``default``)."""

    body: str | None
    """Body script text, or ``None`` for fallthrough (``-``)."""

    is_braced: bool
    """Whether the body was a braced word ``{...}``."""

    pattern_token: Token
    """First token of the pattern word."""

    body_token: Token | None
    """First token of the body word; for fallthrough this is the ``-`` token."""


def iter_switch_case_list(
    case_list_text: str,
    *,
    base_offset: int = 0,
    base_line: int = 0,
    base_col: int = 0,
) -> Iterator[SwitchCase]:
    """Parse a ``switch`` braced case list into pattern/body pairs.

    Yields :class:`SwitchCase` for each pair, handling ``-`` fallthrough.

    *case_list_text* is the content inside the outer braces of the case
    list (i.e. the ``Token.text`` of the STR token, not including the
    surrounding ``{`` ``}``).
    """
    from compiler.parsing.lexer import TclLexer
    from compiler.parsing.tokens import TokenType

    lexer = TclLexer(
        case_list_text,
        base_offset=base_offset,
        base_line=base_line,
        base_col=base_col,
    )
    # Collect words with their raw source spans so we preserve
    # original quoting/bracing (e.g. $var keeps its $, braced
    # patterns keep their braces).
    #
    # Each entry: (raw_text, inner_text, first_token, is_braced, start_offset, end_offset)
    word_starts: list[int] = []
    word_ends: list[int] = []
    word_tokens: list[Token] = []
    word_inner: list[str] = []  # tok.text (content without delimiters)
    word_braced: list[bool] = []
    prev_type = TokenType.EOL

    while True:
        tok = lexer.get_token()
        if tok is None:
            break
        if tok.type in (TokenType.SEP, TokenType.EOL, TokenType.COMMENT):
            prev_type = tok.type
            continue
        if prev_type in (TokenType.SEP, TokenType.EOL, TokenType.COMMENT):
            word_starts.append(tok.start.offset - base_offset)
            word_ends.append(tok.end.offset - base_offset + 1)
            word_tokens.append(tok)
            word_inner.append(tok.text)
            word_braced.append(tok.type == TokenType.STR)
        elif word_ends:
            word_ends[-1] = tok.end.offset - base_offset + 1
            word_inner[-1] += tok.text
        else:
            word_starts.append(tok.start.offset - base_offset)
            word_ends.append(tok.end.offset - base_offset + 1)
            word_tokens.append(tok)
            word_inner.append(tok.text)
            word_braced.append(tok.type == TokenType.STR)
        prev_type = tok.type

    idx = 0
    while idx + 1 < len(word_starts):
        # Use the raw source span for the pattern so quoting is preserved.
        raw_pattern = case_list_text[word_starts[idx] : word_ends[idx]]
        body_inner = word_inner[idx + 1]
        if body_inner == "-":
            yield SwitchCase(
                pattern=raw_pattern,
                body=None,
                is_braced=False,
                pattern_token=word_tokens[idx],
                body_token=word_tokens[idx + 1],
            )
        else:
            yield SwitchCase(
                pattern=raw_pattern,
                body=body_inner,
                is_braced=word_braced[idx + 1],
                pattern_token=word_tokens[idx],
                body_token=word_tokens[idx + 1],
            )
        idx += 2


def regexp_pattern_index(args: list[str] | tuple[str, ...]) -> int | None:
    """Return the pattern argument index for regexp/regsub (0-based after cmd).

    Skips over option switches (``-nocase``, ``-start N``, ``--``, etc.)
    to find the first positional argument which is the regex pattern.

    *args* should **not** include the command name itself.
    """
    i = skip_options(args, options_with_value("regexp"))
    if i < len(args):
        return i
    return None


@lru_cache(maxsize=None)
def options_with_value(command: str) -> frozenset[str]:
    """Return the set of option names that consume a following value argument.

    Derived from ``OptionSpec.takes_value`` on the command's first form.
    The result is cached since the registry is immutable after initialisation.
    """
    spec = REGISTRY.get_any(command)
    if spec is None or not spec.forms:
        return frozenset()
    return frozenset(opt.name for opt in spec.forms[0].options if opt.takes_value)


def skip_options(
    args: list[str] | tuple[str, ...],
    value_options: frozenset[str] | None = None,
) -> int:
    """Return the index of the first non-option argument.

    Scans *args* (0-based, command name excluded) skipping ``-option`` flags
    and their values.  *value_options* is the set of options that consume a
    following value argument; if ``None``, only the ``--`` terminator is
    recognised.
    """
    if value_options is None:
        value_options = frozenset()
    i = 0
    while i < len(args):
        arg = args[i]
        if arg == "--":
            i += 1
            break
        if arg.startswith("-"):
            i += 1
            if arg in value_options and i < len(args):
                i += 1
            continue
        break
    return i


# Rewrite-alias and exported-short-name maps.
#
# The CFG lowering pass rewrites a few ensemble-form commands (``dict for``,
# ``dict map``) into qualified internal names so codegen can dispatch them
# uniformly; ``namespace import`` brings exported short names into scope
# (``test`` → ``::tcltest::test`` in any namespace that imports
# ``::tcltest::*`` or ``::tcltest::test``).  Downstream registry consumers
# see the alternative spelling and need a reverse-lookup map to recover the
# canonical spec — without it, every consumer would have to hardcode the
# correspondence (the regression mode that #234, #236, and #243 stemmed
# from).
#
# Both maps are derived from declarative spec properties — never hardcoded
# here — so adding a new rewritten name or exported short name is a single
# edit on the source spec.  ``_invalidate_runtime_caches`` rebuilds the
# maps whenever a dialect's specs are loaded.
#
# **Caveat for the exported-short-name map.**  ``namespace import`` is
# dynamic and namespace-scoped in real Tcl — knowing whether a bare ``test``
# at a given call site refers to ``::tcltest::test`` requires tracking the
# imports active in the current namespace.  Issue #246's broader plan
# stages this canonicalisation into the IR.  Until then, the map below is
# a static over-approximation: it assumes every exported short name is
# imported.  This matches today's analyser behaviour and avoids regressing
# tcltest-style packages where the bare name is the conventional spelling,
# but at the cost of treating a user-defined ``test`` proc as if it were
# ``::tcltest::test`` for body-kind/role queries.
_REWRITE_ALIASES: dict[str, tuple[str, str]] = {}
_COMMAND_NAME_ALIASES: dict[str, str] = {}


def _build_rewrite_aliases() -> dict[str, tuple[str, str]]:
    """Collect ``cfg_rewrite_name`` declarations into a reverse-lookup map."""
    result: dict[str, tuple[str, str]] = {}
    for name, specs in REGISTRY.specs_by_name.items():
        for spec in specs:
            for sub_name, sub in spec.subcommands.items():
                if sub.cfg_rewrite_name is not None:
                    # Last writer wins on duplicates — there should only be
                    # one rewriter per qualified name in practice.
                    result[sub.cfg_rewrite_name] = (name, sub_name)
    return result


def _build_command_name_aliases() -> dict[str, str]:
    """Collect bare names that ``is_namespace_exported`` specs may resolve to.

    For each spec at qualified name ``<ns>::<bare>`` with
    ``is_namespace_exported=True``, the bare ``<bare>`` is registered as
    a possible alias for the qualified spec.  This is a static
    over-approximation — see the module-level note for details.
    """
    result: dict[str, str] = {}
    for name, specs in REGISTRY.specs_by_name.items():
        for spec in specs:
            if not spec.is_namespace_exported:
                continue
            if "::" not in name:
                continue
            bare = name.rsplit("::", 1)[-1]
            if bare and bare != name:
                # Last writer wins — duplicate exports across namespaces
                # collapse to whichever spec was registered last.  The
                # bare-name collision is a static-analysis ambiguity in
                # any case; per-namespace IR canonicalisation resolves it.
                result[bare] = name
    return result


def _rebuild_alias_maps() -> None:
    global _REWRITE_ALIASES, _COMMAND_NAME_ALIASES
    _REWRITE_ALIASES = _build_rewrite_aliases()
    _COMMAND_NAME_ALIASES = _build_command_name_aliases()


_rebuild_alias_maps()


def resolve_rewrite_alias(command: str) -> tuple[str, str] | None:
    """Resolve a CFG-rewritten command name to ``(source_cmd, subcommand)``.

    Returns ``None`` for any command that is not a registered rewrite of an
    ensemble-form command.  Callers that branch on the rewritten name should
    consult this map first so a future rewrite addition is automatically
    visible to every registry-driven query.
    """
    return _REWRITE_ALIASES.get(command)


def _canonicalise_command_name(command: str) -> str:
    """Resolve a :data:`_COMMAND_NAME_ALIASES` entry to its source spec name.

    Returns *command* unchanged when no alias is registered.
    """
    return _COMMAND_NAME_ALIASES.get(command, command)


def _resolve_arg_roles(command: str, args: list[str]) -> tuple[dict[int, frozenset[ArgRole]], int]:
    """Return ``(role_map, base_index)`` for a *command* invocation.

    *role_map* maps argument indices into ``args[base_index:]`` to a
    ``frozenset`` of :class:`ArgRole` values.  ``base_index`` is the offset
    to add when expressing the indices against the original *args* list —
    1 for ensemble-form commands (where ``args[0]`` is the subcommand word)
    and 0 for plain commands and CFG-rewritten ensemble names.

    Returns ``({}, 0)`` for unregistered or unmatched commands; callers
    should treat the empty role map as "no roles known" and skip output.

    Resolution order matters: a direct registry hit on *command* always
    wins over a namespace-export alias entry — so an explicit
    ``configure_signatures(extra_commands=["test"])`` registration (or
    any other dialect/extra-command spec for the bare name) shadows the
    static ``test`` → ``tcltest::test`` over-approximation.  Only when
    no spec exists for the user-written name does the alias map come
    into play.  CFG-rewritten names (``::tcl::dict::for``) sit outside
    this rule because they're qualified internal spellings that never
    have a direct spec of their own.
    """
    alias = _REWRITE_ALIASES.get(command)
    if alias is not None:
        source_cmd, sub_name = alias
        parent = SIGNATURES.get(source_cmd) or _ROLE_HINTS.get(source_cmd)
        if isinstance(parent, SubcommandSig):
            sub_sig = parent.subcommands.get(sub_name)
            if sub_sig is not None:
                return _resolved_role_map(sub_sig, args), 0
        return {}, 0

    sig = SIGNATURES.get(command) or _ROLE_HINTS.get(command)
    if sig is None:
        canonical = _canonicalise_command_name(command)
        if canonical != command:
            sig = SIGNATURES.get(canonical) or _ROLE_HINTS.get(canonical)
    if isinstance(sig, SubcommandSig):
        if not args:
            return {}, 1
        sub_sig = sig.subcommands.get(args[0])
        if sub_sig is None:
            return {}, 1
        return _resolved_role_map(sub_sig, args[1:]), 1
    if isinstance(sig, CommandSig):
        return _resolved_role_map(sig, args), 0
    return {}, 0


def _resolved_role_map(sig: CommandSig, args: list[str]) -> dict[int, frozenset[ArgRole]]:
    """Resolve :class:`CommandSig` roles for *args*, dynamic resolver first."""
    if sig.arg_role_resolver is not None:
        resolved = sig.arg_role_resolver(list(args))
        return {idx: r for idx, r in resolved.items() if idx < len(args)}
    return {idx: r for idx, r in sig.arg_roles.items() if idx < len(args)}


def body_kind_for_command(command: str, args: list[str]) -> BodyKind:
    """Return the :class:`BodyKind` for ``ArgRole.BODY`` arguments of *command*.

    Subcommand-form invocations (``dict for``, ``namespace eval``, …) inherit
    from the subcommand's ``body_kind``; CFG-rewritten ensemble forms
    (``::tcl::dict::for``) resolve through :data:`_REWRITE_ALIASES` to the
    source ensemble's subcommand; and bare names that match an
    ``is_namespace_exported`` spec (``test`` → ``tcltest::test``) resolve
    through :data:`_COMMAND_NAME_ALIASES`.  See the module note above for
    the over-approximation in the latter case.

    Defaults to :class:`BodyKind.INLINE` when the command has no body or
    isn't registered.
    """
    alias = _REWRITE_ALIASES.get(command)
    if alias is not None:
        source_cmd, sub_name = alias
        spec = REGISTRY.get_any(source_cmd)
        if spec is not None:
            sub = spec.subcommands.get(sub_name)
            if sub is not None:
                return sub.body_kind
        return BodyKind.INLINE

    # If the user-written name has any direct registration — a real
    # spec, a role hint, or an ``extra_commands`` stub in SIGNATURES —
    # commit to that name and never fall through to the namespace-export
    # alias.  Otherwise an explicit ``configure_signatures(extra_commands=
    # ["test"])`` would silently get tcltest::test's structural body kind.
    if command in SIGNATURES or command in _ROLE_HINTS:
        target = command
    else:
        target = _canonicalise_command_name(command)

    spec = REGISTRY.get_any(target)
    if spec is None:
        return BodyKind.INLINE
    if spec.subcommands and args:
        sub = spec.subcommands.get(args[0])
        if sub is not None:
            return sub.body_kind
    return spec.body_kind


def body_arg_implicit_args_for_command(command: str, args: list[str]) -> int:
    """Return the number of runtime-appended args for *command*'s BODY arg.

    Mirrors the spec/subcommand resolution used by
    :func:`body_kind_for_command`.  Returns 0 for unregistered
    commands or commands whose ``BODY`` arg is a literal script (the
    default).  A positive value signals that the body is invoked as a
    command prefix with that many extra args appended at runtime —
    arity checks on the first top-level command of the body should
    account for the implicit args.
    """
    alias = _REWRITE_ALIASES.get(command)
    if alias is not None:
        source_cmd, sub_name = alias
        spec = REGISTRY.get_any(source_cmd)
        if spec is not None:
            sub = spec.subcommands.get(sub_name)
            if sub is not None:
                return sub.body_arg_implicit_args
        return 0

    if command in SIGNATURES or command in _ROLE_HINTS:
        target = command
    else:
        target = _canonicalise_command_name(command)

    spec = REGISTRY.get_any(target)
    if spec is None:
        return 0
    if spec.subcommands and args:
        sub = spec.subcommands.get(args[0])
        if sub is not None:
            return sub.body_arg_implicit_args
    return spec.body_arg_implicit_args


def arg_indices_for_role(command: str, args: list[str], role: ArgRole) -> set[int]:
    """Return argument indices (0-based, after command name) for a role."""
    if role is ArgRole.BODY:
        # OO definition subcommands are context-sensitive and cannot be
        # registered in SIGNATURES (a user proc named "method" outside OO
        # context would be misidentified).
        oo_body = _oo_definition_body_indices(command, args)
        if oo_body:
            return oo_body
    if role is ArgRole.PATTERN:
        if command in ("regexp", "regsub"):
            idx = regexp_pattern_index(args)
            if idx is not None:
                return {idx}
            return set()

    role_map, base_index = _resolve_arg_roles(command, args)
    if not role_map:
        return set()

    return {idx + base_index for idx, roles in role_map.items() if role in roles}


def resolve_arg_role_map(command: str, args: list[str]) -> dict[int, frozenset[ArgRole]]:
    """Return ``{arg_index: frozenset[ArgRole]}`` for *command* with *args*.

    Indices are expressed against the original *args* list (after the
    command name).  Subcommand offsets are pre-applied so callers don't
    need to know whether *command* is a plain command or an ensemble.

    Includes the same special-case role logic that
    :func:`arg_indices_for_role` applies on top of the registry data:
    TclOO definition subcommands (``method``/``constructor``/...) get
    their context-sensitive ``BODY`` index, and ``regexp``/``regsub``
    pick up the ``PATTERN`` index.  Unregistered commands return an
    empty dict.  Indices not listed by the registry default to "no
    declared roles" — this map does not synthesise ``ArgRole.VALUE``
    entries for unannotated arguments.
    """
    role_map, base_index = _resolve_arg_roles(command, args)
    result: dict[int, frozenset[ArgRole]] = {}
    if role_map:
        if base_index == 0:
            result.update(role_map)
        else:
            for idx, roles in role_map.items():
                result[idx + base_index] = roles

    oo_body = _oo_definition_body_indices(command, args)
    for idx in oo_body:
        result[idx] = result.get(idx, frozenset()) | {ArgRole.BODY}

    if command in ("regexp", "regsub"):
        idx = regexp_pattern_index(args)
        if idx is not None:
            result[idx] = result.get(idx, frozenset()) | {ArgRole.PATTERN}

    return result


_SPECIAL_ROLES = frozenset({ArgRole.BODY, ArgRole.PATTERN})


def arg_indices_for_roles(
    command: str,
    args: list[str],
    roles: tuple[ArgRole, ...],
) -> tuple[set[int], ...]:
    """Return argument indices for multiple roles with a single signature lookup.

    This avoids the repeated ``SIGNATURES.get(command)`` call that
    ``arg_indices_for_role`` incurs per role.  Returns a tuple of
    ``set[int]`` in the same order as *roles*.
    """
    results: list[set[int]] = []

    # Roles with special-case logic must still go through the per-role function
    # because they short-circuit before consulting SIGNATURES.
    need_sig_roles: list[tuple[int, ArgRole]] = []
    for i, role in enumerate(roles):
        if role in _SPECIAL_ROLES:
            results.append(arg_indices_for_role(command, args, role))
        else:
            results.append(set())  # placeholder
            need_sig_roles.append((i, role))

    if not need_sig_roles:
        return tuple(results)

    role_map, base_index = _resolve_arg_roles(command, args)
    if not role_map:
        return tuple(results)

    for idx_in_result, role in need_sig_roles:
        results[idx_in_result] = {
            idx + base_index for idx, roles in role_map.items() if role in roles
        }

    return tuple(results)


def body_arg_indices(command: str, args: list[str]) -> set[int]:
    """Return BODY argument indices for *command* given args after command name."""
    return arg_indices_for_role(command, args, ArgRole.BODY)


def expr_arg_indices(command: str, args: list[str]) -> set[int]:
    """Return EXPR argument indices for *command* given args after command name."""
    return arg_indices_for_role(command, args, ArgRole.EXPR)


# Body argument iteration


@dataclass(frozen=True, slots=True)
class BodyArgument:
    """A validated body argument from a Tcl command.

    Yielded by :func:`iter_body_arguments` after bounds-checking.
    """

    index: int
    """0-based argument index (after command name)."""

    text: str
    """The body text (``args[index]``)."""

    token: Token
    """The token for this argument (``arg_tokens[index]``)."""


def iter_body_arguments(
    cmd_name: str,
    args: list[str],
    arg_tokens: list[Token],
    *,
    prepend_n: int = 0,
) -> Iterator[BodyArgument]:
    """Yield validated :class:`BodyArgument` entries for *cmd_name*.

    Resolves ``ArgRole.BODY`` indices via :func:`arg_indices_for_role` and
    yields one ``BodyArgument`` per index that is within bounds of both
    *args* and *arg_tokens*.  Indices are yielded in ascending order.

    When *prepend_n* > 0 (alias resolution with prepended arguments),
    *args* contains the virtual argument list but *arg_tokens* only
    covers the real (non-prepended) arguments.  Virtual indices are
    mapped back by subtracting *prepend_n* before accessing *arg_tokens*.

    Callers should apply any further filtering they need (e.g. checking
    ``body.token.type is TokenType.STR`` or ``body.text.strip()``).
    """
    for virtual_idx in sorted(arg_indices_for_role(cmd_name, args, ArgRole.BODY)):
        if virtual_idx >= len(args):
            continue
        real_idx = virtual_idx - prepend_n
        if real_idx < 0 or real_idx >= len(arg_tokens):
            continue
        yield BodyArgument(index=real_idx, text=args[virtual_idx], token=arg_tokens[real_idx])


# Register the cache-invalidation callback so that
# CommandRegistry.load_dialect_specs() can notify us.
import compiler.registry.command_registry as _cmd_reg  # noqa: E402

_cmd_reg._on_specs_loaded = _invalidate_runtime_caches

# Eagerly populate the signature cache for the default profile so the first
# request doesn't pay the build cost.  Per-request scoping via
# ``dialect_scope`` (see shared.dialect) overrides this default.
signatures_for("tcl8.6", ())
