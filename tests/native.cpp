#include "cani.hpp"
#include <cassert>
#include <iostream>
int main() {
    cani::Position state;
    assert(state.position_key().value==UINT64_C(0xcf87b888d934b52e));
    assert(state.state_key().value==UINT64_C(0x5afa352fa525047c));
    auto turn=cani::Turn::parse("0a1a2:1b1b2");
    assert(turn.str()=="0a1a2:1b1b2");
    std::array<cani::Turn,1024> moves{};
    assert(state.legal_turns_into(moves)==232);
    auto sentinel=moves[0]; std::array<cani::Turn,1> small{sentinel};
    try { state.legal_turns_into(small); assert(false); }
    catch(const cani::BufferTooSmall &e) { assert(e.required==232); }
    assert(small[0]==sentinel);
    auto copy=state;
    auto undo=copy.apply(turn);
    assert(copy.sfen()==state.played(turn).sfen());
    assert(copy.position_key().value==UINT64_C(0x89762bf94edc2c16));
    assert(copy.state_key().value==UINT64_C(0xeb73baf62b36d8d9));
    copy.undo(undo); assert(copy.sfen()==state.sfen());
    assert(copy.stone(cani::BoardId(0),cani::Square(0))==1);
    int calls=0;
    cani::Engine engine([&](const cani::Position &p,std::span<const cani::Turn> turns) {
        ++calls; assert(p.position_key()==state.position_key()); return turns.back();
    });
    assert(engine.handle("hello 1")=="ok usp 1");
    assert(engine.handle("go")=="bestmove "+moves[231].str());
    assert(calls==1);
    // Callback context is rebound when moving a session.
    auto moved=std::move(engine);
    assert(moved.handle("go")=="bestmove "+moves[231].str());
    assert(calls==2);
    moved.set_selector([](const cani::Position &,std::span<const cani::Turn>) { return cani::Turn{}; });
    try { moved.handle("go"); assert(false); }
    catch(const cani::Error &e) { assert(e.code()==CANI_SELECTOR); }
    moved.set_selector({});
    assert(moved.handle("go")=="bestmove 0a1a2:1a1a2");
    std::cout << "PASS C++26 typed facade, direct copying, undo, keys, selectors\n";
}
