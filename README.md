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

Positions own eight `u16` bitboards, side, and ply in a copyable 24-byte value,
and enforce their invariants through private fields. `stone(board, square)` inspects stones; `side_to_move` and `ply` inspect
turn metadata. `play` returns a new position, preserving its input on success or
failure. `legal` returns sorted complete-turn encodings. `winner` returns a player
or `None`; `result` returns the USP outcome string. Parsing accepts structurally
valid study positions without asserting historical reachability.

From Black's seat, boards 0/1 are Black's homeboards and 2/3 White's. Even boards
are dark; odd boards are light. Square indices are `(rank - 1) * 4 + file`, with
a1 = 0. White uses the same absolute coordinates. Each complete turn contains a
passive and aggressive leg, e.g. `0a1a2:1b1b2`.

## Rules and protocol

### Typed bot APIs

`BoardId::new` and `Square::new` validate identifiers. `Move` contains a board,
source, and destination; `Turn` pairs passive and aggressive moves. They implement
`FromStr` and `Display`; structural validity is separate from position legality.

```rust
use cani::{State, Turn, MAX_LEGAL_TURNS};
let mut state = State::default();
let mut turns = [Turn::default(); MAX_LEGAL_TURNS];
let count = state.legal_turns_into(&mut turns)?;
let before = state;
let undo = state.apply(turns[0])?;
state.undo(undo)?;
assert_eq!(state, before);
# Ok::<(), Box<dyn std::error::Error>>(())
```

`played(Turn)` is the immutable typed transition; `apply`/`undo` are mutable.
Undo tokens contain before/after values and must be used in reverse order.
An out-of-order token is rejected without changing state. Rust tokens are consumed
on attempted undo. Tokens are not an internal history stack.

`legal_turns_into` leaves undersized buffers untouched and returns
`BufferTooSmall { required }`. `visit_legal_turns` takes a `ControlFlow` callback
and supports early exit. Both yield sorted, unique complete turns without heap
allocation; 1024 entries always suffice. `Turn::default()` is an invalid null turn
suitable for buffer initialization, not an instruction to pass.

`outcome()` returns `Outcome::Ongoing` or `Outcome::Win(Player)`. Typed transitions
return `MoveError` variants for geometry, wrong board/stone, blockage, termination,
or the ply limit. `parse_sfen` and `from_bitboards` expose `ValidationError`.
`Board` has public Black/White masks; constructing a `State` checks overlap,
per-board counts, contradictory winners, and the counter limit. `board`/`at` provide
typed inspection. Existing string methods retain their original error categories.

Generation, typed transitions, outcomes, undo, and keys allocate nothing. SFEN,
string adapters, and protocol response formatting can allocate.

`position_key()` is a deterministic Zobrist v1 hash of boards and side, excluding
ply; `state_key()` additionally includes ply. Keys can collide and must not be used
as an equality proof. A position key alone does not capture counter-limit behavior.
Constants use SplitMix64 with seed `0x43414e495f4b4559`, board-major, player-major,
a1-through-d4 square order, followed by the White-to-move and ply-salt constants.
State-key hashing XORs the position key with the SplitMix64 finalizer of
`ply_salt ^ ply`. Keys are recomputed rather than cached.

The initial position has position key `0xcf87b888d934b52e` and state key
`0x5afa352fa525047c`; fixtures are shared with the native tests.

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
first legal turn without committing it. Bot implementations supply search or
neural evaluation through the selector interface.
Bot authors can provide `Engine::with_selector` or `run_with_selector`, using
the `MoveSelector` trait or a closure:

```rust
use cani::{Engine, State, Turn};
let mut session = Engine::with_selector(|position: &State, legal: &[Turn]| {
    // Replace this policy with your bot's search/evaluation.
    legal[legal.len() - 1]
});
session.handle_checked("hello 1")?;
let response = session.handle_checked("go")?;
# Ok::<(), cani::SessionError>(())
```

Selectors receive a nonempty legal slice and an immutable position, and are skipped
at termination and the ply limit. The returned turn is checked for membership.
`handle_checked` reports `SessionError::Selection` for invalid choices without
changing the position; the legacy `handle` reports `"selector"`. Stream runners
terminate with an I/O error and no invented USP error response. Selectors may own
mutable search state; only the game position is protected from selection mutation.

## C23 and C++26

