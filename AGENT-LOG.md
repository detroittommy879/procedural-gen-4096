# Agent log

## 2026-11-24

- Built Wildframe as a standalone Rust/egui project. A shared wgpu renderer drives the preview and headless PNG export. Four shader scenes render all artwork procedurally. PNGs are encoded off the UI thread and moved into place after encoding succeeds.
- Fixed reserved WGSL identifiers, gaps at shallow terrain angles, and clipped inspector controls during testing. The preview now samples through an sRGB view so it displays the encoded PNG colors correctly.
- Verified the Windows release on Intel UHD 770. Rendered all four scenes, saved from the native dialog, decoded 4K and portrait outputs, and checked readback at a width with row padding. Two identical seed renders matched byte-for-byte and a different seed changed the output.
- Live-tested adding a shader, preserving the canvas after a syntax error, and correcting its color with F5. Removed the temporary shader. Release build, formatting, and strict Clippy checks passed. There are no automated unit tests. Other platforms were not exercised.
