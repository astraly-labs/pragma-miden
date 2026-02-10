#!/bin/bash

set -e

NETWORK="${NETWORK:-testnet}"
FAUCET_IDS=("1:0" "2:0" "3:0" "4:0" "5:0" "6:0" "7:0")
DECIMALS=6
PUBLISH_INTERVAL=5

get_binance_symbol() {
    case "$1" in
        "1:0") echo "BTCUSDT" ;;
        "2:0") echo "ETHUSDT" ;;
        "3:0") echo "SOLUSDT" ;;
        "4:0") echo "BNBUSDT" ;;
        "5:0") echo "XRPUSDT" ;;
        "7:0") echo "POLUSDT" ;;
        *) echo "" ;;
    esac
}

get_bybit_symbol() {
    case "$1" in
        "1:0") echo "BTCUSDT" ;;
        "2:0") echo "ETHUSDT" ;;
        "3:0") echo "SOLUSDT" ;;
        "4:0") echo "BNBUSDT" ;;
        "5:0") echo "XRPUSDT" ;;
        "6:0") echo "HYPEUSDT" ;;
        "7:0") echo "POLUSDT" ;;
        *) echo "" ;;
    esac
}

get_coinbase_symbol() {
    case "$1" in
        "1:0") echo "BTC-USD" ;;
        "2:0") echo "ETH-USD" ;;
        "3:0") echo "SOL-USD" ;;
        "4:0") echo "BNB-USD" ;;
        "5:0") echo "XRP-USD" ;;
        "6:0") echo "HYPE-USD" ;;
        "7:0") echo "POL-USD" ;;
        *) echo "" ;;
    esac
}

get_display_name() {
    case "$1" in
        "1:0") echo "BTC/USD" ;;
        "2:0") echo "ETH/USD" ;;
        "3:0") echo "SOL/USD" ;;
        "4:0") echo "BNB/USD" ;;
        "5:0") echo "XRP/USD" ;;
        "6:0") echo "HYPE/USD" ;;
        "7:0") echo "POL/USD" ;;
        *) echo "$1" ;;
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
    echo -e "\n${YELLOW}🛑 Stopping publishers...${NC}"
    kill $(jobs -p) 2>/dev/null || true
    echo -e "${GREEN}✓ Publishers stopped!${NC}"
    exit 0
}

trap cleanup SIGINT SIGTERM

fetch_source1_price() {
    local faucet_id=$1
    if [[ "$faucet_id" == "HYPE/USD" ]]; then
        local symbol=$(get_bybit_symbol "$faucet_id")
        curl -s "https://api.bybit.com/v5/market/tickers?category=spot&symbol=$symbol" | jq -r '.result.list[0].lastPrice' 2>/dev/null || echo "0"
    else
        local symbol=$(get_binance_symbol "$faucet_id")
        curl -s "https://api.binance.com/api/v3/ticker/price?symbol=$symbol" | jq -r '.price' 2>/dev/null || echo "0"
    fi
}

fetch_source2_price() {
    local faucet_id=$1
    local symbol=$(get_bybit_symbol "$faucet_id")
    curl -s "https://api.bybit.com/v5/market/tickers?category=spot&symbol=$symbol" | jq -r '.result.list[0].lastPrice' 2>/dev/null || echo "0"
}

fetch_source3_price() {
    local faucet_id=$1
    local symbol=$(get_coinbase_symbol "$faucet_id")
    curl -s "https://api.coinbase.com/v2/prices/${symbol}/spot" | jq -r '.data.amount' 2>/dev/null || echo "0"
}

if [ ! -d "$ORACLE_DIR/local-node" ] || [ ! -f "$ORACLE_DIR/pragma_miden.json" ]; then
    echo -e "${RED}Error: Workspaces not found!${NC}"
    echo -e "${YELLOW}Run the setup first:${NC}"
    echo -e "  ${CYAN}./demo-live.sh${NC} (it will create accounts and exit)"
    echo -e "  ${CYAN}Wait 1 minute, then run ./demo-live.sh again${NC}"
    exit 1
fi

PUBLISHER_ADDRESS_1=$(jq -r ".networks.$NETWORK.publisher_account_ids[0]" "$ORACLE_DIR/pragma_miden.json")
PUBLISHER_ADDRESS_2=$(jq -r ".networks.$NETWORK.publisher_account_ids[1]" "$ORACLE_DIR/pragma_miden.json")
PUBLISHER_ADDRESS_3=$(jq -r ".networks.$NETWORK.publisher_account_ids[2]" "$ORACLE_DIR/pragma_miden.json")

echo -e "${CYAN}╔══════════════════════════════════════════════════════════════════╗${NC}"
echo -e "${CYAN}║${NC}  ${BOLD}${MAGENTA}📊 Pragma Miden - Live Publishers Feed${NC}                       ${CYAN}║${NC}"
echo -e "${CYAN}╚══════════════════════════════════════════════════════════════════╝${NC}\n"

echo -e "${GREEN}🌐 Network: ${BOLD}${CYAN}${NETWORK}${NC}"
echo -e "${GREEN}✅ Using existing publishers:${NC}"
echo -e "${CYAN}   Publisher 1: $PUBLISHER_ADDRESS_1${NC}"
echo -e "${CYAN}   Publisher 2: $PUBLISHER_ADDRESS_2${NC}"
echo -e "${CYAN}   Publisher 3: $PUBLISHER_ADDRESS_3${NC}\n"

