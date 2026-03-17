use proptest::prelude::*;

// ============================================================================
// CRC8-CCITT (duplicated from firmware/src/crc8_ccitt.rs)
// ============================================================================

const CRC8_CCITT_TABLE: [u8; 256] = [
    0x00, 0x07, 0x0E, 0x09, 0x1C, 0x1B, 0x12, 0x15,
    0x38, 0x3F, 0x36, 0x31, 0x24, 0x23, 0x2A, 0x2D,
    0x70, 0x77, 0x7E, 0x79, 0x6C, 0x6B, 0x62, 0x65,
    0x48, 0x4F, 0x46, 0x41, 0x54, 0x53, 0x5A, 0x5D,
    0xE0, 0xE7, 0xEE, 0xE9, 0xFC, 0xFB, 0xF2, 0xF5,
    0xD8, 0xDF, 0xD6, 0xD1, 0xC4, 0xC3, 0xCA, 0xCD,
    0x90, 0x97, 0x9E, 0x99, 0x8C, 0x8B, 0x82, 0x85,
    0xA8, 0xAF, 0xA6, 0xA1, 0xB4, 0xB3, 0xBA, 0xBD,
    0xC7, 0xC0, 0xC9, 0xCE, 0xDB, 0xDC, 0xD5, 0xD2,
    0xFF, 0xF8, 0xF1, 0xF6, 0xE3, 0xE4, 0xED, 0xEA,
    0xB7, 0xB0, 0xB9, 0xBE, 0xAB, 0xAC, 0xA5, 0xA2,
    0x8F, 0x88, 0x81, 0x86, 0x93, 0x94, 0x9D, 0x9A,
    0x27, 0x20, 0x29, 0x2E, 0x3B, 0x3C, 0x35, 0x32,
    0x1F, 0x18, 0x11, 0x16, 0x03, 0x04, 0x0D, 0x0A,
    0x57, 0x50, 0x59, 0x5E, 0x4B, 0x4C, 0x45, 0x42,
    0x6F, 0x68, 0x61, 0x66, 0x73, 0x74, 0x7D, 0x7A,
    0x89, 0x8E, 0x87, 0x80, 0x95, 0x92, 0x9B, 0x9C,
    0xB1, 0xB6, 0xBF, 0xB8, 0xAD, 0xAA, 0xA3, 0xA4,
    0xF9, 0xFE, 0xF7, 0xF0, 0xE5, 0xE2, 0xEB, 0xEC,
    0xC1, 0xC6, 0xCF, 0xC8, 0xDD, 0xDA, 0xD3, 0xD4,
    0x69, 0x6E, 0x67, 0x60, 0x75, 0x72, 0x7B, 0x7C,
    0x51, 0x56, 0x5F, 0x58, 0x4D, 0x4A, 0x43, 0x44,
    0x19, 0x1E, 0x17, 0x10, 0x05, 0x02, 0x0B, 0x0C,
    0x21, 0x26, 0x2F, 0x28, 0x3D, 0x3A, 0x33, 0x34,
    0x4E, 0x49, 0x40, 0x47, 0x52, 0x55, 0x5C, 0x5B,
    0x76, 0x71, 0x78, 0x7F, 0x6A, 0x6D, 0x64, 0x63,
    0x3E, 0x39, 0x30, 0x37, 0x22, 0x25, 0x2C, 0x2B,
    0x06, 0x01, 0x08, 0x0F, 0x1A, 0x1D, 0x14, 0x13,
    0xAE, 0xA9, 0xA0, 0xA7, 0xB2, 0xB5, 0xBC, 0xBB,
    0x96, 0x91, 0x98, 0x9F, 0x8A, 0x8D, 0x84, 0x83,
    0xDE, 0xD9, 0xD0, 0xD7, 0xC2, 0xC5, 0xCC, 0xCB,
    0xE6, 0xE1, 0xE8, 0xEF, 0xFA, 0xFD, 0xF4, 0xF3,
];

fn crc8_ccitt_buffer(data: &[u8]) -> u8 {
    let mut crc: u8 = 0x00;
    for &b in data {
        crc = CRC8_CCITT_TABLE[(crc ^ b) as usize];
    }
    crc
}

fn crc8_ccitt_validate(data: &[u8]) -> bool {
    if data.len() < 2 { return false; }
    crc8_ccitt_buffer(&data[..data.len() - 1]) == data[data.len() - 1]
}

