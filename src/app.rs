use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use eframe::{
    egui::{self, Color32, Frame, RichText, Vec2},
    egui_wgpu, wgpu,
};

use crate::{
    gpu::{self, Effects, RenderInput, Renderer, Settings},
    scenes::{self, Library, Scene},
    story::Timeline,
};

const BACKGROUND: Color32 = Color32::from_rgb(13, 17, 18);
const PANEL: Color32 = Color32::from_rgb(21, 26, 27);
const EDGE: Color32 = Color32::from_rgb(43, 53, 54);
const WHITE: Color32 = Color32::from_rgb(234, 241, 237);
const MUTED: Color32 = Color32::from_rgb(154, 170, 166);
const MINT: Color32 = Color32::from_rgb(178, 242, 209);

const FORMATS: [(&str, [u32; 2]); 5] = [
    ("Full HD", [1920, 1080]),
    ("QHD", [2560, 1440]),
    ("4K UHD", [3840, 2160]),
    ("Ultrawide", [3440, 1440]),
    ("Portrait", [1440, 2560]),
];

const EFFECT_OPTIONS: [(u32, &str); 5] = [
    (Effects::CLOUDS, "Clouds"),
    (Effects::REFLECTIONS, "Reflections"),
    (Effects::PARTICLES, "Particles"),
    (Effects::SHADOWS, "Cast shadows"),
    (Effects::GRAIN, "Film grain"),
];

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum Mode {
    #[default]
    Studio,
    Story,
}

struct Playback {
    seconds: f64,
    rate: f64,
    playing: bool,
    checked: Instant,
}

impl Default for Playback {
    fn default() -> Self {
        Self {
            seconds: 0.0,
            rate: 1.0,
            playing: false,
            checked: Instant::now(),
        }
    }
}

impl Playback {
    fn tick(&mut self, active: bool, end: f64) {
        let now = Instant::now();
        if self.playing && active {
            self.seconds = (self.seconds + (now - self.checked).as_secs_f64() * self.rate).min(end);
            if self.seconds >= end {
                self.playing = false;
            }
        }
        self.checked = now;
    }

    fn seek(&mut self, seconds: f64) {
        self.seconds = seconds;
        self.checked = Instant::now();
    }

    fn toggle(&mut self, end: f64) {
        if !self.playing && self.seconds >= end {
            self.seek(0.0);
        }
        self.checked = Instant::now();
        self.playing = !self.playing;
    }
}

struct Shader {
    pipeline: Option<wgpu::RenderPipeline>,
    error: Option<String>,
}

struct Preview {
    frame: gpu::Frame,
    id: egui::TextureId,
}

pub struct Studio {
    gpu: Renderer,
    render_state: egui_wgpu::RenderState,
    directory: PathBuf,
    scenes: Vec<Scene>,
    active: usize,
    settings: Settings,
    shaders: HashMap<usize, Shader>,
    effects: Effects,
    mode: Mode,
    motion: Playback,
    timeline: Timeline,
    story_playback: Playback,
    captions: bool,
    preview: Option<Preview>,
    rendered: Option<(usize, RenderInput, [u32; 2])>,
    drawing: Arc<AtomicBool>,
    format: usize,
    export: Option<gpu::Export>,
    notice: Option<Result<String, String>>,
}

