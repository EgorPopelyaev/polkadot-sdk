#!/usr/bin/env bash

# Local semver check script for testing
# Usage: ./scripts/check-semver-local.sh <PR_NUMBER> [BASE_BRANCH]
# Example: ./scripts/check-semver-local.sh 10248 stable2509

set -e

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Parse arguments
PR="${1:-}"
BASE_BRANCH="${2:-master}"

if [ -z "$PR" ]; then
  echo "Usage: $0 <PR_NUMBER> [BASE_BRANCH]"
  echo "Example: $0 10248 stable2503"
  exit 1
fi

echo -e "${BLUE}Running semver check for PR #${PR} (base branch: ${BASE_BRANCH})${NC}"
echo ""

prdoc_file="prdoc/pr_$PR.prdoc"

if [ ! -f "$prdoc_file" ]; then
  echo -e "${RED}Error: prdoc file not found: $prdoc_file${NC}"
  exit 1
fi

echo -e "${BLUE}Found prdoc file: $prdoc_file${NC}"
echo ""

# Set environment variables (adjust these as needed for local testing)
export CARGO_TARGET_DIR=target
export RUSTFLAGS='-A warnings -A missing_docs'
export SKIP_WASM_BUILD=1
export TOOLCHAIN="${TOOLCHAIN:-nightly-2025-05-09}"

# Check if parity-publish is installed
if ! command -v parity-publish &> /dev/null; then
  echo -e "${YELLOW}Warning: parity-publish not found. Skipping parity-publish validation.${NC}"
  echo -e "${YELLOW}To install: cargo install parity-publish@0.10.6 --locked${NC}"
  echo ""
  SKIP_PARITY_PUBLISH=true
else
  SKIP_PARITY_PUBLISH=false
fi

# Always run parity-publish to check for all issues (mismatches and missing crates)
if [ "$SKIP_PARITY_PUBLISH" = false ]; then
  echo -e "${BLUE}Running parity-publish validation...${NC}"
  echo ""
  
  if ! "../parity-publish/target/release/parity-publish" --color always prdoc --since HEAD~2 --validate "$prdoc_file" -v --toolchain "$TOOLCHAIN"; then
    
    # Check if any crate has validate: false to override the failure
    if grep -q "validate:[[:space:]]*false" "$prdoc_file"; then
      echo ""
      echo -e "${BLUE}ℹ️  Found crates with 'validate: false' in prdoc. Semver validation failure is overridden.${NC}"
      echo -e "${YELLOW}⚠️  Please ensure the semver override is justified and documented in the PR description.${NC}"
    else
      # No validate: false found, fail with error message
      echo ""
      echo -e "${RED}👋 Hello developer! The SemVer information that you declared in the prdoc file did not match what the CI detected.${NC}"
      echo ""
      echo "Please check the output above and see the following links for more help:"
      echo "- https://github.com/paritytech/polkadot-sdk/blob/master/docs/contributor/prdoc.md#record-semver-changes"
      echo "- https://forum.polkadot.network/t/psa-polkadot-sdk-to-use-semver"
      echo ""
      echo "Otherwise feel free to ask in the Merge Request or in Matrix chat."
      exit 1
    fi
  else
    echo -e "${GREEN}✅ Parity-publish validation passed!${NC}"
  fi
  echo ""
fi

# Only enforce SemVer restrictions for backports targeting stable branches
if [[ "$BASE_BRANCH" != stable* && "$BASE_BRANCH" != unstable* ]]; then
    echo -e "${BLUE}ℹ️  Branch '$BASE_BRANCH' is not a (un)stable branch. Skipping SemVer backport-specific enforcements.${NC}"
    echo -e "${GREEN}✅ Check completed successfully!${NC}"
    exit 0
fi

echo -e "${BLUE}🔍 Backport branch detected, checking for disallowed semver changes...${NC}"
echo ""

# Check for minor/patch bumps with validate: false
if grep -qE "bump:[[:space:]]*(minor|patch)" "$prdoc_file"; then
    minor_patch_temp=$(mktemp)
    grep -A1 -E "bump:[[:space:]]*(minor|patch)" "$prdoc_file" > "$minor_patch_temp"

    has_validate_false=false
    while read -r line; do
        if [[ "$line" =~ bump:[[:space:]]*(minor|patch) ]]; then
            read -r next_line
            if [[ "$next_line" =~ validate:[[:space:]]*false ]]; then
                has_validate_false=true
                break
            fi
        fi
    done < "$minor_patch_temp"

    rm -f "$minor_patch_temp"

    if [ "$has_validate_false" = true ]; then
        echo -e "${BLUE}ℹ️  Found minor/patch bumps with validate: false override. Semver validation was skipped for these crates by parity-publish.${NC}"
        echo ""
    fi
fi

# Check if there are any major bumps
if ! grep -q "bump:[[:space:]]*major" "$prdoc_file"; then
    echo -e "${GREEN}✅ All semver changes in backport are valid (minor, patch, or none).${NC}"
    exit 0
fi

# Process each major bump and check the next line
temp_file=$(mktemp)
grep -A1 "bump:[[:space:]]*major" "$prdoc_file" > "$temp_file"

error_found=false
while IFS= read -r line; do
    if [[ "$line" =~ bump:[[:space:]]*major ]]; then
        # This is the bump line, read the next line
        if IFS= read -r next_line; then
            if [[ "$next_line" =~ validate:[[:space:]]*false ]]; then
                continue  # This major bump is properly validated
            else
                error_found=true
                break
            fi
        else
            # No next line, means no validate: false
            error_found=true
            break
        fi
    fi
done < "$temp_file"

rm -f "$temp_file"

if [ "$error_found" = true ]; then
    echo -e "${RED}❌ Error: Found major bump without 'validate: false'${NC}"
    echo -e "${BLUE}📘 See: https://github.com/paritytech/polkadot-sdk/blob/master/docs/contributor/prdoc.md#backporting-prs${NC}"
    echo -e "${YELLOW}🔧 Add 'validate: false' after the major bump in $prdoc_file with justification.${NC}"
    exit 1
fi

# If we reach here, all major bumps have validate: false
echo -e "${YELLOW}⚠️  Backport contains major bumps, but they are all marked with validate: false.${NC}"
echo -e "${GREEN}✅ Semver override accepted. Please ensure justification is documented in the PR description.${NC}"
