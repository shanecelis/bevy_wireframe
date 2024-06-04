use bevy::prelude::*;
use bevy::sprite::{Material2d, MaterialMesh2dBundle};

use bevy::render::{

        texture::GpuImage,
        render_asset::RenderAssets,
    render_resource::*,
};

#[derive(Reflect, Debug, Clone)]
pub enum Style {
    Solid,
    Dash,
    Dot,
}

// NOTE: These must match the bit flags in bevy_sprite/src/mesh2d/color_material.wgsl!
bitflags::bitflags! {
    #[repr(transparent)]
    pub struct WireframeMaterialFlags: u32 {
        const MODEL_SPACE       = 1 << 1;
        const SCREEN_SPACE      = 1 << 0;
        const NONE              = 0;
        const UNINITIALIZED     = 0xFFFF;
    }
}
#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
#[reflect(Default, Debug)]
#[uniform(1, WireframeMaterialUniform)]
pub struct WireframeMaterial {
    pub color: Color,
    pub style: Style,
}

impl Default for WireframeMaterial {
    fn default() -> Self {
        WireframeMaterial {
            color: Color::WHITE,
            style: Style::Solid,
        }
    }
}


/// The GPU representation of the uniform data of a [`WireframeMaterial`].
#[derive(Clone, Default, ShaderType)]
pub struct WireframeMaterialUniform {
    pub color: Vec4,
    pub flags: u32,
    pub style: Vec2,
}

impl AsBindGroupShaderType<WireframeMaterialUniform> for WireframeMaterial {
    fn as_bind_group_shader_type(&self, _images: &RenderAssets<GpuImage>) -> WireframeMaterialUniform {
        // let mut flags = ColorMaterialFlags::NONE;
        // if self.texture.is_some() {
        //     flags |= ColorMaterialFlags::TEXTURE;
        // }

        WireframeMaterialUniform {
            color: LinearRgba::from(self.color).to_f32_array().into(),
            flags: 0,
            style: Vec2::new(5.0, 5.0)
        }
    }
}

impl Material2d for WireframeMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://bevy_wireframe/wireframe.wgsl".into()
    }
}

/// A component bundle for entities with a [`Mesh2dHandle`](crate::Mesh2dHandle) and a [`WireframeMaterial`].
pub type WireframeMesh2dBundle = MaterialMesh2dBundle<WireframeMaterial>;
