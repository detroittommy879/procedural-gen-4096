#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod app;
mod gpu;
mod scenes;

use std::{error::Error, path::PathBuf};

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if !args.is_empty() {
        return render_from_args(&args);
    }

    eframe::run_native(
        "Wildframe",
        eframe::NativeOptions {
            renderer: eframe::Renderer::Wgpu,
            viewport: eframe::egui::ViewportBuilder::default()
                .with_inner_size([1350.0, 860.0])
                .with_min_inner_size([830.0, 580.0]),
            ..Default::default()
        },
        Box::new(|cc| Ok(Box::new(app::Studio::new(cc)?))),
    )?;
    Ok(())
}

fn render_from_args(args: &[std::ffi::OsString]) -> Result<(), Box<dyn Error>> {
    let [flag, id, seed, width, height, destination] = args else {
        return Err(
            "Usage: wildframe --render <scene> <seed> <width> <height> <output.png>".into(),
        );
    };
    if flag != "--render" {
        return Err("Expected --render".into());
    }
    let scene = scenes::Library::load(&scenes::directory())?
        .into_iter()
        .find(|scene| id == scene.id.as_str())
        .ok_or("Unknown scene. Use a filename from scenes/ without .wgsl.")?;
    let mut settings = scene.defaults;
    settings.seed = seed.to_string_lossy().parse()?;
    let size = [
        width.to_string_lossy().parse()?,
        height.to_string_lossy().parse()?,
    ];

    let gpu = pollster::block_on(gpu::Renderer::headless())?;
    let pipeline = gpu.compile(&scene.source, &scene.name)?;
    let pending = gpu.export(&pipeline, settings, size, PathBuf::from(destination))?;
    pending.wait()?;
    Ok(())
}
