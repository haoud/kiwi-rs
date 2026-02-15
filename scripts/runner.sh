#!/bin/sh
set -e

# Used exclusively by cargo, not intended to be run manually
# This script allow us to run our kernel with cargo run as if it was a normal binary
cd ..

./scripts/build_iso.sh kernel/$1
./scripts/run_iso.sh