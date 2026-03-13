"""Scaffold class-per-command files for Tcl commands from man pages.

Usage:
    # Scaffold all Tcl commands that don't already have a class file:
    python scripts/registry/scaffold_tcl_commands.py \
        --doc-dir /path/to/tcl9/doc

    # Scaffold a single command:
    python scripts/registry/scaffold_tcl_commands.py \
        --doc-dir /path/to/tcl9/doc --command after

    # Force re-scaffold (write to .scaffolded for diffing):
    python scripts/registry/scaffold_tcl_commands.py \
        --doc-dir /path/to/tcl9/doc --command after --force
"""

from __future__ import annotations

import argparse
import keyword
import re
import sys
import textwrap
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[2]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from core.commands.registry.runtime import (  # noqa: E402
    SIGNATURES,
    CommandSig,
    SubcommandSig,
    configure_signatures,
)
from core.commands.registry.signatures import Arity  # noqa: E402

# Man page parsing (reused from generate_tcl_core_from_manpages.py)

DIALECTS = ("tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0")

MARKUP_RE = re.compile(r"\\f[BRIUP]")
MARKER_RE = re.compile(r'^\s*\.\\"\s+(OPTION|METHOD|COMMAND):\s*(\S+)')
SYNOPSIS_SWITCH_RE = re.compile(r"--|-[A-Za-z][A-Za-z0-9_-]*")
METHOD_NAME_RE = re.compile(r"^[A-Za-z][A-Za-z0-9_:.+-]*$")

EXPLICIT_PAGE_OVERRIDES: dict[str, str] = {
    "oo::objdefine": "define.n",
}

# No longer maintained: all commands use class files.  The scaffolder
# skips any command whose class file already exists (line 661).


def _clean_nroff(text: str) -> str:
    value = text
    value = MARKUP_RE.sub("", value)
    value = value.replace("\\-", "-")
    value = value.replace("\\e", "\\")
    value = value.replace('\\"', '"')
    value = value.replace("\\{", "{").replace("\\}", "}")
    value = value.replace("\\[", "[").replace("\\]", "]")
    value = value.replace("\\|", "|")
    value = value.replace("\\~", " ")
    value = value.replace("\\&", "")
    value = value.replace("\\^", "")
    value = value.replace("\\(", "")
    value = value.replace("\\)", "")
    value = " ".join(value.split())
    return value.strip()


def _extract_section(lines: list[str], title: str) -> list[str]:
    start: int | None = None
    for idx, line in enumerate(lines):
        if line.startswith(".SH ") and title in line:
            start = idx + 1
            break
    if start is None:
        return []
    end = len(lines)
    for idx in range(start, len(lines)):
        if lines[idx].startswith(".SH "):
            end = idx
            break
    return lines[start:end]


def _split_first_sentence(text: str) -> str:
    if not text:
        return ""
    sentence_end = re.search(r"[.!?](?:\s|$)", text)
    if sentence_end:
        return text[: sentence_end.end()].strip()
    return text.strip()


def _truncate(text: str, max_len: int = 280) -> str:
    if len(text) <= max_len:
        return text
    return text[: max_len - 1].rstrip() + "\u2026"


def _parse_name_section(lines: list[str]) -> tuple[tuple[str, ...], dict[str, str]]:
    section = _extract_section(lines, "NAME")
    raw_lines = [
        _clean_nroff(line)
        for line in section
        if line and not line.startswith(".") and not line.startswith("'")
    ]
    if not raw_lines:
        return (), {}
    combined = " ".join(raw_lines)
    lhs, _, rhs = combined.partition(" - ")
    summary = rhs.strip()
    names: list[str] = []
    for part in lhs.split(","):
        token = part.strip().split()[0] if part.strip() else ""
        if token and re.match(r"^[A-Za-z0-9_:]+$", token):
            names.append(token)
    summary_by_name = {name: summary for name in names}
    return tuple(names), summary_by_name


