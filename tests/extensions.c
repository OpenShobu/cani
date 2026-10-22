#include "cani.h"
#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
_Static_assert(sizeof(cani_tensor)==140,"tensor ABI");
int main(void) {
    cani_state_data initial, transformed, restored, child, mapped_child;
    assert(cani_state_init(&initial)==CANI_OK);
    cani_tensor tensor;
    assert(cani_neural_encode(&initial,&tensor)==CANI_OK);
    assert(tensor.version==1 && tensor.ply==0 && tensor.cells[128]==0);
    for(size_t b=0;b<4;b++) for(size_t q=0;q<16;q++) {
        assert(tensor.cells[b*32+q]==(q<4));
        assert(tensor.cells[b*32+16+q]==(q>=12));
    }
    assert(cani_neural_decode(&tensor,&restored)==CANI_OK);
    assert(memcmp(&restored,&initial,sizeof initial)==0);
    tensor.cells[0]=2;
    assert(cani_neural_decode(&tensor,&restored)==CANI_INVALID);
    assert(memcmp(&restored,&initial,sizeof initial)==0);
    cani_turn turn, mapped, decoded;
    assert(cani_turn_parse((const uint8_t *)"0a1a2:1b1b2",11,&turn)==CANI_OK);
    uint16_t action=0;
    assert(cani_action_encode(&turn,&action)==CANI_OK && action==0x0104);
    assert(cani_action_decode(action,&decoded)==CANI_OK && memcmp(&decoded,&turn,sizeof turn)==0);
    unsigned count=0;
    for(unsigned i=0;i<65536;i++) if(cani_action_decode((uint16_t)i,&decoded)==CANI_OK) {
        assert(cani_action_encode(&decoded,&action)==CANI_OK && action==i);count++;
    }
    assert(count==19520);
    assert(cani_state_played(&initial,&turn,&child)==CANI_OK);
    for(uint8_t id=0;id<32;id++) {
        uint8_t inverse;
        assert(cani_symmetry_inverse(id,&inverse)==CANI_OK);
        assert(cani_state_transform(&initial,id,&transformed)==CANI_OK);
        assert(cani_state_transform(&transformed,inverse,&restored)==CANI_OK);
        assert(memcmp(&restored,&initial,sizeof initial)==0);
        assert(cani_turn_transform(&turn,id,&mapped)==CANI_OK);
        assert(cani_state_played(&transformed,&mapped,&restored)==CANI_OK);
        assert(cani_state_transform(&child,id,&mapped_child)==CANI_OK);
        assert(memcmp(&restored,&mapped_child,sizeof restored)==0);
    }
#ifdef CANI_WITH_SSGF
    cani_ssgf_builder *builder=NULL; cani_ssgf_record *record=NULL; cani_ssgf_error error;
    assert(cani_ssgf_builder_new(&initial,&builder)==CANI_OK);
    assert(cani_ssgf_builder_undo(builder,&decoded,&error)==CANI_SSGF_INDEX);
    assert(cani_ssgf_builder_append(builder,&turn,&error)==CANI_OK);
    assert(cani_ssgf_builder_comment(builder,0,(const uint8_t *)"development",11,&error)==CANI_OK);
    assert(cani_ssgf_builder_comment(builder,1,(const uint8_t *)"bad",3,&error)==CANI_SSGF_INDEX && error.turn_index==1);
    const char *bad="{\"version\":1,\"version\":1}";
    assert(cani_ssgf_builder_metadata(builder,(const uint8_t *)bad,strlen(bad),&error)==CANI_SSGF_JSON);
    assert(cani_ssgf_builder_record(builder,&record,&error)==CANI_OK);
    size_t needed=0;
    assert(cani_ssgf_write(record,CANI_BSSGF,NULL,NULL,0,&needed,&error)==CANI_BUFFER);
    uint8_t *bytes=malloc(needed);assert(bytes);
    assert(cani_ssgf_write(record,CANI_BSSGF,NULL,bytes,needed,&needed,&error)==CANI_OK);
    FILE *file=fopen("build/native/builder-c.bssgf","wb");assert(file);
    assert(fwrite(bytes,1,needed,file)==needed);assert(fclose(file)==0);free(bytes);
    assert(cani_ssgf_builder_result(builder,CANI_SSGF_WHITE,CANI_SSGF_RESIGNATION,&error)==CANI_OK);
    assert(cani_ssgf_builder_append(builder,&turn,&error)==CANI_SSGF_RESULT);
    assert(cani_ssgf_builder_undo(builder,&decoded,&error)==CANI_OK && memcmp(&decoded,&turn,sizeof turn)==0);
    assert(cani_ssgf_builder_position(builder,&restored)==CANI_OK && memcmp(&restored,&initial,sizeof initial)==0);
    cani_ssgf_builder_free(builder);
    size_t count_turns=0;assert(cani_ssgf_turn_count(record,&count_turns)==CANI_OK && count_turns==1);
    assert(cani_ssgf_builder_from_record(record,&builder)==CANI_OK);
    assert(cani_ssgf_builder_position(builder,&restored)==CANI_OK && restored.ply==1);
    cani_ssgf_builder_free(builder);cani_ssgf_free(record);
#endif
    puts("C recorder/symmetry/neural checks passed");
}
