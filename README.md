# cani (カニ)

A dependency-free Shobu rules SDK for Rust, with a C ABI usable from C23 and a
C++26-compatible value wrapper. Builds a Rust library, static/shared native
libraries, and the `cani-usp` stdin/stdout engine.

## Rust

```rust
use cani::{State, Player};

let position = State::default();
assert_eq!(position.legal().len(), 232);
let next = position.play("0a1a2:1b1b2")?;
assert_eq!(next.side_to_move(), Player::White);
println!("{}", next.sfen());
# Ok::<(), cani::Error>(())
```

Positions own four 16-square arrays and enforce their invariants through private
fields. `stone(board, square)` inspects stones; `side_to_move` and `ply` inspect
turn metadata. `play` returns a new position, preserving its input on success or
failure. `legal` returns sorted complete-turn encodings. `winner` returns a player
or `None`; `result` returns the USP outcome string. Parsing accepts structurally
valid study positions without asserting historical reachability.

From Black's seat, boards 0/1 are Black's homeboards and 2/3 White's. Even boards
are dark; odd boards are light. Square indices are `(rank - 1) * 4 + file`, with
a1 = 0. White uses the same absolute coordinates. Each complete turn contains a
passive and aggressive leg, e.g. `0a1a2:1b1b2`.

## Rules and protocol

The implementation follows the publisher-maintained
[Dized rules](https://rules.dized.com/game/VJQ5V5aAR4aqK31q1V1OXA/shobu):
one or two straight/diagonal steps on a homeboard without jumping or pushing,
then the identical displacement on an opposite-color board. Only the aggressive
leg can push one enemy stone, including repeatedly during a two-step move, or
capture it off the edge. Friendly stones and occupied push destinations block
the move. Both legs commit atomically. Removing all enemy stones on any one
board wins.

Coordinates, blocked-player losses, no automatic repetition draw, and the
2147483647 turn-counter limit follow DaShobu's independent
[game profile 1.0](https://github.com/DaShobu/Universal-Shobu-Protocol-Specifications/blob/main/docs/game.md).
The counter limit prevents further play but does not itself declare a loss.

`State::parse` / `sfen` implement canonical
[SFEN 1.0](https://github.com/DaShobu/Universal-Shobu-Protocol-Specifications/blob/main/docs/sfen.md).
`Engine::handle` provides an in-process
[USP 1.0](https://github.com/DaShobu/Universal-Shobu-Protocol-Specifications/blob/main/docs/protocol.md)
session. `run` adds bounded newline framing, LF/CRLF input, error recovery,
response flushing, and incomplete-EOF handling.

```text
cargo run --bin cani-usp
```

Then send `hello 1`, `state`, `legal`, `play 0a1a2:1b1b2`, `result`, `go`, and
`quit`, one per line. All mandatory commands are supported. `go` proposes the
first legal turn without committing it; competitive search is a later layer.

## C23 and C++26

Build with `cargo build --release`. Include `include/cani.h` in C or
`include/cani.hpp` in C++. Both link to the same Rust implementation. No duplicate
rules engine or Rust runtime process is required.

The C interface uses opaque owned position/session handles, explicit byte lengths,
status codes, and caller-owned output buffers. Output sizes include the NUL byte.
A `CANI_BUFFER` response reports the required size without changing state or
output. Free each handle once; avoid concurrent mutations and overlapping buffers.
See the header for the complete contract.

The C++ facade provides `cani::Position` with copy/move semantics, `sfen`, `play`,
`legal`, and `result`, plus `cani::Engine::handle`. Invalid inputs throw exceptions.
Headers use broadly supported language features and compile in C23/C++26 modes;
they do not depend on experimental C++26 library facilities.

Windows with MSYS2 GCC/G++ and the default Rust MSVC target:

```powershell
cargo build
New-Item -ItemType Directory -Force build
gcc -std=c23 -Wall -Wextra -Werror -I include examples/c23/main.c target/debug/cani.dll -o build/cani-c23.exe
g++ -std=c++26 -Wall -Wextra -Werror -I include examples/cpp26/main.cpp target/debug/cani.dll -o build/cani-cpp26.exe
Copy-Item target/debug/cani.dll build/cani.dll
./build/cani-c23.exe
./build/cani-cpp26.exe
```

Link the DLL directly with MinGW when Rust uses MSVC; the static `.lib` requires
a matching MSVC linker and native system libraries. On Linux, use
`-L target/debug -lcani -Wl,-rpath,'$ORIGIN/../target/debug'` instead of the DLL path.
Compilers that name draft C++26 mode `-std=c++2c` can use that spelling.

## Verification

```text
cargo test
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo build
python tests/upstream/tests/conformance.py --engine C:/SahilAppProjects/cani/target/debug/cani-usp.exe
```

Use your executable's absolute path on other systems. The vendored Apache-2.0
DaShobu suite checks golden push/capture cases, invalid moves and SFEN, seeded
games, arbitrary study positions, full legal lists, and transport framing against
an independent Python reference. Native examples contain ABI assertions and
should also be compiled and run as above. Upstream code attribution is in NOTICE.
