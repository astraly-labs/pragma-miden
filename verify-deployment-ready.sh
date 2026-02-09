#!/bin/bash

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

ERRORS=0
WARNINGS=0

echo -e "${CYAN}╔══════════════════════════════════════════════════════════════════╗${NC}"
echo -e "${CYAN}║${NC}  ${BOLD}Pragma Miden - Pre-Deployment Verification${NC}                    ${CYAN}║${NC}"
echo -e "${CYAN}╚══════════════════════════════════════════════════════════════════╝${NC}\n"

check_pass() {
    echo -e "${GREEN}✓${NC} $1"
}

check_fail() {
    echo -e "${RED}✗${NC} $1"
    ERRORS=$((ERRORS + 1))
}

check_warn() {
    echo -e "${YELLOW}⚠${NC} $1"
    WARNINGS=$((WARNINGS + 1))
}

echo -e "${BOLD}1. Build Verification${NC}"

if cargo check --workspace --quiet 2>/dev/null; then
    check_pass "Workspace compiles without errors"
else
    check_fail "Workspace compilation failed"
fi

if [ -f "target/release/pm-publisher-cli" ]; then
    check_pass "Publisher CLI binary exists"
else
    check_warn "Publisher CLI not built in release mode (run: cargo build --release)"
fi

if [ -f "target/release/pm-oracle-cli" ]; then
    check_pass "Oracle CLI binary exists"
else
    check_warn "Oracle CLI not built in release mode (run: cargo build --release)"
fi

echo ""
echo -e "${BOLD}2. Unit Tests${NC}"

if cargo test --workspace --lib --quiet 2>/dev/null; then
    check_pass "All unit tests pass"
else
    check_fail "Unit tests failed"
fi

echo ""
echo -e "${BOLD}3. MASM Compilation${NC}"

if cargo test -p pm-accounts --lib oracle --quiet 2>/dev/null; then
    check_pass "Oracle MASM compiles successfully"
else
    check_fail "Oracle MASM compilation failed"
fi

echo ""
echo -e "${BOLD}4. Documentation${NC}"

if [ -f "FAUCET_ID_MAPPING.md" ]; then
    check_pass "Faucet ID mapping documentation exists"
else
    check_fail "FAUCET_ID_MAPPING.md not found"
fi

if [ -f "crates/accounts/GET_USD_MEDIAN.md" ]; then
    check_pass "API documentation exists"
else
    check_fail "GET_USD_MEDIAN.md not found"
fi

if [ -f "TESTNET_DEPLOYMENT_GUIDE.md" ]; then
    check_pass "Deployment guide exists"
else
    check_fail "TESTNET_DEPLOYMENT_GUIDE.md not found"
fi

echo ""
echo -e "${BOLD}5. Demo Scripts${NC}"

if grep -q "get_faucet_id" demo-live.sh 2>/dev/null; then
    check_pass "demo-live.sh updated with faucet_id support"
else
    check_fail "demo-live.sh not updated for faucet_id"
fi

if grep -q "get_faucet_id" demo-publishers.sh 2>/dev/null; then
    check_pass "demo-publishers.sh updated with faucet_id support"
else
    check_fail "demo-publishers.sh not updated for faucet_id"
fi

echo ""
echo -e "${BOLD}6. CLI Interface Verification${NC}"

if ./target/release/pm-publisher-cli --help 2>/dev/null | grep -q "faucet_id"; then
    check_pass "Publisher CLI uses faucet_id interface"
else
    check_warn "Publisher CLI help doesn't mention faucet_id"
fi

if ./target/release/pm-oracle-cli --help 2>/dev/null | grep -q "median"; then
    check_pass "Oracle CLI has median command"
else
    check_fail "Oracle CLI missing median command"
fi

echo ""
echo -e "${BOLD}7. Type System${NC}"

if grep -q "impl FromStr for FaucetId" crates/types/src/faucet_id.rs; then
    check_pass "FaucetId::from_str() implemented"
else
    check_fail "FaucetId missing FromStr implementation"
fi

if grep -q "pub struct FaucetEntry" crates/types/src/faucet_entry.rs; then
    check_pass "FaucetEntry type defined"
else
    check_fail "FaucetEntry type missing"
fi

echo ""
echo -e "${BOLD}8. MASM Interface Check${NC}"

if grep -q "export.get_usd_median" crates/accounts/src/oracle/oracle.masm; then
    check_pass "get_usd_median procedure exported"
else
    check_fail "get_usd_median not found in oracle.masm"
fi

if grep -q "export.publish_entry" crates/accounts/src/publisher/publisher.masm; then
    check_pass "publish_entry procedure exported"
else
    check_fail "publish_entry not found in publisher.masm"
fi

echo ""
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"

if [ $ERRORS -eq 0 ] && [ $WARNINGS -eq 0 ]; then
    echo -e "${GREEN}${BOLD}✅ ALL CHECKS PASSED${NC}"
    echo -e "${GREEN}System is ready for testnet deployment!${NC}\n"
    echo -e "${CYAN}Next steps:${NC}"
    echo -e "  1. Review TESTNET_DEPLOYMENT_GUIDE.md"
    echo -e "  2. Run: ${BOLD}cargo build --release${NC}"
    echo -e "  3. Deploy to testnet following the guide\n"
    exit 0
elif [ $ERRORS -eq 0 ]; then
    echo -e "${YELLOW}${BOLD}⚠️  WARNINGS DETECTED (${WARNINGS})${NC}"
    echo -e "${YELLOW}System can be deployed but some optimizations recommended.${NC}\n"
    exit 0
else
    echo -e "${RED}${BOLD}❌ ERRORS DETECTED (${ERRORS})${NC}"
    if [ $WARNINGS -gt 0 ]; then
        echo -e "${YELLOW}Also found ${WARNINGS} warning(s).${NC}"
    fi
    echo -e "${RED}Please fix errors before deployment.${NC}\n"
    exit 1
fi
