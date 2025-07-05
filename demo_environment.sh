#!/usr/bin/env bash

# Demo script showing the improved environment system for flash shell

echo "=== Flash Shell Environment System Demo ==="
echo

echo "1. Basic Environment Variables:"
echo "SHELL: $SHELL"
echo "FLASH_VERSION: $FLASH_VERSION"
echo "PWD: $PWD"
echo "MACHTYPE: $MACHTYPE"
echo "HOSTTYPE: $HOSTTYPE"
echo "OSTYPE: $OSTYPE"
echo

echo "2. Special Parameters:"
echo "Exit status (\$?): $?"
echo "Process ID (\$\$): $$"
echo "Number of args (\$#): $#"
echo "All args (\$*): $*"
echo "Shell flags (\$-): $-"
echo

echo "3. Shell Level:"
echo "SHLVL: $SHLVL"
echo

echo "4. History Configuration:"
echo "HISTFILE: $HISTFILE"
echo "HISTSIZE: $HISTSIZE"
echo "HISTFILESIZE: $HISTFILESIZE"
echo

echo "5. Prompt Configuration:"
echo "PS1: $PS1"
echo "PS2: $PS2"
echo "PS4: $PS4"
echo

echo "6. Field Separator:"
echo "IFS: '$IFS'"
echo

echo "=== Testing Variable Scoping ==="
echo

# Test function with local variables
test_function() {
    echo "Inside function:"
    local LOCAL_VAR="function_local"
    echo "LOCAL_VAR: $LOCAL_VAR"
    
    # Override global variable locally
    local GLOBAL_VAR="overridden_in_function"
    echo "GLOBAL_VAR (local): $GLOBAL_VAR"
}

# Set global variable
GLOBAL_VAR="global_value"
echo "Before function call:"
echo "GLOBAL_VAR: $GLOBAL_VAR"
echo

test_function
echo

echo "After function call:"
echo "GLOBAL_VAR: $GLOBAL_VAR"
echo

echo "=== Testing Export Functionality ==="
echo

# Test export
TEST_EXPORT="exported_value"
export TEST_EXPORT
echo "TEST_EXPORT (should be in environment): $TEST_EXPORT"

# Test in subshell
(
    echo "In subshell:"
    echo "TEST_EXPORT: $TEST_EXPORT"
    echo "SHLVL: $SHLVL"
)

echo

echo "=== Testing Array Variables ==="
echo

# Test array (if supported)
declare -a TEST_ARRAY=("item1" "item2" "item3")
echo "Array elements: ${TEST_ARRAY[@]}"
echo "Array length: ${#TEST_ARRAY[@]}"
echo "First element: ${TEST_ARRAY[0]}"

echo

echo "=== Testing Positional Parameters ==="
echo

# Function to test positional parameters
test_positional() {
    echo "Function arguments:"
    echo "\$0: $0"
    echo "\$1: $1"
    echo "\$2: $2"
    echo "\$#: $#"
    echo "\$*: $*"
    echo "\$@: $@"
}

test_positional "arg1" "arg2" "arg3"

echo

echo "=== Demo Complete ==="