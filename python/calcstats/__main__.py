import sys

from .protocol import serve

if __name__ == "__main__":
    serve(sys.stdin, sys.stdout)
