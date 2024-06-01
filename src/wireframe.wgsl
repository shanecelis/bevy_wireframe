// Import the standard 2d mesh uniforms and set their bind groups
#import bevy_sprite::mesh2d_functions

@group(2) @binding(0) var<storage> tri: array<vec4<f32>>;

// The structure of the vertex buffer is as specified in `specialize()`
struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @builtin(vertex_index) id : u32,
    @location(0) position: vec3<f32>,
    // @location(10) dist: vec4<f32>,
};

struct VertexOutput {
    // The vertex shader must set the on-screen position of the vertex
    @builtin(position) clip_position: vec4<f32>,
    // We pass the vertex color to the fragment shader in location 0
    @location(10) dist: vec4<f32>,
    @location(11) bary: vec3<f32>,
};

/// Entry point for the vertex shader
@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
    // Project the world position of the mesh into screen position
    let ti = vertex.id / 3;
    let vi = vertex.id % 3;
    out.bary = vec3<f32>(f32(vi == 0u), f32(vi == 1u), f32(vi == 2u));
    // out.bary = out.bary * 100.0;
    // out.bary = out.bary / max(vertex.dist.y, vertex.dist.x, vertex.dist.z);
    // out.bary = vec3<f32>(f32(vi == 0u)/vertex.dist.x, 0.0, 0.0);
    // out.bary = vec3<f32>(0.0, 0.0, 0.0);
    // out.bary = vec3<f32>(f32(vi == 0u)/vertex.dist.x, f32(vi == 1u), f32(vi == 2u));
    //out.bary = vec3<f32>(f32(vi == 0u)/vertex.dist.x, f32(vi == 1u)/vertex.dist.y, f32(vi == 2u)/vertex.dist.z);
    // out.bary *= 1.0 / vertex.dist.xyz;
    let model = mesh2d_functions::get_model_matrix(vertex.instance_index);
    out.clip_position = mesh2d_functions::mesh2d_position_local_to_clip(model, vec4<f32>(vertex.position, 1.0));
    // out.clip_position = vec4<f32>(vertex.position / 100.0, 1.0);
    // out.color = vec4<f32>(1.0, 1.0, 0.0, 1.0);
    //out.dist = vertex.dist;
    out.dist = vec4<f32>(tri[ti].w/tri[ti].xyz * out.bary, f32(ti));
    return out;
}

// The input of the fragment shader must correspond to the output of the vertex shader for all `location`s
struct FragmentInput {
    // The color is interpolated between vertices by default
    // @location(0) color: vec4<f32>,
    @location(10) dist: vec4<f32>,
    @location(11) bary: vec3<f32>,
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
    //return vec4<f32>(in.dist.w / 10000.0, 0.0, 0.0, 1.0);
    var color = vec4<f32>(1.0, 1.0, 0.0, 1.0);
    //let dist = in.dist;
    // let dist = vec3<f32>(in.dist.w / in.dist.x, in.dist.w / in.dist.y, in.dist.w / in.dist.z);
    let dist = in.dist.xyz;
    let i = min_index(dist.xyz);
    // color[i] = 1.0;
    let j = (i + 1) % 3;
    let d = dist[i];
    //let d = min(dist[0], min(dist[1], dist[2]));
    var I = exp2(-2.0 * d * d);
    var k = 1.0;
    if i == 1 {
            k = -1.0;
    }
    var wire_color = vec4<f32>(0.0, 0.0, 0.0, 1.0);
    wire_color[i] = 1.0;


    // return vec4(in.bary, 1.0);
    let a = 1.0 / 10.0;
    var length = tri[u32(in.dist.w)][i];
    // length = 100.0;

    // // if step(sin(k * in.bary[j] * 1.0 * pi), 0.0) > 0.0 {
    I *= step(sin(k * in.bary[j] * a * length * pi), -0.01);
    // I *= sin(k * in.bary[j] * 0.001 * pi);
       // return color;
    // } else {
       return I * wire_color + (1.0 - I) * color;
    // }
}
