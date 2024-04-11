#import bevy_sprite::mesh2d_vertex_output::VertexOutput

@group(2) @binding(1) var base_color_texture: texture_2d<f32>;
@group(2) @binding(2) var base_color_sampler: sampler;
@group(2) @binding(3) var<uniform> is_grayscale: u32;

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(base_color_texture, base_color_sampler, mesh.uv);
    if is_grayscale == 0 {
        return color;
    } else {
        let grayscale = 0.299 * color.r + 0.587 * color.g + 0.114 * color.b;
        return vec4<f32>(vec3<f32>(grayscale, grayscale, grayscale), color.a);
    }
}