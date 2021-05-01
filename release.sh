#!/usr/bin/env nix-shell
#!nix-shell --keep GITHUB_USER --keep GITHUB_TOKEN --pure -i bash -p cacert cargo cargo-release gitAndTools.gh git jq yj util-linux

set -eo pipefail

options="$(getopt -o d --long notes: --long patch-level: --long target: -- "$@")"
eval set -- "$options"

while [ "$1" != "--" ]; do
  case "$1" in
  "--notes")
    shift
    RELEASE_NOTES="$1"
    shift
    ;;
  "--patch-level")
    shift
    PATCH_LEVEL="$1"
    if [[ ! "$PATCH_LEVEL" =~ major|minor|patch ]]; then
      echo >&2 "patch level must be on of major|minor|patch, got $PATCH_LEVEL"
      exit 1
    fi
    shift
    ;;
  "--target")
    shift
    TARGET="$1"
    shift
    ;;
  "-d")
    shift
    DRY_RUN="--dry-run"
    ;;
  "--")
    shift
    break
    ;;
  esac
done

if [ -z "$RELEASE_NOTES" ]; then
  echo >&2 "$0: required argument --notes not provided or empty"
  exit 1
fi

if [ -z "$PATCH_LEVEL" ]; then
  echo >&2 "$0: required argument --patch-level not provided or empty"
  exit 1
fi

cargo release ${DRY_RUN} "$PATCH_LEVEL"

tag="$(yj -tj <Cargo.toml | jq '.package.version' -rcM)"
title="Release $tag"

if [ -z "$TARGET" ]; then
  TARGET="$(git rev-parse HEAD)"
fi

if [ -n "$DRY_RUN" ]; then
  echo gh release create "$tag" --target "$TARGET" --title "$title" --notes "$RELEASE_NOTES"
else
  gh release create "$tag" --target "$TARGET" --title "$title" --notes "$RELEASE_NOTES"
fi
