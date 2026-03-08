#!/usr/bin/env bash
# topgrade-audit.sh — Detect tools NOT covered by topgrade's update pipeline
#
# Topgrade handles dozens of package managers automatically (Homebrew, npm,
# pipx, cargo, etc.), but tools installed via direct binary download, curl
# installers, or git clones have no built-in update path. Over time these
# "orphan" binaries silently fall behind.
#
# This script scans common binary locations and cross-references each tool
# against topgrade.toml [commands], [pre_commands], [post_commands], and
# built-in package manager steps. Any tool without a known update path is
# flagged as a blind spot.
#
# Usage:
#   topgrade-audit.sh                # Full audit with all details
#   topgrade-audit.sh --quiet        # Only show blind spots (for post_commands)
#   topgrade-audit.sh --json         # Machine-readable JSON output
#
# Integration:
#   Add to your topgrade.toml to run after every update cycle:
#
#     [post_commands]
#     "Coverage Audit" = "~/.config/topgrade-audit.sh --quiet || true"
#
# Customization:
#   Set TOPGRADE_AUDIT_DIRS to override which directories to scan:
#     export TOPGRADE_AUDIT_DIRS="$HOME/.local/bin /usr/local/bin"
#
#   Set TOPGRADE_AUDIT_KNOWN to add tool-to-source mappings:
#     export TOPGRADE_AUDIT_KNOWN="mytool=custom anothertool=brew"

set -eo pipefail

TOPGRADE_TOML="${TOPGRADE_TOML:-${HOME}/.config/topgrade.toml}"
QUIET=false
JSON=false

for arg in "$@"; do
    case "$arg" in
        --quiet) QUIET=true ;;
        --json)  JSON=true; QUIET=true ;;
    esac
done

log() {
    if [[ "$QUIET" == false ]]; then
        echo "$@"
    fi
}

# ─── 1. Parse topgrade.toml ──────────────────────────────────────

if [[ ! -f "$TOPGRADE_TOML" ]]; then
    echo "ERROR: topgrade.toml not found at $TOPGRADE_TOML"
    echo "Set TOPGRADE_TOML to the correct path."
    exit 2
fi

# Extract disabled steps
DISABLED_STEPS=""
DISABLED_STEPS=$(grep -E '^disable\s*=' "$TOPGRADE_TOML" 2>/dev/null \
    | sed 's/.*\[//' | sed 's/\]//' | tr -d '"' | tr ',' '\n' | xargs || true)

is_disabled() {
    echo " $DISABLED_STEPS " | grep -q " $1 " 2>/dev/null
}

# Extract custom command names (case-insensitive matching against binary names)
# Filter for lines with '=' to skip triple-quote closing lines (""")
extract_command_names() {
    local section="$1"
    sed -n "/^\\[${section}\\]/,/^\\[/p" "$TOPGRADE_TOML" \
        | grep '^"' | grep '=' \
        | sed -n 's/^"\([^"]*\)".*/\1/p' \
        | tr '[:upper:]' '[:lower:]'
}
CUSTOM_COMMANDS="$(extract_command_names commands || true)
$(extract_command_names pre_commands || true)
$(extract_command_names post_commands || true)"

has_custom_command_for() {
    local name_lower
    name_lower=$(echo "$1" | tr '[:upper:]' '[:lower:]')
    echo "$CUSTOM_COMMANDS" | grep -qi "$name_lower" 2>/dev/null
}

log "══════════════════════════════════════════════════════════"
log "  Topgrade Coverage Audit"
log "══════════════════════════════════════════════════════════"
log ""

# ─── 2. Accumulators ─────────────────────────────────────────────

declare -a GAPS_LIST=()
declare -a COVERED_LIST=()

add_covered() {
    COVERED_LIST+=("$1")
}

add_gap() {
    GAPS_LIST+=("$1")
}

# ─── 3. Classify a tool ──────────────────────────────────────────

