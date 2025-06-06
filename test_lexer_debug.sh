#!/bin/bash
echo "Testing how the shell tokenizes alias commands:"
echo "Input: alias test_alias=echo hello world"
echo "This should be tokenized as separate arguments"
echo ""
echo "Input: alias path_alias=/path/to/file\\ with\\ spaces"  
echo "This should preserve the escaped spaces"
