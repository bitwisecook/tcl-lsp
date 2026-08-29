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

//! `Content-Length` framing — the LSP base protocol on a byte stream.
//!
//! The browser transport needs none of this: `postMessage` already delimits
//! messages. A byte stream does not, so this is the half of the WASI transport
//! that is genuinely new rather than a port. It is deliberately *incremental*:
//! [`Decoder::push`] takes whatever one `read` returned, however little that
//! was, and [`Decoder::next_message`] yields only complete messages. A driver
//! that has to hand the thread back after every read cannot afford a decoder
//! that blocks until a frame is whole.

/// An incremental `Content-Length` frame reader.
#[derive(Debug, Default)]
pub struct Decoder {
    /// Bytes read but not yet consumed by a complete message.
    buffer: Vec<u8>,
}

impl Decoder {
    /// An empty decoder.
    pub const fn new() -> Self {
        Self { buffer: Vec::new() }
    }

    /// Add bytes from one read.
    pub fn push(&mut self, bytes: &[u8]) {
        self.buffer.extend_from_slice(bytes);
    }

    /// How many bytes are buffered but not yet part of a complete message.
    #[cfg(test)]
    pub fn buffered(&self) -> usize {
        self.buffer.len()
    }

    /// Take the next complete message, if the buffer holds one.
    ///
    /// Returns `None` when the buffer holds only part of a frame — the caller
    /// reads more and asks again. A frame whose headers are malformed (no
    /// `Content-Length`, an unparseable one, or one naming a body that no
    /// address space could hold) is dropped along with its header block rather
    /// than wedging the stream: the next `\r\n\r\n` resynchronises.
    ///
    /// # What that recovery can and cannot do
    ///
    /// The *header block* is dropped, but its body cannot be: a body's length
    /// is exactly the thing a missing `Content-Length` failed to tell us, so
    /// there is nothing to skip *by*. Dropping "the body too" would mean
    /// guessing at a number the stream never carried.
    ///
    /// Leaving the orphaned body to be rescanned as headers is not good enough
    /// either, and measurably so. A JSON body carries no trailing newline, so
    /// it glues itself to the *next* frame's `Content-Length` line; the first
    /// colon on that combined line then belongs to the body's own JSON, and the
    /// real header is never seen. One malformed block would swallow the good
    /// frame after it, and the cascade could repeat.
    ///
    /// So recovery instead scans forward to the next `Content-Length:` in the
    /// buffer and resumes there — see [`Decoder::resynchronise`]. A JSON-RPC
    /// body cannot contain a raw newline (the grammar escapes them), so that
    /// text is overwhelmingly the next real header rather than something inside
    /// a body. The residue is a genuine heuristic: a body containing the
    /// literal text `Content-Length:` can still misdirect the resync once. It
    /// cannot wedge, because every pass drops at least the header block, so the
    /// decoder always makes progress.
    ///
    /// That residue is acceptable because the condition is a protocol violation
    /// by the client, not a state a conforming one reaches: every message the
    /// LSP specification defines carries a `Content-Length`. The contract here
    /// is "stay alive and keep making progress", not "reconstruct what a broken
    /// client meant".
    pub fn next_message(&mut self) -> Option<Result<String, FrameError>> {
        let (header_end, header_len) = find_header_end(&self.buffer)?;
        let Some(length) = content_length(&self.buffer[..header_end]) else {
            // Drop the header block, then skip whatever body it introduced.
            self.buffer.drain(..header_end + header_len);
            self.resynchronise();
            return Some(Err(FrameError::MissingContentLength));
        };
        let body_start = header_end + header_len;
        // Checked, because `length` is whatever the client wrote and the
        // release profile leaves overflow checks off: a wrapped end index
        // would pass the completeness test below and then slice backwards,
        // and a panic in this module is a process abort (`panic = "abort"`
        // on wasip1), so one hostile header would end the session.
        let Some(body_end) = body_start.checked_add(length) else {
            self.buffer.drain(..body_start);
            self.resynchronise();
            return Some(Err(FrameError::ContentLengthOutOfRange));
        };
        if self.buffer.len() < body_end {
            return None;
        }
        let body: Vec<u8> = self.buffer[body_start..body_end].to_vec();
        self.buffer.drain(..body_end);
        Some(String::from_utf8(body).map_err(|_| FrameError::NotUtf8))
    }

