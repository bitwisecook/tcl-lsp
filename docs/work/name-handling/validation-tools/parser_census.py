"""List first-party source files and lexical leads for the parser ownership review.

From the repo root run python3 docs/work/name-handling/validation-tools/parser_census.py.
Use --sites for lexical leads. Use --files for every tracked path, including paths
with no lexical lead. A surface classification describes review scope, not sign-off.
"""

import argparse
import re
import subprocess
from collections import Counter
from pathlib import Path

ROOTS = {"rust", "runtime", "editors", "grammars", "scripts", "ai", "specs"}
SUFFIXES = {
    ".rs",
    ".c",
    ".h",
    ".cpp",
    ".cc",
    ".ts",
    ".tsx",
    ".js",
    ".mjs",
    ".kt",
    ".kts",
    ".py",
    ".tcl",
    ".tclspec",
    ".el",
    ".lua",
    ".sh",
    ".scm",
}
SKIP = {
    "vendor",
    "third_party",
    "node_modules",
    "test",
    "tests",
    "testdata",
    "testFixture",
    "fixtures",
    "benches",
    "examples",
    "experiments",
    "evidence",
    "gen",
    "generated",
}
LEADS = {
    "reader/writer declaration": re.compile(
        r"\b(?:fn|def|function|fun|proc)\s+(?:\w*::)?"
        r"(?:parse|scan|lex|tokeni[sz]e|split|join|quote|escape|decode|encode|"
        r"seriali[sz]e|segment|classify|normalise|normalize|render)(?:\w*|[-:]\S*)"
    ),
    "host primitive": re.compile(
        r"\b(?:split_whitespace|split_ascii_whitespace|split_once|rsplit_once|"
        r"splitn|rsplitn|trim_matches|trim_start_matches|from_str_radix)\s*\("
        r"|\.parse\s*::\s*<"
    ),
    "owner call or wrapper": re.compile(
        r"\b(?:split_(?:native_)?list\w*|join_list|append_list_element|"
        r"group_commands\w*|segment_commands\w*|scan_var_ref|split_element_ref\w*|"
        r"parse_formal_parameters\w*|parse_conversion|parse_spec|"
        r"command_substitution_end|parse_expression\w*)\s*\("
    ),
}


def component_for(path: Path, packages: set[Path]) -> str:
    package = next((parent for parent in path.parents if parent in packages), None)
    if package is not None:
        return str(package)
    if path.parts[0] in {"editors", "grammars"}:
        return str(Path(*path.parts[:2]))
    return path.parts[0] if len(path.parts) > 1 else "."


def surface_for(path: Path) -> str:
    """Describe the tracked input. Never discard a new root, language or extension."""
    parts = set(path.parts)
    top = path.parts[0]
    if parts & {"vendor", "third_party", "node_modules"}:
        return "vendored"
    if parts & {"tests", "testdata", "testFixture", "fixtures", "benches", "examples"}:
        return "test-or-example"
    if path.name == "tests.rs" or path.stem.endswith(("_tests", "_test", ".test")):
        return "test-or-example"
    if top in {"samples", "tests", "experiments", "evidence"}:
        return "corpus-or-oracle"
    if parts & {"gen", "generated"} or (top == "grammars" and path.name == "parser.c"):
        return "generated"
    if (
        top == "grammars"
        or "syntaxes" in parts
        or path.suffix in {".scm", ".sublime-syntax"}
    ):
        return "grammar-or-editor-rules"
    if (
        path.suffix
        in {".tclspec", ".tm", ".jim", ".irule", ".irules", ".apl", ".iapp", ".scf"}
        or top == "specs"
    ):
        return "pack-or-script"
    if path.suffix in SUFFIXES | {".swift", ".pyi", ".cjs", ".j2", ".jinja", ".bat"}:
        return "source"
    if (
        path.name in {"Makefile", "Cargo.lock", "Cargo.toml", "CMakeLists.txt"}
        or top == ".github"
    ):
        return "build-or-workflow"
    if path.suffix == ".conf" and "bigip-probes" in parts:
        return "device-probe"
    if path.suffix in {
        ".json",
        ".jsonc",
        ".jsonl",
        ".toml",
        ".yaml",
        ".yml",
        ".xml",
        ".tsv",
        ".csv",
        ".conf",
    }:
        return "declarative-data"
    if path.suffix in {".md", ".rst"}:
        return "documentation"
    return "other-review"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    modes = parser.add_mutually_exclusive_group()
    modes.add_argument("--sites", action="store_true")
    modes.add_argument(
        "--files",
        action="store_true",
        help="list every tracked path and its review surface",
    )
    args = parser.parse_args()
    root = Path(
        subprocess.check_output(
            ["git", "rev-parse", "--show-toplevel"], text=True
        ).strip()
    )
    tracked = (
        subprocess.check_output(["git", "-C", str(root), "ls-files", "-z"])
        .decode()
        .split("\0")
    )
    packages = {Path(name).parent for name in tracked if name.endswith("/Cargo.toml")}
    if args.files:
        print("component\tsurface\tpath")
        for name in tracked:
            if name:
                path = Path(name)
                print(component_for(path, packages), surface_for(path), name, sep="\t")
        return
    totals = {}
    if args.sites:
        print("component\tpath\tline\tlead\ttext")
    for name in tracked:
        path = Path(name)
        if not name or path.parts[0] not in ROOTS or path.suffix not in SUFFIXES:
            continue
        if SKIP.intersection(path.parts) or path.stem.endswith(
            ("_tests", "_test", ".test")
        ):
            continue
        if path.name == "tests.rs" or (
            path.parts[0] == "grammars" and path.name == "parser.c"
        ):
            continue
        component = component_for(path, packages)
        counts = totals.setdefault(str(component), Counter())
        counts["files"] += 1
        for index, line in enumerate(
            (root / path).read_text(errors="replace").splitlines(), 1
        ):
            if line.lstrip().startswith(("//", "/*", "* ", "# ")):
                continue
            for label, pattern in LEADS.items():
                if pattern.search(line):
                    counts[label] += 1
                    if args.sites:
                        print(
                            component,
                            path,
                            index,
                            label,
                            line.strip().replace("\t", " "),
                            sep="\t",
                        )
    if not args.sites:
        print("component", "files", *LEADS, sep="\t")
        for component, counts in sorted(totals.items()):
            print(component, counts["files"], *(counts[k] for k in LEADS), sep="\t")


if __name__ == "__main__":
    main()
