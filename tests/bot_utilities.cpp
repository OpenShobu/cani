#include "cani.hpp"
#include <cassert>
#include <iostream>
int main() {
    cani::Position state;auto turn=cani::Turn::parse("0a1a2:1b1b2");auto child=state.played(turn);
    auto canonical=cani::canonicalize(child);assert(canonical.to_canonical.inverse().position(canonical.position).sfen()==child.sfen());
    for(unsigned id=0;id<32;id++) assert(cani::canonicalize(cani::Symmetry(id).position(child)).position.sfen()==canonical.position.sfen());
    auto mask=cani::ActionMask::for_position(state);assert(mask.count()==232&&mask.bytes[0]==0x12&&mask.bytes[1]==0x12);
    std::array<cani::ActionIndex,1024> actions{};assert(cani::legal_actions_into(state,actions)==232);
    for(size_t i=0;i<232;i++) {assert(mask.contains(actions[i]));(void)state.played(actions[i].decode());}
    std::array<cani::ActionIndex,1> small{{{65535}}};
    try {(void)cani::legal_actions_into(state,small);assert(false);}catch(const cani::BufferTooSmall&e){assert(e.required==232);}assert(small[0].value==65535);
    std::array<cani::Position,2> positions{state,child};std::array<cani::PositionTensor,3> tensors{};tensors[2].data.version=99;
    try {(void)cani::encode_batch_into(std::span<const cani::Position>(positions),std::span<cani::PositionTensor>(tensors.data(),1));assert(false);}catch(const cani::BufferTooSmall&e){assert(e.required==2);}assert(tensors[0].data.version==0);
    assert(cani::encode_batch_into(std::span<const cani::Position>(positions),std::span<cani::PositionTensor>(tensors))==2);assert(tensors[1].decode().sfen()==child.sfen()&&tensors[2].data.version==99);
    std::array<cani_state_data,2> native{state.native(),child.native()};std::array<cani_tensor,2> output{};
    assert(cani::encode_batch_into(std::span<const cani_state_data>(native),std::span<cani_tensor>(output))==2);assert(output[1].ply==1);
    auto stats=cani::perft(state,2,50741);assert(stats.leaves==50508&&stats.nodes==50741);
    try {(void)cani::perft(state,2,1);assert(false);}catch(const cani::Error&e){assert(e.code()==CANI_NODE_LIMIT);}
    std::cout<<"PASS C++ canonical positions, masks, batches and perft\n";
}
