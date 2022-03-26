#!/bin/bash
#:
#: name = "opte-xde"
#: variety = "basic"
#: target = "helios"
#: rust_toolchain = "nightly"
#: output_rules = [
#:   "/work/debug/*",
#:   "/work/release/*",
#: ]
#:

set -o errexit
set -o pipefail
set -o xtrace

#
# TGT_BASE allows one to run this more easily in their local
# environment:
#
#   TGT_BASE=/var/tmp ./xde.sh
#
TGT_BASE=${TGT_BASE:=/work}

DBG_SRC=target/x86_64-unknown-unknown/debug
DBG_TGT=$TGT_BASE/debug

REL_SRC=target/x86_64-unknown-unknown/release
REL_TGT=$TGT_BASE/release

mkdir -p $DBG_TGT $REL_TGT

function header {
	echo "# ==== $* ==== #"
}

cargo --version
rustc --version

pushd xde

header "check style"
ptime -m cargo +nightly fmt -- --check

header "analyze"
ptime -m cargo +nightly check

#
# XXX This might be something that should be controlled by buildomat
# itself through the config above. But for now I'll try to do it here.
#
# This is required for the unstable buid-std feature to work.
#
header "install rust-src"
ptime -m rustup component add rust-src --toolchain nightly

header "build xde (debug)"
ptime -m ./build-debug.sh

header "build xde (release)"
ptime -m ./build.sh

#
# Inspect the kernel module for bad relocations in case the old
# codegen issue ever shows its face again.
#
if elfdump $DBG_SRC/xde.dbg | grep GOTPCREL; then
	echo "found GOTPCREL relocation in debug build"
	exit 1
fi

if elfdump $REL_SRC/xde | grep GOTPCREL; then
	echo "found GOTPCREL relocation in release build"
	exit 1
fi

cp $DBG_SRC/xde.dbg $DBG_TGT/
sha256sum $DBG_TGT/xde.dbg > $DBG_TGT/xde.dbg.sha256

cp $REL_SRC/xde $REL_TGT/
sha256sum $REL_TGT/xde > $REL_TGT/xde.sha256