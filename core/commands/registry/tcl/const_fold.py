"""Compile-time constant folding callbacks for Tcl core commands.

Each function takes a tuple of resolved string arguments and returns
the result string, or ``None`` if the arguments are invalid or the
operation cannot be folded.

Results containing Tcl-special characters (semicolons, brackets, etc.)
are rejected to avoid semantic changes when the optimiser inlines
them as bare words.

These callbacks are referenced by the ``const_fold`` field on
``CommandSpec`` and ``SubCommand`` instances in the command registry.
SCCP calls them when all arguments resolve to known constants.
"""

from __future__ import annotations


# ---------------------------------------------------------------------------
# list / concat / join / split / lindex / lrange / llength / lreverse / lrepeat
# ---------------------------------------------------------------------------

def fold_list(args: tuple[str, ...]) -> str | None:
    """``list arg ...`` — returns a proper Tcl list."""
    # list just joins args as proper list elements.  For constant
    # folding we join with spaces (safe for simple values).
    return " ".join(args)


def fold_concat(args: tuple[str, ...]) -> str | None:
    """``concat ...`` — concatenates lists/strings with trimming."""
    return " ".join(a.strip() for a in args if a.strip())


def fold_join(args: tuple[str, ...]) -> str | None:
    """``join list ?joinString?``"""
    if len(args) < 1 or len(args) > 2:
        return None
    elements = _split_list(args[0])
    if elements is None:
        return None
    sep = args[1] if len(args) == 2 else " "
    return sep.join(elements)


def fold_split(args: tuple[str, ...]) -> str | None:
    """``split string ?splitChars?``"""
    if len(args) < 1 or len(args) > 2:
        return None
    s = args[0]
    chars = args[1] if len(args) == 2 else " \t\n"
    if not chars:
        # split on every character
        return " ".join(s)
    # Simple single-char split
    if len(chars) == 1:
        return " ".join(s.split(chars))
    # Multi-char splitChars: split on any char in the set
    result: list[str] = []
    current: list[str] = []
    for ch in s:
        if ch in chars:
            result.append("".join(current))
            current = []
        else:
            current.append(ch)
    result.append("".join(current))
    return " ".join(result)


def fold_lindex(args: tuple[str, ...]) -> str | None:
    """``lindex list ?index ...?``"""
    if len(args) < 1:
        return None
    elements = _split_list(args[0])
    if elements is None:
        return None
    if len(args) == 1:
        return args[0]
    # Single index case
    if len(args) == 2:
        idx = _parse_index(args[1], len(elements))
        if idx is None:
            return None
        if 0 <= idx < len(elements):
            return elements[idx]
        return ""
    # Nested indexing
    current = args[0]
    for idx_str in args[1:]:
        elems = _split_list(current)
        if elems is None:
            return None
        idx = _parse_index(idx_str, len(elems))
        if idx is None:
            return None
        if 0 <= idx < len(elems):
            current = elems[idx]
        else:
            return ""
    return current


def fold_lrange(args: tuple[str, ...]) -> str | None:
    """``lrange list first last``"""
    if len(args) != 3:
        return None
    elements = _split_list(args[0])
    if elements is None:
        return None
    first = _parse_index(args[1], len(elements))
    last = _parse_index(args[2], len(elements))
    if first is None or last is None:
        return None
    first = max(first, 0)
    last = min(last, len(elements) - 1)
    if first > last:
        return ""
    return " ".join(elements[first : last + 1])


def fold_llength(args: tuple[str, ...]) -> str | None:
    """``llength list``"""
    if len(args) != 1:
        return None
    elements = _split_list(args[0])
    if elements is None:
        return None
    return str(len(elements))


def fold_lreverse(args: tuple[str, ...]) -> str | None:
    """``lreverse list``"""
    if len(args) != 1:
        return None
    elements = _split_list(args[0])
    if elements is None:
        return None
    return " ".join(reversed(elements))


