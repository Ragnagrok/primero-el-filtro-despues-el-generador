// Without std, the clock is still one C call away. #![forbid(unsafe_code)] closes it.
// expects: usage of an `unsafe extern` block
unsafe extern "C" {
    fn time(t: *mut i64) -> i64;
}

pub fn now() -> i64 {
    unsafe { time(core::ptr::null_mut()) }
}
