//! The eight synthesis engines.
//!
//! Every engine is an *oscillator type*: the voice runs three instances of
//! the selected one (OSC 1–3), each at its own freely tuned pitch and with its
//! own amplitude envelope, and mixes them. All three share the engine's macro
//! knobs.
//!
//! [`Ctx`] carries the per-block values every oscillator shares (the macros
//! after chaos and jitter, the velocity), [`OscCtx`] the per-oscillator ones
//! (pitch after sweep, chaos and jitter, its envelope). `control` recomputes
//! coefficients once per control block and `tick` renders one sample, before
//! the voice applies the envelope.

pub mod boom;
pub mod fm;
pub mod fold;
pub mod pd;
pub mod reese;
pub mod sync;
pub mod table;
pub mod tables;
pub mod wtfm;

use crate::util::Noise;

/// Oscillators per voice.
pub const OSCS: usize = 3;

/// Control values shared by the three oscillators for one block.
#[derive(Clone, Copy, Debug)]
pub struct Ctx {
    /// Rate `tick` is called at.
    pub fs: f32,
    /// Engine macros, `0..=1`, after chaos and jitter.
    pub x: f32,
    pub y: f32,
    pub z: f32,
    /// Velocity of the current hit after SENSE, `0..=1`.
    pub velocity: f32,
}

impl Ctx {
    pub fn new(fs: f32) -> Self {
        Self {
            fs,
            x: 0.3,
            y: 0.3,
            z: 0.3,
            velocity: 1.0,
        }
    }
}

/// Control values for one oscillator for one block.
#[derive(Clone, Copy, Debug)]
pub struct OscCtx {
    /// Pitch in Hz, after sweep, chaos and jitter.
    pub hz: f32,
    /// This oscillator's amplitude envelope relative to the hit's velocity:
    /// 1 at the peak, decaying towards 0.
    pub env: f32,
    /// This oscillator's -60 dB decay time in seconds.
    pub decay_s: f32,
}

impl Default for OscCtx {
    fn default() -> Self {
        Self {
            hz: 55.0,
            env: 0.0,
            decay_s: 0.5,
        }
    }
}

/// One oscillator of an engine.
pub trait Osc: Clone + Default {
    /// Silence and clear state (engine switch, numerical accident).
    fn reset(&mut self);
    /// A new hit: restart phases and per-hit envelopes.
    fn trigger(&mut self, c: &Ctx, o: &OscCtx, rng: &mut Noise);
    /// Recompute coefficients for the coming block.
    fn control(&mut self, c: &Ctx, o: &OscCtx);
    /// One sample, roughly within ±1, before the amplitude envelope.
    fn tick(&mut self) -> f32;
}

/// All engines, three oscillators each. Only the selected engine runs.
#[derive(Clone, Debug, Default)]
pub struct Engines {
    sync: [sync::Sync; OSCS],
    reese: [reese::Reese; OSCS],
    fm: [fm::Fm; OSCS],
    pd: [pd::Pd; OSCS],
    table: [table::Table; OSCS],
    wtfm: [wtfm::WtFm; OSCS],
    boom: [boom::Boom; OSCS],
    fold: [fold::Fold; OSCS],
}

/// Run `$body` with `$o` bound to the selected engine's oscillator array.
macro_rules! with_engine {
    ($self:ident, $engine:expr, $o:ident => $body:expr) => {
        match $engine {
            0 => {
                let $o = &mut $self.sync;
                $body
            }
            1 => {
                let $o = &mut $self.reese;
                $body
            }
            2 => {
                let $o = &mut $self.fm;
                $body
            }
            3 => {
                let $o = &mut $self.pd;
                $body
            }
            4 => {
                let $o = &mut $self.table;
                $body
            }
            5 => {
                let $o = &mut $self.wtfm;
                $body
            }
            6 => {
                let $o = &mut $self.boom;
                $body
            }
            _ => {
                let $o = &mut $self.fold;
                $body
            }
        }
    };
}

#[inline]
fn mix<T: Osc>(oscs: &mut [T; OSCS], gains: &[f32; OSCS]) -> f32 {
    let mut y = 0.0;
    for (o, &g) in oscs.iter_mut().zip(gains) {
        // A silent oscillator doesn't need to run: its phase is irrelevant
        // until the next trigger restarts it.
        if g != 0.0 {
            y += g * o.tick();
        }
    }
    y
}

impl Engines {
    pub fn new() -> Self {
        // Build the shared wavetables now rather than on the first hit.
        let _ = tables::tables();
        Self::default()
    }