Build with `cargo build --release`. Include `include/cani.h` in C or
`include/cani.hpp` in C++. Both link to the same Rust implementation. No duplicate
rules engine or Rust runtime process is required.

The compatibility C interface uses opaque owned position/session handles, explicit byte lengths,
status codes, and caller-owned output buffers. Output sizes include the NUL byte.
A `CANI_BUFFER` response reports the required size without changing state or
output. Free each handle once; avoid concurrent mutations and overlapping buffers.
See the header for the complete contract.

For search, the C interface adds caller-owned `cani_state_data`, `cani_turn`, and
`cani_undo` values. Struct assignment copies positions directly. `cani_state_*`
functions expose inspection, typed generation, apply/undo, immutable transitions,
outcomes, and keys. New typed transitions return specific error status codes.
`cani_position_import`/`export` bridge opaque handles and values. Reserved bytes
must be zero and all value inputs are validated. Undo tokens must be unmodified;
they are checked for structural validity and a matching child, not authenticated.

The C++ facade provides typed `BoardId`, `Square`, `Move`, `Turn`, and `Outcome`;
`cani::Position` uses direct value copies. It adds `legal_turns_into(std::span<Turn>)`,
`played`, `apply`/`undo`, inspection, and keys. Undersized spans throw
`BufferTooSmall` with a `required` count. Existing text methods remain available.

`cani_engine_set_selector` registers a synchronous C callback with caller-owned
context; a null callback restores first-legal behavior. The callback receives
position data and legal turns and writes a selected turn. Failures return
`CANI_SELECTOR`, preserving game state and response output. It must not unwind,
retain borrowed pointers, or reenter the same engine. Context must outlive
registration. Capacity is checked before invoking callbacks, so queries/retries
do not trigger search. `go` needs 21 output bytes (14 when no move exists), including
NUL. C++ accepts a selector via its `Engine` constructor or `set_selector`;
exceptions from the callback become selector failures.

Invalid inputs throw C++ exceptions carrying C status codes.
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

## Saved games (SSGF v1)

Enable the optional `ssgf` feature for RSSGF (readable JSON) and BSSGF (JSON
metadata plus two bytes per complete turn). The default build retains a
dependency-free rules core. Records include player names/usernames, Elo or other
ratings, tournament/event metadata, comments, and rules/external match outcomes.
Every record is replay-validated, with configurable byte/turn limits and indexed
errors. The normative specification is [SSGF v1](docs/ssgf-v1.md).

```rust
use cani::ssgf::{Encoding, GameRecord, Limits};
let input = std::fs::read("examples/ssgf/tournament.rssgf")?;
let record = GameRecord::parse(&input, Limits::default())?;
let binary = record.write(Encoding::Binary, Limits::default())?;
for position in record.replay() { println!("{}", position.sfen()); }
# Ok::<(), Box<dyn std::error::Error>>(())
```

`GameMetadata`, `PlayerData`, `Rating`, `EventData`, `GameResult`, and `Comment`
provide typed Rust construction through `GameRecord::new`. Fields are immutable
after validation. C/C++ expose owned record handles, serialization, metadata JSON,
typed indexed turns/positions, and final positions. Define `CANI_WITH_SSGF` when
including native headers and link a library built with `--features ssgf`.

```text
cargo run --features ssgf --example ssgf -- examples/ssgf/tournament.rssgf build/tournament.bssgf bssgf
cargo run --features ssgf --example ssgf -- build/tournament.bssgf build/tournament.rssgf rssgf
```

See [external bot examples](examples/bots/README.md) for runnable Rust, C23, and
C++26 selectors using typed turns, child positions and collision-checked key caches.

## Verification

Feature-enabled checks:

```text
cargo test --features ssgf
cargo build --features ssgf --examples
cargo build --features ssgf
python scripts/check_native.py --ssgf
```

```text
cargo test
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo build
python scripts/check_native.py
python tests/upstream/tests/conformance.py --engine C:/SahilAppProjects/cani/target/debug/cani-usp.exe
```

Use your executable's absolute path on other systems. The vendored Apache-2.0
DaShobu suite checks golden push/capture cases, invalid moves and SFEN, seeded
games, arbitrary study positions, full legal lists, and transport framing against
an independent Python reference. Native examples contain ABI assertions and
should also be compiled and run as above. Upstream code attribution is in NOTICE.
The native check script compiles both ABI test programs and examples with warnings
as errors. Use `--cpp-standard c++2c` where needed, or `--library-dir target/release`.
