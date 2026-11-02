# Draft: bug report for HiGHS (not filed)

## MIP: dual proofs drop tiny coefficients without their bound term, giving invalid conflicts and wrong optima

**Version.** HiGHS master at 73cac48c53 (2026-08-19, 1.15.1), file
`highs/mip/HighsLpRelaxation.cpp`.

**Symptom.** On traininstance6 (MIPLIB 2017, optimum 28290), a
HiGHS-derived solver with the same code reports "optimal" with a dual bound
above the optimum. It reported 28350 with `random_seed = 9`, and 28380 and
29090 on a nearby search path. It depends on the path: most seeds are
right.

**In HiGHS itself: not reproduced.** HiGHS v1.15.1, built from the
tag on macOS arm64 (clang, Release, 1 thread), solves traininstance6
correctly (28290) on seeds 0-11. So does a build that follows HiGHS's
paths exactly (Crestline's oracle mode, which matches the C++ app's logs
byte for byte), on the same 12 seeds. The wrong results came only from
Crestline's own default paths, which differ from HiGHS's in the LP (the
hyper-sparse solve order and pricing) and in a few heuristics. The faulty
code is identical, so HiGHS can produce such a proof on another path, but
we have no HiGHS run that ends with a wrong answer.

With the debug solution (`-DDEBUGSOL=ON`, `mip_debug_solution_file` set
to an optimum, and HiGHS's assertions in `HighsDebugSol.cpp` turned into
prints so that a run keeps going after the first report), the dual
infeasibility proofs of v1.15.1 are violated by the optimum on 6 of the 8
seeds (0-6 and 9; 1 to 21 times per run, up to 2.2e-2 absolute). All of
those proofs have very large right-hand sides (|rhs| 1e6 to 1e11), so the
relative violations are about 1e-11 or smaller. With the fix below,
seeds 5 and 9 still show such violations but smaller (at most 9.4e-6,
same |rhs| range). That looks like roundoff in badly scaled rays, not
this bug, and it does not show the bug in HiGHS directly. (The first
violated proof we found in Crestline had rhs 5.1e-5 and violation 1.1e-6:
2% relative.)

**Cause.** Both proofs are built as the aggregate `a = c - y^T A` (bound
proofs) or `a = y^T A` (infeasibility proofs, from the dual ray), and then
`sum_j a_j x_j <= rhs` is used as a valid inequality:

- `computeDualProof`, line 880:
  `if (std::fabs(val) <= mipsolver.options_mip_->small_matrix_value) continue;`
- `storeDualInfProof`, line 993:
  `if (std::fabs(val) <= mipsolver.mipdata_->epsilon) continue;`

These lines drop entries `a_j` with `|a_j| <= 1e-9` from the inequality.
Unlike the entries below feastol a few lines later, they do not move
`a_j * bound_j` to the right-hand side. The inequality left over is implied
only when `|a_j| * |bound_j|` is negligible next to the slack of the proof,
and that does not hold for columns with large bounds.

What we observed: an infeasible strong-branching LP gave a ray proof with
47 coefficients of about 1.1e-6 and right-hand side 5.10e-5. One dropped
entry lay on an integer column with a bound of about 29000. Dropping it
tightened the proof by about 3e-5. The optimal solution has activity
5.21e-5 on the proof, so the proof cut it off. `generate_conflict` turned
the proof into a conflict cut that excludes every optimum. After the next
restart, that cut was part of the model, whose optimum was then 31780.

**Why it went unnoticed.** `checkDualProof()` exists, but its result is
ignored at line 1278: `if (true || checkDualProof()) return
Status::kInfeasible;`. The violation here is also tiny in absolute terms
(1e-6), so only checking the cuts against a known optimum (the
`debugSolution` machinery, or our equivalent) shows it.

**Fix.** Treat tiny entries like the entries below feastol: move them to
the right-hand side with their global bound, and give up on the proof if
that bound is infinite. In both functions:

```cpp
if (val == 0.0) continue;  // was: fabs(val) <= small_matrix_value / epsilon
bool removeValue = std::fabs(val) <= mipsolver.mipdata_->feastol;
```

(Skipping rows with tiny weights when `y^T A` is formed, as
`storeDualInfProof` also does, is valid: it is just another nonnegative
combination of the rows. Only dropping a column term without its bound is
not.)

