"""Wire format shared with the Rust bridge.

Request:  {"id": 1, "fn": "mean", "args": ["1", "2", "3/2"]}
Reply:    {"id": 1, "ok": true, "value": "9/6"}
Error:    {"id": 1, "ok": false, "error": {"code": "DOMAIN", "message": "..."}}

Numbers are exact strings ("7", "1/3", "2.5e-3") parsed into Fractions, so nothing is rounded on
the way in. Results with a very large denominator are rounded to 12 decimal places on the way out
so the Rust side can always hold them.
"""

import json
from fractions import Fraction

MAX_DENOMINATOR = 10**15
OUTPUT_SCALE = 10**12


class CalcError(Exception):
    def __init__(self, code, message):
        super().__init__(message)
        self.code = code
        self.message = message


def parse_number(text):
    if isinstance(text, bool) or not isinstance(text, (str, int)):
        raise CalcError("DOMAIN", f"argument must be a numeric string, got {text!r}")
    try:
        return Fraction(str(text).strip())
    except (ValueError, ZeroDivisionError):
        raise CalcError("DOMAIN", f"invalid number {text!r}") from None


def format_number(value):
    value = Fraction(value)
    if value.denominator > MAX_DENOMINATOR:
        value = Fraction(round(value * OUTPUT_SCALE), OUTPUT_SCALE)
    return str(value)


def _registry():
    from . import finance, stats

    functions = {}
    functions.update(stats.FUNCTIONS)
    functions.update(finance.FUNCTIONS)
    return functions


FUNCTIONS = _registry()


def handle_request(request):
    request_id = request.get("id") if isinstance(request, dict) else None
    try:
        if not isinstance(request, dict):
            raise CalcError("BAD_REQUEST", "request must be a JSON object")
        name = request.get("fn")
        function = FUNCTIONS.get(name)
        if function is None:
            raise CalcError("UNKNOWN_FUNC", f"unknown function {name!r}")
        args = request.get("args", [])
        if not isinstance(args, list):
            raise CalcError("BAD_REQUEST", "'args' must be a list")
        values = [parse_number(a) for a in args]
        try:
            result = function(*values)
        except TypeError:
            raise CalcError("BAD_ARITY", f"wrong number of arguments for {name}") from None
        return {"id": request_id, "ok": True, "value": format_number(result)}
    except CalcError as e:
        return {"id": request_id, "ok": False, "error": {"code": e.code, "message": e.message}}


def serve(stdin, stdout):
    for line in stdin:
        line = line.strip()
        if not line:
            continue
        try:
            request = json.loads(line)
        except json.JSONDecodeError as e:
            reply = {"id": None, "ok": False, "error": {"code": "BAD_REQUEST", "message": f"invalid JSON: {e}"}}
        else:
            reply = handle_request(request)
        stdout.write(json.dumps(reply) + "\n")
        stdout.flush()
