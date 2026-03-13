"""The ``string`` command and its subcommands."""

from __future__ import annotations

from collections.abc import Callable
from typing import TYPE_CHECKING

from ..machine import _parse_index
from ..types import TclError, TclResult

if TYPE_CHECKING:
    from ..interp import TclInterp

_IS_CHECKS: dict[str, Callable[[str], bool]] = {
    "alnum": str.isalnum,
    "alpha": str.isalpha,
    "ascii": lambda s: all(ord(c) < 128 for c in s),
    "boolean": lambda s: s.lower() in ("0", "1", "true", "false", "yes", "no", "on", "off"),
    "control": lambda s: all(ord(c) < 32 or ord(c) == 127 for c in s) if s else False,
    "digit": str.isdigit,
    "double": lambda s: _is_double(s),
    "entier": lambda s: _is_integer(s),
    "false": lambda s: s.lower() in ("0", "false", "no", "off"),
    "graph": lambda s: all(c.isprintable() and not c.isspace() for c in s) if s else False,
    "integer": lambda s: _is_integer(s),
    "list": lambda s: True,  # simplified
    "lower": str.islower,
    "print": lambda s: all(c.isprintable() for c in s) if s else False,
    "punct": lambda s: (
        all(not c.isalnum() and not c.isspace() and c.isprintable() for c in s) if s else False
    ),
    "space": str.isspace,
    "true": lambda s: s.lower() in ("1", "true", "yes", "on"),
    "upper": str.isupper,
    "wideinteger": lambda s: _is_integer(s),
    "wordchar": lambda s: all(c.isalnum() or c == "_" for c in s) if s else False,
    "xdigit": lambda s: all(c in "0123456789abcdefABCDEF" for c in s) if s else False,
}


def _is_double(s: str) -> bool:
    try:
        float(s)
        return True
    except ValueError:
        return False


def _is_integer(s: str) -> bool:
    try:
        int(s, 0)
        return True
    except (ValueError, TypeError):
        return False