    /// Drop the orphaned body left by a header block that named no usable
    /// length.
    ///
    /// Advances the buffer to the next `Content-Length:`, which is where the
    /// next well-formed frame begins. If there is no such text yet the buffer
    /// is left alone: more bytes may still be coming, and the caller has
    /// already made progress by dropping the bad header block.
    fn resynchronise(&mut self) {
        if let Some(at) = find_ignore_ascii_case(&self.buffer, b"content-length:") {
            self.buffer.drain(..at);
        }
    }
}

/// Why a frame could not be handed on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameError {
    /// The header block carried no usable `Content-Length`.
    MissingContentLength,
    /// The `Content-Length` named a body reaching past the end of the address
    /// space, so no read could ever complete the frame.
    ContentLengthOutOfRange,
    /// The body was not valid UTF-8. JSON-RPC over LSP is always UTF-8.
    NotUtf8,
}

impl std::fmt::Display for FrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingContentLength => f.write_str("a header block with no Content-Length"),
            Self::ContentLengthOutOfRange => {
                f.write_str("a Content-Length larger than the address space can hold")
            }
            Self::NotUtf8 => f.write_str("a message body that is not valid UTF-8"),
        }
    }
}

/// Where the header block ends, and how many bytes the terminator took.
///
/// `\r\n\r\n` is what the base protocol specifies and what every client sends;
/// `\n\n` is accepted too so a hand-driven session (a shell pipeline, a test
/// fixture typed by hand) is not silently ignored.
fn find_header_end(buffer: &[u8]) -> Option<(usize, usize)> {
    let crlf = find(buffer, b"\r\n\r\n").map(|at| (at, 4));
    let lf = find(buffer, b"\n\n").map(|at| (at, 2));
    match (crlf, lf) {
        (Some(a), Some(b)) => Some(if a.0 <= b.0 { a } else { b }),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    }
}

/// The `Content-Length` value in a header block, if it names one.
fn content_length(headers: &[u8]) -> Option<usize> {
    let text = std::str::from_utf8(headers).ok()?;
    for line in text.split('\n') {
        let line = line.trim_end_matches('\r');
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        if name.trim().eq_ignore_ascii_case("content-length") {
            return value.trim().parse().ok();
        }
    }
    None
}

/// The first offset at which `needle` occurs in `haystack`, ignoring ASCII case.
///
/// Header names are case-insensitive in the base protocol, so the resync point
/// has to be found the same way [`content_length`] matches the name.
fn find_ignore_ascii_case(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    haystack
        .windows(needle.len())
        .position(|window| window.eq_ignore_ascii_case(needle))
}

/// The first offset at which `needle` occurs in `haystack`.
fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// Frame one message for the wire.
pub fn encode(body: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(body.len() + 32);
    out.extend_from_slice(format!("Content-Length: {}\r\n\r\n", body.len()).as_bytes());
    out.extend_from_slice(body.as_bytes());
    out
}

#[cfg(test)]
mod tests {
    use super::{Decoder, FrameError, encode};

    fn decode_all(chunks: &[&[u8]]) -> Vec<Result<String, FrameError>> {
        let mut decoder = Decoder::new();
        let mut out = Vec::new();
        for chunk in chunks {
            decoder.push(chunk);
            while let Some(message) = decoder.next_message() {
                out.push(message);
            }
        }
        out
    }

    #[test]
    fn one_whole_frame_decodes() {
        let messages = decode_all(&[b"Content-Length: 2\r\n\r\n{}"]);
        assert_eq!(messages, vec![Ok("{}".to_owned())]);
    }

    #[test]
    fn a_frame_split_across_reads_decodes_once_complete() {
        // The starvation-safe property that matters: a partial frame yields
        // nothing and leaves the decoder ready for the rest.
        let messages = decode_all(&[b"Content-Len", b"gth: 7\r\n\r\n{\"a\"", b":1}"]);
        assert_eq!(messages, vec![Ok("{\"a\":1}".to_owned())]);
    }

    #[test]
    fn two_frames_in_one_read_both_decode() {
        let messages = decode_all(&[b"Content-Length: 2\r\n\r\n{}Content-Length: 2\r\n\r\n[]"]);
        assert_eq!(messages, vec![Ok("{}".to_owned()), Ok("[]".to_owned())]);
    }

    #[test]
    fn extra_headers_are_ignored() {
        let messages = decode_all(&[
            b"Content-Type: application/vscode-jsonrpc; charset=utf-8\r\nContent-Length: 2\r\n\r\n{}",
        ]);
        assert_eq!(messages, vec![Ok("{}".to_owned())]);
    }

    #[test]
    fn a_bare_lf_terminator_is_accepted() {
        let messages = decode_all(&[b"Content-Length: 2\n\n{}"]);
        assert_eq!(messages, vec![Ok("{}".to_owned())]);
    }

