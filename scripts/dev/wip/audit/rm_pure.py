import pathlib, re, sys
TOKEN = re.compile(r"(?:crate::|tcl_registry::)?(?:traits::)?Traits::PURE\b(?!_)")
def in_comment_or_string(text, pos):
    line_start = text.rfind("\n", 0, pos) + 1
    line = text[line_start:pos]
    if "//" in line:
        return True
    return line.count('"') % 2 == 1
def matching(text, open_idx):
    depth = 0
    for k in range(open_idx, len(text)):
        if text[k] == "(":
            depth += 1
        elif text[k] == ")":
            depth -= 1
            if depth == 0:
                return k
    raise ValueError
changed = 0
for path in sys.argv[1:]:
    p = pathlib.Path(path); text = p.read_text(); orig = text
    while True:
        found = None
        for m in TOKEN.finditer(text):
            if not in_comment_or_string(text, m.start()):
                found = m; break
        if not found:
            break
        s, e = found.start(), found.end()
        line_start = text.rfind("\n", 0, s) + 1
        line_end = text.find("\n", e)
        line = text[line_start:line_end]
        after = text[e:]
        before = text[:s]
        m_union_after = re.match(r"\s*\.union\(", after)
        m_bar_after = re.match(r"\s*\|\s*", after)
        m_bar_before = re.search(r"\s*\|\s*$", before)
        m_union_wrap = re.search(r"\.union\(\s*$", before) and re.match(r"\s*\)", after)
        if re.fullmatch(r"\s*traits: " + re.escape(text[s:e]) + r",\s*", line):
            text = text[:line_start] + text[line_end + 1:]
        elif m_union_after:
            open_idx = e + m_union_after.end() - 1
            close = matching(text, open_idx)
            inner = text[open_idx + 1:close]
            text = text[:s] + inner.strip() + text[close + 1:]
        elif m_union_wrap:
            ws = re.search(r"\.union\(\s*$", before)
            close = e + re.match(r"\s*\)", after).end()
            text = text[:ws.start()] + text[close:]
        elif m_bar_before:
            text = text[:m_bar_before.start()] + text[e:]
        elif m_bar_after:
            text = text[:s] + text[e + m_bar_after.end():]
        else:
            text = text[:s] + "Traits::empty()" + text[e:]
    if text != orig:
        p.write_text(text); changed += 1
print("files changed", changed)
