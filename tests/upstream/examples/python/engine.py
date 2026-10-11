"""USP 1.0 reference engine; Python 3.10+, standard library only."""
import re
import sys
from dataclasses import dataclass

MAX_PLY = 2147483647
INITIAL = '|'.join(['WWWW/4/4/BBBB'] * 4) + ' b 0'
MOVE = re.compile(r'[0-3][a-d][1-4][a-d][1-4]:[0-3][a-d][1-4][a-d][1-4]', re.ASCII)


@dataclass
class State:
    boards: list
    side: str
    ply: int

    def copy(self):
        return State([b[:] for b in self.boards], self.side, self.ply)


def parse_sfen(text):
    if not re.fullmatch(r'[BW1-4/|]+ [bw] (0|[1-9][0-9]{0,9})', text, re.ASCII):
        raise ValueError('sfen')
    layout, side, counter = text.split(' ')
    if int(counter) > MAX_PLY:
        raise ValueError('sfen')
    encoded = layout.split('|')
    if len(encoded) != 4:
        raise ValueError('sfen')
    boards = []
    for board in encoded:
        ranks = board.split('/')
        if len(ranks) != 4:
            raise ValueError('sfen')
        squares = []
        for rank in reversed(ranks):
            row, digit = [], False
            for c in rank:
                if c in '1234':
                    if digit:
                        raise ValueError('sfen')
                    row.extend('.' * int(c))
                    digit = True
                else:
                    row.append(c)
                    digit = False
            if len(row) != 4:
                raise ValueError('sfen')
            squares.extend(row)
        if squares.count('B') > 4 or squares.count('W') > 4:
            raise ValueError('sfen')
        boards.append(squares)
    if any('B' not in b for b in boards) and any('W' not in b for b in boards):
        raise ValueError('sfen')
    return State(boards, side, int(counter))


def sfen(state):
    boards = []
    for board in state.boards:
        ranks = []
        for y in range(3, -1, -1):
            row, empty = '', 0
            for c in board[y*4:y*4+4]:
                if c == '.':
                    empty += 1
                else:
                    if empty:
                        row += str(empty)
                    row += c
                    empty = 0
            if empty:
                row += str(empty)
            ranks.append(row)
        boards.append('/'.join(ranks))
    return '|'.join(boards) + f' {state.side} {state.ply}'


def decode(move):
    if not MOVE.fullmatch(move):
        raise ValueError('syntax')
    def leg(t):
        return int(t[0]), ord(t[1])-97, int(t[2])-1, ord(t[3])-97, int(t[4])-1
    return leg(move[:5]), leg(move[6:])


def empty_winner(state):
    if any('B' not in b for b in state.boards):
        return 'w'
    if any('W' not in b for b in state.boards):
        return 'b'
    return None


def apply_raw(state, move):
    """Validate geometry and both legs; return a new state, never mutate input."""
    p, a = decode(move)
    pb, x, y, tx, ty = p
    ab, ax, ay, atx, aty = a
    dx, dy = tx-x, ty-y
    n = max(abs(dx), abs(dy))
    if (pb//2 != (0 if state.side == 'b' else 1) or pb%2 == ab%2
            or n not in (1, 2) or abs(dx) not in (0, n) or abs(dy) not in (0, n)
            or (atx-ax, aty-ay) != (dx, dy)):
        raise ValueError('illegal')
    ux, uy = dx//n, dy//n
    own = state.side.upper()
    out = state.copy()
    for board_id, sx, sy, aggressive in [(pb, x, y, False), (ab, ax, ay, True)]:
        board = out.boards[board_id]
        if board[sy*4+sx] != own:
            raise ValueError('illegal')
        pushed = None
        for _ in range(n):
            nx, ny = sx+ux, sy+uy
            if not (0 <= nx < 4 and 0 <= ny < 4):
                raise ValueError('illegal')
            target = ny*4+nx
            if board[target] != '.':
                if not aggressive or board[target] == own or (pushed is not None and pushed != target):
                    raise ValueError('illegal')
                px, py = nx+ux, ny+uy
                next_push = py*4+px if 0 <= px < 4 and 0 <= py < 4 else -1
                if next_push >= 0:
                    if board[next_push] != '.':
                        raise ValueError('illegal')
                    board[next_push] = board[target]
                pushed = next_push
            board[sy*4+sx] = '.'
            board[target] = own
            sx, sy = nx, ny
    out.side = 'w' if state.side == 'b' else 'b'
    out.ply += 1
    return out


