# Standard benchmarks

`bench/std.py` and `.github/workflows/bench-std.yml` run crest against the
latest HiGHS release, SCIP and CBC on public test sets, on native GitHub
runners (x86_64 and arm64), to track Crestline against existing solvers.

## Sets

| suite | instances | what |
|---|---|---|
| `miplib` | 240 | the [MIPLIB 2017](https://miplib.zib.de) benchmark set |
| `miplib-easy` | 66 | those HiGHS 1.15.1 solves in under 60 s in the MIPFEAS logs (i9-285K) |
| `mipfeas` | 233 | MIPFEAS: the benchmark set less its 7 infeasible instances |
| `lp` | 250 | LP relaxations of the benchmark set (`solve_relaxation`), and 10 models of [Mittelmann's LP test set](https://plato.asu.edu/ftp/lptestset/) that are not MIPLIB relaxations |

Instances are downloaded at run time (miplib.zib.de, plato.asu.edu) and
kept in `actions/cache`; none are committed.

## Solvers

- **crest**, built from the given ref (`crest_ref`; x86-64-v3 on x86_64,
  as on AWS Lambda);
- **highs**: the static `highs` binary of the latest
  [HiGHS release](https://github.com/ERGO-Code/HiGHS/releases) (generic
  x86_64 build);
- **scip**: SCIP from the `pyscipopt` wheel (its bundled SCIP and SoPlex);
- **cbc**: Ubuntu 24.04's `coinor-cbc` (2.10), off by default (slow).

Default settings and relative gap 1e-4 (HiGHS's default; set for SCIP and
CBC). SCIP and CBC skip the LP suite (SCIP is not an LP code; CLP would
be the fair COIN-OR comparison). `options` passes HiGHS options to crest
and highs only, e.g. `solver=ipm` (IPX in both).

## Metrics

- **solved**: status optimal (MIP; LP also infeasible/unbounded) within the
  limit and not wrong;
- **sgm time**: shifted geometric mean of wall time (model read included)
  over all instances, unsolved at the limit; shift 10 s for MIP, 1 s for LP;
- **sgm primal integral** (MIP): the MIPFEAS score, primal gap to MIPLIB's
  known objective integrated over the limit and divided by it, 2 before the
  first solution; shift 0.001 (as the MIPFEAS study). Over the 233 instances
  with a known objective it is the MIPFEAS benchmark at that limit;
- **correctness**: every MIP result is checked against
  `miplib2017-v31.solu`. A status of infeasible on a feasible instance (or
  a solution of an infeasible one), a dual bound beyond the known objective,
  a primal value better than the optimum (tolerance 1e-6 relative), or
  "optimal" at a value off by more than the gap is WRONG, listed first in the
  report with crashes (a signal) and kills (past 1.5x the limit + 60 s). LP
  objectives of two solvers both optimal must agree to 1e-6 relative.

Each JSONL record (artifact `std-results`) has the instance, solver and
version, arch, CPU model, status, primal and dual bounds, wall time, the
incumbent trajectory and the check. On each runner every instance runs
all solvers back to back (the order rotating), so they see the same CPU.

## Running

Actions, bench-std, "Run workflow": suite, solvers (`crest,highs,scip`),
time limit (300 s), threads (1), archs (`x64,arm64`), shards per arch (32),
crest ref, instance globs, options. Or push a branch `bench-std-run/*`
whose commit changes the defaults in the workflow's `env`. A run job takes
at most 6 h: about shards >= instances x solvers x limit / 4 h.

Locally:

    python3 bench/std.py run out.jsonl --suite miplib-easy \
        --solvers crest=target/release/crest,highs=path/to/highs,scip --tl 60
    python3 bench/std.py report out.jsonl

## Baseline
_Running; tables to follow._
