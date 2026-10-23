#!/usr/bin/env python3
"""Developer-run bounded differential check against Python's JSON parser.

Build the Rust probe first; this script never builds/downloads dependencies.
No external service, nonstandard Python package, or generated file is required.
"""
import argparse
import json
import random
import subprocess


def reject_constant(value):
    raise ValueError(value)


def accepted(data):
    try:
        # Decode UTF-8 explicitly: json.loads(bytes) also accepts UTF-16/32.
        # Preserve numeric spelling rather than imposing Python float limits.
        value = json.loads(data.decode("utf-8"), parse_int=str, parse_float=str,
                           parse_constant=reject_constant)
        return isinstance(value, (dict, list))
    except (ValueError, UnicodeError):
        return False


def corpus():
    rng = random.Random(42)
    cases = [b"", b"{}", b"[]", b"true", b"{} []", b"[01]", b"[1e999999]",
             b'["\\uD800"]', b'["\\uDC00"]', b'["\\uD834\\uDD1E"]',
             b'["\\q"]', b'["\xff"]', b'[NaN]', b'[Infinity]', b'\xef\xbb\xbf{}']
    for index in range(64):
        value = {"key": [index, rng.randint(-10**30, 10**30), None, True,
                         "雪😀\\\"\n" + chr(rng.randint(32, 126))], "empty": {}}
        encoded = json.dumps(value, ensure_ascii=index % 2 == 0, separators=(",", ":")).encode()
        cases.append(encoded)
        # Truncation and single-byte mutations include lexical/UTF-8 boundaries.
        for _ in range(3):
            at = rng.randrange(len(encoded))
            cases.append(encoded[:at])
            cases.append(encoded[:at] + bytes([rng.randrange(256)]) + encoded[at + 1:])
        cases.append(b"junk [invalid] " + encoded + b" [1,] []")
    for _ in range(128):
        cases.append(bytes(rng.randrange(256) for _ in range(rng.randrange(64))))
    return cases


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--probe", default="target/debug/examples/reference_probe")
    args = parser.parse_args()
    cases = corpus()
    result = subprocess.run([args.probe], input=b"".join(data.hex().encode() + b"\n" for data in cases),
                            stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=120, check=True)
    rows = result.stdout.splitlines()
    assert len(rows) == len(cases), (len(rows), len(cases), result.stderr)
    candidates = 0
    for data, row in zip(cases, rows):
        observation = json.loads(row)
        assert observation["strict"] == accepted(data), (data, observation)
        for start, end in observation["spans"]:
            assert 0 <= start < end <= len(data), (data, observation)
            assert accepted(data[start:end]), (data, start, end)
            candidates += 1
    print(f"PASS: {len(cases)} inputs; {candidates} recovered candidates; "
          "whole/every split/one-byte events and EOF lifecycle agree")


if __name__ == "__main__":
    main()
