#!/bin/sh
#
# check-no-avx512.sh FILE... — fail if any object code in FILE (static
# archive, object or executable) contains an AVX-512 (EVEX) instruction.
#
# GitHub-hosted runners mix CPUs with and without AVX-512, and the Rust
# cache moves the compiled ggml from one runner to the next. Code built
# for AVX-512 dies with SIGILL on a runner without it.
#
# In 64-bit mode the 0x62 opcode byte is always the EVEX prefix, so an
# instruction whose first byte after the optional address-size or
# segment prefixes is 62 is an AVX-512 instruction.
set -eu

[ "$#" -gt 0 ] || { echo "usage: $0 FILE..." >&2; exit 2; }

listing=$(mktemp)
trap 'rm -f "$listing"' EXIT

status=0
for file in "$@"; do
    # Without this, a file objdump cannot read would count as clean.
    objdump -d -w "$file" > "$listing"
    count=$(awk -F'\t' '
        NF >= 3 {
            n = split($2, b, " ")
            i = 1
            while (i <= n && b[i] ~ /^(67|26|2e|36|3e|64|65)$/) i++
            if (b[i] == "62") evex++
        }
        END { print evex + 0 }' "$listing")
    if [ "$count" -ne 0 ]; then
        echo "ERROR: $file has $count AVX-512 (EVEX) instructions" >&2
        status=1
    else
        echo "ok: $file has no AVX-512 instructions"
    fi
done
exit "$status"
