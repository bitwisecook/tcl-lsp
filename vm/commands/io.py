"""I/O commands: puts, gets, open, close, read, flush, eof, fconfigure, chan."""

from __future__ import annotations

import sys
from typing import TYPE_CHECKING, TextIO, cast

from ..types import TclError, TclResult

if TYPE_CHECKING:
    from ..interp import TclInterp

# Standard channel mapping
_CHANNELS: dict[str, object] = {
    "stdout": sys.stdout,
    "stderr": sys.stderr,
    "stdin": sys.stdin,
}


def _cmd_puts(interp: TclInterp, args: list[str]) -> TclResult:
    """puts ?-nonewline? ?channelId? string"""
    nonewline = False
    channel = "stdout"

    remaining = list(args)
    if not remaining:
        raise TclError('wrong # args: should be "puts ?-nonewline? ?channelId? string"')

    if remaining[0] == "-nonewline":
        nonewline = True
        remaining = remaining[1:]

    if not remaining:
        raise TclError('wrong # args: should be "puts ?-nonewline? ?channelId? string"')

    if len(remaining) == 2:
        channel = remaining[0]
        text = remaining[1]
    elif len(remaining) == 1:
        text = remaining[0]
    else:
        raise TclError('wrong # args: should be "puts ?-nonewline? ?channelId? string"')

    ch = interp.channels.get(channel)
    if ch is None:
        raise TclError(f'can not find channel named "{channel}"')

    if nonewline:
        ch.write(text)
    else:
        ch.write(text + "\n")
    ch.flush()

    return TclResult()


def _cmd_gets(interp: TclInterp, args: list[str]) -> TclResult:
    """gets channelId ?varName?"""
    if not args:
        raise TclError('wrong # args: should be "gets channelId ?varName?"')
    channel = args[0]
    ch = interp.channels.get(channel)
    if ch is None:
        raise TclError(f'can not find channel named "{channel}"')

    line = ch.readline()
    if line.endswith("\n"):
        line = line[:-1]

    if len(args) >= 2:
        interp.current_frame.set_var(args[1], line)
        return TclResult(value=str(len(line)))
    return TclResult(value=line)


def _cmd_flush(interp: TclInterp, args: list[str]) -> TclResult:
    """flush channelId"""
    if not args:
        raise TclError('wrong # args: should be "flush channelId"')
    ch = interp.channels.get(args[0])
    if ch is None:
        raise TclError(f'can not find channel named "{args[0]}"')
    ch.flush()
    return TclResult()


def _cmd_eof(interp: TclInterp, args: list[str]) -> TclResult:
    """eof channelId"""
    if not args:
        raise TclError('wrong # args: should be "eof channelId"')
    # Simplified
    return TclResult(value="0")


def _cmd_close(interp: TclInterp, args: list[str]) -> TclResult:
    """close channelId"""
    if not args:
        raise TclError('wrong # args: should be "close channelId"')
    channel = args[0]
    if channel in ("stdout", "stderr", "stdin"):
        return TclResult()
    ch = interp.channels.get(channel)
    if ch is not None:
        ch.close()
        del interp.channels[channel]
    return TclResult()


def _cmd_open(interp: TclInterp, args: list[str]) -> TclResult:
    """open fileName ?access? ?permissions?"""
    if not args:
        raise TclError('wrong # args: should be "open fileName ?access? ?permissions?"')
    filename = args[0]
    access = args[1] if len(args) > 1 else "r"

    mode_map: dict[str, str] = {
        "r": "r",
        "r+": "r+",
        "w": "w",
        "w+": "w+",
        "a": "a",
        "a+": "a+",
        "RDONLY": "r",
        "WRONLY": "w",
        "RDWR": "r+",
    }
    py_mode = mode_map.get(access, access)

    try:
        f = cast("TextIO", open(filename, py_mode))  # noqa: SIM115
    except OSError as e:
        raise TclError(f'couldn\'t open "{filename}": {e}') from e

    # Generate a unique channel name
    channel_id = f"file{id(f)}"
    interp.channels[channel_id] = f
    return TclResult(value=channel_id)


def _cmd_read(interp: TclInterp, args: list[str]) -> TclResult:
    """read ?-nonewline? channelId ?numChars?"""
    nonewline = False
    remaining = list(args)
    if remaining and remaining[0] == "-nonewline":
        nonewline = True
        remaining = remaining[1:]
    if not remaining:
        raise TclError('wrong # args: should be "read channelId ?numChars?"')
    channel = remaining[0]
    ch = interp.channels.get(channel)
    if ch is None:
        raise TclError(f'can not find channel named "{channel}"')
    if len(remaining) > 1:
        num_chars = int(remaining[1])
        data = ch.read(num_chars)
    else:
        data = ch.read()
    if nonewline and data.endswith("\n"):
        data = data[:-1]
    return TclResult(value=data)


def _cmd_seek(interp: TclInterp, args: list[str]) -> TclResult:
    """seek channelId offset ?origin?"""
    if len(args) < 2:
        raise TclError('wrong # args: should be "seek channelId offset ?origin?"')
    ch = interp.channels.get(args[0])
    if ch is None:
        raise TclError(f'can not find channel named "{args[0]}"')
    offset = int(args[1])
    origin = args[2] if len(args) > 2 else "start"
    whence = {"start": 0, "current": 1, "end": 2}.get(origin, 0)
    ch.seek(offset, whence)
    return TclResult()


def _cmd_tell(interp: TclInterp, args: list[str]) -> TclResult:
    """tell channelId"""
    if not args:
        raise TclError('wrong # args: should be "tell channelId"')
    ch = interp.channels.get(args[0])
    if ch is None:
        raise TclError(f'can not find channel named "{args[0]}"')
    return TclResult(value=str(ch.tell()))


def _cmd_fconfigure(interp: TclInterp, args: list[str]) -> TclResult:
    """fconfigure channelId ?name? ?value? ?name value ...?"""
    if not args:
        raise TclError('wrong # args: should be "fconfigure channelId ?name value ...?"')
    # Stub: ignore configuration for now
    return TclResult()


def register() -> None:
    """Register I/O commands."""
    from core.commands.registry import REGISTRY

    REGISTRY.register_handler("puts", _cmd_puts)
    REGISTRY.register_handler("gets", _cmd_gets)
    REGISTRY.register_handler("flush", _cmd_flush)
    REGISTRY.register_handler("eof", _cmd_eof)
    REGISTRY.register_handler("close", _cmd_close)
    REGISTRY.register_handler("open", _cmd_open)
    REGISTRY.register_handler("read", _cmd_read)
    REGISTRY.register_handler("seek", _cmd_seek)
    REGISTRY.register_handler("tell", _cmd_tell)
    REGISTRY.register_handler("fconfigure", _cmd_fconfigure)
