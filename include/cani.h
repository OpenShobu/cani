#ifndef CANI_H
#define CANI_H
#include <stddef.h>
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif
/* Opaque single-owner handles. No concurrent mutation or use after free.
 * Input is UTF-8 bytes plus length, without a terminator.
 * Text output includes NUL; needed includes NUL. BUFFER preserves output and
 * state. Query with out=NULL, cap=0, and a valid needed pointer, then retry.
 * All buffers and needed must be nonoverlapping and valid for their sizes.
 */
typedef struct cani_position cani_position;
typedef struct cani_engine cani_engine;
enum { CANI_OK=0, CANI_INVALID=1, CANI_ILLEGAL=2, CANI_BUFFER=3,
    CANI_SELECTOR=4, CANI_UNDO_MISMATCH=5,
    CANI_GEOMETRY=10, CANI_HOMEBOARD=11, CANI_BOARD_COLOR=12,
    CANI_WRONG_STONE=13, CANI_BLOCKED=14, CANI_TERMINAL=15,
    CANI_COUNTER_LIMIT=16 };
enum { CANI_MAX_LEGAL_TURNS=1024, CANI_BLACK=0, CANI_WHITE=1 };
typedef int32_t cani_outcome;
enum { CANI_ONGOING=0, CANI_BLACK_WINS=1, CANI_WHITE_WINS=2 };
typedef uint64_t cani_position_key;
typedef uint64_t cani_state_key;
typedef struct { uint16_t black, white; } cani_board_data;
/* side is CANI_BLACK/CANI_WHITE; reserved MUST be zero. Square a1 = 0.
 * Bitboards are disjoint, with at most four stones per color per board.
 * Assignment copies a position; use validate after manually editing study data.
 */
typedef struct {
    cani_board_data boards[4];
    uint32_t ply;
    uint8_t side, reserved[3];
} cani_state_data;
typedef struct { uint8_t board, from, to; } cani_move;
typedef struct { cani_move passive, aggressive; } cani_turn;
typedef struct { cani_state_data before, after; } cani_undo;

/* Value operations allocate no heap memory except SFEN/text formatting.
 * Failure preserves outputs (except needed) and state. All pointers must be
 * live, aligned and nonoverlapping, except played allows out == input.
 * Undo tokens must be retained unmodified and used in reverse order.
 */
int32_t cani_state_init(cani_state_data *);
int32_t cani_state_validate(const cani_state_data *);
int32_t cani_state_parse(cani_state_data *, const uint8_t *, size_t);
int32_t cani_state_sfen(const cani_state_data *, uint8_t *, size_t, size_t *);
/* Stone output: 0 empty, 1 Black, 2 White. */
int32_t cani_state_stone(const cani_state_data *, uint8_t board, uint8_t square, int32_t *);
/* needed is a TURN COUNT, not bytes. cap=0 permits out=NULL.
 * Order is USP ASCII lexicographic; a 1024-turn buffer always suffices. */
int32_t cani_state_legal(const cani_state_data *, cani_turn *, size_t cap, size_t *needed);
int32_t cani_state_played(const cani_state_data *, const cani_turn *, cani_state_data *);
int32_t cani_state_apply(cani_state_data *, const cani_turn *, cani_undo *);
int32_t cani_state_undo(cani_state_data *, const cani_undo *);
int32_t cani_state_outcome(const cani_state_data *, cani_outcome *);
/* Zobrist v1 hashes: position excludes ply, state includes it. Collisions
 * are possible; position keys alone do not encode the counter play limit. */
