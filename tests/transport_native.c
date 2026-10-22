/* Stream fault injection: errors are distinct from clean EOF and quit. */
#include <assert.h>
#include <stdio.h>
#include <string.h>
static const char *input;
static int input_error, output_error, flush_error, write_error;
static int injected_getchar(void) {return *input ? (unsigned char)*input++ : EOF;}
static int injected_fflush(FILE *f) {(void)f;return flush_error ? EOF : 0;}
static int injected_ferror(FILE *f) {return f==stdin ? input_error : output_error;}
static size_t injected_fwrite(const void *p,size_t size,size_t n,FILE *f) {(void)p;(void)size;(void)f;return write_error ? 0 : n;}
static int injected_putchar(int c) {return write_error ? EOF : c;}
#define getchar injected_getchar
#define fflush injected_fflush
#define ferror injected_ferror
#define fwrite injected_fwrite
#define putchar injected_putchar
#include "../examples/bots/transport.h"
static int32_t respond(void *context,const uint8_t *request,size_t len,uint8_t *out,size_t cap,size_t *needed) {
    (void)context;(void)request;(void)len;assert(cap>=7);memcpy(out,"ok bye",7);*needed=7;return CANI_OK;
}
int main(void) {
    input="quit\n";flush_error=1;assert(example_transport(respond,NULL)==1);
    flush_error=0;input="quit\n";output_error=1;assert(example_transport(respond,NULL)==1);
    output_error=0;input="quit\n";write_error=1;assert(example_transport(respond,NULL)==1);
    write_error=0;input="";input_error=1;assert(example_transport(respond,NULL)==1);
    input="partial";assert(example_transport(respond,NULL)==1);
    input_error=0;input="partial";assert(example_transport(respond,NULL)==0);
    input="quit\n";assert(example_transport(respond,NULL)==0);
    puts("PASS native stream error, EOF and quit handling");
}
