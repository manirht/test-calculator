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


if __name__ == "__main__":
    unittest.main()
