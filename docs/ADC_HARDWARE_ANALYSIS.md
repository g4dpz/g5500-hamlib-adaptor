# ADC Hardware Analysis — Calibration Compatibility

Analysis of the PCB schematic (`pcb/g5500-net-interface.kicad_sch`) with respect to the firmware two-point ADC voltage calibration feature.

## Current Circuit

### Signal Path

```
G-5500 DIN-8 (J2)                    J5 Breakout Header
  Pin → EXT_AZ_VRAW ──────────────────→ Pin 8
  Pin → EXT_EL_VRAW ──────────────────→ Pin 1

                    Azimuth                          Elevation
                    ───────                          ─────────
  EXT_AZ_VRAW (0–5V)                  EXT_EL_VRAW (0–5V)
       │                                    │
      R3 (10K)                             R5 (10K)
       │                                    │
       ├──── AZ_ADC ──→ GP26 (ADC0)        ├──── EL_ADC ──→ GP27 (ADC1)
       │                                    │
      R2 (10K)                             R4 (10K)
       │                                    │
      GND                                 GND
       │                                    │
      C (1µF)                              C (1µF)
    (to GND at ADC pin)                  (to GND at ADC pin)
```

### ADC Power

```
+5V → LM2937 (U1) → +3.3V → FB (Ferrite Bead) → ADC_VREF (Pin 35)
                              │
                         C (10µF + 1µF) bypass caps
```

### Relay Outputs

```
AZ_CW  (GP2) ──→ R1 (120Ω) ──→ EXT_AZ_CW  ──→ J5 Pin 3 → DIN-8
AZ_CCW (GP3) ──→              ──→ EXT_AZ_CCW ──→ J5 Pin 5 → DIN-8
EL_UP  (GP4) ──→ R6 (120Ω) ──→ EXT_EL_UP  ──→ J5 Pin 2 → DIN-8
EL_DN  (GP5) ──→              ──→ EXT_EL_DN  ──→ J5 Pin 7 → DIN-8
```

## Voltage Divider Characteristics

| Parameter | Value |
|-----------|-------|
| Upper resistor (R3/R5) | 10KΩ |
| Lower resistor (R2/R4) | 10KΩ |
| Divider ratio | 0.5 |
| G-5500 output range | 0–5V |
| ADC input range | 0–2.5V |
| RP2040 ADC reference | 3.3V |
| Usable ADC range | 0–3103 / 4096 counts (76%) |
| Resolution | ~0.145° per count (azimuth, 450° range) |
| Source impedance at ADC | 5KΩ (two 10K in parallel) |
| RC filter cutoff (with 1µF) | ~32 Hz |

## Compatibility with Firmware Calibration

The existing hardware is fully compatible with the two-point calibration feature. No PCB changes are required.

The firmware calibration captures actual raw ADC readings at two known physical rotator positions and derives a per-axis linear mapping (`degrees = slope × raw + intercept`). This approach inherently compensates for:

- Resistor tolerance in the voltage divider (1% or 5% parts)
- G-5500 potentiometer non-linearity and wear
- ADC reference voltage variation
- Board-to-board manufacturing differences
- Cable resistance and contact resistance at the DIN-8 connector

The calibration parameters persist in flash (Config v2, 60-byte layout) and survive power cycles.

## Potential Hardware Improvements

These are optional enhancements — none are required for the calibration feature to work.

### 1. Optimise Voltage Divider Ratio

The 10K/10K divider maps 0–5V to 0–2.5V, using only 76% of the ADC range. A different ratio could improve resolution:

| Upper R | Lower R | Ratio | ADC Range (5V in) | % of 4096 | Az Resolution |
|---------|---------|-------|--------------------|-----------|---------------|
| 10K | 10K | 0.500 | 0–2500mV (0–3103) | 76% | 0.145°/count |
| 6.8K | 10K | 0.595 | 0–2976mV (0–3694) | 90% | 0.122°/count |
| 5.6K | 10K | 0.641 | 0–3205mV (0–3978) | 97% | 0.113°/count |

With firmware calibration, the actual divider ratio doesn't matter for accuracy — calibration maps whatever raw range you get to the correct degrees. A wider raw range just gives more counts per degree (better granularity).

If changing resistors, keep the lower resistor at 10K to maintain a reasonable source impedance for the ADC sample-and-hold capacitor.

### 2. Improved Low-Pass Filtering

The current 5KΩ source impedance + 1µF cap gives a ~32 Hz cutoff. For a rotator position signal that changes at most a few degrees per second, this is higher than necessary.

Options:
- Add a 100nF ceramic cap directly at the ADC pin (in addition to the existing 1µF) for high-frequency noise rejection
- Increase to 10µF for a ~3.2 Hz cutoff — better noise rejection, still fast enough for rotator tracking
- Add a dedicated RC stage: 1KΩ series + 10µF to GND before the ADC pin (1.6 Hz cutoff)

The firmware already averages 512 samples over 100ms, which provides significant software filtering. Hardware filtering is complementary — it reduces aliasing and high-frequency noise before sampling.

### 3. RP2040 ADC DNL Spikes

The RP2040 ADC has documented differential non-linearity (DNL) spikes at codes 512, 1536, 2560, and 3584. The firmware already handles this by skipping these exact values during the 512-sample averaging window.

No hardware mitigation needed. The two-point calibration is unaffected since it operates on averaged values, not individual samples.

### 4. External ADC (Major Upgrade)

For significantly better ADC performance, an external ADC could be added using spare GPIO pins:

| Option | Interface | Resolution | Spare GPIOs | Notes |
|--------|-----------|------------|-------------|-------|
| ADS1115 | I2C (GP0/GP1) | 16-bit | GP0, GP1 | PGA, differential inputs, 860 SPS max |
| MCP3202 | SPI (GP6-GP9) | 12-bit | GP6, GP7, GP8, GP9 | Dual channel, 100 kSPS |
| ADS1015 | I2C (GP0/GP1) | 12-bit | GP0, GP1 | Cheaper ADS1115, 3300 SPS |

This would eliminate all RP2040 ADC quirks (DNL spikes, noisy reference, limited ENOB). Likely overkill for rotator positioning where ±1° accuracy is more than sufficient, but worth noting if precision becomes a goal.

Available spare GPIOs: 0, 1, 6–14, 22, 28.

## Conclusion

The existing PCB provides a stable, repeatable analog signal path that works well with the firmware two-point calibration. The 10K/10K voltage dividers, ferrite-bead-filtered ADC reference, and bypass capacitors give a clean enough signal for the 512-sample averaging to produce reliable position readings.

The firmware calibration compensates for all the analog chain tolerances, making hardware precision improvements a matter of diminishing returns. The most impactful change would be optimising the divider ratio (option 1) for better resolution, but even this is optional given the calibration's ability to map whatever range exists.
