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
