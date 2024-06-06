use bevy::prelude::*;
use bevy::sprite::{Material2d, MaterialMesh2dBundle};

use bevy::{

    asset::{embedded_asset, DirectAssetAccessExt, Handle},
    sprite::{SetMesh2dBindGroup, SetMesh2dViewBindGroup, DrawMesh2d, Material2dGenericPlugin, SetMaterial2dBindGroup, Material2dKey, Material2dPipeline, Mesh2dPipelineKey, Material2dLayout},

    render::{
Extract, ExtractSchedule,
        renderer::RenderDevice,
        mesh::MeshVertexBufferLayoutRef,
    render_phase::{
        AddRenderCommand, DrawFunctions, PhaseItem, PhaseItemExtraIndex, RenderCommand,
        RenderCommandResult, SetItemPipeline, TrackedRenderPass, ViewSortedRenderPhases,
    },

        texture::GpuImage,
        render_asset::RenderAssets,
        render_resource::{*, binding_types::storage_buffer_read_only},
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

#[derive(Clone, Resource)]
pub struct WireframePipeline {
    material2d_pipeline: Material2dPipeline<WireframeMaterial>,
    face_layout: BindGroupLayout,
}

fn extract_wireframe_meshes_2d(
    mut commands: Commands,
    query: Extract<Query<(Entity, &ViewVisibility, &Handle<WireframeMaterial>)>>,
) {
    for (entity, view_visibility, handle) in &query {
        if view_visibility.get() {
            commands.entity(entity).insert(handle.clone());
        }
    }
}

impl FromWorld for WireframePipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        // let shader = world.load_asset::<Shader>("embedded://bevy_wireframe/wireframe.wgsl");
        let face_layout = render_device.create_bind_group_layout(
            "Face",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                (storage_buffer_read_only::<Vec<Vec4>>(false),),
            ),
        );
        let material2d_pipeline = Material2dPipeline::from_world(world);

        // world.insert_resource(Material2dLayout(face_layout.clone()));
        Self {
            material2d_pipeline,
            // shader,
            face_layout,
        }
    }
}

// We implement `SpecializedPipeline` to customize the default rendering from `Mesh2dPipeline`
impl SpecializedMeshPipeline for WireframePipeline {
    type Key = Material2dKey<WireframeMaterial>;

    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayoutRef,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let mut descriptor = self.material2d_pipeline.specialize(key, layout)?;
        assert_eq!(descriptor.layout.len(), 3);
        descriptor.layout.push(self.face_layout.clone());
        assert_eq!(descriptor.layout.len(), 4);
        descriptor.vertex.shader_defs.push("WIREFRAME_MATERIAL".into());
        descriptor.fragment.as_mut().map(|f| f.shader_defs.push("WIREFRAME_MATERIAL".into()));
        // descriptor.vertex.shader = self.shader.clone();
        // descriptor.fragment.as_mut().unwrap().shader = self.shader.clone();
        descriptor.label = Some("wireframe_material2d_pipeline".into());
        Ok(descriptor)
    }
}


#[derive(Default)]
pub struct WireframeMaterial2dPlugin;

impl Plugin for WireframeMaterial2dPlugin {
    fn build(&self, app: &mut App) {

        app.add_plugins(crate::compute::FacePlugin);
        embedded_asset!(app, "wireframe.wgsl");
        // app.add_plugins(Material2dGenericPlugin::<WireframeMaterial, DrawWireframeMaterial2d<WireframeMaterial>, Material2dPipeline<WireframeMaterial>>::default());
        app.add_plugins(Material2dGenericPlugin::<WireframeMaterial, DrawWireframeMaterial2d<WireframeMaterial>, WireframePipeline>::default())
            // .register_asset_reflect::<WireframeMaterial>();

                .add_systems(ExtractSchedule, extract_wireframe_meshes_2d);
        app.world_mut()
            .resource_mut::<Assets<WireframeMaterial>>()
            .insert(
                &Handle::<WireframeMaterial>::default(),
                WireframeMaterial {
                    color: Color::srgb(1.0, 0.0, 1.0).into(),
                    ..Default::default()
                },
            );
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
    #[uniform(0)]
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

    // fn specialize(
    //     descriptor: &mut RenderPipelineDescriptor,
    //     layout: &MeshVertexBufferLayoutRef,
    //     key: Material2dKey<Self>,
    // ) -> Result<(), SpecializedMeshPipelineError> {
    //     Ok(())
    // }
}

/// A component bundle for entities with a [`Mesh2dHandle`](crate::Mesh2dHandle) and a [`WireframeMaterial`].
pub type WireframeMesh2dBundle = MaterialMesh2dBundle<WireframeMaterial>;
