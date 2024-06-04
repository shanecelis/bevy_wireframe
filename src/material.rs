use bevy::prelude::*;
use bevy::sprite::{Material2d, MaterialMesh2dBundle};

use bevy::{
    asset::{embedded_asset, DirectAssetAccessExt, Handle},
    sprite::{SetMesh2dBindGroup, SetMesh2dViewBindGroup, DrawMesh2d, Material2dDrawPlugin, SetMaterial2dBindGroup},

    render::{
    render_phase::{
        AddRenderCommand, DrawFunctions, PhaseItem, PhaseItemExtraIndex, RenderCommand,
        RenderCommandResult, SetItemPipeline, TrackedRenderPass, ViewSortedRenderPhases,
    },

        texture::GpuImage,
        render_asset::RenderAssets,
    render_resource::*,
    }};
use crate::wireframe2d::SetFaceBindGroup;

#[derive(Reflect, Debug, Clone)]
pub enum Style {
    Solid,
    Dash,
    Dot,
}

pub type DrawWireframeMaterial2d<M> = (
    SetItemPipeline,
    SetMesh2dViewBindGroup<0>,
    SetMesh2dBindGroup<1>,
    SetMaterial2dBindGroup<M, 2>,
    SetFaceBindGroup<3>,
    DrawMesh2d,
);

#[derive(Default)]
pub struct WireframeMaterial2dPlugin;

impl Plugin for WireframeMaterial2dPlugin {
    fn build(&self, app: &mut App) {

        app.add_plugins(crate::compute::FacePlugin);
        embedded_asset!(app, "wireframe.wgsl");
        app.add_plugins(Material2dDrawPlugin::<WireframeMaterial, DrawWireframeMaterial2d<WireframeMaterial>>::default());
            // .register_asset_reflect::<WireframeMaterial>();

        // app.world_mut()
        //     .resource_mut::<Assets<WireframeMaterial>>()
        //     .insert(
        //         &Handle::<WireframeMaterial>::default(),
        //         WireframeMaterial {
        //             color: Color::srgb(1.0, 0.0, 1.0),
        //             ..Default::default()
        //         },
        //     );
    }
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
#[derive(Asset, AsBindGroup, Debug, Clone, TypePath)]
// #[reflect(Default, Debug)]
// #[uniform(1, WireframeMaterialUniform)]
pub struct WireframeMaterial {
    // #[storage(0, read_only, buffer)]
    // faces: Option<Buffer>,
    #[uniform(1)]
    pub color: LinearRgba,
    // pub style: Style,
}

impl Default for WireframeMaterial {
    fn default() -> Self {
        WireframeMaterial {
            color: Color::WHITE.into(),
            // style: Style::Solid,
        }
    }
}


/// The GPU representation of the uniform data of a [`WireframeMaterial`].
// #[derive(Clone, Default, ShaderType)]
// pub struct WireframeMaterialUniform {
//     pub color: Vec4,
//     pub flags: u32,
//     pub style: Vec2,
// }

// impl AsBindGroupShaderType<WireframeMaterialUniform> for WireframeMaterial {
//     fn as_bind_group_shader_type(&self, _images: &RenderAssets<GpuImage>) -> WireframeMaterialUniform {
//         // let mut flags = ColorMaterialFlags::NONE;
//         // if self.texture.is_some() {
//         //     flags |= ColorMaterialFlags::TEXTURE;
//         // }

//         WireframeMaterialUniform {
//             color: LinearRgba::from(self.color).to_f32_array().into(),
//             flags: 0,
//             style: Vec2::new(5.0, 5.0)
//         }
//     }
// }

impl Material2d for WireframeMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://bevy_wireframe/wireframe.wgsl".into()
    }

    fn specialize(
        _pipeline: &MaterialPipeline<Self>,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayout,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        /// XXX: How can I add the face buffer layout? I don't have access to
        /// the renderdevice.
        ///
        /// Seems like it requires its own pipeline.

        // let vertex_layout = layout.get_layout(&[
        //     Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
        //     ATTRIBUTE_BLEND_COLOR.at_shader_location(1),
        // ])?;
        // descriptor.vertex.buffers = vec![vertex_layout];
        //
        Ok(())
    }
}

/// A component bundle for entities with a [`Mesh2dHandle`](crate::Mesh2dHandle) and a [`WireframeMaterial`].
pub type WireframeMesh2dBundle = MaterialMesh2dBundle<WireframeMaterial>;
