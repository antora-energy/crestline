//! The LP of check/TestIpx.cpp, solved end-to-end (IPM and crossover).

use super::model::UserLp;
use super::{LpSolver, Parameters, STATUS_OPTIMAL, STATUS_SOLVED};

const INF: f64 = f64::INFINITY;

#[test]
fn test_ipx() {
    let obj = [-0.2194, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -0.32, -0.5564, 0.6, -0.48];
    let lb = [0.0; 12];
    let ub = [80.0, 283.303, 283.303, 312.813, 349.187, INF, INF, INF, 57.201, 500.0, 500.501, 357.501];
    let ap = [0, 2, 6, 10, 14, 18, 20, 22, 24, 26, 28, 30, 32];
    let ai = [0, 5, 1, 6, 7, 8, 2, 6, 7, 8, 3, 6, 7, 8, 4, 6, 7, 8, 1, 2, 2, 3, 2, 4, 0, 6, 0, 5, 2, 5, 5, 7];
    let ax = [
        -1.0, 0.301, 1.0, -1.0, 0.301, 1.06, 1.0, -1.0, 0.313, 1.06, 1.0, -1.0, 0.313, 0.96, 1.0, -1.0, 0.326,
        0.86, -1.0, 0.99078, 1.00922, -1.0, 1.01802, -1.0, 1.4, 1.0, 0.109, -1.0, -0.419111, 1.0, 1.4, -1.0,
    ];
    let rhs = [0.0, 80.0, 0.0, 0.0, 0.0, 0.0, 0.0, 44.0, 300.0];
    let constr_type = *b"<<=<<=<<<";

    let mut lps = LpSolver::new();
    lps.set_parameters(Parameters {
        display: 0,
        ..Default::default()
    });
    let lp = UserLp {
        num_constr: 9,
        num_var: 12,
        ap: Some(&ap),
        ai: Some(&ai),
        ax: Some(&ax),
        rhs: Some(&rhs),
        constr_type: Some(&constr_type),
        offset: 0.0,
        obj: Some(&obj),
        lbuser: Some(&lb),
        ubuser: Some(&ub),
    };
    assert_eq!(lps.load_model(&lp), 0);
    assert_eq!(lps.solve(), STATUS_SOLVED);
    let info = lps.get_info();
    assert_eq!(info.status_ipm, STATUS_OPTIMAL);
    assert_eq!(info.status_crossover, STATUS_OPTIMAL);

    let mut x = [0.0; 12];
    let mut xl = [0.0; 12];
    let mut xu = [0.0; 12];
    let mut slack = [0.0; 9];
    assert_eq!(
        lps.get_interior_solution([Some(&mut x), Some(&mut xl), Some(&mut xu), Some(&mut slack), None, None, None]),
        0
    );
    assert!((x[11] - 339.9).abs() < 1.0);
    assert!((xl[11] - 339.94).abs() < 1.0);
    assert!((xu[11] - 17.55).abs() < 1.0);
    assert!((slack[8] - 234.76).abs() < 1.0);

    let mut xb = [0.0; 12];
    let mut vbasis = [0; 12];
    assert_eq!(lps.get_basic_solution(Some(&mut xb), None, None, None, None, Some(&mut vbasis)), 0);
    assert!((xb[11] - 339.9).abs() < 1.0);
}

/// augmenting_path with dead-tree pruning finds the matching of plain
/// cs_augment (the reference below) on random sparse matrices, with some
/// rows excluded, columns in random order
#[test]
fn test_augmenting_path_pruning() {
    use super::sparse_utils::augmenting_path;
    use super::Int;
    fn cs_augment(k: Int, ap: &[Int], ai: &[Int], jmatch: &mut [Int], cheap: &mut [Int], w: &mut [Int]) -> bool {
        // recursive form of cs_augment: same search order
        fn go(j: usize, k: Int, ap: &[Int], ai: &[Int], jmatch: &mut [Int], cheap: &mut [Int], w: &mut [Int]) -> bool {
            w[j] = k;
            let mut p = cheap[j];
            let mut found = false;
            while p < ap[j + 1] && !found {
                found = jmatch[ai[p as usize] as usize] == -1;
                p += 1;
            }
            cheap[j] = p;
            if found {
                jmatch[ai[p as usize - 1] as usize] = j as Int;
                return true;
            }
            for p in ap[j]..ap[j + 1] {
                let i = ai[p as usize] as usize;
                let jm = jmatch[i];
                if jm < -1 || w[jm as usize] == k {
                    continue;
                }
                if go(jm as usize, k, ap, ai, jmatch, cheap, w) {
                    jmatch[i] = j as Int;
                    return true;
                }
            }
            false
        }
        go(k as usize, k, ap, ai, jmatch, cheap, w)
    }
    let mut seed = 12345u64;
    let mut rnd = |n: u64| {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (seed >> 33) % n
    };
    for _ in 0..200 {
        let m = 1 + rnd(40) as usize;
        let n = 1 + rnd(60) as usize;
        let (mut ap, mut ai) = (vec![0 as Int], vec![]);
        for _ in 0..n {
            let mut rows: Vec<Int> = (0..1 + rnd(4)).map(|_| rnd(m as u64) as Int).collect();
            rows.sort();
            rows.dedup();
            ai.extend(rows);
            ap.push(ai.len() as Int);
        }
        let mut jm0: Vec<Int> = (0..m).map(|_| if rnd(8) == 0 { -2 } else { -1 }).collect();
        let mut jm1 = jm0.clone();
        let (mut ch0, mut ch1) = (ap[..n].to_vec(), ap[..n].to_vec());
        let (mut w0, mut marked) = (vec![-1; n], vec![-1; n]);
        let (mut a, mut b, mut c, mut d) = (vec![0; m], vec![0; m + 1], vec![0; m + 1], vec![0; m + 1]);
        let mut cols: Vec<Int> = (0..n as Int).collect();
        for k in (1..n).rev() {
            cols.swap(k, rnd(k as u64 + 1) as usize);
        }
        for &j in &cols {
            let r0 = cs_augment(j, &ap, &ai, &mut jm0, &mut ch0, &mut w0);
            let r1 = augmenting_path(j, &ap, &ai, &mut jm1, &mut ch1, &mut marked, &mut a, &mut b, &mut c, &mut d);
            assert_eq!(r0, r1);
            assert_eq!(jm0, jm1);
        }
    }
}
