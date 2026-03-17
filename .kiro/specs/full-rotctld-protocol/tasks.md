# Implementation Plan: Full rotctld Protocol Support

## Overview

Incrementally extend the firmware's rotctld implementation in `firmware/src/main.rs` to cover the complete Hamlib command dispatch table. Each task builds on the previous, starting with data model additions, then parser extensions, then handler logic, and finally wiring and integration. All changes are in `firmware/src/main.rs` except for `Cargo.toml` dev-dependencies.

## Tasks

- [x] 1. Extend HamlibCommand enum and add data model types
  - [x] 1.1 Add new HamlibCommand variants to the enum
    - Add `GetStatus`, `SetConf`, `GetConf`, `DumpConf` (functional commands)
    - Add `SetLevel`, `GetLevel`, `SetFunc`, `GetFunc`, `SetParm`, `GetParm`, `SendCmd` (stub commands)
    - Add `Lonlat2Loc`, `Loc2Lonlat`, `Dms2Dec`, `Dec2Dms`, `Dmmm2Dec`, `Dec2Dmmm`, `Qrb`, `AzSp2AzLp`, `DistSp2DistLp` (locator stubs)
    - _Requirements: 11.1, 11.2, 11.3_

  - [x] 1.2 Add direction code constants and status flag constants
    - Define `ROT_MOVE_UP` through `ROT_MOVE_DOWN_RIGHT` constants (2, 4, 8, 16, 32, 64, 128, 256)
    - Define `ROT_STATUS_NONE` (0) and `ROT_STATUS_MOVING` (2) constants
    - _Requirements: 2.1–2.6, 4.2, 4.3_

  - [x] 1.3 Add ConfigToken enum with `from_bytes` method
    - Implement `ConfigToken` enum with variants `MinAz`, `MaxAz`, `MinEl`, `MaxEl`, `ParkAz`, `ParkEl`
    - Implement `ConfigToken::from_bytes(input: &[u8]) -> Option<Self>` matching the 6 token strings
    - _Requirements: 5.6_

  - [x] 1.4 Extend `Command::parse()` return type to carry optional string args
    - Change return type from `(HamlibCommand, f32, f32)` to `(HamlibCommand, f32, f32, Option<(&[u8], &[u8])>)` for conf commands
    - Update all existing call sites in `listen_task` to destructure the new 4-tuple
    - _Requirements: 5.1_

- [x] 2. Extend the parser to recognize all protocol commands
  - [x] 2.1 Add parser functions for functional commands
    - `parse_get_status`: matches `s` or `\get_status` (no args)
    - `parse_set_conf`: matches `C` or `\set_conf`, extracts token and value byte slices
    - `parse_get_conf`: matches `\get_conf`, extracts token byte slice (note: no short-form character for get_conf)
    - `parse_dump_conf`: matches `3` or `\dump_conf` (no args)
    - _Requirements: 4.1, 5.1, 5.4, 6.1, 11.1, 11.2_

  - [x] 2.2 Add parser functions for stub commands that consume trailing arguments
    - `parse_set_level`: matches `V` or `\set_level`, discards rest of line
    - `parse_get_level`: matches `v` or `\get_level`, discards rest of line
    - `parse_set_func`: matches `U` or `\set_func`, discards rest of line
    - `parse_get_func`: matches `u` or `\get_func`, discards rest of line
    - `parse_set_parm`: matches `X` or `\set_parm`, discards rest of line
    - `parse_get_parm`: matches `x` or `\get_parm`, discards rest of line
    - `parse_send_cmd`: matches `w` or `\send_cmd`, discards rest of line
    - _Requirements: 7.1–7.7, 11.1, 11.2, 11.4_

  - [x] 2.3 Add parser functions for locator stub commands
    - Match both short-form (`L`, `l`, `D`, `d`, `E`, `e`, `B`, `A`, `a`) and long-form (`\lonlat2loc`, `\loc2lonlat`, `\dms2dec`, `\dec2dms`, `\dmmm2dec`, `\dec2dmmm`, `\qrb`, `\a_sp2a_lp`, `\d_sp2d_lp`)
    - Discard any trailing arguments
    - _Requirements: 10.1, 10.2, 11.3, 11.4_

  - [x] 2.4 Update `parse_reset` to accept optional integer argument
    - Modify parser to optionally consume a space followed by an integer after `R` or `\reset`
    - Bare `R` or `\reset` without argument must still parse successfully
    - _Requirements: 3.1, 3.3_

  - [x] 2.5 Wire all new parse functions into `Command::parse()`
    - Add calls to each new parse function in the `Command::parse()` method
    - Return the appropriate `HamlibCommand` variant and string args where applicable
    - Ensure ordering does not cause short-form conflicts (e.g., `s` vs `\set_pos`)
    - _Requirements: 11.1, 11.2, 11.3_

