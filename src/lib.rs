//! Shobu rules, SFEN, USP 1.0, and native bindings.
mod core;
pub mod ffi;
pub mod neural;
#[cfg(feature = "ssgf")]
pub mod ssgf;
pub mod symmetry;
pub use core::*;
use std::io::{self, BufRead, BufReader, Read, Write};
/// Compatibility error categories. Typed core methods expose structured errors.
pub type Error = &'static str;
type E = Error;

/// Select from a nonempty list of legal complete turns. The engine validates
/// the returned turn; selection never commits the position.
pub trait MoveSelector {
    fn select(&mut self, position: &State, legal: &[Turn]) -> Turn;
}
impl<F: FnMut(&State, &[Turn]) -> Turn> MoveSelector for F {
    fn select(&mut self, position: &State, legal: &[Turn]) -> Turn {
        self(position, legal)
    }
}
#[derive(Clone, Copy, Debug, Default)]
pub struct FirstLegal;
impl MoveSelector for FirstLegal {
    fn select(&mut self, _: &State, legal: &[Turn]) -> Turn {
        legal[0]
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SelectionError;
impl std::fmt::Display for SelectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("selector returned an illegal turn")
    }
}
impl std::error::Error for SelectionError {}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionError {
    Protocol(Error),
    Selection(SelectionError),
}
impl std::fmt::Display for SessionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Protocol(e) => f.write_str(e),
            Self::Selection(e) => e.fmt(f),
        }
    }
}
impl std::error::Error for SessionError {}

/// Synchronous USP session. Errors preserve state and negotiation.
#[derive(Clone)]
pub struct Engine<S = FirstLegal> {
    state: State,
    negotiated: bool,
    selector: S,
}
impl<S: MoveSelector> Engine<S> {
    pub fn handle(&mut self, line: &str) -> Result<String, E> {
        if line.is_empty()
            || line.len() > 4096
            || line.starts_with(' ')
            || line.ends_with(' ')
            || line.contains("  ")
            || !line.bytes().all(|c| (32..=126).contains(&c))
        {
            return Err("syntax");
        }
        if line == "quit" {
            return Ok("ok bye".into());
        }
        if line == "hello" || line.starts_with("hello ") {
            if line != "hello 1" {
                return Err("version");
            }
            self.negotiated = true;
            return Ok("ok usp 1".into());
        }
        if !self.negotiated {
            return Err("handshake");
        }
        match line {
            "ready" => Ok("ok ready".into()),
            "new" | "position startpos" => {
                self.state = State::parse(INITIAL)?;
                Ok(if line == "new" {
                    "ok new"
                } else {
                    "ok position"
                }
                .into())
            }
            "state" => Ok(format!("state {}", self.state.sfen())),
            "result" => Ok(format!("result {}", self.state.result())),
            "legal" => {
                let m = self.state.legal();
                Ok(if m.is_empty() {
                    "legal".into()
                } else {
                    format!("legal {}", m.join(" "))
                })
            }
            "go" => {
                let mut turns = [Turn::default(); MAX_LEGAL_TURNS];
                let count = self
                    .state
                    .legal_turns_into(&mut turns)
                    .expect("bounded turns");
                if count == 0 {
                    return Ok("bestmove none".into());
                }
                let selected = self.selector.select(&self.state, &turns[..count]);
                if !turns[..count].contains(&selected) {
                    return Err("selector");
                }
                Ok(format!("bestmove {selected}"))
            }
            _ => {
                if line == "position" || line.starts_with("position ") {
                    self.state = State::parse(line.get(9..).unwrap_or(""))?;
                    Ok("ok position".into())
                } else if line == "play" || line.starts_with("play ") {
                    self.state = self.state.play(line.get(5..).unwrap_or(""))?;
                    Ok("ok play".into())
                } else {
                    Err("syntax")
                }
            }
        }
    }
}
pub fn run(input: &mut impl Read, output: &mut impl Write) -> io::Result<()> {
    run_with_selector(input, output, FirstLegal)
}
pub fn run_with_selector<S: MoveSelector>(
    input: &mut impl Read,
    output: &mut impl Write,
    selector: S,
) -> io::Result<()> {
    // Keep the Read API compatible while amortizing reads. Callers that need
    // to retain bytes after quit should use run_buffered_with_selector directly.
    run_buffered_with_selector(&mut BufReader::new(input), output, selector)
}
/// Buffered transport retaining unread bytes after quit in the caller's reader.
pub fn run_buffered_with_selector<S: MoveSelector>(
    input: &mut impl BufRead,
    output: &mut impl Write,
    selector: S,
) -> io::Result<()> {
    let mut engine = Engine::with_selector(selector);
    let mut line = Vec::with_capacity(4098);
    let mut overflow = false;
    loop {
        let chunk = match input.fill_buf() {
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            result => result?,
        };
        if chunk.is_empty() {
            return Ok(());
        } // discard incomplete EOF request
        let newline = chunk.iter().position(|&b| b == b'\n');
        let len = newline.unwrap_or(chunk.len());
        let keep = len.min(4098 - line.len());
        line.extend_from_slice(&chunk[..keep]);
        overflow |= keep < len;
        input.consume(len + usize::from(newline.is_some()));
        if newline.is_none() {
            continue;
        }
        if line.last() == Some(&b'\r') {
            line.pop();
        }
        let reply = if overflow {
            "error syntax".into()
        } else {
            match std::str::from_utf8(&line)
                .map_err(|_| "syntax")
                .and_then(|s| engine.handle(s))
            {
                Ok(s) => s,
                Err("selector") => {
                    return Err(io::Error::new(io::ErrorKind::InvalidData, SelectionError));
                }
                Err(e) => format!("error {e}"),
            }
        };
        writeln!(output, "{reply}")?;
        output.flush()?;
        if reply == "ok bye" {
            return Ok(());
        }
        line.clear();
        overflow = false;
    }
}

impl Default for Engine<FirstLegal> {
    fn default() -> Self {
        Self::with_selector(FirstLegal)
    }
}
impl<S: MoveSelector> Engine<S> {
    pub fn with_selector(selector: S) -> Self {
        Self {
            state: State::default(),
            negotiated: false,
            selector,
        }
    }
    pub fn state(&self) -> &State {
        &self.state
    }
    /// Structured session errors. Selection failure has no USP wire encoding.
    pub fn handle_checked(&mut self, line: &str) -> Result<String, SessionError> {
        self.handle(line).map_err(|e| {
            if e == "selector" {
                SessionError::Selection(SelectionError)
            } else {
                SessionError::Protocol(e)
            }
        })
    }
}
