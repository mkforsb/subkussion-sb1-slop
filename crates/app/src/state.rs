//! Panel state, the controller shared through the Dioxus context, and
//! persistence (localStorage on the web, `~/.config/subkussion` on desktop).

use std::cell::RefCell;
use std::rc::Rc;

use dioxus::prelude::*;
use subkussion_dsp::params::{FILTER_BP, FILTER_HP, ParamKind, default_patch};
use subkussion_dsp::util::Noise;
use subkussion_dsp::{ALL_PARAMS, PARAM_COUNT, Param, VOICES, VoicePatch};

use crate::audio::{AudioHandle, Command, ParamCache};

/// Which parameters of one channel are locked against RND and MUTATE.
pub type Locks = [bool; PARAM_COUNT];

pub const NO_LOCKS: Locks = [false; PARAM_COUNT];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PanelState {
    pub voices: [VoicePatch; VOICES],
    pub master: f32,
    /// Per-channel MUTATE knob: 0 = off … 1 = full randomization on every
    /// trigger. A performance control rather than part of the sound, so it
    /// lives outside the patch and survives presets, RND and channel copies.
    pub mutate: [f32; VOICES],
    /// Per-channel parameter locks (Ctrl-click a control). Like MUTATE they
    /// belong to the channel, not the patch: presets and copies leave them be.
    pub locks: [Locks; VOICES],
}

impl Default for PanelState {
    fn default() -> Self {
        // Give the two channels different starting sounds.
        let preset = |name: &str| {
            subkussion_dsp::presets::PRESETS
                .iter()
                .find(|p| p.name == name)
                .map(|p| p.patch())
                .unwrap_or_else(default_patch)
        };
        let mut voices = [preset("808 Long"), preset("Hollow Reese")];
        voices[0][Param::Pan.index()] = 0.5;
        Self {
            voices,
            master: subkussion_dsp::engine::DEFAULT_MASTER,
            mutate: [0.0; VOICES],
            locks: [NO_LOCKS; VOICES],
        }
    }
}

/// Can this control be locked? Everything RND and MUTATE would change.
pub fn lockable(p: Param) -> bool {
    !p.is_performance()
}

