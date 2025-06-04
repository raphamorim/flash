#!/bin/bash

echo "Testing Ctrl+C behavior in Flash shell"
echo "This script will start the Flash shell."
echo "Try pressing Ctrl+C and observe the behavior:"
echo "- Should print ^C"
echo "- Should go to a new line"
echo "- Should show a fresh prompt"
echo "- Should NOT exit the shell"
echo ""
echo "Type 'exit' to quit the shell when done testing."
echo ""

./target/release/flash