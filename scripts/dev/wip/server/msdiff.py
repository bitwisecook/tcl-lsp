import re
import sys
from collections import defaultdict

NODE = re.compile(r"^(\s*)n(\d+): (\d+) (?:0x[0-9A-F]+: )?(.*)$")


def parse(path):
    """Inclusive bytes per function, counting a function once per path."""
    text = open(path).read()
    tree = text[text.index("heap_tree=") :].splitlines()[1:]
    inclusive = defaultdict(int)
    stack = []  # (indent, name)
    total = None
    for line in tree:
        m = NODE.match(line)
        if not m:
            continue
        indent, _, size, name = len(m.group(1)), m.group(2), int(m.group(3)), m.group(4)
        name = re.sub(r" \(in /.*\)$", "", name)
        name = re.sub(r" \([^()]*:\d+\)$", "", name)
        while stack and stack[-1][0] >= indent:
            stack.pop()
        if total is None:
            total = size
        ancestors = {n for _, n in stack}
        if name not in ancestors:
            inclusive[name] += size
        stack.append((indent, name))
    return total, inclusive


def main():
    a_total, a = parse(sys.argv[1])
    b_total, b = parse(sys.argv[2])
    print(f"total {a_total/1e6:.1f} MB -> {b_total/1e6:.1f} MB")
    skip = re.compile(r"^(alloc::|core::|std::|hashbrown::|__rust|malloc|realloc|calloc|<alloc|<core|<std|<hashbrown|\?\?\?|_?start|clone|tokio::|<tokio)")
    rows = sorted(((b.get(k, 0) - a.get(k, 0), k) for k in set(a) | set(b)), reverse=True)
    shown = 0
    for delta, name in rows:
        if skip.search(name):
            continue
        print(f"{delta/1e6:8.2f} MB  {b.get(name,0)/1e6:8.1f}  {name[:200]}")
        shown += 1
        if shown >= int(sys.argv[3] if len(sys.argv) > 3 else 40):
            break


main()
