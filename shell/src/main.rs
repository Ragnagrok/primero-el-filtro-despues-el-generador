use giftcard_shell::{two_branches, Write};

fn main() -> rusqlite::Result<()> {
    let path = std::env::temp_dir().join("giftcard-race.sqlite");
    for write in [Write::Naive, Write::Conditional] {
        let n = two_branches(&path, write)?;
        println!("{write:?}: two branches redeem the same code, {n} gift(s) handed over");
    }
    Ok(())
}