def _cmd_string(interp: TclInterp, args: list[str]) -> TclResult:
    """string subcommand ?arg ...?"""
    if not args:
        raise TclError('wrong # args: should be "string subcommand ?arg ...?"')
    sub = args[0]
    rest = args[1:]

    match sub:
        case "length":
            if len(rest) != 1:
                raise TclError('wrong # args: should be "string length string"')
            return TclResult(value=str(len(rest[0])))

        case "index":
            if len(rest) != 2:
                raise TclError('wrong # args: should be "string index string charIndex"')
            s = rest[0]
            idx = _parse_index(rest[1], len(s))
            if 0 <= idx < len(s):
                return TclResult(value=s[idx])
            return TclResult(value="")

        case "range":
            if len(rest) != 3:
                raise TclError('wrong # args: should be "string range string first last"')
            s = rest[0]
            first = _parse_index(rest[1], len(s))
            last = _parse_index(rest[2], len(s))
            first = max(0, first)
            last = min(len(s) - 1, last)
            if first > last:
                return TclResult(value="")
            return TclResult(value=s[first : last + 1])

        case "compare":
            nocase = False
            length = -1
            r = list(rest)
            while r and r[0].startswith("-"):
                if r[0] == "-nocase":
                    nocase = True
                    r.pop(0)
                elif r[0] == "-length" and len(r) > 1:
                    length = int(r[1])
                    r = r[2:]
                else:
                    r.pop(0)
            if len(r) != 2:
                raise TclError(
                    'wrong # args: should be "string compare ?-nocase? ?-length int? string1 string2"'
                )
            a, b = r[0], r[1]
            if length >= 0:
                a, b = a[:length], b[:length]
            if nocase:
                a, b = a.lower(), b.lower()
            if a < b:
                return TclResult(value="-1")
            if a > b:
                return TclResult(value="1")
            return TclResult(value="0")

        case "equal":
            nocase = False
            length = -1
            r = list(rest)
            while r and r[0].startswith("-"):
                if r[0] == "-nocase":
                    nocase = True
                    r.pop(0)
                elif r[0] == "-length" and len(r) > 1:
                    length = int(r[1])
                    r = r[2:]
                else:
                    r.pop(0)
            if len(r) != 2:
                raise TclError(
                    'wrong # args: should be "string equal ?-nocase? ?-length int? string1 string2"'
                )
            a, b = r[0], r[1]
            if length >= 0:
                a, b = a[:length], b[:length]
            if nocase:
                a, b = a.lower(), b.lower()
            return TclResult(value="1" if a == b else "0")

        case "match":
            nocase = False
            r = list(rest)
            if r and r[0] == "-nocase":
                nocase = True
                r.pop(0)
            if len(r) != 2:
                raise TclError('wrong # args: should be "string match ?-nocase? pattern string"')
            import fnmatch

            pattern, string = r[0], r[1]
            if nocase:
                matched = fnmatch.fnmatch(string.lower(), pattern.lower())
            else:
                matched = fnmatch.fnmatch(string, pattern)
            return TclResult(value="1" if matched else "0")

        case "map":
            nocase = False
            r = list(rest)
            if r and r[0] == "-nocase":
                nocase = True
                r.pop(0)
            if len(r) != 2:
                raise TclError('wrong # args: should be "string map ?-nocase? mapping string"')
            from ..machine import _split_list

            mapping_list = _split_list(r[0])
            if len(mapping_list) % 2 != 0:
                raise TclError("list must have an even number of elements")
            string = r[1]
            pairs = list(zip(mapping_list[::2], mapping_list[1::2]))
            result = _string_map(string, pairs, nocase)
            return TclResult(value=result)

        case "first":
            if len(rest) < 2:
                raise TclError(
                    'wrong # args: should be "string first needleString haystackString ?startIndex?"'
                )
            needle, haystack = rest[0], rest[1]
            start = _parse_index(rest[2], len(haystack)) if len(rest) > 2 else 0
            idx = haystack.find(needle, max(0, start))
            return TclResult(value=str(idx))

        case "last":
            if len(rest) < 2:
                raise TclError(
                    'wrong # args: should be "string last needleString haystackString ?lastIndex?"'
                )
            needle, haystack = rest[0], rest[1]
            end = _parse_index(rest[2], len(haystack)) + 1 if len(rest) > 2 else len(haystack)
            idx = haystack.rfind(needle, 0, end)
            return TclResult(value=str(idx))

        case "repeat":
            if len(rest) != 2:
                raise TclError('wrong # args: should be "string repeat string count"')
            return TclResult(value=rest[0] * int(rest[1]))

        case "reverse":
            if len(rest) != 1:
                raise TclError('wrong # args: should be "string reverse string"')
            return TclResult(value=rest[0][::-1])

        case "tolower":
            if not rest:
                raise TclError('wrong # args: should be "string tolower string ?first? ?last?"')
            s = rest[0]
            if len(rest) >= 3:
                first = _parse_index(rest[1], len(s))
                last = _parse_index(rest[2], len(s))
                return TclResult(value=s[:first] + s[first : last + 1].lower() + s[last + 1 :])
            return TclResult(value=s.lower())

        case "toupper":
            if not rest:
                raise TclError('wrong # args: should be "string toupper string ?first? ?last?"')
            s = rest[0]
            if len(rest) >= 3:
                first = _parse_index(rest[1], len(s))
                last = _parse_index(rest[2], len(s))
                return TclResult(value=s[:first] + s[first : last + 1].upper() + s[last + 1 :])
            return TclResult(value=s.upper())

        case "totitle":
            if not rest:
                raise TclError('wrong # args: should be "string totitle string ?first? ?last?"')
            s = rest[0]
            if s:
                return TclResult(value=s[0].upper() + s[1:].lower())
            return TclResult(value="")

        case "trim":
            if not rest:
                raise TclError('wrong # args: should be "string trim string ?chars?"')
            chars = rest[1] if len(rest) > 1 else None
            return TclResult(value=rest[0].strip(chars))

        case "trimleft":
            if not rest:
                raise TclError('wrong # args: should be "string trimleft string ?chars?"')
            chars = rest[1] if len(rest) > 1 else None
            return TclResult(value=rest[0].lstrip(chars))

        case "trimright":
            if not rest:
                raise TclError('wrong # args: should be "string trimright string ?chars?"')
            chars = rest[1] if len(rest) > 1 else None
            return TclResult(value=rest[0].rstrip(chars))

        case "is":
            if not rest:
                raise TclError(
                    'wrong # args: should be "string is class ?-strict? ?-failindex var? string"'
                )
            cls = rest[0]
            r = rest[1:]
            strict = False
            while r and r[0] in ("-strict", "-failindex"):
                if r[0] == "-strict":
                    strict = True
                    r = r[1:]
                elif r[0] == "-failindex" and len(r) > 1:
                    r = r[2:]  # skip -failindex varName
                else:
                    break
            if not r:
                raise TclError(
                    'wrong # args: should be "string is class ?-strict? ?-failindex var? string"'
                )
            s = r[0]
            checker = _IS_CHECKS.get(cls)
            if checker is None:
                raise TclError(f'bad class "{cls}": must be {", ".join(sorted(_IS_CHECKS.keys()))}')
            if not s and not strict:
                result = True
            else:
                result = checker(s)
            return TclResult(value="1" if result else "0")

        case "replace":
            if len(rest) < 3:
                raise TclError(
                    'wrong # args: should be "string replace string first last ?newString?"'
                )
            s = rest[0]
            first = _parse_index(rest[1], len(s))
            last = _parse_index(rest[2], len(s))
            new_str = rest[3] if len(rest) > 3 else ""
            if first > last or first >= len(s):
                return TclResult(value=s)
            first = max(0, first)
            last = min(len(s) - 1, last)
            return TclResult(value=s[:first] + new_str + s[last + 1 :])

        case "cat":
            return TclResult(value="".join(rest))

        case "wordend":
            if len(rest) != 2:
                raise TclError('wrong # args: should be "string wordend string charIndex"')
            s = rest[0]
            idx = _parse_index(rest[1], len(s))
            i = min(idx, len(s) - 1)
            while i < len(s) and (s[i].isalnum() or s[i] == "_"):
                i += 1
            return TclResult(value=str(i))

        case "wordstart":
            if len(rest) != 2:
                raise TclError('wrong # args: should be "string wordstart string charIndex"')
            s = rest[0]
            idx = _parse_index(rest[1], len(s))
            i = min(idx, len(s) - 1)
            while i > 0 and (s[i - 1].isalnum() or s[i - 1] == "_"):
                i -= 1
            return TclResult(value=str(i))

        case _:
            raise TclError(
                f'unknown or ambiguous subcommand "{sub}": must be '
                "cat, compare, equal, first, index, is, last, length, map, "
                "match, range, repeat, replace, reverse, tolower, totitle, "
                "toupper, trim, trimleft, trimright, wordend, or wordstart"
            )


def _string_map(s: str, pairs: list[tuple[str, str]], nocase: bool) -> str:
    """Apply string map replacement pairs."""
    result: list[str] = []
    i = 0
    while i < len(s):
        matched = False
        for key, value in pairs:
            if not key:
                continue
            if nocase:
                if s[i : i + len(key)].lower() == key.lower():
                    result.append(value)
                    i += len(key)
                    matched = True
                    break
            else:
                if s[i : i + len(key)] == key:
                    result.append(value)
                    i += len(key)
                    matched = True
                    break
        if not matched:
            result.append(s[i])
            i += 1
    return "".join(result)


def register() -> None:
    """Register string commands."""
    from core.commands.registry import REGISTRY

    REGISTRY.register_handler("string", _cmd_string)
