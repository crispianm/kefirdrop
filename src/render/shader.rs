//! Shader assembly.
//!
//! A preset shader only defines `fn render(uv: vec2<f32>) -> vec4<f32>`. We
//! sandwich it between the shared prelude (uniforms, bindings, vertex shader and
//! helpers) and a fixed `fs_main` that calls `render`. WGSL has no forward
//! references, so `render` must appear *before* `fs_main` — hence this ordering.

/// The entry-point fragment shader appended after the preset body.
const FS_MAIN: &str = r#"
@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    return render(in.uv);
}
"#;

/// Combine the prelude and a preset body into a complete shader module source.
pub fn assemble(prelude: &str, preset_body: &str) -> String {
    format!("{prelude}\n// ---- preset ----\n{preset_body}\n{FS_MAIN}")
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    /// Parse and fully validate a WGSL source the same way wgpu would at runtime.
    fn validate(label: &str, source: &str) {
        let module = naga::front::wgsl::parse_str(source)
            .unwrap_or_else(|e| panic!("{label}: WGSL parse error:\n{}", e.emit_to_string(source)));
        let mut validator = naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        );
        validator
            .validate(&module)
            .unwrap_or_else(|e| panic!("{label}: WGSL validation error: {e:?}"));
    }

    fn assets() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("assets")
    }

    /// Every bundled preset, once assembled with the prelude, must compile.
    #[test]
    fn all_presets_validate() {
        let assets = assets();
        let prelude = std::fs::read_to_string(assets.join("shaders/prelude.wgsl")).unwrap();

        let presets_dir = assets.join("presets");
        let mut checked = 0;
        for entry in std::fs::read_dir(&presets_dir).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().and_then(|e| e.to_str()) != Some("toml") {
                continue;
            }
            let spec: crate::preset::PresetSpec =
                toml::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
            let body = std::fs::read_to_string(assets.join(&spec.shader)).unwrap();
            validate(&spec.name, &super::assemble(&prelude, &body));
            checked += 1;
        }
        assert!(checked > 0, "no presets were validated");
    }

    /// The standalone blit shader must compile too.
    #[test]
    fn blit_validates() {
        let source = std::fs::read_to_string(assets().join("shaders/blit.wgsl")).unwrap();
        validate("blit", &source);
    }
}
