"""Time-value-of-money functions. Rates are per period and given as plain numbers (0.05 is 5%)."""

from fractions import Fraction
from math import gcd, lcm

from .protocol import CalcError


def _periods(n, name="nper"):
    if n.denominator != 1 or n < 1:
        raise CalcError("DOMAIN", f"{name} must be a positive whole number")
    return int(n)


def _check_rate(rate):
    if rate <= -1:
        raise CalcError("DOMAIN", "rate must be greater than -1")


def pmt(rate, nper, pv):
    """Level payment per period that repays `pv` over `nper` periods."""
    _check_rate(rate)
    n = _periods(nper)
    if rate == 0:
        return pv / n
    growth = (1 + rate) ** n
    return pv * rate * growth / (growth - 1)


def fv(rate, nper, payment, pv=Fraction(0)):
    """Value after `nper` periods of deposits `payment` on top of a starting balance `pv`."""
    _check_rate(rate)
    n = _periods(nper)
    if rate == 0:
        return pv + payment * n
    growth = (1 + rate) ** n
    return pv * growth + payment * (growth - 1) / rate


def pv(rate, nper, payment):
    """Present value of `nper` level payments."""
    _check_rate(rate)
    n = _periods(nper)
    if rate == 0:
        return payment * n
    return payment * (1 - (1 + rate) ** -n) / rate


def npv(rate, *cashflows):
    """Net present value; the first cash flow happens now (period 0)."""
    _check_rate(rate)
    if not cashflows:
        raise CalcError("DOMAIN", "npv needs at least one cash flow")
    return sum((cf / (1 + rate) ** i for i, cf in enumerate(cashflows)), Fraction(0))


def _irr_limit():
    raise CalcError("DOMAIN", "irr calculation exceeds supported size")


