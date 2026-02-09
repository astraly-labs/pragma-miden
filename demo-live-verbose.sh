#!/bin/bash

set -e

NETWORK="${NETWORK:-testnet}"
PAIRS=("BTC/USD" "ETH/USD" "SOL/USD")
DECIMALS=6
PUBLISH_INTERVAL=20
PAIR_DELAY=5
PUBLISHER2_STAGGER=9

get_binance_symbol() {
    case "$1" in
        "BTC/USD") echo "BTCUSDT" ;;
        "ETH/USD") echo "ETHUSDT" ;;
        "SOL/USD") echo "SOLUSDT" ;;
        *) echo "" ;;
    esac
}

get_bybit_symbol() {
    case "$1" in
        "BTC/USD") echo "BTCUSDT" ;;
        "ETH/USD") echo "ETHUSDT" ;;
        "SOL/USD") echo "SOLUSDT" ;;
        *) echo "" ;;
    esac
}

get_faucet_id() {
    case "$1" in
        "BTC/USD") echo "1:0" ;;
        "ETH/USD") echo "2:0" ;;
        "SOL/USD") echo "3:0" ;;
        *) echo "" ;;
    esac
}

RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
MAGENTA='\033[0;35m'
BOLD='\033[1m'
NC='\033[0m'

ROOT_DIR=$(pwd)
PUBLISHER1_DIR="${ROOT_DIR}/.demo-workspaces/publisher1"
PUBLISHER2_DIR="${ROOT_DIR}/.demo-workspaces/publisher2"
ORACLE_DIR="${ROOT_DIR}/.demo-workspaces/oracle"

cleanup() {
    echo -e "\n${YELLOW}🛑 Stopping demo...${NC}"
    kill $(jobs -p) 2>/dev/null || true
    echo -e "${GREEN}✓ Demo stopped!${NC}"
    echo -e "${CYAN}ℹ️  Workspaces preserved in .demo-workspaces/ for next run${NC}"
    exit 0
}

trap cleanup SIGINT SIGTERM

echo -e "${CYAN}╔══════════════════════════════════════════════════════════════════╗${NC}"
echo -e "${CYAN}║${NC}  ${BOLD}${MAGENTA}🚀 Pragma Miden - Live Price Feed Demo (VERBOSE)${NC}             ${CYAN}║${NC}"
echo -e "${CYAN}╚══════════════════════════════════════════════════════════════════╝${NC}\n"

echo -e "${YELLOW}Network: ${BOLD}$NETWORK${NC}\n"

if [ -d "$ORACLE_DIR/$NETWORK-node" ] || [ -d "$ORACLE_DIR/local-node" ]; then
    echo -e "${GREEN}✅ Found existing workspaces${NC}\n"
    
    cd "$ORACLE_DIR"
    ORACLE_ID=$(jq -r ".networks.$NETWORK.oracle_account_id" "pragma_miden.json" 2>/dev/null || echo "null")
    PUBLISHER_ADDRESS_1=$(jq -r ".networks.$NETWORK.publisher_account_ids[0]" "pragma_miden.json" 2>/dev/null || echo "null")
    PUBLISHER_ADDRESS_2=$(jq -r ".networks.$NETWORK.publisher_account_ids[1]" "pragma_miden.json" 2>/dev/null || echo "null")
    
    if [[ "$ORACLE_ID" == "null" || "$PUBLISHER_ADDRESS_1" == "null" || "$PUBLISHER_ADDRESS_2" == "null" ]]; then
        echo -e "${RED}❌ Incomplete configuration detected. Cleaning up...${NC}"
        cd "$ROOT_DIR"
        rm -rf .demo-workspaces/
        echo -e "${YELLOW}Re-run the script to create new accounts.${NC}\n"
        exit 1
    fi
    
    echo -e "${CYAN}   Oracle: $ORACLE_ID${NC}"
    echo -e "${CYAN}   Publisher 1: $PUBLISHER_ADDRESS_1${NC}"
    echo -e "${CYAN}   Publisher 2: $PUBLISHER_ADDRESS_2${NC}\n"
    
    echo -e "${YELLOW}Accounts already exist. To recreate, run:${NC}"
    echo -e "${CYAN}  rm -rf .demo-workspaces/ && ./demo-live-verbose.sh${NC}\n"
    exit 0
