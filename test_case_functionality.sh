#!/bin/bash

# Test script for case/esac functionality in Flash shell
# This script tests various case statement constructs

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

echo "Testing case/esac functionality with binary: $FLASH_BINARY"

# Test 1: Simple case statement with exact match
echo "Test 1: Simple case statement with exact match"
result=$("$FLASH_BINARY" -c 'case "hello" in hello) echo "matched" ;; *) echo "no match" ;; esac')
expected="matched"
if [ "$result" = "$expected" ]; then
    echo "✓ PASS"
else
    echo "✗ FAIL: Expected '$expected', got '$result'"
    exit 1
fi

# Test 2: Case statement with wildcard
echo "Test 2: Case statement with wildcard"
result=$("$FLASH_BINARY" -c 'case "anything" in hello) echo "hello" ;; *) echo "wildcard" ;; esac')
expected="wildcard"
if [ "$result" = "$expected" ]; then
    echo "✓ PASS"
else
    echo "✗ FAIL: Expected '$expected', got '$result'"
    exit 1
fi

# Test 3: Case statement with multiple patterns
echo "Test 3: Case statement with multiple patterns"
result=$("$FLASH_BINARY" -c 'case "world" in hello|hi) echo "greeting" ;; world|earth) echo "planet" ;; *) echo "unknown" ;; esac')
expected="planet"
if [ "$result" = "$expected" ]; then
    echo "✓ PASS"
else
    echo "✗ FAIL: Expected '$expected', got '$result'"
    exit 1
fi

# Test 4: Case statement with variables
echo "Test 4: Case statement with variables"
result=$("$FLASH_BINARY" -c 'var="test"; case "$var" in test) echo "variable matched" ;; *) echo "no match" ;; esac')
expected="variable matched"
if [ "$result" = "$expected" ]; then
    echo "✓ PASS"
else
    echo "✗ FAIL: Expected '$expected', got '$result'"
    exit 1
fi

# Test 5: Case statement with no matching pattern
echo "Test 5: Case statement with no matching pattern"
result=$("$FLASH_BINARY" -c 'case "nomatch" in hello) echo "hello" ;; world) echo "world" ;; esac')
expected=""
if [ "$result" = "$expected" ]; then
    echo "✓ PASS"
else
    echo "✗ FAIL: Expected empty output, got '$result'"
    exit 1
fi

# Test 6: Case statement with pattern matching (wildcards)
echo "Test 6: Case statement with pattern matching"
result=$("$FLASH_BINARY" -c 'case "file.txt" in *.txt) echo "text file" ;; *.log) echo "log file" ;; *) echo "other" ;; esac')
expected="text file"
if [ "$result" = "$expected" ]; then
    echo "✓ PASS"
else
    echo "✗ FAIL: Expected '$expected', got '$result'"
    exit 1
fi

# Test 7: Case statement with complex body
echo "Test 7: Case statement with complex body"
result=$("$FLASH_BINARY" -c 'case "complex" in complex) echo "first"; echo "second" ;; *) echo "simple" ;; esac')
expected="first
second"
if [ "$result" = "$expected" ]; then
    echo "✓ PASS"
else
    echo "✗ FAIL: Expected '$expected', got '$result'"
    exit 1
fi

# Test 8: Nested case statements
echo "Test 8: Nested case statements"
result=$("$FLASH_BINARY" -c 'outer="file"; case "$outer" in file) inner="txt"; case "$inner" in txt) echo "nested match" ;; *) echo "inner no match" ;; esac ;; *) echo "outer no match" ;; esac')
expected="nested match"
if [ "$result" = "$expected" ]; then
    echo "✓ PASS"
else
    echo "✗ FAIL: Expected '$expected', got '$result'"
    exit 1
fi

# Test 9: Case statement with environment variables
echo "Test 9: Case statement with environment variables"
result=$("$FLASH_BINARY" -c 'export TEST_VAR="production"; case "$TEST_VAR" in development) echo "dev" ;; production) echo "prod" ;; *) echo "unknown" ;; esac')
expected="prod"
if [ "$result" = "$expected" ]; then
    echo "✓ PASS"
else
    echo "✗ FAIL: Expected '$expected', got '$result'"
    exit 1
fi

# Test 10: Case statement with command substitution
echo "Test 10: Case statement with command substitution"
result=$("$FLASH_BINARY" -c 'result=$(echo hello); case "$result" in hello) echo "command substitution works" ;; *) echo "failed" ;; esac')
expected="command substitution works"
if [ "$result" = "$expected" ]; then
    echo "✓ PASS"
else
    echo "✗ FAIL: Expected '$expected', got '$result'"
    exit 1
fi

echo ""
echo "All case/esac functionality tests passed! ✓"