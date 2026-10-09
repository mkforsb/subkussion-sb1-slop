//! TABLE: a band-limited wavetable oscillator.
//!
//! * POS: position in the bank, crossfading through SINE, OCTAVE, TRI, ORGAN,
//!   NASAL, SQUARE, SAW and BUZZ (dark → bright).
//! * SCAN: bipolar. The hit starts this far from POS and travels back to it
//!   as the envelope decays: up for a bright attack that darkens, down for one
//!   that brightens.
//! * BEND: bipolar phase bend (like Serum's "bend"). The cycle is read
//!   faster in one half than the other, squeezing the wave to one side; it
//!   adds harmonics even to the sine. The warp is a sine,
//!   `p − b·sin(2πp)/2π`, rather than a kinked line, so it stays smooth and
//!   band-limited: it is phase modulation at ratio 1.

use super::tables::{FRAMES, Mip, Tables, tables};
use super::{Ctx, Osc, OscCtx, inc};
use crate::util::{Noise, sin_turns, wrap};

/// Phase-warp depth at the ends of the BEND knob (below 1 keeps the warp
/// monotonic: the wave is bent, never read backwards).
const MAX_BEND: f32 = 0.9;

#[derive(Clone, Debug, Default)]
pub struct Table {
    phase: f32,
    inc: f32,
    pos: f32,
    /// Warp depth in turns (`b / 2π`).
    bend: f32,
    mip: Mip,
}

impl Osc for Table {
    fn reset(&mut self) {
        self.phase = 0.0;
    }

    fn trigger(&mut self, c: &Ctx, o: &OscCtx, _rng: &mut Noise) {
        self.reset();
        self.control(c, o);
    }

    fn control(&mut self, c: &Ctx, o: &OscCtx) {
        self.inc = inc(o.hz, c.fs);
        let scan = (c.y - 0.5) * 2.0 * o.env;
        self.pos = (c.x + scan).clamp(0.0, 1.0) * (FRAMES - 1) as f32;
        let b = (c.z - 0.5) * 2.0 * MAX_BEND;
        self.bend = b / core::f32::consts::TAU;
        // As on WT-FM: harmonic k swings to k·(1 + |b|) times the pitch.
        let sidebands = if b.abs() > 0.01 { o.hz } else { 0.0 };
        self.mip = Tables::mip_blend(o.hz * (1.0 + b.abs()), sidebands, c.fs);
    }

    #[inline]
    fn tick(&mut self) -> f32 {
        let p = self.phase;
        let y = tables().read_mip(self.pos, self.mip, wrap(p - self.bend * sin_turns(p)));
        self.phase += self.inc;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }
        y
    }
}