- [x] 3. Checkpoint — Parser compiles and existing commands still work
  - Ensure all tests pass, ask the user if questions arise.

- [x] 4. Rewrite dump_state to protocol v1 wire format
  - [x] 4.1 Replace the current dump_state handler with protocol v1 output
    - Output 9 lines: protocol version `1`, rotator model `601`, `min_az=0.000000`, `max_az=450.000000`, `min_el=0.000000`, `max_el=180.000000`, `south_zero=0`, `rot_type=AzEl`, `done`
    - Use `{:.6}` format for float values to match C `%lf` default
    - Terminate each line with `\n`
    - Use a 256-byte stack buffer
    - _Requirements: 1.1–1.10_

  - [ ]* 4.2 Write unit test for dump_state wire format
    - Verify all 9 lines are present and correctly formatted
    - Verify line terminators are `\n`
    - _Requirements: 1.1–1.10_

- [x] 5. Implement diagonal move directions and fix error codes
  - [x] 5.1 Add diagonal direction cases to the Move handler
    - Add match arms for direction codes 32 (UP_LEFT), 64 (UP_RIGHT), 128 (DOWN_LEFT), 256 (DOWN_RIGHT)
    - Map each to the correct pair of axis extremes (e.g., UP_RIGHT → max az + max el)
    - Change the unknown direction error from `RPRT -1` to `RPRT -2` (RIG_EINVAL)
    - _Requirements: 2.1–2.6_

  - [ ]* 5.2 Write property test for direction code mapping (Property 1)
    - **Property 1: Direction code mapping and move response**
    - For any u16 direction code, valid codes produce correct demand + `RPRT 0`, invalid codes produce `RPRT -2`
    - **Validates: Requirements 2.1, 2.2, 2.3, 2.4, 2.5, 2.6**

- [x] 6. Implement get_status handler
  - [x] 6.1 Add get_status match arm in listen_task
    - Read `DEMAND_RUN_AZ_EL_DEGREES` to get `demand_run` boolean
    - Return `2` (ROT_STATUS_MOVING) if demand_run is true, `0` (ROT_STATUS_NONE) if false
    - Support ERP format: `get_status:\n<value>\nRPRT 0\n`
    - _Requirements: 4.1, 4.2, 4.3, 9.3_

  - [ ]* 6.2 Write property test for status flag derivation (Property 2)
    - **Property 2: Status flag derivation from demand state**
    - For any boolean demand_run, verify correct status value returned
    - **Validates: Requirements 4.2, 4.3**

- [x] 7. Implement set_conf, get_conf, and dump_conf handlers
  - [x] 7.1 Add set_conf match arm in listen_task
    - Parse token via `ConfigToken::from_bytes`, return `RPRT -2` for unrecognized tokens
    - For writable tokens (`park_az`, `park_el`): parse value as f32, update `STORED_CONFIG`, set `CONFIG_SAVE_PENDING`
    - For read-only tokens (`min_az`, `max_az`, `min_el`, `max_el`): return `RPRT 0` without changing value
    - Support ERP format
    - _Requirements: 5.1, 5.2, 5.3, 9.3_

  - [x] 7.2 Add get_conf match arm in listen_task
    - Parse token via `ConfigToken::from_bytes`, return `RPRT -2` for unrecognized tokens
    - Return current value for recognized tokens (constants for min/max, config values for park)
    - Support ERP format
    - _Requirements: 5.4, 5.5, 9.3_

  - [x] 7.3 Add dump_conf match arm in listen_task
    - Output all 6 config tokens as `<token>=<value>` lines with `{:.6}` formatting
    - Support ERP format
    - _Requirements: 6.1, 6.2, 9.3_

  - [ ]* 7.4 Write property test for config round-trip (Property 3)
    - **Property 3: Config parameter round-trip**
    - For any writable token and valid f32 value, set_conf then get_conf returns same value
    - **Validates: Requirements 5.2, 5.4**

  - [ ]* 7.5 Write property test for unrecognized config token rejection (Property 4)
    - **Property 4: Unrecognized config token rejection**
    - For any byte string not in the recognized set, both set_conf and get_conf return `RPRT -2`
    - **Validates: Requirements 5.3, 5.5**

