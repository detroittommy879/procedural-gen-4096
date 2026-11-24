# Wildframe

A Rust and egui wallpaper studio with four GPU-rendered scenes. All geometry, textures, lighting, and reflections are generated in WGSL.

## Run

```powershell
cargo run --release --locked
```

Pick **Alpine Lake**, **Red Arch**, **Lavender Fields**, or **Neon Passage**. Use **Shuffle** for a new seed or enter a seed to revisit a composition. **Reset** restores the scene's default settings. **Save PNG** offers Full HD, QHD, 4K UHD, ultrawide, and portrait output. The preview follows the selected aspect ratio.

## Edit or add scenes while the app runs

Edit a file under `scenes/`, or duplicate one and rename the copy. Press **F5** or click **Reload scenes**. The app displays a shader error with the line number if compilation fails. When editing the selected scene, the last valid preview remains available while you fix it.

Each file defines `fn scene(film: vec2f) -> vec3f`. Optional `// name:` and `// description:` lines supply the display text. `film` is centered with Y pointing up. The shared uniforms are `p.seed` and the four controls `p.light`, `p.color`, `p.camera`, and `p.detail` (each 0–1). See `src/common.wgsl` for shared functions and the color grade.

The executable includes the four defaults. If you move it out of the project and want editable scenes, put the `scenes` folder next to it.

## Render from a terminal

The command uses the scene's default controls and the supplied seed and dimensions.

```powershell
cargo run --release --locked -- --render neon-passage 915 3840 2160 neon.png
```