int32_t cani_state_keys(const cani_state_data *, cani_position_key *, cani_state_key *);
int32_t cani_turn_parse(const uint8_t *, size_t, cani_turn *);
int32_t cani_turn_format(const cani_turn *, uint8_t *, size_t, size_t *);
cani_position *cani_position_new(void);
void cani_position_free(cani_position *position);
int32_t cani_position_parse(cani_position *, const uint8_t *, size_t);
int32_t cani_position_play(cani_position *, const uint8_t *, size_t);
int32_t cani_position_sfen(const cani_position *, uint8_t *, size_t, size_t *);
int32_t cani_position_legal(const cani_position *, uint8_t *, size_t, size_t *);
/* 0 ongoing, 1 Black wins, 2 White wins, -1 null handle. */
int32_t cani_position_result(const cani_position *);
int32_t cani_position_export(const cani_position *, cani_state_data *);
int32_t cani_position_import(cani_position *, const cani_state_data *);
cani_engine *cani_engine_new(void);
void cani_engine_free(cani_engine *);
/* One request without newline. Protocol errors are response text.
 * No transport/quit lifecycle is imposed on this in-process session. */
int32_t cani_engine_handle(cani_engine *, const uint8_t *, size_t, uint8_t *, size_t, size_t *);
/* Called only for a nonempty legal list. Return CANI_OK and write a legal turn;
 * otherwise handle returns CANI_SELECTOR, preserving position and output.
 * Callbacks are synchronous and serialized. Context must outlive registration.
 * Do not retain borrowed pointers, unwind, or reenter/free the same session.
 * Output capacity is checked BEFORE the callback; queries never call it.
 */
typedef int32_t (*cani_selector)(void *, const cani_state_data *, const cani_turn *, size_t, cani_turn *);
/* NULL restores first-legal selection. No ownership of context is transferred. */
int32_t cani_engine_set_selector(cani_engine *, cani_selector, void *context);

/* Symmetry v1: IDs 0..31. Bits 0..1: counterclockwise square quarter turns;
 * bit 2: reflect files BEFORE rotation; bit 3: globally swap board colors;
 * bit 4: swap homeboards AND players/side. Ply is preserved.
 * All transformations act uniformly on squares across boards.
 * Pointers must be live, aligned, nonoverlapping; failures preserve outputs. */
int32_t cani_state_transform(const cani_state_data *, uint8_t symmetry, cani_state_data *);
int32_t cani_turn_transform(const cani_turn *, uint8_t symmetry, cani_turn *);
int32_t cani_symmetry_inverse(uint8_t symmetry, uint8_t *);
/* Neural v1: cells[board*32+player*16+square] occupancy, cells[128] side.
 * Absolute coordinates; 0/1 bytes, with ply separate. No normalization.
 * Action index is SSGF's 16-bit packing; invalid geometry returns GEOMETRY.
 * Decode does not establish position legality. Use legal turns for masks.
 * Reserved bytes MUST be zero. Version mismatch returns INVALID. */
enum { CANI_NEURAL_VERSION=1, CANI_TENSOR_LEN=129, CANI_ACTION_COUNT=65536 };
typedef struct { uint32_t version, ply; uint8_t cells[129], reserved[3]; } cani_tensor;
int32_t cani_neural_encode(const cani_state_data *, cani_tensor *);
int32_t cani_neural_decode(const cani_tensor *, cani_state_data *);
int32_t cani_action_encode(const cani_turn *, uint16_t *);
int32_t cani_action_decode(uint16_t, cani_turn *);

#ifdef CANI_WITH_SSGF
/* Available ONLY when the library is built with cargo --features ssgf.
 * Independent of USP. All buffers/handles must be live, sized, aligned and
 * nonoverlapping. Records are immutable. Each parsed handle has one owner.
 */
typedef struct cani_ssgf_record cani_ssgf_record;
typedef struct { uint64_t total_bytes, metadata_bytes, turns; } cani_ssgf_limits;
typedef struct { int32_t code; uint32_t reserved; uint64_t turn_index; } cani_ssgf_error;
enum { CANI_RSSGF=0, CANI_BSSGF=1, CANI_SSGF_JSON=30, CANI_SSGF_SCHEMA=31,
    CANI_SSGF_VERSION=32, CANI_SSGF_PROFILE=33, CANI_SSGF_LIMIT=34,
    CANI_SSGF_BINARY=35, CANI_SSGF_METADATA=36, CANI_SSGF_TURN=37,
    CANI_SSGF_ILLEGAL_TURN=38, CANI_SSGF_RESULT=39, CANI_SSGF_INDEX=40 };
