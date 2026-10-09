//! BOOM: the 808 body. A sine, a punch and a shaper.
//!
//! * DRIVE: gain into a `tanh` shaper, up to +30 dB. The drive follows the
//!   oscillator's envelope (and velocity), so a hit starts growling and its
//!   tail rings out as a clean sine, like an overdriven 808 whose distortion
//!   fades with the level.
//! * PUNCH: an extra pitch drop of up to 3 octaves over ~6 ms on top of the
//!   SWEEP: the knock at the front of a kick.
//! * ASYM: offsets the sine into the shaper, so it clips one side first
//!   (even harmonics, a tubby "tone").

use super::{Ctx, Osc, OscCtx};
use crate::params::map;
use crate::util::{Noise, fast_tanh, sin_turns};

const PUNCH_TAU_S: f32 = 0.006;

#[derive(Clone, Debug, Default)]
pub struct Boom {
    phase: f32,
    inc: f32,
    punch: f32,
    punch_oct: f32,
    punch_mul: f32,
    drive: f32,
    bias: f32,
    offset: f32,
    norm: f32,
}

impl Osc for Boom {
    fn reset(&mut self) {
        self.phase = 0.0;
        self.punch = 0.0;
    }

    fn trigger(&mut self, c: &Ctx, o: &OscCtx, _rng: &mut Noise) {
        self.reset();
        self.punch = 1.0;
        self.control(c, o);
    }

    fn control(&mut self, c: &Ctx, o: &OscCtx) {
        // The punch is applied per sample; keep the base increment here.
        self.inc = o.hz / c.fs;
        self.punch_oct = map::punch_octaves(c.y);
        self.punch_mul = (-1.0 / (PUNCH_TAU_S * c.fs)).exp();
        let peak = (map::boom_drive_db(c.x) / 20.0 * core::f32::consts::LN_10).exp();
        self.drive = 1.0 + (peak - 1.0) * o.env * (0.5 + 0.5 * c.velocity);
        self.bias = 0.8 * c.z;
        // Remove the static offset of the biased shaper, then normalize the
        // larger of the two peaks to 1.
        self.offset = fast_tanh(self.drive * self.bias);
        let hi = fast_tanh(self.drive * (1.0 + self.bias)) - self.offset;
        let lo = self.offset - fast_tanh(self.drive * (self.bias - 1.0));
        self.norm = 1.0 / hi.max(lo).max(1e-3);
    }

    #[inline]
    fn tick(&mut self) -> f32 {
        let s = sin_turns(self.phase);
        let y = (fast_tanh(self.drive * (s + self.bias)) - self.offset) * self.norm;
        let mut step = self.inc;
        if self.punch > 1e-4 {
            step *= (self.punch_oct * self.punch).exp2();
            self.punch *= self.punch_mul;
        }
        self.phase += step.min(0.45);
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }
        y
    }
}