def fold_lrepeat(args: tuple[str, ...]) -> str | None:
    """``lrepeat count ?element ...?``"""
    if len(args) < 2:
        return None
    try:
        count = int(args[0])
    except ValueError:
        return None
    if count < 0 or count > 1000:  # sanity limit
        return None
    elements = list(args[1:])
    return " ".join(elements * count)


# ---------------------------------------------------------------------------
# string subcommands
# ---------------------------------------------------------------------------

def fold_string_cat(args: tuple[str, ...]) -> str | None:
    """``string cat ?string ...?``"""
    return "".join(args)


def fold_string_compare(args: tuple[str, ...]) -> str | None:
    """``string compare ?-nocase? ?-length N? string1 string2``"""
    nocase = False
    length = -1
    i = 0
    while i < len(args) and args[i].startswith("-"):
        if args[i] == "-nocase":
            nocase = True
            i += 1
        elif args[i] == "-length" and i + 1 < len(args):
            try:
                length = int(args[i + 1])
            except ValueError:
                return None
            i += 2
        elif args[i] == "--":
            i += 1
            break
        else:
            return None
    if len(args) - i != 2:
        return None
    s1, s2 = args[i], args[i + 1]
    if nocase:
        s1, s2 = s1.lower(), s2.lower()
    if length >= 0:
        s1, s2 = s1[:length], s2[:length]
    return str((s1 > s2) - (s1 < s2))


def fold_string_equal(args: tuple[str, ...]) -> str | None:
    """``string equal ?-nocase? ?-length N? string1 string2``"""
    result = fold_string_compare(args)
    if result is None:
        return None
    return "1" if result == "0" else "0"


def fold_string_first(args: tuple[str, ...]) -> str | None:
    """``string first needleString haystackString ?startIndex?``"""
    if len(args) < 2 or len(args) > 3:
        return None
    needle, haystack = args[0], args[1]
    start = 0
    if len(args) == 3:
        idx = _parse_index(args[2], len(haystack))
        if idx is None:
            return None
        start = max(idx, 0)
    return str(haystack.find(needle, start))


def fold_string_index(args: tuple[str, ...]) -> str | None:
    """``string index string charIndex``"""
    if len(args) != 2:
        return None
    s = args[0]
    idx = _parse_index(args[1], len(s))
    if idx is None:
        return None
    if 0 <= idx < len(s):
        return s[idx]
    return ""


def fold_string_is(args: tuple[str, ...]) -> str | None:
    """``string is class ?-strict? ?-failindex var? string``"""
    if len(args) < 2:
        return None
    cls = args[0]
    # Skip options
    i = 1
    strict = False
    while i < len(args) and args[i].startswith("-"):
        if args[i] == "-strict":
            strict = True
            i += 1
        elif args[i] == "-failindex":
            # Can't fold — writes to variable
            return None
        else:
            return None
    if i >= len(args):
        return None
    s = args[i]
    return _string_is(cls, s, strict)


def fold_string_last(args: tuple[str, ...]) -> str | None:
    """``string last needleString haystackString ?lastIndex?``"""
    if len(args) < 2 or len(args) > 3:
        return None
    needle, haystack = args[0], args[1]
    end = len(haystack)
    if len(args) == 3:
        idx = _parse_index(args[2], len(haystack))
        if idx is None:
            return None
        end = idx + 1
    return str(haystack.rfind(needle, 0, end))


def fold_string_length(args: tuple[str, ...]) -> str | None:
    """``string length string``"""
    if len(args) != 1:
        return None
    return str(len(args[0]))


def fold_string_map(args: tuple[str, ...]) -> str | None:
    """``string map ?-nocase? mapping string``"""
    nocase = False
    i = 0
    if i < len(args) and args[i] == "-nocase":
        nocase = True
        i += 1
    if len(args) - i != 2:
        return None
    mapping_str, s = args[i], args[i + 1]
    pairs = _split_list(mapping_str)
    if pairs is None or len(pairs) % 2 != 0:
        return None
    # Build replacement pairs
    replacements = [(pairs[j], pairs[j + 1]) for j in range(0, len(pairs), 2)]
    result: list[str] = []
    pos = 0
    while pos < len(s):
        matched = False
        for old, new in replacements:
            if not old:
                continue
            if nocase:
                if s[pos : pos + len(old)].lower() == old.lower():
                    result.append(new)
                    pos += len(old)
                    matched = True
                    break
            else:
                if s[pos : pos + len(old)] == old:
                    result.append(new)
                    pos += len(old)
                    matched = True
                    break
        if not matched:
            result.append(s[pos])
            pos += 1
    return "".join(result)


