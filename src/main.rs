#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod app;
mod gpu;
mod scenes;
mod story;

use gpu::{Effects, RenderInput};
use std::{error::Error, path::PathBuf};
use story::Timeline;

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
    let directory = scenes::directory();
    let scenes = scenes::Library::load(&directory)?;
    let (scene, input, width, height, destination) = match args {
        [flag, id, seed, width, height, destination, extra @ ..] if flag == "--render" && extra.len() <= 1 => {
            let scene = scenes.iter().find(|scene| id == scene.id.as_str())
                .ok_or("Unknown scene. Use a filename from scenes/ without .wgsl.")?;
            let mut input = RenderInput::still(scene.defaults);
            input.settings.seed = seed.to_string_lossy().parse()?;
            if let Some(time) = extra.first() {
                input.time = parse_time(time, 86400.0)?;
            }
            (scene, input, width, height, destination)
        }
        [flag, time, width, height, destination] if flag == "--story" => {
            let timeline = Timeline::load(&directory, &scenes)?;
            let time = parse_time(time, timeline.duration())?;
            let (index, input) = timeline.frame(time, &scenes, Effects::default());
            (&scenes[index], input, width, height, destination)
        }
        _ => return Err("Usage: wildframe --render <scene> <seed> <width> <height> <output.png> [seconds]\n       wildframe --story <seconds> <width> <height> <output.png>".into()),
    };
    let size = [
        width.to_string_lossy().parse()?,
        height.to_string_lossy().parse()?,
    ];

    let gpu = pollster::block_on(gpu::Renderer::headless())?;
    let pipeline = gpu.compile(&scene.source, &scene.name)?;
    let pending = gpu.export(&pipeline, input, size, PathBuf::from(destination))?;
    pending.wait()?;
    Ok(())
}

fn parse_time(value: &std::ffi::OsStr, end: f32) -> Result<f32, Box<dyn Error>> {
    let time = value.to_string_lossy().parse()?;
    if (0.0..=end).contains(&time) {
        Ok(time)
    } else {
        Err(format!("Seconds must be a number between 0 and {end}.").into())
    }
}
