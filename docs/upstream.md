# HiGHS: dual proofs that drop tiny coefficients (fixed upstream)

**Status.** Fixed in HiGHS by
[PR #3223](https://github.com/ERGO-Code/HiGHS/pull/3223) ("Fix
traininstance6 bug", Mark Turner), merged into `latest` on 2026-10-20 and
not yet in a release (1.15.1 has the bug). Crest follows that fix by
default; the cargo feature `highs_dual_proofs` keeps 1.15.1's behaviour
for the CI oracle. We found the same bug independently and had drafted a
report; it is no longer needed.

## The bug (HiGHS 1.15.1)

`computeDualProof` and `storeDualInfProof` in
`highs/mip/HighsLpRelaxation.cpp` build a proof `sum_j a_j x_j <= rhs`
(`a = c - y^T A` from the LP duals, or `a = y^T A` from a dual ray) and
skipped every entry with `|a_j| <= 1e-9`. Unlike the entries below
feastol a few lines later, they did not move `a_j * bound_j` to the
right-hand side, so with large bounds the proof was strengthened past
validity. `checkDualProof()` would not catch it: its result is ignored
(`if (true || checkDualProof())`).

What we saw in crest: on traininstance6 (MIPLIB 2017, optimum 28290) a
strong-branching ray proof with 47 coefficients of about 1.1e-6 and
right-hand side 5.1e-5 dropped an entry on an integer column with a bound
of about 29000. That tightened it by about 3e-5 and cut off the optimum;
the conflict derived from it ended in "optimal" 28350 (seed 9), and 28380
or 29090 on nearby paths. HiGHS 1.15.1 itself gave 28290 on seeds 0-11:
the code was the same, but we had no HiGHS run that ended wrong. The PR
reports occasional failures on traininstance6 and traininstance2.

## The upstream fix

- `HighsDomain::termIsZero(val, lb, ub, epsilon)`: an entry is dropped only
  if `|val| <= epsilon` and `|val| * max(|lb|, |ub|) <= epsilon`; the rest
  take the feastol path to the right-hand side (no proof if the bound is
  infinite). Also used in `HighsTransformedLp::transform`/`untransform`.
- Exact (`HighsCDouble`) products in the proof sums, the ray aggregation,
  `tightenCoefficients`, objective propagation and
  `extractCliquesFromCut`. The PR notes that this alone fixed the case it
  isolated.
- [#2874](https://github.com/ERGO-Code/HiGHS/issues/2874) problem 4: bounds
  derived in `extractCliquesFromCut` use the tolerance
  `max(feastol, epsilon / |coefficient|)`.

Our own earlier fix (move every nonzero entry to the right-hand side)
was stricter and cost about 1% of MIPLIB time; crest now uses the
upstream version so that the oracle can follow HiGHS once it is released.
