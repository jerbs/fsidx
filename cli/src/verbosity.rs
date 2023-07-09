static mut VERBOSITY_LEVEL: u8 = 0;

pub fn verbosity() -> bool {
    let v = unsafe { VERBOSITY_LEVEL };
    v > 0
}

pub fn set_verbosity(v: u8) {
    unsafe { VERBOSITY_LEVEL = v; }
}
