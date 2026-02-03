#!/bin/bash

set -e

NETWORK="testnet"
PAIRS=("BTC/USD" "ETH/USD" "SOL/USD")
DECIMALS=6
PUBLISH_INTERVAL=15
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
    echo -e "\n${YELLOW}🛑 Stopping publishers...${NC}"
    kill $(jobs -p) 2>/dev/null || true
    echo -e "${GREEN}✓ Publishers stopped!${NC}"
    exit 0
}

trap cleanup SIGINT SIGTERM

fetch_source1_price() {
    local pair=$1
    local symbol=$(get_binance_symbol "$pair")
    curl -s "https://api.binance.com/api/v3/ticker/price?symbol=$symbol" | jq -r '.price' 2>/dev/null || echo "0"
}

fetch_source2_price() {
    local pair=$1
    local symbol=$(get_bybit_symbol "$pair")
    curl -s "https://api.bybit.com/v5/market/tickers?category=spot&symbol=$symbol" | jq -r '.result.list[0].lastPrice' 2>/dev/null || echo "0"
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

echo -e "${CYAN}╔══════════════════════════════════════════════════════════════════╗${NC}"
echo -e "${CYAN}║${NC}  ${BOLD}${MAGENTA}📊 Pragma Miden - Live Publishers Feed${NC}                       ${CYAN}║${NC}"
echo -e "${CYAN}╚══════════════════════════════════════════════════════════════════╝${NC}\n"

echo -e "${GREEN}✅ Using existing publishers:${NC}"
echo -e "${CYAN}   Publisher 1: $PUBLISHER_ADDRESS_1${NC}"
echo -e "${CYAN}   Publisher 2: $PUBLISHER_ADDRESS_2${NC}\n"

echo -e "${YELLOW}🔄 Syncing publishers...${NC}"
cd "$PUBLISHER1_DIR"
"${ROOT_DIR}/target/release/pm-publisher-cli" sync --network $NETWORK > /dev/null 2>&1 || true
cd "$PUBLISHER2_DIR"
"${ROOT_DIR}/target/release/pm-publisher-cli" sync --network $NETWORK > /dev/null 2>&1 || true
echo -e "${GREEN}✓ Sync complete${NC}\n"

sleep 1
clear

publisher1_loop() {
    cd "$PUBLISHER1_DIR"
    local iteration=0
    
    while true; do
        iteration=$((iteration + 1))
        
        echo -e "${BOLD}${BLUE}[SOURCE1]${NC} #$iteration - $(date +%H:%M:%S)"
        
        local pair_count=0
        local total_pairs=${#PAIRS[@]}
        
        for pair in "${PAIRS[@]}"; do
            pair_count=$((pair_count + 1))
            timestamp=$(date +%s)
            price=$(fetch_source1_price "$pair")
            
            if [[ "$price" != "0" && "$price" != "null" && -n "$price" ]]; then
                price_int=$(printf '%.0f' $(echo "$price * 1000000" | bc 2>/dev/null))
                price_display=$(printf "%.2f" "$price")
                
                echo -e "  ${CYAN}$pair${NC} → ${GREEN}\$$price_display${NC}"
                
                if "${ROOT_DIR}/target/release/pm-publisher-cli" publish "$pair" $price_int $DECIMALS $timestamp --network $NETWORK --publisher-id "$PUBLISHER_ADDRESS_1" > /dev/null 2>&1; then
                    echo -e "    ${GREEN}✓${NC} Published"
                else
                    echo -e "    ${RED}✗${NC} Failed"
                fi
                
                if [[ $pair_count -lt $total_pairs ]]; then
                    sleep $PAIR_DELAY
                fi
            else
                echo -e "  ${CYAN}$pair${NC} → ${RED}Failed to fetch${NC}"
            fi
        done
        
        echo ""
        sleep $PUBLISH_INTERVAL
    done
}

publisher2_loop() {
    cd "$PUBLISHER2_DIR"
    
    if [[ $PUBLISHER2_STAGGER -gt 0 ]]; then
        echo -e "${BOLD}${MAGENTA}[SOURCE2]${NC} Waiting ${PUBLISHER2_STAGGER}s for stagger..."
        sleep $PUBLISHER2_STAGGER
    fi
    
    local iteration=0
    
    while true; do
        iteration=$((iteration + 1))
        
        echo -e "${BOLD}${MAGENTA}[SOURCE2]${NC} #$iteration - $(date +%H:%M:%S)"
        
        local pair_count=0
        local total_pairs=${#PAIRS[@]}
        
        for pair in "${PAIRS[@]}"; do
            pair_count=$((pair_count + 1))
            timestamp=$(date +%s)
            price=$(fetch_source2_price "$pair")
            
            if [[ "$price" != "0" && "$price" != "null" && -n "$price" ]]; then
                price_int=$(printf '%.0f' $(echo "$price * 1000000" | bc 2>/dev/null))
                price_display=$(printf "%.2f" "$price")
                
                echo -e "  ${CYAN}$pair${NC} → ${GREEN}\$$price_display${NC}"
                
                if "${ROOT_DIR}/target/release/pm-publisher-cli" publish "$pair" $price_int $DECIMALS $timestamp --network $NETWORK --publisher-id "$PUBLISHER_ADDRESS_2" > /dev/null 2>&1; then
                    echo -e "    ${GREEN}✓${NC} Published"
                else
                    echo -e "    ${RED}✗${NC} Failed"
                fi
                
                if [[ $pair_count -lt $total_pairs ]]; then
                    sleep $PAIR_DELAY
                fi
            else
                echo -e "  ${CYAN}$pair${NC} → ${RED}Failed to fetch${NC}"
            fi
        done
        
        echo ""
        sleep $PUBLISH_INTERVAL
    done
}

echo -e "${CYAN}╔══════════════════════════════════════════════════════════════════╗${NC}"
echo -e "${CYAN}║${NC}  ${BOLD}LIVE PUBLISHERS FEED - Press Ctrl+C to stop${NC}                   ${CYAN}║${NC}"
echo -e "${CYAN}║${NC}  ${YELLOW}3 pairs with 5s delay (1 block time)${NC}                      ${CYAN}║${NC}"
echo -e "${CYAN}║${NC}  ${YELLOW}Publisher2 staggered by 9s for smooth updates${NC}               ${CYAN}║${NC}"
echo -e "${CYAN}║${NC}  ${YELLOW}Cycle: ~15s, continuous updates every 3-5s${NC}                 ${CYAN}║${NC}"
echo -e "${CYAN}╚══════════════════════════════════════════════════════════════════╝${NC}\n"

publisher1_loop &
PUB1_PID=$!

publisher2_loop &
PUB2_PID=$!

wait