def legal(state, enforce_limit=True):
    if empty_winner(state) or (enforce_limit and state.ply == MAX_PLY):
        return []
    moves = []
    for pb in range(4):
        if pb//2 != (0 if state.side == 'b' else 1):
            continue
        for src in range(16):
            if state.boards[pb][src] != state.side.upper():
                continue
            x, y = src%4, src//4
            for ux in (-1, 0, 1):
                for uy in (-1, 0, 1):
                    if ux == uy == 0:
                        continue
                    for n in (1, 2):
                        tx, ty = x+n*ux, y+n*uy
                        if not (0 <= tx < 4 and 0 <= ty < 4):
                            continue
                        for ab in range(4):
                            if pb%2 == ab%2:
                                continue
                            for src2 in range(16):
                                if state.boards[ab][src2] != state.side.upper():
                                    continue
                                ax, ay = src2%4, src2//4
                                atx, aty = ax+n*ux, ay+n*uy
                                if not (0 <= atx < 4 and 0 <= aty < 4):
                                    continue
                                m = f'{pb}{chr(97+x)}{y+1}{chr(97+tx)}{ty+1}:{ab}{chr(97+ax)}{ay+1}{chr(97+atx)}{aty+1}'
                                try:
                                    apply_raw(state, m)
                                    moves.append(m)
                                except ValueError:
                                    pass
    return sorted(moves)


def result(state):
    return empty_winner(state) or ('ongoing' if legal(state, False) else ('w' if state.side == 'b' else 'b'))


def play(state, move):
    decode(move)  # syntax errors precede terminal legality errors
    if state.ply == MAX_PLY or empty_winner(state):
        raise ValueError('illegal')
    return apply_raw(state, move)


class Engine:
    def __init__(self):
        self.state = parse_sfen(INITIAL)
        self.negotiated = False

    def handle(self, line):
        if (len(line) > 4096 or not line or line.strip(' ') != line or '  ' in line
                or any(ord(c) < 32 or ord(c) > 126 for c in line)):
            return 'error syntax'
        if line == 'quit':
            return 'ok bye'
        if line == 'hello' or line.startswith('hello '):
            if line != 'hello 1':
                return 'error version'
            self.negotiated = True
            return 'ok usp 1'
        if not self.negotiated:
            return 'error handshake'
        try:
            if line == 'ready':
                return 'ok ready'
            if line in ('new', 'position startpos'):
                self.state = parse_sfen(INITIAL)
                return 'ok new' if line == 'new' else 'ok position'
            if line == 'position' or line.startswith('position '):
                self.state = parse_sfen(line[9:])
                return 'ok position'
            if line == 'play' or line.startswith('play '):
                self.state = play(self.state, line[5:])
                return 'ok play'
            if line == 'state':
                return 'state ' + sfen(self.state)
            if line == 'result':
                return 'result ' + result(self.state)
            if line in ('legal', 'go'):
                moves = legal(self.state)
                return ('legal' + ''.join(' '+m for m in moves) if line == 'legal'
                        else 'bestmove ' + (moves[0] if moves else 'none'))
            return 'error syntax'
        except ValueError as e:
            return 'error ' + str(e)


def main():
    engine = Engine()
    stream = sys.stdin.buffer
    while True:
        data = stream.readline(4099)
        if not data:
            break
        oversized = len(data) > 4098 or (not data.endswith(b'\n') and len(data) >= 4099)
        if oversized:
            while data and not data.endswith(b'\n'):
                data = stream.readline(4099)
            if not data:
                break
            reply = 'error syntax'
        elif not data.endswith(b'\n'):
            break
        else:
            data = data[:-1]
            if data.endswith(b'\r'):
                data = data[:-1]
            reply = engine.handle(data.decode('latin1'))
        print(reply, flush=True)
        if reply == 'ok bye':
            break


if __name__ == '__main__':
    main()

