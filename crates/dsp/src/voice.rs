//! One Subkussion channel, up to the stereo stage.
//!
//! ```text
//!  PITCH ─┬─ sweep EG ─┬─ chaos ─┬─ jitter ─► pitch ─┬─ +TUNE2 ─┬─ +TUNE3
//!         ▼            ▼         ▼                   ▼          ▼          ▼
//!                                              ┌── OSC 1 ── OSC 2 ── OSC 3 ──┐  (the selected engine, ×3)
//!  TRIGGER ─► AHD envelopes (one decay per OSC) ┴─── × LEVEL, × ENV each ───┴─► Σ ─┐
//!         └─► CLICK + NOISE ────────────────────────────────────────────────────────┤
//!                                    ┌──────────────────────────────────────────────┘
//!                                    ▼
//!                       [SAT pre] ─► LADDER ─► EQ ─► [SAT post] ─► OUTPUT ─► (chorus, pan, plate)
//! ```
//!
//! Controls are evaluated once per [`CONTROL_BLOCK`] samples (parameter
//! smoothing, modulation, engine and filter coefficients). The envelopes, the
//! transients and the audio path run every sample.

use crate::engines::{Ctx, Engines, OSCS, OscCtx};
use crate::exciter::Transient;
use crate::fx::{Eq, SatCoeffs};
use crate::modulation::ChaosLfo;
use crate::params::{
    CHAOS_CUTOFF, CHAOS_PITCH, CHAOS_X, CHAOS_Y, CHAOS_Z, PARAM_COUNT, Param, SAT_POST, VoicePatch,
    default_patch, map,
};
use crate::util::{Ladder, LadderCoeffs, Noise, one_pole_coeff, t60_gain};

/// Samples per control update.
pub const CONTROL_BLOCK: usize = 16;
/// Parameter de-zippering.
const SMOOTH_TAU_S: f32 = 0.004;
/// Chaos modulation range at full depth.
const CHAOS_PITCH_OCT: f32 = 1.0;
const CHAOS_MACRO: f32 = 0.5;
const CHAOS_CUTOFF_OCT: f32 = 4.0;
/// Jitter range at full setting.
const JITTER_PITCH_OCT: f32 = 0.5;
const JITTER_MACRO: f32 = 0.35;

/// Which parameters are de-zippered (the continuous ones).
const SMOOTHED: [bool; PARAM_COUNT] = {
    let mut s = [true; PARAM_COUNT];
    s[Param::Engine as usize] = false;
    s[Param::ChaosDest as usize] = false;
    s[Param::FilterType as usize] = false;
    s[Param::SatType as usize] = false;
    s[Param::SatPos as usize] = false;
    s
};

/// The three oscillators' (tune, level, decay ratio) parameters. Oscillator
/// 1 is the reference: no offset, DECAY as is.
const OSC_PARAMS: [(Option<Param>, Param, Option<Param>); OSCS] = [
    (None, Param::Level1, None),
    (Some(Param::Tune2), Param::Level2, Some(Param::Decay2)),
    (Some(Param::Tune3), Param::Level3, Some(Param::Decay3)),
];

#[derive(Clone, Debug)]
pub struct Voice {
    fs: f32,
    target: VoicePatch,
    smooth: VoicePatch,
    k_smooth: f32,

    noise: Noise,
    transient: Transient,
    engines: Engines,
    engine: u32,
    ctx: Ctx,
    oscs: [OscCtx; OSCS],
    block_pos: usize,

    /// Amplitude envelopes, 0..velocity, one per oscillator. Attack and hold
    /// are shared; each decays at its own rate.
    env: [f32; OSCS],
    decay_mul: [f32; OSCS],
    attack_left: u32,
    attack_inc: f32,
    hold_left: u32,
    velocity: f32,
    /// Level × mix normalization per oscillator.
    osc_gain: [f32; OSCS],
    /// Sweep envelope, 1 → 0.
    sweep: f32,
    sweep_mul: f32,

    chaos: ChaosLfo,
    /// Per-hit offsets: pitch (octaves), X, Y, Z.
    jitter: [f32; 4],

    ladder: Ladder,
    ladder_coeffs: LadderCoeffs,
    sat: SatCoeffs,
    sat_post: bool,
    eq: Eq,
    level: f32,
}