    #[test]
    fn a_header_block_without_a_length_resynchronises() {
        let messages = decode_all(&[b"X-Nonsense: 1\r\n\r\nContent-Length: 2\r\n\r\n{}"]);
        assert_eq!(
            messages,
            vec![Err(FrameError::MissingContentLength), Ok("{}".to_owned())]
        );
    }

    /// The orphaned body costs the frame it belongs to, and no more.
    ///
    /// This is the case that motivates `resynchronise`: the body carries no
    /// trailing newline, so without the forward scan it would glue itself to
    /// the next `Content-Length` line, hide that header behind its own JSON
    /// colon, and swallow the good frame as well.
    #[test]
    fn a_length_less_frames_body_is_skipped_and_the_next_frame_decodes() {
        let messages =
            decode_all(&[b"X-Nonsense: 1\r\n\r\n{\"orphaned\":true}Content-Length: 2\r\n\r\n{}"]);
        assert_eq!(
            messages,
            vec![Err(FrameError::MissingContentLength), Ok("{}".to_owned())],
            "the orphaned body is skipped and the following frame still decodes"
        );
    }

    /// The resync matches the header name case-insensitively, as the base
    /// protocol says header names are.
    #[test]
    fn resynchronisation_ignores_header_name_case() {
        let messages = decode_all(&[b"X-Nonsense: 1\r\n\r\n{\"a\":1}content-length: 2\r\n\r\n{}"]);
        assert_eq!(
            messages,
            vec![Err(FrameError::MissingContentLength), Ok("{}".to_owned())]
        );
    }

    /// The decoder always makes progress: even a run of malformed header
    /// blocks is consumed rather than rescanned forever, and the good frame
    /// at the end still arrives.
    #[test]
    fn repeated_malformed_header_blocks_still_make_progress() {
        let mut decoder = Decoder::new();
        decoder.push(b"A: 1\r\n\r\nB: 2\r\n\r\nC: 3\r\n\r\nContent-Length: 2\r\n\r\n{}");
        let mut seen = Vec::new();
        while let Some(message) = decoder.next_message() {
            seen.push(message);
        }
        assert_eq!(seen.last(), Some(&Ok("{}".to_owned())));
        assert_eq!(decoder.buffered(), 0, "nothing is left rescanning itself");
    }

    #[test]
    fn encode_round_trips_through_the_decoder() {
        let framed = encode("{\"jsonrpc\":\"2.0\"}");
        let messages = decode_all(&[&framed]);
        assert_eq!(messages, vec![Ok("{\"jsonrpc\":\"2.0\"}".to_owned())]);
    }

    /// A `Content-Length` no address space can hold is a malformed frame, not
    /// a fatal one.
    ///
    /// The end index is `body_start + length`, and `length` is whatever the
    /// client wrote: unchecked, it wraps below `body_start`, passes the
    /// completeness test, and slices backwards. Under `panic = "abort"` that
    /// ends the session, so the property under test is that the decoder
    /// survives the frame *and* still parses the next one.
    #[test]
    fn a_content_length_near_usize_max_is_rejected_and_the_next_frame_decodes() {
        let stream = format!(
            "Content-Length: {}\r\n\r\n{{\"orphaned\":true}}Content-Length: 2\r\n\r\n{{}}",
            usize::MAX
        );
        let messages = decode_all(&[stream.as_bytes()]);
        assert_eq!(
            messages,
            vec![
                Err(FrameError::ContentLengthOutOfRange),
                Ok("{}".to_owned())
            ]
        );
    }

    /// Every length that *can* address a body stays a completeness question.
    ///
    /// The rejection above is about representability alone — an enormous but
    /// addressable length is simply a frame that has not arrived yet, and the
    /// decoder must keep waiting for it rather than reporting an error.
    #[test]
    fn a_huge_but_addressable_content_length_is_merely_incomplete() {
        let mut decoder = Decoder::new();
        decoder.push(format!("Content-Length: {}\r\n\r\n", usize::MAX / 2).as_bytes());
        decoder.push(b"{}");
        assert!(decoder.next_message().is_none());
        assert!(decoder.buffered() > 0);
    }

    #[test]
    fn a_partial_frame_leaves_its_bytes_buffered() {
        let mut decoder = Decoder::new();
        decoder.push(b"Content-Length: 9\r\n\r\n{}");
        assert!(decoder.next_message().is_none());
        assert!(decoder.buffered() > 0);
    }
}
