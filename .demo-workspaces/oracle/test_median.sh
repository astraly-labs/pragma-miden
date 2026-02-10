#!/bin/bash
echo "Testing MASM script generation:"
echo "push.1.0.0.0"
echo ""
echo "Calling median with debug..."
RUST_LOG=debug ../../target/release/pm-oracle-cli median 1:0 --network local 2>&1 | grep -A 5 -B 5 "push\|script"
