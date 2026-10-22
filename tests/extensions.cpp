#include "cani.hpp"
#include <cassert>
#include <fstream>
#include <iterator>
#include <iostream>
int main() {
    cani::Position initial;auto turn=cani::Turn::parse("0a1a2:1b1b2");
    auto tensor=cani::PositionTensor::encode(initial);assert(tensor.data.version==1);
    assert(tensor.decode().sfen()==initial.sfen());
    assert(cani::ActionIndex::encode(turn).value==0x0104);
    assert(cani::ActionIndex::encode(turn).decode()==turn);
    for(unsigned id=0;id<32;id++) {
        cani::Symmetry sym(id);auto transformed=sym.position(initial);
        assert(sym.inverse().position(transformed).sfen()==initial.sfen());
        assert(transformed.played(sym.turn(turn)).sfen()==sym.position(initial.played(turn)).sfen());
        assert(sym.inverse().turn(sym.turn(turn))==turn);
    }
    try {cani::Symmetry bad(32);assert(false);} catch(const std::out_of_range &) {}
    tensor.data.version=2;
    try {(void)tensor.decode();assert(false);} catch(const cani::Error &) {}
#ifdef CANI_WITH_SSGF
    cani::GameBuilder builder(initial);builder.append(turn);builder.comment(0,"development");
    auto record=builder.record();
    std::ifstream file("build/native/builder-c.bssgf",std::ios::binary);assert(file);
    std::vector<uint8_t> bytes{std::istreambuf_iterator<char>(file),std::istreambuf_iterator<char>()};
    assert(builder.write(cani::SSGFEncoding::Binary)==bytes);
    builder.set_metadata(record.metadata_json());
    try {builder.comment(1,"bad");assert(false);} catch(const cani::RecordError &e) {assert(e.turn_index==1);}
    builder.declare_result(cani::RecordOutcome::White,cani::Termination::Resignation);
    try {builder.append(turn);assert(false);} catch(const cani::RecordError &) {}
    assert(builder.undo()==turn);assert(builder.position().sfen()==initial.sfen());
    assert(record.turn_count()==1);
    cani::GameBuilder copied(record);auto moved=std::move(copied);assert(moved.position().ply()==1);
    assert(moved.record().write(cani::SSGFEncoding::Binary)==bytes);
#endif
    std::cout<<"C++ recorder/symmetry/neural checks passed\n";
}
