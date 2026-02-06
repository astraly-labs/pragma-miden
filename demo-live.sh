#!/bin/bash

set -e

NETWORK="${NETWORK:-testnet}"
PAIRS=("BTC/USD" "ETH/USD" "SOL/USD" "BNB/USD" "XRP/USD" "HYPE/USD" "POL/USD")
DECIMALS=6
PUBLISH_INTERVAL=5

get_binance_symbol() {
    case "$1" in
        "BTC/USD") echo "BTCUSDT" ;;
        "ETH/USD") echo "ETHUSDT" ;;
        "SOL/USD") echo "SOLUSDT" ;;
        "BNB/USD") echo "BNBUSDT" ;;
        "XRP/USD") echo "XRPUSDT" ;;
        "POL/USD") echo "POLUSDT" ;;
        *) echo "" ;;
    esac
}

get_bybit_symbol() {
    case "$1" in
        "BTC/USD") echo "BTCUSDT" ;;
        "ETH/USD") echo "ETHUSDT" ;;
        "SOL/USD") echo "SOLUSDT" ;;
        "BNB/USD") echo "BNBUSDT" ;;
        "XRP/USD") echo "XRPUSDT" ;;
        "HYPE/USD") echo "HYPEUSDT" ;;
        "POL/USD") echo "POLUSDT" ;;
        *) echo "" ;;
    esac
}

get_coinbase_symbol() {
    case "$1" in
        "BTC/USD") echo "BTC-USD" ;;
        "ETH/USD") echo "ETH-USD" ;;
        "SOL/USD") echo "SOL-USD" ;;
        "BNB/USD") echo "BNB-USD" ;;
        "XRP/USD") echo "XRP-USD" ;;
        "HYPE/USD") echo "HYPE-USD" ;;
        "POL/USD") echo "POL-USD" ;;
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
PUBLISHER3_DIR="${ROOT_DIR}/.demo-workspaces/publisher3"
ORACLE_DIR="${ROOT_DIR}/.demo-workspaces/oracle"

cleanup() {
    echo -e "\n${YELLOW}🛑 Stopping demo...${NC}"
    kill $(jobs -p) 2>/dev/null || true
    echo -e "${GREEN}✓ Demo stopped!${NC}"
    echo -e "${CYAN}ℹ️  Workspaces preserved in .demo-workspaces/ for next run${NC}"
    exit 0
}

trap cleanup SIGINT SIGTERM

fetch_source1_price() {
    local pair=$1
    if [[ "$pair" == "HYPE/USD" ]]; then
        local symbol=$(get_bybit_symbol "$pair")
        curl -s "https://api.bybit.com/v5/market/tickers?category=spot&symbol=$symbol" | jq -r '.result.list[0].lastPrice' 2>/dev/null || echo "0"
    else
        local symbol=$(get_binance_symbol "$pair")
        curl -s "https://api.binance.com/api/v3/ticker/price?symbol=$symbol" | jq -r '.price' 2>/dev/null || echo "0"
    fi
}

fetch_source2_price() {
    local pair=$1
    local symbol=$(get_bybit_symbol "$pair")
    curl -s "https://api.bybit.com/v5/market/tickers?category=spot&symbol=$symbol" | jq -r '.result.list[0].lastPrice' 2>/dev/null || echo "0"
}

fetch_source3_price() {
    local pair=$1
    local symbol=$(get_coinbase_symbol "$pair")
    curl -s "https://api.coinbase.com/v2/prices/${symbol}/spot" | jq -r '.data.amount' 2>/dev/null || echo "0"
}

echo -e "${CYAN}╔══════════════════════════════════════════════════════════════════╗${NC}"
echo -e "${CYAN}║${NC}  ${BOLD}${MAGENTA}🚀 Pragma Miden - Live Price Feed Demo${NC}                        ${CYAN}║${NC}"
echo -e "${CYAN}╚══════════════════════════════════════════════════════════════════╝${NC}\n"
echo -e "${GREEN}🌐 Network: ${BOLD}${CYAN}${NETWORK}${NC}\n"

