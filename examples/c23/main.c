#include "cani.h"
#include <assert.h>
#include <stdio.h>
#include <string.h>
int main(void) {
    cani_position *position = cani_position_new();
    assert(cani_position_result(position) == 0);
    const char *turn = "0a1a2:1b1b2";
    assert(cani_position_play(position, (const uint8_t *)turn, strlen(turn)) == CANI_OK);
    uint8_t output[256];
    size_t needed = 0;
    assert(cani_position_sfen(position, output, sizeof output, &needed) == CANI_OK);
    assert(strcmp((const char *)output, "WWWW/4/B3/1BBB|WWWW/4/1B2/B1BB|WWWW/4/4/BBBB|WWWW/4/4/BBBB w 1") == 0);
    puts((const char *)output);
    cani_position_free(position);
    cani_engine *engine = cani_engine_new();
    assert(cani_engine_handle(engine, (const uint8_t *)"hello 1", 7, output, sizeof output, &needed) == CANI_OK);
    assert(strcmp((const char *)output, "ok usp 1") == 0);
    cani_engine_free(engine);
}
