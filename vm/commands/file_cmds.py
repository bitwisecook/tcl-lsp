"""The ``file`` command and its subcommands, plus ``glob``."""

from __future__ import annotations

import glob as pyglob
import os
import shutil
import stat
import tempfile
from pathlib import PurePosixPath
from typing import TYPE_CHECKING

from ..machine import _list_escape, _split_list
from ..types import TclError, TclResult

if TYPE_CHECKING:
    from ..interp import TclInterp


# file subcommands


def _file_join(args: list[str]) -> str:
    if not args:
        raise TclError('wrong # args: should be "file join name ?name ...?"')
    # Tcl file join: if any component is absolute, it replaces the path so far
    result = args[0]
    for part in args[1:]:
        if os.path.isabs(part):
            result = part
        elif result == "" or result.endswith("/"):
            result = result + part
        else:
            result = result + "/" + part
    return result


def _file_dirname(args: list[str]) -> str:
    if len(args) != 1:
        raise TclError('wrong # args: should be "file dirname name"')
    d = os.path.dirname(args[0])
    return d if d else "."


def _file_tail(args: list[str]) -> str:
    if len(args) != 1:
        raise TclError('wrong # args: should be "file tail name"')
    return os.path.basename(args[0])


def _file_exists(args: list[str]) -> str:
    if len(args) != 1:
        raise TclError('wrong # args: should be "file exists name"')
    return "1" if os.path.exists(args[0]) else "0"


def _file_normalize(args: list[str]) -> str:
    if len(args) != 1:
        raise TclError('wrong # args: should be "file normalize name"')
    return os.path.normpath(os.path.abspath(args[0]))


def _file_split(args: list[str]) -> str:
    if len(args) != 1:
        raise TclError('wrong # args: should be "file split name"')
    path = args[0]
    if not path:
        return ""
    parts = PurePosixPath(path).parts
    return " ".join(_list_escape(p) for p in parts)


def _file_extension(args: list[str]) -> str:
    if len(args) != 1:
        raise TclError('wrong # args: should be "file extension name"')
    _, ext = os.path.splitext(args[0])
    return ext


def _file_rootname(args: list[str]) -> str:
    if len(args) != 1:
        raise TclError('wrong # args: should be "file rootname name"')
    root, _ = os.path.splitext(args[0])
    return root


def _file_isdirectory(args: list[str]) -> str:
    if len(args) != 1:
        raise TclError('wrong # args: should be "file isdirectory name"')
    return "1" if os.path.isdir(args[0]) else "0"


def _file_isfile(args: list[str]) -> str:
    if len(args) != 1:
        raise TclError('wrong # args: should be "file isfile name"')
    return "1" if os.path.isfile(args[0]) else "0"


def _file_readable(args: list[str]) -> str:
    if len(args) != 1:
        raise TclError('wrong # args: should be "file readable name"')
    return "1" if os.access(args[0], os.R_OK) else "0"


def _file_writable(args: list[str]) -> str:
    if len(args) != 1:
        raise TclError('wrong # args: should be "file writable name"')
    return "1" if os.access(args[0], os.W_OK) else "0"


def _file_executable(args: list[str]) -> str:
    if len(args) != 1:
        raise TclError('wrong # args: should be "file executable name"')
    return "1" if os.access(args[0], os.X_OK) else "0"


def _file_mkdir(args: list[str]) -> None:
    if not args:
        raise TclError('wrong # args: should be "file mkdir ?dir ...?"')
    for d in args:
        try:
            os.makedirs(d, exist_ok=True)
        except OSError as e:
            raise TclError(f'can\'t create directory "{d}": {e}') from e


