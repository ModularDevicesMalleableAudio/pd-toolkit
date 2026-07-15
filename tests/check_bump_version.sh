#!/usr/bin/env bash
# check_bump_version.sh — unit tests for .github/scripts/bump-version.sh,
# the version-decision logic used by the release workflow.
#
# Exercises the tag-exists / bump-kind / rerun / malformed-input branches
# against a throwaway git repo so the release pipeline's version arithmetic
# can be verified without pushing to the real repo.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
BUMP_SCRIPT="$REPO_ROOT/.github/scripts/bump-version.sh"

FAIL=0

fail() {
    FAIL=$((FAIL + 1))
    echo "  FAIL: $1" >&2
}

# Sets up a scratch git repo (required so bump-version.sh has tags to check
# against) containing a Cargo.toml at the given version, then runs
# bump-version.sh with the given bump kind and asserts its stdout.
assert_case() {
    local description="$1" version="$2" bump_kind="$3" expected="$4" tag_mode="$5"
    local tmp
    tmp=$(mktemp -d)
    (
        cd "$tmp"
        git init -q
        git config user.email "test@example.com"
        git config user.name "test"
        printf '[package]\nname = "pdtk"\nversion = "%s"\n' "$version" >Cargo.toml
        git add Cargo.toml
        git commit -q -m "init"
        case "$tag_mode" in
            tag-previous)
                git tag "v$version"
                printf 'post-release change\n' >README.md
                git add README.md
                git commit -q -m "post-release change"
                ;;
            tag-head)
                git tag "v$version"
                ;;
            no-tag) ;;
        esac
    )

    if [[ "$expected" == *"{HEAD}"* ]]; then
        local head_sha
        head_sha=$(cd "$tmp" && git rev-parse HEAD)
        expected="${expected/\{HEAD\}/$head_sha}"
    fi

    local actual
    actual=$(cd "$tmp" && "$BUMP_SCRIPT" Cargo.toml "$bump_kind")
    rm -rf "$tmp"

    if [ "$actual" = "$expected" ]; then
        echo "  PASS: $description"
    else
        fail "$description — expected '$expected', got '$actual'"
    fi
}

assert_existing_next_tag_reused() {
    local tmp actual expected tag_sha merge_sha
    tmp=$(mktemp -d)
    (
        cd "$tmp"
        git init -q
        git config user.email "test@example.com"
        git config user.name "test"
        printf '[package]\nname = "pdtk"\nversion = "1.2.3"\n' >Cargo.toml
        git add Cargo.toml
        git commit -q -m "init"
        git tag "v1.2.3"
        printf 'merged feature\n' >README.md
        git add README.md
        git commit -q -m "merged feature"
        merge_sha=$(git rev-parse HEAD)
        printf '[package]\nname = "pdtk"\nversion = "1.2.4"\n' >Cargo.toml
        git add Cargo.toml
        git commit -q -m "release 1.2.4"
        git tag "v1.2.4"
        tag_sha=$(git rev-parse HEAD)
        git checkout -q "$merge_sha"
        printf '%s\n' "$tag_sha" >expected-tag-sha
    )

    tag_sha=$(cat "$tmp/expected-tag-sha")
    expected="existing 1.2.4 $tag_sha"
    actual=$(cd "$tmp" && "$BUMP_SCRIPT" Cargo.toml patch)
    rm -rf "$tmp"

    if [ "$actual" = "$expected" ]; then
        echo "  PASS: rerun reuses existing next-version tag"
    else
        fail "rerun reuses existing next-version tag — expected '$expected', got '$actual'"
    fi
}

echo "bump-version.sh"

# Already-tagged previous release: each bump kind computes the expected next version.
assert_case "already-tagged patch bump" "1.2.3" "patch" "bump 1.2.4" "tag-previous"
assert_case "already-tagged minor bump" "1.2.3" "minor" "bump 1.3.0" "tag-previous"
assert_case "already-tagged major bump" "1.2.3" "major" "bump 2.0.0" "tag-previous"

# Unreleased version (PR already bumped Cargo.toml): release as-is, no bump.
assert_case "unreleased version — release as-is" "1.2.4" "patch" "release 1.2.4" "no-tag"

# Reruns after a tag already exists should reuse that tag instead of bumping again.
assert_case "rerun reuses current-version tag" "1.2.4" "patch" "existing 1.2.4 {HEAD}" "tag-head"
assert_existing_next_tag_reused

# Invalid bump kind must fail rather than silently defaulting.
tmp=$(mktemp -d)
(
    cd "$tmp"
    git init -q
    printf '[package]\nname = "pdtk"\nversion = "1.0.0"\n' >Cargo.toml
)
if (cd "$tmp" && "$BUMP_SCRIPT" Cargo.toml "banana") >/dev/null 2>&1; then
    fail "invalid bump kind should exit non-zero"
else
    echo "  PASS: invalid bump kind rejected"
fi

# Malformed version in Cargo.toml must fail rather than mis-parsing.
printf '[package]\nname = "pdtk"\nversion = "not-a-version"\n' >"$tmp/Cargo.toml"
if (cd "$tmp" && "$BUMP_SCRIPT" Cargo.toml "patch") >/dev/null 2>&1; then
    fail "malformed version should exit non-zero"
else
    echo "  PASS: malformed version rejected"
fi
rm -rf "$tmp"

if [ "$FAIL" -gt 0 ]; then
    echo "bump-version.sh: $FAIL failure(s)" >&2
    exit 1
fi
exit 0
