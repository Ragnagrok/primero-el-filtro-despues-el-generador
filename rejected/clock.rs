// The core looks at the clock to decide whether a code expired.
// expects: unlinked crate `std`
pub fn expired(deadline_secs: u64) -> bool {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap();
    now.as_secs() > deadline_secs
}
