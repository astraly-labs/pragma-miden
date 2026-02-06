#!/bin/bash

set -e

NETWORK="${NETWORK:-testnet}"
DEFAULT_PAIRS=("BTC/USD" "ETH/USD" "SOL/USD" "BNB/USD" "XRP/USD" "ADA/USD" "AVAX/USD")

RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
MAGENTA='\033[0;35m'
BOLD='\033[1m'
NC='\033[0m'

ROOT_DIR=$(pwd)

if [ "$NETWORK" = "local" ]; then
    ORACLE_DIR="${ROOT_DIR}/local-node"
else
    ORACLE_DIR="${ROOT_DIR}/.demo-workspaces/oracle"
fi

MODE="${1:-interactive}"
PAIR_ARG="${2:-all}"

if [[ "$PAIR_ARG" == "all" ]]; then
    PAIRS=("${DEFAULT_PAIRS[@]}")
else
    PAIRS=("$PAIR_ARG")
fi

if [ "$NETWORK" = "local" ]; then
    if [ ! -f "$ORACLE_DIR/pragma_miden.json" ]; then
        echo -e "${RED}Error: Local oracle not initialized!${NC}"
        echo -e "${YELLOW}Run the init script first:${NC}"
        echo -e "  ${CYAN}./init-local.sh${NC}"
        exit 1
    fi
else
    if [ ! -d "$ORACLE_DIR/local-node" ] || [ ! -f "$ORACLE_DIR/pragma_miden.json" ]; then
        echo -e "${RED}Error: Oracle workspace not found!${NC}"
        echo -e "${YELLOW}Run the setup first:${NC}"
        echo -e "  ${CYAN}./demo-live.sh${NC} (it will create accounts and exit)"
        exit 1
    fi
fi

ORACLE_ID=$(jq -r ".networks.$NETWORK.oracle_account_id" "$ORACLE_DIR/pragma_miden.json")

fetch_median() {
    local pair=$1
    cd "$ORACLE_DIR"
    
    local max_retries=2
    local retry=0
    
    while [ $retry -le $max_retries ]; do
        local start=$(date +%s.%N)
        local output=$(timeout 30 "${ROOT_DIR}/target/release/pm-oracle-cli" median "$pair" --network $NETWORK 2>&1)
        local exit_code=$?
        local end=$(date +%s.%N)
        local duration=$(echo "$end - $start" | bc)
        
        if [ $exit_code -eq 0 ]; then
            local median_value=$(echo "$output" | grep -oE 'Median value: [0-9]+' | awk '{print $3}')
            if [[ -n "$median_value" ]]; then
                local median_display=$(echo "scale=2; $median_value / 1000000" | bc)
                if [ $retry -gt 0 ]; then
                    echo -e "  ${CYAN}$pair${NC} → ${BOLD}${GREEN}\$$median_display${NC} ${CYAN}(${duration}s, retry #${retry})${NC}"
                else
                    echo -e "  ${CYAN}$pair${NC} → ${BOLD}${GREEN}\$$median_display${NC} ${CYAN}(${duration}s)${NC}"
                fi
                return 0
            fi
        fi
        
        retry=$((retry + 1))
        if [ $retry -le $max_retries ]; then
            echo -e "  ${CYAN}$pair${NC} → ${YELLOW}Retrying...${NC} ($retry/$max_retries)"
            sleep 1
        else
            echo -e "  ${CYAN}$pair${NC} → ${RED}Failed after $max_retries retries${NC}"
            if echo "$output" | grep -q "Merkle store"; then
                echo -e "    ${YELLOW}Reason:${NC} RPC issue"
            else
                local err=$(echo "$output" | grep -i "error" | head -1 | cut -c1-60)
                if [[ -n "$err" ]]; then
                    echo -e "    ${YELLOW}Error:${NC} $err"
                fi
            fi
            return 1
        fi
    done
}

fetch_all_medians() {
    for pair in "${PAIRS[@]}"; do
        fetch_median "$pair"
    done
}

echo -e "${CYAN}╔══════════════════════════════════════════════════════════════════╗${NC}"
echo -e "${CYAN}║${NC}  ${BOLD}${MAGENTA}🔮 Pragma Oracle Explorer${NC}                                     ${CYAN}║${NC}"
echo -e "${CYAN}╚══════════════════════════════════════════════════════════════════╝${NC}\n"

echo -e "${GREEN}🌐 Network: ${BOLD}${CYAN}${NETWORK}${NC}"
echo -e "${GREEN}Oracle ID: ${CYAN}$ORACLE_ID${NC}"
if [[ "${#PAIRS[@]}" -eq 1 ]]; then
    echo -e "${GREEN}Pair: ${CYAN}${PAIRS[0]}${NC}\n"
else
    echo -e "${GREEN}Pairs: ${CYAN}${PAIRS[*]}${NC}\n"
fi

if [ "$MODE" = "auto" ]; then
    INTERVAL="${3:-10}"
    echo -e "${YELLOW}Running in AUTO mode - fetching median every ${INTERVAL}s${NC}"
    echo -e "${YELLOW}Press Ctrl+C to stop${NC}\n"
    
    iteration=0
    while true; do
        iteration=$((iteration + 1))
        echo -e "${BOLD}[#$iteration]${NC} $(date +%H:%M:%S)"
        fetch_all_medians
        echo ""
        sleep $INTERVAL
    done
else
    echo -e "${YELLOW}Running in INTERACTIVE mode${NC}"
    echo -e "${CYAN}Press Enter to fetch median, or 'q' + Enter to quit${NC}\n"
    
    iteration=0
    while true; do
        read -r input
        
        if [[ "$input" == "q" ]]; then
            echo -e "${GREEN}Goodbye!${NC}"
            exit 0
        fi
        
        iteration=$((iteration + 1))
        echo -e "${BOLD}[#$iteration]${NC} $(date +%H:%M:%S)"
        fetch_all_medians
        echo ""
    done
fi
