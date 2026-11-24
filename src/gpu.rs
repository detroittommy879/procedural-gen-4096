use std::{
    io::{BufWriter, Write},
    num::NonZeroU64,
    path::{Path, PathBuf},
    sync::mpsc,
    thread,
};

use bytemuck::{Pod, Zeroable};
use eframe::wgpu::{self as w, util::DeviceExt as _};
use image::ImageEncoder as _;

const FORMAT: w::TextureFormat = w::TextureFormat::Rgba8Unorm;

#[derive(Clone, Copy, PartialEq)]
pub struct Settings {
    pub seed: u32,
    pub light: f32,
    pub color: f32,
    pub camera: f32,
    pub detail: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Effects(u32);

impl Effects {
    pub const CLOUDS: u32 = 1 << 0;
    pub const REFLECTIONS: u32 = 1 << 1;
    pub const PARTICLES: u32 = 1 << 2;
    pub const SHADOWS: u32 = 1 << 3;
    pub const GRAIN: u32 = 1 << 4;

    pub const CAMERA: u32 = 1 << 5;
    pub const ENSEMBLE: u32 = 1 << 6;

    pub fn has(self, bit: u32) -> bool {
        self.0 & bit != 0
    }

    pub fn set(&mut self, bit: u32, enabled: bool) {
        self.0 = if enabled { self.0 | bit } else { self.0 & !bit };
    }
}

impl Default for Effects {
    fn default() -> Self {
        Self(
            Self::CLOUDS
                | Self::REFLECTIONS
                | Self::PARTICLES
                | Self::SHADOWS
                | Self::GRAIN
                | Self::CAMERA
                | Self::ENSEMBLE,
        )
    }
}

#[derive(Clone, Copy, PartialEq)]
pub struct Shot {
    pub action: u32,
    pub progress: f32,
    pub transition: f32,
    pub camera_frame: [f32; 3],
}

#[derive(Clone, Copy, PartialEq)]
pub struct RenderInput {
    pub settings: Settings,
    pub time: f32,
    pub effects: Effects,
    pub shot: Shot,
}

impl RenderInput {
    pub fn still(settings: Settings) -> Self {
        Self {
            settings,
            time: 0.0,
            effects: Effects::default(),
            shot: Shot {
                action: 0,
                progress: 0.0,
                transition: 1.0,
                camera_frame: [0.0, 0.0, 1.0],
            },
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Uniforms {
    size: [f32; 2],
    seed: u32,
    samples: u32,
    light: f32,
    color: f32,
    camera: f32,
    detail: f32,
    time: f32,
    effects: u32,
    action: u32,
    progress: f32,
    transition: f32,
    camera_frame: [f32; 3],
}

#[derive(Clone)]
pub struct Renderer {
    pub device: w::Device,
    pub queue: w::Queue,
    layout: w::BindGroupLayout,
    pipeline_layout: w::PipelineLayout,
}

pub struct Frame {
    pub texture: w::Texture,
    pub view: w::TextureView,
    pub size: [u32; 2],
}

impl Frame {
    pub fn display_view(&self) -> w::TextureView {
        // The render target stores display-encoded bytes. Let egui sample them as sRGB.
        self.texture.create_view(&w::TextureViewDescriptor {
            format: Some(w::TextureFormat::Rgba8UnormSrgb),
            ..Default::default()
        })
    }
}

impl Renderer {
    pub fn new(device: w::Device, queue: w::Queue) -> Self {
        let layout = device.create_bind_group_layout(&w::BindGroupLayoutDescriptor {
            label: Some("Scene uniforms"),
            entries: &[w::BindGroupLayoutEntry {
                binding: 0,
                visibility: w::ShaderStages::FRAGMENT,
                ty: w::BindingType::Buffer {
                    ty: w::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: NonZeroU64::new(size_of::<Uniforms>() as u64),
                },
                count: None,
            }],
        });
        let pipeline_layout = device.create_pipeline_layout(&w::PipelineLayoutDescriptor {
            label: Some("Wallpaper pipeline layout"),
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });
        Self {
            device,
            queue,
            layout,
            pipeline_layout,
        }
    }

    pub async fn headless() -> Result<Self, String> {
        let instance = w::Instance::new(&w::InstanceDescriptor::from_env_or_default());
        let adapter = instance
            .request_adapter(&w::RequestAdapterOptions::default())
            .await
            .ok_or("No compatible GPU was found")?;
        let (device, queue) = adapter
            .request_device(&w::DeviceDescriptor::default(), None)
            .await
            .map_err(|err| err.to_string())?;
        Ok(Self::new(device, queue))
    }

    pub fn compile(&self, source: &str, name: &str) -> Result<w::RenderPipeline, String> {
        let path = crate::scenes::directory().join("shared/cast.wgsl");
        let cast = match std::fs::read_to_string(&path) {
            Ok(text) => std::borrow::Cow::Owned(text),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                std::borrow::Cow::Borrowed(include_str!("../scenes/shared/cast.wgsl"))
            }
            Err(error) => return Err(format!("{}: {error}", path.display())),
        };
        let device = &self.device;
        device.push_error_scope(w::ErrorFilter::Validation);
        let source = format!("{}\n{}\n{}", source, include_str!("common.wgsl"), cast);
        let shader = device.create_shader_module(w::ShaderModuleDescriptor {
            label: Some(name),
            source: w::ShaderSource::Wgsl(source.into()),
        });
        let pipeline = device.create_render_pipeline(&w::RenderPipelineDescriptor {
            label: Some(name),
            layout: Some(&self.pipeline_layout),
            vertex: w::VertexState {
                module: &shader,
                entry_point: Some("fullscreen"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(w::FragmentState {
                module: &shader,
                entry_point: Some("pixel"),
                compilation_options: Default::default(),
                targets: &[Some(FORMAT.into())],
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview: None,
            cache: None,
        });
        match pollster::block_on(device.pop_error_scope()) {
            Some(err) => Err(err.to_string()),
            None => Ok(pipeline),
        }
    }

    pub fn frame(&self, size: [u32; 2]) -> Result<Frame, String> {
        let max = self.device.limits().max_texture_dimension_2d;
        if size.iter().any(|&n| n == 0 || n > max) {
            return Err(format!(
                "Each image dimension must be between 1 and {max} pixels."
            ));
        }
        let texture = self.device.create_texture(&w::TextureDescriptor {
            label: Some("Wallpaper"),
            size: extent(size),
            mip_level_count: 1,
            sample_count: 1,
            dimension: w::TextureDimension::D2,
            format: FORMAT,
            usage: w::TextureUsages::RENDER_ATTACHMENT
                | w::TextureUsages::TEXTURE_BINDING
                | w::TextureUsages::COPY_SRC,
            view_formats: &[w::TextureFormat::Rgba8UnormSrgb],
        });
        let view = texture.create_view(&Default::default());
        Ok(Frame {
            texture,
            view,
            size,
        })
    }

    pub fn draw(
        &self,
        encoder: &mut w::CommandEncoder,
        pipeline: &w::RenderPipeline,
        frame: &Frame,
        input: RenderInput,
        samples: u32,
    ) {
        let uniforms = Uniforms {
            size: frame.size.map(|n| n as f32),
            seed: input.settings.seed,
            samples,
            light: input.settings.light,
            color: input.settings.color,
            camera: input.settings.camera,
            detail: input.settings.detail,
            time: input.time,
            effects: input.effects.0,
            action: input.shot.action,
            progress: input.shot.progress,
            transition: input.shot.transition,
            camera_frame: input.shot.camera_frame,
        };
        let buffer = self
            .device
            .create_buffer_init(&w::util::BufferInitDescriptor {
                label: Some("Scene parameters"),
                contents: bytemuck::bytes_of(&uniforms),
                usage: w::BufferUsages::UNIFORM,
            });
        let group = self.device.create_bind_group(&w::BindGroupDescriptor {
            label: Some("Scene parameters"),
            layout: &self.layout,
            entries: &[w::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });

        let mut pass = encoder.begin_render_pass(&w::RenderPassDescriptor {
            label: Some("Render wallpaper"),
            color_attachments: &[Some(w::RenderPassColorAttachment {
                view: &frame.view,
                resolve_target: None,
                ops: w::Operations {
                    load: w::LoadOp::Clear(w::Color::BLACK),
                    store: w::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.draw(0..3, 0..1);
    }

    pub fn export(
        &self,
        pipeline: &w::RenderPipeline,
        input: RenderInput,
        size: [u32; 2],
        path: PathBuf,
    ) -> Result<Export, String> {
        let frame = self.frame(size)?;
        let row_bytes = size[0] * 4;
        let stride = row_bytes.next_multiple_of(w::COPY_BYTES_PER_ROW_ALIGNMENT);
        let buffer_size = u64::from(stride) * u64::from(size[1]);
        if buffer_size > self.device.limits().max_buffer_size {
            return Err("This image exceeds the GPU readback buffer limit.".into());
        }
        let buffer = self.device.create_buffer(&w::BufferDescriptor {
            label: Some("PNG readback"),
            size: buffer_size,
            usage: w::BufferUsages::COPY_DST | w::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = self.device.create_command_encoder(&Default::default());
        self.draw(&mut encoder, pipeline, &frame, input, 2);
        encoder.copy_texture_to_buffer(
            frame.texture.as_image_copy(),
            w::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: w::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(stride),
                    rows_per_image: None,
                },
            },
            extent(size),
        );
        let submission = self.queue.submit([encoder.finish()]);
        let (mapped_tx, mapped_rx) = mpsc::channel();
        buffer.slice(..).map_async(w::MapMode::Read, move |result| {
            let _ = mapped_tx.send(result);
        });

        let device = self.device.clone();
        let output = path.clone();
        let (done_tx, done_rx) = mpsc::channel();
        thread::spawn(move || {
            device.poll(w::Maintain::WaitForSubmissionIndex(submission));
            let result = mapped_rx
                .recv()
                .map_err(|err| err.to_string())
                .and_then(|result| result.map_err(|err| err.to_string()))
                .and_then(|()| {
                    let mapped = buffer.slice(..).get_mapped_range();
                    let mut pixels = Vec::with_capacity(row_bytes as usize * size[1] as usize);
                    for row in mapped.chunks_exact(stride as usize) {
                        pixels.extend_from_slice(&row[..row_bytes as usize]);
                    }
                    drop(mapped);
                    buffer.unmap();
                    save_png(&output, &pixels, size).map_err(|error| error.to_string())
                });
            let _ = done_tx.send(result);
        });
        Ok(Export {
            path,
            done: done_rx,
        })
    }
}

fn save_png(
    path: &Path,
    pixels: &[u8],
    [width, height]: [u32; 2],
) -> Result<(), Box<dyn std::error::Error>> {
    let directory = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut file = tempfile::NamedTempFile::new_in(directory)?;
    {
        let mut writer = BufWriter::new(file.as_file_mut());
        image::codecs::png::PngEncoder::new(&mut writer).write_image(
            pixels,
            width,
            height,
            image::ExtendedColorType::Rgba8,
        )?;
        writer.flush()?;
    }
    file.persist(path)?;
    Ok(())
}

pub struct Export {
    pub path: PathBuf,
    done: mpsc::Receiver<Result<(), String>>,
}

impl Export {
    pub fn check(&self) -> Option<Result<(), String>> {
        match self.done.try_recv() {
            Ok(result) => Some(result),
            Err(mpsc::TryRecvError::Disconnected) => Some(Err("The export worker stopped.".into())),
            Err(mpsc::TryRecvError::Empty) => None,
        }
    }

    pub fn wait(self) -> Result<(), String> {
        self.done.recv().map_err(|err| err.to_string())?
    }
}

fn extent([width, height]: [u32; 2]) -> w::Extent3d {
    w::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    }
}
