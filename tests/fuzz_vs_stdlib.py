"""Differential fuzz: statistics_rs vs stdlib statistics, bit-exact (compared by repr, plus exception type and message).
usage: fuzz_vs_stdlib.py CASES SEED"""
import math, random, struct, sys
import statistics as S
import statistics_rs as R
from fractions import Fraction
from decimal import Decimal

cases = int(sys.argv[1]); random.seed(int(sys.argv[2]))
FUN = ["mean", "variance", "pvariance", "stdev", "pstdev"]

def rnd_float():
    k = random.random()
    if k < .25: return random.uniform(-1e3, 1e3)
    if k < .4: return random.choice([0.0, -0.0, 0.1, 0.2, 0.3, 1/3, 1e16, -1e16, 1.0, 2.5])
    if k < .55: return struct.unpack("<d", struct.pack("<Q", random.getrandbits(64)))[0]   # any bit pattern (incl. nan/inf/subnormals)
    if k < .65: return random.choice([5e-324, 1e-310, 2.2250738585072014e-308, 1.7976931348623157e308, -1.7976931348623157e308])
    if k < .7: return random.choice([math.inf, -math.inf, math.nan])
    return random.gauss(0, 10 ** random.randint(-30, 30))

def rnd_int():
    k = random.random()
    if k < .5: return random.randint(-100, 100)
    if k < .8: return random.randint(-2**62, 2**62)
    if k < .9: return random.choice([True, False])
    return random.randint(-2**80, 2**80)   # beyond 64 bits -> stdlib fallback path

def dataset():
    n = random.choice([0, 1, 2, 3, 5, 10, 50, 300])
    kind = random.random()
    if kind < .35: d = [rnd_float() for _ in range(n)]
    elif kind < .55: d = [rnd_int() for _ in range(n)]
    elif kind < .85: d = [rnd_float() if random.random() < .5 else rnd_int() for _ in range(n)]
    elif kind < .9: d = [Fraction(random.randint(-50, 50), random.randint(1, 9)) for _ in range(n)]
    elif kind < .95: d = [Decimal(random.randint(-999, 999)) / 100 for _ in range(n)]
    else: d = [rnd_float() for _ in range(n)]; d = tuple(d)
    if random.random() < .1: d = iter(list(d))   # iterators are allowed by the stdlib too
    return d

def run(mod, f, d):
    try: return ("ok", repr(getattr(mod, f)(d)))
    except Exception as e: return ("err", type(e).__name__, str(e))

bad = 0
for i in range(cases):
    d = dataset()
    lst = list(d) if not isinstance(d, (list, tuple)) else d
    for f in FUN:
        a, b = run(S, f, list(lst) if isinstance(lst, list) else lst), run(R, f, iter(list(lst)) if not isinstance(d, (list, tuple)) else lst)
        if a != b:
            bad += 1; print("MISMATCH", f, a, b, repr(lst)[:300], flush=True)
            if bad >= 5: sys.exit(1)
print(f"cases={cases} seed={sys.argv[2]} functions={len(FUN)} mismatches={bad}")
sys.exit(1 if bad else 0)
