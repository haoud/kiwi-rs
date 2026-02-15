#!/bin/bash
set -e

# Check if the script is run from the root of the project by looking for files
# that should only be present in the root of the project. This script must be run
# from the root of the project because it relies on relative paths to download
# and build the toolchain in the right directories.
if [ ! -e ./README.md ]; then
	echo "This script must be run from the root of the project" 1>&2;
	echo "Try `./scripts/toolchain.sh` from the root of the project" 1>&2;
	exit 1;
fi

qemu-system-x86_64 -m 128                               \
    -drive format=raw,media=cdrom,file=bin/kiwi.iso     \
    -device isa-debug-exit                              \
    -serial stdio                                       \
    -no-shutdown                                        \
    -no-reboot                                          \
    -smp 4
