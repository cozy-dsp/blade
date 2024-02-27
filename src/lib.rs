#![feature(portable_simd)]

mod filter;
mod editor;

use std::f32::consts;
use std::simd::f32x2;
use nih_plug::prelude::*;
use std::sync::Arc;
use crate::filter::{Biquad, BiquadCoefficients};

const FAST_FREQ: f32 = 35.0;
const MEDIUM_FREQ: f32 = 20.0;
const SLOW_FREQ: f32 = 5.0;

const LFO_CENTER: f32 = 1_000.0;
const LFO_RANGE: f32 = 500.0;
const FILTER_RESONANCE: f32 = 2.0;

pub struct BLADE {
    params: Arc<BLADEParams>,
    sample_rate: f32,
    lfo_freq: Smoother<f32>,
    lfo_phase: f32,
    filter: Biquad<f32x2>
}

#[derive(Default, Enum, PartialEq)]
enum FanSpeed {
    #[default]
    Off,
    Fast,
    Medium,
    Slow
}

impl FanSpeed {
    pub const fn cycle(&self) -> Self {
        match self {
            Self::Off => Self::Fast,
            Self::Fast => Self::Medium,
            Self::Medium => Self::Slow,
            Self::Slow => Self::Off,
        }
    }

    pub const fn to_freq(&self) -> Result<f32, ()> {
        match self {
            Self::Fast => Ok(FAST_FREQ),
            Self::Medium => Ok(MEDIUM_FREQ),
            Self::Slow => Ok(SLOW_FREQ),
            Self::Off => Err(())
        }
    }
}

#[derive(Params)]
struct BLADEParams {
    /// The parameter's ID is used to identify the parameter in the wrappred plugin API. As long as
    /// these IDs remain constant, you can rename and reorder these fields as you wish. The
    /// parameters are exposed to the host in the same order they were defined. In this case, this
    /// gain parameter is stored as linear gain while the values are displayed in decibels.
    #[id = "speed"]
    pub speed: EnumParam<FanSpeed>,
    #[cfg(feature = "plus")]
    #[id = "lfo_center"]
    pub lfo_center: FloatParam,
    #[cfg(feature = "plus")]
    #[id = "lfo_range"]
    pub lfo_range: FloatParam,
    #[cfg(feature = "plus")]
    #[id = "filter_resonance"]
    pub filter_resonance: FloatParam
}

impl Default for BLADE {
    fn default() -> Self {
        Self {
            params: Arc::new(BLADEParams::default()),
            lfo_freq: Smoother::new(SmoothingStyle::Linear(20.0)),
            lfo_phase: 0.,
            sample_rate: 0.,
            filter: Biquad::default()
        }
    }
}

#[cfg(not(feature = "plus"))]
impl Default for BLADEParams {
    fn default() -> Self {
        Self {
            speed: EnumParam::new("Speed", FanSpeed::default())
        }
    }
}

#[cfg(feature = "plus")]
impl Default for BLADEParams {
    fn default() -> Self {
        Self {
            speed: EnumParam::new("Speed", FanSpeed::default()).hide_in_generic_ui(),
            lfo_center: FloatParam::new("LFO Center", LFO_CENTER, FloatRange::Linear {
                min: 500.0,
                max: 5_000.0
            }).with_unit(" Hz").with_step_size(0.01),
            lfo_range: FloatParam::new("LFO Range", LFO_RANGE, FloatRange::Linear {
                min: 100.0,
                max: 2_000.0
            }).with_unit(" Hz").with_step_size(0.01),
            filter_resonance: FloatParam::new("Resonance", FILTER_RESONANCE, FloatRange::Linear {
                min: 1.0,
                max: 4.0
            }).with_step_size(0.01)
        }
    }
}

impl BLADE {
    fn calculate_lfo(&mut self, frequency: f32) -> f32 {
        let phase_delta = frequency / self.sample_rate;
        let sine = (self.lfo_phase * consts::TAU).sin();

        self.lfo_phase += phase_delta;
        if self.lfo_phase >= 1.0 {
            self.lfo_phase -= 1.0;
        }

        sine
    }

    #[cfg(feature = "plus")]
    #[inline]
    fn get_param_values(&self) -> (f32, f32, f32) {
        (self.params.lfo_center.value(), self.params.lfo_range.value(), self.params.filter_resonance.value())
    }

    #[cfg(not(feature = "plus"))]
    #[inline]
    const fn get_param_values(&self) -> (f32, f32, f32) {
        (LFO_CENTER, LFO_RANGE, FILTER_RESONANCE)
    }
}

impl Plugin for BLADE {
    const NAME: &'static str = "BLADE";
    const VENDOR: &'static str = "METALWINGS DSP";
    const URL: &'static str = env!("CARGO_PKG_HOMEPAGE");
    const EMAIL: &'static str = "metalwings@draconium.productions";

    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: NonZeroU32::new(2),
        main_output_channels: NonZeroU32::new(2),

        ..AudioIOLayout::const_default()
    }];


    const MIDI_INPUT: MidiConfig = MidiConfig::None;
    const MIDI_OUTPUT: MidiConfig = MidiConfig::None;

    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    type SysExMessage = ();

    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        let params = self.params.clone();
        editor::create(params, editor::default_state())
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        self.sample_rate = buffer_config.sample_rate;
        #[cfg(not(feature = "plus"))]
        {
            self.filter.coefficients = BiquadCoefficients::bandpass(buffer_config.sample_rate, 1_000., 2.);
        }
        #[cfg(feature = "plus")]
        {
            self.filter.coefficients = BiquadCoefficients::bandpass(buffer_config.sample_rate, self.params.lfo_center.value(), self.params.filter_resonance.value());
        }
        true
    }

    fn reset(&mut self) {
        if let Ok(freq) = self.params.speed.value().to_freq() {
            self.lfo_freq.reset(freq);
        }
        self.lfo_phase = 0.0;
        self.filter.reset();
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let speed = self.params.speed.value();
        match speed {
            FanSpeed::Off => {}
            _ => {
                self.lfo_freq.set_target(self.sample_rate, speed.to_freq().expect("FanSpeed is somehow off and yet we reached this branch, what the fuck?"))
            }
        }

        // done this way to make refactoring out plus logic easy
        let (center, range, resonance) = self.get_param_values();

        for mut channel_samples in buffer.iter_samples() {
            // even if the fan is off, calculate the lfo phase
            let lfo_val = self.calculate_lfo(self.lfo_freq.next());

            if speed != FanSpeed::Off {
                self.filter.coefficients = BiquadCoefficients::bandpass(self.sample_rate, range.mul_add(lfo_val, center), resonance);
                // SAFETY: we're only ever working with 2 channels.
                let samples = unsafe { channel_samples.to_simd_unchecked() };
                let filtered = self.filter.process(samples);
                unsafe { channel_samples.from_simd_unchecked(filtered) };
            }
        }

        ProcessStatus::Normal
    }
}

impl ClapPlugin for BLADE {
    const CLAP_ID: &'static str = "dsp.metalwings.blade";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("An innovative filter that works on everything");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;

    const CLAP_FEATURES: &'static [ClapFeature] = &[ClapFeature::AudioEffect, ClapFeature::Stereo, ClapFeature::Filter];
}

impl Vst3Plugin for BLADE {
    const VST3_CLASS_ID: [u8; 16] = *b"dmetalwingsblade";

    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Filter];
}

nih_export_clap!(BLADE);
nih_export_vst3!(BLADE);