**Cost.** Measured in Crestline (a Rust port of HiGHS's MIP solver) on
MIPLIB 2017 "benchmark" (240 instances, 300 s, 1 thread, shared CI
runners on x64 and arm64), the fix costs about one or two solved
instances and about 1% in shifted geometric mean time. One pair of runs
showed more:

| Arch | Solved | Shifted geometric mean |
|---|---|---|
| x64 | 95 -> 90 | 140.6 -> 143.4 s |
| arm64 | 94 -> 92 | 144.2 -> 145.5 s |

But a repeat run that takes the same search paths as the fixed build
solved 93 on x64 and 93 on arm64. On these runners the solved count
varies by about ±3 from run to run.

No run gave a wrong result with the fix. With the fix, traininstance6
gives 28290 on every seed we tried. The extra time is the price of valid
proofs: the old ones were stronger only because they were invalid.


## MIP: bounds and substitutions through tiny coefficients cut off feasible points

**Version.** HiGHS 1.15.1 (`highs/mip/HighsDomain.cpp`,
`highs/presolve/HPresolve.cpp`); the same code in Crestline, which gives
the same answers.

**Symptom.** Differential fuzzing (docs/benchmarks.md, "Fuzzing for wrong
answers") of small random MIPs whose rows mix coefficients of 1e-9..1e-6
with ones of 1e2..1e6 finds models that HiGHS calls infeasible, or solves
to a wrong optimum, with default options, where a solution exists that
satisfies every row exactly (checked by fixing its integers and
re-solving the LP to 1e-10, or by enumeration). In 3000 random models
(500 of each family, 2 option settings each): 74, 88 and 22 flagged
solves in the three tiny-coefficient families; none in knapsacks and set
covers; 5 in the family scaled like the dispatch models, all of them
feasible only within tolerance (in exact arithmetic the other solver's
point is infeasible). Two shrunk cases:

- `bench/fuzz.py make random 23` (9 integer columns, 22 rows): HiGHS says
  infeasible, with and without presolve; optimum 45.2259.
- `bench/fuzz.py make random 1784` (fixed charge, big-M rows with M up to
  7e5, three rows with a coefficient of 2e-8..4e-8): HiGHS says optimal at
  533.74; an exactly feasible solution has 150.29. Shrunk to 14 rows, 19
  free columns: HiGHS 302.23, optimum 140.01 (enumerating the 4 binaries).
  `presolve_rule_off = 4096` (the aggregator) or zeroing the 3.7e-8 entry
  removes the error.

**Cause (random 23).** Shrunk to 3 rows and 2 free integer columns, presolve
off: row `r15` is `-2.085e-8 c1 - 29.04 c4 + 1.299 c7 + ... >= -290.96`,
tight at the feasible point (c4, c7) = (11, 22), its maximal activity
short of the side by 1.9e-13 (roundoff). Propagating it, `HighsDomain`
derives `c1 <= -1.9e-13 / 2.085e-8 = -9e-6` and rounds that with feastol
in the column's space: `floor(-9e-6 + 1e-6) = -1 < 0 = lb(c1)`, a
conflict. A point that violates the row by 1.9e-13 is feasible (the
tolerance is 1e-6 on the row), but dividing by the tiny coefficient turns
that into a bound violation of 9e-6 on the column. The column-space
tolerance covers the row tolerance only for |a| >= 1.
`HPresolve::updateColImpliedBounds` does the same (`c1 <= -8.2e-6`), and so
does the bound transfer of `doubletonEq` through a coefficient of 9.4e-8.

**Fix (sketch, not applied).** Derive implied bounds from the side relaxed
by feastol when |a| < 1, so that no point violating the row by at most
feastol is cut off. In `propagateRowUpper/Lower` and
`updateColImpliedBounds` this fixes random 23 (45.2259 with and without
presolve) and 7 of 11 wrong answers on 200 random models, but changes
search paths, and 2 models then meet the same pattern elsewhere (the
doubleton bound transfer; a solution rejected for an absolute row
violation of 2.4e-6 on a row with side 8e5). Each place that divides a
bound out of a row would need it.

**Not seen on the dispatch models**: their coefficients lie in
[5.7e-3, 1.1e2] and their sides below 2e3 (no wrong answer in 216
perturbed dispatch models), so Crestline leaves this as HiGHS has it.
