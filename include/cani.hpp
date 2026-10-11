#ifndef CANI_HPP
#define CANI_HPP
#include "cani.h"
#include <memory>
#include <stdexcept>
#include <string>
#include <string_view>
#include <vector>

namespace cani {
class Position {
    std::unique_ptr<cani_position, decltype(&cani_position_free)> handle_{cani_position_new(), cani_position_free};
    static void check(int32_t code) {
        if (code != CANI_OK) throw std::invalid_argument(code == CANI_ILLEGAL ? "illegal turn" : "invalid position or turn");
    }
    using Writer = int32_t (*)(const cani_position *, uint8_t *, size_t, size_t *);
    std::string text(Writer writer) const {
        size_t size = 0;
        if (writer(handle_.get(), nullptr, 0, &size) != CANI_BUFFER) throw std::runtime_error("invalid handle");
        std::vector<uint8_t> buffer(size);
        check(writer(handle_.get(), buffer.data(), buffer.size(), &size));
        return {reinterpret_cast<const char *>(buffer.data()), size - 1};
    }
public:
    Position() = default;
    explicit Position(std::string_view sfen) { parse(sfen); }
    Position(const Position &other) : Position(other.sfen()) {}
    Position &operator=(const Position &other) { if (this != &other) parse(other.sfen()); return *this; }
    Position(Position &&) noexcept = default;
    Position &operator=(Position &&) noexcept = default;
    void parse(std::string_view sfen) { check(cani_position_parse(handle_.get(), reinterpret_cast<const uint8_t *>(sfen.data()), sfen.size())); }
    void play(std::string_view turn) { check(cani_position_play(handle_.get(), reinterpret_cast<const uint8_t *>(turn.data()), turn.size())); }
    std::string sfen() const { return text(cani_position_sfen); }
    std::vector<std::string> legal() const {
        auto moves = text(cani_position_legal);
        std::vector<std::string> result;
        for (size_t pos = 0; pos < moves.size(); pos += 12) result.push_back(moves.substr(pos, 11));
        return result;
    }
    int32_t result() const { return cani_position_result(handle_.get()); }
};
class Engine {
    std::unique_ptr<cani_engine, decltype(&cani_engine_free)> handle_{cani_engine_new(), cani_engine_free};
public:
    std::string handle(std::string_view request) {
        std::vector<uint8_t> output(12294);
        size_t needed = 0;
        auto code = cani_engine_handle(handle_.get(), reinterpret_cast<const uint8_t *>(request.data()), request.size(), output.data(), output.size(), &needed);
        if (code != CANI_OK) throw std::invalid_argument("invalid request");
        return {reinterpret_cast<const char *>(output.data()), needed - 1};
    }
};
}
#endif
