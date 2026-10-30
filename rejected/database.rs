// The core looks a code up in the database "because it was easier".
// expects: unlinked crate `rusqlite`
pub fn is_sold(id: &str) -> bool {
    let conn = rusqlite::Connection::open("codes.sqlite").unwrap();
    conn.query_row("SELECT status = 'sold' FROM codes WHERE id = ?1", [id], |r| r.get(0))
        .unwrap()
}
