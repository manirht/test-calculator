import unittest
from fractions import Fraction as F

from calcstats import finance
from calcstats.protocol import CalcError


class Loans(unittest.TestCase):
    def test_pmt_with_zero_rate_splits_evenly(self):
        self.assertEqual(finance.pmt(F(0), F(12), F(1200)), 100)

    def test_pmt_matches_known_payment(self):
        payment = finance.pmt(F(5, 1200), F(360), F(200000))
        self.assertAlmostEqual(float(payment), 1073.64, places=2)

    def test_pmt_repays_the_loan(self):
        rate, n, loan = F(1, 100), F(12), F(1000)
        payment = finance.pmt(rate, n, loan)
        self.assertEqual(finance.pv(rate, n, payment), loan)

    def test_periods_must_be_whole_and_positive(self):
        for bad in (F(0), F(-3), F(5, 2)):
            with self.assertRaises(CalcError):
                finance.pmt(F(1, 100), bad, F(1000))


class Growth(unittest.TestCase):
    def test_fv_of_deposits(self):
        self.assertEqual(finance.fv(F(1, 10), F(2), F(100)), 210)

    def test_fv_with_starting_balance(self):
        self.assertEqual(finance.fv(F(1, 10), F(2), F(0), F(100)), 121)

    def test_fv_zero_rate(self):
        self.assertEqual(finance.fv(F(0), F(5), F(10), F(7)), 57)

    def test_compound_yearly_and_monthly(self):
        self.assertEqual(finance.compound(F(1000), F(1, 10), F(2)), 1210)
        monthly = finance.compound(F(1000), F(12, 100), F(1), F(12))
        self.assertAlmostEqual(float(monthly), 1126.8250301319698, places=9)

    def test_rate_floor(self):
        with self.assertRaises(CalcError):
            finance.compound(F(1), F(-2), F(1))


class CashFlows(unittest.TestCase):
    def test_npv_discounts_from_period_zero(self):
        self.assertEqual(finance.npv(F(1, 10), F(-100), F(110)), 0)
        self.assertEqual(finance.npv(F(0), F(-100), F(50), F(50)), 0)

    def test_npv_needs_a_cash_flow(self):
        with self.assertRaises(CalcError):
            finance.npv(F(1, 10))

    def test_irr_recovers_the_discount_rate(self):
        rate = finance.irr(F(-100), F(110))
        self.assertAlmostEqual(float(rate), 0.10, places=9)

    def test_irr_needs_both_signs(self):
        with self.assertRaises(CalcError):
            finance.irr(F(100), F(50))

    def test_irr_finds_roots_outside_the_old_rate_bracket(self):
        for flows, expected in (
            ((-1, 12), F(11)),
            ((-1000, 1), F(-999, 1000)),
            # Growth polynomials -y^2 + 144 and -1000000*y^2 + 1.
            ((-1, 0, 144), F(11)),
            ((-1000000, 0, 1), F(-999, 1000)),
            ((-1, 0, 10**40), F(10**20 - 1)),
        ):
            with self.subTest(flows=flows):
                self.assertEqual(finance.irr(*(F(x) for x in flows)), expected)

    def test_irr_accepts_one_distinct_repeated_root(self):
        # -(y-1)^2, then (y^2-2)^2: only one positive growth factor each.
        self.assertEqual(finance.irr(F(-1), F(2), F(-1)), 0)
        self.assertEqual(finance.irr(F(1), F(0), F(-4), F(0), F(4)), F(414213562373, 10**12))

    def test_irr_rejects_ambiguous_positive_growth_roots(self):
        # (y-1)(y-2), then (y-1)^2(y-2): rates 0 and 1 both solve NPV=0.
        for flows in ((1, -3, 2), (1, -4, 5, -2)):
            with self.subTest(flows=flows):
                with self.assertRaises(CalcError) as ctx:
                    finance.irr(*(F(x) for x in flows))
                self.assertEqual(ctx.exception.code, "DOMAIN")

    def test_irr_rejects_mixed_cashflows_without_a_real_rate(self):
        # y^2-y+1 = (y-1/2)^2 + 3/4 never vanishes.
        with self.assertRaises(CalcError) as ctx:
            finance.irr(F(1), F(-1), F(1))
        self.assertEqual(ctx.exception.code, "DOMAIN")

    def test_irr_keeps_unique_nonconventional_roots(self):
        # (1-y)(y^2+1) has exactly one positive root, despite three sign changes.
        self.assertEqual(finance.irr(F(-1), F(1), F(-1), F(1)), 0)
        self.assertEqual(finance.irr(F(0), F(-100), F(110), F(0)), F(1, 10))

    def test_irr_rounds_exact_halfway_rates_to_even(self):
        denominator = 2 * 10**12
        for numerator, expected in ((1, F(0)), (3, F(2, 10**12))):
            # -d^2*y^2 + (d+n)^2 has growth factor 1+n/d.
            with self.subTest(numerator=numerator):
                self.assertEqual(
                    finance.irr(F(-denominator**2), F(0), F((denominator + numerator)**2)),
                    expected,
                )

    def test_irr_rejects_work_beyond_its_resource_budget(self):
        for flows in (
            [F(-1)] + [F(1)] * 1024,
            [F((-1)**i) for i in range(34)],
            [F(-1), F(2**513)],
        ):
            with self.subTest(count=len(flows)):
                with self.assertRaises(CalcError) as ctx:
                    finance.irr(*flows)
                self.assertEqual(ctx.exception.code, "DOMAIN")


if __name__ == "__main__":
    unittest.main()
