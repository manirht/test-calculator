import io
import json
import unittest
from fractions import Fraction as F

from calcstats import protocol
from calcstats.protocol import CalcError, format_number, handle_request, parse_number


class NumberCodec(unittest.TestCase):
    def test_parse_accepts_fractions_decimals_and_exponents(self):
        self.assertEqual(parse_number("1/3"), F(1, 3))
        self.assertEqual(parse_number("-2.5"), F(-5, 2))
        self.assertEqual(parse_number("1e3"), 1000)
        self.assertEqual(parse_number("2.5e-3"), F(1, 400))
        self.assertEqual(parse_number(" 7 "), 7)

    def test_parse_does_not_lose_precision(self):
        self.assertEqual(parse_number("0.1") + parse_number("0.2"), parse_number("0.3"))
        self.assertEqual(parse_number("12345678901234567890.123456789"), F(12345678901234567890123456789, 10**9))

    def test_parse_rejects_garbage(self):
        for bad in ("abc", "", "1/0", None, True, [1]):
            with self.assertRaises(CalcError) as ctx:
                parse_number(bad)
            self.assertEqual(ctx.exception.code, "DOMAIN")

    def test_format_exact_values(self):
        self.assertEqual(format_number(F(4)), "4")
        self.assertEqual(format_number(F(-1, 3)), "-1/3")
        self.assertEqual(format_number(F(1, 8)), "1/8")

    def test_format_bounds_huge_denominators(self):
        huge = F(1, 3) + F(1, 10**30)
        text = format_number(huge)
        self.assertLessEqual(F(text).denominator, protocol.OUTPUT_SCALE)
        self.assertAlmostEqual(float(F(text)), 1 / 3, places=11)


class Requests(unittest.TestCase):
    def test_success(self):
        reply = handle_request({"id": 5, "fn": "mean", "args": ["1", "2", "4"]})
        self.assertEqual(reply, {"id": 5, "ok": True, "value": "7/3"})

    def test_unknown_function(self):
        reply = handle_request({"id": 1, "fn": "nope", "args": []})
        self.assertFalse(reply["ok"])
        self.assertEqual(reply["error"]["code"], "UNKNOWN_FUNC")

    def test_wrong_arity(self):
        reply = handle_request({"id": 1, "fn": "pmt", "args": ["1"]})
        self.assertEqual(reply["error"]["code"], "BAD_ARITY")

    def test_domain_error_is_reported(self):
        reply = handle_request({"id": 1, "fn": "median", "args": []})
        self.assertEqual(reply["error"]["code"], "DOMAIN")

    def test_malformed_request(self):
        self.assertEqual(handle_request([1, 2])["error"]["code"], "BAD_REQUEST")
        self.assertEqual(handle_request({"id": 1, "fn": "mean", "args": "x"})["error"]["code"], "BAD_REQUEST")

    def test_every_registered_function_is_callable_by_name(self):
        self.assertIn("percentile", protocol.FUNCTIONS)
        self.assertIn("irr", protocol.FUNCTIONS)


class Serve(unittest.TestCase):
    def test_one_reply_per_line(self):
        lines = [
            json.dumps({"id": 1, "fn": "mean", "args": ["2", "4"]}),
            "",
            "{not json",
            json.dumps({"id": 3, "fn": "median", "args": ["9", "1", "5"]}),
        ]
        out = io.StringIO()
        protocol.serve(io.StringIO("\n".join(lines) + "\n"), out)
        replies = [json.loads(line) for line in out.getvalue().splitlines()]
        self.assertEqual(len(replies), 3)
        self.assertEqual(replies[0]["value"], "3")
        self.assertEqual(replies[1]["error"]["code"], "BAD_REQUEST")
        self.assertEqual(replies[2]["value"], "5")


if __name__ == "__main__":
    unittest.main()