/* NULL limits selects defaults. NULL error omits detail. An absent turn_index
 * is UINT64_MAX. Parse failure preserves *out; success transfers a handle and
 * clears error. Free once. Callers construct records by parsing RSSGF metadata
 * and moves; no C JSON dependency is imposed. */
/* Mutable recorder: separate from USP, exclusively owned, no concurrent access.
 * new uses default metadata and derives the starting SFEN/result from position.
 * from_record copies metadata and history. append validates a complete turn;
 * externally closed records reject append. undo clears a declaration and removes
 * comments on the removed turn. Metadata replacement uses common metadata JSON
 * (no format/turns), validates against history, and cannot change the start.
 * Operations are atomic on failure. Snapshot returns a separately owned record.
 * Export via cani_ssgf_write; snapshots remain valid after builder changes/free.
 * No callback or protocol command is involved. */
typedef struct cani_ssgf_builder cani_ssgf_builder;
enum { CANI_SSGF_ONGOING=0, CANI_SSGF_BLACK=1, CANI_SSGF_WHITE=2, CANI_SSGF_DRAW=3 };
enum { CANI_SSGF_RULES=0, CANI_SSGF_RESIGNATION=1, CANI_SSGF_TIMEOUT=2,
    CANI_SSGF_AGREEMENT=3, CANI_SSGF_ADJUDICATION=4, CANI_SSGF_ABANDONMENT=5,
    CANI_SSGF_UNFINISHED=6, CANI_SSGF_COUNTER_LIMIT=7 };
int32_t cani_ssgf_builder_new(const cani_state_data *, cani_ssgf_builder **);
int32_t cani_ssgf_builder_from_record(const cani_ssgf_record *, cani_ssgf_builder **);
void cani_ssgf_builder_free(cani_ssgf_builder *);
int32_t cani_ssgf_builder_append(cani_ssgf_builder *, const cani_turn *, cani_ssgf_error *);
int32_t cani_ssgf_builder_undo(cani_ssgf_builder *, cani_turn *, cani_ssgf_error *);
int32_t cani_ssgf_builder_comment(cani_ssgf_builder *, size_t, const uint8_t *, size_t, cani_ssgf_error *);
int32_t cani_ssgf_builder_metadata(cani_ssgf_builder *, const uint8_t *, size_t, cani_ssgf_error *);
int32_t cani_ssgf_builder_result(cani_ssgf_builder *, uint32_t outcome, uint32_t reason, cani_ssgf_error *);
int32_t cani_ssgf_builder_record(const cani_ssgf_builder *, cani_ssgf_record **, cani_ssgf_error *);
int32_t cani_ssgf_builder_position(const cani_ssgf_builder *, cani_state_data *);
int32_t cani_ssgf_default_limits(cani_ssgf_limits *);
int32_t cani_ssgf_parse(const uint8_t *, size_t, const cani_ssgf_limits *, cani_ssgf_record **out, cani_ssgf_error *);
void cani_ssgf_free(cani_ssgf_record *);
/* write needed is raw BYTE COUNT without NUL, for BOTH encodings. Metadata
 * needed includes NUL. Query with out=NULL, cap=0. BUFFER preserves output.
 * On record errors, write fills error and preserves output/needed.
 */
int32_t cani_ssgf_write(const cani_ssgf_record *, uint32_t encoding, const cani_ssgf_limits *, uint8_t *, size_t, size_t *, cani_ssgf_error *);
int32_t cani_ssgf_metadata(const cani_ssgf_record *, uint8_t *, size_t, size_t *);
int32_t cani_ssgf_turn_count(const cani_ssgf_record *, size_t *);
int32_t cani_ssgf_turn(const cani_ssgf_record *, size_t index, cani_turn *);
/* position index 0 is initial; turn_count is the final position. */
int32_t cani_ssgf_position(const cani_ssgf_record *, size_t index, cani_state_data *);
int32_t cani_ssgf_final_position(const cani_ssgf_record *, cani_state_data *);
#endif
#ifdef __cplusplus
}
#endif
#endif