impl Voice {
    /// `fs` is the rate `tick` is called at (the oversampled rate).
    pub fn new(fs: f32, seed: u32) -> Self {
        let patch = default_patch();
        let mut v = Self {
            fs,
            target: patch,
            smooth: patch,
            k_smooth: one_pole_coeff(SMOOTH_TAU_S, fs / CONTROL_BLOCK as f32),
            noise: Noise::new(seed),
            transient: Transient::new(fs),
            engines: Engines::new(),
            engine: patch[Param::Engine.index()] as u32,
            ctx: Ctx::new(fs),
            oscs: [OscCtx::default(); OSCS],
            block_pos: 0,
            env: [0.0; OSCS],
            decay_mul: [0.0; OSCS],
            attack_left: 0,
            attack_inc: 0.0,
            hold_left: 0,
            velocity: 1.0,
            osc_gain: [0.0; OSCS],
            sweep: 0.0,
            sweep_mul: 0.0,
            chaos: ChaosLfo::new(seed),
            jitter: [0.0; 4],
            ladder: Ladder::default(),
            ladder_coeffs: LadderCoeffs::new(20_000.0, 0.0, 1, fs),
            sat: SatCoeffs::default(),
            sat_post: false,
            eq: Eq::default(),
            level: 0.0,
        };
        v.control();
        v
    }

    pub fn set_param(&mut self, param: Param, value: f32) {
        let v = param.sanitize(value);
        self.target[param.index()] = v;
        if !SMOOTHED[param.index()] {
            self.smooth[param.index()] = v;
        }
    }

    pub fn param(&self, param: Param) -> f32 {
        self.target[param.index()]
    }

    /// The de-zippered value the voice is currently using.
    pub fn smoothed(&self, param: Param) -> f32 {
        self.smooth[param.index()]
    }

    /// Jump all smoothed parameters to their targets.
    pub fn snap_params(&mut self) {
        self.smooth = self.target;
        self.control();
    }

    /// Fire the trigger. `velocity` is the raw hit strength `0..=1`; SENSE
    /// scales it like a trigger input's preamp.
    pub fn trigger(&mut self, velocity: f32) {
        let gain = map::sense_gain(self.target[Param::Sense.index()]);
        self.velocity = (velocity.clamp(0.0, 1.0) * gain).clamp(0.02, 1.0);
        let p = self.smooth;
        let attack = ((map::attack_seconds(p[Param::Attack.index()]) * self.fs) as u32).max(1);
        self.attack_left = attack;
        self.attack_inc = self.velocity / attack as f32;
        self.hold_left = (map::hold_seconds(p[Param::Hold.index()]) * self.fs) as u32;
        self.sweep = 1.0;
        let amount = p[Param::Jitter.index()];
        self.jitter = if amount > 0.0 {
            [
                JITTER_PITCH_OCT * amount * amount * self.noise.sample(),
                JITTER_MACRO * amount * self.noise.sample(),
                JITTER_MACRO * amount * self.noise.sample(),
                JITTER_MACRO * amount * self.noise.sample(),
            ]
        } else {
            [0.0; 4]
        };
        // Evaluate the controls now so the engine sees this hit's pitch,
        // jitter and velocity from its first sample.
        self.control();
        self.block_pos = 1;
        self.transient.trigger(
            self.velocity,
            self.smooth[Param::Click.index()],
            self.smooth[Param::Noise.index()],
        );
        self.engines
            .trigger(self.engine, &self.ctx, &self.oscs, &mut self.noise);
    }

    /// Chaos LED: lit on one lobe of the attractor while it modulates.
    pub fn lfo_led(&self) -> bool {
        self.smooth[Param::ChaosDepth.index()] > 0.0 && self.chaos.value() > 0.0
    }

    /// Current amplitude envelope of oscillator 1 (for metering).
    pub fn envelope(&self) -> f32 {
        self.env[0]
    }

    /// The selected engine.
    pub fn engine(&self) -> u32 {
        self.engine
    }