if [ -d "$ORACLE_DIR/local-node" ] && [ -f "$ORACLE_DIR/pragma_miden.json" ]; then
    echo -e "${GREEN}✅ Found existing workspaces - reusing accounts${NC}"
    
    ORACLE_ID=$(jq -r ".networks.$NETWORK.oracle_account_id" "$ORACLE_DIR/pragma_miden.json")
    PUBLISHER_ADDRESS_1=$(jq -r ".networks.$NETWORK.publisher_account_ids[0]" "$ORACLE_DIR/pragma_miden.json")
    PUBLISHER_ADDRESS_2=$(jq -r ".networks.$NETWORK.publisher_account_ids[1]" "$ORACLE_DIR/pragma_miden.json")
    PUBLISHER_ADDRESS_3=$(jq -r ".networks.$NETWORK.publisher_account_ids[2]" "$ORACLE_DIR/pragma_miden.json")
    
    echo -e "${CYAN}   Oracle: $ORACLE_ID${NC}"
    echo -e "${CYAN}   Publisher 1: $PUBLISHER_ADDRESS_1${NC}"
    echo -e "${CYAN}   Publisher 2: $PUBLISHER_ADDRESS_2${NC}"
    echo -e "${CYAN}   Publisher 3: $PUBLISHER_ADDRESS_3${NC}\n"
    
    cp "$ORACLE_DIR/pragma_miden.json" "${ROOT_DIR}/pragma_miden.json"
    
    echo -e "${YELLOW}🔄 Syncing accounts with blockchain...${NC}"
    cd "$PUBLISHER1_DIR"
    "${ROOT_DIR}/target/release/pm-publisher-cli" sync --network $NETWORK > /dev/null 2>&1 || true
    cd "$PUBLISHER2_DIR"
    "${ROOT_DIR}/target/release/pm-publisher-cli" sync --network $NETWORK > /dev/null 2>&1 || true
    cd "$PUBLISHER3_DIR"
    "${ROOT_DIR}/target/release/pm-publisher-cli" sync --network $NETWORK > /dev/null 2>&1 || true
    cd "$ORACLE_DIR"
    "${ROOT_DIR}/target/release/pm-oracle-cli" sync --network $NETWORK > /dev/null 2>&1 || true
    echo -e "${GREEN}✓ Sync complete${NC}\n"
