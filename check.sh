#!/usr/bin/env bash
# Checks everything the README claims:
#   1. the tests pass: the core's property, and the two-branch race in the shell;
#   2. the gate accepts the core as it is, and rejects it with a dependency added
#      or with #![no_std] removed;
#   3. every file in rejected/ fails to compile inside the core, with the error
#      announced on its "// expects:" line;
#   4. the property has teeth: a mutant core that lets a redeemed code be sold
#      again is caught, and shrunk to a short sequence of events.
set -euo pipefail
cd "$(dirname "$0")"

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
failures=0

cargo test --workspace --quiet >/dev/null 2>&1 || { cargo test --workspace; exit 1; }
echo "ok   cargo test: redeemed_at_most_once holds; naive write redeems twice, conditional write once"

./core-gate.sh >/dev/null
echo "ok   the gate accepts the core as it is"

# A copy of the workspace to break things in.
copy() { rm -rf "$work/ws"; mkdir "$work/ws"; cp -r Cargo.toml Cargo.lock core shell core-gate.sh "$work/ws/"; }

copy
printf '\n[dependencies]\nrusqlite = "0.37"\n' >> "$work/ws/core/Cargo.toml"
if out=$("$work/ws/core-gate.sh" "$work/ws" 2>&1); then
  echo "FAIL the gate accepted a core that depends on rusqlite"; failures=1
else
  echo "ok   the gate rejects a dependency: $out"
fi

copy
sed -i '/^#!\[no_std\]$/d' "$work/ws/core/src/lib.rs"
if out=$("$work/ws/core-gate.sh" "$work/ws" 2>&1); then
  echo "FAIL the gate accepted a core without #![no_std]"; failures=1
else
  echo "ok   the gate rejects removing no_std: $out"
fi

# The core alone, outside the workspace and without its dev-dependencies:
# exactly what the core's own code can reach.
probe() {
  rm -rf "$work/core"; cp -r core "$work/core"; rm -rf "$work/core/tests"
  sed -i '/^\[dev-dependencies\]/,$d' "$work/core/Cargo.toml"
  cp "$1" "$work/core/src/probe.rs"
  echo 'mod probe;' >> "$work/core/src/lib.rs"
  (cd "$work/core" && CARGO_TARGET_DIR="$work/target" cargo build --offline 2>&1)
}

for f in rejected/*.rs; do
  expected=$(sed -n 's|^// expects: ||p' "$f")
  if output=$(probe "$f"); then
    echo "FAIL $f compiled, and it shouldn't"; failures=1
  elif grep -qF -- "$expected" <<<"$output"; then
    echo "ok   $f doesn't compile: $expected"
  else
    echo "FAIL $f doesn't compile, but not for the expected reason:"; echo "$output"; failures=1
  fi
done

# Why no_std matters: with std, reading a file needs no dependency at all.
probe rejected/files.rs >/dev/null || true
sed -i '/^#!\[no_std\]$/d' "$work/core/src/lib.rs"
if (cd "$work/core" && CARGO_TARGET_DIR="$work/target" cargo build --offline -q 2>/dev/null); then
  echo "ok   without #![no_std], rejected/files.rs compiles: an empty dependency list alone doesn't stop file I/O"
else
  echo "FAIL rejected/files.rs should compile once #![no_std] is removed"; failures=1
fi

# The mutant: a redeemed code can be sold again, and so redeemed again.
copy
sed -i 's|^        (c, e) => (c, vec!\[Command::Reject(e)\]),|        (Redeemed, Event::Sale) => (Sold, vec![Command::Notify(Sold)]),\n&|' "$work/ws/core/src/lib.rs"
grep -q '(Redeemed, Event::Sale)' "$work/ws/core/src/lib.rs" || { echo "FAIL the mutant wasn't applied"; exit 1; }
if out=$(cd "$work/ws" && CARGO_TARGET_DIR="$work/target-ws" cargo test -p giftcard-core 2>&1); then
  echo "FAIL the property didn't catch the mutant"; failures=1
else
  minimal=$(awk '/minimal failing input/{f=1} f{printf "%s", $0} f&&/^\]/{exit}' <<<"$out" | tr -s ' ' | sed 's/,]/]/')
  echo "ok   the property catches the mutant; $minimal"
fi

exit $failures