    /// Control-rate update: smoothing, modulation, coefficients.
    fn control(&mut self) {
        let fs = self.fs;
        let block_s = CONTROL_BLOCK as f32 / fs;
        for ((cur, &target), &smoothed) in self.smooth.iter_mut().zip(&self.target).zip(&SMOOTHED) {
            if smoothed {
                *cur += (target - *cur) * self.k_smooth;
            }
        }
        let p = self.smooth;
        let param = |q: Param| p[q.index()];

        let engine = param(Param::Engine) as u32;
        if engine != self.engine {
            self.engines.reset(engine);
            self.engine = engine;
        }

        // Modulation.
        self.chaos.advance(param(Param::ChaosRate), block_s);
        let depth = param(Param::ChaosDepth);
        let chaos = self.chaos.value() * depth * depth;
        let dest = param(Param::ChaosDest) as u32;
        let chaos_to = |d: u32| if dest == d { chaos } else { 0.0 };

        let macro_value =
            |q: Param, d: u32, j: f32| (param(q) + CHAOS_MACRO * chaos_to(d) + j).clamp(0.0, 1.0);
        self.ctx = Ctx {
            fs,
            x: macro_value(Param::MacroX, CHAOS_X, self.jitter[1]),
            y: macro_value(Param::MacroY, CHAOS_Y, self.jitter[2]),
            z: macro_value(Param::MacroZ, CHAOS_Z, self.jitter[3]),
            velocity: self.velocity,
        };

        // The oscillators: pitch, envelope and level.
        let decay_s = map::decay_seconds(param(Param::Decay));
        let oct = map::pitch_octaves(param(Param::Pitch), param(Param::Fine))
            + map::sweep_octaves(param(Param::SweepDepth)) * self.sweep
            + CHAOS_PITCH_OCT * chaos_to(CHAOS_PITCH)
            + self.jitter[0];
        let mut gains = [0.0f32; OSCS];
        for (i, &(tune, level, ratio)) in OSC_PARAMS.iter().enumerate() {
            let semis = tune.map_or(0.0, |t| map::tune_semitones(param(t)));
            let decay = decay_s * ratio.map_or(1.0, |r| map::osc_decay_ratio(param(r)));
            self.oscs[i] = OscCtx {
                hz: (oct + semis / 12.0).exp2().clamp(2.0, 0.45 * fs),
                // Count the attack as the peak, so engines that follow the
                // envelope start at the top.
                env: if self.attack_left > 0 {
                    1.0
                } else {
                    (self.env[i] / self.velocity).clamp(0.0, 1.0)
                },
                decay_s: decay,
            };
            self.decay_mul[i] = t60_gain(decay, fs);
            gains[i] = map::osc_gain(param(level));
        }
        // Stacking oscillators shouldn't simply pile up level: scale the sum
        // back towards the loudness of one.
        let total: f32 = gains.iter().sum();
        let norm = 1.0 / total.max(1.0).powf(0.75);
        self.osc_gain = gains.map(|g| g * norm);
        self.engines.control(self.engine, &self.ctx, &self.oscs);

        self.sweep_mul = (-1.0 / (map::sweep_seconds(param(Param::SweepSpeed)) * fs)).exp();

        // Filter: the envelope moves the cutoff, the chaos can too.
        let cut_oct = map::cutoff_hz(param(Param::Cutoff)).log2()
            + map::filter_env_octaves(param(Param::FilterEnv)) * self.env[0]
            + CHAOS_CUTOFF_OCT * chaos_to(CHAOS_CUTOFF);
        self.ladder_coeffs = LadderCoeffs::new(
            cut_oct.exp2(),
            map::reso_k(param(Param::Reso)),
            param(Param::FilterType) as u32,
            fs,
        );

        self.sat = SatCoeffs::new(
            param(Param::Drive),
            param(Param::SatType) as u32,
            param(Param::SatMix),
        );
        self.sat_post = param(Param::SatPos) as u32 == SAT_POST;
        self.eq.update(
            [
                param(Param::LowCut),
                param(Param::LowFreq),
                param(Param::LowGain),
                param(Param::MidFreq),
                param(Param::MidGain),
                param(Param::MidQ),
                param(Param::HighFreq),
                param(Param::HighGain),
            ],
            fs,
        );
        self.level = map::level_gain(param(Param::Level));
    }

