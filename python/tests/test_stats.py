import unittest
from fractions import Fraction as F

from calcstats import stats
from calcstats.protocol import CalcError


def fr(*items):
    return [F(i) for i in items]


class MeanMedianMode(unittest.TestCase):
    def test_mean_is_exact(self):
        self.assertEqual(stats.mean(*fr(1, 2, 4)), F(7, 3))
        self.assertEqual(stats.mean(F(1, 10), F(2, 10)), F(3, 20))

    def test_mean_of_one_value(self):
        self.assertEqual(stats.mean(F(5)), 5)

    def test_median_odd_and_even(self):
        self.assertEqual(stats.median(*fr(5, 1, 3)), 3)
        self.assertEqual(stats.median(*fr(4, 1, 3, 2)), F(5, 2))

    def test_median_does_not_depend_on_input_order(self):
        self.assertEqual(stats.median(*fr(9, 1, 5, 7, 3)), stats.median(*fr(1, 3, 5, 7, 9)))

    def test_mode_picks_most_common(self):
        self.assertEqual(stats.mode(*fr(1, 2, 2, 3)), 2)

    def test_mode_breaks_ties_with_the_smallest(self):
        self.assertEqual(stats.mode(*fr(3, 3, 1, 1, 2)), 1)

    def test_empty_input_is_a_domain_error(self):
        for fn in (stats.mean, stats.median, stats.mode):
            with self.assertRaises(CalcError) as ctx:
                fn()
            self.assertEqual(ctx.exception.code, "DOMAIN")


class Spread(unittest.TestCase):
    def test_sample_and_population_variance(self):
        data = fr(2, 4, 4, 4, 5, 5, 7, 9)
        self.assertEqual(stats.pvariance(*data), 4)
        self.assertEqual(stats.variance(*data), F(32, 7))

    def test_stddev_exact_when_rational(self):
        self.assertEqual(stats.pstddev(*fr(2, 4, 4, 4, 5, 5, 7, 9)), 2)

    def test_stddev_is_close_when_irrational(self):
        value = stats.stddev(*fr(2, 4, 4, 4, 5, 5, 7, 9))
        self.assertAlmostEqual(float(value), 2.138089935299395, places=12)

    def test_sample_variance_needs_two_values(self):
        with self.assertRaises(CalcError):
            stats.variance(F(1))
        self.assertEqual(stats.pvariance(F(1)), 0)

    def test_sqrt_fraction(self):
        self.assertEqual(stats.sqrt_fraction(F(9, 4)), F(3, 2))
        self.assertEqual(stats.sqrt_fraction(F(0)), 0)
        with self.assertRaises(CalcError):
            stats.sqrt_fraction(F(-1))


class Percentile(unittest.TestCase):
    def test_endpoints(self):
        data = fr(10, 20, 30, 40)
        self.assertEqual(stats.percentile(F(0), *data), 10)
        self.assertEqual(stats.percentile(F(100), *data), 40)

    def test_median_percentile(self):
        self.assertEqual(stats.percentile(F(50), *fr(1, 2, 3, 4)), F(5, 2))

    def test_interpolates_between_ranks(self):
        self.assertEqual(stats.percentile(F(25), *fr(10, 20, 30, 40, 50)), 20)
        self.assertEqual(stats.percentile(F(90), *fr(1, 2, 3, 4, 5)), F(23, 5))

    def test_unsorted_input(self):
        self.assertEqual(stats.percentile(F(50), *fr(40, 10, 30, 20)), 25)

    def test_out_of_range(self):
        with self.assertRaises(CalcError):
            stats.percentile(F(101), F(1))
        with self.assertRaises(CalcError):
            stats.percentile(F(-1), F(1))


if __name__ == "__main__":
    unittest.main()
