//! The shell does. It reads the code's state from SQLite, asks the core what to
//! do, and carries it out: it writes the new state and, on a redemption, hands
//! over the gift (here, a row in `handovers`).
//!
//! Deciding and doing are two separate calls on purpose, so the tests can
//! interleave two branches step by step instead of relying on timing.

use giftcard_core::{step, Code, Command, Event};
use rusqlite::{params, Connection, Result};

pub const SCHEMA: &str = "
    DROP TABLE IF EXISTS handovers;
    DROP TABLE IF EXISTS codes;
    CREATE TABLE codes (
        id          TEXT PRIMARY KEY,
        status      TEXT NOT NULL CHECK (status IN ('available', 'sold', 'redeemed', 'expired')),
        redeemed_at TEXT
    );
    CREATE TABLE handovers (
        id      INTEGER PRIMARY KEY,
        code_id TEXT NOT NULL REFERENCES codes (id),
        branch  TEXT NOT NULL
    );
";

/// How the shell writes the core's decision back.
#[derive(Clone, Copy, Debug)]
pub enum Write {
    /// Overwrite the state it read a moment ago, whatever it is now.
    Naive,
    /// Only take effect if nobody got there first: for a redemption,
    /// `UPDATE ... WHERE redeemed_at IS NULL`, and check that one row changed.
    Conditional,
}

/// What the core decided, with what it was decided from.
#[derive(Debug)]
pub struct Decision {
    pub code_id: String,
    pub from: Code,
    pub to: Code,
    pub commands: Vec<Command>,
}

pub fn setup(conn: &Connection) -> Result<()> {
    conn.execute_batch(SCHEMA)
}

pub fn insert(conn: &Connection, id: &str, code: Code) -> Result<()> {
    conn.execute("INSERT INTO codes (id, status) VALUES (?1, ?2)", params![id, name(code)])?;
    Ok(())
}

/// Read the world, ask the core. Nothing is written yet.
pub fn decide(conn: &Connection, id: &str, event: Event) -> Result<Decision> {
    let status: String = conn.query_row("SELECT status FROM codes WHERE id = ?1", [id], |r| r.get(0))?;
    let from = parse(&status);
    let (to, commands) = step(from, event);
    Ok(Decision { code_id: id.to_string(), from, to, commands })
}

/// Carry the decision out. Returns whether the gift was handed over.
pub fn commit(conn: &Connection, d: &Decision, branch: &str, write: Write) -> Result<bool> {
    if d.to == d.from {
        return Ok(false); // the core rejected the event: nothing to do
    }
    let changed = match (write, d.to) {
        (Write::Naive, _) => conn.execute(
            "UPDATE codes SET status = ?2,
                 redeemed_at = CASE WHEN ?2 = 'redeemed' THEN datetime('now') ELSE redeemed_at END
             WHERE id = ?1",
            params![d.code_id, name(d.to)],
        )?,
        (Write::Conditional, Code::Redeemed) => conn.execute(
            "UPDATE codes SET status = 'redeemed', redeemed_at = datetime('now')
             WHERE id = ?1 AND redeemed_at IS NULL",
            params![d.code_id],
        )?,
        (Write::Conditional, to) => conn.execute(
            "UPDATE codes SET status = ?2 WHERE id = ?1 AND status = ?3",
            params![d.code_id, name(to), name(d.from)],
        )?,
    };
    if changed == 0 {
        return Ok(false); // somebody else got there first
    }
    let mut handed_over = false;
    for c in &d.commands {
        if *c == Command::Notify(Code::Redeemed) {
            conn.execute(
                "INSERT INTO handovers (code_id, branch) VALUES (?1, ?2)",
                params![d.code_id, branch],
            )?;
            handed_over = true;
        }
    }
    Ok(handed_over)
}

pub fn handovers(conn: &Connection, id: &str) -> Result<i64> {
    conn.query_row("SELECT count(*) FROM handovers WHERE code_id = ?1", [id], |r| r.get(0))
}

fn name(code: Code) -> &'static str {
    match code {
        Code::Available => "available",
        Code::Sold => "sold",
        Code::Redeemed => "redeemed",
        Code::Expired => "expired",
    }
}

fn parse(status: &str) -> Code {
    match status {
        "available" => Code::Available,
        "sold" => Code::Sold,
        "redeemed" => Code::Redeemed,
        "expired" => Code::Expired,
        other => panic!("the CHECK constraint should have rejected status {other:?}"),
    }
}

/// Two branches redeem the same code at the same time: both read, then both
/// write. Returns how many gifts were handed over.
pub fn two_branches(path: &std::path::Path, write: Write) -> Result<i64> {
    let _ = std::fs::remove_file(path);
    let north = Connection::open(path)?;
    let south = Connection::open(path)?;
    setup(&north)?;
    insert(&north, "GC-1", Code::Sold)?;

    let a = decide(&north, "GC-1", Event::Redemption)?; // north reads: sold
    let b = decide(&south, "GC-1", Event::Redemption)?; // south reads: sold
    commit(&north, &a, "north", write)?;
    commit(&south, &b, "south", write)?;

    handovers(&north, "GC-1")
}
