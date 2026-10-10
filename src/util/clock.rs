//! The clock of the run timers (time limits, log times, the P-D integral).
//! Wall time, unless the environment variable `CRESTLINE_INSTR_CLOCK=R` is
//! set (macOS only): then a second is R * 1e9 instructions retired by the
//! process. A benchmarking aid: a one-thread run then stops at the same
//! point however loaded the machine is, so its time-limited result is
//! reproducible.

use std::sync::OnceLock;

/// Seconds, from an arbitrary origin
pub fn now() -> f64 {
    static RATE: OnceLock<f64> = OnceLock::new();
    let rate = *RATE.get_or_init(|| {
        let r = std::env::var("CRESTLINE_INSTR_CLOCK").ok().and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0);
        if cfg!(target_os = "macos") && r > 0.0 { r * 1e9 } else { 0.0 }
    });
    if rate > 0.0 {
        return instructions() / rate;
    }
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0)
}

#[cfg(target_os = "macos")]
fn instructions() -> f64 {
    extern "C" {
        fn proc_pid_rusage(pid: i32, flavor: i32, buffer: *mut u64) -> i32;
    }
    // rusage_info_v4 (RUSAGE_INFO_V4 = 4): 37 u64s, ri_instructions at 31
    let mut buf = [0u64; 37];
    // SAFETY: buf is as large as rusage_info_v4
    unsafe { proc_pid_rusage(std::process::id() as i32, 4, buf.as_mut_ptr()) };
    buf[31] as f64
}

#[cfg(not(target_os = "macos"))]
fn instructions() -> f64 {
    0.0
}