    #[inline]
    pub fn tick(&mut self) -> f32 {
        if self.block_pos == 0 {
            self.control();
        }
        self.block_pos += 1;
        if self.block_pos == CONTROL_BLOCK {
            self.block_pos = 0;
        }

        // Envelopes: a linear attack towards the velocity, a hold, then each
        // oscillator decays at its own rate.
        if self.attack_left > 0 {
            self.attack_left -= 1;
            for e in self.env.iter_mut() {
                if *e < self.velocity {
                    *e = (*e + self.attack_inc).min(self.velocity);
                }
            }
        } else if self.hold_left > 0 {
            self.hold_left -= 1;
        } else {
            for (e, &m) in self.env.iter_mut().zip(&self.decay_mul) {
                *e *= m;
                if *e < 1e-6 {
                    *e = 0.0;
                }
            }
        }
        self.sweep *= self.sweep_mul;
        if self.sweep < 1e-6 {
            self.sweep = 0.0;
        }

        let gains: [f32; OSCS] = core::array::from_fn(|i| self.osc_gain[i] * self.env[i]);
        let mut x = self.engines.tick(self.engine, &gains) + self.transient.tick(&mut self.noise);
        if !x.is_finite() {
            // Never let a numerical accident poison the channel.
            self.engines.reset(self.engine);
            self.ladder.reset();
            x = 0.0;
        }

        if self.sat.active() && !self.sat_post {
            x = self.sat.process(x);
        }
        x = self.ladder.process(x, &self.ladder_coeffs);
        x = self.eq.process(x);
        if self.sat.active() && self.sat_post {
            x = self.sat.process(x);
        }
        if !x.is_finite() {
            self.ladder.reset();
            self.eq = Eq::default();
            x = 0.0;
        }
        x * self.level
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::{ENGINE_BOOM, ENGINE_COUNT, ENGINE_FM, ENGINE_SYNC, FILTER_HP};

    fn render(v: &mut Voice, n: usize) -> Vec<f32> {
        (0..n).map(|_| v.tick()).collect()
    }

    fn peak(x: &[f32]) -> f32 {
        x.iter().fold(0.0f32, |a, &b| a.max(b.abs()))
    }

    fn rms(x: &[f32]) -> f32 {
        (x.iter().map(|v| v * v).sum::<f32>() / x.len() as f32).sqrt()
    }

    fn zero_crossings(x: &[f32]) -> usize {
        x.windows(2).filter(|w| w[0] <= 0.0 && w[1] > 0.0).count()
    }

    /// A plain sine at PITCH: SYNC with no sync, no sweep, no click.
    fn sine_voice(fs: f32) -> Voice {
        let mut v = Voice::new(fs, 1);
        v.set_param(Param::Engine, ENGINE_SYNC as f32);
        v.set_param(Param::MacroX, 0.0);
        v.set_param(Param::MacroY, 0.0);
        v.set_param(Param::MacroZ, 0.0);
        v.set_param(Param::SweepDepth, 0.5);
        v.set_param(Param::Click, 0.0);
        v.set_param(Param::Decay, 1.0);
        v
    }

    #[test]
    fn each_engine_sounds_and_decays() {
        let fs = 96_000.0;
        for engine in 0..ENGINE_COUNT {
            let mut v = Voice::new(fs, 1);
            v.set_param(Param::Engine, engine as f32);
            v.set_param(Param::Decay, 0.3);
            v.snap_params();
            v.trigger(1.0);
            let early = peak(&render(&mut v, 9_600));
            let _ = render(&mut v, fs as usize * 2);
            let late = peak(&render(&mut v, 9_600));
            assert!(early > 0.05, "engine {engine}: quiet ({early})");
            assert!(early < 2.0, "engine {engine}: loud ({early})");
            assert!(late < early * 0.01, "engine {engine}: {early} -> {late}");
        }
    }

    #[test]
    fn pitch_tracks_the_fader_and_tune_is_free() {
        let fs = 96_000.0;
        for (pitch, tune) in [(0.2f32, 0.0f32), (0.5, 7.02), (0.35, -12.0), (0.3, 3.33)] {
            let mut v = sine_voice(fs);
            v.set_param(Param::Pitch, pitch);
            // Oscillator 2 alone, offset by `tune` semitones.
            v.set_param(Param::Level1, 0.0);
            v.set_param(Param::Level2, 1.0);
            v.set_param(Param::Decay2, 1.0);
            v.set_param(Param::Tune2, map::tune_knob(tune));
            v.snap_params();
            v.trigger(1.0);
            let out = render(&mut v, fs as usize);
            let measured = zero_crossings(&out[9_600..]) as f32 / (1.0 - 0.1);
            let expected = map::pitch_hz(pitch, 0.5) * (tune / 12.0).exp2();
            let cents = 1200.0 * (measured / expected).log2();
            assert!(
                cents.abs() < 20.0,
                "{pitch} {tune}: {measured} vs {expected}"
            );
        }
    }

    #[test]
    fn oscillator_decays_are_independent() {
        let fs = 48_000.0;
        let tail = |ratio: f32| {
            let mut v = sine_voice(fs);
            v.set_param(Param::Decay, 0.4);
            v.set_param(Param::Level1, 0.0);
            v.set_param(Param::Level2, 1.0);
            v.set_param(Param::Decay2, ratio);
            v.snap_params();
            v.trigger(1.0);
            let out = render(&mut v, 24_000);
            rms(&out[12_000..]) / rms(&out[..4_800])
        };
        assert!(tail(1.0) > 4.0 * tail(0.4));
    }

    #[test]
    fn hold_keeps_the_body_up() {
        let fs = 48_000.0;
        let level_at_100ms = |hold: f32| {
            let mut v = sine_voice(fs);
            v.set_param(Param::Decay, 0.2);
            v.set_param(Param::Hold, hold);
            v.snap_params();
            v.trigger(1.0);
            let out = render(&mut v, 6_000);
            peak(&out[4_800..])
        };
        assert!(level_at_100ms(0.6) > 5.0 * level_at_100ms(0.0));
    }

    #[test]
    fn attack_ramps_in() {
        let fs = 48_000.0;
        let mut v = sine_voice(fs);
        v.set_param(Param::Attack, 1.0);
        v.snap_params();
        v.trigger(1.0);
        let out = render(&mut v, 9_600);
        assert!(peak(&out[..480]) < 0.25 * peak(&out[4_800..]));
    }

    #[test]
    fn sweep_down_starts_high() {
        let fs = 96_000.0;
        let mut v = sine_voice(fs);
        v.set_param(Param::Pitch, 0.3);
        v.set_param(Param::SweepDepth, 0.85);
        v.set_param(Param::SweepSpeed, 0.4);
        v.snap_params();
        v.trigger(1.0);
        let out = render(&mut v, 96_000);
        let first = zero_crossings(&out[..9_600]);
        let last = zero_crossings(&out[out.len() - 9_600..]);
        assert!(first > last * 2, "{first} vs {last}");
    }

    #[test]
    fn jitter_makes_hits_differ() {
        let fs = 96_000.0;
        let mut v = sine_voice(fs);
        v.set_param(Param::Pitch, 0.5);
        v.set_param(Param::Jitter, 1.0);
        v.snap_params();
        let mut pitches = Vec::new();
        for _ in 0..4 {
            v.trigger(1.0);
            pitches.push(zero_crossings(&render(&mut v, 19_200)));
        }
        let (lo, hi) = (pitches.iter().min().unwrap(), pitches.iter().max().unwrap());
        assert!(*hi as f32 > *lo as f32 * 1.05, "{pitches:?}");
    }

    #[test]
    fn highpass_removes_the_sub() {
        let fs = 48_000.0;
        let level = |ftype: u32| {
            let mut v = sine_voice(fs);
            v.set_param(Param::Pitch, 0.2);
            v.set_param(Param::FilterType, ftype as f32);
            v.set_param(Param::Cutoff, 0.5);
            v.snap_params();
            v.trigger(1.0);
            rms(&render(&mut v, 9_600)[2_400..])
        };
        assert!(level(FILTER_HP) < 0.05 * level(1));
    }

    #[test]
    fn drive_adds_harmonics_but_not_much_level() {
        let fs = 48_000.0;
        let run = |drive: f32, engine: u32| {
            let mut v = sine_voice(fs);
            v.set_param(Param::Engine, engine as f32);
            v.set_param(Param::MacroX, 0.0);
            v.set_param(Param::MacroY, 0.0);
            v.set_param(Param::Drive, drive);
            v.snap_params();
            v.trigger(1.0);
            render(&mut v, 9_600)
        };
        let brightness = |x: &[f32]| {
            let d = x.windows(2).map(|w| (w[1] - w[0]).powi(2)).sum::<f32>();
            (d / x.iter().map(|v| v * v).sum::<f32>()).sqrt()
        };
        for engine in [ENGINE_SYNC, ENGINE_FM, ENGINE_BOOM] {
            let (clean, dirty) = (run(0.0, engine), run(0.8, engine));
            assert!(
                brightness(&dirty) > 2.0 * brightness(&clean),
                "engine {engine}"
            );
            assert!(peak(&dirty) < 2.0 * peak(&clean), "engine {engine}");
        }
    }

    #[test]
    fn extremes_stay_finite() {
        let fs = 96_000.0;
        for engine in 0..ENGINE_COUNT {
            for &corner in &[0.0f32, 1.0] {
                let mut v = Voice::new(fs, 9);
                v.set_param(Param::Engine, engine as f32);
                for p in crate::params::ALL_PARAMS {
                    if matches!(p.info().kind, crate::params::ParamKind::Continuous) {
                        v.set_param(p, corner);
                    }
                }
                v.set_param(Param::Level, 1.0);
                v.set_param(Param::ChaosDepth, 1.0);
                v.set_param(Param::ChaosRate, 1.0);
                v.snap_params();
                v.trigger(1.0);
                // Every EQ band at +18 dB into a self-oscillating ladder is
                // loud by design; the engine's output stage bounds it.
                for s in render(&mut v, 48_000) {
                    assert!(
                        s.is_finite() && s.abs() < 64.0,
                        "engine {engine} corner {corner}: {s}"
                    );
                }
            }
        }
    }
}
