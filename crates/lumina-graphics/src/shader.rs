//! Embedded WGSL shaders. Keeping them inline avoids shipping extra
//! files and lets the engine be a single self-contained binary.

pub const SPRITE_SHADER: &str = r#"// 2D sprite shader - textured quad with vertex color tint.
struct VertexIn {
    @location(0) position: vec2<f32>,
    @location(1) uv:       vec2<f32>,
    @location(2) color:    vec4<f32>,
};

struct VertexOut {
    @builtin(position) clip: vec4<f32>,
    @location(0)       uv:   vec2<f32>,
    @location(1)       color: vec4<f32>,
};

struct CameraUniform {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0) var<uniform> camera: CameraUniform;
@group(0) @binding(1) var tex: texture_2d<f32>;
@group(0) @binding(2) var samp: sampler;

@vertex
fn vs_main(in: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.clip  = camera.view_proj * vec4<f32>(in.position, 0.0, 1.0);
    out.uv    = in.uv;
    out.color = in.color;
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let texel = textureSample(tex, samp, in.uv);
    return texel * in.color;
}
"#;

pub const MESH_SHADER: &str = r#"// 3D mesh shader - simple directional light + ambient.
struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) normal:   vec3<f32>,
    @location(2) uv:       vec2<f32>,
};

struct VertexOut {
    @builtin(position) clip:  vec4<f32>,
    @location(0)       world: vec3<f32>,
    @location(1)       normal: vec3<f32>,
    @location(2)       uv:    vec2<f32>,
};

struct CameraUniform {
    view_proj: mat4x4<f32>,
};

struct ModelUniform {
    model: mat4x4<f32>,
    color: vec4<f32>,
};

@group(0) @binding(0) var<uniform> camera: CameraUniform;
@group(0) @binding(1) var<uniform> model: ModelUniform;
@group(0) @binding(2) var tex: texture_2d<f32>;
@group(0) @binding(3) var samp: sampler;

@vertex
fn vs_main(in: VertexIn) -> VertexOut {
    var out: VertexOut;
    let world = model.model * vec4<f32>(in.position, 1.0);
    out.clip   = camera.view_proj * world;
    out.world  = world.xyz;
    out.normal = (model.model * vec4<f32>(in.normal, 0.0)).xyz;
    out.uv     = in.uv;
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let n = normalize(in.normal);
    let light_dir = normalize(vec3<f32>(0.4, 0.8, 0.3));
    let diff = max(dot(n, light_dir), 0.0);
    let ambient = 0.25;
    let texel = textureSample(tex, samp, in.uv);
    let lit = (ambient + 0.75 * diff) * texel * model.color;
    return vec4<f32>(lit.rgb, texel.a * model.color.a);
}
"#;

pub const PARTICLE_SHADER: &str = r#"// Point/sprite particle shader.
struct VertexIn {
    @location(0) position: vec2<f32>,
    @location(1) uv:       vec2<f32>,
    @location(2) color:    vec4<f32>,
};

struct VertexOut {
    @builtin(position) clip:  vec4<f32>,
    @location(0)       uv:    vec2<f32>,
    @location(1)       color: vec4<f32>,
};

struct CameraUniform { view_proj: mat4x4<f32>; };

@group(0) @binding(0) var<uniform> camera: CameraUniform;

@vertex
fn vs_main(in: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.clip  = camera.view_proj * vec4<f32>(in.position, 0.0, 1.0);
    out.uv    = in.uv;
    out.color = in.color;
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    // Soft circular falloff
    let d = in.uv - vec2<f32>(0.5);
    let r = length(d) * 2.0;
    let alpha = clamp(1.0 - r, 0.0, 1.0);
    return vec4<f32>(in.color.rgb, in.color.a * alpha);
}
"#;
