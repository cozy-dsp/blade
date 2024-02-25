use std::io::Cursor;
use std::iter::Iterator;
use std::sync::Arc;
use image::codecs::gif::GifDecoder;
use image::{AnimationDecoder, ImageFormat};
use nih_plug::params::Param;
use nih_plug::prelude::Editor;
use nih_plug_egui::{create_egui_editor, EguiState};
use nih_plug_egui::egui::{Align, Button, CentralPanel, Color32, Frame, Image, ImageSource, Label, Layout, Rounding, Sense, Style, TopBottomPanel, Window};
use nih_plug_egui::egui::ecolor::Hsva;
use nih_plug_egui::egui::epaint::Shadow;
use nih_plug_egui::widgets::{generic_ui, ParamSlider};
use nih_plug_egui::widgets::generic_ui::GenericSlider;
use stopwatch::Stopwatch;
use crate::{BLADEParams, FanSpeed};

struct EditorState {
    gif_frame: usize,
    stopwatch: Stopwatch,
    #[cfg(feature = "plus")]
    show_settings_window: bool
}

impl EditorState {
    fn new() -> Self {
        Self {
            gif_frame: 0,
            stopwatch: Stopwatch::start_new(),
            #[cfg(feature = "plus")]
            show_settings_window: false
        }
    }
}

pub(crate) fn default_state() -> Arc<EguiState> {
    EguiState::from_size(398, 520)
}

pub(crate) fn create(params: Arc<BLADEParams>, editor_state: Arc<EguiState>) -> Option<Box<dyn Editor>> {
    let image = GifDecoder::new(&include_bytes!("../assets/fan-spinning.gif")[..]).unwrap();
    let mut frames = Vec::default();
    for (idx, frame) in image.into_frames().enumerate() {
        let frame = frame.unwrap();
        let mut encoded_frame = Cursor::new(Vec::new());
        frame.buffer().write_to(&mut encoded_frame, ImageFormat::Png).unwrap();
        frames.push(ImageSource::from((format!("bytes://fan_frame_{}", idx), encoded_frame.into_inner())));
    }

    create_egui_editor(editor_state, EditorState::new(), |ctx, _| {
        egui_extras::install_image_loaders(ctx);
    }, move |ctx, setter, state| {
        let frame_time = match params.speed.value() {
            FanSpeed::Off => -1,
            FanSpeed::Fast => 14,
            FanSpeed::Medium => 30,
            FanSpeed::Slow => 60
        };

        if params.speed.value() != FanSpeed::Off && state.stopwatch.elapsed_ms() >= frame_time {
            state.stopwatch.restart();
            state.gif_frame += 1;
            state.gif_frame = state.gif_frame % (frames.len() - 1);
        }

        TopBottomPanel::bottom("info").show(ctx, |ui| {
            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                ui.add(Label::new("METALWINGS DSP, 2024"));
                #[cfg(feature = "plus")]
                {
                    state.show_settings_window = state.show_settings_window || ui.add(Button::new("SETTINGS")).clicked();
                }
            })
        });

        CentralPanel::default().frame(Frame::none()).show(ctx, |ui| {

            let image = Image::new(frames.get(state.gif_frame).unwrap().clone()).sense(Sense {
                click: true,
                drag: false,
                focusable: false
            });
            if ui.add(image).clicked() {
                setter.begin_set_parameter(&params.speed);
                setter.set_parameter(&params.speed, params.speed.value().cycle());
                setter.end_set_parameter(&params.speed);
            };

            #[cfg(feature = "plus")]
            {
                let mut style = Style::default();
                style.spacing.indent = 0.;
                style.visuals.window_shadow = Shadow::NONE;
                style.visuals.window_rounding = Rounding::ZERO;
                style.visuals.window_stroke.width = 2.0;
                style.visuals.window_stroke.color = Color32::from(Hsva::new((ctx.frame_nr() % 1000) as f32 / 1000.0, 1., 1., 1.));


                Window::new("SETTINGS").frame(Frame::menu(&style)).collapsible(false).open(&mut state.show_settings_window).show(ctx, |ui| {
                    generic_ui::create(ui, params.clone(), setter, GenericSlider);
                });
            }
        });
    })
}