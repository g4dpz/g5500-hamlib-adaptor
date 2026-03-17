# Implementation Plan: ADC Voltage Calibration

## Overview

Incrementally expand the firmware to support two-point ADC calibration: first grow the Config struct and flash format (v1→v2 migration), then wire calibration into the ADC conversion loop, add the three HTTP endpoints, and finally render the calibration UI section. Each step builds on the previous and ends with a checkpoint.

## Tasks

- [x] 1. Expand Config struct and flash format to v2
  - [x] 1.1 Add calibration fields to Config struct and update defaults
    - Add `calibration_valid: bool`, `az_raw_low: f32`, `az_raw_high: f32`, `el_raw_low: f32`, `el_raw_high: f32`, `az_deg_low: f32`, `az_deg_high: f32`, `el_deg_low: f32`, `el_deg_high: f32` to `Config` in `firmware/src/config.rs`
    - Update `Config::default()` to set `calibration_valid: false` and all calibration fields to `0.0`
    - Change `CONFIG_VERSION` to `0x02` and `CONFIG_SIZE` to `60`
    - _Requirements: 1.1, 1.2, 1.3, 1.4_

  - [x] 1.2 Implement v2 serialization (`to_bytes`) and deserialization (`from_bytes`)
    - Serialize the 9 new fields into the v2 flash layout (offsets 23–58, CRC at byte 59) as specified in the design
    - Implement `from_bytes` to dispatch on version byte: v1 → `from_bytes_v1`, v2 → `from_bytes_v2`, other → `None`
    - Implement `from_bytes_v2` to deserialize all 60 bytes with CRC8 validation
    - _Requirements: 2.1, 2.3, 2.5_

  - [x] 1.3 Implement v1→v2 migration (`from_bytes_v1`)
    - Read and validate the 24-byte v1 layout (CRC8 over bytes 0..23)
    - Populate v1 fields (static_ip, cal offsets, park) into the v2 Config
    - Set `calibration_valid = false` and all calibration fields to `0.0`
    - Return `None` for unrecognized versions (not 0x01 or 0x02)
    - _Requirements: 2.2, 2.4, 7.3_

  - [x] 1.4 Update `load_config` to read 60-byte buffer
    - Change the read buffer in `load_config` from 24 to 60 bytes
    - Pass the 60-byte buffer to the updated `from_bytes`
    - Update `save_config` to write the 60-byte v2 buffer
    - _Requirements: 2.2, 2.3_

  - [x]* 1.5 Write property test: Config v2 round-trip (Property 1)
    - **Property 1: Config v2 serialization round-trip**
    - For any valid Config, `from_bytes(to_bytes(cfg))` produces a field-equal Config
    - **Validates: Requirements 1.1, 1.2, 1.3, 2.3**

  - [x]* 1.6 Write property test: CRC8 integrity (Property 2)
    - **Property 2: CRC8 integrity invariant**
    - For any valid Config, `crc8_ccitt_validate(to_bytes(cfg))` is true
    - **Validates: Requirements 2.5**

  - [x]* 1.7 Write property test: v1→v2 migration preserves fields (Property 3)
    - **Property 3: v1→v2 migration preserves existing fields**
    - Serialize as v1, deserialize with v2 from_bytes, verify v1 fields match and cal fields are defaults
    - **Validates: Requirements 2.2, 7.3**

  - [x]* 1.8 Write property test: unknown version returns None (Property 4)
    - **Property 4: Unknown version returns None**
    - Buffer with magic 0xAE and version ∉ {0x01, 0x02} → `from_bytes()` returns None
    - **Validates: Requirements 2.4**

- [x] 2. Checkpoint — Config and flash migration
  - Ensure all tests pass, ask the user if questions arise.
  - Verify `cargo build --release` succeeds from `firmware/` directory

- [x] 3. Implement calibrated ADC conversion in adc_task
  - [x] 3.1 Add CalParams struct and computation function
    - Define `CalParams` struct local to `adc_task` in `firmware/src/main.rs` with fields: `az_slope`, `az_intercept`, `el_slope`, `el_intercept`, `az_offset`, `el_offset`, `use_cal_az`, `use_cal_el`
    - Implement a function to compute `CalParams` from a `Config`: per-axis validation (raw span >= 100 counts, degree span != 0), slope/intercept derivation
    - _Requirements: 6.1, 6.2, 6.5, 9.1, 9.2, 9.3, 9.4_

  - [x] 3.2 Integrate CalParams into the adc_task loop
    - Read `STORED_CONFIG` once at startup to compute initial `CalParams`
    - Re-read config each 100ms tick (outside the sample accumulation) to detect changes and recompute `CalParams`
    - Replace the hardcoded theoretical conversion with: if `use_cal_az`/`use_cal_el`, apply `slope * raw + intercept + offset`; otherwise use existing theoretical constants + offset
    - Clamp azimuth to 0.0–450.0 and elevation to 0.0–180.0
    - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5, 7.1, 7.2_

  - [x]* 3.3 Write property test: linear mapping reproduces reference points (Property 5)
    - **Property 5: Linear mapping reproduces reference points**
    - For any two distinct reference points with raw span >= 100 and deg span != 0, mapping at raw_low → deg_low and raw_high → deg_high within f32 tolerance
    - **Validates: Requirements 6.1, 6.2**

  - [x]* 3.4 Write property test: output clamping invariant (Property 6)
    - **Property 6: Output clamping invariant**
    - For any raw value and any calibration params, az output ∈ [0.0, 450.0] and el output ∈ [0.0, 180.0]
    - **Validates: Requirements 6.4**

  - [x]* 3.5 Write property test: offset is additive before clamping (Property 7)
    - **Property 7: Offset is additive before clamping**
    - `convert(raw, cal, offset) == clamp(linear(raw) + offset)`
    - **Validates: Requirements 7.1, 7.2**

