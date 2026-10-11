//! Small file conversion/replay example, independent of USP transport.
use cani::ssgf::{Encoding, GameRecord, Limits};
use std::{
    error::Error,
    fs::File,
    io::{Read, Write},
};
fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 4 || !matches!(args[3].as_str(), "rssgf" | "bssgf") {
        return Err("usage: ssgf INPUT OUTPUT rssgf|bssgf".into());
    }
    let limits = Limits::default();
    let mut input = Vec::new();
    File::open(&args[1])?
        .take(limits.total_bytes as u64 + 1)
        .read_to_end(&mut input)?;
    let record = GameRecord::parse(&input, limits)?;
    let encoding = if args[3] == "rssgf" {
        Encoding::Readable
    } else {
        Encoding::Binary
    };
    let bytes = record.write(encoding, limits)?;
    if let Some(parent) = std::path::Path::new(&args[2]).parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }
    File::create(&args[2])?.write_all(&bytes)?;
    eprintln!(
        "{} turns, final SFEN: {}",
        record.turns().len(),
        record.final_position().sfen()
    );
    Ok(())
}
