"""Black-box golden and differential protocol tests; no third-party dependencies.

python tests/conformance.py --require-all
python tests/conformance.py --engine python examples/python/engine.py
"""
import argparse
import json
import random
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'examples/python'))
from engine import Engine, INITIAL, State, legal, sfen
V = json.loads((ROOT / 'tests/vectors.json').read_text())


def transcript():
    requests, expected = [], []
    oracle = Engine()

    def add(line, response=None):
        actual = oracle.handle(line)
        if response is not None and actual != response:
            raise AssertionError(f'Golden oracle mismatch: {line!r}: {actual!r} != {response!r}')
        requests.append(line)
        expected.append(actual if response is None else response)

    add('state', 'error handshake')
    add('', 'error syntax')
    add('hello 2', 'error version')
    add('hello', 'error version')
    add('hello 1', 'ok usp 1')
    add('ready', 'ok ready')
    add('state', 'state '+INITIAL)
    add('go', 'bestmove 0a1a2:1a1a2')
    add('state', 'state '+INITIAL)  # go is not a commit
    add('legal')
    for v in V['turns']:
        add('position '+v['before'], 'ok position')
        add('play '+v['move'], 'ok play')
        add('state', 'state '+v['after'])
        add('result', 'result '+v['result'] if 'result' in v else None)
        add('legal')
        add('go')
    for v in V['illegal']:
        base = v.get('before', INITIAL)
        add('position '+base, 'ok position')
        add('play '+v['move'], 'error illegal')
        add('state', 'state '+base)
    add('new', 'ok new')
    for text in V['invalid_sfen']:
        add('position '+text if text else 'position', 'error sfen')
        add('state', 'state '+INITIAL)
    for bad in ['play','play bad','play 0a1a2:1a1a2x','unknown','state extra','ready x',
                'hello  1',' hello 1','hello 1 ','hello\t1','x\0y','réady',
                'x'*4096,'x'*4097,'x'*10000]:
        add(bad, 'error syntax')
        add('state', 'state '+INITIAL)
    add('hello 3', 'error version')
    add('ready', 'ok ready')
    add('hello 1', 'ok usp 1')
    add('state', 'state '+INITIAL)
    blocked='4/4/WW2/BW2|4/4/WW2/BW2|3W/4/4/B3|3W/4/4/B3 b 0'
    add('position '+blocked,'ok position')
    add('result','result w')
    add('legal','legal')
    add('go','bestmove none')
    add('position '+INITIAL[:-1]+'2147483647','ok position')
    add('legal','legal')
    add('result','result ongoing')
    add('go','bestmove none')
    add('new','ok new')
    # Seeded games, testing complete legal lists as well as transitions.
    rng=random.Random(98765)
    for _ in range(5):
        add('new','ok new')
        for _ in range(40):
            moves=legal(oracle.state)
            add('legal')
            add('result')
            add('go')
            if not moves:
                break
            add('play '+rng.choice(moves),'ok play')
            add('state')
    # Arbitrary structurally valid studies exercise edge cases not reached in play.
    for _ in range(80):
        boards=[]
        for _ in range(4):
            squares=rng.sample(range(16),8)
            board=['.']*16
            for idx in squares[:rng.randint(1,4)]:
                board[idx]='B'
            for idx in squares[4:4+rng.randint(1,4)]:
                board[idx]='W'
            boards.append(board)
        state=State(boards,rng.choice(['b','w']),rng.randint(0,100))
        add('position '+sfen(state),'ok position')
        add('state','state '+sfen(state))
        add('legal')
        add('result')
        add('go')
        # Test arbitrary pairs, usually illegal, and ensure errors are atomic.
        for _ in range(3):
            sq=lambda:chr(97+rng.randrange(4))+str(1+rng.randrange(4))
            move=f'{rng.randrange(4)}{sq()}{sq()}:{rng.randrange(4)}{sq()}{sq()}'
            add('play '+move)
            add('state')
    add('quit','ok bye')
    return requests, expected


def run(command, data, expected, label):
    process=subprocess.run(command,input=data,stdout=subprocess.PIPE,stderr=subprocess.PIPE,
                           cwd=ROOT,timeout=120)
    if process.returncode:
        raise AssertionError(f'{label}: exit {process.returncode}: {process.stderr.decode(errors="replace")}')
    if process.stderr:
        raise AssertionError(f'{label}: unexpected stderr: {process.stderr!r}')
    lines=process.stdout.decode('ascii').splitlines()
    if len(lines)!=len(expected):
        raise AssertionError(f'{label}: got {len(lines)} responses, expected {len(expected)}')
    for i,(actual,want) in enumerate(zip(lines,expected)):
        if actual!=want:
            raise AssertionError(f'{label}: response {i+1}: {actual[:500]!r} != {want[:500]!r}')


def main():
    parser=argparse.ArgumentParser()
    parser.add_argument('--engine',nargs='+',action='append')
    parser.add_argument('--require-all',action='store_true')
    args=parser.parse_args()
    commands=args.engine or [[sys.executable,'examples/python/engine.py']]
    if not args.engine:
        node=shutil.which('node')
        if node: commands.append([node,'examples/javascript/engine.mjs'])
        suffix='.exe' if sys.platform=='win32' else ''
        for name in ['c','cpp','rust','zig']:
            path=ROOT/'build'/f'shobu-{name}{suffix}'
            if path.exists(): commands.append([str(path)])
    if args.require_all and len(commands)!=6:
        parser.error('all six examples must be built and Node.js installed')
    requests,expected=transcript()
    # Deliberately mix LF/CRLF, then verify EOF and quit behavior in separate runs.
    data=b''.join(line.encode('utf-8')+(b'\r\n' if i%2 else b'\n') for i,line in enumerate(requests))
    for command in commands:
        label=Path(command[0]).name+(' '+command[1] if len(command)>1 else '')
        run(command,data,expected,label)
        run(command,b'hello 1\nplay 0a1a2:1a1a2', ['ok usp 1'],label+' EOF')
        run(command,b'hello 1\n'+b'x'*10000, ['ok usp 1'],label+' oversized EOF')
        run(command,b'quit\nhello 1\n', ['ok bye'],label+' quit')
        run(command,b'hello 1\nstate\r\r\nready\n', ['ok usp 1','error syntax','ok ready'],label+' CR')
        # Exact byte boundaries with and without CRLF.
        for ending in [b'\n',b'\r\n']:
            run(command,b'hello 1'+ending+b'x'*4096+ending+b'ready'+ending,
                ['ok usp 1','error syntax','ok ready'],label+' boundary')
            run(command,b'x'*4096+ending+b'x'*4097+ending+b'hello 1'+ending,
                ['error handshake','error syntax','ok usp 1'],label+' framing precedence')
        run(command,b'hello 1\n\x1a\nready\n', ['ok usp 1','error syntax','ok ready'],label+' Ctrl-Z')
        print(f'PASS {label}: {len(expected)} golden/differential responses + framing checks')


if __name__=='__main__':
    main()

