import random, time, statistics as S, statistics_rs as R
random.seed(3)
sets = [("1,000,000 floats (gauss)", [random.gauss(100, 15) for _ in range(1_000_000)]),
        ("1,000,000 ints", [random.randint(-10**9, 10**9) for _ in range(1_000_000)]),
        ("10,000 floats", [random.random() for _ in range(10_000)])]
print("| Data | Function | stdlib statistics | statistics_rs | Speedup | Bit-identical |\n|---|---|---|---|---|---|")
for name, d in sets:
    for f in ("mean", "stdev", "pvariance"):
        reps = 1 if len(d) > 100000 else 20
        t = time.perf_counter()
        for _ in range(reps): a = getattr(S, f)(d)
        t1 = (time.perf_counter() - t) / reps
        t = time.perf_counter()
        for _ in range(reps): b = getattr(R, f)(d)
        t2 = (time.perf_counter() - t) / reps
        print(f"| {name} | `{f}` | {t1*1000:.1f} ms | {t2*1000:.2f} ms | {t1/t2:.0f}× | {'yes' if repr(a) == repr(b) else 'NO'} |")