// ============================================================================
// Config struct and serialization (duplicated from firmware/src/config.rs)
// ============================================================================

const CONFIG_MAGIC: u8 = 0xAE;
const CONFIG_VERSION: u8 = 0x02;
const CONFIG_SIZE: usize = 60;
const CONFIG_SIZE_V1: usize = 24;
const FLAG_STATIC_IP_ENABLED: u8 = 0x01;
const FLAG_CALIBRATION_VALID: u8 = 0x01;

#[derive(Clone, Debug)]
struct Config {
    static_ip_enabled: bool,
    static_ip: [u8; 4],
    az_cal_offset: f32,
    el_cal_offset: f32,
    park_az: f32,
    park_el: f32,
    calibration_valid: bool,
    az_raw_low: f32,
    az_raw_high: f32,
    el_raw_low: f32,
    el_raw_high: f32,
    az_deg_low: f32,
    az_deg_high: f32,
    el_deg_low: f32,
    el_deg_high: f32,
}

impl Config {
    fn default_config() -> Self {
        Config {
            static_ip_enabled: false,
            static_ip: [192, 168, 1, 100],
            az_cal_offset: 0.0, el_cal_offset: 0.0,
            park_az: 180.0, park_el: 0.0,
            calibration_valid: false,
            az_raw_low: 0.0, az_raw_high: 0.0,
            el_raw_low: 0.0, el_raw_high: 0.0,
            az_deg_low: 0.0, az_deg_high: 0.0,
            el_deg_low: 0.0, el_deg_high: 0.0,
        }
    }

    fn to_bytes(&self) -> [u8; CONFIG_SIZE] {
        let mut buf = [0u8; CONFIG_SIZE];
        buf[0] = CONFIG_MAGIC;
        buf[1] = CONFIG_VERSION;
        buf[2] = if self.static_ip_enabled { FLAG_STATIC_IP_ENABLED } else { 0 };
        buf[3..7].copy_from_slice(&self.static_ip);
        buf[7..11].copy_from_slice(&self.az_cal_offset.to_le_bytes());
        buf[11..15].copy_from_slice(&self.el_cal_offset.to_le_bytes());
        buf[15..19].copy_from_slice(&self.park_az.to_le_bytes());
        buf[19..23].copy_from_slice(&self.park_el.to_le_bytes());
        buf[23] = if self.calibration_valid { FLAG_CALIBRATION_VALID } else { 0 };
        buf[24..28].copy_from_slice(&self.az_raw_low.to_le_bytes());
        buf[28..32].copy_from_slice(&self.az_raw_high.to_le_bytes());
        buf[32..36].copy_from_slice(&self.el_raw_low.to_le_bytes());
        buf[36..40].copy_from_slice(&self.el_raw_high.to_le_bytes());
        buf[40..44].copy_from_slice(&self.az_deg_low.to_le_bytes());
        buf[44..48].copy_from_slice(&self.az_deg_high.to_le_bytes());
        buf[48..52].copy_from_slice(&self.el_deg_low.to_le_bytes());
        buf[52..56].copy_from_slice(&self.el_deg_high.to_le_bytes());
        // bytes 56..59 reserved (0x00)
        buf[59] = crc8_ccitt_buffer(&buf[..59]);
        buf
    }

    fn from_bytes(buf: &[u8; CONFIG_SIZE]) -> Option<Self> {
        if buf[0] != CONFIG_MAGIC { return None; }
        match buf[1] {
            0x01 => Self::from_bytes_v1(buf),
            0x02 => Self::from_bytes_v2(buf),
            _ => None,
        }
    }

