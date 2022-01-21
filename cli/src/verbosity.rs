static mut VERBOSITY_LEVEL: u32 = 0;

pub fn verbosity() -> bool {
    let v = unsafe { VERBOSITY_LEVEL };
    v > 0
}

pub fn set_verbosity(v: u64) {
    unsafe { VERBOSITY_LEVEL = (v + u64::MAX / 2) as u32; }
}