impl Studio {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let render_state = cc
            .wgpu_render_state
            .clone()
            .ok_or("Wildframe requires wgpu.")?;
        let gpu = Renderer::new(render_state.device.clone(), render_state.queue.clone());
        let mut style = (*cc.egui_ctx.style()).clone();
        style.visuals = egui::Visuals::dark();
        style.visuals.override_text_color = Some(WHITE);
        style.visuals.panel_fill = BACKGROUND;
        style.visuals.window_fill = PANEL;
        style.visuals.selection.bg_fill = Color32::from_rgb(50, 83, 73);
        style.visuals.selection.stroke.color = MINT;
        style.visuals.widgets.inactive.bg_fill = Color32::from_rgb(34, 42, 43);
        style.visuals.widgets.hovered.bg_fill = Color32::from_rgb(47, 59, 58);
        style.visuals.widgets.active.bg_fill = Color32::from_rgb(55, 78, 68);
        style.visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, EDGE);
        style.visuals.widgets.hovered.bg_stroke.color = MINT;
        style.visuals.widgets.inactive.corner_radius = 5.into();
        style.visuals.widgets.hovered.corner_radius = 5.into();
        style.visuals.widgets.active.corner_radius = 5.into();
        style.spacing.item_spacing = Vec2::new(9.0, 8.0);
        style.spacing.interact_size.y = 30.0;
        style.spacing.slider_width = 91.0;
        style.spacing.slider_rail_height = 4.0;
        style
            .text_styles
            .get_mut(&egui::TextStyle::Body)
            .unwrap()
            .size = 15.0;
        cc.egui_ctx.set_style(style);

        let directory = scenes::directory();
        let (scenes, mut notice) = match Library::load(&directory) {
            Ok(scenes) => (scenes, None),
            Err(error) => (Library::bundled(), Some(Err(error.to_string()))),
        };
        let timeline = match Timeline::load(&directory, &scenes) {
            Ok(timeline) => timeline,
            Err(error) => {
                notice = Some(Err(error));
                Timeline::bundled()
            }
        };
        let settings = scenes[0].defaults;
        let mut app = Self {
            gpu,
            render_state,
            directory,
            scenes,
            active: 0,
            settings,
            shaders: HashMap::new(),
            effects: Effects::default(),
            mode: Mode::default(),
            motion: Playback::default(),
            timeline,
            story_playback: Playback::default(),
            captions: true,
            preview: None,
            rendered: None,
            drawing: Arc::default(),
            format: 1,
            export: None,
            notice,
        };
        app.prepare(0);
        Ok(app)
    }

    fn prepare(&mut self, index: usize) {
        self.shaders.entry(index).or_insert_with(|| {
            let scene = &self.scenes[index];
            match self.gpu.compile(&scene.source, &scene.name) {
                Ok(pipeline) => Shader {
                    pipeline: Some(pipeline),
                    error: None,
                },
                Err(error) => Shader {
                    pipeline: None,
                    error: Some(error),
                },
            }
        });
    }

    fn input(&self) -> (usize, RenderInput) {
        match self.mode {
            Mode::Studio => (
                self.active,
                RenderInput {
                    time: self.motion.seconds as f32,
                    effects: self.effects,
                    ..RenderInput::still(self.settings)
                },
            ),
            Mode::Story => self.timeline.frame(
                self.story_playback.seconds as f32,
                &self.scenes,
                self.effects,
            ),
        }
    }

    fn select(&mut self, index: usize) {
        if index != self.active {
            self.active = index;
            self.settings = self.scenes[index].defaults;
            self.motion.seek(0.0);
            self.notice = None;
        }
    }

    fn reload(&mut self) {
        let scenes = match Library::load(&self.directory) {
            Ok(scenes) => scenes,
            Err(error) => {
                self.notice = Some(Err(error.to_string()));
                return;
            }
        };
        let previous_visible = self.input().0;
        let previous_id = self.scenes[previous_visible].id.clone();
        let fallback = self
            .shaders
            .remove(&previous_visible)
            .and_then(|shader| shader.pipeline);
        let id = &self.scenes[self.active].id;
        if let Some(index) = scenes.iter().position(|scene| &scene.id == id) {
            self.active = index;
        } else {
            self.active = 0;
            self.settings = scenes[0].defaults;
        }
        self.notice = Some(match Timeline::load(&self.directory, &scenes) {
            Ok(timeline) => {
                self.timeline = timeline;
                Ok(format!("Reloaded {} scenes and the story.", scenes.len()))
            }
            Err(error) => {
                if self.timeline.validate(&scenes).is_err() {
                    self.timeline = Timeline::bundled();
                }
                Err(error)
            }
        });
        self.story_playback.seek(
            self.story_playback
                .seconds
                .min(f64::from(self.timeline.duration())),
        );
        self.scenes = scenes;
        self.shaders.clear();
        self.rendered = None;
        let index = self.input().0;
        self.prepare(index);
        if self.scenes[index].id == previous_id
            && let Some(shader) = self.shaders.get_mut(&index)
            && shader.error.is_some()
        {
            shader.pipeline = fallback;
        }
    }

    fn shuffle(&mut self) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;
        let mut n = (nanos ^ (nanos >> 32)) as u32 ^ self.settings.seed;
        n = (n ^ (n >> 16)).wrapping_mul(0x7feb352d);
        n = (n ^ (n >> 15)).wrapping_mul(0x846ca68b);
        self.settings.seed = n ^ (n >> 16);
    }

    fn update_preview(&mut self, ctx: &egui::Context) {
        if self.drawing.load(Ordering::Acquire) {
            self.gpu.device.poll(wgpu::Maintain::Poll);
            if self.drawing.load(Ordering::Acquire) {
                ctx.request_repaint_after(Duration::from_millis(24));
                return;
            }
        }
        let (index, input) = self.input();
        self.prepare(index);
        let Some(pipeline) = &self.shaders[&index].pipeline else {
            return;
        };
        if self.export.is_some() {
            return;
        }
        let [width, height] = FORMATS[self.format].1;
        let playing = match self.mode {
            Mode::Studio => self.motion.playing,
            Mode::Story => self.story_playback.playing,
        };
        let long_edge = if playing { 720.0 } else { 960.0 };
        let scale = long_edge / width.max(height) as f32;
        let size = [width, height].map(|n| (n as f32 * scale).round() as u32);
        if self.rendered == Some((index, input, size)) {
            return;
        }

        if self
            .preview
            .as_ref()
            .is_none_or(|preview| preview.frame.size != size)
        {
            let frame = self
                .gpu
                .frame(size)
                .expect("Preview size is within the default device limits");
            let display_view = frame.display_view();
            let mut renderer = self.render_state.renderer.write();
            let id = match &self.preview {
                Some(old) => {
                    renderer.update_egui_texture_from_wgpu_texture(
                        &self.gpu.device,
                        &display_view,
                        wgpu::FilterMode::Linear,
                        old.id,
                    );
                    old.id
                }
                None => renderer.register_native_texture(
                    &self.gpu.device,
                    &display_view,
                    wgpu::FilterMode::Linear,
                ),
            };
            self.preview = Some(Preview { frame, id });
        }

        let Some(preview) = &self.preview else { return };
        let mut encoder = self.gpu.device.create_command_encoder(&Default::default());
        self.gpu
            .draw(&mut encoder, pipeline, &preview.frame, input, 1);
        self.gpu.queue.submit([encoder.finish()]);
        self.rendered = Some((index, input, size));
        self.drawing.store(true, Ordering::Release);
        let drawing = self.drawing.clone();
        self.gpu
            .queue
            .on_submitted_work_done(move || drawing.store(false, Ordering::Release));
        ctx.request_repaint_after(Duration::from_millis(24));
    }

    fn save(&mut self, frame: &eframe::Frame) {
        let (index, input) = self.input();
        let Some(pipeline) = self
            .shaders
            .get(&index)
            .and_then(|shader| shader.pipeline.as_ref())
        else {
            return;
        };
        let scene = &self.scenes[index];
        let size = FORMATS[self.format].1;
        let time = match self.mode {
            Mode::Story => format!("-story-{:07.2}s", self.story_playback.seconds),
            Mode::Studio if input.time > 0.0 => format!("-{:07.2}s", input.time),
            Mode::Studio => String::new(),
        };
        let suggested_name = format!(
            "{}-{}{time}-{}x{}.png",
            scene.id, input.settings.seed, size[0], size[1]
        );
        let chosen = rfd::FileDialog::new()
            .set_title("Save wallpaper")
            .set_parent(frame)
            .add_filter("PNG image", &["png"])
            .set_file_name(suggested_name)
            .save_file();
        self.motion.checked = Instant::now();
        self.story_playback.checked = self.motion.checked;
        let Some(mut path) = chosen else { return };
        self.motion.playing = false;
        self.story_playback.playing = false;
        if path.extension().is_none() {
            path.set_extension("png");
        }
        match self.gpu.export(pipeline, input, size, path) {
            Ok(export) => {
                self.notice = None;
                self.export = Some(export);
            }
            Err(error) => self.notice = Some(Err(error)),
        }
    }

    fn top_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_centered(|ui| {
            let (box_rect, _) = ui.allocate_exact_size(Vec2::splat(34.0), egui::Sense::hover());
            let painter = ui.painter();
            painter.rect_filled(box_rect, 9, MINT);
            let pos = |x, y| box_rect.min + Vec2::new(x, y);
            let ink = egui::Stroke::new(1.7, Color32::from_rgb(22, 49, 40));
            painter.line_segment([pos(6.0, 24.0), pos(15.0, 11.0)], ink);
            painter.line_segment([pos(15.0, 11.0), pos(21.0, 20.0)], ink);
            painter.line_segment([pos(18.0, 16.0), pos(22.0, 12.0)], ink);
            painter.line_segment([pos(22.0, 12.0), pos(28.0, 24.0)], ink);
            painter.line_segment([pos(6.0, 24.0), pos(28.0, 24.0)], ink);
            ui.label(RichText::new("Wildframe").size(23.0).strong());
            ui.add_space(17.0);
            ui.selectable_value(&mut self.mode, Mode::Studio, "Studio");
            ui.selectable_value(&mut self.mode, Mode::Story, "Story");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button("Reload files")
                    .on_hover_text_at_pointer(format!("F5 · {}", self.directory.display()))
                    .clicked()
                {
                    self.reload();
                }
            });
        });
    }

    fn controls(&mut self, ui: &mut egui::Ui, frame: &eframe::Frame) {
        let width = ui.available_width();
        ui.set_width(width);
        match self.mode {
            Mode::Studio => self.scene_controls(ui),
            Mode::Story => self.chapter_controls(ui),
        }
        ui.separator();
        egui::CollapsingHeader::new("Render effects")
            .id_salt("effect switches")
            .show(ui, |ui| {
                for (bit, name) in EFFECT_OPTIONS {
                    let mut enabled = self.effects.has(bit);
                    if ui.checkbox(&mut enabled, name).changed() {
                        self.effects.set(bit, enabled);
                    }
                }
            });
        ui.separator();
        self.export_controls(ui, frame, width);
    }

    fn scene_controls(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("Scene").size(19.0).strong());
        let mut selection = self.active;
        for (index, scene) in self.scenes.iter().enumerate() {
            if scene_button(ui, &scene.name, scene.badge, index == self.active, None).clicked() {
                selection = index;
            }
        }
        self.select(selection);

        ui.add_space(8.0);
        ui.separator();
        ui.horizontal(|ui| {
            ui.label(RichText::new("Variation").size(19.0).strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("Reset").clicked() {
                    self.settings = self.scenes[self.active].defaults;
                    self.motion.seek(0.0);
                    self.motion.playing = false;
                }
            });
        });
        ui.horizontal(|ui| {
            ui.label("Seed");
            ui.add_sized(
                [103.0, 29.0],
                egui::DragValue::new(&mut self.settings.seed).range(0..=u32::MAX),
            );
            if ui.button("Shuffle").clicked() {
                self.shuffle();
            }
        });
        ui.add_space(4.0);
        let id = self.scenes[self.active].id.as_str();
        let labels = match id {
            "neon-passage" => ["Ambient light", "Palette", "Twist", "Glow"],
            "glass-reef" => ["Sun shafts", "Saturation", "Camera drift", "Coral growth"],
            "lantern-observatory" => ["Dawn light", "Saturation", "Camera angle", "Beam focus"],
            _ => ["Sun height", "Saturation", "Camera angle", "Texture detail"],
        };
        slider(ui, labels[0], &mut self.settings.light);
        slider(ui, labels[1], &mut self.settings.color);
        slider(ui, labels[2], &mut self.settings.camera);
        slider(ui, labels[3], &mut self.settings.detail);
    }

    fn chapter_controls(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("Chapters").size(19.0).strong());
        let current = self
            .timeline
            .moment(self.story_playback.seconds as f32)
            .chapter_index;
        let mut start = 0.0;
        for (index, chapter) in self.timeline.chapters.iter().enumerate() {
            let color = self
                .scenes
                .iter()
                .find(|scene| scene.id == chapter.scene)
                .map_or([155, 189, 181], |scene| scene.badge);
            let time = timestamp(start);
            if scene_button(ui, &chapter.title, color, index == current, Some(&time)).clicked() {
                self.story_playback.seek(start);
                self.notice = None;
            }
            start += f64::from(chapter.seconds);
        }
        ui.checkbox(&mut self.captions, "Show story captions");
    }

    fn export_controls(&mut self, ui: &mut egui::Ui, frame: &eframe::Frame, width: f32) {
        ui.label(RichText::new("Export").size(19.0).strong());
        egui::ComboBox::from_id_salt("format")
            .width(width - ui.spacing().button_padding.x * 2.0)
            .selected_text(format_description(self.format))
            .show_ui(ui, |ui| {
                for (index, _) in FORMATS.iter().enumerate() {
                    ui.selectable_value(&mut self.format, index, format_description(index));
                }
            });
        let index = self.input().0;
        let can_save = self.export.is_none()
            && self
                .shaders
                .get(&index)
                .is_some_and(|shader| shader.pipeline.is_some() && shader.error.is_none());
        let button = egui::Button::new(
            RichText::new("Save frame as PNG")
                .color(BACKGROUND)
                .strong(),
        )
        .fill(MINT)
        .stroke(egui::Stroke::NONE)
        .min_size(Vec2::new(width, 38.0));
        ui.vertical_centered_justified(|ui| {
            if ui.add_enabled(can_save, button).clicked() {
                self.save(frame);
            }
        });
        if self.export.is_some() {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Rendering PNG…");
            });
        }
        if let Some(notice) = &self.notice {
            let (message, color) = match notice {
                Ok(message) => (message, MINT),
                Err(message) => (message, Color32::LIGHT_RED),
            };
            ui.label(RichText::new(message).size(12.0).color(color));
        }
        if let Some(error) = self
            .shaders
            .get(&index)
            .and_then(|shader| shader.error.as_ref())
        {
            ui.label(
                RichText::new("Shader error · edit the scene and reload")
                    .size(12.0)
                    .color(Color32::LIGHT_RED),
            );
            egui::CollapsingHeader::new("Details").show(ui, |ui| {
                ui.monospace(error);
            });
        }
    }

    fn canvas(&mut self, ui: &mut egui::Ui) {
        let (index, input) = self.input();
        let scene = &self.scenes[index];
        let [width, height] = FORMATS[self.format].1;
        match self.mode {
            Mode::Studio => {
                ui.heading(RichText::new(&scene.name).size(29.0).strong());
                if !scene.description.is_empty() {
                    ui.label(RichText::new(&scene.description).size(15.0).color(MUTED));
                }
            }
            Mode::Story => {
                ui.heading(RichText::new(&self.timeline.title).size(29.0).strong());
                ui.label(
                    RichText::new(&self.timeline.premise)
                        .size(15.0)
                        .color(MUTED),
                );
            }
        }
        ui.add_space(11.0);

        let available = ui.available_size();
        let footer = if self.mode == Mode::Story {
            153.0
        } else {
            53.0
        };
        let viewport_size = Vec2::new(available.x, (available.y - footer).max(64.0));
        let (rect, _) = ui.allocate_exact_size(viewport_size, egui::Sense::hover());
        let painter = ui.painter().with_clip_rect(rect);
        painter.rect_filled(rect, 10, Color32::from_rgb(8, 11, 12));
        let shader = self.shaders.get(&index);
        let has_pipeline = shader.is_some_and(|shader| shader.pipeline.is_some());
        let status = if !has_pipeline {
            "Preview unavailable"
        } else if shader.is_some_and(|shader| shader.error.is_some()) {
            "Last valid shader"
        } else {
            "Live preview"
        };
        if let Some(preview) = self
            .preview
            .as_ref()
            .filter(|_| has_pipeline && self.rendered.is_some_and(|(shown, _, _)| shown == index))
        {
            let aspect = preview.frame.size[0] as f32 / preview.frame.size[1] as f32;
            let bounds = (rect.size() - Vec2::splat(16.0)).max(Vec2::splat(1.0));
            let w = bounds.x.min(bounds.y * aspect);
            let image_rect = egui::Rect::from_center_size(rect.center(), Vec2::new(w, w / aspect));
            painter.image(
                preview.id,
                image_rect,
                egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                Color32::WHITE,
            );
        }
        painter.rect_stroke(
            rect,
            10,
            egui::Stroke::new(1.0, EDGE),
            egui::StrokeKind::Inside,
        );
        if self.mode == Mode::Story {
            self.story_transport(ui);
        } else {
            ui.horizontal(|ui| {
                let clock = &mut self.motion;
                if ui
                    .button(if clock.playing {
                        "Pause motion"
                    } else {
                        "Play motion"
                    })
                    .clicked()
                {
                    clock.toggle(86400.0);
                }
                let mut seconds = clock.seconds;
                if ui
                    .add(
                        egui::DragValue::new(&mut seconds)
                            .range(0.0..=86400.0)
                            .speed(0.12)
                            .fixed_decimals(2)
                            .suffix(" s"),
                    )
                    .on_hover_text("Set the scene time")
                    .changed()
                {
                    clock.seek(seconds);
                }
                if ui.small_button("Rewind").clicked() {
                    clock.seek(0.0);
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new(format!(
                            "{status}  ·  {width} × {height}  ·  Seed {}",
                            input.settings.seed
                        ))
                        .size(12.0)
                        .color(MUTED),
                    );
                });
            });
        }
    }

    fn story_transport(&mut self, ui: &mut egui::Ui) {
        let end = f64::from(self.timeline.duration());
        let moment = self.timeline.moment(self.story_playback.seconds as f32);
        ui.horizontal(|ui| {
            ui.label(RichText::new(&moment.chapter.title).strong().color(MINT));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(format!(
                        "{} / {}",
                        timestamp(self.story_playback.seconds),
                        timestamp(end)
                    ))
                    .monospace()
                    .color(MUTED),
                );
            });
        });
        let caption = if self.captions {
            moment.caption.unwrap_or("")
        } else {
            ""
        };
        ui.allocate_ui(Vec2::new(ui.available_width(), 20.0), |ui| {
            ui.label(RichText::new(caption).size(14.0));
        });
        let clock = &mut self.story_playback;
        let mut time = clock.seconds;
        let progress_width = ui.available_width();
        ui.spacing_mut().slider_width = progress_width;
        if ui
            .add(egui::Slider::new(&mut time, 0.0..=end).show_value(false))
            .on_hover_text("Seek through the story")
            .changed()
        {
            clock.seek(time);
        }
        ui.horizontal(|ui| {
            if ui
                .button(if clock.playing { "Pause" } else { "Play story" })
                .clicked()
            {
                clock.toggle(end);
            }
            if ui.button("Restart").clicked() {
                clock.seek(0.0);
            }
            egui::ComboBox::from_id_salt("story speed")
                .width(49.0)
                .selected_text(format!("{}×", clock.rate))
                .show_ui(ui, |ui| {
                    for rate in [1.0, 2.0, 4.0, 12.0, 60.0] {
                        ui.selectable_value(&mut clock.rate, rate, format!("{rate}×"));
                    }
                });
        });
    }
}

