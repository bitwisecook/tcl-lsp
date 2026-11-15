#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Extract each classic Tk widget's option table from the Tk C sources.

Reads the `Tk_OptionSpec` / `Tk_ConfigSpec` tables in `tmp/tk<release>/generic`
and writes one text file per release under
`rust/tcl-registry/tests/data/tk_option_tables/`: one line per widget command,
its name followed by the sorted option spellings its table accepts, synonyms
(`-bd`, `-vcmd`) included.
The registry test `tk_option_tables_match_the_c_sources` compares the shipped
specs with these files.

Usage: scripts/dev/gen-tk-option-tables.py [tmp-dir]
"""

import pathlib
import re
import sys

RELEASES = ["8.4.20", "8.5.19", "8.6.18", "9.0.4", "9.1.0"]

# Widget command -> (source file, candidate table names). The frame family
# chains its own table to `commonOptSpec`, so both contribute.
TABLES = {
    "label": ("tkButton.c", ["labelOptionSpecs"]),
    "button": ("tkButton.c", ["buttonOptionSpecs"]),
    "checkbutton": ("tkButton.c", ["checkbuttonOptionSpecs"]),
    "radiobutton": ("tkButton.c", ["radiobuttonOptionSpecs"]),
    "canvas": ("tkCanvas.c", ["configSpecs", "optionSpecs"]),
    "entry": ("tkEntry.c", ["entryOptSpec"]),
    "spinbox": ("tkEntry.c", ["sbOptSpec"]),
    "frame": ("tkFrame.c", ["frameOptSpec", "commonOptSpec"]),
    "toplevel": ("tkFrame.c", ["toplevelOptSpec", "commonOptSpec"]),
    "labelframe": ("tkFrame.c", ["labelframeOptSpec", "commonOptSpec"]),
    "listbox": ("tkListbox.c", ["optionSpecs"]),
    "menu": ("tkMenu.c", ["tkMenuConfigSpecs"]),
    "menubutton": ("tkMenubutton.c", ["optionSpecs"]),
    "message": ("tkMessage.c", ["optionSpecs"]),
    "panedwindow": ("tkPanedWindow.c", ["optionSpecs"]),
    "scale": ("tkScale.c", ["optionSpecs"]),
    "scrollbar": ("tkScrollbar.c", ["tkpScrollbarConfigSpecs", "configSpecs", "optionSpecs"]),
    "text": ("tkText.c", ["optionSpecs", "configSpecs"]),
}

COMBINED = {"frame", "toplevel", "labelframe"}


def table(source: str, name: str) -> set[str] | None:
    match = re.search(
        r"(?:static\s+)?(?:const\s+|CONST\s+)?Tk_(?:Option|Config)Spec\s+"
        + name
        + r"\[\]\s*=\s*\{(.*?)\n\};",
        source,
        re.S,
    )
    if not match:
        return None
    return set(
        re.findall(r'\{\s*TK_(?:OPTION|CONFIG)_[A-Z_]+,\s*"(-[a-zA-Z0-9]+)"', match.group(1))
    )


def release_tables(tk: pathlib.Path) -> dict[str, list[str]]:
    out = {}
    for widget, (file, names) in TABLES.items():
        source = (tk / "generic" / file).read_text(errors="ignore")
        options: set[str] = set()
        for name in names:
            found = table(source, name)
            if found is None:
                continue
            options |= found
            if widget not in COMBINED:
                break
        if not options:
            raise SystemExit(f"no option table for {widget} in {tk}")
        out[widget] = sorted(options)
    return out


def main() -> None:
    tmp = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else "tmp")
    target = pathlib.Path("rust/tcl-registry/tests/data/tk_option_tables")
    target.mkdir(parents=True, exist_ok=True)
    for release in RELEASES:
        tables = release_tables(tmp / f"tk{release}")
        lines = [" ".join([widget, *options]) for widget, options in sorted(tables.items())]
        (target / f"tk{release}.txt").write_text("\n".join(lines) + "\n")


if __name__ == "__main__":
    main()
