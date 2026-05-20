"""``tcl completion`` — emit shell completion bootstrap (argcomplete)."""

from __future__ import annotations

import argparse
import sys

from ._registry import verb

_SHELLS: tuple[str, ...] = ("bash", "fish", "zsh")

_INSTALL_HINTS: dict[str, str] = {
    "bash": (
        "Install (eager, per-shell):\n"
        "  echo 'source <(tcl completion bash)' >> ~/.bashrc\n"
        "Or write to a system completion dir (loaded lazily by bash-completion):\n"
        "  tcl completion bash | sudo tee /etc/bash_completion.d/tcl >/dev/null\n"
    ),
    "fish": (
        "Install:\n"
        "  mkdir -p ~/.config/fish/completions\n"
        "  tcl completion fish > ~/.config/fish/completions/tcl.fish\n"
        "Then start a new fish session."
    ),
    "zsh": (
        "Install (eager, per-shell):\n"
        "  echo 'source <(tcl completion zsh)' >> ~/.zshrc\n"
        "Or write to a directory on $fpath (loaded lazily by compinit):\n"
        '  mkdir -p "${ZDOTDIR:-$HOME}/.zsh/completions"\n'
        '  tcl completion zsh > "${ZDOTDIR:-$HOME}/.zsh/completions/_tcl"\n'
        "Then ensure ~/.zshrc has, before `compinit`:\n"
        '  fpath=("${ZDOTDIR:-$HOME}/.zsh/completions" $fpath)\n'
    ),
}


@verb(
    "completion",
    aliases=(),
    help="Print a bash / fish / zsh completion script for the tcl CLI.",
    formatter_class=argparse.RawDescriptionHelpFormatter,
)
def _configure(p: argparse.ArgumentParser, *, prog_name: str, default_dialect: str) -> None:  # noqa: ARG001
    p.description = (
        "Print a shell completion bootstrap for the tcl CLI.  The output is\n"
        "produced by argcomplete from the live argparse definition, so verb\n"
        "names, flags, and choice lists stay in sync with the CLI without\n"
        "any per-shell template to maintain.  Pipe the output into your\n"
        "shell startup (or into the shell's completion directory).\n"
    )
    p.epilog = (
        "Examples:\n"
        f"  source <({prog_name} completion bash)             # eager (~/.bashrc)\n"
        f"  {prog_name} completion bash | sudo tee /etc/bash_completion.d/tcl\n"
        f"  {prog_name} completion fish > ~/.config/fish/completions/tcl.fish\n"
        f"  source <({prog_name} completion zsh)              # eager (~/.zshrc)\n"
        f"  {prog_name} completion zsh --hint                 # print install help to stderr\n"
    )
    p.add_argument(
        "shell",
        choices=_SHELLS,
        help="Shell to print a completion script for.",
    )
    p.add_argument(
        "--hint",
        action="store_true",
        help="Print install instructions for the chosen shell to stderr.",
    )
    p.set_defaults(handler=_run_completion)


def _run_completion(args: argparse.Namespace) -> int:
    try:
        from argcomplete import shellcode
    except ImportError:
        print(
            "error: argcomplete is not installed — install it with `pip install argcomplete`",
            file=sys.stderr,
        )
        return 2

    # Cover both the ``tcl`` console-script entry and the zipapp shipped
    # under ``tcl.pyz`` so users get completion regardless of how they
    # invoke the CLI.
    sys.stdout.write(shellcode(["tcl", "tcl.pyz"], shell=args.shell))
    if args.hint:
        print(_INSTALL_HINTS[args.shell], file=sys.stderr)
    return 0