def _parse_synopsis(lines: list[str]) -> tuple[str, ...]:
    section = _extract_section(lines, "SYNOPSIS")
    syn: list[str] = []
    in_nf = False
    for line in section:
        if line.startswith(".nf"):
            in_nf = True
            continue
        if line.startswith(".fi"):
            in_nf = False
            continue
        if line.startswith(".SH "):
            break
        if line.startswith("'"):
            continue
        if line.startswith(".") and not in_nf:
            continue
        cleaned = _clean_nroff(line)
        if cleaned:
            syn.append(cleaned)
    seen: set[str] = set()
    ordered: list[str] = []
    for line in syn:
        if line not in seen:
            seen.add(line)
            ordered.append(line)
    return tuple(ordered)


def _parse_description_snippet(lines: list[str]) -> str:
    section = _extract_section(lines, "DESCRIPTION")
    if not section:
        return ""
    paragraphs: list[str] = []
    current: list[str] = []
    for line in section:
        if line.startswith(".SH "):
            break
        if line.startswith("'"):
            continue
        if line.startswith((".PP", ".P")):
            if current:
                paragraphs.append(" ".join(current))
                current = []
            continue
        if line.startswith("."):
            continue
        cleaned = _clean_nroff(line)
        if cleaned:
            current.append(cleaned)
    if current:
        paragraphs.append(" ".join(current))
    if not paragraphs:
        return ""
    first = paragraphs[0].strip()
    return _truncate(_split_first_sentence(first), max_len=300)


def _parse_marker_entries(lines: list[str], marker_kind: str) -> dict[str, dict[str, str]]:
    entries: dict[str, dict[str, str]] = {}
    for idx, line in enumerate(lines):
        marker_match = MARKER_RE.match(line)
        if not marker_match:
            continue
        kind, name = marker_match.groups()
        if kind != marker_kind:
            continue
        block_end = len(lines)
        for j in range(idx + 1, len(lines)):
            if lines[j].startswith(".SH "):
                block_end = j
                break
            if MARKER_RE.match(lines[j]):
                block_end = j
                break
        block = lines[idx + 1 : block_end]
        cleaned_block: list[str] = []
        for block_line in block:
            if block_line.startswith("'") or block_line.startswith("."):
                continue
            cleaned = _clean_nroff(block_line)
            if cleaned:
                cleaned_block.append(cleaned)
        synopsis = cleaned_block[0] if cleaned_block else ""
        summary_source = " ".join(cleaned_block[1:]) if len(cleaned_block) > 1 else ""
        summary = _truncate(_split_first_sentence(summary_source), max_len=220)
        if name not in entries:
            entries[name] = {"name": name, "synopsis": synopsis, "summary": summary}
    return entries


def _parse_page(path: Path) -> dict[str, Any]:
    lines = path.read_text(encoding="utf-8", errors="ignore").splitlines()
    name_entries, summary_by_name = _parse_name_section(lines)
    synopsis_lines = _parse_synopsis(lines)
    description_snippet = _parse_description_snippet(lines)
    option_entries = _parse_marker_entries(lines, "OPTION")
    method_entries = _parse_marker_entries(lines, "METHOD")
    return {
        "filename": path.name,
        "name_entries": name_entries,
        "summary_by_name": summary_by_name,
        "synopsis_lines": synopsis_lines,
        "description_snippet": description_snippet,
        "option_entries": option_entries,
        "method_entries": method_entries,
    }


def _select_synopsis(command: str, page: dict) -> list[str]:
    if not page["synopsis_lines"]:
        return [command]
    base = command.rsplit("::", 1)[-1]
    lowered = command.lower()
    lowered_base = base.lower()
    chosen: list[str] = []
    for line in page["synopsis_lines"]:
        value = line.lower()
        if value.startswith(lowered + " ") or value == lowered:
            chosen.append(line)
            continue
        if lowered != lowered_base and (
            value.startswith(lowered_base + " ") or value == lowered_base
        ):
            chosen.append(line)
    if not chosen:
        chosen = list(page["synopsis_lines"][:3])
    return chosen[:4]


