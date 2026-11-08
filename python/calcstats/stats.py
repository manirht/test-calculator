"""Descriptive statistics over exact Fractions."""

import math
from collections import Counter
from decimal import Decimal, localcontext
from fractions import Fraction

from .protocol import CalcError

SQRT_DIGITS = 20


def _need(values, minimum, name):
    if len(values) < minimum:
        raise CalcError("DOMAIN", f"{name} needs at least {minimum} value(s), got {len(values)}")


def sqrt_fraction(value):
    """Square root with SQRT_DIGITS significant digits; exact when the root is rational."""
    if value < 0:
        raise CalcError("DOMAIN", "square root of a negative number")
    num_root = math.isqrt(value.numerator)
    den_root = math.isqrt(value.denominator)
    if num_root * num_root == value.numerator and den_root * den_root == value.denominator:
        return Fraction(num_root, den_root)
    with localcontext() as ctx:
        ctx.prec = SQRT_DIGITS
        root = (Decimal(value.numerator) / Decimal(value.denominator)).sqrt()
    return Fraction(root)


def mean(*values):
    _need(values, 1, "mean")
    return sum(values, Fraction(0)) / len(values)


def median(*values):
    _need(values, 1, "median")
    ordered = sorted(values)
    middle = len(ordered) // 2
    if len(ordered) % 2:
        return ordered[middle]
    return (ordered[middle - 1] + ordered[middle]) / 2


def mode(*values):
    _need(values, 1, "mode")
    counts = Counter(values)
    best = max(counts.values())
    return min(v for v, c in counts.items() if c == best)


def pvariance(*values):
    _need(values, 1, "pvariance")
    centre = mean(*values)
    return sum(((v - centre) ** 2 for v in values), Fraction(0)) / len(values)


def variance(*values):
    _need(values, 2, "variance")
    centre = mean(*values)
    return sum(((v - centre) ** 2 for v in values), Fraction(0)) / (len(values) - 1)


def pstddev(*values):
    return sqrt_fraction(pvariance(*values))


def stddev(*values):
    return sqrt_fraction(variance(*values))


def percentile(p, *values):
    """Linear interpolation between closest ranks; p runs from 0 to 100."""
    _need(values, 1, "percentile")
    if p < 0 or p > 100:
        raise CalcError("DOMAIN", "percentile must be between 0 and 100")
    ordered = sorted(values)
    rank = p / 100 * (len(ordered) - 1)
    lower = math.floor(rank)
    upper = math.ceil(rank)
    fraction = rank - lower
    return ordered[lower] + (ordered[upper] - ordered[lower]) * fraction


FUNCTIONS = {
    "mean": mean,
    "median": median,
    "mode": mode,
    "variance": variance,
    "pvariance": pvariance,
    "stddev": stddev,
    "pstddev": pstddev,
    "percentile": percentile,
}
