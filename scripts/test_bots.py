"""Verify the example policies against the independent USP Python rules oracle."""
import argparse
from pathlib import Path
import random
import subprocess
import sys

ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT / "tests/upstream/examples/python"))
from engine import Engine, INITIAL, State, legal, play, result, sfen


def select(state):
    moves=legal(state)
    if not moves:
        return "none"
    def score(move):
        child=play(state,move)
        outcome=result(child)
        value=10000 if outcome=="b" else -10000 if outcome=="w" else sum(b.count("B")-b.count("W") for b in child.boards)
        return value if state.side=="b" else -value
    # Python max keeps the first entry on equal scores, matching the examples.
    return max(moves,key=score)


def transcript():
    oracle=Engine(); requests=[]; expected=[]
    def add(line):
        if line=="go":
            reply="bestmove "+select(oracle.state)
        else:
            reply=oracle.handle(line)
        requests.append(line); expected.append(reply)
    add("state"); add("hello 1"); add("ready"); add("state"); add("go"); add("go"); add("state")
    capture="3W/4/B3/4|4/4/1BW1/4|3W/4/4/B3|3W/4/4/B3 b 0"
    add("position "+capture); add("go"); add("state")
    assert expected[-2]=="bestmove 0a2c2:1b2d2"
    add("play 0a2c2:1b2d2"); add("go"); add("state")
    add("position "+INITIAL[:-1]+"2147483647"); add("go"); add("result")
    add("position 4/4/WW2/BW2|4/4/WW2/BW2|3W/4/4/B3|3W/4/4/B3 b 0"); add("go")
    rng=random.Random(20261011)
    # Real games plus arbitrary studies exercise White, terminal detection and
    # collision-safe caches; go is tested twice without committing each state.
    add("new")
    for _ in range(12):
        add("go"); add("go"); add("state")
        moves=legal(oracle.state)
        if not moves:
            break
        add("play "+rng.choice(moves)); add("state")
    for _ in range(8):
        boards=[]
        for _ in range(4):
            board=["."]*16; squares=rng.sample(range(16),8)
            for q in squares[:rng.randint(1,4)]: board[q]="B"
            for q in squares[4:4+rng.randint(1,4)]: board[q]="W"
            boards.append(board)
        add("position "+sfen(State(boards,rng.choice(["b","w"]),10)))
        add("go"); add("state")
    for line in ["", "hello\t1", "x"*10000, "réady", "\x1a", "state\r"]:
        add(line); add("ready")
    add("quit")
    data=b"".join(line.encode("utf-8")+(b"\r\n" if i%2 else b"\n") for i,line in enumerate(requests))
    return data,expected


def run(command,data,expected):
    p=subprocess.run(command,input=data,stdout=subprocess.PIPE,stderr=subprocess.PIPE,cwd=ROOT,timeout=120)
    assert p.returncode==0,(command,p.returncode,p.stderr)
    assert not p.stderr,(command,p.stderr)
    actual=p.stdout.decode("ascii").splitlines()
    assert len(actual)==len(expected),(command,len(actual),len(expected))
    for i,(a,b) in enumerate(zip(actual,expected)):
        assert a==b,(command,i,a,b)


def main():
    parser=argparse.ArgumentParser()
    parser.add_argument("--engine",nargs="+",action="append",required=True)
    args=parser.parse_args()
    data,expected=transcript()
    for command in args.engine:
        run(command,data,expected)
        run(command,b"hello 1\ngo",["ok usp 1"])
        run(command,b"quit\nhello 1\n",["ok bye"])
        run(command,b"hello 1\n"+b"x"*10000,["ok usp 1"])
        print(f"PASS {Path(command[0]).name}: {len(expected)} bot-policy responses + EOF/quit checks")


if __name__=="__main__": main()