def _primitive_polynomial(coefficients):
    """Descending integer coefficients, divided by their positive common factor."""
    denominator = 1
    for coefficient in coefficients:
        denominator = lcm(denominator, coefficient.denominator)
        if denominator.bit_length() > 16384:
            _irr_limit()
    integers = [coefficient.numerator * (denominator // coefficient.denominator)
                for coefficient in coefficients]
    divisor = 0
    for coefficient in integers:
        divisor = gcd(divisor, coefficient)
    return [coefficient // divisor for coefficient in integers] if divisor else []


def _polynomial_sign(coefficients, point):
    """Evaluate the sign at a rational point without constructing large Fractions."""
    numerator, denominator = point.numerator, point.denominator
    # Homogeneous Horner values need at most approximately this many bits.
    # Bound the actual evaluation work, including high degree with a huge root.
    degree = len(coefficients) - 1
    coefficient_bits = max(abs(coefficient).bit_length() for coefficient in coefficients)
    point_bits = max(abs(numerator).bit_length(), denominator.bit_length())
    if coefficient_bits + degree * point_bits + (degree + 1).bit_length() > 65536:
        _irr_limit()
    value, power = coefficients[0], denominator
    for coefficient in coefficients[1:]:
        value = value * numerator + coefficient * power
        power *= denominator
    return (value > 0) - (value < 0)


def _sign_changes(signs):
    nonzero = [sign for sign in signs if sign]
    return sum(a * b < 0 for a, b in zip(nonzero, nonzero[1:]))


def _sturm_sequence(polynomial):
    """Primitive Sturm remainders; positive rescaling preserves root counts."""
    degree = len(polynomial) - 1
    sequence = [polynomial, [coefficient * (degree - i)
                             for i, coefficient in enumerate(polynomial[:-1])]]
    while len(sequence[-1]) > 1:
        remainder = [Fraction(coefficient) for coefficient in sequence[-2]]
        divisor = sequence[-1]
        while len(remainder) >= len(divisor):
            factor = remainder[0] / divisor[0]
            for i, coefficient in enumerate(divisor):
                remainder[i] -= factor * coefficient
            while remainder and remainder[0] == 0:
                remainder.pop(0)
        if not remainder:
            break
        remainder = _primitive_polynomial([-coefficient for coefficient in remainder])
        if sum(abs(coefficient).bit_length() for coefficient in remainder) > 65536:
            _irr_limit()
        sequence.append(remainder)
    return sequence


def irr(*cashflows):
    """The unique real IRR above -1, rounded to 12 places (half to even).

    Multiple distinct admissible roots and no admissible root are domain errors.
    A repeated root is accepted when it is the only distinct admissible root.
    Work is bounded: at most 1024 flows, 512 bits per primitive coefficient,
    16384 coefficient bits in total, and degree 32 for multiple sign changes.
    Larger intermediate Sturm remainders also produce a domain error.
    A point evaluation's estimated intermediate size is limited to 65536 bits.
    """
    if not (any(cf > 0 for cf in cashflows) and any(cf < 0 for cf in cashflows)):
        raise CalcError("DOMAIN", "irr needs at least one positive and one negative cash flow")
    if len(cashflows) > 1024:
        _irr_limit()
    flows = list(cashflows)
    while flows[0] == 0:
        flows.pop(0)
    while flows[-1] == 0:
        flows.pop()
    # For y = 1 + rate > 0, multiplying NPV by y**degree gives this polynomial.
    polynomial = _primitive_polynomial(flows)
    sizes = [abs(coefficient).bit_length() for coefficient in polynomial]
    if max(sizes) > 512 or sum(sizes) > 16384:
        _irr_limit()

    sequence = None
    if _sign_changes(polynomial) > 1:
        if len(polynomial) > 33:
            _irr_limit()
        sequence = _sturm_sequence(polynomial)
        low_variations = _sign_changes([p[-1] for p in sequence])
        roots = low_variations - _sign_changes([p[0] for p in sequence])
        if roots != 1:
            message = "irr has multiple admissible rates" if roots else "irr has no admissible rate"
            raise CalcError("DOMAIN", message)
    # One coefficient sign change guarantees exactly one positive root (Descartes).
    # Cauchy's bound puts all roots strictly below this data-derived integer.
    leading = abs(polynomial[0])
    bound = 2 + max((abs(coefficient) + leading - 1) // leading for coefficient in polynomial[1:])
    low_sign = (polynomial[-1] > 0) - (polynomial[-1] < 0)

    def root_is_left(point):
        if sequence is None:
            return _polynomial_sign(polynomial, point) != low_sign
        variations = _sign_changes([_polynomial_sign(p, point) for p in sequence])
        return low_variations - variations > 0

    scale = 10**12
    # Sparse large coefficients can give a huge Cauchy bound but a root near1.
    # Find a small bracket first; doubling is bounded by the Cauchy bound.
    low, high = Fraction(0), Fraction(1)
    for _ in range(bound.bit_length() + 1):
        if _polynomial_sign(polynomial, high) == 0:
            return high - 1
        if root_is_left(high):
            break
        low, high = high, min(high * 2, Fraction(bound))
    # This many halvings guarantees width < one output unit in this bracket.
    width = high - low
    for _ in range(width.numerator.bit_length() + scale.bit_length()):
        mid = (low + high) / 2
        if _polynomial_sign(polynomial, mid) == 0:
            return Fraction(round((mid - 1) * scale), scale)
        if root_is_left(mid):
            high = mid
        else:
            low = mid
    lower, upper = round((low - 1) * scale), round((high - 1) * scale)
    if lower == upper:
        return Fraction(lower, scale)
    # The narrow interval crosses just one rounding boundary. Check exact ties.
    boundary = Fraction(2 * lower + 1, 2 * scale) + 1
    if _polynomial_sign(polynomial, boundary) == 0:
        return Fraction(round((boundary - 1) * scale), scale)
    return Fraction(lower if root_is_left(boundary) else upper, scale)


def compound(principal, rate, years, per_year=Fraction(1)):
    """Balance after compounding `per_year` times a year for `years` years."""
    _check_rate(rate)
    periods = per_year * years
    if per_year < 1 or per_year.denominator != 1 or periods.denominator != 1 or periods < 0:
        raise CalcError("DOMAIN", "compounding periods must be a whole number")
    return principal * (1 + rate / per_year) ** int(periods)


FUNCTIONS = {
    "pmt": pmt,
    "fv": fv,
    "pv": pv,
    "npv": npv,
    "irr": irr,
    "compound": compound,
}
