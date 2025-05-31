#!/bin/bash

# Test script for if/elif/else functionality in Flash shell
# This script tests the interpreter's ability to handle conditional statements

set -e  # Exit on any error

FLASH_BIN="./target/release/flash"
PASSED=0
FAILED=0

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to run a test case
run_test() {
    local test_name="$1"
    local input="$2"
    local expected="$3"
    
    echo -n "Testing $test_name... "
    
    # Run the test and capture output
    local actual
    actual=$(echo -e "$input\nexit" | $FLASH_BIN 2>/dev/null | head -1 | tr -d '\n')
    
    if [ "$actual" = "$expected" ]; then
        echo -e "${GREEN}PASS${NC}"
        PASSED=$((PASSED + 1))
    else
        echo -e "${RED}FAIL${NC}"
        echo "  Expected: '$expected'"
        echo "  Actual:   '$actual'"
        FAILED=$((FAILED + 1))
    fi
}

# Function to print test section header
print_section() {
    echo -e "\n${YELLOW}=== $1 ===${NC}"
}

echo "Flash If/Elif/Else Functionality Tests"
echo "======================================"

# Check if flash binary exists
if [ ! -f "$FLASH_BIN" ]; then
    echo -e "${RED}Error: Flash binary not found at $FLASH_BIN${NC}"
    echo "Please run 'cargo build --release' first"
    exit 1
fi

print_section "Basic If/Else Tests"

run_test "simple if true" \
    'if [ "test" = "test" ]; then echo "match"; else echo "no match"; fi' \
    "match"

run_test "simple if false" \
    'if [ "test" = "other" ]; then echo "match"; else echo "no match"; fi' \
    "no match"

run_test "if without else (true)" \
    'if [ "a" = "a" ]; then echo "true"; fi' \
    "true"

run_test "if without else (false)" \
    'if [ "a" = "b" ]; then echo "true"; fi' \
    ""

print_section "If/Elif/Else Tests"

run_test "elif first condition true" \
    'if [ "a" = "a" ]; then echo "first"; elif [ "b" = "b" ]; then echo "second"; else echo "third"; fi' \
    "first"

run_test "elif second condition true" \
    'if [ "a" = "b" ]; then echo "first"; elif [ "c" = "c" ]; then echo "second"; else echo "third"; fi' \
    "second"

run_test "elif else branch" \
    'if [ "a" = "b" ]; then echo "first"; elif [ "c" = "d" ]; then echo "second"; else echo "third"; fi' \
    "third"

print_section "Multiple Elif Tests"

run_test "multiple elif - first true" \
    'if [ "a" = "a" ]; then echo "first"; elif [ "b" = "b" ]; then echo "second"; elif [ "c" = "c" ]; then echo "third"; else echo "fourth"; fi' \
    "first"

run_test "multiple elif - second true" \
    'if [ "a" = "b" ]; then echo "first"; elif [ "c" = "c" ]; then echo "second"; elif [ "d" = "d" ]; then echo "third"; else echo "fourth"; fi' \
    "second"

run_test "multiple elif - third true" \
    'if [ "a" = "b" ]; then echo "first"; elif [ "c" = "d" ]; then echo "second"; elif [ "e" = "e" ]; then echo "third"; else echo "fourth"; fi' \
    "third"

run_test "multiple elif - else" \
    'if [ "a" = "b" ]; then echo "first"; elif [ "c" = "d" ]; then echo "second"; elif [ "e" = "f" ]; then echo "third"; else echo "fourth"; fi' \
    "fourth"

print_section "Variable Expansion Tests"

run_test "variable in condition" \
    'X=test; if [ "$X" = "test" ]; then echo "variable match"; else echo "no match"; fi' \
    "variable match"

run_test "variable in both sides" \
    'X=hello; Y=hello; if [ "$X" = "$Y" ]; then echo "both match"; else echo "no match"; fi' \
    "both match"

run_test "variable inequality" \
    'X=hello; Y=world; if [ "$X" = "$Y" ]; then echo "match"; else echo "different"; fi' \
    "different"

print_section "Numeric Comparison Tests"

