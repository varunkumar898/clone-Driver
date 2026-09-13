# Testing Strategy & Test Execution Guide

## Overview

DiskClone employs a multi-tiered testing strategy to guarantee maximum safety, correctness, and performance without requiring physical destruction of real storage hardware during testing.

## Test Tiers

1. **Unit Tests (`tests/unit/`)**:
   - `state_machine.rs`: Validates state transition graph, illegal transitions, error branches, and context mutation.
   - `device_filter.rs`: Tests system drive detection, mount filtering, and capacity comparisons using mock devices.
   - `checksum.rs`: Asserts correctness of xxHash64 and SHA256 against standard vector benchmarks.
   - `buffer_pool.rs`: Validates aligned buffer allocation, pooling, exhaustion handling, and thread safety.
   - `error_handling.rs`: Verifies error enum variants, error codes, and formatting.
   - `partition_parser.rs`: Tests parsing of binary GPT and MBR fixtures against known partition layouts.

2. **Integration Tests (`tests/integration/`)**:
   - `device_discovery.rs`: Validates device manager querying using synthetic sysfs structures.
   - `clone_operation.rs`: Tests clone execution between temporary sparse files with simulated bad blocks.
   - `verification.rs`: Tests full block verification pass on cloned sparse files.

3. **End-to-End (E2E) Tests (`tests/e2e/`)**:
   - `full_clone.rs`: Executes simulated end-to-end clone with state machine events, resume journal generation, pause/resume, and verification.

---

## Running Tests

### Automated Test Runner
```bash
./scripts/test.sh
```

### Running Specific Test Suites
```bash
# Unit tests
cargo test --test state_machine
cargo test --test checksum
cargo test --test partition_parser

# Run with verbose output
cargo test -- --nocapture
```

### Static Analysis and Code Formatting
```bash
./scripts/lint.sh
```
