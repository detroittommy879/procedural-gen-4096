# Agent log

## 2026-11-24 · Tidal Engine and story direction

- Added an animated flooded machine hall with nested geared rings, orbiting weights, fans, and water reflections. Split the return journey into two chapters and authored 25 camera shots across the 30-minute story. Added service drones, flocks, reef rays, relays, and a cell Mica can pick up and deliver.
- Added shot navigation, independent camera and supporting-cast switches, and F5 reload for the shared character shader. Existing camera-free story files still use the wide view.
- A reserved WGSL name failed validation during development. Renamed it and rendered all eight chapters on Windows. First use of the new shader also exposed a UI stall. Moved compilation to a background thread and tested seeking from 20:30 to 21:10 while the chamber prepared.
- Release build, strict Clippy, and three timeline tests passed. Captured a 48-frame animated preview from the renderer. In the native app an intentional shared-shader error retained the last valid image and disabled export.

## 2026-11-24 · Motion and story

- Added animation to the original scenes and built Lantern Observatory and Glass Reef. Studio can play, pause, rewind, or set a scene time. Cloud, reflection, particle, cast-shadow, and grain switches change the render independently.
- Built *The Last Light*, a 30-minute timeline with seven chapters, timed captions, and shader-drawn Mica and Kite. Chapter data lives in `scenes/story.json`. Reload validates the timeline, and preview resolution drops during playback to avoid piling up GPU work. PNG export uses the selected frame at full resolution.
- Rendered every chapter and ran the whole timeline at 60× on Windows. Confirmed the last chapter stops at 30:00, independent reflection switching, and story edits via F5. Increased the preview footer space after finding clipped playback controls. Release build, both timeline tests, and strict Clippy passed.

## 2026-11-24

- Built Wildframe as a standalone Rust/egui project. A shared wgpu renderer drives the preview and headless PNG export. Four shader scenes render all artwork procedurally. PNGs are encoded off the UI thread and moved into place after encoding succeeds.
- Fixed reserved WGSL identifiers, gaps at shallow terrain angles, and clipped inspector controls during testing. The preview now samples through an sRGB view so it displays the encoded PNG colors correctly.
- Verified the Windows release on Intel UHD 770. Rendered all four scenes, saved from the native dialog, decoded 4K and portrait outputs, and checked readback at a width with row padding. Two identical seed renders matched byte-for-byte and a different seed changed the output.
- Live-tested adding a shader, preserving the canvas after a syntax error, and correcting its color with F5. Removed the temporary shader. Release build, formatting, and strict Clippy checks passed. There are no automated unit tests. Other platforms were not exercised.
