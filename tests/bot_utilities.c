#include "cani.h"
#include <assert.h>
#include <stdio.h>
#include <string.h>
_Static_assert(sizeof(cani_perft_stats)==16,"perft ABI");
int main(void) {
    cani_state_data state;assert(cani_state_init(&state)==CANI_OK);
    cani_turn turn;assert(cani_turn_parse((const uint8_t *)"0a1a2:1b1b2",11,&turn)==CANI_OK);
    cani_state_data child,canonical;assert(cani_state_played(&state,&turn,&child)==CANI_OK);uint8_t mapping;
    assert(cani_state_canonical(&child,&canonical,&mapping)==CANI_OK);
    for(uint8_t id=0;id<32;id++) {
        cani_state_data changed,other,restored;uint8_t to,inverse;
        assert(cani_state_transform(&child,id,&changed)==CANI_OK);
        assert(cani_state_canonical(&changed,&other,&to)==CANI_OK);assert(memcmp(&other,&canonical,sizeof other)==0);
        assert(cani_symmetry_inverse(to,&inverse)==CANI_OK);assert(cani_state_transform(&other,inverse,&restored)==CANI_OK);assert(memcmp(&restored,&changed,sizeof changed)==0);
    }
    uint8_t bytes[CANI_ACTION_MASK_BYTES+1];memset(bytes,0xa5,sizeof bytes);size_t needed=0;
    assert(cani_state_action_mask(&state,bytes,CANI_ACTION_MASK_BYTES-1,&needed)==CANI_BUFFER && needed==CANI_ACTION_MASK_BYTES);
    for(size_t i=0;i<sizeof bytes;i++) assert(bytes[i]==0xa5);
    assert(cani_state_action_mask(&state,bytes,sizeof bytes,&needed)==CANI_OK);assert(bytes[0]==0x12&&bytes[1]==0x12&&bytes[CANI_ACTION_MASK_BYTES]==0xa5);
    uint16_t actions[CANI_MAX_LEGAL_TURNS];for(size_t i=0;i<CANI_MAX_LEGAL_TURNS;i++) actions[i]=0xffff;
    assert(cani_state_legal_actions(&state,actions,231,&needed)==CANI_BUFFER&&needed==232);assert(actions[0]==0xffff);
    assert(cani_state_legal_actions(&state,actions,CANI_MAX_LEGAL_TURNS,&needed)==CANI_OK&&needed==232);
    for(size_t i=0;i<needed;i++){assert((bytes[actions[i]/8]&(1u<<(actions[i]%8)))!=0);cani_turn t;cani_state_data out;assert(cani_action_decode(actions[i],&t)==CANI_OK);assert(cani_state_played(&state,&t,&out)==CANI_OK);}
    assert(actions[232]==0xffff);
    cani_state_data inputs[2]={state,child};cani_tensor tensors[3],sentinel;memset(&sentinel,0xa5,sizeof sentinel);for(size_t i=0;i<3;i++) tensors[i]=sentinel;
    assert(cani_neural_encode_batch(inputs,2,tensors,1,&needed)==CANI_BUFFER&&needed==2);assert(memcmp(&tensors[0],&sentinel,sizeof sentinel)==0);
    inputs[1].reserved[0]=1;assert(cani_neural_encode_batch(inputs,2,tensors,3,&needed)==CANI_INVALID);for(size_t i=0;i<3;i++) assert(memcmp(&tensors[i],&sentinel,sizeof sentinel)==0);
    inputs[1]=child;assert(cani_neural_encode_batch(inputs,2,tensors,3,&needed)==CANI_OK&&needed==2);assert(tensors[0].version==1&&tensors[1].ply==1);assert(memcmp(&tensors[2],&sentinel,sizeof sentinel)==0);
    assert(cani_neural_encode_batch(NULL,0,NULL,0,&needed)==CANI_OK&&needed==0);
    cani_perft_stats stats={123,456};assert(cani_state_perft(&state,2,1,&stats)==CANI_NODE_LIMIT&&stats.nodes==123&&stats.leaves==456);
    assert(cani_state_perft(&state,65,1,&stats)==CANI_DEPTH_LIMIT&&stats.nodes==123);
    assert(cani_state_perft(&state,2,50741,&stats)==CANI_OK&&stats.nodes==50741&&stats.leaves==50508);
    puts("PASS C canonical positions, masks, batches and perft");
}
