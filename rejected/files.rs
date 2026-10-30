// The core reads the expiry policy from a file. The dependency list can't stop
// this one, std has std::fs: it's #![no_std] that does.
// expects: unlinked crate `std`
pub fn expiry_days() -> u32 {
    std::fs::read_to_string("expiry.txt").unwrap().trim().parse().unwrap()
}
