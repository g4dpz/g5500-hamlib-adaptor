# Requirements Document

## Introduction

The G-5500 Hamlib Adaptor firmware converts raw ADC readings to rotator position in degrees using hardcoded theoretical voltage ranges (0–5V through a voltage divider). In practice, real-world component tolerances and rotator wiring variations mean the theoretical mapping is inaccurate. This feature adds a two-point calibration mechanism accessible through the existing web UI, allowing users to capture actual ADC readings at known physical positions and derive a per-installation linear mapping. Calibration data is persisted in flash alongside the existing configuration.

## Glossary

- **Firmware**: The G-5500 Hamlib Adaptor embedded application running on the RP2040
- **ADC_Task**: The Embassy async task that samples azimuth and elevation ADC channels at 10kHz, averages 512 samples per 100ms cycle, and publishes position to shared state
- **Config**: The persistent configuration struct stored in a single 4KB flash sector with CRC8 validation
- **Web_UI**: The minimal HTTP server on port 80 that serves the status/configuration page and handles POST requests
- **Calibration_Data**: The set of four raw ADC reference values (az_raw_low, az_raw_high, el_raw_low, el_raw_high) and their corresponding known degree values, used to derive a linear ADC-to-degrees mapping
- **Capture_Low**: The action of recording the current averaged raw ADC values as the low reference point for calibration
- **Capture_High**: The action of recording the current averaged raw ADC values as the high reference point for calibration
- **Theoretical_Constants**: The current hardcoded ADC-to-degrees conversion values derived from ideal 0–5V range through a 10k/10k voltage divider at 3.3V ADC reference
- **Flash_Format_Version**: The version byte in the flash config header, used to detect and migrate between config layout revisions
- **Linear_Mapping**: A first-order (y = mx + b) conversion from raw ADC value to degrees, derived from two known reference points
- **STORED_CONFIG**: The global shared mutex-protected Config instance used by all tasks at runtime

## Requirements

### Requirement 1: Expand Config Struct with Calibration Fields

**User Story:** As a firmware developer, I want the Config struct to store two-point calibration reference data, so that calibration persists across power cycles.

#### Acceptance Criteria

1. THE Config SHALL include fields for azimuth low raw ADC value (az_raw_low: f32), azimuth high raw ADC value (az_raw_high: f32), elevation low raw ADC value (el_raw_low: f32), and elevation high raw ADC value (el_raw_high: f32)
2. THE Config SHALL include fields for the known degree values at the low reference point (az_deg_low: f32, el_deg_low: f32) and the high reference point (az_deg_high: f32, el_deg_high: f32)
3. THE Config SHALL include a boolean field (calibration_valid: bool) indicating whether calibration data has been fully captured
4. THE Config SHALL default calibration_valid to false and all calibration raw/degree fields to 0.0 when no calibration data exists

### Requirement 2: Flash Format Version Migration

**User Story:** As a firmware developer, I want the flash format to version-bump cleanly, so that existing devices upgrade without losing their current settings.

#### Acceptance Criteria

1. THE Config SHALL use flash format version 0x02 for the new layout that includes calibration fields
2. WHEN the Firmware reads a flash sector with version 0x01, THE Config SHALL migrate the existing fields (static IP, cal offsets, park position) into the version 0x02 layout and set calibration_valid to false
3. WHEN the Firmware reads a flash sector with version 0x02, THE Config SHALL deserialize all fields including calibration data
4. IF the Firmware reads a flash sector with an unrecognized version, THEN THE Config SHALL fall back to default values
5. THE Config serialization SHALL produce a byte buffer with CRC8-CCITT validation covering all bytes preceding the CRC byte

### Requirement 3: Capture Low Calibration Endpoint

**User Story:** As a user, I want to click "Capture Low" in the web UI after positioning my rotator at a known low position, so that the firmware records the current raw ADC readings as the low reference point.

#### Acceptance Criteria

1. WHEN the Web_UI receives a POST request to /cal/capture-low with form fields for az_deg_low and el_deg_low, THE Web_UI SHALL read the current averaged raw ADC values from CURRENT_AZ_EL_RAW
2. WHEN the Web_UI processes a valid Capture_Low request, THE Web_UI SHALL store the captured raw ADC values and the user-provided degree values into the az_raw_low, el_raw_low, az_deg_low, and el_deg_low fields of STORED_CONFIG
3. WHEN the Web_UI completes a Capture_Low request, THE Web_UI SHALL persist the updated Config to flash
4. WHEN the Web_UI completes a Capture_Low request, THE Web_UI SHALL respond with an HTTP 303 redirect to the status page

### Requirement 4: Capture High Calibration Endpoint

**User Story:** As a user, I want to click "Capture High" in the web UI after positioning my rotator at a known high position, so that the firmware records the current raw ADC readings as the high reference point.

#### Acceptance Criteria

