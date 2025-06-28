#!/bin/bash
echo "Starting debug test..."
timeout 3s ./target/release/thyme test_rust.rs 2>&1 | tee debug_output.txt || echo "Test completed"
echo "Debug output:"
cat debug_output.txt
