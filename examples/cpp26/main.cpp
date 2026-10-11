#include "cani.hpp"
#include <cassert>
#include <iostream>
int main() {
    cani::Position position;
    assert(position.legal().size() == 232);
    auto copy = position;
    copy.play("0a1a2:1b1b2");
    assert(copy.sfen() != position.sfen());
    try { position.play("0a1a2:2a1a2"); assert(false); }
    catch (const std::invalid_argument &) {}
    assert(position.legal().size() == 232);
    cani::Engine engine;
    assert(engine.handle("hello 1") == "ok usp 1");
    assert(engine.handle("go") == "bestmove 0a1a2:1a1a2");
    std::cout << copy.sfen() << '\n';
}
