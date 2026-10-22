#![cfg(feature = "ssgf")]
//! Isolated allocator regression: rejected input must not become a JSON tree.
use cani::ssgf::{ErrorKind, GameRecord, Limits};
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
};
thread_local! {static TRACK:Cell<bool>=const {Cell::new(false)};static MAX:Cell<usize>=const {Cell::new(0)};}
struct Allocator;
fn track(size: usize) {
    if TRACK.try_with(Cell::get).unwrap_or(false) {
        let _ = MAX.try_with(|c| c.set(c.get().max(size)));
    }
}
unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        track(l.size());
        unsafe { System.alloc(l) }
    }
    unsafe fn alloc_zeroed(&self, l: Layout) -> *mut u8 {
        track(l.size());
        unsafe { System.alloc_zeroed(l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        track(n);
        unsafe { System.realloc(p, l, n) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        unsafe { System.dealloc(p, l) }
    }
}
#[global_allocator]
static ALLOCATOR: Allocator = Allocator;
#[test]
fn oversized_turns_and_metadata_do_not_allocate_large_value_arrays() {
    let turns = format!(
        "{{\"turns\":[{}\"0a1a2:1b1b2\"]}}",
        "\"0a1a2:1b1b2\",".repeat(100000)
    );
    let metadata = format!(
        "{{\"extensions\":{{\"x\":[{}0]}},\"turns\":[]}}",
        "0,".repeat(100000)
    );
    for (input, limits) in [
        (
            &turns,
            Limits {
                turns: 0,
                ..Limits::default()
            },
        ),
        (
            &metadata,
            Limits {
                metadata_bytes: 256,
                ..Limits::default()
            },
        ),
    ] {
        MAX.with(|c| c.set(0));
        TRACK.with(|c| c.set(true));
        let error = GameRecord::parse(input.as_bytes(), limits).unwrap_err();
        TRACK.with(|c| c.set(false));
        assert_eq!(error.kind, ErrorKind::Limit);
        assert!(
            MAX.with(Cell::get) < 8192,
            "rejected data allocated a large tree"
        );
    }
}
