struct Uniforms {
    view_proj: mat4x4<f32>,
    eye: vec4<f32>,
    right: vec4<f32>,
    up: vec4<f32>,
    forward: vec4<f32>,
    surface_color: vec4<f32>,
    // rgb: 選択中の面に混ぜる色、a: 混ぜる割合
    highlight_color: vec4<f32>,
    // x, y: ビューの大きさ（ピクセル）、z: 1 ポイントのピクセル数、w: 出力先が sRGB なら 1
    params: vec4<f32>,
};

@group(0) @binding(0) var<uniform> u: Uniforms;

fn srgb_to_linear(c: vec3<f32>) -> vec3<f32> {
    let lo = c / 12.92;
    let hi = pow((c + vec3<f32>(0.055)) / 1.055, vec3<f32>(2.4));
    return select(hi, lo, c <= vec3<f32>(0.04045));
}

fn to_output(c: vec3<f32>) -> vec4<f32> {
    let clamped = clamp(c, vec3<f32>(0.0), vec3<f32>(1.0));
    if (u.params.w > 0.5) {
        return vec4<f32>(srgb_to_linear(clamped), 1.0);
    }
    return vec4<f32>(clamped, 1.0);
}

// ---- 曲面 ----

struct SurfaceOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) world: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tint: f32,
};

@vertex
fn vs_surface(
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tint: f32,
) -> SurfaceOut {
    var out: SurfaceOut;
    out.clip = u.view_proj * vec4<f32>(position, 1.0);
    out.world = position;
    out.normal = normal;
    out.tint = tint;
    return out;
}

// JS 試作と同じく、光を当てた後の色に選択の色を混ぜる
fn tinted(c: vec3<f32>, tint: f32) -> vec3<f32> {
    let k = clamp(tint, 0.0, 1.0) * u.highlight_color.a;
    return mix(clamp(c, vec3<f32>(0.0), vec3<f32>(1.0)), u.highlight_color.rgb, k);
}

@fragment
fn fs_surface(in: SurfaceOut) -> @location(0) vec4<f32> {
    // 三角形の巻き方向に頼らず両面を照らすため、画面の微分から視点側を向く面の向きを求める
    var geometric = cross(dpdx(in.world), dpdy(in.world));
    if (dot(geometric, u.eye.xyz - in.world) < 0.0) {
        geometric = -geometric;
    }
    let len = length(in.normal);
    let base = u.surface_color.rgb;
    if (len < 1e-9) {
        return to_output(tinted(base * 0.3, in.tint));
    }
    var n = in.normal / len;
    if (dot(n, geometric) < 0.0) {
        n = -n;
    }
    // JS 試作と同じく、カメラの右・上・視線方向の座標で光を当てる
    let v = vec3<f32>(dot(n, u.right.xyz), dot(n, u.up.xyz), dot(n, u.forward.xyz));
    let light = normalize(vec3<f32>(-0.4, 0.6, -0.7));
    let half_dir = normalize(light - vec3<f32>(0.0, 0.0, 1.0));
    let diffuse = 0.3 + 0.1 * v.y + 0.68 * max(0.0, dot(v, light));
    let spec = 0.22 * pow(max(0.0, dot(v, half_dir)), 28.0);
    return to_output(tinted(base * diffuse + vec3<f32>(spec), in.tint));
}

// ---- ケージの線 ----

struct ColorOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) color: vec3<f32>,
};

@vertex
fn vs_line(@location(0) position: vec3<f32>, @location(1) color: vec3<f32>) -> ColorOut {
    var out: ColorOut;
    out.clip = u.view_proj * vec4<f32>(position, 1.0);
    out.color = color;
    return out;
}

// ---- ケージの点（画面上で一定の大きさの四角） ----

@vertex
fn vs_point(
    @builtin(vertex_index) corner: u32,
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
    @location(2) size: f32,
) -> ColorOut {
    var offsets = array<vec2<f32>, 6>(
        vec2<f32>(-0.5, -0.5),
        vec2<f32>(0.5, -0.5),
        vec2<f32>(0.5, 0.5),
        vec2<f32>(-0.5, -0.5),
        vec2<f32>(0.5, 0.5),
        vec2<f32>(-0.5, 0.5),
    );
    var clip = u.view_proj * vec4<f32>(position, 1.0);
    let viewport = max(u.params.xy, vec2<f32>(1.0));
    clip = vec4<f32>(clip.xy + offsets[corner % 6u] * size * u.params.z * 2.0 / viewport * clip.w, clip.zw);
    var out: ColorOut;
    out.clip = clip;
    out.color = color;
    return out;
}

@fragment
fn fs_color(in: ColorOut) -> @location(0) vec4<f32> {
    return to_output(in.color);
}
