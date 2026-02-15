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

# Creating required directories if they don't already exist.
mkdir -p bin
mkdir -p bin/src

# Downloading and extracting limine
cd bin/src
echo "Downloading limine..."
git clone https://github.com/limine-bootloader/limine.git \
    --branch=v10.x-binary \
    --depth=1

# Build limine
echo "Building limine..."
make -C limine
