// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The CLI's standard output and error streams.
//!
//! A verb writes through [`outln!`](crate::outln) / [`errln!`](crate::errln),
//! [`stdout`] and [`stderr`] rather than `println!` or `std::io::stdout()`.
//! Normally they are the process streams. Inside [`capture`] they are the
//! calling thread's buffers instead, which is how one long-lived `tcl` process
//! answers many test invocations ([`crate::worker`]) with the bytes a fresh
//! process would have printed to a pipe.

use std::cell::RefCell;
use std::fmt;
use std::io::{self, IsTerminal, Write};

/// Bytes a captured invocation wrote.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Captured {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

thread_local! {
    static CAPTURE: RefCell<Option<Captured>> = const { RefCell::new(None) };
}

/// Run `f` with this thread's streams captured, returning what it wrote.
///
/// Captured streams behave as a pipe does: not a terminal, so colour is off
/// unless forced.
pub fn capture<R>(f: impl FnOnce() -> R) -> (R, Captured) {
    let previous = CAPTURE.with(|c| c.replace(Some(Captured::default())));
    let result = f();
    let captured = CAPTURE.with(|c| c.replace(previous)).unwrap_or_default();
    (result, captured)
}

fn is_capturing() -> bool {
    CAPTURE.with(|c| c.borrow().is_some())
}

#[derive(Clone, Copy)]
enum Which {
    Out,
    Err,
}

fn write_captured(which: Which, bytes: &[u8]) -> bool {
    CAPTURE.with(|c| match c.borrow_mut().as_mut() {
        Some(captured) => {
            match which {
                Which::Out => captured.stdout.extend_from_slice(bytes),
                Which::Err => captured.stderr.extend_from_slice(bytes),
            }
            true
        }
        None => false,
    })
}

/// A handle on the CLI's stdout or stderr.
pub struct Stream(Which);

/// The CLI's standard output.
#[must_use]
pub fn stdout() -> Stream {
    Stream(Which::Out)
}

/// The CLI's standard error.
#[must_use]
pub fn stderr() -> Stream {
    Stream(Which::Err)
}

impl Write for Stream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if write_captured(self.0, buf) {
            return Ok(buf.len());
        }
        match self.0 {
            Which::Out => io::stdout().write(buf),
            Which::Err => io::stderr().write(buf),
        }
    }

    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        if write_captured(self.0, buf) {
            return Ok(());
        }
        match self.0 {
            Which::Out => io::stdout().lock().write_all(buf),
            Which::Err => io::stderr().lock().write_all(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        if is_capturing() {
            return Ok(());
        }
        match self.0 {
            Which::Out => io::stdout().flush(),
            Which::Err => io::stderr().flush(),
        }
    }
}

/// Whether stdout is an interactive terminal; never while captured.
#[must_use]
pub fn stdout_is_terminal() -> bool {
    !is_capturing() && io::stdout().is_terminal()
}

/// Whether stderr is an interactive terminal; never while captured.
#[must_use]
pub fn stderr_is_terminal() -> bool {
    !is_capturing() && io::stderr().is_terminal()
}

/// Styled stderr: ANSI is kept only where anstream would keep it for the real
/// stream, so a captured run strips it exactly as a piped process does.
#[must_use]
pub fn styled_stderr() -> Box<dyn Write> {
    if is_capturing() {
        let choice = anstream::AutoStream::choice(&Vec::<u8>::new());
        Box::new(CapturedStyled(Some(anstream::AutoStream::new(
            Vec::new(),
            choice,
        ))))
    } else {
        Box::new(anstream::stderr())
    }
}

/// A styled writer whose bytes join the captured stderr when it is dropped.
struct CapturedStyled(Option<anstream::AutoStream<Vec<u8>>>);

impl Write for CapturedStyled {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.as_mut().map_or(Ok(0), |s| s.write(buf))
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Drop for CapturedStyled {
    fn drop(&mut self) {
        if let Some(stream) = self.0.take() {
            write_captured(Which::Err, &stream.into_inner());
        }
    }
}

#[doc(hidden)]
pub fn print_out(args: fmt::Arguments<'_>) {
    if CAPTURE.with(|c| c.borrow().is_some()) {
        write_captured(Which::Out, fmt::format(args).as_bytes());
    } else {
        std::print!("{args}");
    }
}

#[doc(hidden)]
pub fn print_err(args: fmt::Arguments<'_>) {
    if CAPTURE.with(|c| c.borrow().is_some()) {
        write_captured(Which::Err, fmt::format(args).as_bytes());
    } else {
        std::eprint!("{args}");
    }
}

/// `println!` to the CLI's stdout.
#[macro_export]
macro_rules! outln {
    () => {
        $crate::sink::print_out(::std::format_args!("\n"))
    };
    ($($arg:tt)*) => {
        $crate::sink::print_out(::std::format_args!("{}\n", ::std::format_args!($($arg)*)))
    };
}

/// `eprintln!` to the CLI's stderr.
#[macro_export]
macro_rules! errln {
    () => {
        $crate::sink::print_err(::std::format_args!("\n"))
    };
    ($($arg:tt)*) => {
        $crate::sink::print_err(::std::format_args!("{}\n", ::std::format_args!($($arg)*)))
    };
}

#[cfg(test)]
mod tests {
    use super::{capture, stderr, stdout, stdout_is_terminal, styled_stderr};
    use std::io::Write as _;

    #[test]
    fn capture_collects_both_streams_and_restores() {
        let ((), got) = capture(|| {
            crate::outln!("one {}", 1);
            crate::errln!("two");
            stdout().write_all(b"raw").unwrap();
            stderr().write_all(b"!").unwrap();
            assert!(!stdout_is_terminal());
        });
        assert_eq!(got.stdout, b"one 1\nraw");
        assert_eq!(got.stderr, b"two\n!");
        let ((), empty) = capture(|| ());
        assert!(empty.stdout.is_empty() && empty.stderr.is_empty());
    }

    #[test]
    fn styled_stderr_strips_ansi_when_captured() {
        let ((), got) = capture(|| {
            let mut err = styled_stderr();
            write!(err, "\x1b[1;31merror:\x1b[0m boom").unwrap();
        });
        assert_eq!(String::from_utf8(got.stderr).unwrap(), "error: boom");
    }

    #[test]
    fn nested_capture_is_isolated() {
        let ((), outer) = capture(|| {
            crate::outln!("outer");
            let ((), inner) = capture(|| crate::outln!("inner"));
            assert_eq!(inner.stdout, b"inner\n");
        });
        assert_eq!(outer.stdout, b"outer\n");
    }
}
