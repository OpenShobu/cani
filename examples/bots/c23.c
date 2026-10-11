#include "transport.h"
#include <limits.h>
typedef struct { int valid; uint64_t key; cani_state_data state; int black_score; } entry;
typedef struct { entry cache[64]; } bot;
static int stones(uint16_t bits) { int n=0; while(bits) { bits=(uint16_t)(bits & (uint16_t)(bits-1)); ++n; } return n; }
static int32_t evaluate(bot *self,const cani_state_data *state,int *score) {
    cani_position_key key; cani_state_key unused;
    int32_t status=cani_state_keys(state,&key,&unused); if(status!=CANI_OK) return status;
    entry *cached=&self->cache[key & 63];
    if(cached->valid && cached->key==key && memcmp(&cached->state,state,sizeof *state)==0) { *score=cached->black_score; return CANI_OK; }
    cani_outcome result; status=cani_state_outcome(state,&result); if(status!=CANI_OK) return status;
    *score=result==CANI_BLACK_WINS ? 10000 : result==CANI_WHITE_WINS ? -10000 : 0;
    if(result==CANI_ONGOING) for(size_t b=0;b<4;++b) *score+=stones(state->boards[b].black)-stones(state->boards[b].white);
    *cached=(entry){1,key,*state,*score}; return CANI_OK;
}
static int32_t select_turn(void *context,const cani_state_data *state,const cani_turn *legal,size_t count,cani_turn *out) {
    bot *self=context; int best=INT_MIN; int sign=state->side==CANI_BLACK ? 1 : -1;
    for(size_t i=0;i<count;++i) {
        cani_state_data child; int score;
        int32_t status=cani_state_played(state,&legal[i],&child); if(status!=CANI_OK) return status;
        status=evaluate(self,&child,&score); if(status!=CANI_OK) return status;
        score*=sign; if(score>best) { best=score; *out=legal[i]; }
    }
    return count ? CANI_OK : CANI_SELECTOR;
}
static int32_t respond(void *engine,const uint8_t *line,size_t len,uint8_t *out,size_t cap,size_t *needed) {
    return cani_engine_handle(engine,line,len,out,cap,needed);
}
int main(void) {
    bot self={0}; cani_engine *engine=cani_engine_new();
    if(cani_engine_set_selector(engine,select_turn,&self)!=CANI_OK) { cani_engine_free(engine); return 1; }
    int status=example_transport(respond,engine); cani_engine_free(engine); return status;
}
