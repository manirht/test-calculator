"""Statistics and finance functions for the calculator, served over a JSON-lines pipe."""

from .protocol import CalcError, handle_request, serve

__all__ = ["CalcError", "handle_request", "serve"]
