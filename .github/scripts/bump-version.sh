#!/usr/bin/env bash
# bump-version.sh — pure version-decision logic for the release workflow.
#
# Given the version currently recorded in a Cargo.toml and a bump kind
# (major/minor/patch), decides whether the release pipeline needs to bump
# the version (the current version is already tagged, i.e. already
# released), can release the current version as-is (a merged PR already
# bumped Cargo.toml to an unreleased version), or should reuse a tag that a
# previous release attempt already created. This is the decision at the
# heart of release.yml's "Version, tag, and release" step, split out so it
# can be tested without pushing to a real repo.
#
# Usage:
#   bump-version.sh <path-to-Cargo.toml> <major|minor|patch>
#
# Must be run from inside the git repository whose tags should be checked
# (release.yml runs it from the repo root right after `actions/checkout`).
#
# Prints exactly one line to stdout:
#   bump <new-version>                 — v<current> is already tagged; bump to <new-version>
#   release <current-version>          — v<current> is not tagged yet; release it as-is
#   existing <version> <tag-target-sha> — v<version> already exists; reuse that tag
#
# Exit codes: 0 on success, 1 on invalid arguments or an unparsable version.

set -euo pipefail

usage() {
    echo "usage: $(basename "$0") <path-to-Cargo.toml> <major|minor|patch>" >&2
}

if [ "$#" -ne 2 ]; then
    usage
    exit 1
fi

CARGO_TOML="$1"
BUMP_KIND="$2"

case "$BUMP_KIND" in
    major | minor | patch) ;;
    *)
        echo "error: bump kind must be one of major, minor, patch (got: '$BUMP_KIND')" >&2
        exit 1
        ;;
esac

if [ ! -f "$CARGO_TOML" ]; then
    echo "error: no such file: $CARGO_TOML" >&2
    exit 1
fi

CURRENT=$(grep '^version' "$CARGO_TOML" | head -1 | sed 's/.*"\(.*\)".*/\1/')

if ! [[ "$CURRENT" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "error: could not parse a semver version from $CARGO_TOML (got: '$CURRENT')" >&2
    exit 1
fi

tag_target() {
    git rev-list -n 1 "refs/tags/v$1"
}

if git rev-parse -q --verify "refs/tags/v$CURRENT" >/dev/null 2>&1; then
    CURRENT_TAG_SHA=$(tag_target "$CURRENT")
    HEAD_SHA=$(git rev-parse HEAD)
    if [ "$CURRENT_TAG_SHA" = "$HEAD_SHA" ]; then
        echo "existing $CURRENT $CURRENT_TAG_SHA"
        exit 0
    fi

    IFS='.' read -r MAJOR MINOR PATCH <<<"$CURRENT"
    case "$BUMP_KIND" in
        major) NEW="$((MAJOR + 1)).0.0" ;;
        minor) NEW="${MAJOR}.$((MINOR + 1)).0" ;;
        patch) NEW="${MAJOR}.${MINOR}.$((PATCH + 1))" ;;
    esac

    if git rev-parse -q --verify "refs/tags/v$NEW" >/dev/null 2>&1; then
        echo "existing $NEW $(tag_target "$NEW")"
    else
        echo "bump $NEW"
    fi
else
    echo "release $CURRENT"
fi