else
    echo -e "${YELLOW}⚙️  First run - creating new accounts (this will take ~30s)...${NC}\n"
    
    mkdir -p "$PUBLISHER1_DIR/local-node" "$PUBLISHER2_DIR/local-node" "$PUBLISHER3_DIR/local-node" "$ORACLE_DIR/local-node"
    
    echo '[package]' > "$PUBLISHER1_DIR/Cargo.toml"
    echo 'name = "workspace1"' >> "$PUBLISHER1_DIR/Cargo.toml"
    echo '[package]' > "$PUBLISHER2_DIR/Cargo.toml"
    echo 'name = "workspace2"' >> "$PUBLISHER2_DIR/Cargo.toml"
    echo '[package]' > "$PUBLISHER3_DIR/Cargo.toml"
    echo 'name = "workspace3"' >> "$PUBLISHER3_DIR/Cargo.toml"
    echo '[package]' > "$ORACLE_DIR/Cargo.toml"
    echo 'name = "workspace_oracle"' >> "$ORACLE_DIR/Cargo.toml"
    
    cd "$ORACLE_DIR"
    jq "del(.networks.$NETWORK.oracle_account_id, .networks.$NETWORK.publisher_account_ids)" "${ROOT_DIR}/pragma_miden.json" > pragma_miden.json 2>/dev/null || echo '{"networks":{}}' > pragma_miden.json
    
    echo -e "${GREEN}🔮 Creating Oracle account...${NC}"
    "${ROOT_DIR}/target/release/pm-oracle-cli" init --network $NETWORK || exit 1
    cp pragma_miden.json "${ROOT_DIR}/pragma_miden.json"
    
    cd "$PUBLISHER1_DIR"
    cp "${ROOT_DIR}/pragma_miden.json" ./pragma_miden.json
    echo -e "\n${BLUE}📊 Creating Publisher 1 (Source 1)...${NC}"
    "${ROOT_DIR}/target/release/pm-publisher-cli" init --network $NETWORK || exit 1
    cp pragma_miden.json "${ROOT_DIR}/pragma_miden.json"
    
    PUBLISHER_ADDRESS_1=$(jq -r ".networks.$NETWORK.publisher_account_ids[0]" pragma_miden.json)
    cd "$ORACLE_DIR"
    cp "${ROOT_DIR}/pragma_miden.json" ./pragma_miden.json
    echo -e "\n${CYAN}   → Registering Publisher 1 with Oracle...${NC}"
    "${ROOT_DIR}/target/release/pm-oracle-cli" register-publisher "$PUBLISHER_ADDRESS_1" --network $NETWORK || exit 1
    cp pragma_miden.json "${ROOT_DIR}/pragma_miden.json"
    
    cd "$PUBLISHER2_DIR"
    cp "${ROOT_DIR}/pragma_miden.json" ./pragma_miden.json
    echo -e "\n${MAGENTA}📊 Creating Publisher 2 (Source 2)...${NC}"
    "${ROOT_DIR}/target/release/pm-publisher-cli" init --network $NETWORK || exit 1
    cp pragma_miden.json "${ROOT_DIR}/pragma_miden.json"
    
    PUBLISHER_ADDRESS_2=$(jq -r ".networks.$NETWORK.publisher_account_ids[1]" pragma_miden.json)
    cd "$ORACLE_DIR"
    cp "${ROOT_DIR}/pragma_miden.json" ./pragma_miden.json
    echo -e "\n${CYAN}   → Registering Publisher 2 with Oracle...${NC}"
    "${ROOT_DIR}/target/release/pm-oracle-cli" register-publisher "$PUBLISHER_ADDRESS_2" --network $NETWORK || exit 1
    cp pragma_miden.json "${ROOT_DIR}/pragma_miden.json"
    
    cd "$PUBLISHER3_DIR"
    cp "${ROOT_DIR}/pragma_miden.json" ./pragma_miden.json
    echo -e "\n${YELLOW}📊 Creating Publisher 3 (Source 3)...${NC}"
    "${ROOT_DIR}/target/release/pm-publisher-cli" init --network $NETWORK || exit 1
    cp pragma_miden.json "${ROOT_DIR}/pragma_miden.json"
    
    PUBLISHER_ADDRESS_3=$(jq -r ".networks.$NETWORK.publisher_account_ids[2]" pragma_miden.json)
    cd "$ORACLE_DIR"
    cp "${ROOT_DIR}/pragma_miden.json" ./pragma_miden.json
    echo -e "\n${CYAN}   → Registering Publisher 3 with Oracle...${NC}"
    "${ROOT_DIR}/target/release/pm-oracle-cli" register-publisher "$PUBLISHER_ADDRESS_3" --network $NETWORK || exit 1
    cp pragma_miden.json "${ROOT_DIR}/pragma_miden.json"
    
    cp "${ROOT_DIR}/pragma_miden.json" "${PUBLISHER1_DIR}/pragma_miden.json"
    cp "${ROOT_DIR}/pragma_miden.json" "${PUBLISHER2_DIR}/pragma_miden.json"
    cp "${ROOT_DIR}/pragma_miden.json" "${PUBLISHER3_DIR}/pragma_miden.json"
    
    ORACLE_ID=$(jq -r ".networks.$NETWORK.oracle_account_id" "$ORACLE_DIR/pragma_miden.json")
    PUBLISHER_ADDRESS_1=$(jq -r ".networks.$NETWORK.publisher_account_ids[0]" "$ORACLE_DIR/pragma_miden.json")
    PUBLISHER_ADDRESS_2=$(jq -r ".networks.$NETWORK.publisher_account_ids[1]" "$ORACLE_DIR/pragma_miden.json")
    PUBLISHER_ADDRESS_3=$(jq -r ".networks.$NETWORK.publisher_account_ids[2]" "$ORACLE_DIR/pragma_miden.json")
    
    echo -e "\n${GREEN}✓ Accounts created!${NC}"
    echo -e "${YELLOW}📝 First-time setup complete. The accounts need ~30-60s to confirm on testnet.${NC}"
    echo -e "${YELLOW}   Run this script again in a minute for the live demo.${NC}\n"
    exit 0
fi

echo -e "\n${CYAN}╔══════════════════════════════════════════════════════════════════╗${NC}"
echo -e "${CYAN}║${NC}  ${GREEN}✅ Setup Complete! Starting Live Feed...${NC}                        ${CYAN}║${NC}"
echo -e "${CYAN}╚══════════════════════════════════════════════════════════════════╝${NC}\n"

sleep 1
clear

