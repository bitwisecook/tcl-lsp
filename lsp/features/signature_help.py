"""Signature help provider -- parameter hints while typing command arguments."""

from __future__ import annotations

from lsprotocol import types

from core.analysis.analyser import analyse
from core.analysis.proc_lookup import find_proc_by_reference
from core.analysis.semantic_model import AnalysisResult, ProcDef
from core.commands.registry import REGISTRY
from core.commands.registry.models import CommandSpec
from core.commands.registry.runtime import SIGNATURES, SubcommandSig
from core.common.dialect import active_dialect

from .symbol_resolution import find_command_context_details_at_position


def _parse_synopsis_params(synopsis: str, skip_tokens: int = 1) -> list[types.ParameterInformation]:
    """Parse a synopsis string like 'set varName ?newValue?' into ParameterInformation list.

    *skip_tokens* is the number of leading tokens to skip (1 for command name,
    2 for command + subcommand).
    """
    tokens = synopsis.split()
    params: list[types.ParameterInformation] = []
    i = skip_tokens
    while i < len(tokens):
        tok = tokens[i]
        # Handle multi-word optional groups like "?arg ...?"
        if tok.startswith("?") and not tok.endswith("?"):
            # Collect until we find the closing ?
            group = [tok]
            i += 1
            while i < len(tokens):
                group.append(tokens[i])
                if tokens[i].endswith("?"):
                    break
                i += 1
            label = " ".join(group)
        else:
            label = tok
        params.append(types.ParameterInformation(label=label))
        i += 1
    return params


def _proc_signature_help(
    proc_def: ProcDef,
    active_param: int,
) -> types.SignatureHelp:
    """Build SignatureHelp for a user-defined proc."""
    param_parts: list[str] = []
    params: list[types.ParameterInformation] = []
    for p in proc_def.params:
        if p.has_default:
            label = f"?{p.name}?"
            param_parts.append(f"?{p.name} {p.default_value}?")
        else:
            label = p.name
            param_parts.append(p.name)
        params.append(types.ParameterInformation(label=label))

    sig_str = f"{proc_def.name} {' '.join(param_parts)}" if param_parts else proc_def.name
    sig = types.SignatureInformation(
        label=sig_str,
        documentation=proc_def.doc or None,
        parameters=params,
    )
    return types.SignatureHelp(
        signatures=[sig],
        active_signature=0,
        active_parameter=max(0, min(active_param, len(params) - 1)) if params else 0,
    )


def _signature_documentation(
    spec: CommandSpec,
    subcmd: str | None = None,
) -> types.MarkupContent | None:
    """Build rich documentation for a SignatureInformation.

    Includes the detail fields (snippet, return_value, examples) that hover
    now omits, giving signature help its own documentation pane.
    """
    hover = spec.hover
    if hover is None:
        return None

    # For subcommands, try the subcommand-specific ArgumentValueSpec hover first.
    if subcmd is not None:
        for form in spec.forms:
            for value_spec in form.arg_values.get(0, ()):
                if value_spec.value == subcmd and value_spec.hover is not None:
                    md = value_spec.hover.render_markdown(f"{spec.name} {subcmd}")
                    return types.MarkupContent(kind=types.MarkupKind.Markdown, value=md)

    # Fall back to the command-level hover rendered in full.
    md = hover.render_markdown(spec.name)
    return types.MarkupContent(kind=types.MarkupKind.Markdown, value=md)


def _builtin_signature_help(
    command: str,
    args: list[str],
    active_param: int,
    *,
    active_packages: frozenset[str] | None = None,
) -> types.SignatureHelp | None:
    """Build SignatureHelp for a built-in command from registry metadata."""
    dialect = active_dialect()
    spec = REGISTRY.get(command, dialect, active_packages=active_packages)
    if spec is None:
        return None

    # Check if this is a subcommand-bearing command
    sig_obj = SIGNATURES.get(command)
    is_subcommand = isinstance(sig_obj, SubcommandSig)

    signatures: list[types.SignatureInformation] = []
    effective_active_param = active_param

    if is_subcommand and args:
        # Resolve to subcommand-specific synopsis
        subcmd = args[0]
        effective_active_param = active_param - 1  # subtract subcommand word
        doc = _signature_documentation(spec, subcmd)

        # Look through forms for subcommand-specific synopses
        for form in spec.forms:
            syn = form.synopsis
            # Match forms containing this subcommand
            syn_tokens = syn.split()
            if len(syn_tokens) >= 2 and syn_tokens[1] == subcmd:
                params = _parse_synopsis_params(syn, skip_tokens=2)
                signatures.append(
                    types.SignatureInformation(
                        label=syn,
                        documentation=doc,
                        parameters=params,
                    )
                )

        # Fallback: try hover synopsis
        if not signatures and spec.hover and spec.hover.synopsis:
            for syn in spec.hover.synopsis:
                syn_tokens = syn.split()
                if len(syn_tokens) >= 2 and syn_tokens[1] == subcmd:
                    params = _parse_synopsis_params(syn, skip_tokens=2)
                    signatures.append(
                        types.SignatureInformation(
                            label=syn,
                            documentation=doc,
                            parameters=params,
                        )
                    )
    else:
        # Non-subcommand: use all forms / hover synopsis
        doc = _signature_documentation(spec)
        for form in spec.forms:
            params = _parse_synopsis_params(form.synopsis)
            signatures.append(
                types.SignatureInformation(
                    label=form.synopsis,
                    documentation=doc,
                    parameters=params,
                )
            )

        # If no forms, try hover synopsis
        if not signatures and spec.hover and spec.hover.synopsis:
            for syn in spec.hover.synopsis:
                params = _parse_synopsis_params(syn)
                signatures.append(
                    types.SignatureInformation(
                        label=syn,
                        documentation=doc,
                        parameters=params,
                    )
                )

    if not signatures:
        return None

    # Clamp active_parameter
    if effective_active_param < 0:
        effective_active_param = 0

    # Pick best matching signature based on parameter count
    active_sig = 0
    for i, sig in enumerate(signatures):
        if sig.parameters and effective_active_param < len(sig.parameters):
            active_sig = i
            break

    params = signatures[active_sig].parameters
    max_params = len(params) if params else 0
    return types.SignatureHelp(
        signatures=signatures,
        active_signature=active_sig,
        active_parameter=min(effective_active_param, max(0, max_params - 1)),
    )


def get_signature_help(
    source: str,
    line: int,
    character: int,
    analysis: AnalysisResult | None = None,
) -> types.SignatureHelp | None:
    """Return signature help for the command being typed at the cursor."""
    if analysis is None:
        analysis = analyse(source)

    command, args, _current_word, word_index = find_command_context_details_at_position(
        source,
        line,
        character,
    )
    if command is None or word_index <= 0:
        return None

    # Active parameter is the argument index (word_index 1 = first arg = param 0)
    active_param = word_index - 1

    # Check user-defined procs first
    proc_match = find_proc_by_reference(analysis, command)
    if proc_match is not None:
        _qname, proc_def = proc_match
        return _proc_signature_help(proc_def, active_param)

    # Built-in command
    return _builtin_signature_help(
        command, args, active_param, active_packages=analysis.active_package_names()
    )