    fn from_bytes_v2(buf: &[u8; CONFIG_SIZE]) -> Option<Self> {
        if !crc8_ccitt_validate(buf) { return None; }
        Some(Config {
            static_ip_enabled: (buf[2] & FLAG_STATIC_IP_ENABLED) != 0,
            static_ip: [buf[3], buf[4], buf[5], buf[6]],
            az_cal_offset: f32::from_le_bytes([buf[7], buf[8], buf[9], buf[10]]),
            el_cal_offset: f32::from_le_bytes([buf[11], buf[12], buf[13], buf[14]]),
            park_az: f32::from_le_bytes([buf[15], buf[16], buf[17], buf[18]]),
            park_el: f32::from_le_bytes([buf[19], buf[20], buf[21], buf[22]]),
            calibration_valid: (buf[23] & FLAG_CALIBRATION_VALID) != 0,
            az_raw_low: f32::from_le_bytes([buf[24], buf[25], buf[26], buf[27]]),
            az_raw_high: f32::from_le_bytes([buf[28], buf[29], buf[30], buf[31]]),
            el_raw_low: f32::from_le_bytes([buf[32], buf[33], buf[34], buf[35]]),
            el_raw_high: f32::from_le_bytes([buf[36], buf[37], buf[38], buf[39]]),
            az_deg_low: f32::from_le_bytes([buf[40], buf[41], buf[42], buf[43]]),
            az_deg_high: f32::from_le_bytes([buf[44], buf[45], buf[46], buf[47]]),
            el_deg_low: f32::from_le_bytes([buf[48], buf[49], buf[50], buf[51]]),
            el_deg_high: f32::from_le_bytes([buf[52], buf[53], buf[54], buf[55]]),
        })
    }

    fn from_bytes_v1(buf: &[u8; CONFIG_SIZE]) -> Option<Self> {
        if !crc8_ccitt_validate(&buf[..CONFIG_SIZE_V1]) { return None; }
        Some(Config {
            static_ip_enabled: (buf[2] & FLAG_STATIC_IP_ENABLED) != 0,
            static_ip: [buf[3], buf[4], buf[5], buf[6]],
            az_cal_offset: f32::from_le_bytes([buf[7], buf[8], buf[9], buf[10]]),
            el_cal_offset: f32::from_le_bytes([buf[11], buf[12], buf[13], buf[14]]),
            park_az: f32::from_le_bytes([buf[15], buf[16], buf[17], buf[18]]),
            park_el: f32::from_le_bytes([buf[19], buf[20], buf[21], buf[22]]),
            calibration_valid: false,
            az_raw_low: 0.0, az_raw_high: 0.0,
            el_raw_low: 0.0, el_raw_high: 0.0,
            az_deg_low: 0.0, az_deg_high: 0.0,
            el_deg_low: 0.0, el_deg_high: 0.0,
        })
    }

    /// Serialize as v1 (24 bytes) for migration testing
    fn to_bytes_v1(&self) -> [u8; CONFIG_SIZE_V1] {
        let mut buf = [0u8; CONFIG_SIZE_V1];
        buf[0] = CONFIG_MAGIC;
        buf[1] = 0x01;
        buf[2] = if self.static_ip_enabled { FLAG_STATIC_IP_ENABLED } else { 0 };
        buf[3..7].copy_from_slice(&self.static_ip);
        buf[7..11].copy_from_slice(&self.az_cal_offset.to_le_bytes());
        buf[11..15].copy_from_slice(&self.el_cal_offset.to_le_bytes());
        buf[15..19].copy_from_slice(&self.park_az.to_le_bytes());
        buf[19..23].copy_from_slice(&self.park_el.to_le_bytes());
        buf[23] = crc8_ccitt_buffer(&buf[..23]);
        buf
    }

    /// Clear calibration fields (used by Property 8)
    fn clear_calibration(&mut self) {
        self.calibration_valid = false;
        self.az_raw_low = 0.0;
        self.az_raw_high = 0.0;
        self.el_raw_low = 0.0;
        self.el_raw_high = 0.0;
        self.az_deg_low = 0.0;
        self.az_deg_high = 0.0;
        self.el_deg_low = 0.0;
        self.el_deg_high = 0.0;
    }
}

// ============================================================================
// CalParams (duplicated from firmware/src/main.rs adc_task)
// ============================================================================

struct CalParams {
    az_slope: f32,
    az_intercept: f32,
    el_slope: f32,
    el_intercept: f32,
    az_offset: f32,
    el_offset: f32,
    use_cal_az: bool,
    use_cal_el: bool,
}

