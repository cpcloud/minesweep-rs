#!/usr/bin/env nix-shell
#!nix-shell --pure -i bash -p sd
# shellcheck shell=bash

set -euo pipefail

last_release="$1"
next_release="$2"

sd "version\s*=\s*\"${last_release}\"" "version = \"${next_release}\"" Cargo.toml
