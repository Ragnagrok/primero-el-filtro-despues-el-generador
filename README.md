# First the Filter, Then the Generator

Proof of concept for the article *First the Filter, Then the Generator*
([English](https://blog.ragnagrok.com/en/2026-10-26-primero-el-filtro-despues-el-generador) ·
[español](https://blog.ragnagrok.com/es/2026-10-26-primero-el-filtro-despues-el-generador)):
a gift card code's lifecycle as a **pure core** in Rust, an **imperative shell** that talks to
SQLite, and the one bug neither of them can prevent.

The idea in one line: the core decides, the shell does. The core can't touch the world because
**its crate has nothing to touch it with**.

```rust
fn step(code: Code, event: Event) -> (Code, Vec<Command>)
```

## What's here

| File | What it shows |
|---|---|
| `core/src/lib.rs` | The core: the article's `step`. No dependencies, `#![no_std]`, `#![forbid(unsafe_code)]`. |
| `core/tests/properties.rs` | The article's property with `proptest`: whatever the sequence of events, a code is redeemed at most once. |
| `core-gate.sh` | The gate for CI: fails if the core declares a dependency or loses `no_std` / `forbid(unsafe_code)`. |
| `rejected/*.rs` | Code that tries to reach the world from the core, each with the compiler error it expects on its `// expects:` line. |
| `shell/src/lib.rs` | The shell: reads a code from SQLite, asks the core, writes the decision and hands over the gift. |
| `shell/tests/race.rs` | Two branches redeeming the same code at the same time, step by step. |
| `check.sh` | Checks all of the above, plus that the property catches a broken core. |

The article's code is in Spanish; here it's in English: `Codigo` → `Code`
(`Disponible`, `Vendido`, `Canjeado`, `Vencido` → `Available`, `Sold`, `Redeemed`, `Expired`),
`Evento` → `Event` (`Venta`, `Canje`, `Vencimiento` → `Sale`, `Redemption`, `Expiry`),
`Comando` → `Command` (`Avisar`, `Rechazar` → `Notify`, `Reject`),
`se_canjea_a_lo_sumo_una_vez` → `redeemed_at_most_once`.

## The core can't touch the world

The rejected files, each compiled as a module of the core:

- `database.rs`: the core looks a code up in SQLite "because it was easier".
  ``use of unresolved module or unlinked crate `rusqlite` ``: the crate doesn't depend on it.
- `files.rs`: the core reads the expiry policy from a file.
  ``unlinked crate `std` ``.
- `clock.rs`: the core looks at the clock. Same error.
- `ffi.rs`: without `std`, the clock is still one C call away (`time()` from libc).
  ``usage of an `unsafe extern` block``: `#![forbid(unsafe_code)]` closes that door.

One caveat the first version of the article glossed over, corrected on 2026-10-30 because of this repo: **an empty dependency list isn't enough on its own.**
Rust's standard library comes with `std::fs`, `std::net` and `std::time`, so a core with no
dependencies can still read files, open sockets and look at the clock. `check.sh` shows it:
remove `#![no_std]` and `files.rs` compiles. What closes the door is the combination: no
dependencies, no `std` (only `core` and `alloc`), and no `unsafe`.

And so that nobody (or no agent) quietly undoes it, `core-gate.sh` checks the three things on
every change. It's a short script over `cargo metadata`, not
[cargo-deny](https://github.com/EmbarkStudios/cargo-deny): cargo-deny works with lists of crates to
ban or allow, and the rule here isn't "not these crates" but "nothing at all", which the script
states directly and without installing anything.

## What no language gives you

The double redemption: two branches, at the same time, redeem the same code. Each reads it,
sees it sold and not redeemed, asks its core, and the core correctly says "redeem it". Then
both write.

```
$ cargo run -q -p giftcard-shell
Naive: two branches redeem the same code, 2 gift(s) handed over
Conditional: two branches redeem the same code, 1 gift(s) handed over
```

`Naive` writes back the state it decided on. `Conditional` writes it with
`UPDATE ... WHERE redeemed_at IS NULL` and checks that one row changed, so of the two branches
only one wins. The interleaving is explicit (both read, then both write), not timing-based, so
the test is deterministic.

Rust's guarantees are about what happens inside one running program. This race happens
between two programs, in the database, and that's where it gets fixed.

## Running it

With a stable Rust toolchain and Python 3 (for the gate):

```sh
./check.sh                       # everything below, and the rejected files
cargo test --workspace           # the property and the race
cargo run -q -p giftcard-shell   # the race, printed
./core-gate.sh                   # the gate on its own
```

`check.sh` prints:

```
ok   cargo test: redeemed_at_most_once holds; naive write redeems twice, conditional write once
ok   the gate accepts the core as it is
ok   the gate rejects a dependency: gate: giftcard-core must not depend on anything, found: rusqlite
ok   the gate rejects removing no_std: gate: core/src/lib.rs lost #![no_std]
ok   rejected/clock.rs doesn't compile: unlinked crate `std`
ok   rejected/database.rs doesn't compile: unlinked crate `rusqlite`
ok   rejected/ffi.rs doesn't compile: usage of an `unsafe extern` block
ok   rejected/files.rs doesn't compile: unlinked crate `std`
ok   without #![no_std], rejected/files.rs compiles: an empty dependency list alone doesn't stop file I/O
ok   the property catches the mutant; minimal failing input: events = [ Sale, Redemption, Sale, Redemption]
```

The last line is a check on the check: a mutant core that lets a redeemed code be sold again
should break the property, and `proptest` shrinks the failure to four events.

## What this proof of concept doesn't cover

It's a toy, on purpose:

- **One code, one table.** No balances, batches, expiry dates or settlement, which the article
  lists as part of the real core.
- **The shell is thin and untested beyond the race.** In a real system it's most of the code.
- **The gate is only as good as CI.** Anyone who can edit `core-gate.sh` or the workflow can
  turn it off; the point is that it's a mechanism, reviewed once, instead of a convention.
- **Transactions are another fix.** Wrapping read and write in `BEGIN IMMEDIATE` would also
  serialize the two branches; the conditional update is the one the article describes.

## Further reading

- Bernhardt, G. (2012). [Functional Core, Imperative Shell](https://www.destroyallsoftware.com/screencasts/catalog/functional-core-imperative-shell) and [Boundaries](https://www.destroyallsoftware.com/talks/boundaries).
- Claessen, K. and Hughes, J. (2000). [QuickCheck: A Lightweight Tool for Random Testing of Haskell Programs](https://dl.acm.org/doi/10.1145/351240.351266). `proptest` is its Rust descendant.
- [quinn](https://github.com/quinn-rs/quinn) (QUIC, with a sans-IO core in `quinn-proto`) and [str0m](https://github.com/algesten/str0m) (sans-IO WebRTC): the same split, in production.
