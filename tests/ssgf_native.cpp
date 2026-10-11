#include "cani.hpp"
#include <cassert>
#include <fstream>
#include <iterator>
#include <iostream>
static std::vector<uint8_t> read_file(const char *path) {
    std::ifstream file(path,std::ios::binary); assert(file);
    return {std::istreambuf_iterator<char>(file),std::istreambuf_iterator<char>()};
}
int main() {
    auto bytes=read_file("build/native/from-c.bssgf");
    auto record=cani::GameRecord::parse(std::span<const uint8_t>(bytes));
    assert(record.turn_count()==2 && record.final_position().ply()==2);
    assert(record.position_at(0).ply()==0);
    assert(record.turn(0).str()=="0a1a2:1b1b2");
    assert(record.metadata_json().find("山田 太郎")!=std::string::npos);
    assert(record.write(cani::SSGFEncoding::Binary)==bytes);
    auto readable=record.write(cani::SSGFEncoding::Readable);
    auto c_readable=read_file("build/native/from-c.rssgf"); assert(readable==c_readable);
    auto second=cani::GameRecord::parse(std::span<const uint8_t>(readable));
    assert(second.final_position().sfen()==record.final_position().sfen());
    auto moved=std::move(second); assert(moved.turn_count()==2);
    std::ofstream output("build/native/from-cpp.bssgf",std::ios::binary); assert(output);
    output.write(reinterpret_cast<const char *>(bytes.data()),bytes.size()); output.close(); assert(output);
    auto bad=read_file("tests/fixtures/ssgf/invalid-illegal.rssgf");
    try { auto invalid=cani::GameRecord::parse(std::span<const uint8_t>(bad)); assert(false); }
    catch(const cani::RecordError &e) { assert(e.code()==CANI_SSGF_ILLEGAL_TURN && e.turn_index==0); }
    try { record.position_at(3); assert(false); } catch(const cani::RecordError &e) { assert(e.code()==CANI_SSGF_INDEX); }
    std::cout << "PASS C++26 SSGF facade and C/C++ file interoperability\n";
}
