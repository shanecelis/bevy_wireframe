pub(crate) mod compute;
pub mod material;
pub mod wireframe2d;

pub(crate) fn pad(v: [f32; 3]) -> [f32; 4] {
    [v[0], v[1], v[2], 0.0]
}
