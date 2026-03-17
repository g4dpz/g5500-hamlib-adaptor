// Feature: adc-voltage-calibration — Property-based tests
//
// Standalone test crate that duplicates pure logic from the firmware to verify
// correctness properties defined in the design document. Separated from the
// firmware crate because the firmware targets thumbv6m-none-eabi (no_std) and
// its dependencies (cortex-m, embassy) cannot compile for the host.
//
// Run with: cargo test (from the tests/ directory)

#[cfg(test)]
mod calibration_properties;