def fold_string_match(args: tuple[str, ...]) -> str | None:
    """``string match ?-nocase? pattern string``"""
    # Too complex for reliable folding (glob patterns)
    return None


def fold_string_range(args: tuple[str, ...]) -> str | None:
    """``string range string first last``"""
    if len(args) != 3:
        return None
    s = args[0]
    first = _parse_index(args[1], len(s))
    last = _parse_index(args[2], len(s))
    if first is None or last is None:
        return None
    first = max(first, 0)
    last = min(last, len(s) - 1)
    if first > last:
        return ""
    return s[first : last + 1]


def fold_string_repeat(args: tuple[str, ...]) -> str | None:
    """``string repeat string count``"""
    if len(args) != 2:
        return None
    try:
        count = int(args[1])
    except ValueError:
        return None
    if count < 0 or count > 10000:
        return None
    return args[0] * count


def fold_string_replace(args: tuple[str, ...]) -> str | None:
    """``string replace string first last ?newString?``"""
    if len(args) < 3 or len(args) > 4:
        return None
    s = args[0]
    first = _parse_index(args[1], len(s))
    last = _parse_index(args[2], len(s))
    if first is None or last is None:
        return None
    first = max(first, 0)
    last = min(last, len(s) - 1)
    replacement = args[3] if len(args) == 4 else ""
    if first > last:
        return s
    return s[:first] + replacement + s[last + 1 :]


def fold_string_reverse(args: tuple[str, ...]) -> str | None:
    """``string reverse string``"""
    if len(args) != 1:
        return None
    return args[0][::-1]


def fold_string_tolower(args: tuple[str, ...]) -> str | None:
    """``string tolower string ?first? ?last?``"""
    if len(args) < 1 or len(args) > 3:
        return None
    s = args[0]
    if len(args) == 1:
        return s.lower()
    first = _parse_index(args[1], len(s))
    if first is None:
        return None
    last = _parse_index(args[2], len(s)) if len(args) == 3 else len(s) - 1
    if last is None:
        return None
    first, last = max(first, 0), min(last, len(s) - 1)
    return s[:first] + s[first : last + 1].lower() + s[last + 1 :]


def fold_string_totitle(args: tuple[str, ...]) -> str | None:
    """``string totitle string ?first? ?last?``"""
    if len(args) < 1 or len(args) > 3:
        return None
    s = args[0]
    if len(args) == 1:
        return s[:1].upper() + s[1:].lower() if s else ""
    # With indices — complex, skip
    return None


def fold_string_toupper(args: tuple[str, ...]) -> str | None:
    """``string toupper string ?first? ?last?``"""
    if len(args) < 1 or len(args) > 3:
        return None
    s = args[0]
    if len(args) == 1:
        return s.upper()
    first = _parse_index(args[1], len(s))
    if first is None:
        return None
    last = _parse_index(args[2], len(s)) if len(args) == 3 else len(s) - 1
    if last is None:
        return None
    first, last = max(first, 0), min(last, len(s) - 1)
    return s[:first] + s[first : last + 1].upper() + s[last + 1 :]


def fold_string_trim(args: tuple[str, ...]) -> str | None:
    """``string trim string ?chars?``"""
    if len(args) < 1 or len(args) > 2:
        return None
    if len(args) == 1:
        return args[0].strip()
    return args[0].strip(args[1])


def fold_string_trimleft(args: tuple[str, ...]) -> str | None:
    """``string trimleft string ?chars?``"""
    if len(args) < 1 or len(args) > 2:
        return None
    if len(args) == 1:
        return args[0].lstrip()
    return args[0].lstrip(args[1])