# Signature + dialect helpers


def _collect_dialect_membership() -> dict[str, set[str]]:
    by_dialect: dict[str, set[str]] = {}
    for dialect in DIALECTS:
        configure_signatures(dialect=dialect, extra_commands=[])
        by_dialect[dialect] = set(SIGNATURES.keys())
    configure_signatures(dialect="tcl8.6", extra_commands=[])
    return by_dialect


def _dialects_for_command(command: str, by_dialect: dict[str, set[str]]) -> list[str]:
    return [dialect for dialect in DIALECTS if command in by_dialect[dialect]]


# Build page index


def _build_page_index(doc_dir: Path) -> tuple[dict[str, dict], dict[str, str]]:
    pages: dict[str, dict] = {}
    command_to_page: dict[str, str] = {}

    def _register(name: str, filename: str) -> None:
        current = command_to_page.get(name)
        preferred = f"{name}.n"
        if current is None:
            command_to_page[name] = filename
        elif filename == preferred and current != preferred:
            command_to_page[name] = filename

    for page_path in sorted(doc_dir.glob("*.n")):
        page = _parse_page(page_path)
        pages[page["filename"]] = page
        for name in page["name_entries"]:
            _register(name, page["filename"])
    return pages, command_to_page


def _resolve_page(command: str, command_to_page: dict[str, str], doc_dir: Path) -> str | None:
    if command in EXPLICIT_PAGE_OVERRIDES:
        return EXPLICIT_PAGE_OVERRIDES[command]
    if command in command_to_page:
        return command_to_page[command]
    base = command.rsplit("::", 1)[-1]
    if base in command_to_page:
        return command_to_page[base]
    direct = f"{base}.n"
    if (doc_dir / direct).exists():
        return direct
    return None


# Code generation


def _safe_module_name(command: str) -> str:
    """Convert a command name to a safe Python module/file name."""
    name = command.replace("::", "_").replace("-", "_").replace("+", "_plus")
    if keyword.iskeyword(name) or name in (
        "open",
        "exec",
        "return",
        "for",
        "if",
        "while",
        "break",
        "continue",
        "global",
        "yield",
        "raise",
        "class",
        "import",
        "from",
        "try",
        "except",
        "finally",
        "with",
        "as",
        "lambda",
        "del",
        "pass",
        "assert",
        "else",
        "elif",
        "not",
        "and",
        "or",
        "is",
        "in",
        "True",
        "False",
        "None",
    ):
        name = name + "_"
    return name


def _safe_class_name(command: str) -> str:
    """Convert a command name to a PascalCase class name."""
    parts = command.replace("::", "_").replace("-", "_").replace("+", "Plus").split("_")
    return "".join(p.capitalize() for p in parts if p) + "Command"


def _escape_str(s: str) -> str:
    """Escape a string for Python source code."""
    return s.replace("\\", "\\\\").replace('"', '\\"')


def _format_sig(sig: CommandSig | SubcommandSig) -> str:
    """Format a signature as Python source code."""
    if isinstance(sig, SubcommandSig):
        lines = ["SubcommandSig(subcommands={"]
        for name, sub_sig in sorted(sig.subcommands.items()):
            lines.append(f'            "{name}": {_format_command_sig(sub_sig)},')
        allow = f", allow_unknown={sig.allow_unknown}" if sig.allow_unknown else ""
        lines.append(f"        }}{allow})")
        return "\n".join(lines)
    return _format_command_sig(sig)


def _format_arity(arity: Arity) -> str:
    """Format an Arity as Python source code."""
    if arity.is_unlimited:
        if arity.min == 0:
            return "Arity()"
        return f"Arity({arity.min})"
    return f"Arity({arity.min}, {arity.max})"


def _format_command_sig(sig: CommandSig) -> str:
    """Format a CommandSig as Python source code."""
    parts = [f"arity={_format_arity(sig.arity)}"]
    if sig.arg_roles:
        roles = ", ".join(f"{k}: ArgRole.{v.name}" for k, v in sorted(sig.arg_roles.items()))
        parts.append(f"arg_roles={{{roles}}}")
    return f"CommandSig({', '.join(parts)})"