/// A random patch, as produced by the RND button (and MUTATE at RND on every
/// hit). Performance controls (level, sense, pan) and `locks`ed controls keep
/// their values from `current`.
///
/// Every control draws exactly one number, in parameter order, whether or
/// not it is locked, so locking one control never changes what the others
/// roll. The draws are shaped so the results are playable bass percussion
/// on every engine: low pitches, mostly downward sweeps, filters that
/// rarely close all the way, and effects that are usually subtle and
/// occasionally extreme.
pub fn random_patch(current: &VoicePatch, locks: &Locks, rng: &mut Noise) -> VoicePatch {
    let mut patch = *current;
    let mut draws = [0.0f32; PARAM_COUNT];
    for p in ALL_PARAMS {
        draws[p.index()] = rng.sample() * 0.5 + 0.5;
    }
    let bipolar = |x: f32, depth: f32| 0.5 + depth * (2.0 * x - 1.0).powi(3);
    // Often off, otherwise anywhere up to `max`.
    let sometimes = |x: f32, off: f32, max: f32| ((x - off) / (1.0 - off)).max(0.0) * max;
    let free = |p: Param| !p.is_performance() && !locks[p.index()];
    for p in ALL_PARAMS {
        if !free(p) {
            continue;
        }
        let x = draws[p.index()];
        patch[p.index()] = p.sanitize(match (p, p.info().kind) {
            // Lowpass more often than not: it's a bass machine.
            (Param::FilterType, _) => match x {
                x if x < 0.35 => 0.0,
                x if x < 0.75 => 1.0,
                x if x < 0.88 => 2.0,
                _ => 3.0,
            },
            (_, ParamKind::Choice(names)) => (x * names.len() as f32).floor(),
            // 24 Hz … 160 Hz.
            (Param::Pitch, _) => 0.1 + 0.45 * x,
            (Param::Fine, _) => 0.3 + 0.4 * x,
            (Param::Decay, _) => 0.25 + 0.55 * x,
            (Param::Level1, _) => 0.65 + 0.35 * x,
            (Param::Level2 | Param::Level3, _) => sometimes(x, 0.4, 0.9),
            (Param::Decay2 | Param::Decay3, _) => 0.3 + 0.6 * x,
            (Param::Attack, _) => 0.3 * x.powi(3),
            (Param::Hold, _) => 0.55 * x.powi(3),
            (Param::Click, _) => 0.6 * x * x,
            (Param::Noise, _) => 0.45 * x.powi(3),
            (Param::SweepSpeed, _) => 0.35 + 0.5 * x,
            // Mostly a downward drop, the kick's signature; now and then a
            // small rise.
            (Param::SweepDepth, _) => 0.44 + 0.46 * x,
            (Param::ChaosDepth, _) => 0.6 * x * x,
            (Param::Jitter, _) => 0.25 * x * x,
            (Param::Cutoff, _) => 0.45 + 0.55 * x,
            (Param::Reso, _) => 0.75 * x * x,
            (Param::FilterEnv, _) => 0.5 + 0.4 * x * x,
            (Param::Drive, _) => 0.7 * x * x,
            (Param::SatMix, _) => 0.5 + 0.5 * x,
            // Rarely high enough to thin the sub.
            (Param::LowCut, _) => sometimes(x, 0.6, 0.3),
            (Param::LowGain, _) => bipolar(x, 0.3),
            (Param::MidGain, _) => bipolar(x, 0.35),
            (Param::MidQ, _) => 0.2 + 0.5 * x,
            (Param::HighGain, _) => bipolar(x, 0.3),
            (Param::ChorusMix, _) => sometimes(x, 0.45, 0.8),
            (Param::ChorusRate, _) => 0.2 + 0.6 * x,
            (Param::ChorusDepth, _) => 0.2 + 0.7 * x,
            (Param::ChorusXover, _) => 0.3 + 0.5 * x,
            // Usually dryish, sometimes drenched.
            (Param::ReverbMix, _) => 0.6 * x * x,
            _ => x,
        });
    }
    // A highpass or bandpass with a lowpass's cutoff would leave little of a
    // bass: pull the cutoff down into the body of the sound instead.
    let ftype = patch[Param::FilterType.index()] as u32;
    if free(Param::Cutoff) && (ftype == FILTER_HP || ftype == FILTER_BP) {
        let x = draws[Param::Cutoff.index()];
        patch[Param::Cutoff.index()] = if ftype == FILTER_HP {
            0.3 * x
        } else {
            0.3 + 0.4 * x
        };
    }
    patch
}

/// Move every free control `amount` (0..=1) of the way towards a fresh random
/// patch: `clamp(current + (random - current) * amount)`. At 1 this is exactly
/// [`random_patch`]; choice controls round to the nearest position. Locked
/// controls never move.
pub fn mutate_patch(
    current: &VoicePatch,
    amount: f32,
    locks: &Locks,
    rng: &mut Noise,
) -> VoicePatch {
    let target = random_patch(current, locks, rng);
    if amount >= 1.0 {
        return target;
    }
    let amount = amount.max(0.0);
    let mut patch = *current;
    for p in ALL_PARAMS {
        let (c, t) = (current[p.index()], target[p.index()]);
        patch[p.index()] = p.sanitize(c + (t - c) * amount);
    }
    patch
}

pub fn mutate_display(amount: f32) -> String {
    if amount <= 0.0 {
        "OFF".into()
    } else if amount >= 1.0 {
        "RND".into()
    } else {
        format!("{:.0}%", amount * 100.0)
    }
}

const FORMAT_HEADER: &str = "subkussion-state 1";

impl PanelState {
    pub fn to_cache(self) -> ParamCache {
        ParamCache {
            voices: self.voices,
            master: self.master,
        }
    }