impl CalParams {
    fn from_config(cfg: &Config) -> Self {
        let use_cal_az = cfg.calibration_valid
            && (cfg.az_raw_high - cfg.az_raw_low).abs() >= 100.0
            && cfg.az_deg_high != cfg.az_deg_low;
        let use_cal_el = cfg.calibration_valid
            && (cfg.el_raw_high - cfg.el_raw_low).abs() >= 100.0
            && cfg.el_deg_high != cfg.el_deg_low;
        let (az_slope, az_intercept) = if use_cal_az {
            let s = (cfg.az_deg_high - cfg.az_deg_low) / (cfg.az_raw_high - cfg.az_raw_low);
            (s, cfg.az_deg_low - s * cfg.az_raw_low)
        } else { (0.0, 0.0) };
        let (el_slope, el_intercept) = if use_cal_el {
            let s = (cfg.el_deg_high - cfg.el_deg_low) / (cfg.el_raw_high - cfg.el_raw_low);
            (s, cfg.el_deg_low - s * cfg.el_raw_low)
        } else { (0.0, 0.0) };
        CalParams {
            az_slope, az_intercept, el_slope, el_intercept,
            az_offset: cfg.az_cal_offset, el_offset: cfg.el_cal_offset,
            use_cal_az, use_cal_el,
        }
    }
}

// ============================================================================
// Conversion functions (duplicated from firmware/src/main.rs adc_task)
// ============================================================================

fn convert_az(raw: f32, cal: &CalParams) -> f32 {
    if cal.use_cal_az {
        (cal.az_slope * raw + cal.az_intercept + cal.az_offset).clamp(0.0, 450.0)
    } else {
        const R: f32 = 10.0 / 20.0; // ladder ratio
        const LOW: f32 = 0.0 * R * (4096.0 / 3.3);
        const HIGH: f32 = 5.0 * R * (4096.0 / 3.3);
        ((raw - LOW) / ((HIGH - LOW) / 450.0) + cal.az_offset).clamp(0.0, 450.0)
    }
}

fn convert_el(raw: f32, cal: &CalParams) -> f32 {
    if cal.use_cal_el {
        (cal.el_slope * raw + cal.el_intercept + cal.el_offset).clamp(0.0, 180.0)
    } else {
        const R: f32 = 10.0 / 20.0;
        const LOW: f32 = 0.0 * R * (4096.0 / 3.3);
        const HIGH: f32 = 5.0 * R * (4096.0 / 3.3);
        ((raw - LOW) / ((HIGH - LOW) / 180.0) + cal.el_offset).clamp(0.0, 180.0)
    }
}

// ============================================================================
// Proptest strategy
// ============================================================================

fn arb_config() -> impl Strategy<Value = Config> {
    (
        any::<bool>(), any::<[u8; 4]>(),
        -50.0f32..50.0, -50.0f32..50.0,        // offsets
        0.0f32..450.0, 0.0f32..180.0,           // park
        any::<bool>(),                           // calibration_valid
        0.0f32..4095.0, 0.0f32..4095.0,         // az_raw_low/high
        0.0f32..4095.0,
        (0.0f32..4095.0, 0.0f32..450.0, 0.0f32..450.0, 0.0f32..180.0, 0.0f32..180.0),
    ).prop_map(|(sip_en, sip, azo, elo, paz, pel, cv,
                 arl, arh, erl, (erh, adl, adh, edl, edh))| {
        Config {
            static_ip_enabled: sip_en, static_ip: sip,
            az_cal_offset: azo, el_cal_offset: elo,
            park_az: paz, park_el: pel, calibration_valid: cv,
            az_raw_low: arl, az_raw_high: arh,
            el_raw_low: erl, el_raw_high: erh,
            az_deg_low: adl, az_deg_high: adh,
            el_deg_low: edl, el_deg_high: edh,
        }
    })
}

// ============================================================================
// Property Tests (P1–P9)
// ============================================================================

