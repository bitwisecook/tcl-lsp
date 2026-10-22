#!/usr/bin/env python3
"""Wrap call expressions in Box::pin(...).

Usage: boxpin.py <file> <call-prefix> [<call-prefix>...]
Each call-prefix is the exact text that starts the call, ending in '(' (for
example 'compute_base_analysis(' or 'self.ensure_current_diagnostic_report(').
Every occurrence not preceded by 'fn ' or already by 'Box::pin(' is wrapped:
the matching ')' is found by counting parentheses (strings are skipped).
"""
import sys

path, *prefixes = sys.argv[1:]
src = open(path).read()


def matching_paren(s, i):
    depth = 0
    j = i
    in_str = False
    while j < len(s):
        c = s[j]
        if in_str:
            if c == "\\":
                j += 2
                continue
            if c == '"':
                in_str = False
        elif c == '"':
            in_str = True
        elif c == "(":
            depth += 1
        elif c == ")":
            depth -= 1
            if depth == 0:
                return j
        j += 1
    raise SystemExit("unbalanced")


count = 0
for prefix in prefixes:
    pos = 0
    while True:
        i = src.find(prefix, pos)
        if i < 0:
            break
        before = src[max(0, i - 12):i]
        if before.endswith("fn ") or before.endswith("Box::pin("):
            pos = i + len(prefix)
            continue
        # A method call like `self\n.foo(` is matched on `.foo(`; wrap from the receiver.
        start = i
        if prefix.startswith("."):
            k = i
            while k > 0 and src[k - 1] in " \n\t":
                k -= 1
            # walk back over the receiver identifier path
            while k > 0 and (src[k - 1].isalnum() or src[k - 1] in "_.&"):
                k -= 1
            start = k
        open_paren = i + len(prefix) - 1
        close = matching_paren(src, open_paren)
        src = src[:start] + "Box::pin(" + src[start:close + 1] + ")" + src[close + 1:]
        count += 1
        pos = close + len("Box::pin(") + 1
print(path, count, "wrapped")
open(path, "w").write(src)
