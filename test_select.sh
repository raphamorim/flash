#!/usr/bin/env bash

# Test script for select functionality
echo "Testing select statement parsing..."

# Test 1: Basic select parsing
echo "select choice in apple banana cherry; do echo \$choice; done" | ./target/debug/flash -c "$(cat)"

echo "Select parsing test completed."