#include "cani.hpp"
#include "transport.h"
#include <bit>
#include <cstring>
#include <limits>
class Bot {
    struct Entry { bool valid=false; uint64_t key=0; cani_state_data state{}; int black_score=0; };
    std::array<Entry,64> cache_{};
    int evaluate(const cani::Position &position) {
        auto key=position.position_key().value; auto data=position.native(); auto &entry=cache_[key & 63];
        if(entry.valid && entry.key==key && std::memcmp(&entry.state,&data,sizeof data)==0) return entry.black_score;
        int score=0;
        switch(position.outcome()) {
            case cani::Outcome::BlackWins: score=10000; break;
            case cani::Outcome::WhiteWins: score=-10000; break;
            case cani::Outcome::Ongoing:
                for(const auto &b:data.boards) score+=std::popcount(b.black)-std::popcount(b.white);
                break;
        }
        entry={true,key,data,score}; return score;
    }
public:
    cani::Turn select(const cani::Position &position,std::span<const cani::Turn> legal) {
        auto best=legal.front(); int score=std::numeric_limits<int>::min();
        const int sign=position.side_to_move()==cani::Player::Black ? 1 : -1;
        for(auto turn:legal) { int value=sign*evaluate(position.played(turn)); if(value>score) { score=value; best=turn; } }
        return best;
    }
};
static int32_t respond(void *context,const uint8_t *line,size_t len,uint8_t *out,size_t cap,size_t *needed) {
    try {
        auto reply=static_cast<cani::Engine *>(context)->handle(std::string_view(reinterpret_cast<const char *>(line),len));
        *needed=reply.size()+1; if(cap<*needed) return CANI_BUFFER;
        std::memcpy(out,reply.c_str(),*needed); return CANI_OK;
    } catch(const cani::Error &e) { return e.code(); } catch(...) { return CANI_INVALID; }
}
int main() {
    Bot bot;
    cani::Engine engine([&](const cani::Position &p,std::span<const cani::Turn> legal) { return bot.select(p,legal); });
    return example_transport(respond,&engine);
}