    pub fn serialize(&self) -> String {
        let mut out = format!("{FORMAT_HEADER}\nmaster {}\n", self.master);
        for (v, m) in self.mutate.iter().enumerate() {
            out += &format!("mutate {v} {m}\n");
        }
        for (v, locks) in self.locks.iter().enumerate() {
            for p in ALL_PARAMS.iter().filter(|p| locks[p.index()]) {
                out += &format!("lock {v} {p:?}\n");
            }
        }
        for (v, patch) in self.voices.iter().enumerate() {
            for p in ALL_PARAMS {
                out += &format!("{v} {p:?} {}\n", patch[p.index()]);
            }
        }
        out
    }

    /// Lenient: unknown lines are skipped and missing values keep defaults.
    pub fn deserialize(text: &str) -> Option<Self> {
        let mut lines = text.lines();
        if lines.next()?.trim() != FORMAT_HEADER {
            return None;
        }
        let param_named = |name: &str| {
            ALL_PARAMS
                .iter()
                .copied()
                .find(|p| format!("{p:?}") == name)
        };
        let mut state = PanelState::default();
        for line in lines {
            let parts: Vec<&str> = line.split_whitespace().collect();
            match parts.as_slice() {
                ["master", v] => {
                    if let Ok(v) = v.parse::<f32>()
                        && v.is_finite()
                    {
                        state.master = v.clamp(0.0, 1.0);
                    }
                }
                ["mutate", voice, v] => {
                    if let (Ok(voice), Ok(v)) = (voice.parse::<usize>(), v.parse::<f32>())
                        && let Some(m) = state.mutate.get_mut(voice)
                        && v.is_finite()
                    {
                        *m = v.clamp(0.0, 1.0);
                    }
                }
                ["lock", voice, name] => {
                    if let (Ok(voice), Some(p)) = (voice.parse::<usize>(), param_named(name))
                        && let Some(locks) = state.locks.get_mut(voice)
                        && lockable(p)
                    {
                        locks[p.index()] = true;
                    }
                }
                [voice, name, v] => {
                    let (Ok(voice), Ok(v)) = (voice.parse::<usize>(), v.parse::<f32>()) else {
                        continue;
                    };
                    if let (Some(patch), Some(p)) = (state.voices.get_mut(voice), param_named(name))
                    {
                        patch[p.index()] = p.sanitize(v);
                    }
                }
                _ => {}
            }
        }
        Some(state)
    }
}

/// What a drag gesture is adjusting.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Target {
    Param(usize, Param),
    Master,
    /// A channel's MUTATE knob.
    Mutate(usize),
}

