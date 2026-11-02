//! The root LP of MIPLIB's maxgasflow after a round of cuts (rows cut down
//! to 4408), with the basis the MIP's LP relaxation resolved it from. The
//! dual simplex on the unscaled LP ends Unknown with primal infeasibilities
//! of 2.4e-6 in all; the primal phase 1 that Crestline then runs stopped
//! after 0 iterations and called the LP infeasible, which made crest prune
//! the root and declare maxgasflow infeasible (or optimal at a wrong value).
//! The LP is feasible (and optimal for the C++ HiGHS).
use std::io::Read;

#[test]
fn unscaled_phase1_does_not_prove_infeasibility() {
    let data = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/maxgasflow_root_lp");
    let dir = std::env::temp_dir().join(format!("crest-phase1-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut basis = String::new();
    flate2::read::GzDecoder::new(std::fs::File::open(format!("{data}.bas.gz")).unwrap()).read_to_string(&mut basis).unwrap();
    std::fs::write(dir.join("lp.bas"), basis).unwrap();
    std::fs::write(dir.join("opts"), format!("presolve = off\nsolver = simplex\nread_basis_file = {}\n", dir.join("lp.bas").display())).unwrap();
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_crest"))
        .args(["--options_file", dir.join("opts").to_str().unwrap(), &format!("{data}.mps.gz")])
        .output()
        .unwrap();
    let log = String::from_utf8_lossy(&out.stdout);
    std::fs::remove_dir_all(&dir).ok();
    assert!(log.contains("Model status        :"), "{log}");
    assert!(!log.contains("Model status        : Infeasible"), "{log}");
}