run_test "numeric equality" \
    'X=5; Y=5; if [ "$X" -eq "$Y" ]; then echo "equal"; else echo "not equal"; fi' \
    "equal"

run_test "numeric inequality" \
    'X=5; Y=3; if [ "$X" -ne "$Y" ]; then echo "not equal"; else echo "equal"; fi' \
    "not equal"

run_test "less than" \
    'X=3; Y=5; if [ "$X" -lt "$Y" ]; then echo "less"; else echo "not less"; fi' \
    "less"

run_test "greater than" \
    'X=7; Y=5; if [ "$X" -gt "$Y" ]; then echo "greater"; else echo "not greater"; fi' \
    "greater"

run_test "less than or equal" \
    'X=5; Y=5; if [ "$X" -le "$Y" ]; then echo "le"; else echo "not le"; fi' \
    "le"

run_test "greater than or equal" \
    'X=5; Y=5; if [ "$X" -ge "$Y" ]; then echo "ge"; else echo "not ge"; fi' \
    "ge"

print_section "String Test Operations"

run_test "non-empty string test" \
    'if [ -n "hello" ]; then echo "non-empty"; else echo "empty"; fi' \
    "non-empty"

run_test "empty string test" \
    'if [ -z "" ]; then echo "empty"; else echo "non-empty"; fi' \
    "empty"

run_test "string inequality" \
    'if [ "hello" != "world" ]; then echo "different"; else echo "same"; fi' \
    "different"

print_section "File System Tests"

run_test "file exists test" \
    'if [ -f "/etc/passwd" ]; then echo "file exists"; else echo "file not found"; fi' \
    "file exists"

run_test "directory exists test" \
    'if [ -d "/tmp" ]; then echo "dir exists"; else echo "dir not found"; fi' \
    "dir exists"

run_test "path exists test" \
    'if [ -e "/etc" ]; then echo "path exists"; else echo "path not found"; fi' \
    "path exists"

run_test "non-existent file test" \
    'if [ -f "/nonexistent/file" ]; then echo "exists"; else echo "does not exist"; fi' \
    "does not exist"

print_section "Complex Scenarios"

run_test "mixed string and numeric" \
    'X=hello; Y=5; if [ "$X" = "hello" ]; then echo "string match"; elif [ "$Y" -gt 3 ]; then echo "number test"; else echo "no match"; fi' \
    "string match"

run_test "mixed with fallback to numeric" \
    'X=world; Y=5; if [ "$X" = "hello" ]; then echo "string match"; elif [ "$Y" -gt 3 ]; then echo "number test"; else echo "no match"; fi' \
    "number test"

run_test "mixed with fallback to else" \
    'X=world; Y=1; if [ "$X" = "hello" ]; then echo "string match"; elif [ "$Y" -gt 3 ]; then echo "number test"; else echo "no match"; fi' \
    "no match"

run_test "nested conditions" \
    'X=5; if [ "$X" -gt 0 ]; then if [ "$X" -lt 10 ]; then echo "between 0 and 10"; else echo "10 or more"; fi; else echo "zero or negative"; fi' \
    "between 0 and 10"

print_section "Edge Cases"

run_test "empty condition" \
    'if [ "" ]; then echo "true"; else echo "false"; fi' \
    "false"

run_test "single argument test" \
    'if [ "hello" ]; then echo "true"; else echo "false"; fi' \
    "true"

run_test "whitespace handling" \
    'X="hello world"; if [ "$X" = "hello world" ]; then echo "match"; else echo "no match"; fi' \
    "match"

# Print summary
echo ""
echo "======================================"
echo "Test Results Summary:"
echo -e "  ${GREEN}Passed: $PASSED${NC}"
if [ $FAILED -gt 0 ]; then
    echo -e "  ${RED}Failed: $FAILED${NC}"
    echo ""
    echo -e "${RED}Some tests failed. Please check the implementation.${NC}"
    exit 1
else
    echo -e "  ${RED}Failed: $FAILED${NC}"
    echo ""
    echo -e "${GREEN}All tests passed! 🎉${NC}"
    exit 0
fi