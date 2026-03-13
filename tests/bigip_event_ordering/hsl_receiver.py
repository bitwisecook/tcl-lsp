#!/usr/bin/env python3
"""UDP receiver for HSL::send output from BigIP event-ordering iRules.

Listens on a configurable port for ``EVTORD`` messages sent via
``HSL::send`` and writes them to stdout and optionally to a log file.

The BIG-IP HSL pool should be configured as a UDP pool pointing at
this receiver's IP and port.

Usage::

    # Listen on default port 5514
    python hsl_receiver.py

    # Custom port, write to file, exit after 30s idle
    python hsl_receiver.py --port 5514 --output events.log --timeout 30
"""

from __future__ import annotations

import argparse
import socket
import sys
import time


def receive_logs(
    *,
    bind: str = "0.0.0.0",
    port: int = 5514,
    output_file: str | None = None,
    timeout: int = 0,
) -> None:
    """Run the UDP log receiver loop.

    Args:
        bind: Address to bind to.
        port: UDP port to listen on.
        output_file: Optional path to append logs to.
        timeout: Exit after this many seconds of inactivity (0 = never).
    """
    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    sock.bind((bind, port))
    sock.settimeout(1.0)  # 1s poll for timeout logic

    outfile = open(output_file, "a") if output_file else None
    last_activity = time.monotonic()

    print(
        f"[hsl-receiver] listening on UDP {bind}:{port}",
        file=sys.stderr,
        flush=True,
    )
    try:
        while True:
            try:
                data, addr = sock.recvfrom(4096)
                line = data.decode("utf-8", errors="replace").strip()
                timestamp = time.strftime("%Y-%m-%dT%H:%M:%S")
                output = f"{timestamp} {addr[0]}:{addr[1]} {line}"
                print(output, flush=True)
                if outfile:
                    outfile.write(output + "\n")
                    outfile.flush()
                last_activity = time.monotonic()
            except socket.timeout:
                if timeout > 0:
                    idle = time.monotonic() - last_activity
                    if idle >= timeout:
                        print(
                            f"[hsl-receiver] idle timeout ({timeout}s)",
                            file=sys.stderr,
                            flush=True,
                        )
                        break
    except KeyboardInterrupt:
        print(
            "\n[hsl-receiver] shutting down",
            file=sys.stderr,
            flush=True,
        )
    finally:
        if outfile:
            outfile.close()
        sock.close()


def main() -> None:
    parser = argparse.ArgumentParser(
        description="HSL log receiver for BigIP event-ordering tests",
    )
    parser.add_argument(
        "--port", type=int, default=5514, help="UDP port to listen on (default: 5514)"
    )
    parser.add_argument("--bind", default="0.0.0.0", help="Address to bind to (default: 0.0.0.0)")
    parser.add_argument("--output", "-o", help="Append logs to this file")
    parser.add_argument(
        "--timeout", type=int, default=0, help="Exit after N seconds of inactivity (0=never)"
    )
    args = parser.parse_args()

    receive_logs(
        bind=args.bind,
        port=args.port,
        output_file=args.output,
        timeout=args.timeout,
    )


if __name__ == "__main__":
    main()
