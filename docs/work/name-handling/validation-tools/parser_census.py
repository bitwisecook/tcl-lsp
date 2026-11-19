"""List first-party source files and lexical leads for the parser ownership review.

From the repo root run python3 docs/work/name-handling/validation-tools/parser_census.py.
Use --sites for a file and line index. Hits require caller and grammar review.
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


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--sites", action="store_true")
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
        component = next(
            (parent for parent in path.parents if parent in packages), None
        )
        if component is None:
            component = (
                Path(*path.parts[:2])
                if path.parts[0] in {"editors", "grammars"}
                else Path(path.parts[0])
            )
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
