# External bot integration examples

These runnable programs implement the same deterministic, one-ply policy outside
the SDK. The engine provides legal complete turns to a selector; the bot evaluates
immutable child positions. Rules wins/losses score +/-10000 from Black's
perspective, ongoing positions score Black minus White stone count, and selection
inverts the score for White. Equal scores retain the first legal turn.

Each bot owns a 64-slot direct-mapped evaluation cache, indexed by position key.
A hit requires both the key AND full position equality, preventing hash collisions
or omitted ply values from being mistaken for equality. Repeating `go` preserves
the game position. This is an integration example, not a competitive search engine.

Rust:

```text
cargo run --example bot
```

C23 and C++26 (using your existing native toolchain):

```text
cargo build
python scripts/check_native.py
build/native/bot-c
build/native/bot-cpp
```

Use `.exe` on Windows. The check script compiles both bots but does not run them
interactively. Native examples share `transport.h` for bounded USP framing,
LF/CRLF, binary-mode Windows streams, flushing, EOF and quit. They use the existing
C callback registration or C++ selector facade. The Rust bot uses `MoveSelector`
and `run_with_selector`. Transport diagnostics are confined to stderr.

Example session:

```text
hello 1
position startpos
go
state
quit
```

Check all policies against the independent reference rules:

```text
cargo build --example bot
python scripts/test_bots.py --engine target/debug/examples/bot --engine build/native/bot-c --engine build/native/bot-cpp
```

Use `.exe` paths on Windows. The suite checks both colors, winning captures,
deterministic ties, terminal/counter-limit bypass, repeated go, state preservation,
malformed framing, EOF and quit. SDK modules contain no example bot policy.