def _generate_class_file(
    command: str,
    page: dict,
    signature: CommandSig | SubcommandSig | None,
    dialects: list[str],
) -> str:
    """Generate the Python source for a command class file."""
    module_doc = (
        f'"""{command} -- {page["summary_by_name"].get(command, f"Tcl command `{command}`")}."""'
    )
    synopsis_lines = _select_synopsis(command, page)
    base = command.rsplit("::", 1)[-1]
    summary = (
        page["summary_by_name"].get(command)
        or page["summary_by_name"].get(base)
        or next(iter(page["summary_by_name"].values()), f"Tcl command `{command}`")
    )
    snippet = page["description_snippet"]
    is_subcmd = isinstance(signature, SubcommandSig)
    sub_signature = signature if isinstance(signature, SubcommandSig) else None
    cmd_signature = signature if isinstance(signature, CommandSig) else None
    has_arg_roles = (
        isinstance(signature, CommandSig)
        and signature.arg_roles
        or isinstance(signature, SubcommandSig)
        and any(s.arg_roles for s in signature.subcommands.values())
    )

    # Determine imports
    model_imports = ["CommandSpec", "FormSpec", "HoverSnippet"]
    sig_imports = []

    has_options = bool(page["option_entries"]) and not is_subcmd
    has_arg_values = bool(page["method_entries"]) or is_subcmd
    has_validation = signature is not None

    if has_arg_values:
        model_imports.insert(0, "ArgumentValueSpec")
    if has_options:
        if "OptionSpec" not in model_imports:
            model_imports.append("OptionSpec")
    if has_validation:
        if "ValidationSpec" not in model_imports:
            model_imports.append("ValidationSpec")
    if is_subcmd:
        if "SubCommand" not in model_imports:
            model_imports.append("SubCommand")

    if is_subcmd:
        sig_imports.extend(["Arity", "CommandSig", "SubcommandSig"])
    elif signature is not None:
        sig_imports.extend(["Arity", "CommandSig"])

    if has_validation and not sig_imports:
        sig_imports.append("Arity")

    if has_arg_roles:
        if "ArgRole" not in sig_imports:
            sig_imports.insert(0, "ArgRole")

    model_imports.sort()
    sig_imports.sort()

    # Build source
    lines: list[str] = []
    lines.append(f"# Scaffolded from {page['filename']} -- refine and commit")
    lines.append(module_doc)
    lines.append("")
    lines.append("from __future__ import annotations")
    lines.append("")
    lines.append(f"from ..models import {', '.join(model_imports)}")
    if sig_imports:
        lines.append(f"from ..signatures import {', '.join(sig_imports)}")
    lines.append("from .._base import CommandDef")
    lines.append("from ._base import register")
    lines.append("")
    lines.append(f'_SOURCE = "Tcl man page {page["filename"]}"')
    lines.append("")
    lines.append("")

    class_name = _safe_class_name(command)
    lines.append("@register")
    lines.append(f"class {class_name}(CommandDef):")
    lines.append(f'    name = "{command}"')
    lines.append("")

    # -- spec() method --
    lines.append("    @classmethod")
    lines.append("    def spec(cls) -> CommandSpec:")

    # Build synopsis tuples
    synopsis_strs = ", ".join(f'"{_escape_str(s)}"' for s in synopsis_lines)

    # Validation
    val_lines = ""
    subcmd_lines = ""
    if has_validation:
        if is_subcmd and sub_signature is not None:
            sub_entries = []
            for sub_name, sub_sig in sorted(sub_signature.subcommands.items()):
                sub_entries.append(
                    f'                "{sub_name}": SubCommand(\n'
                    f'                    name="{sub_name}",\n'
                    f"                    arity={_format_arity(sub_sig.arity)},\n"
                    f'                    detail="{command} {sub_name}",\n'
                    f'                    synopsis="{command} {sub_name}",\n'
                    f"                ),"
                )
            subcmd_lines = (
                "            subcommands={\n" + "\n".join(sub_entries) + "\n            },\n"
            )
            val_lines = (
                "            validation=ValidationSpec(\n"
                "                arity=Arity(1),\n"
                "            ),\n"
            )
        elif cmd_signature is not None:
            val_lines = (
                "            validation=ValidationSpec(\n"
                f"                arity={_format_arity(cmd_signature.arity)},\n"
                "            ),\n"
            )

    # Options
    option_lines = ""
    if has_options:
        opts: list[str] = []
        seen: set[str] = set()
        for opt_name, opt_data in page["option_entries"].items():
            if not opt_name.startswith("-"):
                continue
            if opt_name in seen:
                continue
            seen.add(opt_name)
            detail = _escape_str(opt_data.get("summary", "") or f"Option {opt_name}.")
            opt_syn = opt_data.get("synopsis", "")
            takes_value = "False"
            if opt_syn and opt_name in opt_syn:
                tail = opt_syn.split(opt_name, 1)[1].strip()
                if tail:
                    takes_value = "True"
            opts.append(
                f'                    OptionSpec(name="{opt_name}", detail="{detail}", takes_value={takes_value}),'
            )
        # Also grab from synopsis
        for syn_line in synopsis_lines:
            for token in SYNOPSIS_SWITCH_RE.findall(syn_line):
                if token not in seen:
                    seen.add(token)
                    opts.append(
                        f'                    OptionSpec(name="{token}", detail="Option {token}.", takes_value=False),'
                    )
        if opts:
            option_lines = (
                "                options=(\n" + "\n".join(opts) + "\n                ),\n"
            )

    # Arg values (subcommands)
    arg_value_lines = ""
    if has_arg_values:
        avs: list[str] = []
        names_order: list[str] = []
        if is_subcmd and sub_signature is not None:
            for name in sorted(sub_signature.subcommands.keys()):
                names_order.append(name)
        for name in page["method_entries"].keys():
            if name.startswith("-") or not METHOD_NAME_RE.match(name):
                continue
            if name not in names_order:
                names_order.append(name)

        for name in names_order:
            marker = page["method_entries"].get(name)
            detail = _escape_str(
                marker["summary"] if marker and marker.get("summary") else f"{command} {name}"
            )
            syn = _escape_str(
                marker["synopsis"] if marker and marker.get("synopsis") else f"{command} {name}"
            )
            avs.append(
                f'                        _av("{name}", "{detail}",\n'
                f'                            "{syn}"),'
            )

        if avs:
            arg_value_lines = (
                "                arg_values={0: (\n" + "\n".join(avs) + "\n                )},\n"
            )

    # Dialects
    dialect_str = ", ".join(f'"{d}"' for d in dialects)

    # Assemble the spec method body
    spec_body = []
    spec_body.append("        return CommandSpec(")
    spec_body.append(f'            name="{command}",')
    if dialects and set(dialects) != set(DIALECTS):
        spec_body.append(f"            dialects=frozenset({{{dialect_str}}}),")
    spec_body.append("            hover=HoverSnippet(")
    spec_body.append(f'                summary="{_escape_str(summary)}",')
    spec_body.append(f"                synopsis=({synopsis_strs},),")
    if snippet:
        spec_body.append(f'                snippet="{_escape_str(snippet)}",')
    spec_body.append("                source=_SOURCE,")
    spec_body.append("            ),")
    spec_body.append("            forms=(")
    spec_body.append("                FormSpec(")
    spec_body.append('                    form_id="default",')
    spec_body.append(
        f'                    synopsis="{_escape_str(synopsis_lines[0] if synopsis_lines else command)}",'
    )
    if option_lines:
        spec_body.append(option_lines.rstrip())
    if arg_value_lines:
        spec_body.append(arg_value_lines.rstrip())
    spec_body.append("                ),")
    spec_body.append("            ),")
    if subcmd_lines:
        spec_body.append(subcmd_lines.rstrip())
    if val_lines:
        spec_body.append(val_lines.rstrip())
    spec_body.append("        )")

    lines.extend(spec_body)
    lines.append("")

    # Helper function for arg values
    if has_arg_values and any(
        not name.startswith("-") and METHOD_NAME_RE.match(name)
        for name in list(sub_signature.subcommands.keys() if sub_signature is not None else [])
        + list(page["method_entries"].keys())
    ):
        # Add _av helper at module level (before class)
        # We'll insert it after the _SOURCE line
        pass

    lines.append("")
    source_code = "\n".join(lines)

    # Insert _av helper if we used it
    if "_av(" in source_code:
        av_helper = textwrap.dedent("""\

def _av(value: str, detail: str, synopsis: str = "") -> ArgumentValueSpec:
    return ArgumentValueSpec(
        value=value,
        detail=detail,
        hover=HoverSnippet(
            summary=detail,
            synopsis=(synopsis,) if synopsis else (),
            source=_SOURCE,
        ),
    )

""")
        # Insert after _SOURCE line
        source_code = source_code.replace(
            f'\n_SOURCE = "Tcl man page {page["filename"]}"\n\n\n',
            f'\n_SOURCE = "Tcl man page {page["filename"]}"\n{av_helper}\n',
        )

    return source_code


