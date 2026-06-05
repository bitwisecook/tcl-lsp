"""Shared group → (python module, factory, rust subdir) table for the
registry-audit porters."""

from __future__ import annotations

# group -> (module under core.commands.registry, factory fn, rust subdir)
GROUPS: dict[str, tuple[str, str, str]] = {
    "tcl": ("tcl", "tcl_command_specs", "tcl"),
    "stdlib": ("stdlib", "stdlib_command_specs", "stdlib"),
    "tcllib": ("tcllib", "tcllib_command_specs", "tcllib"),
    "irules": ("irules", "irules_command_specs", "irules"),
    "iapps": ("iapps", "iapps_command_specs", "iapps"),
    "tk": ("tk", "tk_command_specs", "tk"),
    "expect": ("expect", "expect_command_specs", "expect"),
    "sdc-base": ("eda_sdc_base", "sdc_base_command_specs", "sdc_base"),
    "synopsys": ("eda_synopsys", "synopsys_command_specs", "eda_synopsys"),
    "cadence": ("eda_cadence", "cadence_command_specs", "eda_cadence"),
    "xilinx": ("eda_xilinx", "xilinx_command_specs", "eda_xilinx"),
    "quartus": ("eda_quartus", "quartus_command_specs", "eda_quartus"),
    "mentor": ("eda_mentor", "mentor_command_specs", "eda_mentor"),
}


def load_specs(group: str):
    mod_name, factory, _ = GROUPS[group]
    mod = __import__(f"core.commands.registry.{mod_name}", fromlist=[factory])
    return getattr(mod, factory)()


def rust_dir(repo_root, group: str):
    return repo_root / "rust/tcl-registry/src/commands" / GROUPS[group][2]


import re as _re  # noqa: E402

# Command-level name = the first `name: "..."` *after* the `CommandSpec {`
# opener (so a preceding `const SUBCOMMANDS` with its own subcommand
# `name:` fields can't shadow it), tolerating intervening comments/fields.
_CMDSPEC_RE = _re.compile(r"CommandSpec\s*\{")
_NAME_AFTER_RE = _re.compile(r'name:\s*"((?:[^"\\]|\\.)*)"')
_USE_RE = _re.compile(r"^use crate::prelude::\*;\s*$", _re.M)


def files_by_name(rdir):
    # Only consider modules actually declared in mod.rs — orphan/duplicate
    # `*.rs` files (e.g. an unused `fcopy_.rs` shadowing the registered
    # `fcopy.rs`) must never be the injection target.
    mod_text = (rdir / "mod.rs").read_text()
    declared = set(_re.findall(r"^\s*(?:pub )?mod (\w+);", mod_text, _re.M))
    out = {}
    for p in rdir.glob("*.rs"):
        if p.name == "mod.rs" or p.stem not in declared:
            continue
        text = p.read_text()
        m = _CMDSPEC_RE.search(text)
        if not m:
            continue
        nm = _NAME_AFTER_RE.search(text, m.end())
        if nm:
            out[nm.group(1).replace('\\"', '"').replace("\\\\", "\\")] = p
    return out


def has_field(text: str, field: str) -> bool:
    return _re.search(r"^\s*" + field + r":", text, _re.M) is not None


def insert_const(text: str, const_name: str, rust_type: str, body: str) -> str | None:
    """Insert `const NAME: TYPE = &[ body ];` after the prelude `use`.

    Returns the new text, or None if the const already exists or there
    is no prelude-use anchor.
    """
    if f"const {const_name}" in text:
        return None
    um = _USE_RE.search(text)
    if not um:
        return None
    block = f"\nconst {const_name}: {rust_type} = &[\n{body}\n];\n"
    return text[: um.end()] + block + text[um.end() :]


def set_spec_field(text: str, field_line: str) -> str:
    """Insert `field_line` just before `..CommandSpec::DEFAULT`."""
    m = _re.search(r"^(\s*)\.\.CommandSpec::DEFAULT", text, _re.M)
    indent = m.group(1)
    return text[: m.start()] + f"{indent}{field_line}\n" + text[m.start() :]
