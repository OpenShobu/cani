//! Shobu rules and USP 1.0, adapted from DaShobu's Apache-2.0 reference engine.
//! Coordinates and adjudication follow USP game profile 1.0.
pub mod ffi;
use std::io::{self, Read, Write};
pub const MAX_PLY: u32 = 2147483647;
pub const INITIAL: &str = "WWWW/4/4/BBBB|WWWW/4/4/BBBB|WWWW/4/4/BBBB|WWWW/4/4/BBBB b 0";
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct State {
    boards: [[u8; 16]; 4],
    side: u8,
    ply: u32,
}
/// Stable error categories: syntax, sfen, illegal, handshake, version.
pub type Error = &'static str;
type E = Error;
impl State {
    pub fn parse(text: &str) -> Result<Self, E> {
        if text.len() > 4096 {
            return Err("sfen");
        }
        let parts: Vec<_> = text.split(' ').collect();
        if parts.len() != 3 || !matches!(parts[1], "b" | "w") {
            return Err("sfen");
        }
        let counter = parts[2];
        if counter.is_empty()
            || counter.len() > 10
            || !counter.bytes().all(|b| b.is_ascii_digit())
            || (counter.len() > 1 && counter.starts_with('0'))
        {
            return Err("sfen");
        }
        let ply = counter.parse::<u32>().map_err(|_| "sfen")?;
        if ply > MAX_PLY {
            return Err("sfen");
        }
        let boards: Vec<_> = parts[0].split('|').collect();
        if boards.len() != 4 {
            return Err("sfen");
        }
        let mut s = Self {
            boards: [[b'.'; 16]; 4],
            side: parts[1].as_bytes()[0],
            ply,
        };
        let (mut eb, mut ew) = (false, false);
        for (bid, board) in boards.iter().enumerate() {
            let ranks: Vec<_> = board.split('/').collect();
            if ranks.len() != 4 {
                return Err("sfen");
            }
            for (row, rank) in ranks.iter().rev().enumerate() {
                let (mut x, mut digit) = (0usize, false);
                for c in rank.bytes() {
                    if (b'1'..=b'4').contains(&c) {
                        if digit {
                            return Err("sfen");
                        }
                        x += (c - b'0') as usize;
                        digit = true;
                    } else if c == b'B' || c == b'W' {
                        if x >= 4 {
                            return Err("sfen");
                        }
                        s.boards[bid][row * 4 + x] = c;
                        x += 1;
                        digit = false;
                    } else {
                        return Err("sfen");
                    }
                    if x > 4 {
                        return Err("sfen");
                    }
                }
                if x != 4 {
                    return Err("sfen");
                }
            }
            let nb = s.boards[bid].iter().filter(|&&c| c == b'B').count();
            let nw = s.boards[bid].iter().filter(|&&c| c == b'W').count();
            if nb > 4 || nw > 4 {
                return Err("sfen");
            }
            eb |= nb == 0;
            ew |= nw == 0;
        }
        if eb && ew {
            return Err("sfen");
        }
        Ok(s)
    }
    pub fn sfen(&self) -> String {
        let mut out = String::new();
        for bid in 0..4 {
            if bid > 0 {
                out.push('|');
            }
            for y in (0..4).rev() {
                if y < 3 {
                    out.push('/');
                }
                let mut empty = 0u8;
                for x in 0..4 {
                    let c = self.boards[bid][y * 4 + x];
                    if c == b'.' {
                        empty += 1;
                    } else {
                        if empty > 0 {
                            out.push((b'0' + empty) as char);
                        }
                        empty = 0;
                        out.push(c as char);
                    }
                }
                if empty > 0 {
                    out.push((b'0' + empty) as char);
                }
            }
        }
        format!("{} {} {}", out, self.side as char, self.ply)
    }
    fn own(&self) -> u8 {
        if self.side == b'b' { b'B' } else { b'W' }
    }
    fn empty_winner(&self) -> Option<char> {
        if self.boards.iter().any(|b| !b.contains(&b'B')) {
            Some('w')
        } else if self.boards.iter().any(|b| !b.contains(&b'W')) {
            Some('b')
        } else {
            None
        }
    }
    fn raw(&self, m: &str) -> Result<Self, E> {
        let v = decode(m)?;
        let dx = v[3] - v[1];
        let dy = v[4] - v[2];
        let n = dx.abs().max(dy.abs());
        if v[0] / 2 != if self.side == b'b' { 0 } else { 1 }
            || v[0] % 2 == v[5] % 2
            || !(1..=2).contains(&n)
            || (dx.abs() != 0 && dx.abs() != n)
            || (dy.abs() != 0 && dy.abs() != n)
            || v[8] - v[6] != dx
            || v[9] - v[7] != dy
        {
            return Err("illegal");
        }
        let ux = dx / n;
        let uy = dy / n;
        let mut out = self.clone();
        for leg in 0..2 {
            let a = &v[leg * 5..leg * 5 + 5];
            let (mut x, mut y, mut pushed) = (a[1], a[2], -2i32);
            let b = &mut out.boards[a[0] as usize];
            if b[(y * 4 + x) as usize] != self.own() {
                return Err("illegal");
            }
            for _ in 0..n {
                let (nx, ny) = (x + ux, y + uy);
                if !inside(nx, ny) {
                    return Err("illegal");
                }
                let target = ny * 4 + nx;
                if b[target as usize] != b'.' {
                    if leg == 0
                        || b[target as usize] == self.own()
                        || (pushed != -2 && pushed != target)
                    {
                        return Err("illegal");
                    }
                    let (px, py) = (nx + ux, ny + uy);
                    let next = if inside(px, py) { py * 4 + px } else { -1 };
                    if next >= 0 {
                        if b[next as usize] != b'.' {
                            return Err("illegal");
                        }
                        b[next as usize] = b[target as usize];
                    }
                    pushed = next;
                }
                b[(y * 4 + x) as usize] = b'.';
                b[target as usize] = self.own();
                x = nx;
                y = ny;
            }
        }
        out.side = if self.side == b'b' { b'w' } else { b'b' };
        out.ply = (self.ply + 1).min(MAX_PLY);
        Ok(out)
    }
    pub fn play(&self, m: &str) -> Result<Self, E> {
        decode(m)?;
        if self.ply == MAX_PLY || self.empty_winner().is_some() {
            return Err("illegal");
        }
        self.raw(m)
    }
    fn generate(&self, enforce_limit: bool) -> Vec<String> {
        if self.empty_winner().is_some() || (enforce_limit && self.ply == MAX_PLY) {
            return vec![];
        }
        let mut out = vec![];
        for pb in 0i32..4 {
            for src in 0i32..16 {
                if pb / 2 != if self.side == b'b' { 0 } else { 1 }
                    || self.boards[pb as usize][src as usize] != self.own()
                {
                    continue;
                }
                let (x, y) = (src % 4, src / 4);
                for ux in -1..=1 {
                    for uy in -1..=1 {
                        for n in 1..=2 {
                            let (tx, ty) = (x + ux * n, y + uy * n);
                            if (ux == 0 && uy == 0) || !inside(tx, ty) {
                                continue;
                            }
                            for ab in 0i32..4 {
                                for src2 in 0i32..16 {
                                    if pb % 2 == ab % 2
                                        || self.boards[ab as usize][src2 as usize] != self.own()
                                    {
                                        continue;
                                    }
                                    let (ax, ay) = (src2 % 4, src2 / 4);
                                    let (atx, aty) = (ax + ux * n, ay + uy * n);
                                    if !inside(atx, aty) {
                                        continue;
                                    }
                                    let m = format!(
                                        "{}{}{}{}{}:{}{}{}{}{}",
                                        pb,
                                        (b'a' + x as u8) as char,
                                        y + 1,
                                        (b'a' + tx as u8) as char,
                                        ty + 1,
                                        ab,
                                        (b'a' + ax as u8) as char,
                                        ay + 1,
                                        (b'a' + atx as u8) as char,
                                        aty + 1
                                    );
                                    if self.raw(&m).is_ok() {
                                        out.push(m);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        out.sort();
        out
    }
    pub fn result(&self) -> String {
        if let Some(w) = self.empty_winner() {
            w.to_string()
        } else if self.generate(false).is_empty() {
            if self.side == b'b' { "w" } else { "b" }.into()
        } else {
            "ongoing".into()
        }
    }
}
fn inside(x: i32, y: i32) -> bool {
    (0..4).contains(&x) && (0..4).contains(&y)
}
fn decode(m: &str) -> Result<[i32; 10], E> {
    let b = m.as_bytes();
    if b.len() != 11 || b[5] != b':' {
        return Err("syntax");
    }
    let mut v = [0; 10];
    for leg in 0..2 {
        let a = &b[leg * 6..leg * 6 + 5];
        if !(b'0'..=b'3').contains(&a[0])
            || !(b'a'..=b'd').contains(&a[1])
            || !(b'1'..=b'4').contains(&a[2])
            || !(b'a'..=b'd').contains(&a[3])
            || !(b'1'..=b'4').contains(&a[4])
        {
            return Err("syntax");
        }
        for (i, base) in b"0a1a1".iter().enumerate() {
            v[leg * 5 + i] = (a[i] - base) as i32;
        }
    }
    Ok(v)
}
/// Synchronous USP session. Errors preserve state and negotiation.
#[derive(Default)]
pub struct Engine {
    state: State,
    negotiated: bool,
}
impl Engine {
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
                let m = self.state.legal();
                Ok(format!(
                    "bestmove {}",
                    m.first().map(String::as_str).unwrap_or("none")
                ))
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
    let mut engine = Engine::default();
    let mut buf = Vec::with_capacity(4098);
    let mut overflow = false;
    let mut byte = [0u8; 1];
    while input.read(&mut byte)? != 0 {
        if byte[0] != b'\n' {
            if buf.len() < 4098 {
                buf.push(byte[0]);
            } else {
                overflow = true;
            }
            continue;
        }
        if buf.last() == Some(&b'\r') {
            buf.pop();
        }
        let reply = if overflow {
            "error syntax".into()
        } else {
            match std::str::from_utf8(&buf)
                .map_err(|_| "syntax")
                .and_then(|s| engine.handle(s))
            {
                Ok(s) => s,
                Err(e) => format!("error {}", e),
            }
        };
        writeln!(output, "{}", reply)?;
        output.flush()?;
        if reply == "ok bye" {
            break;
        }
        buf.clear();
        overflow = false;
    }
    Ok(())
}

impl Default for State {
    fn default() -> Self {
        Self::parse(INITIAL).expect("valid initial state")
    }
}
/// Stone color, independent of board color.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Player {
    Black,
    White,
}
impl State {
    /// Canonically ordered complete turns. Empty at termination or the ply limit.
    pub fn legal(&self) -> Vec<String> {
        self.generate(true)
    }
    pub fn side_to_move(&self) -> Player {
        if self.side == b'b' {
            Player::Black
        } else {
            Player::White
        }
    }
    pub fn ply(&self) -> u32 {
        self.ply
    }
    /// Board 0/1 are Black's homeboards; 2/3 are White's.
    /// Square index is rank * 4 + file, with a1 = 0.
    pub fn stone(&self, board: usize, square: usize) -> Option<Player> {
        match self.boards.get(board)?.get(square)? {
            b'B' => Some(Player::Black),
            b'W' => Some(Player::White),
            _ => None,
        }
    }
    pub fn winner(&self) -> Option<Player> {
        match self.result().as_str() {
            "b" => Some(Player::Black),
            "w" => Some(Player::White),
            _ => None,
        }
    }
}
impl Engine {
    pub fn state(&self) -> &State {
        &self.state
    }
}