# Main driver


def scaffold_commands(
    doc_dir: Path,
    output_dir: Path,
    command_filter: str | None = None,
    force: bool = False,
) -> list[str]:
    """Scaffold command class files. Returns list of written file paths."""
    pages, command_to_page = _build_page_index(doc_dir)
    by_dialect = _collect_dialect_membership()

    # Get tcl9.0 signatures (broadest set)
    configure_signatures(dialect="tcl9.0", extra_commands=[])
    tcl9_signatures = dict(SIGNATURES)
    configure_signatures(dialect="tcl8.6", extra_commands=[])

    all_commands = set().union(*by_dialect.values())
    written: list[str] = []
    skipped_no_page: list[str] = []

    for command in sorted(all_commands):
        if command_filter and command != command_filter:
            continue

        filename = _resolve_page(command, command_to_page, doc_dir)
        if filename is None:
            skipped_no_page.append(command)
            continue

        module_name = _safe_module_name(command)
        target = output_dir / f"{module_name}.py"

        if target.exists() and not force:
            continue

        if target.exists() and force:
            target = output_dir / f"{module_name}.py.scaffolded"

        page = pages[filename]
        signature = tcl9_signatures.get(command)
        dialects = _dialects_for_command(command, by_dialect)

        source = _generate_class_file(command, page, signature, dialects)
        target.write_text(source, encoding="utf-8")
        written.append(str(target))

    if skipped_no_page and not command_filter:
        print(
            f"Warning: {len(skipped_no_page)} commands had no man page mapping: "
            f"{', '.join(skipped_no_page[:10])}{'...' if len(skipped_no_page) > 10 else ''}"
        )

    return written


def main() -> None:
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    parser.add_argument(
        "--doc-dir",
        type=Path,
        default=Path("tmp/tcl9.0.3/doc"),
        help="Directory containing Tcl .n man pages",
    )
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=ROOT / "server" / "commands" / "registry" / "tcl",
        help="Output directory for command class files",
    )
    parser.add_argument("--command", help="Scaffold a single command")
    parser.add_argument(
        "--force", action="store_true", help="Re-scaffold existing files (writes to .scaffolded)"
    )
    args = parser.parse_args()

    if not args.doc_dir.exists():
        raise FileNotFoundError(f"Man page directory not found: {args.doc_dir}")

    args.output_dir.mkdir(parents=True, exist_ok=True)

    written = scaffold_commands(args.doc_dir, args.output_dir, args.command, args.force)
    print(f"Scaffolded {len(written)} command files in {args.output_dir}")
    for path in written[:20]:
        print(f"  {path}")
    if len(written) > 20:
        print(f"  ... and {len(written) - 20} more")


if __name__ == "__main__":
    main()
