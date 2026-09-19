// Per-strength video transform magnitudes recovered in spec §7.1.
// Ranges are [lo,hi]; amps are the ± symmetric scale; None = disabled at that strength.

#[derive(Debug, Clone, Copy)]
pub struct StrengthParams {
    pub rotation_amp: f64,             // deg, symmetric
    pub zoom: (f64, f64),              // range
    pub saturation: (f64, f64),        // range
    pub contrast_amp: f64,             // 1 ± amp
    pub brightness_static_amp: f64,    // symmetric
    pub hue_amp: f64,                  // deg, symmetric
    pub flicker: Option<(f64, f64)>,   // range, eq brightness expr amp
    pub noise: Option<(i64, i64)>,     // integer range c0s=c1s=c2s
    pub pad_base: Option<(i64, i64)>,  // integer range; px = 2*base
    pub speed: (f64, f64),             // range, setpts=1/speed
    pub pitch: Option<(f64, f64)>,     // range, asetrate*
    pub volume_amp: f64,               // dB, symmetric
}

pub fn params(strength: &str) -> StrengthParams {
    match strength {
        "minimal" => StrengthParams {
            rotation_amp: 0.3,
            zoom: (1.01, 1.02),
            saturation: (0.99, 1.01),
            contrast_amp: 0.005,
            brightness_static_amp: 0.005,
            hue_amp: 0.8,
            flicker: None,
            noise: None,
            pad_base: None,
            speed: (0.998, 1.002),
            pitch: None,
            volume_amp: 0.2,
        },
        "hard" => StrengthParams {
            rotation_amp: 1.8,
            zoom: (1.06, 1.11),
            saturation: (0.90, 1.10),
            contrast_amp: 0.08,
            brightness_static_amp: 0.05,
            hue_amp: 8.0,
            flicker: Some((0.007, 0.014)),
            noise: Some((3, 5)),
            pad_base: Some((2, 4)),
            speed: (0.94, 1.06),
            pitch: Some((0.98, 1.02)),
            volume_amp: 1.5,
        },
        "extreme" => StrengthParams {
            rotation_amp: 3.5,
            zoom: (1.12, 1.20),
            saturation: (0.82, 1.18),
            contrast_amp: 0.14,
            brightness_static_amp: 0.09,
            hue_amp: 16.0,
            flicker: Some((0.014, 0.026)),
            noise: Some((5, 10)),
            pad_base: Some((3, 8)),
            speed: (0.90, 1.10),
            pitch: Some((0.95, 1.05)),
            volume_amp: 3.0,
        },
        // "medium" and default
        _ => StrengthParams {
            rotation_amp: 0.6,
            zoom: (1.02, 1.03),
            saturation: (0.97, 1.03),
            contrast_amp: 0.02,
            brightness_static_amp: 0.015,
            hue_amp: 2.0,
            flicker: Some((0.003, 0.006)),
            noise: Some((2, 2)),
            pad_base: None,
            speed: (0.99, 1.01),
            pitch: None,
            volume_amp: 0.5,
        },
    }
}