    pub fn reset(&mut self, engine: u32) {
        with_engine!(self, engine, o => o.iter_mut().for_each(Osc::reset));
    }

    pub fn control(&mut self, engine: u32, c: &Ctx, oscs: &[OscCtx; OSCS]) {
        with_engine!(self, engine, o => {
            for (osc, oc) in o.iter_mut().zip(oscs) {
                osc.control(c, oc);
            }
        });
    }

    pub fn trigger(&mut self, engine: u32, c: &Ctx, oscs: &[OscCtx; OSCS], rng: &mut Noise) {
        with_engine!(self, engine, o => {
            for (osc, oc) in o.iter_mut().zip(oscs) {
                osc.trigger(c, oc, rng);
            }
        });
    }

    /// One sample of the three oscillators, each scaled by its gain
    /// (level × envelope).
    #[inline]
    pub fn tick(&mut self, engine: u32, gains: &[f32; OSCS]) -> f32 {
        with_engine!(self, engine, o => mix(o, gains))
    }
}

/// Phase increment for `hz` at `fs`, kept below the polyBLEP/aliasing limit.
#[inline]
pub(crate) fn inc(hz: f32, fs: f32) -> f32 {
    (hz / fs).clamp(0.0, 0.45)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::{ENGINE_COUNT, ENGINE_FM, ENGINE_REESE, ENGINE_WTFM, map};

    /// Fraction (dB) of the 20 Hz … 20 kHz energy that is *not* within a few
    /// Hz of a harmonic of `f0`: aliases fold onto inharmonic frequencies.
    fn inharmonic_db(x: &[f32], f0: f32, fs: f32) -> f32 {
        let n = x.len();
        let mut re: Vec<f64> = x
            .iter()
            .enumerate()
            .map(|(i, &v)| {
                // 4-term Blackman-Harris: sidelobes ~ -92 dB.
                let t = core::f64::consts::TAU * i as f64 / n as f64;
                let w = 0.35875 - 0.48829 * t.cos() + 0.14128 * (2.0 * t).cos()
                    - 0.01168 * (3.0 * t).cos();
                v as f64 * w
            })
            .collect();
        let mut im = vec![0.0; n];
        tables::fft(&mut re, &mut im, false);
        let (mut total, mut off) = (0.0, 0.0);
        for bin in 1..n / 2 {
            let f = bin as f32 * fs / n as f32;
            if !(20.0..20_000.0).contains(&f) {
                continue;
            }
            let p = re[bin] * re[bin] + im[bin] * im[bin];
            total += p;
            let k = (f / f0).round();
            if k < 1.0 || (f - k * f0).abs() > 4.0 * fs / n as f32 {
                off += p;
            }
        }
        10.0 * (off / total).log10() as f32
    }

    /// Every engine at bright, static settings: the aliasing stays far down.
    #[test]
    fn engines_do_not_alias() {
        let fs = 96_000.0;
        for engine in 0..ENGINE_COUNT as u32 {
            for hz in [157.6f32, 549.0] {
                let mut c = Ctx::new(fs);
                (c.x, c.y, c.z) = (0.75, 0.8, 0.0);
                if engine == ENGINE_REESE {
                    // Detuned voices aren't harmonic; test the oscillators.
                    (c.x, c.z) = (0.0, 0.5);
                }
                if engine == ENGINE_FM {
                    // Ratio 2, so all sidebands land on harmonics.
                    c.x = 0.6;
                    assert!((map::fm_ratio(c.x) - 2.0).abs() < 1e-5);
                }
                if engine == ENGINE_WTFM {
                    // Ratio 1, so all sidebands land on harmonics.
                    c.z = 0.4;
                    assert!((map::fm_ratio(c.z) - 1.0).abs() < 1e-5);
                }
                // An endless decay keeps index envelopes static.
                let o = OscCtx {
                    hz,
                    env: 1.0,
                    decay_s: 1.0e6,
                };
                let oscs = [o; OSCS];
                let mut e = Engines::new();
                e.trigger(engine, &c, &oscs, &mut Noise::new(1));
                let gains = [1.0, 0.0, 0.0];
                let x: Vec<f32> = (0..9_600 + 32_768)
                    .map(|i| {
                        if i % 16 == 0 {
                            e.control(engine, &c, &oscs);
                        }
                        e.tick(engine, &gains)
                    })
                    .skip(9_600)
                    .collect();
                let db = inharmonic_db(&x, hz, fs);
                assert!(db < -45.0, "engine {engine} at {hz} Hz: {db:.1} dB");
            }
        }
    }
}
