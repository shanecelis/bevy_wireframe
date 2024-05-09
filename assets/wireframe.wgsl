struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(100) dist: vec3<f32>,
}

@vertex
fn main(
    @location(0) position: vec4<f32>,  // Input vertex attribute
) -> VertexOutput {
    var output: VertexOutput;
    output.position = position;  // Pass-through
    return output;
}
