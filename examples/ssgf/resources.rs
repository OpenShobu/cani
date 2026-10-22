//! Reproducible replay/parser heap and transport-read benchmark. No model runtime.
use cani::{
    Turn,
    neural::ActionIndex,
    ssgf::{GameRecord, Limits},
};
use std::{
    alloc::{GlobalAlloc, Layout, System},
    io::{self, Read},
    sync::atomic::{AtomicUsize, Ordering::Relaxed},
    time::Instant,
};
static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);
static ALLOCATED: AtomicUsize = AtomicUsize::new(0);
struct Tracked;
fn add(n: usize) {
    let live = LIVE.fetch_add(n, Relaxed) + n;
    PEAK.fetch_max(live, Relaxed);
    ALLOCATED.fetch_add(n, Relaxed);
}
unsafe impl GlobalAlloc for Tracked {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        let p = unsafe { System.alloc(l) };
        if !p.is_null() {
            add(l.size());
        }
        p
    }
    unsafe fn alloc_zeroed(&self, l: Layout) -> *mut u8 {
        let p = unsafe { System.alloc_zeroed(l) };
        if !p.is_null() {
            add(l.size());
        }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        LIVE.fetch_sub(l.size(), Relaxed);
        unsafe { System.dealloc(p, l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        let q = unsafe { System.realloc(p, l, n) };
        if !q.is_null() {
            LIVE.fetch_sub(l.size(), Relaxed);
            add(n);
        }
        q
    }
}
#[global_allocator]
static ALLOCATOR: Tracked = Tracked;
fn measure(name: &str, case: impl FnOnce() -> String) {
    let baseline = LIVE.load(Relaxed);
    PEAK.store(baseline, Relaxed);
    let allocated = ALLOCATED.load(Relaxed);
    let start = Instant::now();
    let result = case();
    let elapsed = start.elapsed();
    let peak = PEAK.load(Relaxed).saturating_sub(baseline);
    let bytes = ALLOCATED.load(Relaxed) - allocated;
    println!(
        "{name},{:.3},{peak},{bytes},{result}",
        elapsed.as_secs_f64() * 1000.0
    );
}
fn parse(data: &[u8], limits: Limits) -> String {
    match GameRecord::parse(data, limits) {
        Ok(r) => format!("ok {} turns", r.turns().len()),
        Err(e) => format!("{:?} {:?}", e.kind, e.turn_index),
    }
}
struct CountReads<'a> {
    data: &'a [u8],
    max: usize,
    calls: usize,
}
impl Read for CountReads<'_> {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        self.calls += 1;
        let n = out.len().min(self.data.len()).min(self.max);
        out[..n].copy_from_slice(&self.data[..n]);
        self.data = &self.data[n..];
        Ok(n)
    }
}
fn main() {
    let n = std::env::args()
        .nth(1)
        .map_or(1_000_000, |s| s.parse::<usize>().expect("turn count"));
    assert!(n <= 1_000_000);
    let cycle = ["0a1a2:1b1b2", "2a4a3:3b4b3", "0a2a1:1b2b1", "2a3a4:3b3b4"];
    let mut readable = String::from(
        r#"{"format":"RSSGF","version":1,"rules":"usp-game-1.0","result":{"outcome":"ongoing","reason":"unfinished"},"turns":["#,
    );
    for i in 0..n {
        if i > 0 {
            readable.push(',');
        }
        readable.push('"');
        readable.push_str(cycle[i % 4]);
        readable.push('"');
    }
    readable.push_str("]}");
    let metadata=br#"{"format":"BSSGF","version":1,"rules":"usp-game-1.0","result":{"outcome":"ongoing","reason":"unfinished"}}"#;
    let mut binary = b"BSSGF\r\n\x1a".to_vec();
    binary.extend_from_slice(&1u16.to_le_bytes());
    binary.extend_from_slice(&0u16.to_le_bytes());
    binary.extend_from_slice(&(metadata.len() as u32).to_le_bytes());
    binary.extend_from_slice(&(n as u32).to_le_bytes());
    binary.extend_from_slice(metadata);
    let words = cycle.map(|s| ActionIndex::encode(s.parse::<Turn>().unwrap()).unwrap().0);
    for i in 0..n {
        binary.extend_from_slice(&words[i % 4].to_le_bytes());
    }
    println!("case,elapsed_ms,peak_extra_heap_bytes,total_allocated_bytes,result");
    measure("rssgf_valid", || {
        parse(readable.as_bytes(), Limits::default())
    });
    measure("rssgf_reject_limit", || {
        parse(
            readable.as_bytes(),
            Limits {
                turns: 0,
                ..Limits::default()
            },
        )
    });
    measure("bssgf_valid", || parse(&binary, Limits::default()));
    if n > 0 {
        binary[20 + metadata.len()..22 + metadata.len()].copy_from_slice(&0x0108u16.to_le_bytes());
    }
    measure("bssgf_invalid_first", || parse(&binary, Limits::default()));
    let metadata_bomb = format!(
        "{{\"extensions\":{{\"x\":[{}0]}},\"turns\":[]}}",
        "0,".repeat(n)
    );
    measure("rssgf_metadata_reject", || {
        parse(
            metadata_bomb.as_bytes(),
            Limits {
                metadata_bytes: 256,
                ..Limits::default()
            },
        )
    });
    let transcript = format!("hello 1\n{}quit\n", "ready\n".repeat(n));
    for (name, max) in [
        ("transport_bulk", usize::MAX),
        ("transport_single_byte_source", 1),
    ] {
        measure(name, || {
            let mut reader = CountReads {
                data: transcript.as_bytes(),
                max,
                calls: 0,
            };
            cani::run(&mut reader, &mut io::sink()).unwrap();
            format!("{} underlying reads", reader.calls)
        });
    }
}
