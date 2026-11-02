//! An integer column fixed at a fractional value within feastol of 0
//! (lb = ub = 4.5e-9, as a fuzz shrinker left it). Once feasibility jump
//! finds an incumbent, the objective propagation computes ub 0 for it:
//! checkChangeBound accepted that, changeBound snapped it back to lb (a
//! no-op) and the propagation loop, counting it as a change, ran forever.
//! C++ HiGHS 1.15.1 loops the same way.
use std::time::{Duration, Instant};

const MODEL: &str = "NAME tiny
ROWS
 N  obj
 G  r0
COLUMNS
    MARKER    'MARKER'                 'INTORG'
    x         obj       1
    MARKER    'MARKER'                 'INTEND'
    y         obj       1              r0        1
    z         obj       2              r0        1
RHS
    RHS       r0        0.6
BOUNDS
 FX BND       x         4.5391e-09
 LO BND       y         0.5
 UP BND       y         10
 UP BND       z         10
ENDATA
";

#[test]
fn objective_propagation_ends_on_a_noop_bound_change() {
    let dir = std::env::temp_dir().join(format!("crest-objprop-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("m.mps"), MODEL).unwrap();
    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_crest"))
        .args(["--presolve", "off", dir.join("m.mps").to_str().unwrap()])
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let start = Instant::now();
    while child.try_wait().unwrap().is_none() {
        if start.elapsed() > Duration::from_secs(30) {
            child.kill().ok();
            panic!("crest still running after 30 s");
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let log = std::io::read_to_string(child.stdout.take().unwrap()).unwrap();
    std::fs::remove_dir_all(&dir).ok();
    assert!(log.contains("  Status            Optimal"), "{log}");
}
