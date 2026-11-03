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
| `lp` | 249 | LP relaxations of the benchmark set (`solve_relaxation`), and 9 models of [Mittelmann's LP test set](https://plato.asu.edu/ftp/lptestset/) that are not MIPLIB relaxations |

Instances are downloaded at run time (miplib.zib.de, plato.asu.edu) and
kept in `actions/cache`; none are committed.

## Solvers

- **crest**, built from the given ref (`crest_ref`; x86-64-v3 on x86_64,
  as on AWS Lambda);
- **highs**: the static `highs` binary of the latest
  [HiGHS release](https://github.com/ERGO-Code/HiGHS/releases) (generic
  x86_64 build);
- **highs-src**: the same release built from source in the runner as the
  release binary is (static, gcc), plus crest's target CPU (x86-64-v3 on
  x86_64, the default on arm64) and LTO: against `highs` it measures the
  build flags, against `crest` the algorithm alone;
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
  a primal value better than the optimum (tolerances 1e-6 relative on bounds, 1e-5 on the primal value), or
  "optimal" at a value off by more than the gap is WRONG, listed first in the
  report with crashes (a signal) and kills (past 1.5x the limit + 60 s). LP
  objectives of two solvers both optimal must agree to 1e-6 relative.

Each JSONL record (artifact `std-results`) has the instance, solver and
version, arch, CPU model, status, primal and dual bounds, wall time, the
iteration counts (simplex, IPM, crossover, PDLP), the
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

### More than 4 cores: spot EC2 (`archs=ec2`)

GitHub's free runners have 4 vCPUs (2 physical cores on x64). For 8+
threads, `bench/ec2_runner.sh up N` starts N spot Graviton instances
(c8g.2xlarge: 8 cores, about $0.10/h in us-east-2) as one-job runners;
then run bench-std or bench-dispatch with `archs=ec2` and `shards=N`.
Builds stay on GitHub's arm64 runners. Each instance terminates after its
job, and at most HOURS (6) after boot; `ls` and `down` list and terminate
leftovers. A full MIPLIB run of three solvers at 8 threads is about 40
instance-hours (about $4).

## Screen first, then the full run

A candidate change goes through two stages, and the full benchmark runs
only for one that passes the first:

| Stage | Dispatch (bench-dispatch) | Standard (bench-std) | Cost |
|---|---|---|---|
| Screen | `screen` input, or a `bench-screen/*` branch: 10 instances (the three Lambda ticks, `hard_10-03_1340`, `diverse_10-01_0052` and the five `dm_*` models most sensitive to past changes), 8 seeds, 120 s, 8 runners per arch | `miplib-easy`, crest against main only | about 1/5 of a full run |
| Full | all instances, 16 seeds, 180 s, 16 runners per arch, 1 and 2 threads (and 4 for changes to threads >= 3) | `miplib` (240), plus `lp` for LP changes | |

Before either, screen locally on a few models by instruction count
(`CRESTLINE_INSTR_CLOCK`). A screen that shows no gain, or a loss on any
motivating tick, ends the experiment: record it in the docs and move on.

## Fuzzing for wrong answers

