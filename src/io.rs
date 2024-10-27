use core::ops::Range;
use std::io;

#[derive(Debug, Clone)]
enum IndentState {
    // We are currently writing a line. Forward writes until the end of the
    // line.
    MidLine,

    // An indent has been requested. Write empty lines, then write an indent
    // before the next non empty line.
    NeedIndent,

    // We are currently writing an indent. This range represents the part of
    // `required_indent` that still needs to be written.
    WritingIndent(Range<usize>),
}

use IndentState::*;

/// Adapter for writers to indent each line
///
/// An `IndentWriter` adapts an [`io::Write`] object to insert an indent before
/// each non-empty line. Specifically, this means it will insert an indent
/// between each newline when followed by a non-newline.
///
/// An `IndentWriter` has an [`Self::indent_level`] which starts at 0, meaning
/// no indentation will be written. Call [`Self::inc()`] and [`Self::dec()`] to
/// increase and decrease the amount of indentation.
///
/// If you want to use differing indentation strings, say a mixture of tabs and
/// spaces, then you can nest writers.
///
/// # Example
///
/// ```
/// # use std::io::Write;
/// use indent_write::io::IndentWriter;
///
/// let output = Vec::new();
///
/// let mut indented = IndentWriter::new("\t", output);
/// indented.inc();
///
/// // Lines will be indented
/// write!(indented, "Line 1\nLine 2\n");
///
/// // Empty lines will not be indented
/// write!(indented, "\n\nLine 3\n\n");
///
/// assert_eq!(indented.get_ref(), b"\tLine 1\n\tLine 2\n\n\n\tLine 3\n\n");
/// ```
#[derive(Debug, Clone)]
pub struct IndentWriter<W> {
    writer: W,
    indent: String,
    indent_level: u16,
    // The `required_indent` is the `indent` repeated `indent_level` times.
    // We recalculate it when `indent_level` changes.
    required_indent: Vec<u8>,
    state: IndentState,
}

impl<W: io::Write> IndentWriter<W> {
    /// Create a new [`IndentWriter`] with a [`Self::indent_level()`] of 0
    /// and `indent` to be used to create the indentation.
    pub fn new<S: Into<String>>(indent: S, writer: W) -> Self {
        Self {
            writer,
            indent: indent.into(),
            indent_level: 0,
            required_indent: Vec::new(),
            state: NeedIndent,
        }
    }

    /// Increments the [`Self::indent_level()`] by 1.
    pub fn inc(&mut self) {
        self.indent_level = self.indent_level.saturating_add(1);
        self.required_indent
            .extend_from_slice(self.indent.as_bytes());
    }

    /// Decrements the [`Self::indent_level()`] by 1.
    pub fn dec(&mut self) {
        self.indent_level = self.indent_level.saturating_sub(1);
        // Note that len() is in bytes, not chars or graphemes so this is
        // correct.
        let new_len = self.required_indent.len() - self.indent.len();
        self.required_indent.truncate(new_len);
    }

    /// Resets the [`Self::indent_level()`] to 0.
    pub fn reset(&mut self) {
        self.indent_level = 0;
        self.required_indent.clear();
    }

    /// Extract the writer from the [`IndentWriter`], discarding any in-progress
    /// indent state.
    #[inline]
    pub fn into_inner(self) -> W {
        self.writer
    }

    /// Get a reference to the wrapped writer
    #[inline]
    pub fn get_ref(&self) -> &W {
        &self.writer
    }

    /// Get the string being used as an indent for each line
    #[inline]
    pub fn indent(&self) -> &str {
        &self.indent
    }
}

impl<W: io::Write> io::Write for IndentWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        loop {
            match self.state {
                // We're currently writing a line. Scan for the end of the line.
                IndentState::MidLine => match buf.iter().position(|&b| b == b'\n') {
                    // No newlines in the input buffer, so write the entire thing.
                    None => break self.writer.write(buf),

                    // We are at a newline presently. Request an indent be
                    // written at the front of the next non-empty line, then
                    // continue looping (since we haven't yet attempted to
                    // write user data).
                    Some(0) => self.state = NeedIndent,

                    // There's an upcoming newline. Write out the remainder of
                    // this line, plus its newline. If the entire line was
                    // written, request an indent on the subsequent call to
                    // write.
                    Some(len) => {
                        break self.writer.write(&buf[..len + 1]).inspect(|&n| {
                            if n >= len {
                                self.state = NeedIndent;
                            }
                        })
                    }
                },

                // We need an indent. Scan for the next non-empty line.
                IndentState::NeedIndent => match buf.iter().position(|&b| b != b'\n') {
                    // No non-empty lines in the input buffer, so write the entire thing
                    None => break self.writer.write(buf),

                    // We are at the beginning of a non-empty line presently.
                    // Begin inserting an indent now, then continue looping
                    // (since we haven't yet attempted to write user data)
                    Some(0) => self.state = WritingIndent(0..self.required_indent.len()),

                    // There's an upcoming non-empty line. Write out the
                    // remainder of the empty lines. If all the empty lines
                    // were written, force an indent on the subsequent call to
                    // write.
                    Some(len) => {
                        break self.writer.write(&buf[..len]).inspect(|&n| {
                            if n >= len {
                                self.state = WritingIndent(0..self.required_indent.len());
                            }
                        })
                    }
                },

                // We are writing an indent unconditionally. If we're in this
                // state, the input buffer is known to be the start of a non-
                // empty line.
                IndentState::WritingIndent(ref mut range) => {
                    match self.writer.write(&self.required_indent[range.clone()])? {
                        // We successfully wrote the entire indent. Continue with
                        // writing the input buffer.
                        n if n >= range.len() => self.state = MidLine,

                        // Eof; stop work immediately
                        0 => break Ok(0),

                        // Only a part of the indent was written. Continue
                        // trying to write the rest of it, but update our state
                        // to keep it consistent in case the next write is an
                        // error
                        n => range.start += n,
                    }
                }
            }
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        // If we're currently in the middle of writing an indent, flush it
        while let WritingIndent(ref mut range) = self.state {
            match self.writer.write(&self.required_indent[range.clone()])? {
                // We wrote the entire indent. Proceed with the flush
                len if len >= range.len() => self.state = MidLine,

                // EoF; return an error
                0 => return Err(io::ErrorKind::WriteZero.into()),

                // Partial write, continue writing.
                len => range.start += len,
            }
        }

        self.writer.flush()
    }
}