classify_tool() {
    local name="$1"
    local location="$2"

    # Check user-provided overrides
    if [[ -n "${TOPGRADE_AUDIT_KNOWN:-}" ]]; then
        for mapping in $TOPGRADE_AUDIT_KNOWN; do
            local key="${mapping%%=*}"
            local val="${mapping##*=}"
            if [[ "$name" == "$key" ]]; then
                add_covered "$name ($location) -- via $val (user override)"
                return
            fi
        done
    fi

    local real_path
    real_path=$(readlink "$location/$name" 2>/dev/null || echo "$location/$name")

    # Symlinks into pipx
    if echo "$real_path" | grep -q "pipx" 2>/dev/null; then
        if is_disabled "pipx"; then
            add_gap "$name ($location) -- pipx step is DISABLED"
        else
            add_covered "$name -- via pipx"
        fi
        return
    fi

    # Symlinks into Homebrew
    if echo "$real_path" | grep -qE "(Cellar|homebrew)" 2>/dev/null; then
        if is_disabled "brew_formula" && is_disabled "brew_cask"; then
            add_gap "$name ($location) -- brew steps are DISABLED"
        else
            add_covered "$name -- via Homebrew"
        fi
        return
    fi

    # Symlinks into npm
    if echo "$real_path" | grep -q "node_modules" 2>/dev/null; then
        if is_disabled "npm"; then
            add_gap "$name ($location) -- npm step is DISABLED"
        else
            add_covered "$name -- via npm"
        fi
        return
    fi

    # Symlinks into cargo
    if echo "$real_path" | grep -q ".cargo" 2>/dev/null; then
        if is_disabled "cargo"; then
            add_gap "$name ($location) -- cargo step is DISABLED"
        else
            add_covered "$name -- via cargo"
        fi
        return
    fi

    # Rustup-managed tools
    if [[ "$real_path" == "rustup" ]] || echo "$real_path" | grep -q "rustup" 2>/dev/null; then
        add_covered "$name -- via rustup"
        return
    fi

    # Check custom commands
    if has_custom_command_for "$name"; then
        add_covered "$name -- via custom command"
        return
    fi

    # Application symlinks (macOS .app bundles manage their own updates)
    if echo "$real_path" | grep -qE "\.app/" 2>/dev/null; then
        add_covered "$name -- via application bundle"
        return
    fi

    # Skip known auxiliary binaries and legacy tools
    case "$name" in
        # Old version backups (e.g. droid-0.46.0)
        *-[0-9]*)         add_covered "$name -- old version backup"; return ;;
        # Nmap sub-binaries ship with nmap
        ncat|ndiff|nping)  add_covered "$name -- ships with nmap"; return ;;
        # Legacy tools superseded by something else
        youtube-dl)        add_covered "$name -- legacy (use yt-dlp)"; return ;;
        # Safe wrappers
        *-safe)            add_covered "$name -- safe wrapper script"; return ;;
    esac

    # If we get here, no update path was found
    add_gap "$name ($location) -- NO UPDATE PATH"
}

# ─── 4. Scan directories ─────────────────────────────────────────

DEFAULT_DIRS="$HOME/.local/bin $HOME/.cargo/bin /usr/local/bin $HOME/.bun/bin"
SCAN_DIRS="${TOPGRADE_AUDIT_DIRS:-$DEFAULT_DIRS}"

for dir in $SCAN_DIRS; do
    if [[ ! -d "$dir" ]]; then
        continue
    fi

    log "-- Scanning $dir --"

    for bin in "$dir"/*; do
        [[ -e "$bin" ]] || continue
        [[ -x "$bin" ]] || continue  # Skip non-executables

        name=$(basename "$bin")

        # Skip hidden files and common non-tools
        [[ "$name" == .* ]] && continue

        classify_tool "$name" "$dir"
    done
done

# ─── 5. Check built-in step warnings ─────────────────────────────

WARNINGS=""
if is_disabled "npm"; then
    WARNINGS="${WARNINGS}npm step is DISABLED — npm globals will NOT be updated\n"
fi
if is_disabled "pipx"; then
    WARNINGS="${WARNINGS}pipx step is DISABLED — pipx packages will NOT be updated\n"
fi
if is_disabled "cargo"; then
    WARNINGS="${WARNINGS}cargo step is DISABLED — cargo binaries will NOT be updated\n"
fi

# ─── 6. Output ────────────────────────────────────────────────────

GAPS_COUNT=${#GAPS_LIST[@]}
COVERED_COUNT=${#COVERED_LIST[@]}

if [[ "$JSON" == true ]]; then
    echo "{"
    echo "  \"timestamp\": \"$(date -u '+%Y-%m-%dT%H:%M:%SZ')\","
    echo "  \"covered_count\": $COVERED_COUNT,"
    echo "  \"gaps_count\": $GAPS_COUNT,"
    echo "  \"gaps\": ["
    for i in "${!GAPS_LIST[@]}"; do
        comma=","
        if [[ $i -eq $((GAPS_COUNT - 1)) ]]; then comma=""; fi
        echo "    \"${GAPS_LIST[$i]}\"${comma}"
    done
    echo "  ],"
    echo "  \"disabled_steps\": \"$DISABLED_STEPS\""
    echo "}"
else
    echo ""
    echo "══════════════════════════════════════════════════════════"
    echo "  AUDIT RESULTS"
    echo "══════════════════════════════════════════════════════════"

    if [[ "$QUIET" == false ]]; then
        echo ""
        echo "-- Covered ($COVERED_COUNT tools) --"
        for item in "${COVERED_LIST[@]}"; do
            echo "  [ok] $item"
        done
    fi

    echo ""
    if [[ $GAPS_COUNT -eq 0 ]]; then
        echo "-- No blind spots detected --"
        echo "  All installed tools have an update path via topgrade."
    else
        echo "-- BLIND SPOTS ($GAPS_COUNT tools without update coverage) --"
        for item in "${GAPS_LIST[@]}"; do
            echo "  [!!] $item"
        done
        echo ""
        echo "  Fix: Add custom commands to topgrade.toml for each tool above."
        echo "  See: config.example.toml for GitHub release update patterns."
    fi

    if [[ -n "$WARNINGS" ]]; then
        echo ""
        echo "-- Warnings --"
        echo -e "$WARNINGS"
    fi

    echo ""
    echo "══════════════════════════════════════════════════════════"
    echo "  Audit complete: $(date '+%Y-%m-%d %H:%M:%S')"
    echo "══════════════════════════════════════════════════════════"
fi

# Exit non-zero if there are gaps (useful for CI / post_commands alerting)
if [[ $GAPS_COUNT -gt 0 ]]; then
    exit 1
fi