def _file_delete(args: list[str]) -> None:
    force = False
    paths = list(args)
    if paths and paths[0] == "-force":
        force = True
        paths = paths[1:]
    if paths and paths[0] == "--":
        paths = paths[1:]
    for p in paths:
        try:
            if os.path.isdir(p):
                if force:
                    shutil.rmtree(p)
                else:
                    os.rmdir(p)
            elif os.path.exists(p) or os.path.islink(p):
                os.remove(p)
            elif not force:
                raise TclError(f'error deleting "{p}": no such file or directory')
        except OSError as e:
            if not force:
                raise TclError(f'error deleting "{p}": {e}') from e


def _file_copy(args: list[str]) -> None:
    force = False
    paths = list(args)
    if paths and paths[0] == "-force":
        force = True
        paths = paths[1:]
    if paths and paths[0] == "--":
        paths = paths[1:]
    if len(paths) < 2:
        raise TclError('wrong # args: should be "file copy ?-force? source target"')
    src, dst = paths[0], paths[-1]
    try:
        if os.path.isdir(src):
            if force and os.path.exists(dst):
                shutil.rmtree(dst)
            shutil.copytree(src, dst)
        else:
            if force or not os.path.exists(dst):
                shutil.copy2(src, dst)
            else:
                raise TclError(f'error copying "{src}" to "{dst}": file already exists')
    except OSError as e:
        raise TclError(f'error copying "{src}" to "{dst}": {e}') from e


def _file_rename(args: list[str]) -> None:
    force = False
    paths = list(args)
    if paths and paths[0] == "-force":
        force = True
        paths = paths[1:]
    if paths and paths[0] == "--":
        paths = paths[1:]
    if len(paths) < 2:
        raise TclError('wrong # args: should be "file rename ?-force? source target"')
    src, dst = paths[0], paths[-1]
    try:
        if not force and os.path.exists(dst):
            raise TclError(f'error renaming "{src}" to "{dst}": file already exists')
        os.rename(src, dst)
    except OSError as e:
        raise TclError(f'error renaming "{src}" to "{dst}": {e}') from e


def _file_size(args: list[str]) -> str:
    if len(args) != 1:
        raise TclError('wrong # args: should be "file size name"')
    try:
        return str(os.path.getsize(args[0]))
    except OSError as e:
        raise TclError(f'could not read "{args[0]}": {e}') from e


def _file_mtime(args: list[str]) -> str:
    if not args or len(args) > 2:
        raise TclError('wrong # args: should be "file mtime name ?time?"')
    if len(args) == 2:
        try:
            os.utime(args[0], (int(args[1]), int(args[1])))
            return ""
        except OSError as e:
            raise TclError(f'could not set mtime for "{args[0]}": {e}') from e
    try:
        return str(int(os.path.getmtime(args[0])))
    except OSError as e:
        raise TclError(f'could not read "{args[0]}": {e}') from e


def _file_type(args: list[str]) -> str:
    if len(args) != 1:
        raise TclError('wrong # args: should be "file type name"')
    try:
        st = os.lstat(args[0])
    except OSError as e:
        raise TclError(f'could not read "{args[0]}": {e}') from e
    mode = st.st_mode
    if stat.S_ISREG(mode):
        return "file"
    if stat.S_ISDIR(mode):
        return "directory"
    if stat.S_ISLNK(mode):
        return "link"
    if stat.S_ISFIFO(mode):
        return "fifo"
    if stat.S_ISSOCK(mode):
        return "socket"
    if stat.S_ISBLK(mode):
        return "blockSpecial"
    if stat.S_ISCHR(mode):
        return "characterSpecial"
    return "file"


def _file_pathtype(args: list[str]) -> str:
    if len(args) != 1:
        raise TclError('wrong # args: should be "file pathtype name"')
    path = args[0]
    if os.path.isabs(path):
        return "absolute"
    if path.startswith("~"):
        return "volumerelative"
    return "relative"


def _file_nativename(args: list[str]) -> str:
    if len(args) != 1:
        raise TclError('wrong # args: should be "file nativename name"')
    return args[0]


