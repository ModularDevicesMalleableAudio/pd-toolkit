#!/usr/bin/env bash
# run_tests.sh — Test harness for pd-toolkit
#
# Runs structural validation tests on .pd fixture files
# before the Rust parser exists. Once pdtk is built, this
# script validates that pdtk agrees with expected results.
#
# Usage:
#   ./tests/run_tests.sh              # Run all tests
#   ./tests/run_tests.sh --quick      # Run only handcrafted tests
#   ./tests/run_tests.sh --corpus     # Run only corpus tests
#   ./tests/run_tests.sh --verbose    # Show details for passing tests too

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
FIXTURE_DIR="$SCRIPT_DIR/fixtures"
HANDCRAFTED_DIR="$FIXTURE_DIR/handcrafted"
CORPUS_DIR="$FIXTURE_DIR/corpus"
ABS_DIR="$FIXTURE_DIR/abstractions"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

PASS=0
FAIL=0
SKIP=0
VERBOSE=0
RUN_HANDCRAFTED=1
RUN_CORPUS=1

for arg in "$@"; do
    case $arg in
        --quick) RUN_CORPUS=0 ;;
        --corpus) RUN_HANDCRAFTED=0 ;;
        --verbose) VERBOSE=1 ;;
    esac
done

pass() {
    PASS=$((PASS + 1))
    if [ "$VERBOSE" -eq 1 ]; then
        echo -e "  ${GREEN}PASS${NC}: $1"
    fi
}

fail() {
    FAIL=$((FAIL + 1))
    echo -e "  ${RED}FAIL${NC}: $1"
    if [ -n "${2:-}" ]; then
        echo -e "        $2"
    fi
}

skip() {
    SKIP=$((SKIP + 1))
    if [ "$VERBOSE" -eq 1 ]; then
        echo -e "  ${YELLOW}SKIP${NC}: $1"
    fi
}

section() {
    echo -e "\n${BLUE}▸ $1${NC}"
}


# Count logical entries (lines starting with # that aren't continuations)
count_entries() {
    local file="$1"
    local n
    n=$(grep -c "^#" "$file" 2>/dev/null || true)
    echo "${n:-0}" | tr -d '[:space:]'
}

# Count object-type entries at top level (rough — doesn't handle depth)
count_objects_rough() {
    local file="$1"
    local n
    n=$(grep -c -E "^#X (obj|msg|text|floatatom|symbolatom|restore)" "$file" 2>/dev/null || true)
    echo "${n:-0}" | tr -d '[:space:]'
}

# Count connections
count_connections() {
    local file="$1"
    local n
    n=$(grep -c "^#X connect" "$file" 2>/dev/null || true)
    echo "${n:-0}" | tr -d '[:space:]'
}

# Check if file starts with #N canvas
starts_with_canvas() {
    local file="$1"
    head -1 "$file" | grep -q "^#N canvas" 2>/dev/null
}

# Check all entries end with ;
all_entries_terminated() {
    local file="$1"
    # Join continuation lines, then check each entry ends with ;
    # Simple check: last non-empty line of each entry group should end with ;
    # For now, check that the file doesn't have orphaned content after last ;
    local last_char
    last_char=$(tail -c 2 "$file" | head -c 1)
    # Allow trailing newline
    [[ "$(cat "$file" | sed '/^$/d' | tail -1)" == *";" ]]
}

# Check that #N canvas and #X restore are balanced
check_depth_balance() {
    local file="$1"
    local opens closes
    opens=$(grep -c "^#N canvas" "$file" 2>/dev/null || true)
    opens=$(echo "${opens:-0}" | tr -d '[:space:]')
    closes=$(grep -c "^#X restore" "$file" 2>/dev/null || true)
    closes=$(echo "${closes:-0}" | tr -d '[:space:]')
    # The first #N canvas is the top-level, doesn't get a matching restore
    # So closes should equal opens - 1
    [ "$closes" -eq "$((opens - 1))" ]
}