1. WHEN the Web_UI receives a POST request to /cal/capture-high with form fields for az_deg_high and el_deg_high, THE Web_UI SHALL read the current averaged raw ADC values from CURRENT_AZ_EL_RAW
2. WHEN the Web_UI processes a valid Capture_High request, THE Web_UI SHALL store the captured raw ADC values and the user-provided degree values into the az_raw_high, el_raw_high, az_deg_high, and el_deg_high fields of STORED_CONFIG
3. WHEN both Capture_Low and Capture_High data are present (all four raw values and degree values are populated), THE Web_UI SHALL set calibration_valid to true
4. WHEN the Web_UI completes a Capture_High request, THE Web_UI SHALL persist the updated Config to flash
5. WHEN the Web_UI completes a Capture_High request, THE Web_UI SHALL respond with an HTTP 303 redirect to the status page

### Requirement 5: Clear Calibration Endpoint

**User Story:** As a user, I want to clear calibration data and revert to theoretical defaults, so that I can start over if calibration is incorrect.

#### Acceptance Criteria

1. WHEN the Web_UI receives a POST request to /cal/clear, THE Web_UI SHALL reset calibration_valid to false and set all calibration raw and degree fields to 0.0 in STORED_CONFIG
2. WHEN the Web_UI completes a clear calibration request, THE Web_UI SHALL persist the updated Config to flash
3. WHEN the Web_UI completes a clear calibration request, THE Web_UI SHALL respond with an HTTP 303 redirect to the status page

### Requirement 6: ADC Task Uses Calibration Data for Degree Conversion

**User Story:** As a user, I want the firmware to use my calibration data for position conversion, so that the reported degrees match my actual rotator positions.

#### Acceptance Criteria

1. WHILE calibration_valid is true in STORED_CONFIG, THE ADC_Task SHALL compute azimuth degrees using the Linear_Mapping derived from (az_raw_low, az_deg_low) and (az_raw_high, az_deg_high)
2. WHILE calibration_valid is true in STORED_CONFIG, THE ADC_Task SHALL compute elevation degrees using the Linear_Mapping derived from (el_raw_low, el_deg_low) and (el_raw_high, el_deg_high)
3. WHILE calibration_valid is false in STORED_CONFIG, THE ADC_Task SHALL compute degrees using the existing Theoretical_Constants (0–5V range through voltage divider)
4. THE ADC_Task SHALL clamp computed azimuth degrees to the range 0.0–450.0 and elevation degrees to the range 0.0–180.0 regardless of calibration source
5. THE ADC_Task SHALL read calibration data from STORED_CONFIG once at startup and re-read when the config changes, without adding per-sample overhead to the 10kHz sampling loop

### Requirement 7: Calibration Offset Compatibility

**User Story:** As a user with existing az_cal_offset/el_cal_offset values, I want those offsets to continue working as fine-tuning adjustments, so that I do not lose my existing configuration.

#### Acceptance Criteria

1. WHEN calibration_valid is true, THE ADC_Task SHALL apply az_cal_offset and el_cal_offset as additive adjustments after the Linear_Mapping conversion and before clamping
2. WHEN calibration_valid is false, THE ADC_Task SHALL apply az_cal_offset and el_cal_offset as additive adjustments after the Theoretical_Constants conversion and before clamping
3. THE Config SHALL preserve az_cal_offset and el_cal_offset fields in the version 0x02 flash layout at the same semantic purpose as version 0x01

### Requirement 8: Calibration UI Section in Web Page

**User Story:** As a user, I want to see calibration status and controls on the web UI, so that I can perform and manage calibration without external tools.

#### Acceptance Criteria

1. THE Web_UI status page SHALL display a "Calibration" section showing the current calibration state (calibrated or uncalibrated)
2. WHEN calibration_valid is true, THE Web_UI SHALL display the stored calibration reference values (raw ADC values and corresponding degrees for both low and high points)
3. THE Web_UI calibration section SHALL include a "Capture Low" form with pre-filled default degree fields (0.0 Az, 0.0 El) and a submit button
4. THE Web_UI calibration section SHALL include a "Capture High" form with pre-filled default degree fields (450.0 Az, 180.0 El) and a submit button
5. THE Web_UI calibration section SHALL include a "Clear Calibration" button that posts to the clear endpoint
6. THE Web_UI calibration section SHALL display the current live raw ADC values so the user can verify the rotator position before capturing

### Requirement 9: Calibration Data Validation

**User Story:** As a firmware developer, I want calibration data to be validated before use, so that invalid calibration does not produce nonsensical position readings.

#### Acceptance Criteria

1. IF the difference between az_raw_high and az_raw_low is less than 100 ADC counts, THEN THE ADC_Task SHALL treat calibration as invalid and fall back to Theoretical_Constants for azimuth
2. IF the difference between el_raw_high and el_raw_low is less than 100 ADC counts, THEN THE ADC_Task SHALL treat calibration as invalid and fall back to Theoretical_Constants for elevation
3. IF az_deg_high equals az_deg_low, THEN THE ADC_Task SHALL treat calibration as invalid and fall back to Theoretical_Constants for azimuth
4. IF el_deg_high equals el_deg_low, THEN THE ADC_Task SHALL treat calibration as invalid and fall back to Theoretical_Constants for elevation