def _file_channels(interp: TclInterp, args: list[str]) -> str:
    import fnmatch

    pattern = args[0] if args else "*"
    names = sorted(interp.channels.keys())
    matched = [n for n in names if fnmatch.fnmatch(n, pattern)]
    return " ".join(matched)


def _file_tempdir(args: list[str]) -> str:
    if args:
        raise TclError('wrong # args: should be "file tempdir"')
    return tempfile.gettempdir()


def _file_tildeexpand(args: list[str]) -> str:
    if len(args) != 1:
        raise TclError('wrong # args: should be "file tildeexpand name"')
    return os.path.expanduser(args[0])


def _file_separator(args: list[str]) -> str:
    return "/"


# main dispatcher


_FILE_SUBCMDS = (
    "channels",
    "copy",
    "delete",
    "dirname",
    "executable",
    "exists",
    "extension",
    "isdirectory",
    "isfile",
    "join",
    "mkdir",
    "mtime",
    "nativename",
    "normalize",
    "pathtype",
    "readable",
    "rename",
    "rootname",
    "separator",
    "size",
    "split",
    "tail",
    "tempdir",
    "tildeexpand",
    "type",
    "writable",
)


def _resolve_file_subcmd(sub: str) -> str:
    """Resolve an abbreviated ``file`` subcommand to its full name."""
    matches = [s for s in _FILE_SUBCMDS if s.startswith(sub)]
    if len(matches) == 1:
        return matches[0]
    return sub  # exact or ambiguous — let match/case handle it


def _cmd_file(interp: TclInterp, args: list[str]) -> TclResult:
    """file option ?arg ...?"""
    if not args:
        raise TclError('wrong # args: should be "file option ?arg ...?"')
    sub = _resolve_file_subcmd(args[0])
    rest = args[1:]

    match sub:
        case "join":
            return TclResult(value=_file_join(rest))
        case "dirname":
            return TclResult(value=_file_dirname(rest))
        case "tail":
            return TclResult(value=_file_tail(rest))
        case "exists":
            return TclResult(value=_file_exists(rest))
        case "normalize":
            return TclResult(value=_file_normalize(rest))
        case "split":
            return TclResult(value=_file_split(rest))
        case "extension":
            return TclResult(value=_file_extension(rest))
        case "rootname":
            return TclResult(value=_file_rootname(rest))
        case "isdirectory":
            return TclResult(value=_file_isdirectory(rest))
        case "isfile":
            return TclResult(value=_file_isfile(rest))
        case "readable":
            return TclResult(value=_file_readable(rest))
        case "writable":
            return TclResult(value=_file_writable(rest))
        case "executable":
            return TclResult(value=_file_executable(rest))
        case "mkdir":
            _file_mkdir(rest)
            return TclResult()
        case "delete":
            _file_delete(rest)
            return TclResult()
        case "copy":
            _file_copy(rest)
            return TclResult()
        case "rename":
            _file_rename(rest)
            return TclResult()
        case "size":
            return TclResult(value=_file_size(rest))
        case "mtime":
            return TclResult(value=_file_mtime(rest))
        case "type":
            return TclResult(value=_file_type(rest))
        case "pathtype":
            return TclResult(value=_file_pathtype(rest))
        case "nativename":
            return TclResult(value=_file_nativename(rest))
        case "channels":
            return TclResult(value=_file_channels(interp, rest))
        case "tempdir":
            return TclResult(value=_file_tempdir(rest))
        case "tildeexpand":
            return TclResult(value=_file_tildeexpand(rest))
        case "separator":
            return TclResult(value=_file_separator(rest))
        case _:
            raise TclError(
                f'unknown or ambiguous subcommand "{sub}": must be '
                "channels, copy, delete, dirname, executable, exists, "
                "extension, isdirectory, isfile, join, mkdir, mtime, "
                "nativename, normalize, pathtype, readable, rename, "
                "rootname, separator, size, split, tail, tempdir, "
                "tildeexpand, type, or writable"
            )


# glob command


