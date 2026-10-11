#ifndef CANI_EXAMPLE_TRANSPORT_H
#define CANI_EXAMPLE_TRANSPORT_H
/* Example-only shared USP transport. Game policy remains in each bot program. */
#include "cani.h"
#include <stdio.h>
#include <string.h>
#ifdef _WIN32
#include <fcntl.h>
#include <io.h>
#endif
typedef int32_t (*example_responder)(void *,const uint8_t *,size_t,uint8_t *,size_t,size_t *);
static int example_transport(example_responder respond,void *context) {
#ifdef _WIN32
    if (_setmode(_fileno(stdin),_O_BINARY)==-1 || _setmode(_fileno(stdout),_O_BINARY)==-1) return 1;
#endif
    uint8_t input[4097],output[12294]; size_t len=0; int overflow=0,c;
    while((c=getchar())!=EOF) {
        if(c!='\n') { if(len<sizeof input) input[len++]=(uint8_t)c; else overflow=1; continue; }
        if(len>0 && input[len-1]=='\r') --len;
        int syntax=overflow || len>4096 || len==0;
        for(size_t i=0;i<len;++i) if(input[i]<32 || input[i]>126) syntax=1;
        size_t needed=0;
        if(syntax) { memcpy(output,"error syntax",13); needed=13; }
        else {
            int32_t status=respond(context,input,len,output,sizeof output,&needed);
            if(status!=CANI_OK) { fprintf(stderr,"bot request failed: %d\n",(int)status); return 1; }
        }
        if(needed==0 || needed>sizeof output || fwrite(output,1,needed-1,stdout)!=needed-1 || putchar('\n')==EOF || fflush(stdout)==EOF) return 1;
        if(strcmp((const char *)output,"ok bye")==0) return 0;
        len=0; overflow=0;
    }
    /* EOF discards an incomplete final request, including oversized ones. */
    return ferror(stdin) ? 1 : 0;
}
#endif