impl Target {
    /// The lockable parameter behind this control, if any.
    pub fn lockable(self) -> Option<(usize, Param)> {
        match self {
            Target::Param(v, p) if lockable(p) => Some((v, p)),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Drag {
    pub target: Target,
    pub start_y: f64,
    pub start_value: f32,
    /// Pixels of vertical travel for the full range.
    pub span_px: f64,
    pub fine: bool,
}

/// Everything the controls need, shared via `use_context::<Synth>()`.
#[derive(Clone)]
pub struct Synth {
    pub state: Signal<PanelState>,
    pub audio: AudioHandle,
    /// Last touched parameter per channel, for the channel display.
    pub touched: Signal<[Option<Param>; VOICES]>,
    /// Trigger counters per channel; the trigger LED keys its flash on these.
    pub hits: Signal<[u32; VOICES]>,
    pub drag: Signal<Option<Drag>>,
    /// One generator for RND and MUTATE, so rapid hits never share a seed.
    rng: Rc<RefCell<Noise>>,
}

impl Synth {
    pub fn new() -> Self {
        let initial = load().unwrap_or_default();
        Self {
            audio: AudioHandle::new(initial.to_cache()),
            state: Signal::new(initial),
            touched: Signal::new([None; VOICES]),
            hits: Signal::new([0; VOICES]),
            drag: Signal::new(None),
            rng: Rc::new(RefCell::new(Noise::new(random_seed()))),
        }
    }

    pub fn value(&self, target: Target) -> f32 {
        let s = self.state.read();
        match target {
            Target::Param(v, p) => s.voices[v][p.index()],
            Target::Master => s.master,
            Target::Mutate(v) => s.mutate[v],
        }
    }

    pub fn set(&self, target: Target, value: f32) {
        match target {
            Target::Param(voice, param) => self.set_param(voice, param, value),
            Target::Master => {
                let value = value.clamp(0.0, 1.0);
                let mut state = self.state;
                if state.peek().master != value {
                    state.write().master = value;
                    self.audio.send(Command::Master(value));
                }
            }
            Target::Mutate(voice) => {
                // UI-only: nothing to send, it acts when the channel is triggered.
                let value = value.clamp(0.0, 1.0);
                let mut state = self.state;
                if state.peek().mutate[voice] != value {
                    state.write().mutate[voice] = value;
                }
            }
        }
    }

    pub fn set_param(&self, voice: usize, param: Param, value: f32) {
        let value = param.sanitize(value);
        let mut state = self.state;
        if state.peek().voices[voice][param.index()] != value {
            state.write().voices[voice][param.index()] = value;
            self.audio.set_param(voice, param, value);
        }
        self.touch(voice, param);
    }

    fn touch(&self, voice: usize, param: Param) {
        let mut touched = self.touched;
        if touched.peek()[voice] != Some(param) {
            touched.write()[voice] = Some(param);
        }
    }

    pub fn reset(&self, target: Target) {
        match target {
            Target::Param(_, p) => self.set(target, p.default_value()),
            Target::Master => self.set(target, subkussion_dsp::engine::DEFAULT_MASTER),
            Target::Mutate(_) => self.set(target, 0.0),
        }
        self.save();
    }

    // --- locks ---------------------------------------------------------------

    /// Reactive: re-renders the caller when the lock changes.
    pub fn is_locked(&self, target: Target) -> bool {
        target
            .lockable()
            .is_some_and(|(v, p)| self.state.read().locks[v][p.index()])
    }

    /// Ctrl-click: lock or unlock a control. Returns whether it was lockable.
    pub fn toggle_lock(&self, target: Target) -> bool {
        let Some((voice, param)) = target.lockable() else {
            return false;
        };
        let mut state = self.state;
        let locked = !state.peek().locks[voice][param.index()];
        state.write().locks[voice][param.index()] = locked;
        self.touch(voice, param);
        self.save();
        true
    }

    pub fn lock_count(&self, voice: usize) -> usize {
        self.state.read().locks[voice]
            .iter()
            .filter(|&&l| l)
            .count()
    }

    pub fn clear_locks(&self, voice: usize) {
        let mut state = self.state;
        state.write().locks[voice] = NO_LOCKS;
        self.save();
    }

    // --- patches -------------------------------------------------------------

    pub fn load_patch(&self, voice: usize, patch: &VoicePatch) {
        self.apply_patch(voice, patch);
        self.save();
    }

    /// Set a whole patch, sending only the values that changed. The channel
    /// display falls back to its summary (engine, tune, decay).
    fn apply_patch(&self, voice: usize, patch: &VoicePatch) {
        let mut state = self.state;
        for p in ALL_PARAMS {
            let value = p.sanitize(patch[p.index()]);
            if state.peek().voices[voice][p.index()] != value {
                state.write().voices[voice][p.index()] = value;
                self.audio.set_param(voice, p, value);
            }
        }
        let mut touched = self.touched;
        touched.write()[voice] = None;
    }

    /// Randomize the free sound-shaping controls, leaving level, sense, pan
    /// and anything locked alone.
    pub fn randomize(&self, voice: usize) {
        let (current, locks) = {
            let s = self.state.peek();
            (s.voices[voice], s.locks[voice])
        };
        let patch = random_patch(&current, &locks, &mut self.rng.borrow_mut());
        self.load_patch(voice, &patch);
    }

    pub fn copy_voice(&self, from: usize, to: usize) {
        let mut patch = self.state.peek().voices[from];
        // Keep the destination's placement in the stereo field.
        patch[Param::Pan.index()] = self.state.peek().voices[to][Param::Pan.index()];
        self.load_patch(to, &patch);
    }

    /// Hit a channel. With MUTATE above zero the channel's patch is mutated
    /// first, and the trigger snaps the voice so the new sound starts exactly
    /// on this hit rather than gliding in.
    pub fn trigger(&self, voice: usize, velocity: f32) {
        self.audio.user_gesture();
        let amount = self.state.peek().mutate[voice];
        let mutated = amount > 0.0;
        if mutated {
            let (current, locks) = {
                let s = self.state.peek();
                (s.voices[voice], s.locks[voice])
            };
            let patch = mutate_patch(&current, amount, &locks, &mut self.rng.borrow_mut());
            self.apply_patch(voice, &patch);
            self.save();
        }
        self.audio.trigger(voice, velocity, mutated);
        let mut hits = self.hits;
        hits.write()[voice] += 1;
    }

    pub fn save(&self) {
        save(&self.state.peek());
    }
}

#[cfg(all(feature = "web", not(feature = "desktop")))]
fn random_seed() -> u32 {
    (js_sys::Math::random() * u32::MAX as f64) as u32
}

#[cfg(not(all(feature = "web", not(feature = "desktop"))))]
fn random_seed() -> u32 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos() ^ d.as_secs() as u32)
        .unwrap_or(12345)
}

// --- persistence -----------------------------------------------------------

#[cfg(all(feature = "web", not(feature = "desktop")))]
const STORAGE_KEY: &str = "subkussion.state";

#[cfg(all(feature = "web", not(feature = "desktop")))]
fn load() -> Option<PanelState> {
    let storage = web_sys::window()?.local_storage().ok()??;
    PanelState::deserialize(&storage.get_item(STORAGE_KEY).ok()??)
}

#[cfg(all(feature = "web", not(feature = "desktop")))]
fn save(state: &PanelState) {
    if let Some(Ok(Some(storage))) = web_sys::window().map(|w| w.local_storage()) {
        let _ = storage.set_item(STORAGE_KEY, &state.serialize());
    }
}

#[cfg(not(all(feature = "web", not(feature = "desktop"))))]
fn state_path() -> Option<std::path::PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .filter(|s| !s.is_empty())
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".config"))
        })?;
    Some(base.join("subkussion").join("state.txt"))
}

