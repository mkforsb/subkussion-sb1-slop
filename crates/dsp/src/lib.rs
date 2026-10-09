//! Subkussion DSP: a dual-channel bass percussion synthesizer. Every channel
//! runs three freely tuned oscillators of one of eight engines (virtual
//! analog with sync, unison Reese, FM, phase distortion, wavetable,
//! wavetable FM, an 808 body and a wavefolder), then a click and noise
//! transient, a saturator, a Moog-style ladder filter, a four-band EQ, a
//! stereo chorus and a plate reverb.
//!
//! The crate has no dependencies and no platform code so it can be compiled to
//! a standalone wasm module for an AudioWorklet as well as used natively.

// The engines loop over several parallel per-oscillator arrays at once;
// index loops read better there than zipped iterators.
#![allow(clippy::needless_range_loop)]

pub mod engine;
pub mod engines;
pub mod exciter;
pub mod fx;
pub mod modulation;
pub mod params;
pub mod presets;
pub mod preview;
pub mod reverb;
pub mod util;
pub mod voice;

pub use engine::{Engine, VOICES};
pub use params::{ALL_PARAMS, PARAM_COUNT, Param, ParamKind, VoicePatch};
