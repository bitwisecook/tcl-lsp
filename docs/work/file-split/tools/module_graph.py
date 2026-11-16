#!/usr/bin/env python3
"""Show which top-level modules of a crate could leave it.

    module_graph.py CRATE_SRC

Reads the production text of each top-level module (test files and inline
test modules left out), finds which other top-level modules it names
(`crate::m`, a name the crate root re-exports from `m`, or `super::m` from the
module's own root file), and prints the strongly connected components, lowest
layer first. A module on layer 0 depends on nothing else in the crate, so it
can become a crate of its own; a component of many modules can only leave
whole. It over-approximates (a name in a comment counts), so it can miss a
clean boundary but not invent one.
"""

import re
import sys
from pathlib import Path

ROOT = "<root>"
TEST_MODULE = re.compile(r"^#\[cfg\(test\)\]\s*\n\s*(?:pub(?:\([^)]*\))? )?mod ", re.M)
ROOT_REEXPORT = re.compile(r"pub(?:\([^)]*\))? use (?:crate::|self::)?(\w+)::\{?([^;]+);")


def production(text: str) -> str:
    cut = TEST_MODULE.search(text)
    return text[: cut.start()] if cut else text


def is_test_file(path: Path) -> bool:
    return "tests" in path.parts or path.stem == "tests" or path.stem.endswith("_tests")


def module_files(src: Path) -> dict[str, list[Path]]:
    modules: dict[str, list[Path]] = {}
    for path in sorted(src.rglob("*.rs")):
        top = path.relative_to(src).parts[0].removesuffix(".rs")
        modules.setdefault(ROOT if top in ("lib", "main") else top, []).append(path)
    return modules


def dependencies(src: Path) -> tuple[dict[str, set[str]], dict[str, int]]:
    """What each top-level module names, and its production line count."""
    modules = module_files(src)
    names = set(modules) - {ROOT}
    root_text = "".join(production(path.read_text(errors="replace")) for path in modules.get(ROOT, []))
    reexported: dict[str, str] = {}
    for match in ROOT_REEXPORT.finditer(root_text):
        if match.group(1) in names:
            for name in re.findall(r"\b([A-Za-z_]\w*)\b(?!\s*::)", match.group(2)):
                reexported.setdefault(name, match.group(1))
    deps: dict[str, set[str]] = {name: set() for name in names}
    lines: dict[str, int] = {}
    for module in names:
        count = 0
        for path in modules[module]:
            if is_test_file(path):
                continue
            text = production(path.read_text(errors="replace"))
            count += text.count("\n")
            named = re.findall(r"\bcrate::(\w+)", text)
            for group in re.findall(r"\bcrate::\{([^;]+)\};", text):
                named += re.findall(r"(?:^|[,{]\s*)(\w+)", group)
            relative = path.relative_to(src).parts
            if len(relative) == 1 or (len(relative) == 2 and path.stem == "mod"):
                named += re.findall(r"\bsuper::(\w+)", text)
            for name in named:
                target = name if name in names else reexported.get(name)
                if target and target != module:
                    deps[module].add(target)
        lines[module] = count
    return deps, lines


def components(deps: dict[str, set[str]]) -> list[list[str]]:
    """Strongly connected components, each after the components it depends on."""
    index: dict[str, int] = {}
    low: dict[str, int] = {}
    on_stack: set[str] = set()
    stack: list[str] = []
    found: list[list[str]] = []
    for start in sorted(deps):
        if start in index:
            continue
        work = [(start, iter(sorted(deps[start])))]
        index[start] = low[start] = len(index)
        stack.append(start)
        on_stack.add(start)
        while work:
            node, rest = work[-1]
            for target in rest:
                if target not in index:
                    index[target] = low[target] = len(index)
                    stack.append(target)
                    on_stack.add(target)
                    work.append((target, iter(sorted(deps[target]))))
                    break
                if target in on_stack:
                    low[node] = min(low[node], index[target])
            else:
                work.pop()
                if work:
                    low[work[-1][0]] = min(low[work[-1][0]], low[node])
                if low[node] == index[node]:
                    component = []
                    while True:
                        member = stack.pop()
                        on_stack.discard(member)
                        component.append(member)
                        if member == node:
                            break
                    found.append(component)
    return found


def layers(deps: dict[str, set[str]], found: list[list[str]]) -> list[int]:
    """Each component's layer: 0 when it depends on no other component."""
    owner = {member: number for number, component in enumerate(found) for member in component}
    level: list[int] = []
    for number, component in enumerate(found):
        below = {owner[target] for member in component for target in deps[member]} - {number}
        level.append(1 + max((level[other] for other in below), default=-1))
    return level


def main() -> int:
    if len(sys.argv) != 2:
        print(__doc__.split("\n")[2].strip(), file=sys.stderr)
        return 2
    deps, lines = dependencies(Path(sys.argv[1]))
    found = components(deps)
    level = layers(deps, found)
    print(f"{len(deps)} modules, {sum(lines.values())} production lines, {len(found)} components")
    order = sorted(range(len(found)), key=lambda number: (level[number], -sum(lines[m] for m in found[number])))
    for number in order:
        members = sorted(found[number], key=lambda member: -lines[member])
        size = sum(lines[member] for member in members)
        if len(members) == 1 and size < 3000:
            continue
        shown = ", ".join(members[:12]) + (f", ... ({len(members)} modules)" if len(members) > 12 else "")
        print(f"layer {level[number]:<2} {size:7} lines  {len(members):3} modules  {shown}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
