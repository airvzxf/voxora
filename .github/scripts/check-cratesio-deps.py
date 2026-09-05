#!/usr/bin/env python3
"""Check whether any published version of a crate on crates.io satisfies
a Cargo semver requirement. Used by the release.yml pre-publish guard
(closes #94). Invoked as:

    python3 check-cratesio-deps.py <requirement> <version1> <version2> ...

or via stdin (one version per line, JSON-quoted via jq). Prints the
highest matching version (by semver order, ignoring pre-release ordering
for our purposes) and exits 0, or prints nothing and exits 1.

Supports the operators cargo accepts:
    - exact:      =X.Y.Z   X.Y.Z
    - caret:      ^X.Y.Z   (X>0 -> >=X.Y.Z <(X+1).0.0;
                                X=0,Y>0 -> >=0.Y.Z <0.(Y+1).0;
                                X=0,Y=0 -> >=0.0.Z <0.0.(Z+1))
    - tilde:      ~X.Y.Z   >=X.Y.Z <X.(Y+1).0
    - comparison: >= > <= < =
    - comma:      ">=A, <B" -> all must hold
"""
import sys
import re


def parse(s):
    m = re.match(
        r'^(\d+)\.(\d+)\.(\d+)(?:-([0-9A-Za-z.-]+))?(?:\+[0-9A-Za-z.-]+)?$',
        s.strip(),
    )
    if not m:
        return None
    return (
        int(m.group(1)),
        int(m.group(2)),
        int(m.group(3)),
        m.group(4) or '',
    )


def key(t):
    # Pre-release ranks below the same x.y.z without pre. Within the
    # same x.y.z, pre-release tags compare lexically.
    return (t[0], t[1], t[2], 0 if not t[3] else 1, t[3])


def cmp(a, b):
    ka, kb = key(a), key(b)
    return (ka > kb) - (ka < kb)


def satisfies(req, v):
    pv = parse(v)
    if not pv:
        return False
    req = req.strip()
    # comma range: '>=A, <B' — check first so the leading `>=`
    # doesn't greedily capture the rest of the range.
    if ',' in req:
        parts = [p.strip() for p in req.split(',')]
        return all(satisfies(p, v) for p in parts)
    # exact: '=X.Y.Z' or 'X.Y.Z'
    m = re.match(r'^(=)?(\d+\.\d+\.\d+)$', req)
    if m:
        return pv == parse(m.group(2))
    # caret: ^X.Y.Z
    m = re.match(r'^\^(\d+)\.(\d+)\.(\d+)$', req)
    if m:
        X, Y, Z = int(m.group(1)), int(m.group(2)), int(m.group(3))
        if X > 0:
            lo = (X, Y, Z, '')
            hi = (X + 1, 0, 0, '')
        elif Y > 0:
            lo = (0, Y, Z, '')
            hi = (0, Y + 1, 0, '')
        else:
            lo = (0, 0, Z, '')
            hi = (0, 0, Z + 1, '')
        return lo <= pv < hi
    # tilde: ~X.Y.Z  (>=X.Y.Z, <X.(Y+1).0)
    m = re.match(r'^~(\d+)\.(\d+)\.(\d+)$', req)
    if m:
        X, Y, Z = int(m.group(1)), int(m.group(2)), int(m.group(3))
        lo = (X, Y, Z, '')
        hi = (X, Y + 1, 0, '')
        return lo <= pv < hi
    # >= or >
    m = re.match(r'^(>=?)(.+)$', req)
    if m:
        op, s = m.group(1), m.group(2).strip()
        rv = parse(s) or parse(s + '.0') or parse('0.' + s) or parse(s + '.0.0')
        if not rv:
            return False
        if op == '>=':
            return cmp(pv, rv) >= 0
        return cmp(pv, rv) > 0
    # <= or <
    m = re.match(r'^(<=?)(.+)$', req)
    if m:
        op, s = m.group(1), m.group(2).strip()
        rv = parse(s) or parse(s + '.0') or parse('0.' + s) or parse(s + '.0.0')
        if not rv:
            return False
        if op == '<=':
            return cmp(pv, rv) <= 0
        return cmp(pv, rv) < 0
    return False


def main():
    if len(sys.argv) < 2:
        print('usage: check-cratesio-deps.py <requirement> [version...]', file=sys.stderr)
        sys.exit(2)
    req = sys.argv[1]
    versions = sys.argv[2:] if len(sys.argv) > 2 else []
    if not versions:
        for line in sys.stdin:
            v = line.strip().strip('"').strip('"')
            if v:
                versions.append(v)
    matched = []
    for v in versions:
        if satisfies(req, v):
            matched.append(v)
    if matched:
        # Highest match wins; sort by parsed tuple key
        best = max(matched, key=lambda s: parse(s) or (0, 0, 0, ''))
        print(best)
        sys.exit(0)
    sys.exit(1)


if __name__ == '__main__':
    main()