else
    echo -e "${YELLOW}⚙️  First run - creating new accounts...${NC}\n"
    
    mkdir -p "$PUBLISHER1_DIR/$NETWORK-node" "$PUBLISHER2_DIR/$NETWORK-node" "$ORACLE_DIR/$NETWORK-node"
    
    echo '[package]' > "$PUBLISHER1_DIR/Cargo.toml"
    echo 'name = "workspace1"' >> "$PUBLISHER1_DIR/Cargo.toml"
    echo '[package]' > "$PUBLISHER2_DIR/Cargo.toml"
    echo 'name = "workspace2"' >> "$PUBLISHER2_DIR/Cargo.toml"
    echo '[package]' > "$ORACLE_DIR/Cargo.toml"
    echo 'name = "workspace_oracle"' >> "$ORACLE_DIR/Cargo.toml"
    
    cd "$ORACLE_DIR"
    echo '{"networks":{}}' > pragma_miden.json
    
    echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${GREEN}🔮 Step 1: Creating Oracle account...${NC}"
    echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}\n"
    
    if "${ROOT_DIR}/target/release/pm-oracle-cli" init --network $NETWORK; then
        echo -e "\n${GREEN}✓ Oracle created successfully${NC}"
        ORACLE_ID=$(jq -r ".networks.$NETWORK.oracle_account_id" pragma_miden.json 2>/dev/null)
        echo -e "${CYAN}   Oracle ID: $ORACLE_ID${NC}\n"
        cp pragma_miden.json "${ROOT_DIR}/pragma_miden.json"
    else
        echo -e "\n${RED}✗ Oracle creation failed${NC}"
        exit 1
    fi
    
    echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${BLUE}📊 Step 2: Creating Publisher 1...${NC}"
    echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}\n"
    
    cd "$PUBLISHER1_DIR"
    cp "${ROOT_DIR}/pragma_miden.json" ./pragma_miden.json
    
    if "${ROOT_DIR}/target/release/pm-publisher-cli" init --network $NETWORK; then
        echo -e "\n${GREEN}✓ Publisher 1 created successfully${NC}"
        PUBLISHER_ADDRESS_1=$(jq -r ".networks.$NETWORK.publisher_account_ids[0]" pragma_miden.json 2>/dev/null)
        echo -e "${CYAN}   Publisher 1 ID: $PUBLISHER_ADDRESS_1${NC}\n"
        cp pragma_miden.json "${ROOT_DIR}/pragma_miden.json"
    else
        echo -e "\n${RED}✗ Publisher 1 creation failed${NC}"
        exit 1
    fi
    
    echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${CYAN}🔗 Step 3: Registering Publisher 1 with Oracle...${NC}"
    echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}\n"
    
    cd "$ORACLE_DIR"
    cp "${ROOT_DIR}/pragma_miden.json" ./pragma_miden.json
    
    if "${ROOT_DIR}/target/release/pm-oracle-cli" register-publisher "$PUBLISHER_ADDRESS_1" --network $NETWORK; then
        echo -e "\n${GREEN}✓ Publisher 1 registered${NC}\n"
        cp pragma_miden.json "${ROOT_DIR}/pragma_miden.json"
    else
        echo -e "\n${RED}✗ Publisher 1 registration failed${NC}"
        exit 1
    fi
    
    echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${MAGENTA}📊 Step 4: Creating Publisher 2...${NC}"
    echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}\n"
    
    cd "$PUBLISHER2_DIR"
    cp "${ROOT_DIR}/pragma_miden.json" ./pragma_miden.json
    
    if "${ROOT_DIR}/target/release/pm-publisher-cli" init --network $NETWORK; then
        echo -e "\n${GREEN}✓ Publisher 2 created successfully${NC}"
        PUBLISHER_ADDRESS_2=$(jq -r ".networks.$NETWORK.publisher_account_ids[1]" pragma_miden.json 2>/dev/null)
        echo -e "${CYAN}   Publisher 2 ID: $PUBLISHER_ADDRESS_2${NC}\n"
        cp pragma_miden.json "${ROOT_DIR}/pragma_miden.json"
    else
        echo -e "\n${RED}✗ Publisher 2 creation failed${NC}"
        exit 1
    fi
    
    echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${CYAN}🔗 Step 5: Registering Publisher 2 with Oracle...${NC}"
    echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}\n"
    
    cd "$ORACLE_DIR"
    cp "${ROOT_DIR}/pragma_miden.json" ./pragma_miden.json
    
    if "${ROOT_DIR}/target/release/pm-oracle-cli" register-publisher "$PUBLISHER_ADDRESS_2" --network $NETWORK; then
        echo -e "\n${GREEN}✓ Publisher 2 registered${NC}\n"
        cp pragma_miden.json "${ROOT_DIR}/pragma_miden.json"
    else
        echo -e "\n${RED}✗ Publisher 2 registration failed${NC}"
        exit 1
    fi
    
    cp "${ROOT_DIR}/pragma_miden.json" "${PUBLISHER1_DIR}/pragma_miden.json"
    cp "${ROOT_DIR}/pragma_miden.json" "${PUBLISHER2_DIR}/pragma_miden.json"
    
    echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${GREEN}${BOLD}✅ ALL ACCOUNTS CREATED SUCCESSFULLY!${NC}"
    echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}\n"
    
    echo -e "${CYAN}📋 Summary:${NC}"
    echo -e "${CYAN}   Oracle: $ORACLE_ID${NC}"
    echo -e "${CYAN}   Publisher 1: $PUBLISHER_ADDRESS_1${NC}"
    echo -e "${CYAN}   Publisher 2: $PUBLISHER_ADDRESS_2${NC}\n"
    
    if [ "$NETWORK" == "testnet" ]; then
        echo -e "${YELLOW}⏱️  Testnet accounts need ~30-60s to confirm.${NC}"
        echo -e "${YELLOW}   Run this script again in a minute to start publishing.${NC}\n"
    else
        echo -e "${GREEN}Ready to start publishing!${NC}"
        echo -e "${CYAN}Run: ./demo-publishers.sh${NC}\n"
    fi
    
    exit 0
fi