def fold_string_trimright(args: tuple[str, ...]) -> str | None:
    """``string trimright string ?chars?``"""
    if len(args) < 1 or len(args) > 2:
        return None
    if len(args) == 1:
        return args[0].rstrip()
    return args[0].rstrip(args[1])


def fold_string_wordend(args: tuple[str, ...]) -> str | None:
    """``string wordend string charIndex``"""
    # Complex (Unicode word boundaries) — skip
    return None


def fold_string_wordstart(args: tuple[str, ...]) -> str | None:
    """``string wordstart string charIndex``"""
    return None


# ---------------------------------------------------------------------------
# dict subcommands
# ---------------------------------------------------------------------------

def fold_dict_create(args: tuple[str, ...]) -> str | None:
    """``dict create ?key value ...?``"""
    if len(args) % 2 != 0:
        return None
    return " ".join(args)


def fold_dict_exists(args: tuple[str, ...]) -> str | None:
    """``dict exists dictionary key ?key ...?``"""
    if len(args) < 2:
        return None
    d = _parse_dict(args[0])
    if d is None:
        return None
    key = args[1]
    return "1" if key in d else "0"


def fold_dict_get(args: tuple[str, ...]) -> str | None:
    """``dict get dictionary ?key ...?``"""
    if len(args) < 1:
        return None
    if len(args) == 1:
        return args[0]
    d = _parse_dict(args[0])
    if d is None:
        return None
    # Walk nested keys
    for key in args[1:]:
        if key not in d:
            return None  # key not found — runtime error
        val = d[key]
        if args.index(key) < len(args) - 2:
            # More keys to go — parse as nested dict
            d = _parse_dict(val)
            if d is None:
                return None
    return d.get(args[-1])


def fold_dict_keys(args: tuple[str, ...]) -> str | None:
    """``dict keys dictionary ?globPattern?``"""
    if len(args) < 1 or len(args) > 2:
        return None
    d = _parse_dict(args[0])
    if d is None:
        return None
    keys = list(d.keys())
    if len(args) == 2:
        # Glob filtering — skip for simplicity
        return None
    return " ".join(keys)


def fold_dict_merge(args: tuple[str, ...]) -> str | None:
    """``dict merge ?dictionary ...?``"""
    result: dict[str, str] = {}
    for arg in args:
        d = _parse_dict(arg)
        if d is None:
            return None
        result.update(d)
    parts: list[str] = []
    for k, v in result.items():
        parts.extend([k, v])
    return " ".join(parts)


def fold_dict_size(args: tuple[str, ...]) -> str | None:
    """``dict size dictionary``"""
    if len(args) != 1:
        return None
    d = _parse_dict(args[0])
    if d is None:
        return None
    return str(len(d))


def fold_dict_values(args: tuple[str, ...]) -> str | None:
    """``dict values dictionary ?globPattern?``"""
    if len(args) < 1 or len(args) > 2:
        return None
    if len(args) == 2:
        return None  # glob filtering
    d = _parse_dict(args[0])
    if d is None:
        return None
    return " ".join(d.values())


# ---------------------------------------------------------------------------
# format
# ---------------------------------------------------------------------------

def fold_format(args: tuple[str, ...]) -> str | None:
    """``format formatString ?arg ...?``"""
    if len(args) < 1:
        return None
    fmt = args[0]
    vals = args[1:]
    try:
        # Convert Tcl format spec to Python — basic %s, %d, %f, %x
        result = _tcl_format(fmt, vals)
        return result
    except Exception:
        return None


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _split_list(s: str) -> list[str] | None:
    """Split a Tcl list string into elements (simple cases)."""
    from ....compiler.tcl_expr_eval import _split_tcl_list

    try:
        return _split_tcl_list(s)
    except Exception:
        return None


def _parse_dict(s: str) -> dict[str, str] | None:
    """Parse a flat Tcl dict string into a Python dict."""
    elements = _split_list(s)
    if elements is None or len(elements) % 2 != 0:
        return None
    return {elements[i]: elements[i + 1] for i in range(0, len(elements), 2)}


