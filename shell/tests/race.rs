use giftcard_shell::{two_branches, Write};

fn db(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("giftcard-{}-{name}.sqlite", std::process::id()))
}

#[test]
fn naive_write_redeems_twice() {
    // Each branch's core made the right decision with what it knew.
    // The code was redeemed twice anyway.
    assert_eq!(two_branches(&db("naive"), Write::Naive).unwrap(), 2);
}

#[test]
fn conditional_write_lets_only_one_win() {
    assert_eq!(two_branches(&db("conditional"), Write::Conditional).unwrap(), 1);
}