proptest! {
    // Property 1: Config v2 serialization round-trip
    // Validates: Requirements 1.1, 1.2, 1.3, 2.3
    #[test]
    fn prop1_config_v2_round_trip(cfg in arb_config()) {
        let bytes = cfg.to_bytes();
        let r = Config::from_bytes(&bytes).expect("round-trip must succeed");
        prop_assert_eq!(r.static_ip_enabled, cfg.static_ip_enabled);
        prop_assert_eq!(r.static_ip, cfg.static_ip);
        prop_assert_eq!(r.az_cal_offset.to_bits(), cfg.az_cal_offset.to_bits());
        prop_assert_eq!(r.el_cal_offset.to_bits(), cfg.el_cal_offset.to_bits());
        prop_assert_eq!(r.park_az.to_bits(), cfg.park_az.to_bits());
        prop_assert_eq!(r.park_el.to_bits(), cfg.park_el.to_bits());
        prop_assert_eq!(r.calibration_valid, cfg.calibration_valid);
        prop_assert_eq!(r.az_raw_low.to_bits(), cfg.az_raw_low.to_bits());
        prop_assert_eq!(r.az_raw_high.to_bits(), cfg.az_raw_high.to_bits());
        prop_assert_eq!(r.el_raw_low.to_bits(), cfg.el_raw_low.to_bits());
        prop_assert_eq!(r.el_raw_high.to_bits(), cfg.el_raw_high.to_bits());
        prop_assert_eq!(r.az_deg_low.to_bits(), cfg.az_deg_low.to_bits());
        prop_assert_eq!(r.az_deg_high.to_bits(), cfg.az_deg_high.to_bits());
        prop_assert_eq!(r.el_deg_low.to_bits(), cfg.el_deg_low.to_bits());
        prop_assert_eq!(r.el_deg_high.to_bits(), cfg.el_deg_high.to_bits());
    }

    // Property 2: CRC8 integrity invariant
    // Validates: Requirements 2.5
    #[test]
    fn prop2_crc8_integrity(cfg in arb_config()) {
        let bytes = cfg.to_bytes();
        prop_assert!(crc8_ccitt_validate(&bytes));
    }

    // Property 3: v1→v2 migration preserves existing fields
    // Validates: Requirements 2.2, 7.3
    #[test]
    fn prop3_v1_to_v2_migration(cfg in arb_config()) {
        let v1 = cfg.to_bytes_v1();
        let mut buf = [0xFFu8; CONFIG_SIZE];
        buf[..CONFIG_SIZE_V1].copy_from_slice(&v1);
        let m = Config::from_bytes(&buf).expect("v1 migration must succeed");
        prop_assert_eq!(m.static_ip_enabled, cfg.static_ip_enabled);
        prop_assert_eq!(m.static_ip, cfg.static_ip);
        prop_assert_eq!(m.az_cal_offset.to_bits(), cfg.az_cal_offset.to_bits());
        prop_assert_eq!(m.el_cal_offset.to_bits(), cfg.el_cal_offset.to_bits());
        prop_assert_eq!(m.park_az.to_bits(), cfg.park_az.to_bits());
        prop_assert_eq!(m.park_el.to_bits(), cfg.park_el.to_bits());
        prop_assert!(!m.calibration_valid);
        prop_assert_eq!(m.az_raw_low, 0.0);
        prop_assert_eq!(m.az_raw_high, 0.0);
        prop_assert_eq!(m.el_raw_low, 0.0);
        prop_assert_eq!(m.el_raw_high, 0.0);
        prop_assert_eq!(m.az_deg_low, 0.0);
        prop_assert_eq!(m.az_deg_high, 0.0);
        prop_assert_eq!(m.el_deg_low, 0.0);
        prop_assert_eq!(m.el_deg_high, 0.0);
    }

    // Property 4: Unknown version returns None
    // Validates: Requirements 2.4
    #[test]
    fn prop4_unknown_version_returns_none(version in 0u8..=255u8) {
        prop_assume!(version != 0x01 && version != 0x02);
        let mut buf = [0u8; CONFIG_SIZE];
        buf[0] = CONFIG_MAGIC;
        buf[1] = version;
        prop_assert!(Config::from_bytes(&buf).is_none());
    }

    // Property 5: Linear mapping reproduces reference points
    // Validates: Requirements 6.1, 6.2
    #[test]
    fn prop5_linear_mapping_reference_points(
        raw_low in 0.0f32..3000.0,
        raw_span in 100.0f32..1000.0,
        deg_low in 0.0f32..200.0,
        deg_span in 1.0f32..250.0,
    ) {
        let raw_high = raw_low + raw_span;
        let deg_high = deg_low + deg_span;
        let slope = (deg_high - deg_low) / (raw_high - raw_low);
        let intercept = deg_low - slope * raw_low;
        let at_low = slope * raw_low + intercept;
        let at_high = slope * raw_high + intercept;
        prop_assert!((at_low - deg_low).abs() < 0.01,
            "low: expected {}, got {}", deg_low, at_low);
        prop_assert!((at_high - deg_high).abs() < 0.01,
            "high: expected {}, got {}", deg_high, at_high);
    }

    // Property 6: Output clamping invariant
    // Validates: Requirements 6.4
    #[test]
    fn prop6_output_clamping(
        cfg in arb_config(),
        raw_az in 0.0f32..4095.0,
        raw_el in 0.0f32..4095.0,
    ) {
        let cal = CalParams::from_config(&cfg);
        let az = convert_az(raw_az, &cal);
        let el = convert_el(raw_el, &cal);
        prop_assert!(az >= 0.0 && az <= 450.0, "az {} out of range", az);
        prop_assert!(el >= 0.0 && el <= 180.0, "el {} out of range", el);
    }

    // Property 7: Offset is additive before clamping
    // Validates: Requirements 7.1, 7.2
    #[test]
    fn prop7_offset_additive(
        raw_low in 0.0f32..2000.0,
        raw_span in 100.0f32..1000.0,
        deg_low in 0.0f32..100.0,
        deg_span in 10.0f32..200.0,
        offset in -20.0f32..20.0,
        raw_test in 0.0f32..4095.0,
    ) {
        let raw_high = raw_low + raw_span;
        let deg_high = deg_low + deg_span;
        let cfg = Config {
            calibration_valid: true,
            az_raw_low: raw_low, az_raw_high: raw_high,
            az_deg_low: deg_low, az_deg_high: deg_high,
            el_raw_low: raw_low, el_raw_high: raw_high,
            el_deg_low: deg_low, el_deg_high: deg_high.min(180.0),
            az_cal_offset: offset, el_cal_offset: offset,
            ..Config::default_config()
        };
        let cal = CalParams::from_config(&cfg);
        let az = convert_az(raw_test, &cal);
        let linear = cal.az_slope * raw_test + cal.az_intercept;
        let expected = (linear + offset).clamp(0.0, 450.0);
        prop_assert!((az - expected).abs() < 0.01,
            "expected {}, got {}", expected, az);
    }

    // Property 8: Clear calibration zeroes all calibration fields
    // Validates: Requirements 5.1
    #[test]
    fn prop8_clear_calibration(cfg in arb_config()) {
        let mut cleared = cfg.clone();
        cleared.clear_calibration();
        prop_assert!(!cleared.calibration_valid);
        prop_assert_eq!(cleared.az_raw_low, 0.0);
        prop_assert_eq!(cleared.az_raw_high, 0.0);
        prop_assert_eq!(cleared.el_raw_low, 0.0);
        prop_assert_eq!(cleared.el_raw_high, 0.0);
        prop_assert_eq!(cleared.az_deg_low, 0.0);
        prop_assert_eq!(cleared.az_deg_high, 0.0);
        prop_assert_eq!(cleared.el_deg_low, 0.0);
        prop_assert_eq!(cleared.el_deg_high, 0.0);
        // Non-cal fields preserved
        prop_assert_eq!(cleared.static_ip_enabled, cfg.static_ip_enabled);
        prop_assert_eq!(cleared.static_ip, cfg.static_ip);
        prop_assert_eq!(cleared.az_cal_offset.to_bits(), cfg.az_cal_offset.to_bits());
        prop_assert_eq!(cleared.el_cal_offset.to_bits(), cfg.el_cal_offset.to_bits());
        prop_assert_eq!(cleared.park_az.to_bits(), cfg.park_az.to_bits());
        prop_assert_eq!(cleared.park_el.to_bits(), cfg.park_el.to_bits());
    }

    // Property 9: calibration_valid is set when both reference points are present
    // Validates: Requirements 4.3
    #[test]
    fn prop9_calibration_valid_when_both_points(
        raw_low_az in 0.0f32..2000.0,
        raw_low_el in 0.0f32..2000.0,
        raw_high_az in 2100.0f32..4095.0,
        raw_high_el in 2100.0f32..4095.0,
        deg_low_az in 0.0f32..100.0,
        deg_low_el in 0.0f32..50.0,
        deg_high_az in 200.0f32..450.0,
        deg_high_el in 90.0f32..180.0,
    ) {
        let cfg = Config {
            calibration_valid: true,
            az_raw_low: raw_low_az, az_raw_high: raw_high_az,
            el_raw_low: raw_low_el, el_raw_high: raw_high_el,
            az_deg_low: deg_low_az, az_deg_high: deg_high_az,
            el_deg_low: deg_low_el, el_deg_high: deg_high_el,
            ..Config::default_config()
        };
        prop_assert!(cfg.calibration_valid);
        let cal = CalParams::from_config(&cfg);
        prop_assert!(cal.use_cal_az, "Az cal should be valid");
        prop_assert!(cal.use_cal_el, "El cal should be valid");
    }
}
