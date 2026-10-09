//! Factory voice presets.

use crate::params::{
    CHAOS_CUTOFF, CHAOS_PITCH, CHAOS_X, ENGINE_BOOM, ENGINE_FM, ENGINE_FOLD, ENGINE_PD,
    ENGINE_REESE, ENGINE_SYNC, ENGINE_TABLE, ENGINE_WTFM, FILTER_BP, FILTER_HP, FILTER_LP12,
    FILTER_LP24, Param, SAT_FOLD, SAT_HARD, SAT_POST, SAT_TUBE, VoicePatch, default_patch,
    map::tune_knob,
};

pub struct Preset {
    pub name: &'static str,
    pub values: &'static [(Param, f32)],
}

impl Preset {
    /// Full patch: defaults overlaid with this preset's values.
    pub fn patch(&self) -> VoicePatch {
        let mut p = default_patch();
        for &(param, v) in self.values {
            p[param.index()] = param.sanitize(v);
        }
        p
    }
}

const SYNC: f32 = ENGINE_SYNC as f32;
const REESE: f32 = ENGINE_REESE as f32;
const FM: f32 = ENGINE_FM as f32;
const PD: f32 = ENGINE_PD as f32;
const TABLE: f32 = ENGINE_TABLE as f32;
const WTFM: f32 = ENGINE_WTFM as f32;
const BOOM: f32 = ENGINE_BOOM as f32;
const FOLD: f32 = ENGINE_FOLD as f32;
const LP12: f32 = FILTER_LP12 as f32;
const LP24: f32 = FILTER_LP24 as f32;
const BP: f32 = FILTER_BP as f32;
const HP: f32 = FILTER_HP as f32;
const TUBE: f32 = SAT_TUBE as f32;
const HARD: f32 = SAT_HARD as f32;
const SFOLD: f32 = SAT_FOLD as f32;
const POST: f32 = SAT_POST as f32;

/// Knob positions for a few useful values.
mod k {
    /// PITCH fader: 16 Hz × 2^(6·v).
    pub const HZ_30: f32 = 0.151;
    pub const HZ_36: f32 = 0.195;
    pub const HZ_41: f32 = 0.227;
    pub const HZ_45: f32 = 0.25;
    pub const HZ_49: f32 = 0.27;
    pub const HZ_55: f32 = 0.3;
    pub const HZ_65: f32 = 0.338;
    pub const HZ_73: f32 = 0.365;
    pub const HZ_87: f32 = 0.408;
    pub const HZ_110: f32 = 0.462;
    pub const HZ_147: f32 = 0.533;
    /// FM / WT-FM RATIO: 0.25 × 32^v.
    pub const RATIO_0_5: f32 = 0.2;
    pub const RATIO_1: f32 = 0.4;
    pub const RATIO_1_41: f32 = 0.5;
    pub const RATIO_2_76: f32 = 0.693;
    pub const RATIO_3_5: f32 = 0.761;
    /// Oscillator DECAY: x1 at 0.8, ×2 per 0.2.
    pub const X1: f32 = 0.8;
    pub const X0_5: f32 = 0.6;
    pub const X0_25: f32 = 0.4;
    pub const X1_5: f32 = 0.917;
}

use Param::*;

