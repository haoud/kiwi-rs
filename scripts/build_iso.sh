#!/bin/sh
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

# Check that we have at least one argument
if [ $# -eq 0 ]; then
    echo "Usage: $0 <path to kernel binary>" 1>&2;
    exit 1;
fi

# Check that limine is installed and build it if necessary
if [ ! -e bin/src/limine/limine-uefi-cd.bin ] || 
   [ ! -e bin/src/limine/limine-bios-cd.bin ] ||
   [ ! -e bin/src/limine/limine-bios.sys ]; then
    echo "Limine is not installed. Downloading and building it..."
    ./scripts/build_limine.sh
fi

# Copy the limine bootloader inside the ISO directory
cp -v                                   \
    bin/src/limine/limine-uefi-cd.bin   \
    bin/src/limine/limine-bios-cd.bin   \
    bin/src/limine/limine-bios.sys      \
    iso/boot/

# Install the kernel. The kernel location is the first argument of this script
cp -v $1 iso/boot/kiwi.elf

# Create the ISO
xorriso -as mkisofs -b boot/limine-bios-cd.bin          \
    -no-emul-boot -boot-load-size 4 -boot-info-table    \
    --efi-boot boot/limine-uefi-cd.bin                  \
    -efi-boot-part --efi-boot-image                     \
    --protective-msdos-label iso -o bin/kiwi.iso

# Deploy Limine to the ISO
./bin/src/limine/limine bios-install bin/kiwi.iso