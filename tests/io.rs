#![cfg(feature = "std")]

use std::io::{self, Write};
use std::str::from_utf8;

use indent_write::io::IndentWriter;

// This is a wrapper for io::Write that only writes one byte at a time, to test
// the invariants of IndentableWrite
#[derive(Debug, Clone)]
struct OneByteAtATime<W>(W);

impl<W: Write> Write for OneByteAtATime<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match *buf {
            [] => Ok(0),
            [b, ..] => self.0.write(&[b]),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}

const CONTENT: &'static [&'static str] = &["\t😀 😀 😀", "\t\t😀 😀 😀", "\t😀 😀 😀"];

#[test]
fn basic_test() {
    let mut dest = Vec::new();

    {
        let mut writer = IndentWriter::new("\t", &mut dest);
        writer.inc();
        for line in CONTENT {
            writeln!(writer, "{}", line).unwrap();
        }
    }

    let result = from_utf8(&dest).expect("Wrote invalid utf8 to dest");
    assert_eq!(result, "\t\t😀 😀 😀\n\t\t\t😀 😀 😀\n\t\t😀 😀 😀\n");
}

#[test]
fn test_prefix() {
    let mut dest = Vec::new();
    let mut writer = IndentWriter::new("    ", &mut dest);
    writer.inc();

    for line in CONTENT {
        write!(writer, "{}\n", line).unwrap();
    }

    let result = from_utf8(&dest).expect("Wrote invalid utf8 to dest");
    assert_eq!(result, "    \t😀 😀 😀\n    \t\t😀 😀 😀\n    \t😀 😀 😀\n");
}

#[test]
fn test_inc_and_dec() {
    let mut dest = Vec::new();
    let mut writer = IndentWriter::new("    ", &mut dest);

    writeln!(writer, "<trk>").unwrap();

    writer.inc();
    writeln!(writer, "<name>Lincs Riding</name>").unwrap();
    writeln!(writer, "<trkseg>").unwrap();

    writer.inc();
    writeln!(writer, "<trkpt lat=\"53.246708\" lon=\"-0.801052\">").unwrap();

    writer.inc();
    writeln!(writer, "<ele>16.4</ele>").unwrap();
    writeln!(writer, "<time>2024-01-02T10:52:25Z</time>").unwrap();

    writer.dec();
    writeln!(writer, "</trkpt>").unwrap();

    writer.dec();
    writeln!(writer, "</trkseg>").unwrap();
    writeln!(writer, "<extensions>\n    <hr>130</hr>\n</extensions>").unwrap();

    writer.dec();
    writeln!(writer, "</trk>").unwrap();

    let result = from_utf8(&dest).expect("Wrote invalid utf8 to dest");
    assert_eq!(
        result,
        "<trk>
    <name>Lincs Riding</name>
    <trkseg>
        <trkpt lat=\"53.246708\" lon=\"-0.801052\">
            <ele>16.4</ele>
            <time>2024-01-02T10:52:25Z</time>
        </trkpt>
    </trkseg>
    <extensions>
        <hr>130</hr>
    </extensions>
</trk>
"
    );
}

#[test]
fn test_reset() {
    let mut dest = Vec::new();
    let mut writer = IndentWriter::new("    ", &mut dest);
    writer.inc();

    writeln!(writer, "FIRST").unwrap();
    writer.reset();
    writeln!(writer, "SECOND").unwrap();

    let result = from_utf8(&dest).expect("Wrote invalid utf8 to dest");
    assert_eq!(result, "    FIRST\nSECOND\n");
}

#[test]
fn test_multi_indent() {
    let mut dest = Vec::new();
    writeln!(dest, "{}", "😀 😀 😀").unwrap();
    {
        let mut indent1 = IndentWriter::new("\t", &mut dest);
        indent1.inc();
        writeln!(indent1, "{}", "😀 😀 😀").unwrap();
        {
            let mut indent2 = IndentWriter::new("\t", &mut indent1);
            indent2.inc();
            writeln!(indent2, "{}", "😀 😀 😀").unwrap();
            {
                let mut indent3 = IndentWriter::new("\t", &mut indent2);
                indent3.inc();
                writeln!(indent3, "{}", "😀 😀 😀").unwrap();
                writeln!(indent3, "").unwrap();
            }
            writeln!(indent2, "{}", "😀 😀 😀").unwrap();
        }
        writeln!(indent1, "{}", "😀 😀 😀").unwrap();
    }

    let result = from_utf8(&dest).expect("Wrote invalid utf8 to dest");
    assert_eq!(
        result,
        "😀 😀 😀
\t😀 😀 😀
\t\t😀 😀 😀
\t\t\t😀 😀 😀

\t\t😀 😀 😀
\t😀 😀 😀\n"
    )
}

// Technically this doesn't test anything in the crate, it just ensures that OneByteAtATime works
#[test]
fn test_partial_writes() {
    let mut dest = Vec::new();
    {
        let mut partial_writer = OneByteAtATime(&mut dest);
        write!(partial_writer, "Hello, {}!", "World").unwrap();
    }
    assert_eq!(from_utf8(&dest), Ok("Hello, World!"));
}

#[test]
fn test_partial_simple_indent_writes() {
    let mut dest = Vec::new();
    {
        let writer = OneByteAtATime(&mut dest);
        let mut writer = IndentWriter::new("\t", writer);
        writer.inc();
        write!(writer, "{}\n", "Hello, World").unwrap();
        write!(writer, "{}\n", "😀 😀 😀\n😀 😀 😀").unwrap();
    }
    assert_eq!(
        from_utf8(&dest),
        Ok("\tHello, World\n\t😀 😀 😀\n\t😀 😀 😀\n")
    );
}

#[test]
fn test_partial_simple_indent_writes_inverted() {
    let mut dest = Vec::new();
    {
        let mut writer = IndentWriter::new("\t", &mut dest);
        writer.inc();
        let mut writer = OneByteAtATime(writer);
        write!(writer, "{}\n", "Hello, World").unwrap();
        write!(writer, "{}\n", "😀 😀 😀\n😀 😀 😀").unwrap();
    }
    assert_eq!(
        from_utf8(&dest),
        Ok("\tHello, World\n\t😀 😀 😀\n\t😀 😀 😀\n")
    );
}

#[test]
fn test_partial_writes_combined() {
    let mut dest = Vec::new();
    {
        let writer = OneByteAtATime(&mut dest);
        let mut writer = IndentWriter::new("    ", writer);
        writer.inc();
        let mut writer = OneByteAtATime(writer);

        write!(writer, "{}\n", "Hello, World").unwrap();
        write!(writer, "{}\n", "😀 😀 😀\n😀 😀 😀").unwrap();
    }
    assert_eq!(
        from_utf8(&dest),
        Ok("    Hello, World\n    😀 😀 😀\n    😀 😀 😀\n")
    );
}