impl eframe::App for Studio {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        let story_end = f64::from(self.timeline.duration());
        self.motion.tick(self.mode == Mode::Studio, 86400.0);
        self.story_playback
            .tick(self.mode == Mode::Story, story_end);
        if ctx.input(|input| input.key_pressed(egui::Key::F5)) {
            self.reload();
        }
        if let Some(export) = &self.export {
            if let Some(result) = export.check() {
                let message = format!(
                    "Saved {}",
                    export
                        .path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                );
                self.notice = Some(result.map(|()| message));
                self.export = None;
            } else {
                ctx.request_repaint_after(Duration::from_millis(100));
            }
        }

        egui::TopBottomPanel::top("header")
            .frame(
                Frame::NONE
                    .fill(PANEL)
                    .inner_margin(egui::Margin::symmetric(20, 12)),
            )
            .show(ctx, |ui| self.top_bar(ui));
        egui::SidePanel::right("inspector")
            .exact_width(303.0)
            .resizable(false)
            .frame(
                Frame::NONE
                    .fill(PANEL)
                    .inner_margin(egui::Margin::symmetric(17, 16)),
            )
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| self.controls(ui, frame));
            });
        self.update_preview(ctx);
        egui::CentralPanel::default()
            .frame(
                Frame::NONE
                    .fill(BACKGROUND)
                    .inner_margin(egui::Margin::symmetric(19, 15)),
            )
            .show(ctx, |ui| self.canvas(ui));
        let playing = match self.mode {
            Mode::Studio => self.motion.playing,
            Mode::Story => self.story_playback.playing,
        };
        if playing {
            ctx.request_repaint_after(Duration::from_millis(35));
        }
    }
}