pub const PRESETS: &[Preset] = &[
    Preset {
        name: "Init",
        values: &[],
    },
    // --- BOOM ----------------------------------------------------------
    Preset {
        name: "808 Long",
        values: &[
            (Engine, BOOM),
            (Level, 0.56),
            (Pitch, k::HZ_45),
            (Decay, 0.78),
            (MacroX, 0.18),
            (MacroY, 0.3),
            (MacroZ, 0.0),
            (Click, 0.2),
            (SweepDepth, 0.68),
            (SweepSpeed, 0.62),
            (Hold, 0.15),
            (LowGain, 0.58),
            (LowFreq, 0.3),
            (ReverbMix, 0.08),
        ],
    },
    Preset {
        name: "808 Overdrive",
        values: &[
            (Engine, BOOM),
            (Level, 1.0),
            (Pitch, k::HZ_41),
            (Decay, 0.74),
            (MacroX, 0.7),
            (MacroY, 0.35),
            (MacroZ, 0.35),
            (Click, 0.25),
            (SweepDepth, 0.7),
            (SweepSpeed, 0.6),
            (Drive, 0.35),
            (SatType, TUBE),
            (SatPos, POST),
            (Cutoff, 0.82),
            (MidFreq, 0.42),
            (MidGain, 0.62),
            (MidQ, 0.3),
            (ReverbMix, 0.1),
        ],
    },
    Preset {
        name: "Sub Kick",
        values: &[
            (Engine, BOOM),
            (Pitch, k::HZ_41),
            (Decay, 0.5),
            (MacroX, 0.25),
            (MacroY, 0.6),
            (MacroZ, 0.0),
            (Click, 0.45),
            (Noise, 0.08),
            (SweepDepth, 0.78),
            (SweepSpeed, 0.68),
            (LowCut, 0.15),
            (ReverbMix, 0.05),
        ],
    },
    Preset {
        name: "Rumble Kick",
        values: &[
            (Engine, BOOM),
            (Pitch, k::HZ_45),
            (Decay, 0.62),
            (MacroX, 0.45),
            (MacroY, 0.55),
            (Click, 0.35),
            (SweepDepth, 0.76),
            (SweepSpeed, 0.66),
            (Drive, 0.4),
            (SatPos, POST),
            (Cutoff, 0.62),
            (FilterType, LP24),
            (ReverbMix, 0.6),
            (ReverbDecay, 0.55),
            (ReverbTone, 0.0),
            (ReverbPredelay, 0.0),
        ],
    },
    Preset {
        name: "Hard Kick",
        values: &[
            (Engine, BOOM),
            (Pitch, k::HZ_49),
            (Decay, 0.6),
            (MacroX, 0.9),
            (MacroY, 0.85),
            (MacroZ, 0.2),
            (Click, 0.5),
            (SweepDepth, 0.84),
            (SweepSpeed, 0.6),
            (Drive, 0.55),
            (SatType, HARD),
            (SatPos, POST),
            (MidFreq, 0.36),
            (MidGain, 0.7),
            (MidQ, 0.4),
            (HighGain, 0.6),
            (Level, 0.68),
            (ReverbMix, 0.12),
        ],
    },
    Preset {
        name: "Deep Tom",
        values: &[
            (Engine, BOOM),
            (Pitch, k::HZ_87),
            (Decay, 0.55),
            (MacroX, 0.1),
            (MacroY, 0.2),
            (Click, 0.3),
            (Noise, 0.12),
            (SweepDepth, 0.64),
            (SweepSpeed, 0.5),
            (ReverbMix, 0.3),
            (ReverbDecay, 0.4),
        ],
    },
    Preset {
        name: "Dub Boom",
        values: &[
            (Engine, BOOM),
            (Pitch, k::HZ_55),
            (Decay, 0.85),
            (MacroX, 0.3),
            (MacroY, 0.15),
            (Attack, 0.2),
            (Click, 0.0),
            (SweepDepth, 0.6),
            (SweepSpeed, 0.4),
            (Level2, 0.55),
            (Tune2, tune_knob(12.0)),
            (Decay2, k::X0_5),
            (ChorusMix, 0.55),
            (ChorusRate, 0.25),
            (ChorusXover, 0.45),
            (ReverbMix, 0.5),
            (ReverbDecay, 0.7),
            (ReverbTone, 0.25),
            (ReverbPredelay, 0.5),
        ],
    },
    // --- SYNC ----------------------------------------------------------
    Preset {
        name: "Sync Zap",
        values: &[
            (Engine, SYNC),
            (Pitch, k::HZ_55),
            (Decay, 0.55),
            (MacroX, 0.66),
            (MacroY, 0.25),
            (MacroZ, 0.7),
            (Click, 0.15),
            (SweepDepth, 0.62),
            (SweepSpeed, 0.55),
            (FilterType, LP24),
            (Cutoff, 0.6),
            (Reso, 0.3),
            (FilterEnv, 0.75),
            (ReverbMix, 0.15),
        ],
    },
    Preset {
        name: "Sync Stab",
        values: &[
            (Engine, SYNC),
            (Pitch, k::HZ_65),
            (Decay, 0.45),
            (MacroX, 0.9),
            (MacroY, 0.55),
            (MacroZ, 0.35),
            (Level2, 0.7),
            (Tune2, tune_knob(12.0)),
            (Decay2, k::X0_5),
            (FilterType, LP12),
            (Cutoff, 0.55),
            (Reso, 0.45),
            (FilterEnv, 0.7),
            (Drive, 0.25),
            (ChorusMix, 0.35),
            (ReverbMix, 0.2),
        ],
    },
    Preset {
        name: "Laser Drop",
        values: &[
            (Engine, SYNC),
            (Pitch, k::HZ_45),
            (Decay, 0.72),
            (MacroX, 0.33),
            (MacroY, 0.0),
            (MacroZ, 0.0),
            (Click, 0.0),
            (SweepDepth, 0.95),
            (SweepSpeed, 0.3),
            (Cutoff, 0.85),
            (Drive, 0.2),
            (ReverbMix, 0.35),
            (ReverbDecay, 0.6),
        ],
    },
    // --- REESE ---------------------------------------------------------
    Preset {
        name: "Hollow Reese",
        values: &[
            (Engine, REESE),
            (Level, 0.97),
            (Pitch, k::HZ_55),
            (Decay, 0.72),
            (MacroX, 0.3),
            (MacroY, 0.75),
            (MacroZ, 0.15),
            (Click, 0.1),
            (SweepDepth, 0.56),
            (FilterType, LP24),
            (Cutoff, 0.52),
            (Reso, 0.3),
            (FilterEnv, 0.66),
            (ChorusMix, 0.65),
            (ChorusRate, 0.35),
            (ChorusDepth, 0.6),
            (ChorusXover, 0.5),
            (LowGain, 0.6),
            (LowFreq, 0.25),
            (ReverbMix, 0.15),
        ],
    },
    Preset {
        name: "Neuro Hit",
        values: &[
            (Engine, REESE),
            (Level, 0.58),
            (Pitch, k::HZ_49),
            (Decay, 0.6),
            (MacroX, 0.55),
            (MacroY, 0.9),
            (MacroZ, 0.5),
            (Click, 0.2),
            (FilterType, BP),
            (Cutoff, 0.42),
            (Reso, 0.6),
            (FilterEnv, 0.7),
            (ChaosRate, 0.72),
            (ChaosDepth, 0.6),
            (ChaosDest, CHAOS_CUTOFF as f32),
            (Drive, 0.45),
            (SatType, SFOLD),
            (SatPos, POST),
            (Level2, 0.6),
            (Tune2, tune_knob(-12.0)),
            (Decay2, k::X1),
            (ChorusMix, 0.3),
            (ReverbMix, 0.12),
        ],
    },
    // --- FM ------------------------------------------------------------
    Preset {
        name: "FM Pluck Bass",
        values: &[
            (Engine, FM),
            (Pitch, k::HZ_55),
            (Decay, 0.55),
            (MacroX, k::RATIO_1),
            (MacroY, 0.55),
            (MacroZ, 0.35),
            (Click, 0.1),
            (SweepDepth, 0.55),
            (ReverbMix, 0.1),
        ],
    },
    Preset {
        name: "DX Kick",
        values: &[
            (Engine, FM),
            (Pitch, k::HZ_45),
            (Decay, 0.52),
            (MacroX, k::RATIO_0_5),
            (MacroY, 0.6),
            (MacroZ, 0.12),
            (Click, 0.3),
            (SweepDepth, 0.76),
            (SweepSpeed, 0.65),
            (ReverbMix, 0.08),
        ],
    },
    Preset {
        name: "Metal Tom",
        values: &[
            (Engine, FM),
            (Pitch, k::HZ_110),
            (Decay, 0.55),
            (MacroX, k::RATIO_1_41),
            (MacroY, 0.5),
            (MacroZ, 0.4),
            (Click, 0.2),
            (Noise, 0.15),
            (SweepDepth, 0.62),
            (SweepSpeed, 0.5),
            (ReverbMix, 0.3),
            (ReverbDecay, 0.45),
        ],
    },
    Preset {
        name: "Gamelan Bass",
        values: &[
            (Engine, FM),
            (Pitch, k::HZ_73),
            (Decay, 0.7),
            (MacroX, k::RATIO_3_5),
            (MacroY, 0.4),
            (MacroZ, 0.5),
            (Click, 0.15),
            (SweepDepth, 0.53),
            // Free tuning: 2.76:1 and 3.36:1, inharmonic like a struck bar.
            (Level2, 0.6),
            (Tune2, tune_knob(17.57)),
            (Decay2, k::X0_5),
            (Level3, 0.45),
            (Tune3, tune_knob(21.0)),
            (Decay3, k::X0_25),
            (ReverbMix, 0.35),
            (ReverbDecay, 0.55),
        ],
    },
    // --- PD ------------------------------------------------------------
    Preset {
        name: "CZ Growl",
        values: &[
            (Engine, PD),
            (Pitch, k::HZ_49),
            (Decay, 0.62),
            (MacroX, 0.75),
            (MacroY, 0.33),
            (MacroZ, 0.6),
            (Click, 0.15),
            (SweepDepth, 0.58),
            (Drive, 0.25),
            (ChorusMix, 0.25),
            (ReverbMix, 0.1),
        ],
    },
    Preset {
        name: "Reso Thump",
        values: &[
            (Engine, PD),
            (Level, 0.92),
            (Pitch, k::HZ_55),
            (Decay, 0.5),
            (MacroX, 0.65),
            (MacroY, 0.67),
            (MacroZ, 0.9),
            (Click, 0.3),
            (SweepDepth, 0.66),
            (SweepSpeed, 0.6),
            (ReverbMix, 0.12),
        ],
    },
    // --- TABLE ---------------------------------------------------------
    Preset {
        name: "Organ Thud",
        values: &[
            (Engine, TABLE),
            (Pitch, k::HZ_65),
            (Decay, 0.55),
            (MacroX, 0.43),
            (MacroY, 0.8),
            (MacroZ, 0.5),
            (Click, 0.2),
            (SweepDepth, 0.6),
            (Level2, 0.65),
            (Tune2, tune_knob(-12.0)),
            (Decay2, k::X1_5),
            (ReverbMix, 0.15),
        ],
    },
    Preset {
        name: "Formant Bass",
        values: &[
            (Engine, TABLE),
            (Pitch, k::HZ_49),
            (Decay, 0.6),
            (MacroX, 0.57),
            (MacroY, 0.65),
            (MacroZ, 0.3),
            (Click, 0.1),
            (FilterType, LP12),
            (Cutoff, 0.55),
            (Reso, 0.5),
            (FilterEnv, 0.68),
            (ChorusMix, 0.3),
            (ReverbMix, 0.12),
        ],
    },
    Preset {
        name: "Just Fifth Stab",
        values: &[
            (Engine, TABLE),
            (Level, 1.0),
            (Pitch, k::HZ_55),
            (Decay, 0.48),
            (MacroX, 0.86),
            (MacroY, 0.5),
            (MacroZ, 0.5),
            // A pure 3:2 fifth (+7.02 st) and a 5:2 major tenth (+15.86 st),
            // neither on the equal-tempered grid.
            (Level2, 0.75),
            (Tune2, tune_knob(7.02)),
            (Decay2, k::X1),
            (Level3, 0.6),
            (Tune3, tune_knob(15.86)),
            (Decay3, k::X0_5),
            (FilterType, LP24),
            (Cutoff, 0.5),
            (Reso, 0.35),
            (FilterEnv, 0.78),
            (ChorusMix, 0.45),
            (ReverbMix, 0.25),
        ],
    },
    // --- WT-FM ---------------------------------------------------------
    Preset {
        name: "Bark Bass",
        values: &[
            (Engine, WTFM),
            (Level, 0.94),
            (Pitch, k::HZ_55),
            (Decay, 0.55),
            (MacroX, 0.86),
            (MacroY, 0.6),
            (MacroZ, k::RATIO_1),
            (Click, 0.15),
            (SweepDepth, 0.57),
            (FilterType, LP24),
            (Cutoff, 0.75),
            (ReverbMix, 0.1),
        ],
    },
    Preset {
        name: "Wobble Hit",
        values: &[
            (Engine, WTFM),
            (Pitch, k::HZ_45),
            (Decay, 0.75),
            (MacroX, 0.3),
            (MacroY, 0.5),
            (MacroZ, k::RATIO_0_5),
            (Hold, 0.35),
            (FilterType, LP24),
            (Cutoff, 0.5),
            (Reso, 0.55),
            (ChaosRate, 0.78),
            (ChaosDepth, 0.75),
            (ChaosDest, CHAOS_CUTOFF as f32),
            (Drive, 0.3),
            (ReverbMix, 0.15),
        ],
    },
    Preset {
        name: "Clang Drop",
        values: &[
            (Engine, WTFM),
            (Pitch, k::HZ_36),
            (Decay, 0.66),
            (MacroX, 0.71),
            (MacroY, 0.75),
            (MacroZ, k::RATIO_2_76),
            (Click, 0.35),
            (SweepDepth, 0.8),
            (SweepSpeed, 0.5),
            (Level2, 0.5),
            (Tune2, tune_knob(-12.0)),
            (Decay2, k::X1_5),
            (ReverbMix, 0.25),
        ],
    },
    // --- FOLD ----------------------------------------------------------
    Preset {
        name: "West Coast Tom",
        values: &[
            (Engine, FOLD),
            (Pitch, k::HZ_87),
            (Decay, 0.5),
            (MacroX, 0.6),
            (MacroY, 0.62),
            (MacroZ, 0.9),
            (Click, 0.2),
            (SweepDepth, 0.64),
            (SweepSpeed, 0.5),
            (ReverbMix, 0.25),
        ],
    },
    Preset {
        name: "Folded Sub",
        values: &[
            (Engine, FOLD),
            (Pitch, k::HZ_55),
            (Decay, 0.66),
            (MacroX, 0.45),
            (MacroY, 0.5),
            (MacroZ, 0.75),
            (Click, 0.15),
            (SweepDepth, 0.6),
            (Level2, 0.7),
            (Tune2, tune_knob(-12.0)),
            (Decay2, k::X1),
            (LowGain, 0.6),
            (LowFreq, 0.3),
            (ReverbMix, 0.1),
        ],
    },
    // --- multi-oscillator studies ---------------------------------------
    Preset {
        name: "Beating Tom",
        values: &[
            (Engine, SYNC),
            (Pitch, k::HZ_73),
            (Decay, 0.6),
            (MacroX, 0.15),
            (MacroY, 0.0),
            (MacroZ, 0.0),
            (Click, 0.3),
            (SweepDepth, 0.63),
            (SweepSpeed, 0.52),
            // Two oscillators a quarter-tone apart beat at ~2 Hz.
            (Level2, 0.9),
            (Tune2, tune_knob(0.5)),
            (Decay2, k::X1),
            (Level3, 0.6),
            (Tune3, tune_knob(-12.0)),
            (Decay3, k::X1_5),
            (ChorusMix, 0.4),
            (ReverbMix, 0.25),
        ],
    },
    Preset {
        name: "Chaos Bloom",
        values: &[
            (Engine, PD),
            (Level, 0.84),
            (Pitch, k::HZ_30),
            (Decay, 0.8),
            (MacroX, 0.55),
            (MacroY, 0.85),
            (MacroZ, 0.4),
            (Attack, 0.3),
            (Hold, 0.3),
            (SweepDepth, 0.5),
            (ChaosRate, 0.55),
            (ChaosDepth, 0.55),
            (ChaosDest, CHAOS_X as f32),
            (Jitter, 0.25),
            (Level2, 0.55),
            (Tune2, tune_knob(19.02)),
            (Decay2, k::X0_5),
            (ChorusMix, 0.6),
            (ChorusXover, 0.35),
            (ReverbMix, 0.4),
            (ReverbDecay, 0.65),
        ],
    },
    Preset {
        name: "Drift Sub",
        values: &[
            (Engine, TABLE),
            (Pitch, k::HZ_41),
            (Decay, 0.82),
            (MacroX, 0.15),
            (MacroY, 0.62),
            (MacroZ, 0.5),
            (Hold, 0.25),
            (Click, 0.25),
            (SweepDepth, 0.66),
            (SweepSpeed, 0.6),
            (ChaosRate, 0.3),
            (ChaosDepth, 0.4),
            (ChaosDest, CHAOS_PITCH as f32),
            (LowCut, 0.1),
            (ReverbMix, 0.1),
        ],
    },
    Preset {
        name: "Thin Snap",
        values: &[
            (Engine, FOLD),
            (Level, 0.92),
            (Pitch, k::HZ_147),
            (Decay, 0.35),
            (MacroX, 0.75),
            (MacroY, 0.4),
            (MacroZ, 1.0),
            (Click, 0.55),
            (Noise, 0.45),
            (SweepDepth, 0.72),
            (SweepSpeed, 0.7),
            (FilterType, HP),
            (Cutoff, 0.22),
            (ReverbMix, 0.35),
            (ReverbDecay, 0.4),
        ],
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::ENGINE_COUNT;
    use crate::voice::Voice;

    #[test]
    fn preset_names_unique() {
        for (i, a) in PRESETS.iter().enumerate() {
            for b in &PRESETS[i + 1..] {
                assert_ne!(a.name, b.name);
            }
        }
    }

    #[test]
    fn presets_are_in_range() {
        for p in PRESETS {
            for &(param, v) in p.values {
                assert_eq!(param.sanitize(v), v, "{}: {:?}", p.name, param);
            }
        }
    }

    #[test]
    fn every_engine_has_presets() {
        for e in 0..ENGINE_COUNT {
            let n = PRESETS
                .iter()
                .filter(|p| p.patch()[Param::Engine.index()] as usize == e)
                .count();
            assert!(n >= 2, "engine {e}: {n} presets");
        }
    }

    #[test]
    fn presets_sound_and_stay_sane() {
        let fs = 96_000.0;
        for p in PRESETS {
            let mut v = Voice::new(fs, 1);
            let patch = p.patch();
            for param in crate::params::ALL_PARAMS {
                v.set_param(param, patch[param.index()]);
            }
            v.snap_params();
            v.trigger(1.0);
            let out: Vec<f32> = (0..(fs * 0.5) as usize).map(|_| v.tick()).collect();
            let peak = out.iter().fold(0.0f32, |a, &b| a.max(b.abs()));
            assert!(out.iter().all(|x| x.is_finite()), "{}", p.name);
            assert!(peak > 0.1 && peak < 2.0, "{}: peak {peak}", p.name);
        }
    }
}
