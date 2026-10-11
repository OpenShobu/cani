#include "cani.h"
#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
static uint8_t *read_file(const char *path,size_t *len) {
    FILE *f=fopen(path,"rb"); assert(f); assert(fseek(f,0,SEEK_END)==0);
    long n=ftell(f); assert(n>=0); rewind(f); *len=(size_t)n;
    uint8_t *bytes=malloc(*len+1); assert(bytes); assert(fread(bytes,1,*len,f)==*len); fclose(f); return bytes;
}
static void write_file(const char *path,const uint8_t *bytes,size_t len) {
    FILE *f=fopen(path,"wb"); assert(f); assert(fwrite(bytes,1,len,f)==len); assert(fclose(f)==0);
}
int main(void) {
    size_t len,needed,count; uint8_t *input=read_file("examples/ssgf/tournament.rssgf",&len);
    cani_ssgf_record *record=NULL; cani_ssgf_error error={0}; cani_ssgf_limits limits;
    assert(cani_ssgf_default_limits(&limits)==CANI_OK);
    assert(cani_ssgf_parse(input,len,&limits,&record,&error)==CANI_OK); free(input);
    assert(error.code==CANI_OK && error.turn_index==UINT64_MAX);
    assert(cani_ssgf_turn_count(record,&count)==CANI_OK && count==2);
    assert(cani_ssgf_metadata(record,NULL,0,&needed)==CANI_BUFFER);
    uint8_t *metadata=malloc(needed); assert(metadata);
    assert(cani_ssgf_metadata(record,metadata,needed,&needed)==CANI_OK);
    assert(strstr((const char *)metadata,"山田 太郎")); assert(strstr((const char *)metadata,"\"system\":\"elo\"")); free(metadata);
    cani_turn turn; assert(cani_ssgf_turn(record,0,&turn)==CANI_OK && turn.passive.board==0);
    cani_turn sentinel=turn;
    assert(cani_ssgf_turn(record,2,&turn)==CANI_SSGF_INDEX && memcmp(&turn,&sentinel,sizeof turn)==0);
    cani_state_data initial,final,at;
    assert(cani_ssgf_position(record,0,&initial)==CANI_OK && initial.ply==0);
    assert(cani_ssgf_final_position(record,&final)==CANI_OK && final.ply==2);
    assert(cani_ssgf_position(record,2,&at)==CANI_OK && memcmp(&at,&final,sizeof at)==0);
    for(uint32_t encoding=0;encoding<2;++encoding) {
        assert(cani_ssgf_write(record,encoding,NULL,NULL,0,&needed,&error)==CANI_BUFFER);
        uint8_t *output=malloc(needed); assert(output); memset(output,0xa5,needed);
        size_t size=needed;
        assert(cani_ssgf_write(record,encoding,NULL,output,size-1,&needed,&error)==CANI_BUFFER);
        for(size_t i=0;i<size;++i) assert(output[i]==0xa5);
        assert(cani_ssgf_write(record,encoding,NULL,output,size,&needed,&error)==CANI_OK);
        cani_ssgf_record *copy=NULL; assert(cani_ssgf_parse(output,size,NULL,&copy,&error)==CANI_OK);
        assert(cani_ssgf_final_position(copy,&at)==CANI_OK && memcmp(&at,&final,sizeof at)==0);
        write_file(encoding ? "build/native/from-c.bssgf" : "build/native/from-c.rssgf",output,size);
        free(output); cani_ssgf_free(copy);
    }
    cani_ssgf_record *saved=record;
    input=read_file("tests/fixtures/ssgf/invalid-illegal.rssgf",&len);
    assert(cani_ssgf_parse(input,len,NULL,&record,&error)==CANI_SSGF_ILLEGAL_TURN && error.turn_index==0 && record==saved); free(input);
    input=read_file("tests/fixtures/ssgf/invalid-duplicate.rssgf",&len);
    assert(cani_ssgf_parse(input,len,NULL,&record,&error)==CANI_SSGF_JSON && record==saved); free(input);
    limits.turns=0; needed=123;
    assert(cani_ssgf_write(record,CANI_BSSGF,&limits,NULL,0,&needed,&error)==CANI_SSGF_LIMIT && needed==123);
    cani_ssgf_free(record);
    input=read_file("tests/fixtures/ssgf/opening.bssgf",&len); record=NULL;
    assert(cani_ssgf_parse(input,len,NULL,&record,&error)==CANI_OK);
    assert(cani_ssgf_write(record,CANI_BSSGF,NULL,NULL,0,&needed,&error)==CANI_BUFFER && needed==len);
    uint8_t *output=malloc(needed); assert(output);
    assert(cani_ssgf_write(record,CANI_BSSGF,NULL,output,needed,&needed,&error)==CANI_OK && memcmp(input,output,len)==0);
    free(input); free(output); cani_ssgf_free(record);
    puts("PASS C23 SSGF codecs, replay, metadata, errors and atomic buffers");
}
