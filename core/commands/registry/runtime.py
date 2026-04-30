"""Registry runtime: dialect profile + role-aware signature resolution.

Command inventory, hover/completion metadata, and baseline arity come from the
registry command specs. This module owns runtime profile state and arg-role
semantics needed by analysis/formatting/compiler passes.
"""

from __future__ import annotations

import importlib
from collections.abc import Iterator
from dataclasses import dataclass
from functools import lru_cache

from ...compiler.types import TclType
from ...parsing.tokens import Token
from .command_registry import REGISTRY
from .models import CommandSpec, PatternType, ValidationSpec
from .signatures import ArgRole, Arity, BodyKind, CommandSig, SubcommandSig
from .taint_hints import TaintColour, TaintHint
from .type_hints import CommandTypeHint, SubcommandTypeHint

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
from .tcl import tcl_taint_hints as _tcl_taint_hints  # noqa: E402

TAINT_HINTS: dict[str, TaintHint] = {**_tcl_taint_hints()}

# Taint hint loaders: (module_name, func_name) for dialects that provide
# taint hints.  Loaded lazily via importlib, mirroring the command spec
# loader pattern in command_registry.py.
_TAINT_HINT_LOADER_SPECS: dict[str, tuple[str, str]] = {
    "f5-irules": ("irules", "irules_taint_hints"),
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
            mod = importlib.import_module(f".{mod_name}", __package__)
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

    # Also clear the parser's known-command cache.
    from ...parsing.known_commands import known_command_names

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
def storage_type_commands() -> dict[str, object]:
    """Return ``{command: StorageType}`` for commands that imply a storage type.

    Used by side-effect analysis to infer whether a variable holds a dict,
    list, or array based on the command that writes to it.
    Derived from ``inferred_storage_type`` on :class:`CommandSpec`.
    """
    result: dict[str, object] = {}
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

from .dialects import KNOWN_DIALECTS as _KNOWN_DIALECTS  # noqa: E402

_active_dialect = "tcl8.6"
_active_extra_commands: tuple[str, ...] = ()
SIGNATURES: dict[str, CommandSig | SubcommandSig] = {}


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


def active_signature_profile() -> dict[str, object]:
    """Return the currently active command-signature profile."""
    return {
        "dialect": _active_dialect,
        "extra_commands": list(_active_extra_commands),
    }


def is_irules_dialect() -> bool:
    """Return True if the active dialect is iRules."""
    return _active_dialect == "f5-irules"


def configure_signatures(
    *,
    dialect: str | None = None,
    extra_commands: list[str] | tuple[str, ...] | None = None,
) -> bool:
    """Configure active command signatures.

    Returns ``True`` if the effective profile changed.
    """
    global _active_dialect, _active_extra_commands

    if dialect is None:
        next_dialect = _active_dialect
    else:
        requested = _canonical_dialect(dialect)
        if requested is None:
            return False
        next_dialect = requested
    if extra_commands is None:
        next_extra = _active_extra_commands
    else:
        next_extra = tuple(
            sorted({name.strip() for name in extra_commands if name and name.strip()})
        )

    if next_dialect == _active_dialect and next_extra == _active_extra_commands and SIGNATURES:
        return False

    # Ensure dialect-specific command specs are loaded before building
    # signatures.  Taint hints are merged automatically via the
    # _on_specs_loaded callback when new specs are loaded.
    REGISTRY.load_dialect_specs(next_dialect)

    new_signatures = _build_signatures(
        next_dialect,
        extra_commands=next_extra,
    )
    SIGNATURES.clear()
    SIGNATURES.update(new_signatures)
    _active_dialect = next_dialect
    _active_extra_commands = next_extra

    # Configure lexer flags for the active dialect.
    from core.parsing.lexer import TclLexer

    from .dialects import dialects_since

    TclLexer.irules_brace_separator = next_dialect == "f5-irules"
    # {*} argument expansion was introduced in Tcl 8.5.  Disable it for
    # 8.4-based dialects (tcl8.4, f5-irules) so ``cmd {*}$args`` is
    # lexed as a single ``*${args}`` word rather than an expansion.
    TclLexer.expand_syntax = next_dialect in dialects_since("tcl8.5")

    return True


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
    from ...parsing.lexer import TclLexer
    from ...parsing.tokens import TokenType

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


def _resolve_arg_roles(command: str, args: list[str]) -> tuple[dict[int, ArgRole], int]:
    """Return ``(role_map, base_index)`` for a *command* invocation.

    *role_map* maps argument indices into ``args[base_index:]`` to their
    :class:`ArgRole`.  ``base_index`` is the offset to add when expressing
    the indices against the original *args* list — 1 for ensemble-form
    commands (where ``args[0]`` is the subcommand word) and 0 for plain
    commands and CFG-rewritten ensemble names.

    Returns ``({}, 0)`` for unregistered or unmatched commands; callers
    should treat the empty role map as "no roles known" and skip output.
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

    canonical = _canonicalise_command_name(command)
    sig = SIGNATURES.get(canonical)
    if sig is None:
        sig = _ROLE_HINTS.get(canonical)
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


def _resolved_role_map(sig: CommandSig, args: list[str]) -> dict[int, ArgRole]:
    """Resolve :class:`CommandSig` roles for *args*, dynamic resolver first."""
    if sig.arg_role_resolver is not None:
        resolved = sig.arg_role_resolver(list(args))
        return {idx: r for idx, r in resolved.items() if idx < len(args)}
    return {idx: r for idx, r in sig.arg_roles.items() if idx < len(args)}


# Subsumption: a query for these "narrower" roles also matches the
# "broader" :data:`ArgRole.VAR_READ_WRITE` role.  Keeping the map small
# means new role additions are explicit; widen as needed when more
# combined-semantic roles are added.
_ROLE_SUBSUMES: dict[ArgRole, frozenset[ArgRole]] = {
    ArgRole.VAR_READ: frozenset({ArgRole.VAR_READ_WRITE}),
    ArgRole.VAR_WRITE: frozenset({ArgRole.VAR_READ_WRITE}),
}


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

    canonical = _canonicalise_command_name(command)
    spec = REGISTRY.get_any(canonical)
    if spec is None:
        return BodyKind.INLINE
    if spec.subcommands and args:
        sub = spec.subcommands.get(args[0])
        if sub is not None:
            return sub.body_kind
    return spec.body_kind


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

    accept = _ROLE_SUBSUMES.get(role, frozenset()) | {role}
    return {idx + base_index for idx, r in role_map.items() if r in accept}


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
        accept = _ROLE_SUBSUMES.get(role, frozenset()) | {role}
        results[idx_in_result] = {idx + base_index for idx, r in role_map.items() if r in accept}

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
import core.commands.registry.command_registry as _cmd_reg  # noqa: E402

_cmd_reg._on_specs_loaded = _invalidate_runtime_caches

# Initialize runtime signatures for default profile.
configure_signatures(dialect="tcl8.6", extra_commands=[])
