#!/bin/sh
# Builds the release binary natively and for WASI, runs every example
# through both, and checks that they print exactly the same thing and end
# with the same exit status.
#
# Needs: rustup target add wasm32-wasip1, and wasmtime on the PATH.
set -eu
cd "$(dirname "$0")/.."

cargo build --release --quiet
cargo build --release --quiet --target wasm32-wasip1
native=target/release/cpl
wasm=target/wasm32-wasip1/release/cpl.wasm

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

total=0
failed=0
for program in examples/*.cpl; do
    input="${program%.cpl}.in"
    [ -f "$input" ] || input=/dev/null
    n=0
    "$native" "$program" < "$input" > "$work/native.out" 2> "$work/native.err" || n=$?
    w=0
    wasmtime run --dir=. "$wasm" "$program" < "$input" > "$work/wasm.out" 2> "$work/wasm.err" || w=$?
    total=$((total + 1))
    if [ "$n" != "$w" ] || ! cmp -s "$work/native.out" "$work/wasm.out" || ! cmp -s "$work/native.err" "$work/wasm.err"; then
        failed=$((failed + 1))
        echo "DIFFERS: $program"
    fi
done

echo "$total programs compared, $failed differ between native and WASI"
[ "$failed" -eq 0 ]
