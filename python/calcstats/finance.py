"""Time-value-of-money functions. Rates are per period and given as plain numbers (0.05 is 5%)."""

from decimal import Decimal, localcontext
from fractions import Fraction

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


def irr(*cashflows):
    """Internal rate of return by bisection; needs at least one inflow and one outflow."""
    if not (any(cf > 0 for cf in cashflows) and any(cf < 0 for cf in cashflows)):
        raise CalcError("DOMAIN", "irr needs at least one positive and one negative cash flow")
    flows = [Decimal(cf.numerator) / Decimal(cf.denominator) for cf in cashflows]
    with localcontext() as ctx:
        ctx.prec = 40

        def value(rate):
            return sum(cf / (1 + rate) ** i for i, cf in enumerate(flows))

        low, high = Decimal("-0.99"), Decimal(10)
        if value(low) * value(high) > 0:
            raise CalcError("DOMAIN", "irr did not converge")
        for _ in range(200):
            mid = (low + high) / 2
            if value(low) * value(mid) <= 0:
                high = mid
            else:
                low = mid
        result = (low + high) / 2
    return Fraction(round(result * 10**12), 10**12)


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
