#!/usr/bin/env python3
"""Per-period count columns for a dispatch MILP (free MPS, Pyomo names):

  dispatch_counts.py IN.mps[.gz] OUT.mps [--changes]

For each period t, a general integer column count_switch(t) in [0, #banks]
and a row count_switch(t) - sum_b switch(b_t) = 0, objective 0 (with
--changes also count_switch_change(t) for switch_change). Every bank stays;
the solver can branch on the count. See "Count columns" in docs/dispatch.md.
"""
import gzip, re, sys
from collections import defaultdict

COL = re.compile(r'^(switch|switch_change)\((.*)_(\d+)\)$')


def counts(src, dst, changes=False):
    kinds = ('switch', 'switch_change') if changes else ('switch',)
    lines = (gzip.open if src.endswith('.gz') else open)(src, 'rt').read().splitlines()
    sec = {l.split()[0]: i for i, l in enumerate(lines) if l and not l[0].isspace() and l[0] != '*'}
    cols = defaultdict(set)  # (kind, t) -> banks
    out, last = [], None

    def close(col):
        m = col and COL.match(col)
        if m and m.group(1) in kinds:
            cols[(m.group(1), m.group(3))].add(m.group(2))
            out.append(f"     {col} c_count_{m.group(1)}({m.group(3)})_ -1")

    for l in lines[sec['COLUMNS'] + 1:sec['RHS']]:
        c = l.split()[0]
        if c != last:
            close(last)
            last = c
        out.append(l)
    close(last)
    key = lambda k: (k[0], int(k[1]))
    new = [f"count_{k}({t})" for k, t in sorted(cols, key=key)]
    out += ["    MARKER_CNT 'MARKER' 'INTORG'"] + [f"     {n} c_{n}_ 1" for n in new] + ["    MARKER_CNT 'MARKER' 'INTEND'"]
    rows = [f" E  c_{n}_" for n in new]
    bounds = [b for (k, t) in sorted(cols, key=key) for b in (f" LO BOUND count_{k}({t}) 0", f" UP BOUND count_{k}({t}) {len(cols[(k, t)])}")]
    end = sec['ENDATA']
    res = (lines[:sec['COLUMNS']] + rows + lines[sec['COLUMNS']:sec['COLUMNS'] + 1] + out + lines[sec['RHS']:end] +
           ([] if 'BOUNDS' in sec else ['BOUNDS']) + bounds + lines[end:])
    open(dst, 'w').write('\n'.join(res) + '\n')
    return len(new)


if __name__ == '__main__':
    a = [x for x in sys.argv[1:] if x != '--changes']
    print(f"{counts(a[0], a[1], '--changes' in sys.argv)} count columns")
