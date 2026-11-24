use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use eframe::{
    egui::{self, Color32, Frame, RichText, Vec2},
    egui_wgpu, wgpu,
};

use crate::{
    gpu::{self, Renderer, Settings},
    scenes::{self, Library, Scene},
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
    pipeline: Option<wgpu::RenderPipeline>,
    preview: Option<Preview>,
    rendered: Option<(Settings, [u32; 2])>,
    drawing: Arc<AtomicBool>,
    format: usize,
    export: Option<gpu::Export>,
    shader_error: Option<String>,
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
        let (scenes, notice) = match Library::load(&directory) {
            Ok(scenes) => (scenes, None),
            Err(error) => (Library::bundled(), Some(Err(error.to_string()))),
        };
        let settings = scenes[0].defaults;
        let mut app = Self {
            gpu,
            render_state,
            directory,
            scenes,
            active: 0,
            settings,
            pipeline: None,
            preview: None,
            rendered: None,
            drawing: Arc::default(),
            format: 1,
            export: None,
            shader_error: None,
            notice,
        };
        app.compile_active();
        Ok(app)
    }

    fn compile_active(&mut self) {
        let scene = &self.scenes[self.active];
        match self.gpu.compile(&scene.source, &scene.name) {
            Ok(pipeline) => {
                self.pipeline = Some(pipeline);
                self.rendered = None;
                self.shader_error = None;
            }
            Err(error) => self.shader_error = Some(error),
        }
    }

    fn select(&mut self, index: usize) {
        if index != self.active {
            self.active = index;
            self.settings = self.scenes[index].defaults;
            self.pipeline = None;
            self.notice = None;
            self.compile_active();
        }
    }

    fn reload(&mut self) {
        let previous = &self.scenes[self.active].id;
        match Library::load(&self.directory) {
            Ok(scenes) => {
                if let Some(index) = scenes.iter().position(|scene| &scene.id == previous) {
                    self.active = index;
                } else {
                    self.active = 0;
                    self.pipeline = None;
                    self.settings = scenes[0].defaults;
                }
                self.notice = None;
                self.scenes = scenes;
                self.compile_active();
                if self.shader_error.is_none() {
                    self.notice = Some(Ok(format!("Reloaded {} scenes.", self.scenes.len())));
                }
            }
            Err(error) => self.notice = Some(Err(error.to_string())),
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
        let Some(pipeline) = &self.pipeline else {
            return;
        };
        let [width, height] = FORMATS[self.format].1;
        let scale = 960.0 / width.max(height) as f32;
        let size = [width, height].map(|n| (n as f32 * scale).round() as u32);
        if self.rendered == Some((self.settings, size)) {
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
            .draw(&mut encoder, pipeline, &preview.frame, self.settings, 1);
        self.gpu.queue.submit([encoder.finish()]);
        self.rendered = Some((self.settings, size));
        self.drawing.store(true, Ordering::Release);
        let drawing = self.drawing.clone();
        self.gpu
            .queue
            .on_submitted_work_done(move || drawing.store(false, Ordering::Release));
        ctx.request_repaint_after(Duration::from_millis(24));
    }

    fn save(&mut self, frame: &eframe::Frame) {
        let Some(pipeline) = &self.pipeline else {
            return;
        };
        let scene = &self.scenes[self.active];
        let size = FORMATS[self.format].1;
        let suggested_name = format!(
            "{}-{}-{}x{}.png",
            scene.id, self.settings.seed, size[0], size[1]
        );
        let Some(mut path) = rfd::FileDialog::new()
            .set_title("Save wallpaper")
            .set_parent(frame)
            .add_filter("PNG image", &["png"])
            .set_file_name(suggested_name)
            .save_file()
        else {
            return;
        };
        if path.extension().is_none() {
            path.set_extension("png");
        }
        match self.gpu.export(pipeline, self.settings, size, path) {
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
            ui.add_space(8.0);
            ui.label(
                RichText::new("Procedural wallpaper studio")
                    .size(13.0)
                    .color(MUTED),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button("Reload scenes")
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
        ui.label(RichText::new("Scene").size(19.0).strong());
        ui.add_space(5.0);
        let mut selection = self.active;
        for (index, scene) in self.scenes.iter().enumerate() {
            if scene_button(ui, scene, index == self.active).clicked() {
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
        let neon = self.scenes[self.active].id == "neon-passage";
        slider(
            ui,
            if neon { "Ambient light" } else { "Sun height" },
            &mut self.settings.light,
        );
        slider(
            ui,
            if neon { "Palette" } else { "Saturation" },
            &mut self.settings.color,
        );
        slider(
            ui,
            if neon { "Twist" } else { "Camera angle" },
            &mut self.settings.camera,
        );
        slider(
            ui,
            if neon { "Glow" } else { "Texture detail" },
            &mut self.settings.detail,
        );

        ui.add_space(8.0);
        ui.separator();
        ui.label(RichText::new("Export").size(19.0).strong());
        egui::ComboBox::from_id_salt("format")
            .width(width - ui.spacing().button_padding.x * 2.0)
            .selected_text(format_description(self.format))
            .show_ui(ui, |ui| {
                for (index, _) in FORMATS.iter().enumerate() {
                    ui.selectable_value(&mut self.format, index, format_description(index));
                }
            });
        let can_save =
            self.pipeline.is_some() && self.export.is_none() && self.shader_error.is_none();
        let button = egui::Button::new(RichText::new("Save PNG").color(BACKGROUND).strong())
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
        if let Some(error) = &self.shader_error {
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

    fn canvas(&self, ui: &mut egui::Ui) {
        let scene = &self.scenes[self.active];
        let [width, height] = FORMATS[self.format].1;
        ui.heading(RichText::new(&scene.name).size(29.0).strong());
        if !scene.description.is_empty() {
            ui.label(RichText::new(&scene.description).size(15.0).color(MUTED));
        }
        ui.add_space(11.0);

        let available = ui.available_size();
        let viewport_size = Vec2::new(available.x, (available.y - 28.0).max(64.0));
        let (rect, _) = ui.allocate_exact_size(viewport_size, egui::Sense::hover());
        let painter = ui.painter().with_clip_rect(rect);
        painter.rect_filled(rect, 10, Color32::from_rgb(8, 11, 12));
        let status = if self.pipeline.is_none() {
            "Preview unavailable"
        } else if self.shader_error.is_some() {
            "Last valid shader"
        } else {
            "Live preview"
        };
        if let Some(preview) = self.preview.as_ref().filter(|_| self.pipeline.is_some()) {
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
        ui.horizontal(|ui| {
            ui.label(RichText::new(status).size(12.0).color(MUTED));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(format!(
                        "{} × {}   ·   Seed {}",
                        width, height, self.settings.seed
                    ))
                    .size(12.0)
                    .color(MUTED),
                );
            });
        });
    }
}

impl eframe::App for Studio {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
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
    }
}

fn scene_button(ui: &mut egui::Ui, scene: &Scene, selected: bool) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 38.0), egui::Sense::click());
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
    let [r, g, b] = scene.badge;
    painter.circle_filled(
        egui::pos2(rect.left() + 15.0, rect.center().y),
        3.4,
        Color32::from_rgb(r, g, b),
    );
    painter.text(
        egui::pos2(rect.left() + 29.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        &scene.name,
        egui::FontId::proportional(15.0),
        if selected { MINT } else { WHITE },
    );
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::SelectableLabel,
            true,
            selected,
            &scene.name,
        )
    });
    response
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
