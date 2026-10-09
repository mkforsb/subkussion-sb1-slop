//! FOLD: a west-coast style sine wavefolder.
//!
//! The sine is amplified and passed through another sine, so every time it
//! would overshoot it folds back, adding a pair of harmonics per fold. With
//! F.ENV up, the fold depth follows the envelope like a low-pass gate's
//! brightness: a hit opens into a wiry, vocal tone and closes to a sine.
//!
//! * FOLD: peak folder gain, ×1 … ×10; velocity adds to it.
//! * SYMM: bipolar offset into the folder (even harmonics, a nasal edge).
//! * F.ENV: how much of FOLD follows the envelope.

use super::{Ctx, Osc, OscCtx, inc};
use crate::params::map;
use crate::util::{Noise, sin_turns};

#[derive(Clone, Debug, Default)]
pub struct Fold {
    phase: f32,
    inc: f32,
    gain: f32,
    bias: f32,
    offset: f32,
}

impl Osc for Fold {
    fn reset(&mut self) {
        self.phase = 0.0;
    }

    fn trigger(&mut self, c: &Ctx, o: &OscCtx, _rng: &mut Noise) {
        self.reset();
        self.control(c, o);
    }

    fn control(&mut self, c: &Ctx, o: &OscCtx) {
        self.inc = inc(o.hz, c.fs);
        let env = 1.0 - c.z + c.z * o.env;
        let depth = map::fold_gain(c.x) - 1.0;
        self.gain = 1.0 + depth * env * (0.6 + 0.4 * c.velocity);
        self.bias = (c.y - 0.5) * 1.2;
        // y(0) = 0, so a phase reset never clicks.
        self.offset = sin_turns(0.25 * self.gain * self.bias);
    }

    #[inline]
    fn tick(&mut self) -> f32 {
        let s = sin_turns(self.phase);
        let y = sin_turns(0.25 * self.gain * (s + self.bias)) - self.offset;
        self.phase += self.inc;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }
        // The offset can push one side past 1; halve it back into range.
        y * (1.0 / (1.0 + 0.5 * self.offset.abs()))
    }
}
