# statistics-rs: `statistics.mean / variance / stdev` 16–45× faster, bit-for-bit identical

Python's `statistics` module is accurate because it sums your data as exact fractions (`fractions.Fraction`)
and rounds only once at the end. That is also why it is slow. This package does the same exact arithmetic
with big integers in Rust and rounds the same way, so the result is **the same float, bit for bit**, not "close".

```python
import statistics_rs as statistics     # drop-in: everything else is re-exported from the stdlib
statistics.stdev(data)
```

## Benchmark (Python 3.14, Rust 1.93, Lenovo M720q)
| Data | Function | stdlib | statistics_rs | Speedup | Bit-identical |
|---|---|---|---|---|---|
| 1,000,000 floats | `mean` | 498 ms | 22.9 ms | 22× | yes |
| 1,000,000 floats | `stdev` | 742 ms | 24.2 ms | 31× | yes |
| 1,000,000 floats | `pvariance` | 721 ms | 17.7 ms | 41× | yes |
| 1,000,000 ints | `mean` | 228 ms | 14.7 ms | 16× | yes |
| 1,000,000 ints | `stdev` | 366 ms | 18.3 ms | 20× | yes |
| 10,000 floats | `stdev` | 7.3 ms | 0.17 ms | 44× | yes |
Reproduce: `python bench/bench.py`. Most of the remaining time is reading a million Python objects.

## How it stays exact
- Every finite float is `m · 2^e`. Mantissas are summed per exponent in 128-bit buckets that spill into big integers, so Σx and Σx² are exact.
- `variance = (n·Σx² − (Σx)²) / (n·(n−1))` as an exact fraction, like the stdlib's `_ss`.
- Converting back: correctly rounded (round-half-even) big-integer division, including subnormals and the same `OverflowError`.
- `stdev`: the stdlib's round-to-odd integer square root (`_float_sqrt_of_frac`), ported as is.
- Integer data keeps the stdlib's types: `mean([2, 4])` is the `int` 3, `mean([1, 2])` is the float 1.5.
- `inf`/`nan` follow the stdlib path exactly (including `stdev` raising `ValueError('inf or nan encountered in data')`).

## Verification
`tests/fuzz_vs_stdlib.py` compares all five functions with the stdlib on random datasets: random bit patterns (so every exponent,
subnormals, ±0.0, inf, nan), huge and tiny values, ints up to 2^62, bools, mixes, empty and one-element data, iterators, plus
Fraction/Decimal (fallback path). Results are compared by `repr` and exceptions by type and message. See `tests/fuzz_*.log`.

## Scope
- Accelerated: `mean`, `variance`, `pvariance`, `stdev`, `pstdev` for `float`, `int` (within 64 bits) and `bool` data.
- Handed to the stdlib unchanged: Fraction, Decimal, subclasses, larger ints, an explicit `xbar`/`mu`, and every other function
  (`fmean`, `median`, `quantiles`, `NormalDist`, ...). The module re-exports them, so you can import it in place of `statistics`.

## License
PSF-2.0 (the algorithms come from CPython's `statistics.py`). See `LICENSE` and `NOTICE`.