- [x] 8. Checkpoint — Functional commands work
  - Ensure all tests pass, ask the user if questions arise.

- [x] 9. Add stub command handlers and fix _None error code
  - [x] 9.1 Add match arms for all stub commands returning RPRT -4
    - Handle `SetLevel`, `GetLevel`, `SetFunc`, `GetFunc`, `SetParm`, `GetParm`, `SendCmd`
    - Handle all 9 locator stubs (`Lonlat2Loc`, `Loc2Lonlat`, `Dms2Dec`, `Dec2Dms`, `Dmmm2Dec`, `Dec2Dmmm`, `Qrb`, `AzSp2AzLp`, `DistSp2DistLp`)
    - Support ERP format for each: `<long_name>:\nRPRT -4\n`
    - _Requirements: 7.1–7.7, 10.1, 9.1, 9.2, 9.3_

  - [x] 9.2 Fix _None handler to return RPRT -1 in standard mode
    - Change `RPRT 1` to `RPRT -1` in the `_None` match arm (standard mode)
    - ERP mode already returns `RPRT -1` — verify it stays correct
    - _Requirements: 8.1, 8.2_

  - [ ]* 9.3 Write property test for stub commands (Property 5)
    - **Property 5: Stub commands return RPRT -4 regardless of trailing arguments**
    - For any stub command with any trailing bytes, parser recognizes command and handler returns `RPRT -4`
    - **Validates: Requirements 7.1–7.7, 10.1, 11.4**

  - [ ]* 9.4 Write property test for unrecognized command error (Property 6)
    - **Property 6: Unrecognized commands return RPRT -1**
    - For any byte sequence not matching any command, handler returns `RPRT -1`
    - **Validates: Requirements 8.1, 8.2**

- [x] 10. Add proptest dev-dependency and parser unit tests
  - [x] 10.1 Add proptest to Cargo.toml dev-dependencies
    - Add `proptest = { version = "1", default-features = false, features = ["std"] }` under `[dev-dependencies]`
    - _Requirements: (testing infrastructure)_

  - [ ]* 10.2 Write unit tests for parser recognition of all commands
    - Test all short-form commands from Requirement 11.1
    - Test all long-form commands from Requirement 11.2
    - Test all locator long-form commands from Requirement 11.3
    - Test `R` without argument and `R 1` with argument (Requirement 3.1, 3.3)
    - Test all 6 config tokens recognized by `ConfigToken::from_bytes` (Requirement 5.6)
    - _Requirements: 11.1, 11.2, 11.3, 3.1, 3.3, 5.6_

  - [ ]* 10.3 Write property test for ERP response format (Property 7)
    - **Property 7: ERP response format for all commands**
    - For any recognized command with ERP prefix, response begins with long command name + `:\n`
    - **Validates: Requirements 9.1, 9.2, 9.3**

  - [ ]* 10.4 Write property test for parser round-trip (Property 8)
    - **Property 8: Parser command round-trip**
    - For all HamlibCommand variants (exhaustive), format as long-form string then parse → same variant
    - **Validates: Requirements 11.5**

- [x] 11. Final checkpoint — Full protocol compliance
  - Ensure all tests pass, ask the user if questions arise.

## Notes

- Tasks marked with `*` are optional and can be skipped for faster MVP
- All code changes are in `firmware/src/main.rs` (single-file approach per design decision)
- Tests run on host (`x86_64`) not on target — parser and logic functions are pure
- Property tests use `proptest` crate with 100 iterations minimum
- Each task references specific requirements for traceability
- Checkpoints ensure incremental validation