#[cfg(not(all(feature = "web", not(feature = "desktop"))))]
fn load() -> Option<PanelState> {
    PanelState::deserialize(&std::fs::read_to_string(state_path()?).ok()?)
}

#[cfg(not(all(feature = "web", not(feature = "desktop"))))]
fn save(state: &PanelState) {
    let Some(path) = state_path() else { return };
    let result = path
        .parent()
        .map(std::fs::create_dir_all)
        .unwrap_or(Ok(()))
        .and_then(|_| std::fs::write(&path, state.serialize()));
    if let Err(e) = result {
        eprintln!(
            "subkussion: could not save state to {}: {e}",
            path.display()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use subkussion_dsp::params::ENGINE_COUNT;

    fn is_sanitized(patch: &VoicePatch) -> bool {
        ALL_PARAMS
            .iter()
            .all(|p| p.sanitize(patch[p.index()]) == patch[p.index()])
    }

    #[test]
    fn mutate_off_changes_nothing() {
        let current = default_patch();
        assert_eq!(
            mutate_patch(&current, 0.0, &NO_LOCKS, &mut Noise::new(7)),
            current
        );
    }

    #[test]
    fn mutate_at_rnd_equals_the_rnd_button() {
        let current = default_patch();
        let a = mutate_patch(&current, 1.0, &NO_LOCKS, &mut Noise::new(7));
        let b = random_patch(&current, &NO_LOCKS, &mut Noise::new(7));
        assert_eq!(a, b);
        assert_ne!(a, current);
    }

    #[test]
    fn partial_mutation_moves_part_way_towards_the_random_patch() {
        let current = default_patch();
        for seed in 1..50 {
            let target = random_patch(&current, &NO_LOCKS, &mut Noise::new(seed));
            let half = mutate_patch(&current, 0.5, &NO_LOCKS, &mut Noise::new(seed));
            assert!(is_sanitized(&half));
            for p in ALL_PARAMS {
                let (c, t, h) = (current[p.index()], target[p.index()], half[p.index()]);
                match p.info().kind {
                    ParamKind::Continuous => {
                        assert!((h - (c + (t - c) * 0.5)).abs() < 1e-6, "{p:?}")
                    }
                    ParamKind::Choice(_) => assert_eq!(h, p.sanitize(c + (t - c) * 0.5), "{p:?}"),
                }
            }
        }
    }

    #[test]
    fn mutation_keeps_level_sense_and_pan() {
        let mut current = default_patch();
        current[Param::Level.index()] = 0.33;
        current[Param::Sense.index()] = 0.66;
        current[Param::Pan.index()] = 0.1;
        let mut rng = Noise::new(3);
        for amount in [0.25, 0.5, 1.0] {
            let m = mutate_patch(&current, amount, &NO_LOCKS, &mut rng);
            for p in [Param::Level, Param::Sense, Param::Pan] {
                assert_eq!(m[p.index()], current[p.index()]);
            }
        }
    }

    #[test]
    fn locked_controls_never_move() {
        let mut current = default_patch();
        current[Param::Engine.index()] = 3.0;
        current[Param::Pitch.index()] = 0.123;
        current[Param::FilterType.index()] = FILTER_HP as f32;
        current[Param::Cutoff.index()] = 0.9;
        let mut locks = NO_LOCKS;
        for p in [
            Param::Engine,
            Param::Pitch,
            Param::FilterType,
            Param::Cutoff,
        ] {
            locks[p.index()] = true;
        }
        let mut rng = Noise::new(17);
        let mut patch = current;
        for i in 0..500 {
            patch = mutate_patch(&patch, [0.3, 1.0][i % 2], &locks, &mut rng);
            for p in [
                Param::Engine,
                Param::Pitch,
                Param::FilterType,
                Param::Cutoff,
            ] {
                assert_eq!(patch[p.index()], current[p.index()], "{p:?}");
            }
        }
        // …while the free controls do move.
        assert_ne!(patch[Param::Decay.index()], current[Param::Decay.index()]);
    }

    #[test]
    fn locking_one_control_leaves_the_other_rolls_unchanged() {
        let current = default_patch();
        let mut locks = NO_LOCKS;
        locks[Param::Decay.index()] = true;
        locks[Param::MacroY.index()] = true;
        let a = random_patch(&current, &NO_LOCKS, &mut Noise::new(42));
        let b = random_patch(&current, &locks, &mut Noise::new(42));
        for p in ALL_PARAMS {
            if !locks[p.index()] {
                assert_eq!(a[p.index()], b[p.index()], "{p:?}");
            }
        }
    }

    #[test]
    fn a_locked_highpass_still_gets_a_low_cutoff() {
        let mut current = default_patch();
        current[Param::FilterType.index()] = FILTER_HP as f32;
        let mut locks = NO_LOCKS;
        locks[Param::FilterType.index()] = true;
        let mut rng = Noise::new(9);
        for _ in 0..100 {
            let p = random_patch(&current, &locks, &mut rng);
            assert!(p[Param::Cutoff.index()] <= 0.3);
        }
    }

    #[test]
    fn repeated_mutation_stays_in_range() {
        let mut patch = default_patch();
        let mut rng = Noise::new(11);
        for i in 0..1000 {
            patch = mutate_patch(&patch, [0.1, 0.5, 0.9][i % 3], &NO_LOCKS, &mut rng);
            assert!(is_sanitized(&patch));
        }
    }

    #[test]
    fn mutate_display_names_the_ends() {
        assert_eq!(mutate_display(0.0), "OFF");
        assert_eq!(mutate_display(0.5), "50%");
        assert_eq!(mutate_display(1.0), "RND");
    }

    #[test]
    fn state_roundtrip() {
        let mut s = PanelState {
            master: 0.33,
            mutate: [0.5, 1.0],
            ..Default::default()
        };
        s.voices[0][Param::Engine.index()] = 4.0;
        s.voices[1][Param::Pitch.index()] = 0.123;
        s.locks[0][Param::Pitch.index()] = true;
        s.locks[1][Param::Engine.index()] = true;
        s.locks[1][Param::ChorusMix.index()] = true;
        let back = PanelState::deserialize(&s.serialize()).unwrap();
        assert_eq!(back, s);
    }

    #[test]
    fn rejects_foreign_text() {
        assert!(PanelState::deserialize("hello").is_none());
    }

    #[test]
    fn tolerates_garbage_lines() {
        let text = format!(
            "{FORMAT_HEADER}\nmaster 0.5\n0 Nope 1\n7 Pitch 0.2\n0 Pitch abc\n1 Engine 3\n\
             lock 0 Nope\nlock 9 Pitch\nlock 0 Level\nlock 1 Decay\n"
        );
        let s = PanelState::deserialize(&text).unwrap();
        assert_eq!(s.master, 0.5);
        assert_eq!(s.voices[1][Param::Engine.index()], 3.0);
        // Performance controls can't be locked.
        assert!(!s.locks[0][Param::Level.index()]);
        assert!(s.locks[1][Param::Decay.index()]);
        assert_eq!(s.locks.iter().flatten().filter(|&&l| l).count(), 1);
    }

    /// RND (and MUTATE at RND) must always give something audible and sane,
    /// whichever engine it lands on.
    #[test]
    fn random_patches_are_playable() {
        use subkussion_dsp::voice::Voice;
        let mut rng = Noise::new(2024);
        let mut engines = [0usize; ENGINE_COUNT];
        let fs = 48_000.0;
        for i in 0..1_000 {
            let patch = random_patch(&default_patch(), &NO_LOCKS, &mut rng);
            engines[patch[Param::Engine.index()] as usize] += 1;
            let mut v = Voice::new(fs, i);
            for p in ALL_PARAMS {
                v.set_param(p, patch[p.index()]);
            }
            v.snap_params();
            v.trigger(0.9);
            let out: Vec<f32> = (0..(fs * 0.4) as usize).map(|_| v.tick()).collect();
            let peak = out.iter().fold(0.0f32, |a, &b| a.max(b.abs()));
            assert!(out.iter().all(|x| x.is_finite()), "patch {i}");
            assert!(
                peak > 0.02 && peak < 4.0,
                "patch {i}: peak {peak} {patch:?}"
            );
        }
        assert!(engines.iter().all(|&n| n > 60), "engine spread {engines:?}");
    }

    /// Both channels at RND: one press of Space gives both a brand new sound.
    #[test]
    fn space_at_rnd_rerolls_both_channels() {
        let mut state = PanelState {
            mutate: [1.0; VOICES],
            ..Default::default()
        };
        let mut rng = Noise::new(5);
        for _ in 0..20 {
            let before = state.voices;
            for (patch, (&amount, locks)) in state
                .voices
                .iter_mut()
                .zip(state.mutate.iter().zip(&state.locks))
            {
                *patch = mutate_patch(patch, amount, locks, &mut rng);
            }
            for (now, then) in state.voices.iter().zip(&before) {
                assert_ne!(now, then);
            }
            assert_ne!(state.voices[0], state.voices[1]);
        }
    }
}
