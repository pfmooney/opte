#!/bin/bash
#:
#: name = "opte-api"
#: variety = "basic"
#: target = "helios"
#: rust_toolchain = "nightly"
#: output_rules = []
#:

set -o errexit
set -o pipefail
set -o xtrace

function header {
	echo "# ==== $* ==== #"
}

cargo --version
rustc --version

cd opte-api

header "check API_VERSION"
./check-api-version.sh

header "check style"
ptime -m cargo fmt -- --check

header "analyze std"
ptime -m cargo check

header "analyze no_std"
ptime -m cargo check --no-default-features

header "debug build std"
ptime -m cargo build

header "debug build no_std"
ptime -m cargo build --no-default-features

header "release build std"
ptime -m cargo build --release

header "release build no_std"
ptime -m cargo build --release --no-default-features

header "test"
ptime -m cargo test
