# Requirements: Full rotctld Protocol Support

## Introduction

The G-5500 Hamlib Adaptor firmware currently implements a subset of the rotctld protocol sufficient for Gpredict satellite tracking. To support the full range of Hamlib client applications (e.g., rotctl CLI, xdx, cqrlog, wsjt-x, N1MM+), the firmware must implement all rotctld commands defined in the Hamlib command dispatch table (`rotctl_parse.c`). Commands not applicable to the G-5500 hardware must return protocol-compliant error codes rather than being silently ignored. The existing `dump_state` output must be rewritten to match the wire format expected by the NET rotctl backend (`netrotctl.c`).

## Glossary

- **Firmware**: The G-5500 Hamlib Adaptor embedded firmware running on the W5500-EVB-Pico (RP2040)
- **Parser**: The `Command::parse()` nom-based protocol parser in the Firmware
- **Control_Task**: The Embassy async task that drives rotator relay outputs based on demand state
- **Rotctld_Protocol**: The TCP text protocol defined by Hamlib's rotctld daemon for remote rotator control
- **ERP**: Extended Response Protocol, activated by a punctuation prefix character on the command line
- **RPRT**: The protocol return code format used by rotctld (`RPRT <code>\n`)
- **RIG_ENIMPL**: Hamlib error code -4, indicating a command is not implemented for this rotator
- **RIG_EINVAL**: Hamlib error code -2, indicating invalid parameter
- **Direction_Code**: Integer bitmask for Move command directions (2=UP, 4=DOWN, 8=LEFT, 16=RIGHT, 32=UP_LEFT, 64=UP_RIGHT, 128=DOWN_LEFT, 256=DOWN_RIGHT)
- **Config_Parameter**: A named runtime configuration value (e.g., min_az, max_az, park_az, park_el) stored in flash

## Requirements

### Requirement 1: Protocol-Compliant dump_state (Rewrite)

**User Story:** As a Hamlib client application, I want `dump_state` to return the standard wire format, so that the NET rotctl backend can parse rotator capabilities on connection.

#### Acceptance Criteria

1. WHEN the Firmware receives a `\dump_state` command, THE Firmware SHALL respond with the protocol version number `1` on the first line
2. WHEN the Firmware receives a `\dump_state` command, THE Firmware SHALL respond with the rotator model number on the second line
3. WHEN the Firmware receives a `\dump_state` command, THE Firmware SHALL respond with `min_az=<value>` on the third line, where `<value>` is the minimum azimuth in degrees (0.000000)
4. WHEN the Firmware receives a `\dump_state` command, THE Firmware SHALL respond with `max_az=<value>` on the fourth line, where `<value>` is the maximum azimuth in degrees (450.000000)
5. WHEN the Firmware receives a `\dump_state` command, THE Firmware SHALL respond with `min_el=<value>` on the fifth line, where `<value>` is the minimum elevation in degrees (0.000000)
6. WHEN the Firmware receives a `\dump_state` command, THE Firmware SHALL respond with `max_el=<value>` on the sixth line, where `<value>` is the maximum elevation in degrees (180.000000)
7. WHEN the Firmware receives a `\dump_state` command, THE Firmware SHALL respond with `south_zero=0` on the seventh line
8. WHEN the Firmware receives a `\dump_state` command, THE Firmware SHALL respond with `rot_type=AzEl` on the eighth line
9. WHEN the Firmware receives a `\dump_state` command, THE Firmware SHALL respond with `done` on the ninth line
10. THE Firmware SHALL terminate each line of the dump_state response with a newline character (`\n`)

### Requirement 2: Diagonal Move Directions

**User Story:** As a Hamlib client application, I want to send diagonal move commands, so that the rotator can move in combined azimuth and elevation directions simultaneously.

#### Acceptance Criteria

1. WHEN the Firmware receives a Move command with Direction_Code 32 (UP_LEFT), THE Control_Task SHALL activate both the UP and CCW relays simultaneously
2. WHEN the Firmware receives a Move command with Direction_Code 64 (UP_RIGHT), THE Control_Task SHALL activate both the UP and CW relays simultaneously
3. WHEN the Firmware receives a Move command with Direction_Code 128 (DOWN_LEFT), THE Control_Task SHALL activate both the DN and CCW relays simultaneously
4. WHEN the Firmware receives a Move command with Direction_Code 256 (DOWN_RIGHT), THE Control_Task SHALL activate both the DN and CW relays simultaneously
5. WHEN the Firmware receives a Move command with a valid Direction_Code (2, 4, 8, 16, 32, 64, 128, or 256), THE Firmware SHALL respond with `RPRT 0`
6. WHEN the Firmware receives a Move command with an invalid Direction_Code, THE Firmware SHALL respond with `RPRT -2` (RIG_EINVAL)

