# Recorder, symmetry, and neural data APIs

These utilities are independent of USP sessions. Symmetry and neural data are
available in dependency-free builds. The recorder requires Cargo feature `ssgf`
and `CANI_WITH_SSGF` for native headers. SSGF v1 bytes and semantics are unchanged.

## Game recorder

Rust `ssgf::GameBuilder::new(initial, metadata)` takes a validated `State`.
It replaces the supplied starting SFEN and result with the actual starting
position and its automatic result; all other metadata is validated. The standard
start omits SFEN. New recorders have no turns, so per-turn comments cannot be
supplied until turns exist. `from_record` copies an existing validated record,
including its declaration, metadata, and history.

`append(Turn)` validates against the current position before changing history.
It automatically records a rules win, a counter-limit stop, or ongoing unfinished
play. `comment(index, text)` attaches an ordered comment after an existing turn.
Multiple comments are allowed. `undo()` returns the last turn, restores the
previous position, removes comments on that turn, and replaces any declared
result with the restored position's automatic result. Undo on empty history fails.

`declare_result(GameResult)` uses SSGF v1 validation. External declarations close
append until an undo or a valid unfinished declaration reopens the record.
Rules wins cannot be overridden. `set_metadata` atomically validates replacement
metadata against the entire history and forbids changing the starting position.
It can also explicitly change a declaration subject to SSGF result validation.
`metadata`, `turns`, and `position` expose immutable inspection.

`record()` creates an independent immutable snapshot; `write()` exports either
SSGF encoding with the existing caller-configurable writer limits. Mutating or
freeing a builder does not affect snapshots. Builders retain a position per turn
for constant-time current-position access and undo. Snapshot validation/export
is linear in history length and allocates. Builders are caller-managed histories;
writer/parser limits do not impose a builder history limit.

C uses an owned `cani_ssgf_builder` handle and explicit operations. Metadata
replacement accepts common metadata JSON without `format` or `turns`, rejecting
duplicate keys and unknown fields. Inspect metadata/turns through a snapshot;
export snapshots using `cani_ssgf_write`. C++ `GameBuilder` owns the same handle,
with move semantics, typed `RecordOutcome`/`Termination`, and exception reporting.
All failed operations preserve builder state and outputs except error details.
No concurrent mutation/access of a builder is permitted.

## Symmetry v1

`Symmetry::new(id)` validates an ID in 0..31. Each ID combines:

| Bits | Transformation |
| --- | --- |
| 0..1 | Number of counterclockwise quarter turns on each square grid |
| 2 | Reflect files (`x -> 3-x`) before rotation |
| 3 | Swap board colors globally (`board -> board XOR 1`) |
| 4 | Swap homeboard pairs (`board -> board XOR 2`) and relabel Black/White |

Coordinates are from Black's absolute seat; x increases toward d and y toward
rank 4. A quarter turn maps `(x,y)` to `(3-y,x)`. All four boards use the same
square transformation. Bit 4 also relabels side to move and winner; ply is retained.
The two board bits combine by XOR. These 32 transformations preserve position
validation, legal complete turns, pushes, captures, and outcomes. They are rules
automorphisms; they need not preserve the standard opening layout.

`position`, `turn`, `board`, `square`, `player`, and `inverse` provide Rust mappings.
A turn mapping validates its typed identifiers, not legality in a position.
Native state/turn mappings reject invalid identifiers and states atomically.
C++ `Symmetry` provides position/turn mappings and inversion. Independent rotations
of boards, arbitrary board permutations, and uncoupled player swaps are excluded
because they need not preserve the complete-turn rules. Transformed position keys
must be recalculated; callers must map cached actions back using the inverse.

## Neural encoding v1

Rust `neural::PositionTensor` contains version `1`, `cells: [u8;129]`, and `ply`.
The first 128 cells are binary occupancy planes, shape `[4,2,4,4]` in row-major
order: board, player (Black then White), rank (1 through 4), file (a through d).
Equivalently `cells[board*32 + player*16 + square]` is 0 or 1. Cell 128 is the side
scalar: 0 Black, 1 White. Coordinates and player channels remain absolute; no
side-relative normalization is applied. Callers can convert these bytes to their
model's preferred numeric tensor type. Ply is separate, allowing exact state
round trips and counter-dependent decisions without treating it as occupancy.

`encode` and `decode` allocate nothing. Decode rejects unsupported versions,
nonbinary cells, overlapping stones, excess piece counts, contradictory winners,
and excessive ply. Terminal and structurally valid study positions are supported.
C `cani_tensor` has `uint32_t version, ply`, `uint8_t cells[129], reserved[3]`;
reserved bytes must be zero. Its ABI size is 140 bytes; it is not a portable file
serialization. C++ `PositionTensor` wraps that representation.

`ActionIndex(u16)` v1 has 65,536 possible indices, exactly the SSGF v1 turn packing:
passive board bits 0..1, aggressive board 2..3, passive source 4..7, aggressive
source 8..11, direction 12..14, and distance minus one in bit 15. Directions are
N, NE, E, SE, S, SW, W, NW, with N increasing rank. Both destinations are derived
from the same displacement. The index is the numeric packed word regardless of
host byte order. `0a1a2:1b1b2` is index `0x0104` and `2a4a3:3b4b3` is `0x4dce`.

