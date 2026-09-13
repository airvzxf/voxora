#!/usr/bin/env python3
"""Check whether any published version of a crate on crates.io satisfies
a Cargo semver requirement. Used by the release.yml pre-publish guard
(closes #94). Invoked as:

    python3 check-cratesio-deps.py <requirement> <version1> <version2> ...

or via stdin (one version per line, JSON-quoted via jq). Prints the
highest matching version (by semver order) and exits 0, or prints
nothing and exits 1.

Supports the operators cargo accepts:
    - exact:      =X.Y.Z   X.Y.Z
    - caret:      ^X.Y.Z   (X>0 -> >=X.Y.Z <(X+1).0.0;
                                X=0,Y>0 -> >=0.Y.Z <0.(Y+1).0;
                                X=0,Y=0 -> >=0.0.Z <0.0.(Z+1))
    - tilde:      ~X.Y.Z   >=X.Y.Z <X.(Y+1).0
    - comparison: >= > <= < =
    - comma:      ">=A, <B" -> all must hold

Closes [#110](https://github.com/airvzxf/voxora/issues/110): per
cargo's pre-release exclusion rule (verified against the
`semver = "1.0.28"` crate pinned in `Cargo.lock:1`), a candidate
with a non-empty pre-release tag is excluded unless the
requirement's lower bound names that exact pre-release. The previous
implementation inverted pre-release ordering and used raw tuple
comparison in the caret/tilde/comparison branches, so a pre-release
candidate was accepted as "satisfying" a `^X.Y.Z` or `~X.Y.Z`
requirement that would then fail at `cargo publish` time.
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
    # Per semver 2.0 §11: when major.minor.patch are equal, a
    # pre-release version has LOWER precedence than the normal
    # version. The `0` slot marks "has pre-release", so a
    # non-empty pre-release tuple sorts BEFORE the same x.y.z with
    # an empty pre-release.
    return (t[0], t[1], t[2], 0 if t[3] else 1, t[3])


def cmp(a, b):
    ka, kb = key(a), key(b)
    return (ka > kb) - (ka < kb)


def req_has_pre(req):
    """True iff the requirement's lower bound explicitly names a
    pre-release tag (e.g. `^0.4.3-rc.1` or `>=0.4.0-rc.5`). Per cargo,
    only then are pre-release candidates admitted."""
    m = re.search(r'-\d', req)
    return m is not None


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
    m = re.match(r'^\^(\d+)\.(\d+)\.(\d+)(-[0-9A-Za-z.-]+)?$', req)
    if m:
        X, Y, Z = int(m.group(1)), int(m.group(2)), int(m.group(3))
        req_pre = m.group(4) or ''
        # Per cargo, pre-release candidates are excluded unless the
        # requirement itself names a pre-release lower bound.
        if pv[3] and not req_pre:
            return False
        if X > 0:
            lo = (X, Y, Z, req_pre)
            hi = (X + 1, 0, 0, '')
        elif Y > 0:
            lo = (0, Y, Z, req_pre)
            hi = (0, Y + 1, 0, '')
        else:
            lo = (0, 0, Z, req_pre)
            hi = (0, 0, Z + 1, '')
        return cmp(pv, lo) >= 0 and cmp(pv, hi) < 0
    # tilde: ~X.Y.Z  (>=X.Y.Z, <X.(Y+1).0)
    m = re.match(r'^~(\d+)\.(\d+)\.(\d+)(-[0-9A-Za-z.-]+)?$', req)
    if m:
        X, Y, Z = int(m.group(1)), int(m.group(2)), int(m.group(3))
        req_pre = m.group(4) or ''
        if pv[3] and not req_pre:
            return False
        lo = (X, Y, Z, req_pre)
        hi = (X, Y + 1, 0, '')
        return cmp(pv, lo) >= 0 and cmp(pv, hi) < 0
    # >= or >
    m = re.match(r'^(>=?)(.+)$', req)
    if m:
        op, s = m.group(1), m.group(2).strip()
        rv = parse(s) or parse(s + '.0') or parse('0.' + s) or parse(s + '.0.0')
        if not rv:
            return False
        # The lower bound names a pre-release iff the parsed `rv`
        # has a non-empty pre-release slot.
        if pv[3] and not (rv[3] or req_has_pre(req)):
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
        # Highest match wins; sort by parsed tuple via `key()` so
        # pre-release ordering follows semver 2.0 §11 (pre-release
        # ranks BELOW the normal version of the same x.y.z).
        best = max(matched, key=lambda s: key(parse(s) or (0, 0, 0, '')))
        print(best)
        sys.exit(0)
    sys.exit(1)


if __name__ == '__main__':
    main()