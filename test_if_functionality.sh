#!/bin/bash

# Test script for if/elif/else functionality in Flash shell
# This script tests various conditional constructs

set -e

# Find the flash binary, handling cross-compilation
find_flash_binary() {
    # First try the target-specific path (for cross-compilation)
    if [ -n "$CARGO_BUILD_TARGET" ]; then
        local target_path="./target/$CARGO_BUILD_TARGET/release/flash"
        if [ -f "$target_path" ]; then
            echo "$target_path"
            return
        fi
    fi
    
    # Fall back to the default path
    local default_path="./target/release/flash"
    if [ -f "$default_path" ]; then
        echo "$default_path"
        return
    fi
    
    # Try to find any flash binary in target directories
    for dir in ./target/*/release; do
        if [ -f "$dir/flash" ]; then
            echo "$dir/flash"
            return
        fi
    done
    
    # Last resort: return the default path
    echo "$default_path"
}

FLASH_BINARY=$(find_flash_binary)

if [ ! -f "$FLASH_BINARY" ]; then
    echo "Error: Flash binary not found at $FLASH_BINARY"
    echo "Available binaries:"
    find ./target -name "flash" -type f 2>/dev/null || echo "No flash binaries found"
    exit 1
fi

echo "Testing if/elif/else functionality with binary: $FLASH_BINARY"

# Test 1: Simple if statement
echo "Test 1: Simple if statement"
result=$($FLASH_BINARY -c 'if [ "hello" = "hello" ]; then echo "success"; fi')
if [ "$result" != "success" ]; then
    echo "FAIL: Simple if statement failed"
    exit 1
fi
echo "PASS: Simple if statement"

# Test 2: If-else statement
echo "Test 2: If-else statement"
result=$($FLASH_BINARY -c 'if [ "hello" = "world" ]; then echo "fail"; else echo "success"; fi')
if [ "$result" != "success" ]; then
    echo "FAIL: If-else statement failed"
    exit 1
fi
echo "PASS: If-else statement"

# Test 3: If-elif-else statement
echo "Test 3: If-elif-else statement"
result=$($FLASH_BINARY -c 'if [ "1" = "2" ]; then echo "fail1"; elif [ "2" = "2" ]; then echo "success"; else echo "fail2"; fi')
if [ "$result" != "success" ]; then
    echo "FAIL: If-elif-else statement failed"
    exit 1
fi
echo "PASS: If-elif-else statement"

# Test 4: Multiple elif branches
echo "Test 4: Multiple elif branches"
result=$($FLASH_BINARY -c 'if [ "1" = "2" ]; then echo "fail1"; elif [ "2" = "3" ]; then echo "fail2"; elif [ "3" = "3" ]; then echo "success"; else echo "fail3"; fi')
if [ "$result" != "success" ]; then
    echo "FAIL: Multiple elif branches failed"
    exit 1
fi
echo "PASS: Multiple elif branches"

# Test 5: Nested if statements
echo "Test 5: Nested if statements"
result=$($FLASH_BINARY -c 'if [ "1" = "1" ]; then if [ "2" = "2" ]; then echo "success"; fi; fi')
if [ "$result" != "success" ]; then
    echo "FAIL: Nested if statements failed"
    exit 1
fi
echo "PASS: Nested if statements"

echo "All if/elif/else tests passed!"