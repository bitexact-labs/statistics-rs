"""Drop-in replacement for the standard library `statistics` module.

mean, variance, pvariance, stdev and pstdev run in Rust for float/int/bool data and return bit-identical
results (same exact-rational algorithm, same correct rounding). Everything else, and any other data type
(Fraction, Decimal, float/int subclasses, ints beyond 64 bits, a given xbar/mu), is handled by the stdlib."""
import statistics as _std
from statistics import *  # noqa: F401,F403  (re-export everything else unchanged)
from statistics import StatisticsError  # noqa: F401

from . import _core

__all__ = list(_std.__all__)


def _seq(data):
    return data if isinstance(data, (list, tuple)) else list(data)


def mean(data):
    data = _seq(data)
    r = _core.mean(data)
    return _std.mean(data) if r is None else r


def variance(data, xbar=None):
    data = _seq(data)
    if xbar is None:
        r = _core.variance(data)
        if r is not None:
            return r
    return _std.variance(data, xbar)


def pvariance(data, mu=None):
    data = _seq(data)
    if mu is None:
        r = _core.pvariance(data)
        if r is not None:
            return r
    return _std.pvariance(data, mu)


def stdev(data, xbar=None):
    data = _seq(data)
    if xbar is None:
        r = _core.stdev(data)
        if r is not None:
            return r
    return _std.stdev(data, xbar)


def pstdev(data, mu=None):
    data = _seq(data)
    if mu is None:
        r = _core.pstdev(data)
        if r is not None:
            return r
    return _std.pstdev(data, mu)