`bench/fuzz.py` and `.github/workflows/fuzz.yml` look for wrong answers,
not speed: a wrong status, a dual bound past a feasible solution, or an
infeasible "solution". Each model is solved by crest twice (default
options, and one variant drawn from the model's key: presolve off, another
seed, 2 threads, symmetry off, or the C++ app's paths), by the HiGHS
release (default options, and the same variant: the *twin*) and by SCIP,
at `mip_rel_gap` 0 (small models) or 1e-6. Every solution file is checked
against the model (bounds, integrality, rows; absolute 1e-6, as HiGHS). A
solution counts as evidence against another solver only once polished:
its integers rounded and fixed, the rest re-solved as an LP to 1e-10.
Without that, most flags on badly scaled models are solutions feasible
only within tolerance (an integer at 4e-8 opening a big-M row, an
all-integer equality met to 2e-6).
A flag on crest that its twin does not share is **crest-only**: a port bug
or a path difference. A flag both share is inherited from HiGHS.

Sources (`--sources`, default `random:2000,check,miplib,dispatch:6`):
random small MIPs (knapsacks, set covers, big-M fixed charge, general MIPs
with coefficients over 1e-9..1e6, and general MIPs scaled like the dispatch
models), the `.mps` models of HiGHS's `check/instances`, the 249 MIPLIB
2017 collection instances tagged easy with at most 20k nonzeros, and
perturbed private dispatch models. A dispatch perturbation scales the costs
by 1 +- 3%, loosens bounds by 0-3%, lowers the initial SOC by 0-3% and, in
3 seeds of 4, truncates the horizon. It is made in the runner's temp
directory and never uploaded. Logs and results hold names, keys, options,
statuses, objectives, bounds, violation sizes and times only.

Run: Actions, fuzz, "Run workflow" (sources, shards, archs, crest ref), or
push a branch `fuzz-run/*`. The report (run summary, artifact
`fuzz-results`) counts solves and flags by source and lists the flagged
models, crest-only first. To reproduce one: `bench/fuzz.py make SOURCE KEY
model.mps` (a dispatch one stays local), then shrink it with
`bench/mipdd.sh`. Locally:

    python3 bench/fuzz.py run out.jsonl --crest target/release/crest \
        --highs path/to/highs --sources random:200 --tmp /tmp/fuzz
    python3 bench/fuzz.py report out.jsonl

**First campaign** (2026-11-02, crest main f2620a9; runs 41351599623
on x64 and local runs on arm64):

| source | models | crest solves | crest flagged | of which crest-only |
|---|---|---|---|---|
| random (runners) | 2000 | 4000 | 166 | 9 |
| random (local, polished evidence) | 3000 | 6000 | 187 | 0 |
| check/instances | 64 | 128 | 0 | 0 |
| MIPLIB collection, easy, <= 20k nonzeros | 245 | 490 | 7 | 3 |
| perturbed dispatch | 216 | 432 | 0 | 0 |

Triage: the random flags shared with the HiGHS release are an inherited
HiGHS weakness with tiny coefficients (docs/upstream.md); the crest-only
ones there are the same models on another search path (crest's FMA build
against the release's plain x86_64 build). MIPLIB: two instances whose
known optimum all three solvers miss by 7e-6 (MIPLIB's value), and two
crest-only wrong answers, reproduced on Linux and macOS with 16 seeds
(HiGHS release: 0 of 8 wrong) for maxgasflow (an unscaled LP called
infeasible, fixed: docs/performance.md, "IPX cleanup"), and
neos-3083784-nive, where crest's dual bounds of 1304-4980 cross MIPLIB's
optimum 0. That one is MIPLIB's: it lists no solution, SCIP 10 proves a
bound of 3446 in 30 min, and "optimal 0" (HiGHS and crest with presolve
off) uses a binary at 1 - 2.75e-10 in a row with a 1e10 big-M.

## Baseline

2026-10-16, crest `bench-std` (main 48f6190 + this harness), HiGHS 1.15.1
release, SCIP 10.0 (pyscipopt 6.3); 1 thread, 300 s; GitHub runners:
arm64 Neoverse-N2, x64 a mix of AMD EPYC 7763/9V45/9V74 and Intel Xeon
8370C/6973P (all solvers of an instance on the same one). Runs
38939685629 (MIPLIB, 32 shards per arch), 38939643071 (LP, 16) and
38939646677 (LP with `solver=ipm`, 16).

**MIPLIB 2017 benchmark** (240 instances; primal integral over the 233
with a known objective, i.e. MIPFEAS at 300 s)

| arch | solver | solved | wrong | killed | sgm time (s, shift 10) | sgm primal integral | with a solution |
|---|---|---|---|---|---|---|---|
| x64 | crest | **93** | 0 | 0 | **141.8** | **0.0879** | 211 |
| x64 | HiGHS 1.15.1 | 87 | 0 | 2 | 150.6 | 0.1035 | 203 |
| x64 | SCIP 10.0 | 74 | 0 | 0 | 164.3 | 0.1461 | 197 |
| arm64 | crest | **92** | 0 | 0 | **144.3** | **0.0920** | 208 |
| arm64 | HiGHS 1.15.1 | 82 | 0 | 2 | 151.2 | 0.1059 | 204 |
| arm64 | SCIP 10.0 | 73 | 0 | 1 | 167.0 | 0.1522 | 196 |

No wrong answer and no crash for crest. HiGHS 1.15.1 overran the limit
past the kill (510 s) on germanrr and neos-5114902-kasavu (both arches),
SCIP on satellites2-40 (arm64). SCIP's incumbent on mas74 is 7e-6 below the
optimum (feasibility tolerance; the check allows 1e-5).

On instances both solve, crest takes 0.875 (x64) / 0.894 (arm64) of HiGHS
1.15.1's shifted time and 0.77 of SCIP's. Where crest is weakest:

- against HiGHS 1.15.1: neos-2978193-inde (3-4x, both arches), decomp2
  (1.7-3.6x), neos-873061 (unsolved; HiGHS 133-155 s), neos-950242 (1.3-1.45x),
  and on arm64 only gmu-35-50 and neos-933966 (unsolved; HiGHS 16 s and 33 s);
- against SCIP: neos-787933 and graph20-20-1rand (SCIP 2-6 s, crest
  unsolved), fhnw-binpack4-4 (infeasible; SCIP proves it in 12-17 s),
  CMS750_4, istanbul-no-cutoff, rocI-4-11, fast0507 (unsolved), decomp2,
  piperout-27, cod105, pk1, enlight_hard (2-4x); neos-3004026-krka has no
  crest solution for most of the limit (primal integral 1.3-2.0 vs 0.3-0.45).

**LP** (MIPLIB benchmark relaxations + 10 Mittelmann LPs; L1_sixm250obs, a
fixed-format file neither reads, has since been dropped)

| arch | method | solver | solved | sgm time (s, shift 1) | crest/HiGHS on both solved |
|---|---|---|---|---|---|
| x64 | default (dual simplex) | crest | **231** | **2.91** | 0.926 |
| x64 | default (dual simplex) | HiGHS 1.15.1 | 228 | 3.22 | |
| x64 | `solver=ipm` (IPX) | crest | 242 | 3.04 | 1.029 |
| x64 | `solver=ipm` (IPX) | HiGHS 1.15.1 | **243** | **2.89** | |
| arm64 | default (dual simplex) | crest | 231 | **2.89** | 0.920 |
| arm64 | default (dual simplex) | HiGHS 1.15.1 | 231 | 3.22 | |
| arm64 | `solver=ipm` (IPX) | crest | 241 | 3.05 | 0.995 |
| arm64 | `solver=ipm` (IPX) | HiGHS 1.15.1 | **242** | **3.04** | |

The `solver=ipm` rows are from 2026-10-17 (crest `ipm`, run
39003533706; was 3.26 against 2.98 on x64, 3.24 against 3.14 on arm64,
before the IPX matching and bounds-check work of docs/performance.md).
What remains is floating-point paths: the release is a gcc build (no FMA
on x64), and crest built like it (`libstdcxx no_fma`, generic x86_64)
takes its path on 243 of 249 LPs. On crest's paths neos-631710 (3.8x on
x64), neos-4763324-toguru and neos-5049753-cuanza (2x) are slower, and
irish-electricity's relaxation ends "Unknown" (the dual simplex on the
original LP after postsolve stops with 3e-5 primal infeasibilities; the
C++ app of the pinned fork does the same, the no_fma build ends
"Optimal" like 1.15.1). Simplex: slowest
are physiciansched3-3 and ns1760995 relaxations (time limit vs 250-260 s on
arm64) and cod105 (1.1-1.4x).

## Threads

2026-11-02, the GitHub runners (4 vCPUs: arm64 Neoverse-N2, 4 cores; x64
EPYC/Xeon, 2 cores with 2 hyperthreads each). SCIP runs its default,
sequential (one thread whatever the setting), as reference.

**MIPLIB 2017 benchmark** (240, 300 s; solved / sgm time, shift 10 s;
runs 41313764455, 41351818355, 41351816521 with crest main)

| solver | arm64 1 | arm64 2 | arm64 4 | x64 1 | x64 2 | x64 4 |
|---|---|---|---|---|---|---|
| crest main | 94 / 142.6 | 94 / 143.2 | 94 / 142.4 | 94 / 141.0 | 98 / 140.5 | 94 / 141.3 |
| crest, 3+ thread changes below | | | **99 / 136.8** | | | **96 / 138.4** |
| HiGHS 1.15.1 release | 82 / 151.6 | 82 / 151.1 | 82 / 150.4 | 87 / 151.7 | 89 / 150.4 | 86 / 150.2 |
| HiGHS 1.15.1, crest's flags (`highs-src`) | | 89 / 149.5 | 88 / 148.8 | | 90 / 147.3 | 89 / 146.6 |
| SCIP 10.0 (sequential) | 74 / 167.1 | | | 73 / 164.9 | | |

Neither crest main nor HiGHS gets anything from more threads on MIPLIB:
at the default gap (1e-4) there is no concurrent helper (it needs
`mip_rel_gap >= 1e-3`), and the parallel tree search is off unless
`parallel=on`; the threads only race IPX at the root and detect symmetry.
Built with crest's flags (x86-64-v3 / LTO), HiGHS solves 1-7 more and is
1-2.5% faster; crest stays ahead by 5-8 solved and 4-5% in time. The
`highs-src` row at 1 thread was not run.

**Dispatch** (all 36 instances, 8 seeds, 180 s, gap 1%; solved of 288 /
sgm time, shift 1 s; runs 41314262664, 41314264650, 41314266781 with main)

| crest | arm64 1 | arm64 2 | arm64 4 | x64 1 | x64 2 | x64 4 |
|---|---|---|---|---|---|---|
| main | 259 / 10.5 | 278 / 6.7 | 276 / 6.7 | 259 / 10.8 | 278 / 6.9 | 278 / 6.3 |
| 3+ thread changes | (main's path) | (main's path) | **280 / 5.8** | (main's path) | (main's path) | 275 / 6.5 |

Speedup 1 -> 2 -> 4 threads, main: 1.57x, then none (arm64); 1.58x, then
noise (x64; the 2-thread and 4-thread rows of two runs differ by
6.3-7.1 s). Threads 3 and 4 were idle.

**From 3 threads on** (main since this section): a concurrent LNS helper
on each thread but the main solver's, at most the physical cores less one,
each with its own seed, sharing the incumbents and bounds of the one pool;
and `parallel=choose` (the default) turns the parallel tree search on. With
1 or 2 threads every path is main's (checked: equal logs at 1 thread, and
with `mip_deterministic` at 2; paired against main at 2 threads, run
41445118849: arm64 1.018 [0.996, 1.041], x64 1.004 [0.978, 1.031], the
usual spread of two-thread timing). Paired against main at 4 threads (run
41445115653, 288 pairs per arch, shifted time ratio, 95% interval):
arm64 **0.879 [0.847, 0.912]**, solved 280 = 280; x64 1.013 [0.989,
1.037], where the 2 physical cores leave one helper (main's helper count)
and only the tree search is new. MIPLIB at 4 threads: +4 solved, sgm
-4.1% (arm64) / -1.5% (x64), all from the tree search (run 41369267711).

Rejected or not done:

- Helpers on all logical CPUs: on the hyperthreaded x64 runners 3
  helpers made the `dm_*` screen models 1.18-1.29x slower (time to 1%
  1.19x; run 41315073863) while arm64 gained; hence the physical-core cap.
- Helpers alone (no parallel tree search) at 4 threads: arm64 0.892
  [0.861, 0.925] on dispatch (run 41369266138), nothing on MIPLIB; the
  tree search adds MIPLIB's gain and does not cost dispatch (screen
  41351769764: arm64 6.88 -> 6.85 s, x64 6.95 -> 7.08 s).
- Diversifying the helpers beyond their seeds (move types, LNS vs deep
  dive), a 3-way root race (dual, primal, IPX) and parallel strong
  branching or separation: not tried; the measurements above show the
  idle cores, not these, as the waste.