publisher1_loop() {
    cd "$PUBLISHER1_DIR"
    local iteration=0
    
    while true; do
        iteration=$((iteration + 1))
        local start_time=$(date +%s)
        
        echo -e "${BOLD}${BLUE}[SOURCE1]${NC} #$iteration - $(date +%H:%M:%S)"
        
        timestamp=$start_time
        local batch_entries=()
        local temp_dir="/tmp/publisher1_$$"
        mkdir -p "$temp_dir"
        
        for i in "${!PAIRS[@]}"; do
            pair="${PAIRS[$i]}"
            (
                price=$(fetch_source1_price "$pair")
                echo "$pair|$price" > "$temp_dir/$i"
            ) &
        done
        wait
        
        for i in "${!PAIRS[@]}"; do
            if [[ -f "$temp_dir/$i" ]]; then
                IFS='|' read -r pair price < "$temp_dir/$i"
                
                if [[ "$price" != "0" && "$price" != "null" && -n "$price" ]]; then
                    price_int=$(printf '%.0f' $(echo "$price * 1000000" | bc 2>/dev/null))
                    price_display=$(printf "%.2f" "$price")
                    
                    echo -e "  ${CYAN}$pair${NC} → ${GREEN}\$$price_display${NC}"
                    
                    batch_entries+=("${pair}:${price_int}:${DECIMALS}:${timestamp}")
                else
                    echo -e "  ${CYAN}$pair${NC} → ${RED}Failed to fetch${NC}"
                fi
            fi
        done
        
        rm -rf "$temp_dir"
        
        if [[ ${#batch_entries[@]} -gt 0 ]]; then
            if "${ROOT_DIR}/target/release/pm-publisher-cli" publish-batch "${batch_entries[@]}" --network $NETWORK --publisher-id "$PUBLISHER_ADDRESS_1" > /dev/null 2>&1; then
                echo -e "  ${GREEN}✓ Batch published (${#batch_entries[@]} pairs)${NC}"
            else
                echo -e "  ${RED}✗ Batch publish failed${NC}"
            fi
        fi
        
        echo ""
        
        local end_time=$(date +%s)
        local elapsed=$((end_time - start_time))
        local remaining=$((PUBLISH_INTERVAL - elapsed))
        if [[ $remaining -gt 0 ]]; then
            sleep $remaining
        fi
    done
}

publisher2_loop() {
    cd "$PUBLISHER2_DIR"
    local iteration=0
    
    while true; do
        iteration=$((iteration + 1))
        local start_time=$(date +%s)
        
        echo -e "${BOLD}${MAGENTA}[SOURCE2]${NC} #$iteration - $(date +%H:%M:%S)"
        
        timestamp=$start_time
        local batch_entries=()
        local temp_dir="/tmp/publisher2_$$"
        mkdir -p "$temp_dir"
        
        for i in "${!PAIRS[@]}"; do
            pair="${PAIRS[$i]}"
            (
                price=$(fetch_source2_price "$pair")
                echo "$pair|$price" > "$temp_dir/$i"
            ) &
        done
        wait
        
        for i in "${!PAIRS[@]}"; do
            if [[ -f "$temp_dir/$i" ]]; then
                IFS='|' read -r pair price < "$temp_dir/$i"
                
                if [[ "$price" != "0" && "$price" != "null" && -n "$price" ]]; then
                    price_int=$(printf '%.0f' $(echo "$price * 1000000" | bc 2>/dev/null))
                    price_display=$(printf "%.2f" "$price")
                    
                    echo -e "  ${CYAN}$pair${NC} → ${GREEN}\$$price_display${NC}"
                    
                    batch_entries+=("${pair}:${price_int}:${DECIMALS}:${timestamp}")
                else
                    echo -e "  ${CYAN}$pair${NC} → ${RED}Failed to fetch${NC}"
                fi
            fi
        done
        
        rm -rf "$temp_dir"
        
        if [[ ${#batch_entries[@]} -gt 0 ]]; then
            if "${ROOT_DIR}/target/release/pm-publisher-cli" publish-batch "${batch_entries[@]}" --network $NETWORK --publisher-id "$PUBLISHER_ADDRESS_2" > /dev/null 2>&1; then
                echo -e "  ${GREEN}✓ Batch published (${#batch_entries[@]} pairs)${NC}"
            else
                echo -e "  ${RED}✗ Batch publish failed${NC}"
            fi
        fi
        
        echo ""
        
        local end_time=$(date +%s)
        local elapsed=$((end_time - start_time))
        local remaining=$((PUBLISH_INTERVAL - elapsed))
        if [[ $remaining -gt 0 ]]; then
            sleep $remaining
        fi
    done
}

publisher3_loop() {
    cd "$PUBLISHER3_DIR"
    local iteration=0
    
    while true; do
        iteration=$((iteration + 1))
        local start_time=$(date +%s)
        
        echo -e "${BOLD}${YELLOW}[SOURCE3]${NC} #$iteration - $(date +%H:%M:%S)"
        
        timestamp=$start_time
        local batch_entries=()
        local temp_dir="/tmp/publisher3_$$"
        mkdir -p "$temp_dir"
        
        for i in "${!PAIRS[@]}"; do
            pair="${PAIRS[$i]}"
            (
                price=$(fetch_source3_price "$pair")
                echo "$pair|$price" > "$temp_dir/$i"
            ) &
        done
        wait
        
        for i in "${!PAIRS[@]}"; do
            if [[ -f "$temp_dir/$i" ]]; then
                IFS='|' read -r pair price < "$temp_dir/$i"
                
                if [[ "$price" != "0" && "$price" != "null" && -n "$price" ]]; then
                    price_int=$(printf '%.0f' $(echo "$price * 1000000" | bc 2>/dev/null))
                    price_display=$(printf "%.2f" "$price")
                    
                    echo -e "  ${CYAN}$pair${NC} → ${GREEN}\$$price_display${NC}"
                    
                    batch_entries+=("${pair}:${price_int}:${DECIMALS}:${timestamp}")
                else
                    echo -e "  ${CYAN}$pair${NC} → ${RED}Failed to fetch${NC}"
                fi
            fi
        done
        
        rm -rf "$temp_dir"
        
        if [[ ${#batch_entries[@]} -gt 0 ]]; then
            if "${ROOT_DIR}/target/release/pm-publisher-cli" publish-batch "${batch_entries[@]}" --network $NETWORK --publisher-id "$PUBLISHER_ADDRESS_3" > /dev/null 2>&1; then
                echo -e "  ${GREEN}✓ Batch published (${#batch_entries[@]} pairs)${NC}"
            else
                echo -e "  ${RED}✗ Batch publish failed${NC}"
            fi
        fi
        
        echo ""
        
        local end_time=$(date +%s)
        local elapsed=$((end_time - start_time))
        local remaining=$((PUBLISH_INTERVAL - elapsed))
        if [[ $remaining -gt 0 ]]; then
            sleep $remaining
        fi
    done
}

oracle_loop() {
    cd "$ORACLE_DIR"
    sleep 6
    
    local iteration=0
    
    while true; do
        iteration=$((iteration + 1))
        
        echo -e "\n${BOLD}${YELLOW}[ORACLE]${NC} #$iteration - $(date +%H:%M:%S)"
        echo -e "  ${CYAN}→${NC} Calculating median from all publishers...\n"
        
        for pair in "${PAIRS[@]}"; do
            median_output=$(timeout 30 "${ROOT_DIR}/target/release/pm-oracle-cli" median "$pair" --network $NETWORK 2>&1)
            median_exit=$?
            median_value=$(echo "$median_output" | grep -oE 'Median value: [0-9]+' | awk '{print $3}' || echo "")
            
            if [[ -n "$median_value" ]]; then
                median_display=$(echo "scale=2; $median_value / 1000000" | bc)
                echo -e "  ${CYAN}$pair${NC} → Median: ${BOLD}${GREEN}\$$median_display${NC}"
            else
                if [[ $median_exit -eq 124 ]]; then
                    echo -e "  ${CYAN}$pair${NC} → ${RED}Timeout${NC}"
                else
                    error_msg=$(echo "$median_output" | grep -i "error\|panic\|failed\|unable" | head -1 || echo "")
                    if [[ -n "$error_msg" ]]; then
                        echo -e "  ${CYAN}$pair${NC} → ${RED}Error${NC}"
                    else
                        echo -e "  ${CYAN}$pair${NC} → ${YELLOW}Waiting for data${NC}"
                    fi
                fi
            fi
        done
        
        echo -e "\n  ${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}\n"
        
        sleep $PUBLISH_INTERVAL
    done
}

echo -e "${CYAN}╔══════════════════════════════════════════════════════════════════╗${NC}"
echo -e "${CYAN}║${NC}  ${BOLD}LIVE PRICE FEED - Press Ctrl+C to stop${NC}                         ${CYAN}║${NC}"
echo -e "${CYAN}║${NC}  ${YELLOW}3 publishers × 7 pairs every 5s (1 block time)${NC}              ${CYAN}║${NC}"
echo -e "${CYAN}║${NC}  ${YELLOW}Sources: Binance, Bybit, Coinbase${NC}                           ${CYAN}║${NC}"
echo -e "${CYAN}╚══════════════════════════════════════════════════════════════════╝${NC}\n"

publisher1_loop &
PUB1_PID=$!

publisher2_loop &
PUB2_PID=$!

publisher3_loop &
PUB3_PID=$!

oracle_loop &
ORACLE_PID=$!

wait
