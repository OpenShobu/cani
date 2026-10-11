#ifndef CANI_HPP
#define CANI_HPP
#include "cani.h"
#include <array>
#include <functional>
#include <memory>
#include <span>
#include <stdexcept>
#include <string>
#include <string_view>
#include <utility>
#include <vector>

namespace cani {
class Error : public std::invalid_argument {
    int32_t code_;
public:
    explicit Error(int32_t code) : std::invalid_argument("cani status " + std::to_string(code)), code_(code) {}
    int32_t code() const noexcept { return code_; }
};
inline void check(int32_t code) { if (code != CANI_OK) throw Error(code); }
class BufferTooSmall : public Error {
public:
    const size_t required;
    explicit BufferTooSmall(size_t count) : Error(CANI_BUFFER), required(count) {}
};
enum class Player : uint8_t { Black = CANI_BLACK, White = CANI_WHITE };
enum class Outcome : int32_t { Ongoing = CANI_ONGOING, BlackWins = CANI_BLACK_WINS, WhiteWins = CANI_WHITE_WINS };
class BoardId {
    uint8_t id_;
public:
    explicit constexpr BoardId(unsigned id = 0) : id_(static_cast<uint8_t>(id)) { if (id >= 4) throw std::out_of_range("board"); }
    constexpr uint8_t index() const noexcept { return id_; }
    bool operator==(const BoardId &) const = default;
};
class Square {
    uint8_t index_;
public:
    explicit constexpr Square(unsigned index = 0) : index_(static_cast<uint8_t>(index)) { if (index >= 16) throw std::out_of_range("square"); }
    constexpr uint8_t index() const noexcept { return index_; }
    bool operator==(const Square &) const = default;
};
struct Move {
    BoardId board{};
    Square from{}, to{};
    bool operator==(const Move &) const = default;
};
struct Turn {
    Move passive{}, aggressive{};
    bool operator==(const Turn &) const = default;
    cani_turn native() const noexcept {
        return {{passive.board.index(), passive.from.index(), passive.to.index()},
                {aggressive.board.index(), aggressive.from.index(), aggressive.to.index()}};
    }
    static Turn from_native(cani_turn t) {
        return {{BoardId(t.passive.board), Square(t.passive.from), Square(t.passive.to)},
                {BoardId(t.aggressive.board), Square(t.aggressive.from), Square(t.aggressive.to)}};
    }
    static Turn parse(std::string_view text) {
        cani_turn turn{};
        check(cani_turn_parse(reinterpret_cast<const uint8_t *>(text.data()), text.size(), &turn));
        return from_native(turn);
    }
    std::string str() const {
        auto t = native(); std::array<uint8_t,12> out{}; size_t needed = 0;
        check(cani_turn_format(&t, out.data(), out.size(), &needed));
        return {reinterpret_cast<const char *>(out.data()), needed - 1};
    }
};
struct PositionKey { uint64_t value; bool operator==(const PositionKey &) const = default; };
struct StateKey { uint64_t value; bool operator==(const StateKey &) const = default; };
class Position;
class Undo {
    cani_undo data_{};
    explicit Undo(cani_undo data) : data_(data) {}
    friend class Position;
};
class Position {
    cani_state_data data_{};
public:
    Position() { check(cani_state_init(&data_)); }
    explicit Position(std::string_view text) { parse(text); }
    explicit Position(cani_state_data data) : data_(data) { check(cani_state_validate(&data_)); }
    // Implicit copy/move is a direct value copy, never an SFEN round trip.
    cani_state_data native() const noexcept { return data_; }
    Player side_to_move() const noexcept { return static_cast<Player>(data_.side); }
    uint32_t ply() const noexcept { return data_.ply; }
    cani_board_data board(BoardId board) const noexcept { return data_.boards[board.index()]; }
    // 0 empty, 1 Black, 2 White, matching the C ABI.
    int32_t stone(BoardId board, Square square) const {
        int32_t value; check(cani_state_stone(&data_,board.index(),square.index(),&value)); return value;
    }
    void parse(std::string_view text) {
        check(cani_state_parse(&data_,reinterpret_cast<const uint8_t *>(text.data()),text.size()));
    }
    std::string sfen() const {
        std::array<uint8_t,256> out{}; size_t needed = 0;
        check(cani_state_sfen(&data_,out.data(),out.size(),&needed));
        return {reinterpret_cast<const char *>(out.data()),needed-1};
    }
    Position played(Turn turn) const {
        auto t = turn.native(); cani_state_data next{};
        check(cani_state_played(&data_,&t,&next)); return Position(next);
    }
    Undo apply(Turn turn) {
        auto t = turn.native(); cani_undo undo{};
        check(cani_state_apply(&data_,&t,&undo)); return Undo(undo);
    }
    void undo(const Undo &undo) { check(cani_state_undo(&data_,&undo.data_)); }
    void play(std::string_view text) { (void)apply(Turn::parse(text)); }
    size_t legal_turns_into(std::span<Turn> output) const {
        std::array<cani_turn,CANI_MAX_LEGAL_TURNS> turns{}; size_t count = 0;
        // FFI preserves output on insufficient capacity, as does this facade.
        const auto cap = output.size() < turns.size() ? output.size() : turns.size();
        auto status = cani_state_legal(&data_,turns.data(),cap,&count);
        if (status == CANI_BUFFER) throw BufferTooSmall(count);
        check(status);
        for (size_t i=0; i<count; ++i) output[i] = Turn::from_native(turns[i]);
        return count;
    }
    std::vector<Turn> legal_turns() const {
        std::array<Turn,CANI_MAX_LEGAL_TURNS> turns{};
        auto count = legal_turns_into(turns);
        return {turns.begin(),turns.begin()+count};
    }
    std::vector<std::string> legal() const {
        auto turns = legal_turns(); std::vector<std::string> result; result.reserve(turns.size());
        for (const auto &t : turns) result.push_back(t.str());
        return result;
    }
    Outcome outcome() const {
        cani_outcome value; check(cani_state_outcome(&data_,&value)); return static_cast<Outcome>(value);
    }
    int32_t result() const { return static_cast<int32_t>(outcome()); }
    PositionKey position_key() const {
        cani_position_key p; cani_state_key s; check(cani_state_keys(&data_,&p,&s)); return {p};
    }
    StateKey state_key() const {
        cani_position_key p; cani_state_key s; check(cani_state_keys(&data_,&p,&s)); return {s};
    }
};
class Engine {
public:
    using Selector = std::function<Turn(const Position &, std::span<const Turn>)>;
private:
    std::unique_ptr<cani_engine, decltype(&cani_engine_free)> handle_{cani_engine_new(), cani_engine_free};
    Selector selector_;
    static int32_t bridge(void *context, const cani_state_data *state, const cani_turn *moves, size_t count, cani_turn *out) noexcept {
        try {
            auto &self = *static_cast<Engine *>(context);
            std::array<Turn,CANI_MAX_LEGAL_TURNS> legal{};
            for (size_t i=0; i<count; ++i) legal[i] = Turn::from_native(moves[i]);
            *out = self.selector_(Position(*state),std::span<const Turn>(legal.data(),count)).native();
            return CANI_OK;
        } catch (...) { return CANI_SELECTOR; }
    }
    void register_selector() {
        check(cani_engine_set_selector(handle_.get(),selector_ ? &bridge : nullptr,selector_ ? this : nullptr));
    }
public:
    Engine() = default;
    explicit Engine(Selector selector) { set_selector(std::move(selector)); }
    Engine(const Engine &) = delete;
    Engine &operator=(const Engine &) = delete;
    Engine(Engine &&other) : handle_(std::move(other.handle_)), selector_(std::move(other.selector_)) { register_selector(); }
    Engine &operator=(Engine &&other) {
        if (this != &other) { handle_ = std::move(other.handle_); selector_ = std::move(other.selector_); register_selector(); }
        return *this;
    }
    void set_selector(Selector selector) { selector_ = std::move(selector); register_selector(); }
    std::string handle(std::string_view request) {
        std::array<uint8_t,12294> output{}; size_t needed = 0;
        check(cani_engine_handle(handle_.get(),reinterpret_cast<const uint8_t *>(request.data()),request.size(),output.data(),output.size(),&needed));
        return {reinterpret_cast<const char *>(output.data()),needed-1};
    }
};
#ifdef CANI_WITH_SSGF
enum class SSGFEncoding : uint32_t { Readable=CANI_RSSGF, Binary=CANI_BSSGF };
inline cani_ssgf_limits default_record_limits() {
    cani_ssgf_limits limits{}; check(cani_ssgf_default_limits(&limits)); return limits;
}
class RecordError : public Error {
public:
    const uint64_t turn_index;
    RecordError(int32_t code,uint64_t index=UINT64_MAX) : Error(code), turn_index(index) {}
};
class GameRecord {
    std::unique_ptr<cani_ssgf_record,decltype(&cani_ssgf_free)> handle_{nullptr,cani_ssgf_free};
    explicit GameRecord(cani_ssgf_record *record) : handle_(record,cani_ssgf_free) {}
    static void checked(int32_t status,cani_ssgf_error error={0,0,UINT64_MAX}) {
        if(status!=CANI_OK) throw RecordError(status,error.turn_index);
    }
public:
    GameRecord(GameRecord &&) noexcept=default;
    GameRecord &operator=(GameRecord &&) noexcept=default;
    GameRecord(const GameRecord &)=delete;
    GameRecord &operator=(const GameRecord &)=delete;
    static GameRecord parse(std::span<const uint8_t> input,cani_ssgf_limits limits=default_record_limits()) {
        cani_ssgf_record *out=nullptr; cani_ssgf_error error{0,0,UINT64_MAX};
        auto status=cani_ssgf_parse(input.data(),input.size(),&limits,&out,&error);
        checked(status,error);
        return GameRecord(out);
    }
    static GameRecord parse(std::string_view input,cani_ssgf_limits limits=default_record_limits()) {
        return parse(std::span<const uint8_t>(reinterpret_cast<const uint8_t *>(input.data()),input.size()),limits);
    }
    std::vector<uint8_t> write(SSGFEncoding encoding,cani_ssgf_limits limits=default_record_limits()) const {
        size_t needed=0; cani_ssgf_error error{0,0,UINT64_MAX};
        auto status=cani_ssgf_write(handle_.get(),static_cast<uint32_t>(encoding),&limits,nullptr,0,&needed,&error);
        if(status!=CANI_BUFFER) checked(status,error);
        std::vector<uint8_t> output(needed);
        status=cani_ssgf_write(handle_.get(),static_cast<uint32_t>(encoding),&limits,output.data(),output.size(),&needed,&error);
        checked(status,error); return output;
    }
    std::string metadata_json() const {
        size_t needed=0; auto status=cani_ssgf_metadata(handle_.get(),nullptr,0,&needed);
        if(status!=CANI_BUFFER) checked(status);
        std::vector<uint8_t> output(needed);
        checked(cani_ssgf_metadata(handle_.get(),output.data(),output.size(),&needed));
        return {reinterpret_cast<const char *>(output.data()),needed-1};
    }
    size_t turn_count() const { size_t n=0; checked(cani_ssgf_turn_count(handle_.get(),&n)); return n; }
    Turn turn(size_t index) const { cani_turn turn{}; checked(cani_ssgf_turn(handle_.get(),index,&turn)); return Turn::from_native(turn); }
    Position position_at(size_t index) const { cani_state_data state{}; checked(cani_ssgf_position(handle_.get(),index,&state)); return Position(state); }
    Position final_position() const { cani_state_data state{}; checked(cani_ssgf_final_position(handle_.get(),&state)); return Position(state); }
};
#endif
}
#endif
