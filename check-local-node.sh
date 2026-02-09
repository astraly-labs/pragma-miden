#!/bin/bash

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

echo -e "${CYAN}╔══════════════════════════════════════════════════════════════════╗${NC}"
echo -e "${CYAN}║${NC}  ${BOLD}Miden Local Node - Diagnostic${NC}                               ${CYAN}║${NC}"
echo -e "${CYAN}╚══════════════════════════════════════════════════════════════════╝${NC}\n"

echo -e "${BOLD}1. Vérification du port 57123 (RPC Miden)${NC}"
if lsof -i :57123 >/dev/null 2>&1; then
    echo -e "${GREEN}✓ Port 57123 est OUVERT${NC}"
    echo -e "${CYAN}  Détails:${NC}"
    lsof -i :57123 | head -5
    echo ""
else
    echo -e "${RED}✗ Port 57123 est FERMÉ${NC}"
    echo -e "${YELLOW}  Le node Miden ne semble pas tourner${NC}\n"
fi

echo -e "${BOLD}2. Recherche de processus 'miden'${NC}"
if ps aux | grep -i miden | grep -v grep >/dev/null 2>&1; then
    echo -e "${GREEN}✓ Processus miden trouvé:${NC}"
    ps aux | grep -i miden | grep -v grep
    echo ""
else
    echo -e "${RED}✗ Aucun processus miden trouvé${NC}\n"
fi

echo -e "${BOLD}3. Vérification des autres ports courants${NC}"
for port in 57291 8080 3000 5000; do
    if lsof -i :$port >/dev/null 2>&1; then
        echo -e "${CYAN}  Port $port:${NC} ${GREEN}OUVERT${NC}"
    else
        echo -e "${CYAN}  Port $port:${NC} ${YELLOW}fermé${NC}"
    fi
done
echo ""

echo -e "${BOLD}4. Test de connexion au node${NC}"
if curl -s http://localhost:57123 >/dev/null 2>&1; then
    echo -e "${GREEN}✓ Le node répond sur http://localhost:57123${NC}\n"
elif nc -z localhost 57123 >/dev/null 2>&1; then
    echo -e "${YELLOW}⚠ Le port 57123 est ouvert mais ne répond pas en HTTP${NC}"
    echo -e "${CYAN}  Cela peut être normal pour un node gRPC${NC}\n"
else
    echo -e "${RED}✗ Impossible de se connecter au port 57123${NC}\n"
fi

echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${BOLD}Solutions:${NC}\n"

if ! lsof -i :57123 >/dev/null 2>&1; then
    echo -e "${YELLOW}Le node Miden local n'est pas démarré.${NC}"
    echo -e "${CYAN}Pour le démarrer:${NC}"
    echo -e "  1. Vérifie que tu as miden-node installé"
    echo -e "  2. Lance: ${BOLD}miden-node start${NC}"
    echo -e "  3. Ou si tu l'as compilé: ${BOLD}./target/release/miden-node start${NC}"
    echo -e "  4. Attends quelques secondes puis relance ce script\n"
else
    echo -e "${GREEN}Le node semble tourner correctement!${NC}"
    echo -e "${CYAN}Tu peux lancer:${NC} ${BOLD}NETWORK=local ./demo-live-verbose.sh${NC}\n"
fi

echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}\n"