def _tcl_glob_to_python(pattern: str) -> str:
    """Translate Tcl glob pattern to Python glob pattern.

    Tcl and Python glob are mostly compatible.  The main difference is
    Tcl brace expansion ``{a,b}`` which Python's glob module does not
    support directly — we handle this by expanding braces ourselves.
    """
    return pattern


def _expand_braces(pattern: str) -> list[str]:
    """Expand top-level Tcl brace alternatives ``{a,b,c}`` into a list of patterns."""
    start = pattern.find("{")
    if start < 0:
        return [pattern]
    end = pattern.find("}", start)
    if end < 0:
        return [pattern]
    prefix = pattern[:start]
    suffix = pattern[end + 1 :]
    alternatives = pattern[start + 1 : end].split(",")
    results: list[str] = []
    for alt in alternatives:
        for expanded in _expand_braces(prefix + alt + suffix):
            results.append(expanded)
    return results


def _cmd_glob(interp: TclInterp, args: list[str]) -> TclResult:
    """glob ?options? pattern ?pattern ...?"""
    directory: str | None = None
    nocomplain = False
    join_patterns = False
    tails = False
    type_filters: list[str] = []
    remaining = list(args)

    # Parse options
    while remaining and remaining[0].startswith("-"):
        opt = remaining.pop(0)
        if opt == "--":
            break
        elif opt == "-directory":
            if not remaining:
                raise TclError('missing argument to "-directory"')
            directory = remaining.pop(0)
        elif opt == "-nocomplain":
            nocomplain = True
        elif opt == "-join":
            join_patterns = True
        elif opt == "-tails":
            tails = True
        elif opt == "-types":
            if not remaining:
                raise TclError('missing argument to "-types"')
            type_filters = _split_list(remaining.pop(0))
        elif opt == "-path":
            if not remaining:
                raise TclError('missing argument to "-path"')
            # -path prefix: use as directory + pattern prefix
            directory = os.path.dirname(remaining.pop(0))
        else:
            raise TclError(
                f'bad option "{opt}": must be -directory, -join, -nocomplain, -path, -tails, or -types'
            )

    if not remaining:
        raise TclError('wrong # args: should be "glob ?-option value ...? pattern ?pattern ...?"')

    # Build the patterns
    if join_patterns:
        raw_patterns = [os.path.join(*remaining)]
    else:
        raw_patterns = remaining

    # Expand brace alternatives
    expanded_patterns: list[str] = []
    for pat in raw_patterns:
        expanded_patterns.extend(_expand_braces(pat))

    # Run glob for each pattern
    matches: list[str] = []
    for pat in expanded_patterns:
        if directory is not None:
            full_pattern = os.path.join(directory, pat)
        else:
            full_pattern = pat

        results = pyglob.glob(full_pattern)
        for r in results:
            # Apply type filters
            if type_filters:
                ok = True
                for tf in type_filters:
                    match tf:
                        case "d":
                            if not os.path.isdir(r):
                                ok = False
                        case "f":
                            if not os.path.isfile(r):
                                ok = False
                        case "r":
                            if not os.access(r, os.R_OK):
                                ok = False
                        case "w":
                            if not os.access(r, os.W_OK):
                                ok = False
                        case "x":
                            if not os.access(r, os.X_OK):
                                ok = False
                if not ok:
                    continue

            if tails:
                if directory is not None:
                    r = os.path.relpath(r, directory)
                else:
                    r = os.path.basename(r)

            if r not in matches:
                matches.append(r)

    if not matches and not nocomplain:
        pat_str = " ".join(f'"{p}"' for p in raw_patterns)
        raise TclError(f"no files matched glob patterns {pat_str}")

    return TclResult(value=" ".join(_list_escape(m) for m in sorted(matches)))


# registration


def register() -> None:
    """Register file and glob commands."""
    from core.commands.registry import REGISTRY

    REGISTRY.register_handler("file", _cmd_file)
    REGISTRY.register_handler("glob", _cmd_glob)
