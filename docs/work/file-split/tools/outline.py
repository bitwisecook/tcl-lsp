#!/usr/bin/env python3
"""Print the outline of a Rust file: every top-level item and impl method
with its id, first line and length, for planning a split."""

import sys
from pathlib import Path

from rust_items import line_of, parse_items


def main() -> int:
    path = Path(sys.argv[1])
    floor = int(sys.argv[2]) if len(sys.argv) > 2 else 0
    text = path.read_text(encoding="utf-8")
    print(f"# {path} ({text.count(chr(10)) + 1} lines)")
    for item in parse_items(text):
        start, end = line_of(text, item.start), line_of(text, item.end - 1)
        length = end - start + 1
        if item.kind == "use":
            continue
        print(f"{start}\t{length}\t{item.visibility or '-'}\t{item.ident}")
        for method in item.methods:
            first, last = line_of(text, method.start), line_of(text, method.end - 1)
            if last - first + 1 >= floor:
                print(f"  {first}\t{last - first + 1}\t{method.visibility or '-'}\t{method.ident}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