There are 19,520 geometrically decodable indices. Others fail decoding because a
destination is off-board. Geometry does not prove legality, including homeboard,
board color, stone ownership, or pushes. Build a policy mask by encoding the
position's legal typed turns; do not enable all decodable actions. Null/pass turns
have no encoding. Encoding/decoding does not commit moves or invoke selectors.
C/C++ provide matching `cani_action_*` / `ActionIndex` operations.

These definitions are fixed for v1. Future incompatible layouts/action mappings
require a new encoding version and distinct APIs; version 1 meanings must remain
unchanged. No model, training targets, search algorithm, or evaluation policy is
part of these SDK interfaces.

## Canonical positions v1

`symmetry::canonicalize(State)` returns `CanonicalPosition { position,
to_canonical }`. The representative is the lexicographically smallest tuple of
numeric u16 masks `(board0.black, board0.white, ..., board3.white)`, followed by
the numeric side (Black 0, White 1), across all 32 symmetries. Hashes, machine byte
order, SFEN formatting, and ply do not determine the choice. Equal tuples choose
the smallest symmetry ID. Ply is always preserved. This ordering is version 1
and must not change silently. An already canonical state maps through identity.

The mapping goes from input to representative. Use its inverse to restore turns
from a cache, and recalculate keys on the representative. When mapping bit 4
swaps players, Black-relative evaluations must change sign; side-to-move-relative
evaluations preserve their perspective. Keep full position comparisons for cache
hits and account for ply/counter behavior. Canonicalization never changes model
inputs implicitly. C exposes `cani_state_canonical`; C++ exposes `canonicalize`.

## Compact policy masks and sparse actions

`neural::ActionMask::for_position(&state)` returns an 8192-byte mask over the
existing 65,536 version-1 actions: bit `i % 8` in byte `i / 8` represents action i.
Its `contains`, `count`, and `as_bytes` inspect it. Only legal actions are set;
terminal and counter-limit positions have all-zero masks. This is eight times
smaller than one byte per action. The initial mask has 232 set bits, with initial
bytes `12 12` in hexadecimal. No invalid packed geometries are enabled.

For callers that need a smaller list, `legal_actions_into` fills an `ActionIndex`
slice in existing USP legal-turn order. Counts/capacities are elements; a 1024-item
buffer always suffices (2048 bytes). Undersized buffers are entirely unchanged,
and successful calls leave the unused tail unchanged. C provides separate mask
(byte capacities, always 8192 required) and sparse (u16 element capacities) APIs.
C++ provides `ActionMask` and `legal_actions_into` with spans. Exceptions may
allocate on C++ error paths; core and native value operations allocate no heap.

## Batch tensor encoding

`neural::encode_batch_into(&[State], &mut [PositionTensor])` writes the existing
v1 tensor layout into caller storage. Counts/capacities are tensors. Insufficient
capacity preserves every output; successful calls preserve the unused tail. Empty
batches succeed without touching output. There is no batch-owned heap or runtime.

C `cani_neural_encode_batch` checks all position values before an actual write,
so even an invalid middle item preserves every output. An undersized-buffer query
reports the required tensor count without inspecting element values. Null arrays
are permitted for count zero. Live, aligned, sized, nonoverlapping arrays and a
writable needed pointer are required. C++ has native-data span and typed Position/
PositionTensor span overloads; the native-data overload makes one batch FFI call,
while the typed overload avoids layout/aliasing assumptions and uses scalar calls.

## Bounded perft diagnostics

`diagnostics::perft(&state, depth, node_limit)` returns visited `nodes` (including
the root) and `leaves` at exactly the requested depth. Depth zero counts one leaf,
even for terminal or counter-limit positions. A branch that ends before the
requested depth contributes no leaves. Work is bounded by the mandatory node
budget and maximum depth 64; depth and node-limit errors publish no partial
statistics. Budget zero rejects even depth zero. The input state is unchanged.
The implementation uses bounded recursion without heap allocation or a cache.

C/C++ expose the same counts and limits. Native failures preserve the output
statistics, with `CANI_NODE_LIMIT` and `CANI_DEPTH_LIMIT` statuses. Independent
Python-reference opening fixtures are 232 depth-1 leaves and 50,508 depth-2 leaves
(50,741 visited nodes). These are diagnostics, with no playing policy or search
configuration imposed on a bot.

## Footprint and measurement

All utilities above are in the default dependency-free build. State remains a
24-byte Copy value, existing neural/SSGF/USP meanings remain unchanged, and all
Rust/native hot paths are covered by an isolated allocation-counting test.
Square permutations now use a 128-byte constant table and visit only occupied bits.

`cargo run --release --example sdk` measures generation, canonicalization, masks,
sparse actions, 32-position encoding and perft. The optional `compact` profile
uses size optimization, thin LTO, one codegen unit and stripped symbols:
`cargo build --profile compact --lib --bin cani-usp`. It changes distribution
artifacts, not dependency settings in a downstream Rust consumer's project.
Compare the same targets/features/toolchain when measuring size or throughput.
Crate version remains 0.1.0 during this work; no v1.0.0 release tag is implied.

On the Windows MSVC development build (default core features), the previous
release DLL/executable were 171,520/169,984 bytes. With these utilities the
ordinary release DLL/executable are 178,176/169,984 bytes; the new optional compact
profile produces 158,720/153,600 bytes. Thus the extra APIs add 6,656 bytes to the
ordinary DLL, while compact artifacts are smaller than the previous baseline.
These are toolchain-specific measurements, not universal size guarantees. The
compact profile can trade throughput for footprint; benchmark the consuming bot.
