use cani::{FirstLegal, run, run_buffered_with_selector, run_with_selector};
use std::io::{self, BufReader, Read, Write};
struct Chunked<'a> {
    data: &'a [u8],
    chunk: usize,
    calls: usize,
    interrupt: bool,
}
impl Read for Chunked<'_> {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        self.calls += 1;
        if self.interrupt {
            self.interrupt = false;
            return Err(io::ErrorKind::Interrupted.into());
        }
        let n = out.len().min(self.chunk).min(self.data.len());
        out[..n].copy_from_slice(&self.data[..n]);
        self.data = &self.data[n..];
        Ok(n)
    }
}
#[test]
fn chunk_boundaries_limits_and_bulk_reads() {
    let input = format!("hello 1\r\n{}\nready\r\nquit\nignored\n", "x".repeat(10000));
    for chunk in [1, 2, 3, 4096, 8192] {
        let mut reader = Chunked {
            data: input.as_bytes(),
            chunk,
            calls: 0,
            interrupt: true,
        };
        let mut output = vec![];
        run(&mut reader, &mut output).unwrap();
        assert_eq!(output, b"ok usp 1\nerror syntax\nok ready\nok bye\n");
        if chunk == 8192 {
            assert!(reader.calls < 5);
        }
    }
    let mut reader = Chunked {
        data: b"hello 1\nready\nquit\n",
        chunk: 8192,
        calls: 0,
        interrupt: false,
    };
    run(&mut reader, &mut vec![]).unwrap();
    assert_eq!(reader.calls, 1);
    for ending in ["", "\r"] {
        for len in [4096, 4097, 4098, 100000] {
            let input = format!("hello 1\n{}{ending}\nready\n", "x".repeat(len));
            let mut output = vec![];
            run(&mut input.as_bytes(), &mut output).unwrap();
            assert_eq!(output, b"ok usp 1\nerror syntax\nok ready\n");
        }
    }
}
struct FailRead;
impl Read for FailRead {
    fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
        Err(io::ErrorKind::BrokenPipe.into())
    }
}
struct FailWrite {
    flush: bool,
}
impl Write for FailWrite {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if self.flush {
            Ok(buf.len())
        } else {
            Err(io::ErrorKind::BrokenPipe.into())
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        Err(io::ErrorKind::BrokenPipe.into())
    }
}
#[test]
fn stream_failures_eof_and_quit() {
    assert_eq!(
        run(&mut FailRead, &mut vec![]).unwrap_err().kind(),
        io::ErrorKind::BrokenPipe
    );
    for input in [b"hello 1\n".as_slice(), b"quit\n"] {
        for flush in [false, true] {
            assert!(run(&mut &input[..], &mut FailWrite { flush }).is_err());
        }
    }
    for incomplete in [b"hello 1".as_slice(), b"hello 1\r", b"quit"] {
        let mut output = vec![];
        run(&mut &incomplete[..], &mut output).unwrap();
        assert!(output.is_empty());
    }
    let mut reader = BufReader::new(b"quit\nready\n".as_slice());
    let mut output = vec![];
    run_buffered_with_selector(&mut reader, &mut output, FirstLegal).unwrap();
    let mut trailing = String::new();
    reader.read_to_string(&mut trailing).unwrap();
    assert_eq!(trailing, "ready\n");
    assert_eq!(output, b"ok bye\n");
    let mut output = vec![];
    assert!(
        run_with_selector(
            &mut b"hello 1\ngo\n".as_slice(),
            &mut output,
            |_: &cani::State, _: &[cani::Turn]| cani::Turn::default()
        )
        .is_err()
    );
    assert_eq!(output, b"ok usp 1\n");
}
