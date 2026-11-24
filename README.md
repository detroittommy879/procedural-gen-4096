# Wildframe

Wildframe is a Rust and egui studio for animated procedural wallpapers. The six scenes, from Alpine Lake to Glass Reef, render on the GPU from editable WGSL.

![Mica and Kite carry a glowing power cell through Neon Passage](docs/story-preview.png)

## Run

```powershell
cargo run --release --locked
```

- **Studio** lets you shuffle the seed, set lighting and camera controls, play or pause the motion, and enter a frame time in seconds.
- **Story** plays *The Last Light*, a 30-minute, seven-chapter trip with Mica the rover and Kite the drone. Select a chapter or scrub the timeline. Playback speeds go up to 60×.
- **Render effects** switches clouds, reflections, particles, cast shadows, and film grain independently. Effects apply where the scene uses them.
- **Save frame as PNG** exports the current frame. Available formats include 4K UHD, ultrawide, and portrait.

## Add or edit scenes and chapters

Edit or duplicate a file in [scenes/](scenes/) and press **F5** or click **Reload files**. Every scene defines `fn scene(film: vec2f) -> vec3f`. The optional `// name:` and `// description:` headers supply the display text. The shared uniforms are `p.seed`, `p.time` in seconds, and `p.light`, `p.color`, `p.camera`, and `p.detail` from 0 to 1. See [common.wgsl](src/common.wgsl) for color, noise, and effect switches.

Edit [scenes/story.json](scenes/story.json) to rearrange chapters, choose scene IDs, adjust durations and light, or change the timed captions. Each chapter can use an `action` from the bundled example to direct the characters, or omit the action for scenery alone. Reload applies scene and story edits in the running app. The executable embeds the defaults and looks for editable files in a `scenes` directory alongside it when moved out of the project.

## Render from a terminal

```powershell
cargo run --release --locked -- --render neon-passage 915 3840 2160 tunnel.png 12.5
cargo run --release --locked -- --story 1435 1920 1080 return-line.png
```

The optional last argument to `--render` is the scene time. `--story` takes an absolute story time in seconds.
