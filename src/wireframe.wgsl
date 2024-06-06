// Import the standard 2d mesh uniforms and set their bind groups
#import bevy_sprite::mesh2d_functions

#import bevy_sprite::{
    mesh2d_view_bindings::view,
    // mesh2d_bindings::mesh,
}

#ifdef WIREFRAME_MATERIAL
struct WireframeMaterial {
    color: vec4<f32>,
    // 'flags' is a bit field indicating various options. u32 is 32 bits so we have up to 32 options.
    // flags: u32,
    // style: vec2<f32>,
};
@group(2) @binding(0) var<uniform> material: WireframeMaterial;
@group(3) @binding(0) var<storage> tri: array<vec4<f32>>;

const WIREFRAME_MATERIAL_FLAGS_SCREENSPACE_BIT: u32 = 1u;
#else
@group(2) @binding(0) var<storage> tri: array<vec4<f32>>;
#endif

// The structure of the vertex buffer is as specified in `specialize()`
struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @builtin(vertex_index) id : u32,
    @location(0) position: vec3<f32>,
};

struct VertexOutput {
    // The vertex shader must set the on-screen position of the vertex
    @builtin(position) clip_position: vec4<f32>,
    // We pass the vertex color to the fragment shader in location 0
    @location(0) dist: vec4<f32>,
    @location(1) bary: vec3<f32>,
};

/// Entry point for the vertex shader
@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
    // Project the world position of the mesh into screen position
    let ti = vertex.id / 3;
    let vi = vertex.id % 3;
    out.bary = vec3<f32>(f32(vi == 0u), f32(vi == 1u), f32(vi == 2u));
    let model = mesh2d_functions::get_world_from_local(vertex.instance_index);
    out.clip_position = mesh2d_functions::mesh2d_position_local_to_clip(model, vec4<f32>(vertex.position, 1.0));
    out.dist = vec4<f32>(tri[ti].w/tri[ti].xyz * out.bary, f32(ti));
    return out;
}

// The input of the fragment shader must correspond to the output of the vertex shader for all `location`s
struct FragmentInput {
    @builtin(position) position: vec4<f32>,
    // The color is interpolated between vertices by default
    // @location(0) color: vec4<f32>,
    @location(0) dist: vec4<f32>,
    @location(1) bary: vec3<f32>,
};

const WIRE_COL: vec4<f32> = vec4(0.0, 0.0, 1.0, 1.0);

fn min_index(v: vec3<f32>) -> u32 {
   var i: u32 = 0;
   for (var j: u32 = 1; j < 3; j++) {
      if v[j] < v[i] {
           i = j;
      }
   }
   return i;
}
const pi = 3.14159265359;

/// Entry point for the fragment shader
@fragment
fn fragment(in: FragmentInput) -> @location(0) vec4<f32> {
    let color = vec4<f32>(1.0, 1.0, 0.0, 1.0);
    let dist = in.dist.xyz;
    let i = min_index(dist.xyz);
    let j = (i + 1) % 3;
    //let d = min(dist[0], min(dist[1], dist[2]));
    let d = dist[i];
    var I = exp2(-2.0 * d * d);
    var k = 1.0;
    if i == 1 {
            k = -1.0;
    }
    var wire_color = vec4<f32>(0.0, 0.0, 0.0, 1.0);
    wire_color[i] = 1.0;


    /// This is still a special case where the model space and pixel space are
    /// equivalent.
    let width_pixel = 10.0;
    var length = tri[u32(in.dist.w)][i];

    I *= step(sin(k * in.bary[j] * length * pi / width_pixel), -0.01);
    return vec4<f32>(wire_color.xyz, I);
    // return I * wire_color + (1.0 - I) * color;
}
