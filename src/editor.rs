use std::io::Cursor;
use std::sync::Arc;
use image::codecs::gif::GifDecoder;
use image::{AnimationDecoder, ImageFormat};
use nih_plug::prelude::Editor;
use nih_plug_egui::{create_egui_editor, EguiState};
use nih_plug_egui::egui::{include_image, Align, CentralPanel, Frame, Image, ImageSource, Layout, RichText, Sense, TopBottomPanel};

use stopwatch::Stopwatch;
use crate::{BLADEParams, FanSpeed, VERSION};

#[cfg(feature = "plus")]
use nih_plug_egui::widgets::generic_ui;
#[cfg(feature = "plus")]
use nih_plug_egui::widgets::generic_ui::GenericSlider;
use nih_plug_egui::egui::{Button, Color32, Rounding, Style, Window, ecolor::Hsva, epaint::Shadow};

const RAINBOW_SPEED: u64 = 100;

struct EditorState {
    gif_frame: usize,
    stopwatch: Stopwatch,
    show_credits_window: bool,
    #[cfg(feature = "plus")]
    show_settings_window: bool
}

impl EditorState {
    fn new() -> Self {
        Self {
            gif_frame: 0,
            stopwatch: Stopwatch::start_new(),
            show_credits_window: false,
            #[cfg(feature = "plus")]
            show_settings_window: false
        }
    }
}

pub fn default_state() -> Arc<EguiState> {
    EguiState::from_size(398, 520)
}

pub fn create(params: Arc<BLADEParams>, editor_state: Arc<EguiState>) -> Option<Box<dyn Editor>> {
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
            state.gif_frame %= frames.len() - 1;
        }

        TopBottomPanel::bottom("info").show(ctx, |ui| {
            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                state.show_credits_window = state.show_credits_window || ui.add(Button::new("CREDITS")).clicked();
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

            let mut style = Style::default();
            let rainbow = Color32::from(Hsva::new((ctx.frame_nr() % RAINBOW_SPEED) as f32 / RAINBOW_SPEED as f32, 1., 1., 1.));
            style.spacing.indent = 0.;
            style.visuals.window_shadow = Shadow::NONE;
            style.visuals.window_rounding = Rounding::ZERO;
            style.visuals.window_stroke.width = 2.0;
            style.visuals.window_stroke.color = rainbow;

            Window::new("CREDITS").frame(Frame::popup(&style)).resizable(false).vscroll(true).collapsible(false).open(&mut state.show_credits_window).show(ctx, |ui| {
                ui.image(include_image!("../assets/Cozy_logo.png"));
                    ui.vertical_centered(|ui| {
                        ui.heading(RichText::new("BLADE").strong().color(rainbow));
                        ui.label(
                            RichText::new(format!("Version {}", VERSION))
                                .italics(),
                        );
                        ui.hyperlink_to("Homepage", env!("CARGO_PKG_HOMEPAGE"));
                        ui.separator();
                        ui.heading(RichText::new("Credits"));
                        ui.label("Original concept by axo1otl");
                        ui.label("Plugin by joe sorensen");
                        ui.label("cozy dsp branding and design by gordo");
                        ui.label("licensed under GPLv3 (thanks steinberg!)");
                    });
            });

            #[cfg(feature = "plus")]
            {
                Window::new("SETTINGS").frame(Frame::menu(&style)).collapsible(false).open(&mut state.show_settings_window).show(ctx, |ui| {
                    generic_ui::create(ui, params.clone(), setter, GenericSlider);
                });
            }
        });
    })
}