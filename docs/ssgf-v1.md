# Shobu Saved Game Format (SSGF), version 1

SSGF is cani's independently defined saved-game format. It records one main line
of Shobu play, descriptive metadata, comments, and the declared match result.
It is separate from USP 1.0 and does not add protocol commands. The words MUST,
MUST NOT, and MAY express requirements. The two encodings represent the same
logical record: readable **RSSGF** (`.rssgf`) and binary **BSSGF** (`.bssgf`).

## Common model

Every record MUST contain version `1`, rules profile `usp-game-1.0`, a result,
and an ordered sequence of complete turns. Coordinates, complete turns, state
validity, pushes, and adjudication use
[DaShobu's game profile](https://github.com/DaShobu/Universal-Shobu-Protocol-Specifications/blob/main/docs/game.md).

JSON MUST be UTF-8, without a BOM. Object keys MUST be unique at every depth,
including inside extensions; escaped equivalents count as the same key. Unknown
fields MUST be rejected outside `extensions`. Numbers MUST be finite. Numeric
integer fields MUST use JSON integers, not floating-point strings or values.
Strings retain Unicode without normalization. Optional scalar fields and optional
player/event objects MAY be omitted or null. Optional arrays and `extensions`
MAY be omitted, but MUST have their declared type when present (not null).
Writers omit absent optional fields and empty default arrays/objects.

The common metadata object has these fields:

| Field | Type / meaning |
| --- | --- |
| `version` | Required integer `1` |
| `rules` | Required string `usp-game-1.0` |
| `result` | Required result object, defined below |
| `start_sfen` | Optional canonical SFEN; omission/null means standard setup |
| `players` | Optional object with optional `black` and `white` player objects |
| `event` | Optional event object |
| `game_id` | Optional string identifier |
| `timestamp` | Optional RFC3339-style date/time with `Z` or numeric timezone |
| `source_url` | Optional descriptive URL string; no retrieval or verification |
| `comment` | Optional game comment string |
| `comments` | Optional ordered array of `{ "turn": integer, "text": string }` |
| `extensions` | Optional object of arbitrary JSON values for additional metadata |

A comment's `turn` is zero-based and MUST refer to an existing turn. Its text
describes the position after that turn. Multiple comments at the same index and
comments in any index order are allowed; array order MUST be preserved.

Player objects support optional string fields `name`, `username`, `id`, `country`,
and `team`; optional `kind` is `human` or `bot`. Optional `ratings` is an array of
objects with required nonempty string `system`, required finite numeric `value`,
and optional string `provider`. Elo uses `system: "elo"`. Other system names are
permitted. Ratings are assertions, not independently verified credentials.

Event objects support optional string fields `name` (tournament name), `id`,
`edition`, `round`, `table`, `location`, and `organizer`. Numeric-looking labels
such as round or table are still strings, allowing values such as `final` or `A`.

Timestamp syntax is `YYYY-MM-DD[Tt]HH:MM:SS[.digits](Z|z|+HH:MM|-HH:MM)`.
The date MUST exist in the proleptic Gregorian calendar; hours are 00–23 and
minutes/seconds 00–59. Offsets allow hours 00–23 and minutes 00–59. Fractional
seconds require at least one digit. Leap-second `60` and timezone names are not
supported by v1. Source URLs, names, countries and other labels are descriptive
strings; v1 does not impose registry membership or URL reachability.

### Results and replay

The result object contains exactly `outcome` and `reason`:

| Reason | Permitted outcome | Validation |
| --- | --- | --- |
| `rules` | `b`, `w` | MUST match the final rules-engine winner |
| `resignation`, `timeout` | `b`, `w` | Final rules outcome MUST still be ongoing |
| `agreement` | `draw` | Final rules outcome MUST still be ongoing |
| `adjudication` | `b`, `w`, `draw` | Final rules outcome MUST still be ongoing |
| `unfinished`, `abandonment` | `ongoing` | Final rules outcome MUST still be ongoing |
| `counter_limit` | `ongoing` | Ongoing final rules outcome and ply 2147483647 |

External outcomes are declarations; SFEN cannot prove them. An established rules
win MUST use reason `rules`, including an initially terminal study position.

The starting SFEN MUST be structurally valid but need not be reachable from the
opening. Every turn MUST be replayed through complete-turn validation. No moves
may follow termination or the counter limit. Invalid syntax, packed geometry, or
an illegal turn MUST be reported with its zero-based turn index. Results and
comments MUST be validated before publishing a record. Position index 0 is the
initial state; position index N is the state after N complete turns. External
results do not alter board states or the rules engine's `Outcome`.

## RSSGF

The root JSON object contains the common metadata, required
`"format": "RSSGF"`, and required `turns`, an array of existing 11-character USP
turn strings. Turns MUST contain both passive and aggressive moves.

```json
{
  "format": "RSSGF",
  "result": {"outcome": "ongoing", "reason": "unfinished"},
  "rules": "usp-game-1.0",
  "turns": ["0a1a2:1b1b2", "2a4a3:3b4b3"],
  "version": 1
}
```

Readers accept normal JSON whitespace and key ordering. Writers MUST sort object
keys recursively in Unicode scalar order (equivalent to UTF-8 byte order), use
two-space indentation, and terminate the document with LF. Array order is retained.
Ordinary equivalent JSON number spellings are accepted; ratings are finite
double-precision values. Byte-identical input reproduction is not required.

## BSSGF

All multibyte integer fields are little-endian. The record consists of:

| Offset | Size | Contents |
| --- | --- | --- |
| 0 | 8 | Magic bytes `42 53 53 47 46 0d 0a 1a` (`BSSGF`, CR, LF, SUB) |
| 8 | 2 | Binary version, unsigned integer `1` |
| 10 | 2 | Flags, unsigned integer `0` |
| 12 | 4 | Metadata byte length M |
| 16 | 4 | Complete-turn count N |
| 20 | M | UTF-8 JSON common metadata with `"format":"BSSGF"` |
| 20+M | 2*N | Packed turns in play order |

Metadata MUST NOT contain `turns`; both header and metadata versions MUST be 1.
Writers use compact, recursively sorted JSON without a newline. Readers accept
JSON whitespace within the declared metadata length. The total file length MUST
be exactly `20 + M + 2*N`. Nonzero flags, unsupported versions, truncation,
overflowing lengths and trailing bytes MUST be rejected. There is no checksum,
compression, or final-position payload; replay reconstructs every position.

Each turn is one little-endian unsigned 16-bit integer:

| Bits | Meaning |
| --- | --- |
| 0–1 | Passive board ID 0–3 |
| 2–3 | Aggressive board ID 0–3 |
| 4–7 | Passive source square 0–15 |
| 8–11 | Aggressive source square 0–15 |
| 12–14 | Direction 0–7 |
| 15 | Distance minus one (0 = one step; 1 = two steps) |

Square indices use `(rank-1)*4 + file`, with a1 = 0. Direction codes are
N, NE, E, SE, S, SW, W, NW in that order. N increases rank and E increases file,
always from Black's seat. Both destinations are reconstructed using the same
direction and distance. Off-board destinations MUST be rejected before replay.
Ownership, homeboard, opposite board color and pushes are checked during replay.
Encoding geometry alone does not establish legality.

Golden examples: `0a1a2:1b1b2` packs as `0x0104` (bytes `04 01`);
`2a4a3:3b4b3` packs as `0x4dce` (bytes `ce 4d`). The complete opening fixture has
106 metadata bytes, one turn, and 128 total bytes in
`tests/fixtures/ssgf/opening.bssgf`.

## Limits, APIs, and versioning

Default cani limits are 64 MiB total bytes, 1 MiB metadata, and 1,000,000 turns;
callers MAY configure different limits. BSSGF metadata size is the actual M bytes.
RSSGF metadata size is compact sorted JSON of the original object after removing
`turns`, including `format`, before optional-field normalization. Limits bound
accepted data; JSON decoding may also reject excessive nesting using the parser's
recursion limit. A limit violation never publishes a partial record.

The Rust module is `cani::ssgf`, enabled by Cargo feature `ssgf`. The default rules
engine remains dependency-free. `GameRecord::new` validates supplied metadata and
turns; records are immutable. `parse` auto-detects BSSGF by magic, otherwise parses
RSSGF; `write` validates and serializes either encoding. `replay` yields initial
and subsequent positions. Native position indexing replays from the start and is
O(index); final position access is O(1).

C APIs require `CANI_WITH_SSGF` and a library built with `--features ssgf`.
Handles are owned and immutable. Parse failures preserve the handle output;
buffer failures preserve byte output. Record serialization reports raw byte
counts WITHOUT NUL for both formats; metadata JSON reports a count INCLUDING NUL.
Common metadata JSON excludes `format` and `turns`. C/C++ callers can construct
records by parsing RSSGF; the native API does not impose a separate JSON library.

Incompatible fields, packing, replay conventions or mandatory semantics require
a new major version. Additional metadata belongs in `extensions`; readers retain
it when converting between encodings. V1 has no branches, move clocks, evaluation
scores, neural policy targets, authentication, or changes to USP commands.