- [x] 4. Checkpoint — ADC calibration conversion
  - Ensure all tests pass, ask the user if questions arise.
  - Verify `cargo build --release` succeeds from `firmware/` directory

- [x] 5. Add calibration HTTP endpoints
  - [x] 5.1 Implement POST /cal/capture-low handler
    - Add `handle_cal_capture_low` in `firmware/src/http.rs`
    - Read current raw ADC from `CURRENT_AZ_EL_RAW`, parse `adl`/`edl` form fields for degree values (default 0.0)
    - Store into `az_raw_low`, `el_raw_low`, `az_deg_low`, `el_deg_low` of `STORED_CONFIG`
    - Set `CONFIG_SAVE_PENDING`, respond with 303 redirect to `/`
    - _Requirements: 3.1, 3.2, 3.3, 3.4_

  - [x] 5.2 Implement POST /cal/capture-high handler
    - Add `handle_cal_capture_high` in `firmware/src/http.rs`
    - Read current raw ADC from `CURRENT_AZ_EL_RAW`, parse `adh`/`edh` form fields for degree values (default 450.0/180.0)
    - Store into `az_raw_high`, `el_raw_high`, `az_deg_high`, `el_deg_high` of `STORED_CONFIG`
    - Set `calibration_valid = true` if both low and high data are present (all four raw values and degree values populated)
    - Set `CONFIG_SAVE_PENDING`, respond with 303 redirect to `/`
    - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5_

  - [x] 5.3 Implement POST /cal/clear handler
    - Add `handle_cal_clear` in `firmware/src/http.rs`
    - Reset `calibration_valid = false` and all 8 calibration fields to `0.0` in `STORED_CONFIG`
    - Set `CONFIG_SAVE_PENDING`, respond with 303 redirect to `/`
    - _Requirements: 5.1, 5.2, 5.3_

  - [x] 5.4 Wire new routes into the HTTP request dispatcher
    - Extend the `if/else if` chain in `http_server` to match `POST /cal/capture-low`, `POST /cal/capture-high`, and `POST /cal/clear`
    - _Requirements: 3.1, 4.1, 5.1_

  - [x]* 5.5 Write property test: clear calibration zeroes all cal fields (Property 8)
    - **Property 8: Clear calibration zeroes all calibration fields**
    - For any Config, clearing calibration sets `calibration_valid = false` and all 8 cal fields to 0.0, non-cal fields unchanged
    - **Validates: Requirements 5.1**

  - [x]* 5.6 Write property test: calibration_valid set when both points present (Property 9)
    - **Property 9: calibration_valid is set when both reference points are present**
    - After capture-high with both low and high data populated, `calibration_valid` is true
    - **Validates: Requirements 4.3**

- [x] 6. Checkpoint — HTTP calibration endpoints
  - Ensure all tests pass, ask the user if questions arise.
  - Verify `cargo build --release` succeeds from `firmware/` directory

- [x] 7. Add calibration UI section to the status page
  - [x] 7.1 Render calibration status and reference values
    - In `serve_status_page` in `firmware/src/http.rs`, add a "Calibration" section after the existing Configuration form
    - Show calibration state badge ("Calibrated" / "Uncalibrated")
    - When `calibration_valid` is true, display a table of stored reference values (raw ADC and degrees for both low and high points)
    - Display current live raw ADC values (already available from `CURRENT_AZ_EL_RAW`)
    - _Requirements: 8.1, 8.2, 8.6_

  - [x] 7.2 Render Capture Low, Capture High, and Clear Calibration forms
    - Add "Capture Low" form posting to `/cal/capture-low` with input fields `adl` (default 0.0) and `edl` (default 0.0)
    - Add "Capture High" form posting to `/cal/capture-high` with input fields `adh` (default 450.0) and `edh` (default 180.0)
    - Add "Clear Calibration" button as a form posting to `/cal/clear` with no fields
    - Stream HTML in chunks via `socket.write_all()` to stay within the 512-byte format buffer
    - _Requirements: 8.3, 8.4, 8.5_

- [x] 8. Final checkpoint — Full integration
  - Ensure all tests pass, ask the user if questions arise.
  - Verify `cargo build --release` succeeds from `firmware/` directory
  - Confirm the v2 flash layout, HTTP endpoints, ADC conversion, and UI section are all wired together

## Notes

- Tasks marked with `*` are optional and can be skipped for faster MVP
- Each task references specific requirements for traceability
- Property tests require host-target compilation (proptest needs std) — extract pure logic into testable functions
- Checkpoints ensure incremental validation after each major component
- The existing `CONFIG_SAVE_PENDING` mechanism handles flash persistence for all new endpoints
