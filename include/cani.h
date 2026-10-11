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
enum { CANI_OK=0, CANI_INVALID=1, CANI_ILLEGAL=2, CANI_BUFFER=3 };
cani_position *cani_position_new(void);
void cani_position_free(cani_position *position);
int32_t cani_position_parse(cani_position *, const uint8_t *, size_t);
int32_t cani_position_play(cani_position *, const uint8_t *, size_t);
int32_t cani_position_sfen(const cani_position *, uint8_t *, size_t, size_t *);
int32_t cani_position_legal(const cani_position *, uint8_t *, size_t, size_t *);
/* 0 ongoing, 1 Black wins, 2 White wins, -1 null handle. */
int32_t cani_position_result(const cani_position *);
cani_engine *cani_engine_new(void);
void cani_engine_free(cani_engine *);
/* One request without newline. Protocol errors are response text.
 * No transport/quit lifecycle is imposed on this in-process session. */
int32_t cani_engine_handle(cani_engine *, const uint8_t *, size_t, uint8_t *, size_t, size_t *);
#ifdef __cplusplus
}
#endif
#endif