### Requirement 3: Reset Command Argument Parsing

**User Story:** As a Hamlib client application, I want to send a reset command with a reset type argument, so that the protocol exchange matches the rotctld specification.

#### Acceptance Criteria

1. WHEN the Firmware receives `R <reset_type>` or `\reset <reset_type>`, THE Parser SHALL accept and parse the integer reset type argument
2. WHEN the Firmware receives a Reset command with any valid reset type argument, THE Firmware SHALL trigger a full system reset via watchdog
3. WHEN the Firmware receives a bare `R` or `\reset` without an argument, THE Parser SHALL still accept the command for backward compatibility

### Requirement 4: get_status Command

**User Story:** As a Hamlib client application, I want to query the rotator status flags, so that I can determine whether the rotator is currently moving or stationary.

#### Acceptance Criteria

1. WHEN the Firmware receives `s` or `\get_status`, THE Firmware SHALL return the current rotator status flags as an integer value
2. WHILE the Control_Task is actively driving relays toward a demand position, THE Firmware SHALL report a status value indicating the rotator is moving
3. WHILE the Control_Task is idle with no active demand, THE Firmware SHALL report a status value indicating the rotator is stopped

### Requirement 5: set_conf and get_conf Commands

**User Story:** As a Hamlib client application, I want to read and write runtime configuration parameters, so that I can adjust rotator settings such as azimuth limits and park position without reflashing.

#### Acceptance Criteria

1. WHEN the Firmware receives `C <token> <value>` or `\set_conf <token> <value>`, THE Parser SHALL parse the token string and value string arguments
2. WHEN the Firmware receives a set_conf command with a recognized Config_Parameter token, THE Firmware SHALL update the corresponding configuration value and respond with `RPRT 0`
3. WHEN the Firmware receives a set_conf command with an unrecognized token, THE Firmware SHALL respond with `RPRT -2` (RIG_EINVAL)
4. WHEN the Firmware receives `\get_conf <token>` or the get_conf command with a recognized Config_Parameter token, THE Firmware SHALL return the current value of that parameter
5. WHEN the Firmware receives a get_conf command with an unrecognized token, THE Firmware SHALL respond with `RPRT -2` (RIG_EINVAL)
6. THE Firmware SHALL support the following Config_Parameter tokens: `min_az`, `max_az`, `min_el`, `max_el`, `park_az`, `park_el`

### Requirement 6: dump_conf Command

**User Story:** As a Hamlib client application, I want to list available configuration parameters, so that I can discover which settings are adjustable on this rotator.

#### Acceptance Criteria

1. WHEN the Firmware receives `3` or `\dump_conf`, THE Firmware SHALL return a list of all supported Config_Parameter tokens with their current values
2. THE Firmware SHALL format each Config_Parameter as `<token>=<value>` on a separate line

### Requirement 7: Not-Implemented Command Stubs

**User Story:** As a Hamlib client application, I want unsupported commands to return the standard `RPRT -4` (RIG_ENIMPL) error code, so that client software can gracefully handle missing features rather than receiving an unrecognized command error.

#### Acceptance Criteria

1. WHEN the Firmware receives `V` or `\set_level`, THE Firmware SHALL respond with `RPRT -4` (RIG_ENIMPL)
2. WHEN the Firmware receives `v` or `\get_level`, THE Firmware SHALL respond with `RPRT -4` (RIG_ENIMPL)
3. WHEN the Firmware receives `U` or `\set_func`, THE Firmware SHALL respond with `RPRT -4` (RIG_ENIMPL)
4. WHEN the Firmware receives `u` or `\get_func`, THE Firmware SHALL respond with `RPRT -4` (RIG_ENIMPL)
5. WHEN the Firmware receives `X` or `\set_parm`, THE Firmware SHALL respond with `RPRT -4` (RIG_ENIMPL)
6. WHEN the Firmware receives `x` or `\get_parm`, THE Firmware SHALL respond with `RPRT -4` (RIG_ENIMPL)
7. WHEN the Firmware receives `w` or `\send_cmd`, THE Firmware SHALL respond with `RPRT -4` (RIG_ENIMPL)