def _parse_index(s: str, length: int) -> int | None:
    """Parse a Tcl index expression (integer, end, end-N)."""
    s = s.strip()
    if s == "end":
        return length - 1
    if s.startswith("end-"):
        try:
            return length - 1 - int(s[4:])
        except ValueError:
            return None
    if s.startswith("end+"):
        try:
            return length - 1 + int(s[4:])
        except ValueError:
            return None
    try:
        return int(s)
    except ValueError:
        return None


def _string_is(cls: str, s: str, strict: bool) -> str | None:
    """Evaluate ``string is class`` for compile-time known values."""
    if not strict and not s:
        return "1"
    checks: dict[str, object] = {
        "alnum": str.isalnum,
        "alpha": str.isalpha,
        "ascii": lambda x: all(ord(c) < 128 for c in x),
        "boolean": lambda x: x.lower() in ("0", "1", "true", "false", "yes", "no", "on", "off"),
        "digit": str.isdigit,
        "double": lambda x: _is_double(x),
        "entier": lambda x: _is_int(x),
        "false": lambda x: x.lower() in ("0", "false", "no", "off"),
        "graph": lambda x: all(c.isprintable() and not c.isspace() for c in x),
        "integer": lambda x: _is_int(x),
        "list": lambda x: _split_list(x) is not None,
        "lower": str.islower,
        "print": lambda x: all(c.isprintable() for c in x),
        "punct": lambda x: all(not c.isalnum() and not c.isspace() and c.isprintable() for c in x),
        "space": str.isspace,
        "true": lambda x: x.lower() in ("1", "true", "yes", "on"),
        "upper": str.isupper,
        "wideinteger": lambda x: _is_int(x),
        "wordchar": lambda x: all(c.isalnum() or c == "_" for c in x),
        "xdigit": lambda x: all(c in "0123456789abcdefABCDEF" for c in x),
    }
    check = checks.get(cls)
    if check is None:
        return None
    try:
        return "1" if check(s) else "0"
    except Exception:
        return None


def _is_double(s: str) -> bool:
    try:
        float(s)
        return True
    except ValueError:
        return False


def _is_int(s: str) -> bool:
    s = s.strip()
    if s.startswith(("0x", "0X")):
        try:
            int(s, 16)
            return True
        except ValueError:
            return False
    if s.startswith(("0o", "0O")):
        try:
            int(s, 8)
            return True
        except ValueError:
            return False
    if s.startswith(("0b", "0B")):
        try:
            int(s, 2)
            return True
        except ValueError:
            return False
    try:
        int(s)
        return True
    except ValueError:
        return False


def _tcl_format(fmt: str, vals: tuple[str, ...]) -> str | None:
    """Basic Tcl format → Python format conversion for %s, %d, %f, %x."""
    result: list[str] = []
    vi = 0
    i = 0
    while i < len(fmt):
        if fmt[i] == "%" and i + 1 < len(fmt):
            i += 1
            if fmt[i] == "%":
                result.append("%")
                i += 1
                continue
            # Skip flags and width
            while i < len(fmt) and fmt[i] in "-+ 0#":
                i += 1
            while i < len(fmt) and fmt[i].isdigit():
                i += 1
            if i < len(fmt) and fmt[i] == ".":
                i += 1
                while i < len(fmt) and fmt[i].isdigit():
                    i += 1
            if i >= len(fmt) or vi >= len(vals):
                return None
            spec = fmt[i]
            val = vals[vi]
            vi += 1
            if spec == "s":
                result.append(val)
            elif spec == "d":
                try:
                    result.append(str(int(val)))
                except ValueError:
                    return None
            elif spec in ("f", "e", "g"):
                try:
                    result.append(str(float(val)))
                except ValueError:
                    return None
            elif spec in ("x", "X"):
                try:
                    n = int(val)
                    result.append(format(n, spec))
                except ValueError:
                    return None
            elif spec == "c":
                try:
                    result.append(chr(int(val)))
                except (ValueError, OverflowError):
                    return None
            else:
                return None
            i += 1
        else:
            result.append(fmt[i])
            i += 1
    return "".join(result)
