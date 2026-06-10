"""``f5 completion`` — emit shell completion bootstrap (argcomplete)."""

from __future__ import annotations

import argparse
import sys

from ._registry import verb

_SHELLS: tuple[str, ...] = ("bash", "fish", "zsh")

_INSTALL_HINTS: dict[str, str] = {
    "bash": (
        "Install (eager, per-shell):\n"
        "  echo 'source <(f5 completion bash)' >> ~/.bashrc\n"
        "Or write to a system completion dir (loaded lazily by bash-completion):\n"
        "  f5 completion bash | sudo tee /etc/bash_completion.d/f5 >/dev/null\n"
    ),
    "fish": (
        "Install:\n"
        "  mkdir -p ~/.config/fish/completions\n"
        "  f5 completion fish > ~/.config/fish/completions/f5.fish\n"
        "Then start a new fish session."
    ),
    "zsh": (
        "Install (eager, per-shell):\n"
        "  echo 'source <(f5 completion zsh)' >> ~/.zshrc\n"
        "Or write to a directory on $fpath (loaded lazily by compinit):\n"
        '  mkdir -p "${ZDOTDIR:-$HOME}/.zsh/completions"\n'
        '  f5 completion zsh > "${ZDOTDIR:-$HOME}/.zsh/completions/_f5"\n'
        "Then ensure ~/.zshrc has, before `compinit`:\n"
        '  fpath=("${ZDOTDIR:-$HOME}/.zsh/completions" $fpath)\n'
    ),
}


@verb(
    "completion",
    aliases=(),
    help="Print a bash / fish / zsh completion script for the f5 CLI.",
    formatter_class=argparse.RawDescriptionHelpFormatter,
)
def _configure(p: argparse.ArgumentParser, *, prog_name: str, default_dialect: str) -> None:  # noqa: ARG001
    p.description = (
        "Print a shell completion bootstrap for the f5 CLI.  The output is\n"
        "produced by argcomplete from the live argparse definition, so verb\n"
        "names, flags, and choice lists stay in sync with the CLI without\n"
        "any per-shell template to maintain.  Pipe the output into your\n"
        "shell startup (or into the shell's completion directory).\n"
    )
    p.epilog = (
        "Examples:\n"
        "  source <(f5 completion bash)             # eager (~/.bashrc)\n"
        "  f5 completion bash | sudo tee /etc/bash_completion.d/f5\n"
        "  f5 completion fish > ~/.config/fish/completions/f5.fish\n"
        "  source <(f5 completion zsh)              # eager (~/.zshrc)\n"
        "  f5 completion zsh --hint                 # print install help to stderr\n"
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

    sys.stdout.write(shellcode(["f5", "f5.pyz"], shell=args.shell))
    if args.hint:
        print(_INSTALL_HINTS[args.shell], file=sys.stderr)
    return 0