fn scene_button(
    ui: &mut egui::Ui,
    title: &str,
    badge: [u8; 3],
    selected: bool,
    time: Option<&str>,
) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 34.0), egui::Sense::click());
    let painter = ui.painter();
    let fill = if selected {
        Color32::from_rgb(34, 52, 49)
    } else if response.hovered() {
        Color32::from_rgb(32, 40, 41)
    } else {
        PANEL
    };
    let outline = if selected {
        Color32::from_rgb(70, 104, 94)
    } else {
        EDGE
    };
    painter.rect_filled(rect, 7, fill);
    painter.rect_stroke(
        rect,
        7,
        egui::Stroke::new(1.0, outline),
        egui::StrokeKind::Inside,
    );
    let [r, g, b] = badge;
    painter.circle_filled(
        egui::pos2(rect.left() + 15.0, rect.center().y),
        3.4,
        Color32::from_rgb(r, g, b),
    );
    let title_rect = rect.with_max_x(rect.right() - if time.is_some() { 62.0 } else { 8.0 });
    painter.with_clip_rect(title_rect).text(
        egui::pos2(rect.left() + 29.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        title,
        egui::FontId::proportional(15.0),
        if selected { MINT } else { WHITE },
    );
    if let Some(time) = time {
        painter.text(
            egui::pos2(rect.right() - 11.0, rect.center().y),
            egui::Align2::RIGHT_CENTER,
            time,
            egui::FontId::monospace(12.0),
            MUTED,
        );
    }
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, selected, title)
    });
    response
}

fn timestamp(seconds: f64) -> String {
    let seconds = seconds as u32;
    format!("{:02}:{:02}", seconds / 60, seconds % 60)
}

fn slider(ui: &mut egui::Ui, name: &str, value: &mut f32) {
    ui.horizontal(|ui| {
        ui.add_sized(
            [106.0, 24.0],
            egui::Label::new(name).halign(egui::Align::Min),
        );
        let mut percent = *value * 100.0;
        if ui
            .add(
                egui::Slider::new(&mut percent, 0.0..=100.0)
                    .integer()
                    .suffix("%"),
            )
            .changed()
        {
            *value = percent / 100.0;
        }
    });
}

fn format_description(index: usize) -> String {
    let (name, [width, height]) = FORMATS[index];
    format!("{name}  ·  {width} × {height}")
}