# Get max connection index referenced
max_connect_index() {
    local file="$1"
    grep "^#X connect" "$file" | \
        sed 's/#X connect //;s/;//' | \
        tr ' ' '\n' | \
        sed -n '1~2p;3~4p' | \
        sort -n | tail -1 2>/dev/null || echo 0
    # This gets src and dst indices (positions 1 and 3 of the 4 fields)
}


test_handcrafted_structural() {
    section "Handcrafted fixtures — structural validation"

    for f in "$HANDCRAFTED_DIR"/*.pd; do
        local name=$(basename "$f")

        # Skip intentionally broken files
        case "$name" in
            malformed_*|empty_file.pd)
                skip "$name (intentionally broken)"
                continue
                ;;
        esac

        # Test 1: File starts with #N canvas
        if starts_with_canvas "$f"; then
            pass "$name — starts with #N canvas"
        else
            fail "$name — does not start with #N canvas"
        fi

        # Test 2: Depth balance (#N canvas vs #X restore)
        if check_depth_balance "$f"; then
            pass "$name — depth balanced"
        else
            local opens=$(grep -c "^#N canvas" "$f" 2>/dev/null || echo 0)
            local closes=$(grep -c "^#X restore" "$f" 2>/dev/null || echo 0)
            fail "$name — depth imbalance: $opens opens, $closes closes (expected $((opens-1)))"
        fi

        # Test 3: Has connections → must have objects
        local conns=$(count_connections "$f")
        local objs=$(count_objects_rough "$f")
        if [ "$conns" -gt 0 ] && [ "$objs" -eq 0 ]; then
            fail "$name — has $conns connections but 0 objects"
        else
            pass "$name — connection/object consistency ($objs objects, $conns connections)"
        fi
    done
}

test_handcrafted_edge_cases() {
    section "Handcrafted fixtures — edge case validation"

    # Test: minimal.pd has exactly 0 objects
    local f="$HANDCRAFTED_DIR/minimal.pd"
    local objs=$(count_objects_rough "$f")
    if [ "$objs" -eq 0 ]; then
        pass "minimal.pd — 0 objects"
    else
        fail "minimal.pd — expected 0 objects, got $objs"
    fi

    # Test: simple_chain.pd has 3 objects, 2 connections
    f="$HANDCRAFTED_DIR/simple_chain.pd"
    objs=$(count_objects_rough "$f")
    local conns=$(count_connections "$f")
    if [ "$objs" -eq 3 ] && [ "$conns" -eq 2 ]; then
        pass "simple_chain.pd — 3 objects, 2 connections"
    else
        fail "simple_chain.pd — expected 3 obj/2 conn, got $objs/$conns"
    fi

    # Test: with_declare.pd has standalone #X declare
    f="$HANDCRAFTED_DIR/with_declare.pd"
    if grep -q "^#X declare" "$f"; then
        pass "with_declare.pd — contains standalone #X declare"
    else
        fail "with_declare.pd — missing standalone #X declare"
    fi

    # Test: with_width_hint.pd has #X f entry
    f="$HANDCRAFTED_DIR/with_width_hint.pd"
    if grep -q "^#X f [0-9]" "$f"; then
        pass "with_width_hint.pd — contains #X f width hint"
    else
        fail "with_width_hint.pd — missing #X f width hint"
    fi

    # Test: with_c_entry.pd has #C entry
    f="$HANDCRAFTED_DIR/with_c_entry.pd"
    if grep -q "^#C" "$f"; then
        pass "with_c_entry.pd — contains #C entry"
    else
        fail "with_c_entry.pd — missing #C entry"
    fi

    # Test: escaped_semicolons.pd has \; in content
    f="$HANDCRAFTED_DIR/escaped_semicolons.pd"
    if grep -q '\\;' "$f"; then
        pass "escaped_semicolons.pd — contains escaped semicolons"
    else
        fail "escaped_semicolons.pd — missing escaped semicolons"
    fi

    # Test: multiline_obj.pd has continuation lines
    f="$HANDCRAFTED_DIR/multiline_obj.pd"
    local non_entry_lines=$(grep -c "^[^#]" "$f" 2>/dev/null || echo 0)
    if [ "$non_entry_lines" -gt 0 ]; then
        pass "multiline_obj.pd — has $non_entry_lines continuation lines"
    else
        fail "multiline_obj.pd — no continuation lines found"
    fi

    # Test: all_gui_types.pd has all expected GUI types
    f="$HANDCRAFTED_DIR/all_gui_types.pd"
    local missing=""
    for gui in tgl bng nbx vsl hsl vradio hradio vu cnv floatatom symbolatom; do
        if ! grep -q "$gui" "$f"; then
            missing="$missing $gui"
        fi
    done
    if [ -z "$missing" ]; then
        pass "all_gui_types.pd — all GUI types present"
    else
        fail "all_gui_types.pd — missing:$missing"
    fi

    # Test: orphans.pd has unconnected objects
    f="$HANDCRAFTED_DIR/orphans.pd"
    objs=$(count_objects_rough "$f")
    conns=$(count_connections "$f")
    if [ "$objs" -gt "$((conns + 1))" ]; then
        pass "orphans.pd — has unconnected objects ($objs objects, $conns connections)"
    else
        fail "orphans.pd — expected more objects than connections imply"
    fi

    # Test: large_patch.pd has 120+ objects
    f="$HANDCRAFTED_DIR/large_patch.pd"
    objs=$(count_objects_rough "$f")
    if [ "$objs" -ge 120 ]; then
        pass "large_patch.pd — has $objs objects (≥120)"
    else
        fail "large_patch.pd — expected ≥120 objects, got $objs"
    fi

    # Test: empty_file.pd is 0 bytes
    f="$HANDCRAFTED_DIR/empty_file.pd"
    local size=$(wc -c < "$f")
    if [ "$size" -eq 0 ]; then
        pass "empty_file.pd — is empty (0 bytes)"
    else
        fail "empty_file.pd — expected 0 bytes, got $size"
    fi

    # Test: malformed_bad_connection.pd references invalid index
    f="$HANDCRAFTED_DIR/malformed_bad_connection.pd"
    objs=$(count_objects_rough "$f")
    local max_idx
    max_idx=$(grep "^#X connect" "$f" | awk '{gsub(/;/,""); print $3; print $5}' | sort -n | tail -1)
    if [ "$max_idx" -ge "$objs" ]; then
        pass "malformed_bad_connection.pd — connection references out-of-range index ($max_idx ≥ $objs)"
    else
        fail "malformed_bad_connection.pd — expected invalid connection index"
    fi
}

test_corpus_structural() {
    section "Corpus fixtures — structural validation"

    for f in "$CORPUS_DIR"/*.pd; do
        local name=$(basename "$f")

        # Test: starts with #N canvas
        if starts_with_canvas "$f"; then
            pass "$name — starts with #N canvas"
        else
            # Some minimal files might be empty-ish
            if [ "$(wc -c < "$f")" -le 2 ]; then
                skip "$name (too small)"
            else
                fail "$name — does not start with #N canvas"
            fi
            continue
        fi

        # Test: depth balance
        if check_depth_balance "$f"; then
            pass "$name — depth balanced"
        else
            local opens=$(grep -c "^#N canvas" "$f" 2>/dev/null || echo 0)
            local closes=$(grep -c "^#X restore" "$f" 2>/dev/null || echo 0)
            fail "$name — depth imbalance: $opens opens, $closes closes"
        fi

        # Test: no empty connections
        local bad_conns
        bad_conns=$(grep "^#X connect" "$f" | grep -c "^#X connect ;" 2>/dev/null || true)
        bad_conns=$(echo "${bad_conns:-0}" | tr -d '[:space:]')
        if [ "$bad_conns" -eq 0 ]; then
            pass "$name — no empty connections"
        else
            fail "$name — $bad_conns empty connection entries"
        fi
    done
}

test_corpus_round_trip_ready() {
    section "Corpus fixtures — round-trip readiness"

    for f in "$CORPUS_DIR"/*.pd; do
        local name=$(basename "$f")
        local size=$(wc -c < "$f")

        if [ "$size" -eq 0 ]; then
            skip "$name (empty)"
            continue
        fi

        # Test: file ends with newline
        local last_byte=$(tail -c 1 "$f" | xxd -p)
        if [ "$last_byte" = "0a" ] || [ "$last_byte" = "" ]; then
            pass "$name — ends with newline"
        else
            fail "$name — does not end with newline (last byte: $last_byte)"
        fi

        # Test: no Windows line endings
        if grep -qP '\r' "$f" 2>/dev/null; then
            fail "$name — contains Windows \\r\\n line endings"
        else
            pass "$name — Unix line endings"
        fi
    done
}

test_fixture_coverage() {
    section "Fixture coverage check"

    local features=(
        "multiline:multiline_obj.pd"
        "escaped_semicolon:escaped_semicolons.pd"
        "standalone_declare:with_declare.pd"
        "width_hint:with_width_hint.pd"
        "c_entry:with_c_entry.pd"
        "graph_subpatch:with_graph.pd"
        "dollar_signs:dollar_signs.pd"
        "gui_types:all_gui_types.pd"
        "send_receive:send_receive.pd"
        "arrays:arrays.pd"
        "orphans:orphans.pd"
        "displays:displays.pd"
        "signal_chain:signal_chain.pd"
        "cycle:cycle.pd"
        "deep_nesting:deep_subpatch.pd"
        "fan_out:branching.pd"
        "fan_in:merging.pd"
        "empty_file:empty_file.pd"
        "malformed_connection:malformed_bad_connection.pd"
        "malformed_semicolon:malformed_missing_semicolon.pd"
        "float_vs_width:float_vs_width.pd"
        "abstractions:../abstractions/uses_abstractions.pd"
    )

    for feature_file in "${features[@]}"; do
        local feature="${feature_file%%:*}"
        local file="${feature_file##*:}"
        local path="$HANDCRAFTED_DIR/$file"
        if [[ "$file" == ../* ]]; then
            path="$FIXTURE_DIR/${file#../}"
        fi

        if [ -f "$path" ]; then
            pass "Feature covered: $feature → $file"
        else
            fail "Feature NOT covered: $feature → $file (file missing)"
        fi
    done
}


test_pdtk_integration() {
    local pdtk="${PDTK:-}"

    # Try to find pdtk
    if [ -z "$pdtk" ]; then
        for candidate in \
            "$SCRIPT_DIR/../target/debug/pdtk" \
            "$SCRIPT_DIR/../target/release/pdtk" \
            "$(which pdtk 2>/dev/null || true)"; do
            if [ -x "$candidate" ]; then
                pdtk="$candidate"
                break
            fi
        done
    fi

    if [ -z "$pdtk" ] || [ ! -x "$pdtk" ]; then
        section "pdtk integration tests"
        skip "pdtk binary not found (set PDTK env var or build with cargo build)"
        return
    fi

    section "pdtk integration tests (binary: $pdtk)"

    # Always-available: binary meta
    if "$pdtk" --version >/dev/null 2>&1; then
        pass "pdtk --version exits 0"
    else
        fail "pdtk --version failed"
    fi

    if "$pdtk" --help >/dev/null 2>&1; then
        pass "pdtk --help exits 0"
    else
        fail "pdtk --help failed"
    fi

    for subcmd in parse validate list; do
        if "$pdtk" "$subcmd" --help >/dev/null 2>&1; then
            pass "pdtk $subcmd --help exits 0"
        else
            fail "pdtk $subcmd --help failed"
        fi
    done

    # Helper: is a subcommand implemented?
    # A command is considered implemented when it no longer returns
    # exit 2 with "not yet implemented" on stderr for a real input.
    # Commands are added below as each implementation step completes.
    cmd_implemented() {
        local subcmd="$1"
        local probe_file="$2"
        local stderr_out
        stderr_out=$("$pdtk" "$subcmd" "$probe_file" 2>&1 >/dev/null || true)
        ! echo "$stderr_out" | grep -q "not yet implemented"
    }

    local probe="$HANDCRAFTED_DIR/simple_chain.pd"

    # parse
    if cmd_implemented "parse" "$probe"; then
        for f in "$HANDCRAFTED_DIR"/*.pd; do
            local name=$(basename "$f")
            case "$name" in malformed_*|empty_file.pd) continue ;; esac
            if "$pdtk" parse "$f" >/dev/null 2>&1; then
                pass "pdtk parse $name"
            else
                fail "pdtk parse $name — exit code $?"
            fi
        done

        # Round-trip: parse --output must produce byte-identical file
        for f in "$HANDCRAFTED_DIR"/*.pd "$CORPUS_DIR"/*.pd; do
            local name=$(basename "$f")
            case "$name" in malformed_*|empty_file.pd) continue ;; esac
            local tmp
            tmp=$(mktemp)
            if "$pdtk" parse "$f" --output "$tmp" 2>/dev/null; then
                if diff -q "$f" "$tmp" >/dev/null 2>&1; then
                    pass "pdtk round-trip $name"
                else
                    fail "pdtk round-trip $name — output differs"
                fi
            fi
            rm -f "$tmp"
        done
    else
        skip "pdtk parse — not yet implemented (step 2.1)"
    fi

    # validate
    if cmd_implemented "validate" "$probe"; then
        for f in "$CORPUS_DIR"/*.pd; do
            local name=$(basename "$f")
            if "$pdtk" validate "$f" >/dev/null 2>&1; then
                pass "pdtk validate $name"
            else
                fail "pdtk validate $name — exit code $?"
            fi
        done
        if ! "$pdtk" validate "$HANDCRAFTED_DIR/malformed_bad_connection.pd" >/dev/null 2>&1; then
            pass "pdtk validate malformed_bad_connection.pd — correctly rejects"
        else
            fail "pdtk validate malformed_bad_connection.pd — should have rejected"
        fi
    else
        skip "pdtk validate — not yet implemented (step 2.3)"
    fi

    # list
    if cmd_implemented "list" "$probe"; then
        if "$pdtk" list "$probe" >/dev/null 2>&1; then
            pass "pdtk list simple_chain.pd"
        else
            fail "pdtk list simple_chain.pd — exit code $?"
        fi
    else
        skip "pdtk list — not yet implemented (step 2.2)"
    fi

    # Add further command blocks here as implementation steps complete.
}

# Main
echo -e "${BLUE}━━━ pd-toolkit test harness ━━━${NC}"
echo "Fixture dir: $FIXTURE_DIR"
echo "Handcrafted: $(ls "$HANDCRAFTED_DIR"/*.pd 2>/dev/null | wc -l) files"
echo "Corpus:      $(ls "$CORPUS_DIR"/*.pd 2>/dev/null | wc -l) files"
echo "Abstractions:$(ls "$ABS_DIR"/*.pd 2>/dev/null | wc -l) files"

if [ "$RUN_HANDCRAFTED" -eq 1 ]; then
    test_handcrafted_structural
    test_handcrafted_edge_cases
fi

if [ "$RUN_CORPUS" -eq 1 ]; then
    test_corpus_structural
    test_corpus_round_trip_ready
fi

test_fixture_coverage
test_pdtk_integration

# Summary
echo ""
echo -e "${BLUE}━━━ Summary ━━━${NC}"
echo -e "  ${GREEN}Passed${NC}: $PASS"
echo -e "  ${RED}Failed${NC}: $FAIL"
echo -e "  ${YELLOW}Skipped${NC}: $SKIP"
echo ""

if [ "$FAIL" -gt 0 ]; then
    echo -e "${RED}TESTS FAILED${NC}"
    exit 1
else
    echo -e "${GREEN}ALL TESTS PASSED${NC}"
    exit 0
fi