### Requirement 8: Unrecognized Command Error Code Correction

**User Story:** As a Hamlib client application, I want unrecognized commands to return the correct Hamlib error code, so that client software can distinguish between "not implemented" and "invalid command."

#### Acceptance Criteria

1. WHEN the Firmware receives an unrecognized command, THE Firmware SHALL respond with `RPRT -1` (RIG_EINVAL) in standard mode
2. WHEN the Firmware receives an unrecognized command with ERP active, THE Firmware SHALL respond with `RPRT -1`

### Requirement 9: Extended Response Protocol Compliance for All Commands

**User Story:** As a Hamlib client application using ERP mode, I want all commands to return properly formatted extended responses, so that automated parsing works consistently across the full command set.

#### Acceptance Criteria

1. WHEN ERP is active and a command succeeds, THE Firmware SHALL prefix the response with the long command name followed by a colon and newline (e.g., `set_level:\nRPRT -4\n`)
2. WHEN ERP is active and a command returns an error, THE Firmware SHALL include the long command name prefix before the RPRT error code
3. THE Firmware SHALL support ERP for all newly added commands (set_level, get_level, set_func, get_func, set_parm, get_parm, get_status, set_conf, get_conf, dump_conf, send_cmd)

### Requirement 10: Locator Commands (Optional — Client-Side)

**User Story:** As a Hamlib client application, I want the rotator to handle locator-related commands gracefully, so that clients sending these commands do not receive unexpected errors.

#### Acceptance Criteria

1. WHEN the Firmware receives any locator command (`L`, `l`, `D`, `d`, `E`, `e`, `B`, `A`, `a`), THE Firmware SHALL respond with `RPRT -4` (RIG_ENIMPL)
2. THE Parser SHALL recognize both the short character and long-form name for each locator command (`\lonlat2loc`, `\loc2lonlat`, `\dms2dec`, `\dec2dms`, `\dmmm2dec`, `\dec2dmmm`, `\qrb`, `\a_sp2a_lp`, `\d_sp2d_lp`)

### Requirement 11: Parser Recognition of All Protocol Commands

**User Story:** As a developer, I want the Parser to recognize every command in the rotctld dispatch table, so that no valid rotctld command falls through to the unrecognized command handler.

#### Acceptance Criteria

1. THE Parser SHALL recognize all of the following short-form commands: `P`, `p`, `K`, `S`, `R`, `M`, `V`, `v`, `U`, `u`, `X`, `x`, `C`, `_`, `s`, `w`, `1`, `3`, `q`
2. THE Parser SHALL recognize all of the following long-form commands: `\set_pos`, `\get_pos`, `\park`, `\stop`, `\reset`, `\move`, `\set_level`, `\get_level`, `\set_func`, `\get_func`, `\set_parm`, `\get_parm`, `\set_conf`, `\get_info`, `\get_status`, `\send_cmd`, `\dump_caps`, `\dump_conf`, `\dump_state`
3. THE Parser SHALL recognize the long-form locator commands: `\lonlat2loc`, `\loc2lonlat`, `\dms2dec`, `\dec2dms`, `\dmmm2dec`, `\dec2dmmm`, `\qrb`, `\a_sp2a_lp`, `\d_sp2d_lp`
4. THE Parser SHALL consume any trailing arguments for stub commands without failing to parse the command itself
5. FOR ALL recognized command strings, parsing then formatting the command name then parsing again SHALL produce the same HamlibCommand variant (round-trip property)

### Requirement 12: Memory Budget Compliance

**User Story:** As a developer, I want the full protocol implementation to fit within the existing memory budget, so that the firmware remains stable on the RP2040.

#### Acceptance Criteria

1. THE Firmware SHALL not exceed 200KB of static RAM usage after adding all new commands
2. THE Firmware SHALL not exceed 2MB of flash usage after adding all new commands
3. THE Firmware SHALL continue to use the existing 256-byte command buffer for all new commands without increasing buffer size
4. THE Firmware SHALL not introduce heap allocation for any new command handler