echo -e "${YELLOW}🔄 Syncing publishers...${NC}"
cd "$PUBLISHER1_DIR"
"${ROOT_DIR}/target/release/pm-publisher-cli" sync --network $NETWORK > /dev/null 2>&1 || true
cd "$PUBLISHER2_DIR"
"${ROOT_DIR}/target/release/pm-publisher-cli" sync --network $NETWORK > /dev/null 2>&1 || true
cd "$PUBLISHER3_DIR"
"${ROOT_DIR}/target/release/pm-publisher-cli" sync --network $NETWORK > /dev/null 2>&1 || true
echo -e "${GREEN}✓ Sync complete${NC}\n"

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
        
        for i in "${!FAUCET_IDS[@]}"; do
            faucet_id="${FAUCET_IDS[$i]}"
            (
                price=$(fetch_source1_price "$faucet_id")
                echo "$faucet_id|$price" > "$temp_dir/$i"
            ) &
        done
        wait
        
        for i in "${!FAUCET_IDS[@]}"; do
            if [[ -f "$temp_dir/$i" ]]; then
                IFS='|' read -r faucet_id price < "$temp_dir/$i"
                
                if [[ "$price" != "0" && "$price" != "null" && -n "$price" ]]; then
                    price_int=$(printf '%.0f' $(echo "$price * 1000000" | bc 2>/dev/null))
                    price_display=$(printf "%.2f" "$price")
                    
                    echo -e "  ${CYAN}$(get_display_name "$faucet_id")${NC} → ${GREEN}\$$price_display${NC}"
                    
                    faucet_prefix=$(echo "$faucet_id" | cut -d':' -f1)
                    faucet_suffix=$(echo "$faucet_id" | cut -d':' -f2)
                    batch_entries+=("${faucet_prefix}:${faucet_suffix}:${price_int}:${DECIMALS}:${timestamp}")
                else
                    echo -e "  ${CYAN}$(get_display_name "$faucet_id")${NC} → ${RED}Failed to fetch${NC}"
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
        
        for i in "${!FAUCET_IDS[@]}"; do
            faucet_id="${FAUCET_IDS[$i]}"
            (
                price=$(fetch_source2_price "$faucet_id")
                echo "$faucet_id|$price" > "$temp_dir/$i"
            ) &
        done
        wait
        
        for i in "${!FAUCET_IDS[@]}"; do
            if [[ -f "$temp_dir/$i" ]]; then
                IFS='|' read -r faucet_id price < "$temp_dir/$i"
                
                if [[ "$price" != "0" && "$price" != "null" && -n "$price" ]]; then
                    price_int=$(printf '%.0f' $(echo "$price * 1000000" | bc 2>/dev/null))
                    price_display=$(printf "%.2f" "$price")
                    
                    echo -e "  ${CYAN}$(get_display_name "$faucet_id")${NC} → ${GREEN}\$$price_display${NC}"
                    
                    faucet_prefix=$(echo "$faucet_id" | cut -d':' -f1)
                    faucet_suffix=$(echo "$faucet_id" | cut -d':' -f2)
                    batch_entries+=("${faucet_prefix}:${faucet_suffix}:${price_int}:${DECIMALS}:${timestamp}")
                else
                    echo -e "  ${CYAN}$(get_display_name "$faucet_id")${NC} → ${RED}Failed to fetch${NC}"
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
        
        for i in "${!FAUCET_IDS[@]}"; do
            faucet_id="${FAUCET_IDS[$i]}"
            (
                price=$(fetch_source3_price "$faucet_id")
                echo "$faucet_id|$price" > "$temp_dir/$i"
            ) &
        done
        wait
        
        for i in "${!FAUCET_IDS[@]}"; do
            if [[ -f "$temp_dir/$i" ]]; then
                IFS='|' read -r faucet_id price < "$temp_dir/$i"
                
                if [[ "$price" != "0" && "$price" != "null" && -n "$price" ]]; then
                    price_int=$(printf '%.0f' $(echo "$price * 1000000" | bc 2>/dev/null))
                    price_display=$(printf "%.2f" "$price")
                    
                    echo -e "  ${CYAN}$(get_display_name "$faucet_id")${NC} → ${GREEN}\$$price_display${NC}"
                    
                    faucet_prefix=$(echo "$faucet_id" | cut -d':' -f1)
                    faucet_suffix=$(echo "$faucet_id" | cut -d':' -f2)
                    batch_entries+=("${faucet_prefix}:${faucet_suffix}:${price_int}:${DECIMALS}:${timestamp}")
                else
                    echo -e "  ${CYAN}$(get_display_name "$faucet_id")${NC} → ${RED}Failed to fetch${NC}"
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

echo -e "${CYAN}╔══════════════════════════════════════════════════════════════════╗${NC}"
echo -e "${CYAN}║${NC}  ${BOLD}LIVE PUBLISHERS FEED - Press Ctrl+C to stop${NC}                   ${CYAN}║${NC}"
echo -e "${CYAN}║${NC}  ${YELLOW}3 publishers × 7 pairs every 5s (1 block time)${NC}              ${CYAN}║${NC}"
echo -e "${CYAN}║${NC}  ${YELLOW}Sources: Binance, Bybit, Coinbase${NC}                           ${CYAN}║${NC}"
echo -e "${CYAN}╚══════════════════════════════════════════════════════════════════╝${NC}\n"

publisher1_loop &
PUB1_PID=$!

publisher2_loop &
PUB2_PID=$!

publisher3_loop &
PUB3_PID=$!

wait
