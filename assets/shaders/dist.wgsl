// Structured buffer for input vertex data
struct VertexData {
    position: vec4<f32>,  // Vertex position with a w-component
}

// Structured buffer for output data
struct OutputData {
    dist: vec4<f32>,  // Output distance values
}

// Uniform for window scale
// struct WinScale {
//     scale: vec2<f32>,
// }

// Buffers
@group(0) @binding(0) var<storage> vertexInput: array<VertexData>;
// @group(0) @binding(1) var<storage, read_write> vertexInput2: array<VertexData>;
@group(0) @binding(1) var<storage, read_write> outputBuffer: array<OutputData>;
// @group(0) @binding(2) var<uniform> winScale: WinScale;

// Compute shader
@compute @workgroup_size(1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let tri_index = global_id.x;
    let index = global_id.x * 3;  // This index maps to a set of vertices (assuming they come in groups of 3)

    // Ensure we have enough data (assuming input vertices come in groups of 3)
    if (index + 2u < arrayLength(&vertexInput)) {
        // let p0 = winScale.scale * (vertexInput[index].position.xy / vertexInput[index].position.w);
        // let p1 = winScale.scale * (vertexInput[index + 1].position.xy / vertexInput[index + 1].position.w);
        // let p2 = winScale.scale * (vertexInput[index + 2].position.xy / vertexInput[index + 2].position.w);
        // let p0 = (vertexInput[index].position.xy / vertexInput[index].position.w);
        // let p1 = (vertexInput[index + 1].position.xy / vertexInput[index + 1].position.w);
        // let p2 = (vertexInput[index + 2].position.xy / vertexInput[index + 2].position.w);
#ifdef MODEL_DIST
        let p0 = vertexInput[index].position.xyz;
        let p1 = vertexInput[index + 1].position.xyz;
        let p2 = vertexInput[index + 2].position.xyz;
#else
        let p0 = vertexInput[index].position.xy;
        let p1 = vertexInput[index + 1].position.xy;
        let p2 = vertexInput[index + 2].position.xy;
#endif

        let v0 = p2 - p1;
        let v1 = p2 - p0;
        let v2 = p1 - p0;

#ifdef MODEL_DIST
        let area = length(cross(v1, v2));
#else
        let area = abs(v1.x * v2.y - v1.y * v2.x); // Where is the 1/2 factor?
#endif

        // outputBuffer[index].dist = vec4(area / length(v0), 0, 0, 0);
        // outputBuffer[index + 1].dist = vec4(0, area / length(v1), 0, 0);
        // outputBuffer[index + 2].dist = vec4(0, 0, area / length(v2), 0);
        // outputBuffer[index + 0].dist = vec4(1/length(v0), 0, 0, area);
        // outputBuffer[index + 1].dist = vec4(0, 1/length(v1), 0, area);
        // outputBuffer[index + 2].dist = vec4(0, 0, 1/length(v2), area);
        outputBuffer[tri_index].dist = vec4(length(v0), length(v1), length(v2), area);
    }
    // outputBuffer[0].dist = vec4(1.0, 1.0, 1.0, 0.0);
}
